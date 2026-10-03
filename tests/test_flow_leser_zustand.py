"""Der Mitschnitt-Leser (scripts/flow.py) kennzeichnet vorläufige Werte als solche.

Vault: decisions/mitschnitt-leser-zeigt-vorlaeufig-als-solches (Instructor 2026-10-02). Der Schreiber
(`produkt/haut/api.py`, `flow.schreibe(..., "antwort", {...})`) schreibt sechs Werte; der Leser druckte fünf.
`zustand` fehlte, ein vorläufiger und ein bestätigter Wert erschienen als dieselbe Zeile, und der Kommentar im
Leser versprach „`zustand` folgt mit dem Leser". Jetzt hängt `zeige()` bei `vorlaeufig` das Suffix
`(vorläufig)` an; die Zeile für `bestaetigt` bleibt byte-gleich.

Kein anderer Test lädt `scripts/flow.py` (es ist ein Skript, kein Paket): hier per `importlib`, unter einem
eigenen Namen, weil `flow` schon `produkt/haut/flow.py` (der Schreiber des Mitschnitts) heisst.
"""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import api_auth          # noqa: E402
import audit             # noqa: E402


def _leser():
    spec = importlib.util.spec_from_file_location("flow_leser", os.path.join(ROOT, "scripts", "flow.py"))
    modul = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(modul)
    return modul


LESER = _leser()

# Die Zeile für einen bestätigten Wert, wie der Leser sie VOR dem Fix druckte (gelaufen am Stand 4b2cf0d2).
BESTAETIGT = "12:34:56  → Fragebogen             ep_arbeitstage                           = 220\n"


def _antwort(**inhalt):
    return {"art": "antwort", "ts": "2026-10-03T12:34:56+00:00", "fall": "f",
            "inhalt": {"feld_id": "ep_arbeitstage", "wert": 220, "weg": "klick@ep_arbeitstage",
                       "ersetzt": False, "schreiber": "ui:laie", **inhalt}}


def _ausgabe(eintraege):
    puffer = io.StringIO()
    with contextlib.redirect_stdout(puffer):
        LESER.zeige(eintraege)
    return puffer.getvalue()


def test_bestaetigt_bleibt_byte_gleich():
    assert _ausgabe([_antwort(zustand="bestaetigt")]) == BESTAETIGT


def test_vorlaeufig_traegt_das_suffix():
    assert _ausgabe([_antwort(zustand="vorlaeufig")]) == BESTAETIGT.rstrip("\n") + "  (vorläufig)\n"


def test_ersetzt_und_vorlaeufig_stehen_beide_da():
    zeile = _ausgabe([_antwort(zustand="vorlaeufig", ersetzt=True)])
    assert zeile == BESTAETIGT.rstrip("\n") + "  (ersetzt)  (vorläufig)\n"


def test_ein_alter_mitschnitt_ohne_zustand_bleibt_wie_er_war():
    """Zeilen aus der Zeit vor `zustand` und jeder andere Wert als `vorlaeufig` bekommen kein Suffix:
    der Leser rät nichts."""
    assert _ausgabe([_antwort()]) == BESTAETIGT
    assert _ausgabe([_antwort(zustand=None)]) == BESTAETIGT


@pytest.fixture
def mitschnitt(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", "pruefer")
    monkeypatch.setenv("TAXGRAPH_FLOW", "1")
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "leser-zustand"})
    return tmp_path / "faelle" / "flow.jsonl"


def test_der_echte_schreiber_und_der_leser_meinen_dasselbe_wort(mitschnitt):
    """Der Leser vergleicht mit dem Wort `vorlaeufig`; hier schreibt der ECHTE Schreiber einen vorläufigen und
    einen bestätigten Wert (ui:laie, der katalogfreie Kanal), und nur die vorläufige Zeile trägt das Suffix."""
    herkunft = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
    for feld, wert, zustand, signal_2 in (("kein_kap", True, "vorlaeufig", None),
                                          ("kein_gewinn", True, "bestaetigt", "klick@kein_gewinn")):
        API.event("leser-zustand", {"feld_id": feld, "wert": wert, "zustand": zustand, "herkunft": herkunft,
                                    "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": signal_2}})
    eintraege = [json.loads(z) for z in open(mitschnitt, encoding="utf-8") if z.strip()]
    assert [(e["inhalt"]["feld_id"], e["inhalt"]["zustand"]) for e in eintraege if e["art"] == "antwort"] == [
        ("kein_kap", "vorlaeufig"), ("kein_gewinn", "bestaetigt")]
    zeilen = _ausgabe(eintraege).splitlines()
    assert [z.endswith("(vorläufig)") for z in zeilen if "→" in z] == [True, False]
