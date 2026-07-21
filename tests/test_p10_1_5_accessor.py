"""§10 Abs.1 Nr.5 Kinderbetreuung — Accessor-Fidelity (VZ2025+: 80%, Deckel 4800€ JE KIND).
Multi-Kind-Komposition (anzahl_kinder × min(aufwand_pro_kind × 80%, 4800)). Gleichverteilungs-
Annahme der Summe (Stufe-1). NULL LLM.
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402


def _abz(aufwendungen, anzahl_kinder):
    return R.catala_p10_1_5_kinderbetreuung({"aufwendungen": aufwendungen, "anzahl_kinder": anzahl_kinder})


def test_single_kind_gedeckelt():
    """1 Kind, 6000€ → 80%=4800, Deckel 4800 → 4800."""
    assert _abz(6000, 1) == 4800


def test_single_kind_unter_deckel():
    """1 Kind, 1000€ → 80%=800 (< Deckel) → 800."""
    assert _abz(1000, 1) == 800


def test_single_kind_ueber_deckel():
    """1 Kind, 10000€ → 80%=8000, Deckel 4800 → 4800."""
    assert _abz(10000, 1) == 4800


def test_multi_kind_je_kind_deckel():
    """2 Kinder, 12000€ (6000/Kind) → je min(4800,4800) → 2×4800 = 9600."""
    assert _abz(12000, 2) == 9600


def test_null_kinder():
    """0 Kinder → 0 (kein Abzug)."""
    assert _abz(5000, 0) == 0


def test_null_aufwand():
    """Aufwand 0 → 0."""
    assert _abz(0, 2) == 0
