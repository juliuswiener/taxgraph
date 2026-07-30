"""§ 3 Nr. 72 EStG — Photovoltaik-Steuerbefreiung (Accessor).

Quelle: sources/gesetze-im-internet/estg_p3_2026-07-30.txt
Wortlaut S. 1: „…wenn die installierte Bruttoleistung laut Marktstammdatenregister bis zu
30 Kilowatt (peak) je Wohn- oder Gewerbeeinheit und insgesamt höchstens 100 Kilowatt (peak)
pro Steuerpflichtigem oder Mitunternehmerschaft beträgt."

Zwei Grenzen, beide einschließend („bis zu" / „höchstens"), beide müssen erfüllt sein.
Es ist eine FREIGRENZE: eine Überschreitung macht die Einnahmen voll steuerpflichtig.
"""
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "golden"))

import runner as R   # noqa: E402

AUF_GEBAEUDE = {"pv_auf_gebaeude": {"wert": True}}


def _s(**kw):
    return {**AUF_GEBAEUDE, **kw}


# ----------------------------------------------------------------- Grenzen einschließend

@pytest.mark.parametrize("leistung,einheiten,erwartet", [
    (30, 1, 5000),      # exakt 30 kWp bei 1 Einheit — „bis zu" schließt ein
    (29, 1, 5000),      # darunter
    (31, 1, 0),         # 1 kWp über der Je-Einheit-Grenze → Freigrenze reißt
    (60, 2, 5000),      # 2 Einheiten × 30 kWp = exakt an der Grenze
    (61, 2, 0),         # darüber
    (100, 4, 5000),     # exakt 100 kWp gesamt — „höchstens" schließt ein
    (101, 4, 0),        # 1 kWp über der Gesamt-Grenze
])
def test_leistungsgrenzen(leistung, einheiten, erwartet):
    got = R.catala_p3_nr72_photovoltaik(
        _s(pv_einnahmen=5000, pv_bruttoleistung_kwp=leistung, pv_anzahl_einheiten=einheiten))
    assert got == erwartet


def test_beide_grenzen_gelten_kumulativ():
    """5 Einheiten erlaubten 150 kWp je-Einheit — die 100-kWp-Gesamtgrenze bleibt bindend."""
    assert R.catala_p3_nr72_photovoltaik(
        _s(pv_einnahmen=9000, pv_bruttoleistung_kwp=120, pv_anzahl_einheiten=5)) == 0
    assert R.catala_p3_nr72_photovoltaik(
        _s(pv_einnahmen=9000, pv_bruttoleistung_kwp=100, pv_anzahl_einheiten=5)) == 9000


def test_freigrenze_nicht_freibetrag():
    """Über der Grenze wird NICHT bis 30 kWp anteilig befreit — alles ist steuerpflichtig."""
    assert R.catala_p3_nr72_photovoltaik(
        _s(pv_einnahmen=8000, pv_bruttoleistung_kwp=35, pv_anzahl_einheiten=1)) == 0


# ----------------------------------------------------------------- Tatbestand

def test_ohne_gebaeude_keine_befreiung():
    """S. 1 verlangt „auf, an oder in Gebäuden" — Freiflächenanlagen fallen nicht darunter."""
    assert R.catala_p3_nr72_photovoltaik(
        {"pv_einnahmen": 5000, "pv_bruttoleistung_kwp": 20, "pv_anzahl_einheiten": 1,
         "pv_auf_gebaeude": {"wert": False}}) == 0


def test_gebaeude_flag_fehlt_ist_fail_closed():
    """Ohne bestätigtes Gebäude-Merkmal keine Befreiung (kein stiller Grant)."""
    assert R.catala_p3_nr72_photovoltaik(
        {"pv_einnahmen": 5000, "pv_bruttoleistung_kwp": 20, "pv_anzahl_einheiten": 1}) == 0


def test_ohne_leistungsangabe_keine_befreiung():
    """Ohne kWp lässt sich die Grenze nicht prüfen → fail-closed statt stiller Befreiung."""
    assert R.catala_p3_nr72_photovoltaik(_s(pv_einnahmen=5000, pv_anzahl_einheiten=1)) == 0


def test_ohne_einheiten_keine_befreiung():
    assert R.catala_p3_nr72_photovoltaik(_s(pv_einnahmen=5000, pv_bruttoleistung_kwp=20)) == 0


def test_keine_einnahmen_kein_abzug():
    assert R.catala_p3_nr72_photovoltaik(
        _s(pv_einnahmen=0, pv_bruttoleistung_kwp=20, pv_anzahl_einheiten=1)) == 0


def test_leerer_input():
    assert R.catala_p3_nr72_photovoltaik({}) == 0


def test_negative_einnahmen_kein_abzug():
    """Ein Verlust ist keine steuerfreie Einnahme — kein negativer Abzug."""
    assert R.catala_p3_nr72_photovoltaik(
        _s(pv_einnahmen=-500, pv_bruttoleistung_kwp=20, pv_anzahl_einheiten=1)) == 0


def test_befreiter_betrag_ist_die_volle_einnahme():
    """Die Rückgabe ist der abzuziehende Betrag, nicht ein Prozentsatz davon."""
    assert R.catala_p3_nr72_photovoltaik(
        _s(pv_einnahmen=12345, pv_bruttoleistung_kwp=15, pv_anzahl_einheiten=1)) == 12345
