"""Gate für das Verhalten der `vorjahr`-Kategorie `uebernehmbar`/`vorschlag` (Julius, 2026-09-05,
BACKLOG vorjahr-kategorie-ohne-verhalten). Deterministisch, NULL LLM, keine Mocks für die Bindung
(TR.lade_bindung() lädt die echten bindung_*.yaml).

Entscheidung: `uebernehmbar` (Stammdaten/kohortenfix) wird OHNE Rückfrage aus dem Vorjahr
übernommen -- verschwindet aus `naechste_fragen`, bleibt aber über /event korrigierbar (das
Event selbst bleibt `vorlaeufig`, der Store-Guard ^import:vorjahr lässt es nie anders -- nur die
Interview-Queue wird gefiltert). `vorschlag` (jahres-spezifischer Betrag) bleibt eine Frage, trägt
aber `vorjahr_kategorie` in der Antwort, damit die Oberfläche sie auffälliger markieren kann.

Felder: `rentner_renten_beginn_jahr` (vorjahr: uebernehmbar) und `rentner_jahresrente`
(vorjahr: vorschlag) aus bindung_rentner.yaml, dieselbe Regel (p22_1_leibrente_besteuerungsanteil).
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/store", "produkt/traverser", "produkt/eingang", "produkt/haut"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import store as ST      # noqa: E402
import traverser as TR   # noqa: E402
import vorjahr_writer as VW  # noqa: E402
import api as API        # noqa: E402

H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
TS = "2026-09-05T09:00:00+00:00"


def _bestaetigt(store, fid, wert):
    ST.append_event(store, feld_id=fid, wert=wert, zustand="bestaetigt", herkunft=H,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"ok@{fid}"}, ts=TS)


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def test_bindung_traegt_die_erwarteten_kategorien(bindung):
    assert bindung["rentner_renten_beginn_jahr"]["vorjahr"] == "uebernehmbar"
    assert bindung["rentner_jahresrente"]["vorjahr"] == "vorschlag"


# ---- Test A: uebernehmbar + Vorjahreswert -> verschwindet aus fragen ----------

def test_uebernehmbar_mit_vorjahreswert_verschwindet_aus_fragen(bindung):
    vj = ST.leerer_store(2024, fall_id="vjk-a-alt")
    _bestaetigt(vj, "rentner_renten_beginn_jahr", 2015)
    vj_felder, _ = ST.materialisiere(vj)
    neu = ST.leerer_store(2025, fall_id="vjk-a-neu")
    n = VW.uebernehme_vorjahr(neu, vj_felder, bindung, vorjahr_vz=2024, ts=TS)
    assert n >= 1
    fragen = TR.naechste_fragen(neu, bindung)
    assert "rentner_renten_beginn_jahr" not in fragen
    nf, _ = ST.materialisiere(neu)
    assert nf["rentner_renten_beginn_jahr"]["wert"] == 2015
    assert nf["rentner_renten_beginn_jahr"]["herkunft"]["herkunft"] == "vorjahr"


# ---- Test B: vorschlag + Vorjahreswert -> bleibt Frage, trägt Kategorie -------

def test_vorschlag_mit_vorjahreswert_bleibt_frage_mit_kategorie(bindung):
    vj = ST.leerer_store(2024, fall_id="vjk-b-alt")
    _bestaetigt(vj, "rentner_jahresrente", 1800000)
    vj_felder, _ = ST.materialisiere(vj)
    neu = ST.leerer_store(2025, fall_id="vjk-b-neu")
    n = VW.uebernehme_vorjahr(neu, vj_felder, bindung, vorjahr_vz=2024, ts=TS)
    assert n >= 1
    fragen = TR.naechste_fragen(neu, bindung)
    assert "rentner_jahresrente" in fragen
    meta = API._frage_metadaten("rentner_jahresrente", bindung, neu)
    assert meta["vorjahr_kategorie"] == "vorschlag"


# ---- Test C: uebernehmbar OHNE Vorjahreswert bleibt Frage (Regressionsschutz) --

def test_uebernehmbar_ohne_vorjahreswert_bleibt_frage(bindung):
    neu = ST.leerer_store(2025, fall_id="vjk-c-neu")
    fragen = TR.naechste_fragen(neu, bindung)
    assert "rentner_renten_beginn_jahr" in fragen
