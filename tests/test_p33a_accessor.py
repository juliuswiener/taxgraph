"""§33a EStG Unterhalt + Ausbildungsfreibetrag — Accessor-Fidelity. NULL LLM.

p33a_unterhalt: 5 pipeline-seeds (Hand-Modul). p33a_ausbildungsfreibetrag: 2 seeds (Snapshot).
Jeder Seed muss EXAKT reproduziert werden. VZ 2026 (GFB 12.348) für alle Unterhalt-Tests.
EURO (integer).
"""

import os, sys
import pytest
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402

VZ = 2026  # GFB 12.348


# -- p33a_unterhalt (5 seeds) ---------------------------------------------------

def test_unterhalt_seed_0_normal():
    """10k Aufwand, 0 KV, 0 andere → min(10000, 12348) = 10000."""
    r = R.catala_p33a_unterhalt({"veranlagungszeitraum": VZ, "aufwendungen": 10000})
    assert r == 10000


def test_unterhalt_seed_1_gfb_deckel():
    """15k Aufwand > GFB 12.348 → gedeckelt auf 12.348."""
    r = R.catala_p33a_unterhalt({"veranlagungszeitraum": VZ, "aufwendungen": 15000})
    assert r == 12348


def test_unterhalt_seed_2_anrechnung_eigener_einkuenfte():
    """15k Aufwand, 2000 andere Einkünfte: anrechnung=2000−624=1376, HB=12348−1376=10972."""
    r = R.catala_p33a_unterhalt({"veranlagungszeitraum": VZ, "aufwendungen": 15000,
                                  "andere_einkuenfte_bezuege": 2000})
    assert r == 10972


def test_unterhalt_seed_3_kv_pv_additiv():
    """15k Aufwand, 1500 KV/PV → HB=12348+1500=13848, min(15000,13848)=13848."""
    r = R.catala_p33a_unterhalt({"veranlagungszeitraum": VZ, "aufwendungen": 15000,
                                  "kv_pv_beitraege": 1500})
    assert r == 13848


def test_unterhalt_seed_4_anrechnung_unter_624():
    """15k Aufwand, 500 andere Einkünfte (500<624) → keine Anrechnung, HB=12348."""
    r = R.catala_p33a_unterhalt({"veranlagungszeitraum": VZ, "aufwendungen": 15000,
                                  "andere_einkuenfte_bezuege": 500})
    assert r == 12348


def test_unterhalt_vz_2025():
    """VZ 2025: GFB 12.096."""
    r = R.catala_p33a_unterhalt({"veranlagungszeitraum": 2025, "aufwendungen": 15000})
    assert r == 12096


def test_unterhalt_vz_2024():
    """VZ 2024: GFB 11.784."""
    r = R.catala_p33a_unterhalt({"veranlagungszeitraum": 2024, "aufwendungen": 15000})
    assert r == 11784


def test_unterhalt_vz_assertion():
    """VZ ausserhalb 2024-2026 → AssertionError."""
    with pytest.raises(AssertionError):
        R.catala_p33a_unterhalt({"veranlagungszeitraum": 2023, "aufwendungen": 10000})
    with pytest.raises(AssertionError):
        R.catala_p33a_unterhalt({"veranlagungszeitraum": 2027, "aufwendungen": 10000})


# -- p33a_ausbildungsfreibetrag (Per-Kind: anzahl * 1200€) ----------------------

def test_ausbildung_1_kind():
    assert R.catala_p33a_ausbildungsfreibetrag({"anzahl_kinder": 1}) == 1200


def test_ausbildung_0_kinder():
    assert R.catala_p33a_ausbildungsfreibetrag({"anzahl_kinder": 0}) == 0


def test_ausbildung_2_kinder():
    assert R.catala_p33a_ausbildungsfreibetrag({"anzahl_kinder": 2}) == 2400


def test_ausbildung_default_0():
    """Key fehlt → anzahl_kinder=0 → 0€ (absent-safe)."""
    assert R.catala_p33a_ausbildungsfreibetrag({}) == 0
