"""Blind-reproduction check: candidate A vs the hand-formalised reference.

Isolation by process boundary, not by namespace. Candidate and reference are
compiled in SEPARATE clerk projects and evaluated in SEPARATE subprocesses:
raster points go in as JSON, results come out as JSON, the comparison happens
here. Consequences:

  * no namespace/enum mixing between the two modules is even possible;
  * the reference file is only ever read, never linked against candidate code -
    a stronger isolation argument for the bake-off protocol;
  * the same mechanism works unchanged for candidate-vs-candidate in the
    equivalence gate.

Reference inputs the prescribed signature deliberately omits (e.g. the
Veranlagungszeitraum enum, which a blind formalisation cannot know - see
CatalaLang/catala#1074) are supplied via `ref.fixed_inputs`.

A build that fails is reported as `blind_build_error` (candidate) or
`blind_ref_error` (reference) - never a crash and never a silent n/a.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile

PIPELINE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, PIPELINE)

from gates import scope_name  # noqa: E402

# Child program: imports one generated module, evaluates the scope on the raster,
# prints results as JSON. Runs in its own interpreter, own sys.path, own package.
_CHILD = r'''
import json, sys, importlib, glob, os, shutil
spec = json.load(sys.stdin)
root, pkg = spec["root"], spec["pkg"]
sys.path.insert(0, os.path.join(root, "_rt"))
sys.path.insert(0, root)
mod = importlib.import_module(pkg + "." + spec["module"])
rt = importlib.import_module("catala_runtime")

def coerce(v, t):
    # money must survive sub-euro amounts: 0.38 Euro/km is a money literal in the
    # reference. int(v) would have silently turned it into $0.00.
    if t == "money":   return rt.Money(f"{float(v):.2f}")
    if t == "decimal": return rt.Decimal(str(v))
    if t == "bool":    return rt.Bool(bool(v))
    # rt.Integer, nicht int: ein nackter Python-int laesst jeden Catala-Vergleich
    # mit "comparison of values of these types is not supported" scheitern.
    return rt.Integer(int(v))

def snake(n):
    import re
    return re.sub(r"(?<!^)(?=[A-Z])", "_", n).lower()

fixed = {}
for k, s in (spec.get("fixed_inputs") or {}).items():
    if isinstance(s, dict) and "enum" in s:
        cls = getattr(mod, s["enum"])
        fixed[k + "_in"] = cls(getattr(cls.Code, s["code"]), None)
    elif isinstance(s, dict) and "value" in s:
        fixed[k + "_in"] = coerce(s["value"], s["type"])
    else:
        fixed[k + "_in"] = s

out = []
try:
    scope, field = spec["scope"], spec["output"]
    fn = getattr(mod, snake(scope))
    In = getattr(mod, scope + "In")
    for point in spec["raster"]:
        kw = dict(fixed)
        for name, (val, typ) in point.items():
            kw[name + "_in"] = coerce(val, typ)
        out.append(str(getattr(fn(In(**kw)), field)))
except Exception as e:
    print(json.dumps({"error": f"{type(e).__name__}: {e}"[:300]}))
    sys.exit(0)
print(json.dumps({"values": out}))
'''


def _build(module: str, d: str, *, src: str | None = None, file: str | None = None) -> str:
    """Compile ONE module in its own clerk project; return the import root."""
    rules = os.path.join(d, "rules")
    os.makedirs(rules, exist_ok=True)
    with open(os.path.join(d, "clerk.toml"), "w", encoding="utf-8") as f:
        f.write('[project]\ninclude_dirs = ["rules"]\nbuild_dir = "_build"\n\n'
                f'[[target]]\nname = "one"\nmodules = ["{module}"]\n'
                'backends = ["python"]\ninclude_sources = true\n')
    dst = os.path.join(rules, f"{module}.catala_en")
    if file:
        shutil.copy(file, dst)          # reference: read-only copy, never edited
    else:
        with open(dst, "w", encoding="utf-8") as f:
            f.write(f"# {module}\n\n> Module {module}\n\n```catala\n{src}\n```\n")
    r = subprocess.run(["clerk", "build", "one"], cwd=d, capture_output=True,
                       text=True, timeout=900)
    if r.returncode != 0:
        raise RuntimeError((r.stderr or r.stdout)[-250:].replace("\n", " "))

    # assemble an importable package: stdlib + module in a package, runtime top-level
    root = os.path.join(d, "_imp")
    pkg = os.path.join(root, "pkg")
    rt_dir = os.path.join(root, "_rt")
    os.makedirs(pkg, exist_ok=True)
    os.makedirs(rt_dir, exist_ok=True)
    for pat in ("libcatala/python/*.py", "rules/python/*.py"):
        for p in glob_py(os.path.join(d, "_build", pat)):
            shutil.copy(p, pkg)
    for n in ("catala_runtime.py", "dates.py"):
        s = os.path.join(pkg, n)
        if os.path.exists(s):
            shutil.copy(s, rt_dir)
    open(os.path.join(pkg, "__init__.py"), "w").close()
    return root


def glob_py(pattern: str) -> list[str]:
    import glob as _g
    return _g.glob(pattern)


def _evaluate(root: str, module: str, scope: str, output: str,
              raster_typed: list[dict], fixed_inputs: dict) -> list[str]:
    """Run the module in its own subprocess; raster in as JSON, values out."""
    spec = {"root": root, "pkg": "pkg", "module": module, "scope": scope,
            "output": output, "raster": raster_typed, "fixed_inputs": fixed_inputs}
    r = subprocess.run([sys.executable, "-c", _CHILD], input=json.dumps(spec),
                       capture_output=True, text=True, timeout=300)
    if r.returncode != 0:
        raise RuntimeError((r.stderr or "")[-250:].replace("\n", " "))
    try:
        d = json.loads(r.stdout.strip().splitlines()[-1])
    except Exception:  # noqa: BLE001
        raise RuntimeError(f"child produced no JSON: {r.stdout[-150:]}") from None
    if "error" in d:
        raise RuntimeError(d["error"])
    return d["values"]


def blind_repro_match(cand_src: str, task: dict, root_repo: str) -> tuple[bool | None, str, str]:
    """Return (match, status, detail).

    status: match | mismatch | blind_build_error | blind_ref_error |
            blind_call_error | blind_no_scope
    """
    ref = task["ref"]
    sig = task["signature"]
    types = sig["inputs"]
    cand_scope = scope_name(cand_src)
    if not cand_scope:
        return None, "blind_no_scope", "candidate has no declaration scope"

    # raster carrying types, so each child can coerce without sharing objects
    cand_raster = [{k: [v, types[k]] for k, v in p.items()} for p in task["raster"]]
    ref_raster = [{ref["input_map"][k]: [v, types[k]] for k, v in p.items()}
                  for p in task["raster"]]

    with tempfile.TemporaryDirectory() as dc, tempfile.TemporaryDirectory() as dr:
        try:
            croot = _build("CandMod", dc, src=cand_src)
        except Exception as e:  # noqa: BLE001
            return None, "blind_build_error", f"candidate build failed: {e}"
        try:
            rroot = _build(ref["module"], dr,
                           file=os.path.join(root_repo, ref["file"]))
        except Exception as e:  # noqa: BLE001
            return None, "blind_ref_error", f"reference build failed: {e}"

        try:
            cvals = _evaluate(croot, "CandMod", cand_scope, sig["output"],
                              cand_raster, {})
        except Exception as e:  # noqa: BLE001
            msg = str(e)
            # Der Kandidat hat die vorgegebene Signatur nicht eingehalten
            # (zusaetzliche/fehlende Eingaben, anderer Scope- oder Feldname).
            # Das ist ein Befund ueber das Modell, kein Infrastrukturfehler.
            if ("Extra field" in msg or "missing" in msg.lower()
                    or "AttributeError" in msg):
                return None, "blind_signature_mismatch", f"signature violated: {msg[:180]}"
            # Catala-Laufzeitfehler des Kandidaten: kollidierende Ausnahmen,
            # fehlende Definition, Assertion. Modellfehler, nicht Infrastruktur.
            if any(k in msg for k in ("Conflict", "NoValue", "AssertionFailed",
                                      "Empty", "Uncomparable")):
                return None, "blind_runtime_error", f"candidate runtime error: {msg[:180]}"
            return None, "blind_call_error", f"candidate call failed: {msg[:180]}"
        try:
            rvals = _evaluate(rroot, ref["module"], ref["scope"], ref["output"],
                              ref_raster, ref.get("fixed_inputs") or {})
        except Exception as e:  # noqa: BLE001
            return None, "blind_ref_error", f"reference call failed: {e}"

    diffs = [(task["raster"][i], c, r) for i, (c, r) in enumerate(zip(cvals, rvals))
             if c != r]
    if diffs:
        return False, "mismatch", (f"{len(diffs)}/{len(cvals)} raster points differ; "
                                   f"first={diffs[0]}")
    return True, "match", f"matches reference on {len(cvals)} raster point(s)"
