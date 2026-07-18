"""Gate für die § 33b Pauschbetrag-Params (params/<vz>/behinderten_pauschbetrag_p33b.yaml). NULL LLM.

Validiert die 2021er-Fassung (Behinderten-Pauschbetragsgesetz, verdoppelte Werte — GdB 50 = 1140, NICHT
alt 570): GdB-Staffel-Vollständigkeit (9 Tiers 20..100) + Tier-Lookup-Semantik (gdb//10*10) + die
Sonder-Pauschbeträge (Blind/Hilflos 7400 § 33b Abs.3 S.3, Hinterbliebene 370 Abs.4, Pflege-Staffel
600/1100/1800 Abs.6 S.3 + hilflos 1800 S.4). Alle drei VZ konsistent.
"""
from __future__ import annotations

import os

import pytest

yaml = pytest.importorskip("yaml")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VZS = (2024, 2025, 2026)


def _p33b(vz):
    p = os.path.join(ROOT, "params", str(vz), "behinderten_pauschbetrag_p33b.yaml")
    return yaml.safe_load(open(p, encoding="utf-8"))


def _gdb_lookup(staffel, gdb):
    """Tier-Lookup: 'mindestens X' -> abgerundeter 10er-Tier (gdb//10*10, gedeckelt 100)."""
    return staffel[min(100, (gdb // 10) * 10)]


def test_gdb_staffel_vollstaendig_9_tiers():
    for vz in VZS:
        staffel = {int(k): v for k, v in _p33b(vz)["gdb_staffel"].items()}
        assert sorted(staffel) == [20, 30, 40, 50, 60, 70, 80, 90, 100]


def test_gdb_staffel_2021er_werte_spots():
    staffel = {int(k): v for k, v in _p33b(2025)["gdb_staffel"].items()}
    assert staffel[20] == 384 and staffel[50] == 1140 and staffel[100] == 2840   # 2021er verdoppelt
    # monoton steigend
    werte = [staffel[t] for t in (20, 30, 40, 50, 60, 70, 80, 90, 100)]
    assert all(werte[i] < werte[i + 1] for i in range(len(werte) - 1))


def test_gdb_tier_lookup_mindestens_semantik():
    staffel = {int(k): v for k, v in _p33b(2025)["gdb_staffel"].items()}
    assert _gdb_lookup(staffel, 55) == 1140      # 55 -> Tier 50
    assert _gdb_lookup(staffel, 50) == 1140
    assert _gdb_lookup(staffel, 100) == 2840
    assert _gdb_lookup(staffel, 20) == 384


def test_sonder_pauschbetraege():
    for vz in VZS:
        d = _p33b(vz)
        assert d["blind_hilflos_taubblind"] == 7400     # Abs.3 S.3
        assert d["hinterbliebenen"] == 370               # Abs.4


def test_pflege_staffel():
    for vz in VZS:
        pf = {int(k): v for k, v in _p33b(vz)["pflege_staffel"].items()}
        assert pf[2] == 600 and pf[3] == 1100 and pf[4] == 1800 and pf[5] == 1800   # Abs.6 S.3
        assert _p33b(vz)["pflege_hilflos"] == 1800        # Abs.6 S.4 (gepflegte Person hilflos)


def test_alle_vz_wertgleich():
    # 2021er-Werte stabil 2024-2026 (keine Gesetzesänderung im Zeitraum)
    ref = {k: _p33b(2024)[k] for k in ("gdb_staffel", "blind_hilflos_taubblind", "hinterbliebenen",
                                       "pflege_staffel", "pflege_hilflos")}
    for vz in (2025, 2026):
        assert {k: _p33b(vz)[k] for k in ref} == ref
