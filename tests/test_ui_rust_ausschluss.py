"""Die Ausschlussliste von `make ui-rust` (tools/ui_rust/ausschluss.tsv) ist gueltig und nicht veraltet.

`make ui-rust` braucht Playwright, Chromium und ein gebautes Rust-Binary; CI hat nichts davon. Diese
Pruefung braucht nichts davon: sie liest die Liste und den Quelltext der Testdateien. So faellt ein
Eintrag auf, dessen Test umbenannt oder geloescht wurde, schon in `make unit` und nicht erst im
naechsten Rust-Lauf. Dass ein gelisteter Test GRUEN wird, meldet nur der Lauf selbst (xfail strict).
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools", "ui_rust"))

import ui_rust_plugin as P  # noqa: E402


def test_die_echte_liste_ist_lesbar_und_nicht_veraltet():
    eintraege = P.lies_ausschluss()
    assert eintraege, "Die Liste ist leer: dann braucht ui-rust keinen Ausschluss mehr, und diese Datei auch nicht."
    assert set(eintraege.values()) <= set(P.URSACHEN)
    assert P._veraltete_eintraege(eintraege) == []


def test_jeder_gelistete_test_liegt_in_einer_ui_datei():
    """Der Lauf nimmt nur Dateien mit `sync_playwright`; ein Eintrag woanders wird nie angewendet."""
    for nodeid in P.lies_ausschluss():
        datei = nodeid.split("::")[0]
        with open(os.path.join(ROOT, datei), encoding="utf-8") as f:
            assert "sync_playwright" in f.read(), f"{nodeid}: {datei} ist keine UI-Datei"


def test_ein_eintrag_ohne_test_wird_als_veraltet_erkannt(tmp_path):
    ziel = "tests/test_ui_login.py"
    assert P._veraltete_eintraege({f"{ziel}::test_anmeldemaske_erscheint_wenn_auth_pflicht_ist": "LLM-Stub"}) == []
    assert P._veraltete_eintraege({f"{ziel}::test_gibt_es_nicht": "LLM-Stub"}) == [
        f"{ziel}::test_gibt_es_nicht: Test fehlt in der Datei"
    ]
    assert P._veraltete_eintraege({"tests/test_gibt_es_nicht.py::test_x": "LLM-Stub"}) == [
        "tests/test_gibt_es_nicht.py::test_x: Datei fehlt"
    ]


@pytest.mark.parametrize("zeile", [
    "tests/test_ui_login.py::test_x\tUnbekannt",       # Ursache nicht in URSACHEN
    "tests/test_ui_login.py::test_x",                   # keine Ursache
    "tests/test_ui_login.py::test_x\tLLM-Stub\tmehr",   # zu viele Spalten
])
def test_eine_fehlerhafte_zeile_bricht_den_lauf_ab(tmp_path, zeile):
    liste = tmp_path / "liste.tsv"
    liste.write_text(f"# Kommentar\n{zeile}\n", encoding="utf-8")
    with pytest.raises(pytest.UsageError):
        P.lies_ausschluss(liste)


def test_ein_doppelter_eintrag_bricht_den_lauf_ab(tmp_path):
    liste = tmp_path / "liste.tsv"
    liste.write_text("a.py::t\tLLM-Stub\na.py::t\tERiC-Stub\n", encoding="utf-8")
    with pytest.raises(pytest.UsageError, match="zweimal"):
        P.lies_ausschluss(liste)
