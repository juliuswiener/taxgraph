"""Blind-reproduction check: candidate A vs the hand-formalised reference.

The candidate never sees the reference (enforced by check_leakage.py). Here the
candidate module and the untouched reference file are compiled together to Python
via clerk, and both scopes are evaluated over the task's input raster. A match
means the candidate reproduced the reference *extensionally*, not textually.

Reference inputs the prescribed signature deliberately omits (e.g. the
Veranlagungszeitraum enum, which a blind formalisation cannot know - see
CatalaLang/catala#1074) are supplied via `ref.fixed_inputs`.
"""

from __future__ import annotations

import glob
import importlib
import os
import shutil
import subprocess
import sys
import tempfile
import uuid

PIPELINE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, PIPELINE)

from gates import scope_name, _snake  # noqa: E402


def _clerk_project(cand_src: str, cand_module: str, ref_file: str, ref_module: str,
                   d: str) -> None:
    rules = os.path.join(d, "rules")
    os.makedirs(rules, exist_ok=True)
    with open(os.path.join(d, "clerk.toml"), "w", encoding="utf-8") as f:
        f.write('[project]\ninclude_dirs = ["rules"]\nbuild_dir = "_build"\n\n'
                '[[target]]\nname = "blind"\n'
                f'modules = ["{cand_module}", "{ref_module}"]\n'
                'backends = ["python"]\ninclude_sources = true\n')
    with open(os.path.join(rules, f"{cand_module}.catala_en"), "w", encoding="utf-8") as f:
        f.write(f"# {cand_module}\n\n> Module {cand_module}\n\n```catala\n{cand_src}\n```\n")
    shutil.copy(ref_file, os.path.join(rules, f"{ref_module}.catala_en"))
    r = subprocess.run(["clerk", "build", "blind"], cwd=d, capture_output=True,
                       text=True, timeout=900)
    if r.returncode != 0:
        raise RuntimeError((r.stderr or r.stdout)[-300:])


def _load(d: str, names: list[str]):
    pkgname = "blindpkg_" + uuid.uuid4().hex[:8]
    root = os.path.join(d, "_imp")
    pkg, rt_dir = os.path.join(root, pkgname), os.path.join(root, "_rt")
    os.makedirs(pkg, exist_ok=True)
    os.makedirs(rt_dir, exist_ok=True)
    for p in glob.glob(os.path.join(d, "_build", "libcatala", "python", "*.py")):
        shutil.copy(p, pkg)
    for p in glob.glob(os.path.join(d, "_build", "rules", "python", "*.py")):
        shutil.copy(p, pkg)
    for n in ("catala_runtime.py", "dates.py"):
        src = os.path.join(pkg, n)
        if os.path.exists(src):
            shutil.copy(src, rt_dir)
    open(os.path.join(pkg, "__init__.py"), "w").close()
    sys.path.insert(0, rt_dir)
    sys.path.insert(0, root)
    importlib.invalidate_caches()
    mods = {n: importlib.import_module(f"{pkgname}.{n}") for n in names}
    return mods, importlib.import_module("catala_runtime")


def _coerce(rt, value, typ: str):
    if typ == "money":
        return rt.Money(f"{int(value)}.00")
    if typ == "decimal":
        return rt.Decimal(str(value))
    if typ == "bool":
        return rt.Bool(bool(value))
    return int(value)


def _call(mod, scope: str, kwargs: dict, out_field: str):
    inp = getattr(mod, f"{scope}In")(**kwargs)
    res = getattr(mod, _snake(scope))(inp)
    return getattr(res, out_field)


def blind_repro_match(cand_src: str, task: dict, root: str) -> tuple[bool | None, str]:
    """Return (match, detail). match=None when the check could not run."""
    ref = task["ref"]
    sig = task["signature"]
    raster = task["raster"]
    types = sig["inputs"]
    cand_scope = scope_name(cand_src)
    if not cand_scope:
        return None, "candidate has no scope"

    cand_module = "CandMod"
    ref_file = os.path.join(root, ref["file"])
    with tempfile.TemporaryDirectory() as d:
        try:
            _clerk_project(cand_src, cand_module, ref_file, ref["module"], d)
            mods, rt = _load(d, [cand_module, ref["module"]])
        except Exception as e:  # noqa: BLE001
            return None, f"build/load failed: {str(e)[-200:]}"

        # fixed reference inputs the signature omits (e.g. enums)
        fixed = {}
        for k, spec in (ref.get("fixed_inputs") or {}).items():
            if isinstance(spec, dict) and "enum" in spec:
                enum_cls = getattr(mods[ref["module"]], spec["enum"])
                fixed[f"{k}_in"] = enum_cls(getattr(enum_cls.Code, spec["code"]), None)
            else:
                fixed[f"{k}_in"] = spec

        diffs = []
        for point in raster:
            ckw = {f"{k}_in": _coerce(rt, v, types[k]) for k, v in point.items()}
            rkw = dict(fixed)
            for sig_name, ref_name in ref["input_map"].items():
                rkw[f"{ref_name}_in"] = _coerce(rt, point[sig_name], types[sig_name])
            try:
                cv = _call(mods[cand_module], cand_scope, ckw, sig["output"])
                rv = _call(mods[ref["module"]], ref["scope"], rkw, ref["output"])
            except Exception as e:  # noqa: BLE001
                return None, f"call failed: {str(e)[-200:]}"
            if str(cv) != str(rv):
                diffs.append((point, str(cv), str(rv)))

    if diffs:
        return False, f"{len(diffs)}/{len(raster)} raster points differ; first={diffs[0]}"
    return True, f"matches reference on {len(raster)} raster point(s)"
