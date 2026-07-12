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
# INFO: ein Befund, der gemeldet wird, aber KEIN Gate kippt (Stufe-1-Rollout des
# Praezisions-Lints). Weder FAIL noch SKIP -> die Queue-Entscheidung ignoriert ihn.
INFO = "INFO"


@dataclass
class GateResult:
    name: str
    status: str
    detail: str = ""


def have(tool: str) -> bool:
    return shutil.which(tool) is not None


# -- parsing model output ----------------------------------------------------

def extract_catala(formalizer_text: str | None) -> tuple[str | None, str | None]:
    """Return (module_name, catala_source) from a formaliser output.

    Tolerates an empty/None answer (OpenRouter returns content=null on refusals
    and pure-reasoning replies); the gates then fail cleanly instead of crashing.
    """
    if not formalizer_text:
        return None, None
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
        return rt.Money(f"{float(value):.2f}")   # sub-euro amounts must survive
    if typ == "decimal":
        return rt.Decimal(str(value))
    if typ == "bool":
        return rt.Bool(bool(value))
    # Ein nackter Python-int ist KEIN Catala-Integer. Vergleiche wie `i >= 3`
    # scheitern dann mit "comparison of values of these types is not supported" -
    # ein Laufzeitfehler des Gates, der wie eine Divergenz von A und B aussieht.
    return rt.Integer(int(value))


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


# Rundungsoperationen in Catala. `truncate` fing den § 33-Defekt (A schnitt auf
# ganze Euro ab, obwohl die Norm keine Rundung vorschreibt).
_RUNDUNG_RE = re.compile(r"\b(round|truncate|floor|ceiling|ceil)\b", re.I)


def _rundung_erlaubt(candidate: dict | None) -> str | None:
    """Traegt die Regel eine deklarierte Rundungs-Erlaubnis (Quelle + Zitatanker,
    Muster § 32a Abs. 1 S. 6)? Der Zitatanker muss woertlich in der Norm stehen -
    sonst ist die Deklaration eine leere Behauptung. Rueckgabe: Herkunft oder None."""
    if not candidate:
        return None
    decls = candidate.get("rundung") or []
    if isinstance(decls, dict):
        decls = [decls]
    norm = candidate.get("norm_text") or ""
    norm_n = _normalize(norm)
    for d in decls:
        if not isinstance(d, dict):
            continue
        anker, quelle = d.get("zitatanker"), d.get("quelle")
        if not anker or not quelle:
            continue
        if not norm or _normalize(anker) in norm_n:
            return f"{quelle} ('{str(anker)[:40]}')"
    return None


# Catala-Rundungsoperation -> Richtung. truncate schneidet ab (Richtung floor fuer
# die nichtnegativen Steuerbetraege), ceiling rundet auf, round ist kaufmaennisch.
_OP_RICHTUNG = {"truncate": "floor", "floor": "floor",
                "ceiling": "ceil", "ceil": "ceil", "round": "kaufmaennisch"}
_RICHTUNGEN = {"floor", "ceil", "kaufmaennisch"}


def _rundung_richtungen(candidate: dict | None) -> set:
    """Deklarierte Rundungs-Richtungen (floor/ceil/kaufmaennisch), falls die
    `rundung`-Deklaration(en) ein `richtung`-Feld tragen. Leere Menge = keine
    Richtung deklariert (Legacy: jede Richtung erlaubt, nur Existenz gelintet)."""
    if not candidate:
        return set()
    decls = candidate.get("rundung") or []
    if isinstance(decls, dict):
        decls = [decls]
    out = set()
    for d in decls:
        if isinstance(d, dict) and d.get("richtung"):
            out.add(str(d["richtung"]).strip().lower())
    return out


def rundungs_lint_gate(catala_src: str | None,
                       candidate: dict | None = None) -> GateResult:
    """Deterministischer Rundungs-Lint (Dekret 2026-07-11, Richtung 2026-07-12).

    Jede Rundungsoperation (truncate/round/floor/ceil) im Kandidaten-Source ist
    nur zulaessig, wenn die Regel eine deklarierte `rundung`-Erlaubnis mit Quelle
    und Zitatanker traegt. Traegt die Deklaration zusaetzlich ein `richtung`-Feld
    (floor/ceil/kaufmaennisch), muss die im Catala genutzte Rundungs-RICHTUNG damit
    uebereinstimmen - sonst FAIL (faengt 'Bruchteile bleiben ausser Ansatz'=floor,
    das Modell rundet aber kaufmaennisch: solzg-Residual). Sonst FAIL bei
    undeklarierter Rundung. Laeuft VOR der Aequivalenz, kostenlos; der Befund geht
    woertlich in die Repair-Runde (wie ein Compiler-Fehler).
    """
    if not catala_src:
        return GateResult("rundungs_lint", SKIP, "kein A-Source")
    treffer = []                              # (zeile, text, {richtungen der ops})
    for i, line in enumerate(catala_src.splitlines(), 1):
        code = line.split("#", 1)[0]          # Zeilenkommentare ignorieren
        richt = {_OP_RICHTUNG[m.group(1).lower()]
                 for m in _RUNDUNG_RE.finditer(code)}
        if richt:
            treffer.append((i, line.strip(), richt))
    if not treffer:
        return GateResult("rundungs_lint", PASS, "keine Rundungsoperation")
    erlaubt = _rundung_erlaubt(candidate)
    if not erlaubt:
        z = "; ".join(f"Zeile {i}: {t}" for i, t, _ in treffer[:5])
        return GateResult("rundungs_lint", FAIL,
                          f"{len(treffer)} nicht deklarierte Rundungsoperation(en). Die "
                          f"Norm schreibt keine Rundung vor (keine rundung-Quelle mit "
                          f"Zitatanker deklariert). Entferne die Rundung und rechne "
                          f"exakt weiter. {z}")
    # Rundung deklariert. Wenn eine Richtung deklariert ist: Richtungs-Abgleich.
    richtungen = _rundung_richtungen(candidate)
    if richtungen:
        falsch = [(i, t, r) for i, t, richt in treffer
                  for r in richt if r not in richtungen]
        if falsch:
            z = "; ".join(f"Zeile {i}: Richtung '{r}' ({t})" for i, t, r in falsch[:5])
            return GateResult("rundungs_lint", FAIL,
                              f"{len(falsch)} Rundungs-RICHTUNGS-Abweichung(en). Die Norm "
                              f"ordnet {sorted(richtungen)} an (floor=truncate, ceil=ceiling, "
                              f"kaufmaennisch=round), der Code rundet anders. Nutze die "
                              f"deklarierte Richtung. {z}")
        return GateResult("rundungs_lint", PASS,
                          f"{len(treffer)} Rundungsop(s) durch {erlaubt} gedeckt, "
                          f"Richtung {sorted(richtungen)} ok")
    return GateResult("rundungs_lint", PASS,
                      f"{len(treffer)} Rundungsop(s) durch {erlaubt} gedeckt")


# -- Praezisions-Lint (Klasse 5): money x/ decimal VOR finalem Cent-Schnitt ------
# Catala-money ist cent-praezise: `money * decimal` rundet das Produkt SOFORT auf
# Cent. Rechnet eine Regel einen Prozentsatz auf eine money-Groesse und schneidet
# ERST danach final auf Cent, ist das Doppelrundung (solzg 20351 -> $0,12 statt 0,11).
# Fix: Prozent in `decimal` rechnen (money per `/ $1.00` oder `decimal of` nach
# decimal), Cent-Schnitt am Ende. Getrennt von rundungs_lint (Klasse 4: Richtung des
# Schnitts) - hier zaehlt die ORDNUNG davor.
# Vorregistrierung: reports/review/2026-07-12-praezisions-lint-vorregistrierung.md.
# STUFE 1 (informativ, jetzt): confident-Befund -> INFO, kippt kein Gate. STUFE 2
# (blockierend, erst nach Julius auf Stufe-1-Empirie): confident -> FAIL. Umschalter:
_PRAEZISION_BLOCKIEREND = False

# Finaler Cent-Schnitt-Idiom: (truncate|round|floor|ceil) of ( <X> / $0.01 )
# X = Ausdruck vor `/ $0.01`; [^\n]*? ueberspannt innere Klammern (lazy bis zum
# ersten Cent-Quotienten auf der Zeile), sonst wuerde ein X mit Klammern den Schnitt
# verstecken (False Negative).
_CENT_CUT_RE = re.compile(
    r"(?:truncate|round|floor|ceil(?:ing)?)\s+of\s*\(\s*([^\n]*?)\s*/\s*\$0\.01", re.I)
_DEC_LIT = r"(?<![\$\d])\d+\.\d+"                     # 0.055 - NICHT $-praefixiert
_MONEY_DECL_RE = re.compile(r"\b(?:input|output|internal)\s+(\w+)\s+content\s+money\b")
_LET_RE = re.compile(r"\blet\s+(\w+)\s+equals\b(.*)$", re.M)
_WORD_RE = re.compile(r"\b[a-z_]\w*\b")
_CATALA_KW = {"let", "equals", "in", "if", "then", "else", "of", "content", "scope",
              "definition", "decimal", "money", "integer", "boolean", "truncate",
              "round", "floor", "ceiling", "ceil", "input", "output", "internal",
              "true", "false", "and", "or", "not", "date", "duration"}


def _praez_money_names(src: str) -> set:
    """money-typisierte Namen. Typ-Tracking: `let Y equals RHS` erbt money nur, wenn RHS
    money traegt UND nicht durch ein money-Literal/-Name teilt (money/money -> decimal)
    und kein `decimal of` traegt. Ohne dieses Tracking flaggt der Lint die Fix-Form."""
    names = set(_MONEY_DECL_RE.findall(src))
    for _ in range(8):                               # Fixpunkt ueber let-Ketten
        added = False
        for m in _LET_RE.finditer(src):
            y, rhs = m.group(1), m.group(2)
            if y in names:
                continue
            has_money = "$" in rhs or any(re.search(rf"\b{re.escape(n)}\b", rhs)
                                          for n in names)
            teilt_money = bool(re.search(r"/\s*\$", rhs)) or \
                any(re.search(rf"/\s*{re.escape(n)}\b", rhs) for n in names)
            if has_money and not (teilt_money or "decimal of" in rhs.lower()):
                names.add(y)
                added = True
        if not added:
            break
    return names


def _praez_idents(expr: str) -> set:
    return {w for w in _WORD_RE.findall(expr) if w not in _CATALA_KW}


def _praez_let_bodies(src: str) -> dict:
    """{let-Variable: Body-Text}. Erfasst MEHRZEILIGE Bodies bis zum schliessenden `in`
    derselben Verschachtelungstiefe (`let` +1, `in` -1). Ohne das bleibt ein `let X
    equals\\n  if ...`-Body leer und die Fluss-Erreichbarkeit bricht ab."""
    bodies = {}
    for m in re.finditer(r"\blet\s+(\w+)\s+equals\b", src):
        start = m.end()
        depth, end = 0, len(src)
        for t in re.finditer(r"\blet\b|\bin\b", src[start:]):
            if t.group() == "let":
                depth += 1
            elif depth == 0:                         # das 'in', das DIESES let schliesst
                end = start + t.start()
                break
            else:
                depth -= 1
        bodies[m.group(1)] = src[start:end]
    return bodies


def _praez_cut_reachable(src: str) -> set | None:
    """Alle Bezeichner, die (transitiv ueber let-Bindungen) in einen finalen Cent-
    Schnitt fliessen. None, wenn kein Catala-Cent-Schnitt-Idiom vorhanden ist (dann
    ist Fluss nicht verfolgbar)."""
    seeds = set()
    for m in _CENT_CUT_RE.finditer(src):
        seeds |= _praez_idents(m.group(1))
    if not seeds:
        return None
    bodies = _praez_let_bodies(src)
    seen, stack = set(), list(seeds)
    while stack:
        v = stack.pop()
        if v in seen:
            continue
        seen.add(v)
        if v in bodies:
            stack.extend(_praez_idents(bodies[v]))
    return seen


def _praez_scale_frag(code: str, money_names: set) -> str | None:
    """Erstes money [*/] decimal-Literal (oder decimal-Literal * money) in `code`, das
    MONEY produziert - Prozent-/Quotient-auf-Geld. Der Cent-Schnitt (`/ $0.01`, `* $0.01`)
    matcht nie, da _DEC_LIT $-Literale ausschliesst."""
    for m in re.finditer(rf"([\w.$]+)\s*[*/]\s*(?:{_DEC_LIT})", code):
        left = m.group(1)
        if left.startswith("$") or left.strip("$") in money_names \
                or any(left.endswith(n) for n in money_names):
            return m.group(0)
    for m in re.finditer(rf"(?:{_DEC_LIT})\s*[*]\s*([\w$]+)", code):   # decimal LINKS
        right = m.group(1)
        if right.startswith("$") or right in money_names:
            return m.group(0)
    return None


def _praez_money_scaled(src: str, money_names: set) -> list:
    """Zeilen mit money [*/] decimal-Literal, die money produzieren. Rueckgabe
    (zeile, fragment, bound_var|None); bound_var aus `let`- ODER `definition`-Kopf."""
    hits = []
    for i, line in enumerate(src.splitlines(), 1):
        code = line.split("#", 1)[0]
        bm = re.match(r"\s*(?:let|definition)\s+(\w+)\s+equals\b", code)
        frag = _praez_scale_frag(code, money_names)
        if frag:
            hits.append((i, frag, bm.group(1) if bm else None))
    return hits


def praezisions_lint_gate(catala_src: str | None,
                          candidate: dict | None = None) -> GateResult:
    """Klasse-5-Lint: money x/ decimal, dessen money-Produkt (cent-gerundet) in einen
    finalen Cent-Schnitt fliesst -> Doppelrundung. Fluss-sensitiv: ein money x decimal,
    das NICHT in den Schnitt fliesst (Nachverarbeitung, Nebenvariable), kippt nicht.

    Stufe 1: confident-Befund -> INFO (kippt kein Gate). Stufe 2 (nach Julius): FAIL.
    """
    if not catala_src:
        return GateResult("praezisions_lint", SKIP, "kein A-Source")
    reachable = _praez_cut_reachable(catala_src)
    if reachable is None:
        return GateResult("praezisions_lint", PASS, "kein finaler Cent-Schnitt-Idiom")
    money_names = _praez_money_names(catala_src)
    scaled = _praez_money_scaled(catala_src, money_names)
    # money x decimal INLINE in der Schnitt-Expression X selbst (truncate of (X/$0.01)):
    # fliesst direkt in den Schnitt, ist immer confident.
    inline = [(0, f) for m in _CENT_CUT_RE.finditer(catala_src)
              for f in [_praez_scale_frag(m.group(1), money_names)] if f]
    if not scaled and not inline:
        return GateResult("praezisions_lint", PASS,
                          "money x decimal vor Cent-Schnitt: keine")
    # confident: bound_var fliesst (transitiv) in den Schnitt, ODER inline in X. Ein
    # money x decimal NACH dem Schnitt (Nachverarbeitung) oder in einer Nebenvariable
    # (bound_var NICHT in reachable) ist NICHT confident -> kein Flag (Fluss zaehlt).
    confident = inline + [(i, frag) for (i, frag, bv) in scaled
                          if bv is not None and bv in reachable]
    moeglich = [(i, frag) for (i, frag, bv) in scaled
                if not (bv is not None and bv in reachable)]
    if confident:
        z = "; ".join(f"Zeile {i}: {frag}" for i, frag in confident[:5])
        status = FAIL if _PRAEZISION_BLOCKIEREND else INFO
        vor = "" if _PRAEZISION_BLOCKIEREND else " (Stufe 1 informativ, wuerde in Stufe 2 blockieren)"
        return GateResult("praezisions_lint", status,
                          f"Klasse-5 Praezisions-Ordnung{vor}: {len(confident)} money x "
                          f"decimal rundet auf Cent VOR dem finalen Cent-Schnitt. Rechne "
                          f"den Prozentanteil in decimal (money per / $1.00 oder "
                          f"`decimal of` nach decimal), Cent-Schnitt am Ende, keine "
                          f"Zwischenrundung. {z}")
    if moeglich:
        z = "; ".join(f"Zeile {i}: {frag}" for i, frag in moeglich[:5])
        return GateResult("praezisions_lint", PASS,
                          f"money x decimal vorhanden, fliesst nicht nachweisbar in den "
                          f"Cent-Schnitt (Fluss ok oder manuell pruefen). {z}")
    return GateResult("praezisions_lint", PASS, "keine Klasse-5-Ordnung erkannt")


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
        "stille_zusatzannahmen": [_normalize_annahme(a)
                                  for a in (d.get("stille_zusatzannahmen") or [])],
        "scope_gap": [_normalize_gap(g) for g in (d.get("scope_gap") or [])],
    }


def _normalize_annahme(a) -> dict:
    """Akzeptiert den String von roundtrip_diff@3 und das Objekt von @4.

    Ein blosser String traegt keine Zuordnung zu einer Geltungsbedingung und gilt
    damit als `undeclared` - Schweigen darf nicht als Anrechnung durchgehen.
    """
    if isinstance(a, str):
        return {"annahme": a, "bedingung_id": None}
    bid = a.get("bedingung_id")
    return {"annahme": str(a.get("annahme", ""))[:400],
            "bedingung_id": bid if isinstance(bid, str) and bid.strip() else None}


def annahmen_mapping(verdict: dict, candidate: dict | None = None) -> tuple[list, list]:
    """(angerechnet, undeclared).

    Angerechnet wird eine Zusatzannahme nur, wenn der Judge sie EXPLIZIT der ID
    einer im Manifest deklarierten Geltungsbedingung zugeordnet hat. Eine ID, die
    es nicht gibt, zaehlt als undeclared: der Runner absorbiert nichts per
    Aehnlichkeit, und ein erfundenes Mapping wird nicht belohnt.
    """
    ids = {b.get("bedingung") for b in ((candidate or {}).get("geltungsbedingungen") or [])}
    angerechnet, undeclared = [], []
    for raw in verdict.get("stille_zusatzannahmen", []):
        # Gespeicherte Verdikte aelterer Laeufe tragen nackte Strings. Sie hier
        # erneut zu normalisieren ist billiger als die Annahme, jeder Aufrufer
        # habe schon geparst - und ein String zaehlt korrekt als undeclared.
        a = _normalize_annahme(raw)
        (angerechnet if a["bedingung_id"] in ids else undeclared).append(a)
    return angerechnet, undeclared


def _normalize_gap(gap) -> dict:
    """Accept both shapes: the plain strings of roundtrip_diff@2 and the
    classified objects of @3. An unclassified gap is treated as `wirkt_hinein`:
    the class that escalates. Silence must not read as harmless."""
    if isinstance(gap, str):
        return {"norm_teil": gap, "klasse": "unklassifiziert", "begruendung": ""}
    klasse = str(gap.get("klasse", "")).strip().lower()
    if klasse not in ("unabhaengig", "wirkt_hinein"):
        klasse = "unklassifiziert"
    d = {"norm_teil": str(gap.get("norm_teil", ""))[:400],
         "klasse": klasse,
         "begruendung": str(gap.get("begruendung", ""))[:400]}
    if "abgedeckt_von" in gap:
        d["abgedeckt_von"] = str(gap["abgedeckt_von"])[:120]
    return d


def wirkt_hinein(verdict: dict) -> list[dict]:
    """scope_gaps that change the result INSIDE the signature scope.

    Without such a norm part the formalised excerpt is wrong, not merely
    incomplete - so it escalates. `unklassifiziert` counts here too (a judge that
    omits the class must not thereby wave a gap through).
    """
    return [g for g in verdict.get("scope_gap", [])
            if g["klasse"] in ("wirkt_hinein", "unklassifiziert")]


def unabhaengige_gaps(verdict: dict) -> list[dict]:
    """scope_gaps that stand on their own -> Backlog, kein Flag."""
    return [g for g in verdict.get("scope_gap", []) if g["klasse"] == "unabhaengig"]


def roundtrip_gate(judge, candidate: dict | None = None) -> GateResult:
    """Abweichungen fallen immer. Zusatzannahmen fallen, solange sie undeclared sind.

    RESTRISIKO (Protokolldekret 2026-07-10, Punkt 5, woertlich): Nach der
    Vervollstaendigung der Bedingungslisten ist die verbleibende Aufgabe dieses
    Gates die Erkennung UNdeklarierter Annahmen, und dort bleibt Recall-Rauschen
    ein False-PASS-Risiko - eine Annahme, die alle Inventarlaeufe verpassen. Das
    Gate garantiert KEINE Vollstaendigkeit. Gegenlager: Union-until-Saturation,
    Golden-Tests, Human-Review.


    Eine deklarierte Annahme ist keine stille Annahme: ist sie vom Judge explizit
    der ID einer Geltungsbedingung zugeordnet, wird sie angerechnet. Das Mapping
    steht im Report und ist Teil des Reviews - eine Annahme, die nur vage zu einer
    breiten Bedingung passt, ist dort ein Reject des Mappings, kein PASS.
    """
    d = _verdict(judge)
    if d.get("parse_error"):
        return GateResult("roundtrip", FAIL, "judge output not valid JSON")
    if d.get("truncated"):
        return GateResult("roundtrip", FAIL, "judge answer truncated")
    angerechnet, undeclared = annahmen_mapping(d, candidate)
    gap = len(d["scope_gap"])
    if d["abweichungen"]:
        return GateResult("roundtrip", FAIL,
                          f"abweichungen={len(d['abweichungen'])}: "
                          f"{str(d['abweichungen'][0])[:110]}")
    if undeclared:
        return GateResult("roundtrip", FAIL,
                          f"{len(undeclared)} undeklarierte Zusatzannahme(n) "
                          f"(angerechnet: {len(angerechnet)}); erste: "
                          f"{undeclared[0]['annahme'][:110]}")
    return GateResult("roundtrip", PASS,
                      f"keine Abweichung; {len(angerechnet)} Zusatzannahme(n) einer "
                      f"deklarierten Geltungsbedingung zugeordnet; scope_gap={gap}")


def _verdict(judge) -> dict:
    """Akzeptiert rohen Judge-Text oder ein bereits geparstes Verdikt.

    `--regate` rechnet die deterministischen Gates aus gespeicherten Reports neu,
    ohne ein Modell erneut zu bezahlen; dort liegt das Verdikt als dict vor."""
    if not isinstance(judge, dict):
        return roundtrip_parse(judge)
    d = dict(judge)
    d["grenzfaelle"] = list(d.get("grenzfaelle") or [])
    d["scope_gap"] = [_normalize_gap(g) for g in (d.get("scope_gap") or [])]
    d["stille_zusatzannahmen"] = [_normalize_annahme(a)
                                  for a in (d.get("stille_zusatzannahmen") or [])]
    return d


def geltungsbereich_gate(judge, candidate: dict | None = None) -> GateResult:
    """Jede Teilformalisierung traegt ihren Geltungsbereich maschinenlesbar.

    Ein `wirkt_hinein`-Norm-Teil heisst: die Regel ist nur unter einer Bedingung
    richtig, die sie selbst nicht modelliert. § 9 Abs. 4a etwa besteht den
    amtlichen Test, gilt aber nur ohne Mahlzeitengestellung und nur im Inland.

    Solche Bedingungen muessen im Manifest als `geltungsbedingungen` deklariert
    sein - je eine mit `bedingung` (maschinenlesbarer Praedikatsname), `deckt_ab`
    (welchen scope_gap sie abdeckt) und `quelle`. Erst dann darf die Regel
    ueberhaupt zur Freigabe. Die Relevanzpropagation (Phase 5) stellt sie spaeter
    als Fragen; die Engine darf die Regel nicht feuern, wenn eine verletzt ist.

    Undeklarierte `wirkt_hinein`-Gaps -> FAIL. Ohne diese Kopplung waeren die
    Bedingungen eine Notiz, die beim naechsten Refactoring verschwindet.
    """
    d = _verdict(judge)
    if d.get("parse_error"):
        return GateResult("geltungsbereich", FAIL, "judge output not valid JSON")
    if d.get("truncated"):
        return GateResult("geltungsbereich", FAIL, "judge answer truncated")
    hinein = wirkt_hinein(d)
    if not hinein:
        return GateResult("geltungsbereich", PASS,
                          "kein Norm-Teil wirkt in den Signatur-Scope hinein")

    bedingungen = (candidate or {}).get("geltungsbedingungen") or []
    if not bedingungen:
        return GateResult("geltungsbereich", FAIL,
                          f"{len(hinein)} Norm-Teil(e) wirken hinein, aber keine "
                          f"`geltungsbedingungen` deklariert")

    ids = {b.get("bedingung") for b in bedingungen}
    # `deckt_ab` darf ein String oder eine Liste sein: eine Bedingung
    # ("keine Mahlzeitengestellung") deckt oft mehrere Norm-Teile ab.
    abgedeckt: list[str] = []
    for b in bedingungen:
        d = b.get("deckt_ab", "")
        abgedeckt += [_normalize(x) for x in (d if isinstance(d, list) else [d])]

    def gedeckt(g: dict) -> bool:
        # Der dekomponierte Judge nennt die abdeckende Bedingung explizit. Sobald
        # das Feld vorliegt, ist es autoritativ - auch sein "none". Sonst wuerde
        # der Textabgleich ein ausdrueckliches "nicht abgedeckt" ueberstimmen.
        # Eine ID, die es nicht gibt, deckt nichts ab: ein erfundenes Mapping darf
        # nichts durchwinken.
        if "abgedeckt_von" in g:
            av = g["abgedeckt_von"]
            return bool(av) and av != "none" and av in ids
        # Fallback fuer Verdikte des monolithischen Judge: Textabgleich.
        return any(a and a in _normalize(g["norm_teil"]) for a in abgedeckt)

    offen = [g for g in hinein if not gedeckt(g)]
    fehlende_felder = [b for b in bedingungen
                       if not (b.get("bedingung") and b.get("quelle"))]
    if fehlende_felder:
        return GateResult("geltungsbereich", FAIL,
                          f"{len(fehlende_felder)} Geltungsbedingung(en) ohne "
                          f"`bedingung` oder `quelle`")
    if offen:
        return GateResult("geltungsbereich", FAIL,
                          f"{len(offen)} von {len(hinein)} wirkt_hinein-Norm-Teil(en) "
                          f"nicht von einer Geltungsbedingung abgedeckt; erster: "
                          f"{offen[0]['norm_teil'][:100]}")
    return GateResult("geltungsbereich", PASS,
                      f"{len(hinein)} wirkt_hinein-Norm-Teil(e) durch "
                      f"{len(bedingungen)} Geltungsbedingung(en) abgedeckt")


def grenzfall_gate(judge) -> GateResult:
    """Objektiv ambige Items routen fest in die Review-Queue.

    Ein Item, das ueber Kampagnen hinweg wiederholt 2:1 gesplittet hat, wird nicht
    in jeder Messung neu ausgewuerfelt (Protokolldekret 2026-07-10, Punkt 4). Der
    Judge markiert es als Grenzfall; dieses Gate faellt dann, damit die Regel in den
    Human-Review geht und nicht mit einem zufaelligen Mehrheitswurf durchlaeuft.
    """
    d = _verdict(judge)
    gf = d.get("grenzfaelle") or []
    if gf:
        return GateResult("grenzfall", FAIL,
                          f"{len(gf)} objektiv ambige(s) Item(s) -> Review; "
                          f"erstes: {str(gf[0].get('norm_teil',''))[:110]}")
    return GateResult("grenzfall", PASS, "keine registrierten Dauersplitter")


def scope_gap_gate(judge) -> GateResult:
    """Informativ, nicht blockierend (Protokolldekret 2026-07-09).

    (a) `unabhaengig` -> Backlog-Item.
    (b) `wirkt_hinein` -> zaehlt hier nur. Blockierend ist `geltungsbereich`, das
        auf jeden wirkt_hinein OHNE abdeckende Geltungsbedingung faellt.

    Rationale: eine deklarierte Annahme ist keine stille Annahme. Wuerde
    scope_gap weiter auf jeden wirkt_hinein fallen, koennte keine korrekt
    spezifizierte Teilformalisierung je einen gruenen Zustand erreichen, und
    `flagged_for_review` verloere seine Bedeutung.
    """
    d = _verdict(judge)
    if d.get("parse_error"):
        return GateResult("scope_gap", FAIL, "judge output not valid JSON")
    hinein, unabh = wirkt_hinein(d), unabhaengige_gaps(d)
    return GateResult("scope_gap", PASS,
                      f"informativ: {len(hinein)} wirkt_hinein (blockierend ist "
                      f"geltungsbereich), {len(unabh)} unabhaengig -> Backlog")


_UMLAUT = str.maketrans({"ä": "ae", "ö": "oe", "ü": "ue", "ß": "ss",
                         "Ä": "ae", "Ö": "oe", "Ü": "ue"})


def _normalize(text: str) -> str:
    return re.sub(r"\s+", " ", text.translate(_UMLAUT).lower()).strip()


def _lit(value, typ: str) -> str | None:
    """Render a Catala literal. Unsupported type -> None."""
    if typ == "money":
        # Cent-Betraege muessen ueberleben: die amtlichen Rechenbeispiele nennen
        # 1.408,70 EUR und 22,40 EUR. `int(value)` haette daraus $1408.00 gemacht -
        # ein still falscher Erwartungswert, der genau das unterlaeuft, wofuer das
        # Test-Gate da ist. Catala-Money will Tausendertrenner: $1,408.70
        # Negative Betraege (Erstattung/Ueberschuss, z.B. § 36): das Minus steht in
        # Catala VOR dem Dollar (`-$3,000.00`), nicht dahinter - sonst Parse-Fehler.
        v = float(value)
        return f"{'-' if v < 0 else ''}${abs(v):,.2f}"
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
    LLM-invented expected values are never a test reference.

    In production the gate is MANDATORY (DoD je Regel): existiert kein
    EStH/BMF-Beispiel, muessen synthetische Faelle her, die Julius im Review
    absegnet. Nur ein Kandidat, der `test_gate_required: False` fuehrt, darf ohne
    Seeds durch - das war die G2-Ausnahme und ist in den G2-Metriken fussnotiert.
    """
    cand = candidate or {}
    seeds = cand.get("test_seed")
    if not seeds or seeds == "none":
        if cand.get("test_gate_required", True):
            return GateResult("clerk", FAIL,
                              "Test-Gate ist Pflicht: keine Test-Seeds. EStH/BMF-"
                              "Rechenbeispiel recherchieren (mit Zitatanker) oder "
                              "synthetische Faelle im Review absegnen lassen")
        return GateResult("clerk", SKIP,
                          "n/a (test_gate_required=false): kein EStH/BMF-"
                          "Rechenbeispiel; Aequivalenz + Round-Trip tragen "
                          "(keine LLM-Erwartungswerte)")
    if not catala_src or not module:
        return GateResult("clerk", FAIL, "no source/module")

    root = root or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    types = cand.get("input_types") or {}
    out_field = cand.get("output_field")
    scope = scope_name(catala_src)
    if not scope or not out_field:
        return GateResult("clerk", FAIL, "cannot determine scope/output_field")

    # 1. Herkunft + Zitatanker je Seed (deterministisch, ohne Modell)
    #
    #    amtlich      - der Erwartungswert steht woertlich im amtlichen Beispiel.
    #    abgeleitet   - die Rechenmechanik steht woertlich dort, die Parameter
    #                   stammen aus der eingefrorenen Fassung (z.B. § 24b: das
    #                   BMF-Beispiel rechnet mit 4.008 EUR, geltend sind 4.260).
    #    synthetisch  - kein amtliches Beispiel; Fall von Julius im Review
    #                   abgesegnet.
    #
    #    `rechenweg` ist fuer alles ausser `amtlich` Pflicht. Ohne ihn waere ein
    #    abgeleiteter Wert von einem erfundenen nicht zu unterscheiden - und genau
    #    das soll dieses Gate ausschliessen.
    for i, s in enumerate(seeds):
        herkunft = s.get("herkunft", "amtlich")
        if herkunft not in ("amtlich", "abgeleitet", "synthetisch"):
            return GateResult("clerk", FAIL, f"seed {i}: unbekannte herkunft {herkunft!r}")
        if herkunft != "amtlich" and not str(s.get("rechenweg", "")).strip():
            return GateResult("clerk", FAIL,
                              f"seed {i}: herkunft={herkunft} verlangt einen "
                              f"`rechenweg` (Herleitung des Erwartungswerts)")
        src = os.path.join(root, s["quelle"])
        if not os.path.exists(src):
            return GateResult("clerk", FAIL, f"seed {i}: source missing {s['quelle']}")
        if _normalize(s["zitatanker"]) not in _normalize(open(src, encoding="utf-8").read()):
            return GateResult("clerk", FAIL,
                              f"seed {i}: Zitatanker nicht im Quelltext ({s['quelle']})")

    # Erst hier wird die Toolchain gebraucht. Die Pruefungen oben (Herkunft,
    # Rechenweg, Zitatanker) sind deterministisch und laufen auch ohne clerk -
    # sonst haenge die Strenge des Gates am zufaellig gesetzten PATH.
    if not have("clerk"):
        return GateResult("clerk", SKIP, "clerk not on PATH (toolchain missing)")

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
