"""Gate für den except-Block in `_zweig_festzusetzende_est_gesamt`
(produkt/bescheid/bescheid_zweige.py) — Backlog stiller-except-block-ohne-log.md /
kegel-sweep-meldet-falschen-abbruchgrund.md.

VORHER: `except Exception: return None` unterschied nicht "Paket fehlt" (ImportError/
ModuleNotFoundError, dokumentiertes Verhalten) von "Bug beim Laden von runner.py" — und
schrieb in KEINEM der beiden Fälle eine Log-Zeile.

NACHHER: ImportError/ModuleNotFoundError bleibt ein stilles `None` für den Nutzer (Zweig
bricht nicht ab), protokolliert aber den Grund. Jede ANDERE Exception bleibt ebenfalls
`None` (kein Fail-Open auf den Nutzerpfad), aber unterscheidbar im Protokoll: eigene
Exception-Klasse, Stufe FEHLER statt WARNUNG.

Einziger Mock: `builtins.__import__` für den Namen "runner" — sonst echter Live-Import,
kein Mock-LLM, kein Store-Mock.
"""
from __future__ import annotations

import builtins
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/eingang", "produkt/bescheid",
             "produkt/engine", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import audit                    # noqa: E402
import bescheid_zweige as BZ    # noqa: E402
import fehler_log as FL         # noqa: E402


def _patch_import_von_runner(monkeypatch, exc: Exception) -> None:
    """`import runner` scheitert mit `exc`, jeder andere Import läuft echt weiter."""
    echt_import = builtins.__import__

    def _gefaelscht(name, *a, **kw):
        if name == "runner":
            raise exc
        return echt_import(name, *a, **kw)

    monkeypatch.setattr(builtins, "__import__", _gefaelscht)


def _rufe_zweig() -> object:
    """Minimaler Aufruf — der except-Block greift vor jedem Zugriff auf felder/store."""
    return BZ._zweig_festzusetzende_est_gesamt(2025, {}, None, None, False, None, None)


def test_paket_fehlt_bleibt_still_aber_geloggt(tmp_path, monkeypatch):
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path))
    _patch_import_von_runner(monkeypatch, ModuleNotFoundError("No module named 'pkg'"))

    ergebnis = _rufe_zweig()

    assert ergebnis is None    # dokumentiertes Verhalten: Nutzer bekommt "liegt an der Software"
    eintraege = FL.lies()
    assert len(eintraege) == 1, "genau eine Log-Zeile, nicht null"
    assert eintraege[0]["typ"] == "ModuleNotFoundError"
    assert eintraege[0]["stufe"] == "WARNING"   # erwarteter Ausfall, kein echter Bug
    assert "runner" in eintraege[0]["ort"]


def test_anderer_fehler_bleibt_ebenfalls_still_aber_unterscheidbar_geloggt(tmp_path, monkeypatch):
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path))
    _patch_import_von_runner(monkeypatch, NameError("name 'x' is not defined"))

    ergebnis = _rufe_zweig()

    assert ergebnis is None    # kein Fail-Open auf den Nutzerpfad — bleibt None wie zuvor
    eintraege = FL.lies()
    assert len(eintraege) == 1, "genau eine Log-Zeile, nicht null"
    # unterscheidbar vom ModuleNotFoundError-Fall oben: eigene Klasse, höhere Stufe
    assert eintraege[0]["typ"] == "NameError"
    assert eintraege[0]["stufe"] == "ERROR"
    assert "runner" in eintraege[0]["ort"]
