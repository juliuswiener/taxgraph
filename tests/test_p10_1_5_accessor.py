"""§10 Abs.1 Nr.5 Kinderbetreuung — Accessor-Fidelity (VZ2025+: 80%, Deckel 4800€ JE KIND).
Per-Kind-Aufruf (KEINE Gleichverteilung mehr — Ring summiert per EM.instanzen).
NULL LLM.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402


def _abz(aufwendungen, vz=2025):
    return R.catala_p10_1_5_kinderbetreuung({"aufwendungen": aufwendungen, "veranlagungszeitraum": vz})


def test_single_kind_gedeckelt():
    """1 Kind, 6000€ → 80%=4800, Deckel 4800 → 4800."""
    assert _abz(6000) == 4800


def test_single_kind_unter_deckel():
    """1 Kind, 1000€ → 80%=800 (< Deckel) → 800."""
    assert _abz(1000) == 800


def test_single_kind_ueber_deckel():
    """1 Kind, 10000€ → 80%=8000, Deckel 4800 → 4800."""
    assert _abz(10000) == 4800


def test_null_aufwand():
    """Aufwand 0 → 0."""
    assert _abz(0) == 0


def test_ungleiche_kinder_regression():
    """2 Kinder 10000€ (8000+2000) — NICHT mehr gleichverteilt. Accessor rechnet PRO KIND.
    Vor Fix: 8000€ (Gleichverteilung). Nach Fix: Ring summiert pro Kind. Accessor allein:
    Aufruf mit 8000 → 4800."""
    assert _abz(8000) == 4800
    assert _abz(2000) == 1600