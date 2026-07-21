"""SolZ-Accessor-Gate (§3, §4 SolzG 1995). Deterministisch, NULL LLM.

Validiert catala_solz (golden/runner.py): Freigrenze, Milderungszone, §32d-Kapital-Split,
VZ-Drift 2024-2026, Splitting. Hand-nachgerechnete Werte (Instructor-abgenommen). CENT.
"""

import os
import sys
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402


# -- §3 Abs.3 S.1 Freigrenze (kein Kapital) ----------------------------------

def test_unter_freigrenze_null():
    """20.000€ unter Freigrenze 20.350 (2026) → 0€."""
    assert R.catala_solz({"veranlagungszeitraum": 2026, "bemessungsgrundlage": 20000,
                           "splitting": False}) == 0


def test_unter_freigrenze_splitting_null():
    """40.000€ unter Splitting-Freigrenze 40.700 (2026) → 0€."""
    assert R.catala_solz({"veranlagungszeitraum": 2026, "bemessungsgrundlage": 40000,
                           "splitting": True}) == 0


# -- §4 S.1 Regelsatz 5,5 % -------------------------------------------

def test_regelsatz_2026():
    """50.000€, einzeln, 2026: 5,5% = 2750,00€."""
    assert R.catala_solz({"veranlagungszeitraum": 2026, "bemessungsgrundlage": 50000,
                           "splitting": False}) == 275000


# -- §4 S.2 Milderungszone --------------------------------------------

def test_milderungszone_splitting_2026():
    """50.000€, Splitting, 2026: 11,9% × (50000−40700) = 1106,70€.
    5,5%-Regelsatz = 2750€; Milderung greift (1106,70 < 2750)."""
    assert R.catala_solz({"veranlagungszeitraum": 2026, "bemessungsgrundlage": 50000,
                           "splitting": True}) == 110670


def test_milderungszone_2024():
    """25.000€, einzeln, 2024 (FG 18.130): 11,9% × (25000−18130) = 817,53€
    (5,5% = 1375€, Milderung günstiger)."""
    assert R.catala_solz({"veranlagungszeitraum": 2024, "bemessungsgrundlage": 25000,
                           "splitting": False}) == 81753


def test_milderungszone_2025():
    """25.000€, einzeln, 2025 (FG 19.950): 11,9% × (25000−19950) = 600,95€."""
    assert R.catala_solz({"veranlagungszeitraum": 2025, "bemessungsgrundlage": 25000,
                           "splitting": False}) == 60095


# -- §3 Abs.3 S.2: §32d-Kapital-SolZ (5,5 % ohne Freigrenze) --------

def test_kapital_solz_ohne_freigrenze():
    """2.000€ Kapital-Steuer → 5,5% = 110€, unabhängig von Haupt-Basis.
    Haupt-Basis 15.000€ unter FG 18.130 → 0€ main. Gesamt = 110€."""
    assert R.catala_solz({"veranlagungszeitraum": 2024, "bemessungsgrundlage": 15000,
                           "kapital_steuer": 2000, "splitting": False}) == 11000


def test_kapital_solz_additiv():
    """40.000€ est, 2024 einzeln (FG 18.130): main = 38.000 > FG → SolZ_main.
    kap=2.000€ immer 5,5% = 110€. SolZ_main = 5,5% × 38.000 = 2.090€.
    Gesamt = 2.200€."""
    assert R.catala_solz({"veranlagungszeitraum": 2024, "bemessungsgrundlage": 40000,
                           "kapital_steuer": 2000, "splitting": False}) == 220000


# -- VZ-Drift ---------------------------------------------------------

def test_vz2024_freigrenze_18130():
    """VZ 2024 Freigrenze 18.130 einzeln."""
    assert R.catala_solz({"veranlagungszeitraum": 2024, "bemessungsgrundlage": 100000,
                           "splitting": False}) == 550000   # 5,5% = 5.500€, deutlich über FG


def test_vz_assertion():
    """VZ ausserhalb 2024-2026 muss AssertionError werfen."""
    with pytest.raises(AssertionError):
        R.catala_solz({"veranlagungszeitraum": 2023, "bemessungsgrundlage": 50000, "splitting": False})
    with pytest.raises(AssertionError):
        R.catala_solz({"veranlagungszeitraum": 2027, "bemessungsgrundlage": 50000, "splitting": False})
