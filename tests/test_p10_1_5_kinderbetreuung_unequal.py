"""REGRESSION (2026-08-06): Kinderbetreuung Gleichverteilungs-Annahme.
2 Kinder, ungleiche Beträge. Vor Fix: Gleichverteilung → 8000€ Abzug.
Nach Fix per-Kind: Kind1 8000→4800, Kind2 2000→1600 = 6400€.

VOR Fix: assert 6400 fail → ROT. NACH Fix: per-Kind-Interface → GRÜN.
NULL LLM.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402


def test_ungleiche_kinder_regression():
    """Accessor PRO KIND (KEINE Gleichverteilung). 8000€→4800, 2000€→1600.
    ROT wenn zur Gleichverteilung (anzahl_kinder-Multiplikator) zurückgebaut."""
    assert R.catala_p10_1_5_kinderbetreuung(
        {"aufwendungen": 8000, "veranlagungszeitraum": 2025}) == 4800
    assert R.catala_p10_1_5_kinderbetreuung(
        {"aufwendungen": 2000, "veranlagungszeitraum": 2025}) == 1600