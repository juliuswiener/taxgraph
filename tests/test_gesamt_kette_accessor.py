"""catala_gesamt_kette — Rechenweg-Kette Consistency."""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402

VZ = 2025


def test_kette_gde_gleich_accessor():
    """Kette[GdE] == catala_gesamt_gde()."""
    s = {"veranlagungszeitraum": VZ, "einkuenfte_nichtselbststaendig": 60000,
         "einkuenfte_kapitalvermoegen": 5000, "veranlagung": "einzel"}
    kette = R.catala_gesamt_kette(s)
    accessor = R.catala_gesamt_gde(s)
    assert kette["gesamtbetrag_der_einkuenfte"] == accessor


def test_kette_zve_gleich_accessor():
    """Kette[zvE] == catala_gesamt_zve()."""
    s = {"veranlagungszeitraum": VZ, "einkuenfte_nichtselbststaendig": 50000,
         "sonderausgaben": 2000, "veranlagung": "einzel"}
    kette = R.catala_gesamt_kette(s)
    accessor = R.catala_gesamt_zve(s)
    assert kette["zu_versteuerndes_einkommen"] == accessor


def test_kette_tariflich_gleich_accessor():
    """Kette[tarifliche_est] == catala_gesamt_tarifliche()."""
    s = {"veranlagungszeitraum": VZ, "einkuenfte_nichtselbststaendig": 45000,
         "veranlagung": "einzel"}
    kette = R.catala_gesamt_kette(s)
    accessor = R.catala_gesamt_tarifliche(s)
    assert kette["tarifliche_est"] == accessor


def test_kette_fest_gleich_accessor():
    """Kette[festzusetzende_est] == catala_gesamt()."""
    s = {"veranlagungszeitraum": VZ, "einkuenfte_nichtselbststaendig": 55000,
         "veranlagung": "einzel"}
    kette = R.catala_gesamt_kette(s)
    accessor = R.catala_gesamt(s)
    assert kette["festzusetzende_est"] == accessor
