"""§10 Abs.1 Nr.9 Schulgeld — Accessor-Fidelity (VZ2025+: 30%, Deckel 2.500€ JE KIND,
bei Zusammenveranlagung 5.000€ je Kind).
Per-Kind-Aufruf (Ring summiert per EM.instanzen wie Nr.5).
Kz E0505607 (Sum, cent → ceiling, _ABZUGS_KZ).
NULL LLM.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402


def _abz(aufwendungen, vz=2025, splitting=False):
    return R.catala_p10_1_9_schulgeld({"aufwendungen": aufwendungen, "veranlagungszeitraum": vz, "splitting": splitting})


def test_single_kind_gedeckelt_einzel():
    """1 Kind 10000€ → 30%=3000, Deckel 2500 → 2500 (Einzelveranlagung)."""
    assert _abz(10000) == 2500


def test_single_kind_unter_deckel_einzel():
    """1 Kind 5000€ → 30%=1500 (< Deckel 2500) → 1500."""
    assert _abz(5000) == 1500


def test_single_kind_gedeckelt_zusammen():
    """1 Kind 20000€ → 30%=6000, Deckel 5000 (Zusammenveranlagung) → 5000."""
    assert _abz(20000, splitting=True) == 5000


def test_single_kind_unter_deckel_zusammen():
    """1 Kind 10000€ → 30%=3000 (< Deckel 5000) → 3000."""
    assert _abz(10000, splitting=True) == 3000


def test_null_aufwand():
    """Aufwand 0 → 0."""
    assert _abz(0) == 0
    assert _abz(0, splitting=True) == 0


def test_veranlagungsweiche_exakt():
    """Gleicher Aufwand, Einzel 2500 vs Zusammen 5000 Deckel."""
    assert _abz(25000) == 2500                # 30%=7500, Deckel 2500
    assert _abz(25000, splitting=True) == 5000  # 30%=7500, Deckel 5000


def test_vz_2024_gleiche_params():
    """VZ 2024 hat gleiche Werte (30%, 2500)."""
    assert _abz(10000, vz=2024) == 2500
    assert _abz(10000, vz=2024, splitting=True) == 3000  # 30%=3000 < 5000


def test_vz_2026_gleiche_params():
    """VZ 2026 hat gleiche Werte (30%, 2500)."""
    assert _abz(10000, vz=2026) == 2500