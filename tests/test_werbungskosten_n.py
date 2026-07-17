"""catala_werbungskosten_n — Roh-WK-Summe Anlage N (Gesamtsteuer-Ring MVP Stufe 1).

Belegt: (a) Stufe 1 = Entfernungspauschale, (b) ohne EP-Slots = 0 (dHf/Verpflegung/AM haben
kein Catala-Modul), (c) ROH — kein § 9a-Pauschbetrag-Günstiger (der sitzt im Tarif; ein hier
angehobener Betrag waere doppelter Abzug).
"""
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "golden"))


def _runner():
    try:
        import runner
        return runner
    except Exception as e:  # Catala-Toolchain fehlt
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")


def test_stufe1_ist_entfernungspauschale():
    runner = _runner()
    s = {"veranlagungszeitraum": 2025, "arbeitstage": 220, "entfernung_km_roh": 30,
         "oepnv_kosten_jahr": 0, "eigenes_oder_ueberlassenes_kfz": True}
    assert runner.catala_werbungskosten_n(s) == 2156


def test_ohne_ep_slots_ist_null():
    runner = _runner()
    # dHf/Verpflegung/AM haben kein Modul -> keine EP-Slots => WK-Summe 0 (nicht erfunden)
    assert runner.catala_werbungskosten_n({"veranlagungszeitraum": 2025}) == 0


def test_roh_kein_9a_pauschbetrag():
    runner = _runner()
    # 50 Tage x 10 km x 0,30 = 150 EUR. Roh bleibt 150 — NICHT auf den Pauschbetrag 1230
    # angehoben (den wendet der Tarif an; hier waere es Doppelabzug).
    s = {"veranlagungszeitraum": 2025, "arbeitstage": 50, "entfernung_km_roh": 10,
         "oepnv_kosten_jahr": 0, "eigenes_oder_ueberlassenes_kfz": True}
    assert runner.catala_werbungskosten_n(s) == 150


# ---- Stufe 1b: doppelte Haushaltsführung (dHf) ----

def _dhf_registry_seed():
    import yaml
    doc = yaml.safe_load(open(os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")))
    return next(r for r in doc["regeln"]
                if r["rule_id"] == "p9_1_3_nr5_doppelte_haushaltsfuehrung")["test_seed"]


def test_dhf_konsistenz_runner_registry():
    """KONSISTENZ-GATE runner↔registry: _dhf_abzug MUSS die Registry-Rechenwege (test_seed von
    p9_1_3_nr5) cent-genau reproduzieren. Divergenz (runner-Formel läuft von der Registry-hinweis-
    Formel weg) → ROT. Kopplung: bei Registry-Änderung diese Formel nachziehen."""
    runner = _runner()
    import yaml  # noqa: F401
    for c in _dhf_registry_seed():
        got = runner._dhf_abzug(c["inputs"], 2025)
        exp = int(c["expected"])
        assert got == exp, (f"runner↔registry-Divergenz: {c['inputs']} → runner {got} ≠ "
                            f"registry {exp} ({c['rechenweg']})")


def test_werbungskosten_n_mit_dhf():
    runner = _runner()
    # EP (2156) + dHf (Miete 1400 → gekappt 1000 × 12 = 12000) = 14156 EUR
    s = {"veranlagungszeitraum": 2025, "arbeitstage": 220, "entfernung_km_roh": 30,
         "oepnv_kosten_jahr": 0, "eigenes_oder_ueberlassenes_kfz": True,
         "unterkunftskosten_monat": 1400, "monate": 12, "im_inland": True}
    assert runner.catala_werbungskosten_n(s) == 2156 + 12000
