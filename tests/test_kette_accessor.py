"""catala_gesamt_kette — Rechenweg-Kette Consistency mit Einzelaccessoren."""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R

VZ = 2025


def test_kette_gde_konsistent():
    """Kette[gesamtbetrag_der_einkuenfte] == catala_gesamt_gde()."""
    s = {"veranlagungszeitraum": VZ, "einkuenfte_nichtselbststaendig": 60000,
         "einkuenfte_kapitalvermoegen": 5000, "veranlagung": "einzel"}
    kette = R.catala_gesamt_kette(s)
    accessor = R.catala_gesamt_gde(s)
    assert kette["gesamtbetrag_der_einkuenfte"] == accessor, \
        f"GdE Mismatch: kette={kette['gesamtbetrag_der_einkuenfte']}, accessor={accessor}"


def test_kette_zve_konsistent():
    """Kette[zu_versteuerndes_einkommen] == catala_gesamt_zve()."""
    s = {"veranlagungszeitraum": VZ, "einkuenfte_nichtselbststaendig": 50000,
         "sonderausgaben": 2000, "veranlagung": "einzel"}
    kette = R.catala_gesamt_kette(s)
    accessor = R.catala_gesamt_zve(s)
    assert kette["zu_versteuerndes_einkommen"] == accessor, \
        f"zvE Mismatch: kette={kette['zu_versteuerndes_einkommen']}, accessor={accessor}"


def test_kette_tariflich_konsistent():
    """Kette[tarifliche_est] == catala_gesamt_tarifliche()."""
    s = {"veranlagungszeitraum": VZ, "einkuenfte_nichtselbststaendig": 45000,
         "veranlagung": "einzel"}
    kette = R.catala_gesamt_kette(s)
    accessor = R.catala_gesamt_tarifliche(s)
    assert kette["tarifliche_est"] == accessor, \
        f"tarifliche Mismatch: kette={kette['tarifliche_est']}, accessor={accessor}"


def test_kette_fest_konsistent():
    """Kette[festzusetzende_est] == catala_gesamt()."""
    s = {"veranlagungszeitraum": VZ, "einkuenfte_nichtselbststaendig": 55000,
         "veranlagung": "einzel"}
    kette = R.catala_gesamt_kette(s)
    accessor = R.catala_gesamt(s)
    assert kette["festzusetzende_est"] == accessor, \
        f"festzusetzende Mismatch: kette={kette['festzusetzende_est']}, accessor={accessor}"
