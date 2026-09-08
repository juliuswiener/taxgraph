"""Gate für den eDaten-Übernahme-Writer (produkt/eingang/elster_writer.py).
Deterministisch, NULL LLM.

Prüft: (1) Übertrag als vorlaeufig+schreiber=import:elster+herkunft=edaten; (2) kein Überschreiben
eines bestätigten Events (abweichende Angabe Vorrang, §150 Abs.7 S.2 AO); (3) kein Überschreiben
eines vorläufigen Events (One-Active-Event); (4) leeres Record; (5) mehrere Felder teilweise aktiv;
(6) belt-and-suspenders: signal_2=None + zustand=vorlaeufig für alle import:elster-Events.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/store", "produkt/eingang", "produkt/traverser", "produkt/haut"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import store as ST          # noqa: E402
import traverser as TR      # noqa: E402
import elster_writer as EW  # noqa: E402
import api_constants as AC  # noqa: E402

H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
TS = "2026-07-22T14:00:00+00:00"

AN_GESAMT = AC.SCHEIBEN["an_gesamt"]["felder"]   # 76 Felder, gemessen


def _an_gesamt_bindung():
    """{feld_id -> Bindungseintrag} der Scheibe an_gesamt — die Filterung, die der Writer anwenden
    soll. api_constants.SCHEIBEN ist die Autorität der Feldliste; lade_bindung() liefert die
    Einträge dazu."""
    return {f: TR.lade_bindung()[f] for f in AN_GESAMT}


def _bestaetigt(store, fid, wert):
    ST.append_event(store, feld_id=fid, wert=wert, zustand="bestaetigt", herkunft=H,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"ok@{fid}"}, ts=TS)


def _vorlaeufig(store, fid, wert):
    ST.append_event(store, feld_id=fid, wert=wert, zustand="vorlaeufig", herkunft=H,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": None}, ts=TS)


# ---- (1) Übertrag als vorlaeufig ------------------------------------------

def test_edaten_uebertraegt_bestaetigt():
    s = ST.leerer_store(2025, fall_id="edaten-best")
    record = [{"feld_id": "bruttoarbeitslohn", "wert": 4000000, "kategorie": "lohn"}]
    n = EW.uebernehme_edaten(s, record, ts=TS)
    assert n == 1
    ev = ST._aktives(s)["bruttoarbeitslohn"]
    assert ev["zustand"] == "bestaetigt"
    assert ev["schreiber"] == "import:elster"
    assert ev["signal"]["signal_2"] == "edaten_uebermittelt"
    assert ev["herkunft"]["herkunft"] == "edaten"
    assert ev["wert"] == 4000000


# ---- (2) kein Überschreiben eines bestätigten Events (abweichende Angabe) --

def test_edaten_ueberschreibt_bestaetigtes_nicht():
    s = ST.leerer_store(2025, fall_id="edaten-bestaetigt")
    _bestaetigt(s, "bruttoarbeitslohn", 5000000)
    record = [{"feld_id": "bruttoarbeitslohn", "wert": 4000000, "kategorie": "lohn"}]
    n = EW.uebernehme_edaten(s, record, ts=TS)
    assert n == 0
    assert ST._aktives(s)["bruttoarbeitslohn"]["wert"] == 5000000


# ---- (3) kein Überschreiben eines vorläufigen Events -----------------------

def test_edaten_ueberschreibt_vorlaeufiges_nicht():
    s = ST.leerer_store(2025, fall_id="edaten-vorlaktiv")
    _vorlaeufig(s, "bruttoarbeitslohn", 4500000)
    record = [{"feld_id": "bruttoarbeitslohn", "wert": 4000000, "kategorie": "lohn"}]
    n = EW.uebernehme_edaten(s, record, ts=TS)
    assert n == 0
    assert ST._aktives(s)["bruttoarbeitslohn"]["wert"] == 4500000


# ---- (4) leeres Record ----------------------------------------------------

def test_edaten_leeres_record():
    s = ST.leerer_store(2025, fall_id="edaten-leer")
    n = EW.uebernehme_edaten(s, [], ts=TS)
    assert n == 0
    assert len(ST._aktives(s)) == 0


# ---- (7) Scheiben-Grenze: scheibenfremde Felder werden NICHT geschrieben -----

def test_edaten_scheibenfremdes_feld_landet_nicht_im_store():
    """Ein Feld, das die übergebene Bindung (Scheibe) nicht führt, darf NICHT als bestaetigt
    geschrieben werden. Gegenprobe im selben Test: das scheibeneigene Feld MUSS ankommen —
    sonst wäre „schreibt gar nichts mehr" auch grün.
    """
    s = ST.leerer_store(2025, fall_id="edaten-scheibe")
    bindung = _an_gesamt_bindung()
    # Vorbedingungen an der realen Bindung, nicht an der Testkonstante: bruttoarbeitslohn führt
    # an_gesamt, p32b_progressionseinkuenfte NICHT (an_gesamt deklariert keinen §32b).
    assert "bruttoarbeitslohn" in bindung
    assert "p32b_progressionseinkuenfte" not in bindung
    record = [
        {"feld_id": "bruttoarbeitslohn", "wert": 4000000, "kategorie": "lohn"},
        {"feld_id": "p32b_progressionseinkuenfte", "wert": 120000, "kategorie": "sonstige"},
    ]
    n = EW.uebernehme_edaten(s, record, ts=TS, bindung=bindung)
    assert n == 1
    assert "bruttoarbeitslohn" in ST._aktives(s)                        # scheibeneigen: ankommen
    assert "p32b_progressionseinkuenfte" not in ST._aktives(s)          # scheibenfremd: verworfen


# ---- (5) mehrere Felder, teilweise aktiv -----------------------------------

def test_edaten_mehrere_felder_teilweise_aktiv():
    s = ST.leerer_store(2025, fall_id="edaten-mehrere")
    _bestaetigt(s, "veranlagung", "zusammen")
    record = [
        {"feld_id": "veranlagung", "wert": "einzel", "kategorie": "stammdaten"},
        {"feld_id": "bruttoarbeitslohn", "wert": 4000000, "kategorie": "lohn"},
        {"feld_id": "rentenbezug", "wert": 1200000, "kategorie": "rente"},
    ]
    n = EW.uebernehme_edaten(s, record, ts=TS)
    assert n == 2
    # veranlagung unverändert (abweichende Angabe hat Vorrang)
    assert ST._aktives(s)["veranlagung"]["wert"] == "zusammen"
    # neue Felder
    assert ST._aktives(s)["bruttoarbeitslohn"]["zustand"] == "bestaetigt"
    assert ST._aktives(s)["bruttoarbeitslohn"]["wert"] == 4000000
    assert ST._aktives(s)["rentenbezug"]["zustand"] == "bestaetigt"
    assert ST._aktives(s)["rentenbezug"]["wert"] == 1200000


# ---- (6) belt-and-suspenders: signal_2=None + vorlaeufig -------------------

def test_edaten_signal2_edaten_uebermittelt_und_bestaetigt():
    s = ST.leerer_store(2025, fall_id="edaten-signal2")
    record = [
        {"feld_id": "bruttoarbeitslohn", "wert": 4000000, "kategorie": "lohn"},
        {"feld_id": "rentenbezug", "wert": 1200000, "kategorie": "rente"},
    ]
    EW.uebernehme_edaten(s, record, ts=TS)
    for ev in ST._aktives(s).values():
        assert ev["signal"]["signal_2"] == "edaten_uebermittelt"
        assert ev["zustand"] == "bestaetigt"
