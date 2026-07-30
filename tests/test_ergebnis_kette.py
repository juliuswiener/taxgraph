"""P5.4 Ergebnis-Kette Naht-Test: catala_gesamt_kette Cent-Gleichung."""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R


def test_kette_cent_gleichung_kinderlos():
    """Naht: kette[festzusetzende_est] * 100 == catala_gesamt() * 100 (Cent-Match)."""
    s = {
        "veranlagungszeitraum": 2025,
        "veranlagung": "einzel",
        "einkuenfte_nichtselbststaendig": 60000,
    }
    kette = R.catala_gesamt_kette(s)
    zahl_euro = R.catala_gesamt(s)

    # Kette vorhanden
    assert kette is not None, "kette=None"
    assert isinstance(kette, dict), f"kette type={type(kette)}"

    # 4 Felder vorhanden
    assert "gesamtbetrag_der_einkuenfte" in kette
    assert "zu_versteuerndes_einkommen" in kette
    assert "tarifliche_est" in kette
    assert "festzusetzende_est" in kette

    # Zentrale Gleichung: kette-letzte-Stufe == zahl_euro
    assert kette["festzusetzende_est"] == zahl_euro, \
        f"kette[festzusetzende_est]={kette['festzusetzende_est']} " \
        f"!= catala_gesamt()={zahl_euro}"


def test_kette_zusammen_mit_kapital():
    """Zusammenveranlagung mit Kapitaleinkünften."""
    s = {
        "veranlagungszeitraum": 2025,
        "veranlagung": "zusammen",
        "einkuenfte_nichtselbststaendig": 80000,
        "einkuenfte_kapitalvermoegen": 10000,
    }
    kette = R.catala_gesamt_kette(s)
    zahl_euro = R.catala_gesamt(s)

    assert kette["festzusetzende_est"] == zahl_euro
