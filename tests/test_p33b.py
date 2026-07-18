"""§ 33b Pauschbeträge (Behinderung/Pflege/Hinterbliebene) — Weg-A-Accessoren + Konsistenz-Gate.

Werte aus params/<vz> (2021er-Reform-Fassung, GdB 50 = 1140). Behinderten + Pflege sind additiv
abziehbar (beide → aussergewoehnliche_belastungen). Konsistenz-Gate bindet die Accessoren an die
p33b-test_seeds der Registry (rot bei Divergenz).
"""
import os
import sys

import pytest
import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "golden"))


def _runner():
    try:
        import runner
        return runner
    except Exception as e:
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")


def _seeds(rule_id):
    doc = yaml.safe_load(open(os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")))
    return next(r for r in doc["regeln"] if r["rule_id"] == rule_id)["test_seed"]


def _gate(runner, rule_id, fn):
    for c in _seeds(rule_id):
        got = fn({**c["inputs"], "veranlagungszeitraum": 2025})
        assert got == int(c["expected"]), (rule_id, c["inputs"], got, c["expected"])


def test_behinderten_konsistenz_runner_registry():
    runner = _runner()
    _gate(runner, "p33b_behinderten_pauschbetrag", runner.catala_behinderten_pb)


def test_pflege_konsistenz_runner_registry():
    runner = _runner()
    _gate(runner, "p33b_pflege_pauschbetrag", runner.catala_pflege_pb)


def test_hinterbliebenen_konsistenz_runner_registry():
    runner = _runner()
    _gate(runner, "p33b_hinterbliebenen_pauschbetrag", runner.catala_hinterbliebenen_pb)


def test_gdb_tier_floor():
    """GdB rundet auf die Zehner-Stufe ab (§ 33b Abs. 3 S. 2): 45 → 40er-Tier 860, 19 → unter 20 → 0."""
    runner = _runner()
    b = lambda gdb: runner.catala_behinderten_pb({"veranlagungszeitraum": 2025, "grad_der_behinderung": gdb})
    assert b(50) == 1140 and b(45) == 860 and b(19) == 0 and b(100) == 2840


def test_hilflos_vorrang():
    """Blind/Hilflos → 7400 (ersetzt GdB-Staffel); Pflege ist_hilflos → 1800 (vor Pflegegrad-Staffel)."""
    runner = _runner()
    assert runner.catala_behinderten_pb({"veranlagungszeitraum": 2025, "grad_der_behinderung": 30,
                                         "ist_hilflos_blind_taubblind": True}) == 7400
    assert runner.catala_pflege_pb({"veranlagungszeitraum": 2025, "pflegegrad": 1,
                                    "ist_hilflos": True}) == 1800


def test_behinderten_und_pflege_additiv():
    """§ 33b: Behinderten- UND Pflege-Pauschbetrag stehen nebeneinander (beide → agB)."""
    runner = _runner()
    b = runner.catala_behinderten_pb({"veranlagungszeitraum": 2025, "grad_der_behinderung": 50})
    p = runner.catala_pflege_pb({"veranlagungszeitraum": 2025, "pflegegrad": 3})
    assert b + p == 1140 + 1100
