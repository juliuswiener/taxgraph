"""Deterministic gate cascade steps.

Model-produced text is mocked in dry-run (via fixtures in the client); the gates
here are deterministic. Compiler-backed gates (typecheck, equivalence, clerk)
require the Catala toolchain; if `catala`/`clerk` are not on PATH they return
status SKIP with a reason instead of failing, so the full cascade wiring is
testable now and the same gates run for real once the toolchain is reinstalled.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass

PASS, FAIL, SKIP = "PASS", "FAIL", "SKIP"


@dataclass
class GateResult:
    name: str
    status: str
    detail: str = ""


def have(tool: str) -> bool:
    return shutil.which(tool) is not None


# -- parsing model output ----------------------------------------------------

def extract_catala(formalizer_text: str) -> tuple[str | None, str | None]:
    """Return (module_name, catala_source) from a formaliser output."""
    m = re.search(r"MODULE:\s*(\S+)", formalizer_text)
    module = m.group(1) if m else None
    b = re.search(r"```catala\s*(.*?)```", formalizer_text, re.DOTALL)
    return module, (b.group(1).strip() if b else None)


# -- gates -------------------------------------------------------------------

_ANSI = re.compile(r"\x1b\[[0-9;]*m")
_CHECK_CACHE: dict[tuple[str, str], tuple[int, str]] = {}


def _strip(s: str) -> str:
    return _ANSI.sub("", s)


def _catala_check(catala_src: str, module: str) -> tuple[int, str]:
    """Run the real Catala parser+typechecker once; cache per (src, module).

    There is no tree-sitter grammar for Catala (not on PyPI, not in the Catala
    sources - only editor highlighters). Rather than hand-roll a second grammar
    that would drift from the compiler, the syntax gate uses the compiler's own
    parser. `clerk typecheck` reports "Syntax error at ..." for parse failures and
    "Incompatible types" for type failures, so one run feeds both gates
    deterministically. Results are cached so the cascade does not pay twice.
    """
    key = (catala_src, module)
    if key in _CHECK_CACHE:
        return _CHECK_CACHE[key]
    with tempfile.TemporaryDirectory() as d:
        path = _write_clerk_project(catala_src, module, d)
        rel = os.path.relpath(path, d)
        try:
            r = subprocess.run(["clerk", "typecheck", rel], cwd=d,
                               capture_output=True, text=True, timeout=600)
            out = _strip((r.stderr or "") + (r.stdout or ""))
            res = (r.returncode, out)
        except Exception as e:  # noqa: BLE001
            res = (-1, f"clerk error: {e}")
    _CHECK_CACHE[key] = res
    return res


def syntax_gate(catala_src: str | None, module: str | None = None) -> GateResult:
    """Parse gate using the Catala compiler's own parser (deterministic)."""
    if not catala_src:
        return GateResult("syntax", FAIL, "no catala block in output")
    if not have("clerk"):
        return GateResult("syntax", SKIP, "clerk not on PATH (toolchain missing)")
    rc, out = _catala_check(catala_src, module or "Cand")
    if "Syntax error" in out:
        first = next((l.strip() for l in out.splitlines() if "Syntax error" in l), "")
        return GateResult("syntax", FAIL, first[:200])
    # rc != 0 without a syntax error means it parsed and failed later (types):
    # that is the typecheck gate's business, not the parser's.
    return GateResult("syntax", PASS, "catala parser ok")


def _write_clerk_project(catala_src: str, module: str, d: str) -> str:
    """Write a minimal clerk project so the Catala stdlib resolves.

    `catala typecheck` on a bare file fails with "Stdlib_en could not be found";
    the stdlib is set up by clerk within a project. So we materialise a temporary
    clerk project and typecheck through clerk.
    """
    rules = os.path.join(d, "rules")
    os.makedirs(rules, exist_ok=True)
    with open(os.path.join(d, "clerk.toml"), "w", encoding="utf-8") as f:
        f.write('[project]\ninclude_dirs = ["rules"]\nbuild_dir = "_build"\n')
    path = os.path.join(rules, f"{module}.catala_en")
    with open(path, "w", encoding="utf-8") as f:
        f.write(f"# {module}\n\n> Module {module}\n\n```catala\n{catala_src}\n```\n")
    return path


def typecheck_gate(catala_src: str | None, module: str | None) -> GateResult:
    if not catala_src or not module:
        return GateResult("typecheck", FAIL, "no source/module")
    if not have("clerk"):
        return GateResult("typecheck", SKIP, "clerk not on PATH (toolchain missing)")
    rc, out = _catala_check(catala_src, module)   # cached; shared with syntax_gate
    if rc == 0:
        return GateResult("typecheck", PASS, "clerk typecheck ok")
    if "Syntax error" in out:
        return GateResult("typecheck", FAIL, "did not parse (see syntax gate)")
    # keep the informative head of the compiler diagnostic, not a tail slice
    lines = [l.strip(" │├└┌─➤") for l in out.splitlines()]
    lines = [l for l in lines if l and not l.startswith("[")]
    msg = " ".join(lines[1:5]) if len(lines) > 1 else " ".join(lines)
    return GateResult("typecheck", FAIL, msg[:220])


def scope_name(catala_src: str) -> str | None:
    m = re.search(r"declaration\s+scope\s+(\w+)\s*:", catala_src)
    return m.group(1) if m else None


def _snake(name: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def _build_python(a_src: str, b_src: str, d: str) -> str:
    """Materialise a clerk project with both candidates and build the python target."""
    rules = os.path.join(d, "rules")
    os.makedirs(rules, exist_ok=True)
    with open(os.path.join(d, "clerk.toml"), "w", encoding="utf-8") as f:
        f.write('[project]\ninclude_dirs = ["rules"]\nbuild_dir = "_build"\n\n'
                '[[target]]\nname = "eq"\nmodules = ["ModA", "ModB"]\n'
                'backends = ["python"]\ninclude_sources = true\n')
    for mod, src in (("ModA", a_src), ("ModB", b_src)):
        with open(os.path.join(rules, f"{mod}.catala_en"), "w", encoding="utf-8") as f:
            f.write(f"# {mod}\n\n> Module {mod}\n\n```catala\n{src}\n```\n")
    r = subprocess.run(["clerk", "build", "eq"], cwd=d, capture_output=True,
                       text=True, timeout=600)
    if r.returncode != 0:
        raise RuntimeError((r.stderr or r.stdout)[-400:])
    return d


def _load_modules(d: str):
    """Import generated ModA/ModB plus the catala python runtime from the build.

    The generated modules use package-relative imports (`from . import Stdlib_en`)
    and an absolute `from catala_runtime import *`. So the stdlib and the rule
    modules are collected into one package directory, and the runtime is placed on
    sys.path as a top-level module. The package name is unique per call so repeated
    gate runs do not collide in sys.modules.
    """
    import glob as _glob
    import importlib
    import sys
    import uuid

    pkgname = "eqpkg_" + uuid.uuid4().hex[:8]
    root = os.path.join(d, "_imp")
    pkg = os.path.join(root, pkgname)
    rt_dir = os.path.join(root, "_rt")
    os.makedirs(pkg, exist_ok=True)
    os.makedirs(rt_dir, exist_ok=True)

    for p in _glob.glob(os.path.join(d, "_build", "libcatala", "python", "*.py")):
        shutil.copy(p, pkg)
    for p in _glob.glob(os.path.join(d, "_build", "rules", "python", "*.py")):
        shutil.copy(p, pkg)
    # the runtime and its own deps must be importable top-level
    # (catala_runtime does `import dates`)
    for name in ("catala_runtime.py", "dates.py"):
        src_rt = os.path.join(pkg, name)
        if os.path.exists(src_rt):
            shutil.copy(src_rt, rt_dir)
    open(os.path.join(pkg, "__init__.py"), "w").close()

    sys.path.insert(0, rt_dir)
    sys.path.insert(0, root)
    importlib.invalidate_caches()
    mods = {name: importlib.import_module(f"{pkgname}.{name}") for name in ("ModA", "ModB")}
    rt = importlib.import_module("catala_runtime")
    return mods, rt


def _coerce(rt, value, typ: str):
    if typ == "money":
        return rt.Money(f"{int(value)}.00")
    if typ == "decimal":
        return rt.Decimal(str(value))
    if typ == "bool":
        return rt.Bool(bool(value))
    return int(value)


def equivalence_gate(a_src: str | None, b_src: str | None,
                     candidate: dict | None = None) -> GateResult:
    """Extensional equivalence of A and B on an input raster.

    Compiles both candidates to Python via clerk and compares the declared output
    field over `candidate["raster"]`. Falls back to a normalised-text proxy when
    the toolchain or a raster spec is unavailable.
    """
    if not a_src or not b_src:
        return GateResult("equivalence", FAIL, "missing A/B source")
    norm = lambda s: re.sub(r"\s+", " ", s).strip()  # noqa: E731
    if not have("clerk"):
        same = norm(a_src) == norm(b_src)
        return GateResult("equivalence", SKIP,
                          f"clerk missing; textual-proxy {'match' if same else 'DIVERGENT'}")

    cand = candidate or {}
    out_field = cand.get("output_field")
    if not out_field:
        same = norm(a_src) == norm(b_src)
        return GateResult("equivalence", SKIP,
                          f"no output_field in candidate; textual-proxy "
                          f"{'match' if same else 'DIVERGENT'}")

    raster = cand.get("raster") or [{}]
    types = cand.get("input_types") or {}
    sa, sb = scope_name(a_src), scope_name(b_src)
    if not sa or not sb:
        return GateResult("equivalence", FAIL, "cannot determine scope name")

    with tempfile.TemporaryDirectory() as d:
        try:
            _build_python(a_src, b_src, d)
            mods, rt = _load_modules(d)
        except Exception as e:  # noqa: BLE001
            return GateResult("equivalence", FAIL, f"build/load failed: {str(e)[-300:]}")

        diffs = []
        for point in raster:
            vals = {}
            for k, v in point.items():
                vals[f"{k}_in"] = _coerce(rt, v, types.get(k, "int"))
            try:
                ra = getattr(mods["ModA"], _snake(sa))(
                    getattr(mods["ModA"], f"{sa}In")(**vals))
                rb = getattr(mods["ModB"], _snake(sb))(
                    getattr(mods["ModB"], f"{sb}In")(**vals))
                va, vb = getattr(ra, out_field), getattr(rb, out_field)
            except Exception as e:  # noqa: BLE001
                return GateResult("equivalence", FAIL, f"run failed: {str(e)[-300:]}")
            if str(va) != str(vb):
                diffs.append((point, str(va), str(vb)))

    if diffs:
        return GateResult("equivalence", FAIL,
                          f"{len(diffs)}/{len(raster)} raster points diverge; "
                          f"first={diffs[0]}")
    return GateResult("equivalence", PASS,
                      f"A == B on {len(raster)} raster point(s) for '{out_field}'")


def roundtrip_parse(judge_json_text: str) -> dict:
    """Structured judge verdict for the two-stage round-trip metric.

    Stage 1 (here): did the judge itself flag deviations / silent assumptions?
    Stage 2 (evaluate.py): if the judge said `faithful` but equivalence or the
    blind reference disagrees, the judge *missed* an assumption. Both are needed;
    a judge that never flags anything scores well on stage 1 and badly on stage 2.
    """
    try:
        d = json.loads(judge_json_text)
    except json.JSONDecodeError:
        # try to salvage a JSON object embedded in prose
        m = re.search(r"\{.*\}", judge_json_text, re.DOTALL)
        if not m:
            return {"parse_error": True}
        try:
            d = json.loads(m.group(0))
        except json.JSONDecodeError:
            return {"parse_error": True}
    return {
        "parse_error": False,
        "faithful": bool(d.get("faithful")),
        "abweichungen": list(d.get("abweichungen") or []),
        "stille_zusatzannahmen": list(d.get("stille_zusatzannahmen") or []),
    }


def roundtrip_gate(judge_json_text: str) -> GateResult:
    d = roundtrip_parse(judge_json_text)
    if d.get("parse_error"):
        return GateResult("roundtrip", FAIL, "judge output not valid JSON")
    if d["faithful"] and not d["stille_zusatzannahmen"]:
        return GateResult("roundtrip", PASS, "round-trip faithful, no silent assumptions")
    return GateResult("roundtrip", FAIL,
                      f"abweichungen={len(d['abweichungen'])} "
                      f"annahmen={len(d['stille_zusatzannahmen'])}")


_UMLAUT = str.maketrans({"ä": "ae", "ö": "oe", "ü": "ue", "ß": "ss",
                         "Ä": "ae", "Ö": "oe", "Ü": "ue"})


def _normalize(text: str) -> str:
    return re.sub(r"\s+", " ", text.translate(_UMLAUT).lower()).strip()


def _lit(value, typ: str) -> str | None:
    """Render a Catala literal. Unsupported type -> None."""
    if typ == "money":
        return f"${int(value)}.00"
    if typ == "int":
        return str(int(value))
    if typ == "bool":
        return "true" if value else "false"
    if typ == "decimal":
        return str(float(value))
    return None


def clerk_gate(catala_src: str | None, module: str | None = None,
               candidate: dict | None = None, root: str | None = None) -> GateResult:
    """Run generated Clerk tests seeded from official worked examples.

    Test seeds must come from EStH/BMF worked examples and carry a verbatim
    citation anchor, verified against the frozen source before anything runs.
    Where no worked example exists the gate is n/a for that task (equivalence +
    round-trip carry it); LLM-invented expected values are never a test reference.
    """
    if not have("clerk"):
        return GateResult("clerk", SKIP, "clerk not on PATH (toolchain missing)")
    cand = candidate or {}
    seeds = cand.get("test_seed")
    if not seeds or seeds == "none":
        return GateResult("clerk", SKIP,
                          "n/a: kein EStH/BMF-Rechenbeispiel; Aequivalenz + "
                          "Round-Trip tragen (keine LLM-Erwartungswerte)")
    if not catala_src or not module:
        return GateResult("clerk", FAIL, "no source/module")

    root = root or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    types = cand.get("input_types") or {}
    out_field = cand.get("output_field")
    scope = scope_name(catala_src)
    if not scope or not out_field:
        return GateResult("clerk", FAIL, "cannot determine scope/output_field")

    # 1. citation-anchor gate on every seed (deterministic, no model involved)
    for i, s in enumerate(seeds):
        src = os.path.join(root, s["quelle"])
        if not os.path.exists(src):
            return GateResult("clerk", FAIL, f"seed {i}: source missing {s['quelle']}")
        if _normalize(s["zitatanker"]) not in _normalize(open(src, encoding="utf-8").read()):
            return GateResult("clerk", FAIL,
                              f"seed {i}: Zitatanker nicht im Quelltext ({s['quelle']})")

    # 2. generate test scopes, run clerk
    tests = []
    for i, s in enumerate(seeds):
        args = []
        for k, v in (s.get("inputs") or {}).items():
            lit = _lit(v, types.get(k, "int"))
            if lit is None:
                return GateResult("clerk", SKIP, f"seed {i}: unsupported input type '{k}'")
            args.append(f"    -- {k}: {lit}")
        exp = _lit(s["expected"], cand.get("output_type", "money"))
        if exp is None:
            return GateResult("clerk", SKIP, f"seed {i}: unsupported output type")
        argblk = ("\n" + "\n".join(args)) if args else ""
        tests.append(
            f"#[test]\ndeclaration scope TestSeed{i}:\n"
            f"  internal r content {module}.{scope}\n"
            f"  output v content money\n"
            f"scope TestSeed{i}:\n"
            f"  definition r equals output of {module}.{scope} with {{{argblk} }}\n"
            f"  definition v equals r.{out_field}\n"
            f"  assertion v = {exp}\n")

    with tempfile.TemporaryDirectory() as d:
        _write_clerk_project(catala_src, module, d)
        with open(os.path.join(d, "rules", "Tests.catala_en"), "w", encoding="utf-8") as f:
            f.write(f"# Tests\n\n> Using {module}\n\n```catala\n" + "\n".join(tests) + "```\n")
        try:
            r = subprocess.run(["clerk", "test", "-W", "rules/Tests.catala_en"], cwd=d,
                               capture_output=True, text=True, timeout=900)
        except Exception as e:  # noqa: BLE001
            return GateResult("clerk", FAIL, f"clerk error: {e}")
        if r.returncode != 0:
            out = _strip((r.stderr or "") + (r.stdout or ""))
            return GateResult("clerk", FAIL, out[-250:].replace("\n", " ").strip())
    return GateResult("clerk", PASS, f"{len(seeds)} seed test(s) passed, anchors verified")
