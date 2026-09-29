"""Pytest-Plugin: zeichnet jeden Aufruf einer `runner.catala_*`-Funktion mit ihren ROHEN
Argumenten auf (Sachverhalt-dict bzw. int), so wie die Produktschicht sie übergibt.

Anders als `record.py` (Scope-Ebene, fertig gerechnete Catala-Eingaben) prüft dieser Korpus
die Accessor-Schicht selbst: dict → Catala-Struct, `.get(k, 0)`-Defaults, `// 100`.

    python3 -m pytest tests/ -p tools.parity.record_runner -q -p no:randomly

Ausgabe: `rust/fixtures/corpus/runner/<fn>.jsonl`, je Zeile
`{"args": [...], "kwargs": {...}, "ok": <ergebnis>}` oder `{"args": ..., "err": "<Typ>"}`.
Identische Aufrufe werden einmal geschrieben. Die erste Zeile jeder Datei ist ein Kopf mit
`git describe --dirty` und sha256 von `git diff` (REWRITE_PLAN.md §5, Referenzstand).
Unter xdist schreibt jeder Worker eigene Dateien (`<fn>.<worker>.jsonl`).
"""

from __future__ import annotations

import functools
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ZIEL = ROOT / "rust" / "fixtures" / "corpus" / "runner"
_WORKER = os.environ.get("PYTEST_XDIST_WORKER", "")
# ponytail: feste Obergrenze je Funktion, damit der eingecheckte Korpus klein bleibt;
# bei Bedarf höher setzen (RECORD_MAX=…).
_MAX = int(os.environ.get("RECORD_MAX", "3000"))

_faelle: dict[str, dict[str, dict]] = {}


def _json_fest(obj):
    """Nur JSON-darstellbare Argumente werden aufgezeichnet; alles andere → None (übersprungen)."""
    try:
        return json.loads(json.dumps(obj, sort_keys=True, ensure_ascii=False))
    except (TypeError, ValueError):
        return None


def _wickle(name: str, fn):
    @functools.wraps(fn)
    def gewickelt(*args, **kwargs):
        # Innere Aufrufe (catala_est → catala_gesamt) werden je Funktion mit aufgezeichnet.
        try:
            ergebnis = fn(*args, **kwargs)
        except Exception as e:
            _merke(name, args, kwargs, {"err": type(e).__name__})
            raise
        _merke(name, args, kwargs, {"ok": ergebnis})
        return ergebnis

    gewickelt.__parity_gewickelt__ = True
    return gewickelt


def _merke(name, args, kwargs, ausgang):
    eintrag = _json_fest({"args": list(args), "kwargs": kwargs, **ausgang})
    if eintrag is None:
        return
    je_fn = _faelle.setdefault(name, {})
    if len(je_fn) >= _MAX:
        return
    schluessel = json.dumps(eintrag, sort_keys=True, ensure_ascii=False)
    je_fn.setdefault(hashlib.sha256(schluessel.encode()).hexdigest(), eintrag)


def _runner_module():
    return [m for n, m in list(sys.modules.items())
            if m is not None and n in ("runner", "produkt.engine.runner")]


def _patche(mod):
    for name in dir(mod):
        if not name.startswith("catala_"):
            continue
        fn = getattr(mod, name)
        if callable(fn) and not getattr(fn, "__parity_gewickelt__", False):
            setattr(mod, name, _wickle(name, fn))


def pytest_configure(config):
    sys.path.insert(0, str(ROOT / "produkt" / "engine"))
    import runner  # noqa: F401  (lädt das Modul, damit es gepatcht werden kann)
    for mod in _runner_module():
        _patche(mod)


def pytest_collection_finish(session):
    # Testmodule können `runner` ein zweites Mal unter anderem Namen laden (conftest hängt
    # produkt/engine bar in sys.path) — nach dem Sammeln erneut patchen.
    for mod in _runner_module():
        _patche(mod)


def pytest_runtest_setup(item):
    for mod in _runner_module():
        _patche(mod)


def _kopf():
    def lauf(*cmd):
        return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True).stdout
    return {"kopf": True,
            "git_describe": lauf("git", "describe", "--always", "--dirty").strip(),
            "git_diff_sha256": hashlib.sha256(lauf("git", "diff").encode()).hexdigest()}


def pytest_unconfigure(config):
    if not _faelle:
        return
    ZIEL.mkdir(parents=True, exist_ok=True)
    kopf = json.dumps(_kopf(), ensure_ascii=False)
    suffix = f".{_WORKER}" if _WORKER else ""
    for name, je_fn in sorted(_faelle.items()):
        pfad = ZIEL / f"{name}{suffix}.jsonl"
        with open(pfad, "w", encoding="utf-8") as f:
            f.write(kopf + "\n")
            for schluessel in sorted(je_fn):
                f.write(json.dumps(je_fn[schluessel], sort_keys=True, ensure_ascii=False) + "\n")
