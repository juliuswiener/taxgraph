"""§ 20 / § 32d Kapital-Accessoren (Weg A) + Konsistenz-Gate runner↔registry.

catala_sparer_pb (§ 20 Abs. 9), catala_kapital_verrechnung (§ 20 Abs. 6, Topf-Trennung), catala_kapital_
steuer (§ 32d Abs. 1/6, Günstigerprüfung). Die zwei est-Größen (est_regulaer_mit/ohne_kap) liefert der
Aufrufer über zwei catala_gesamt-Läufe — hier über die Registry-test_seeds direkt gespeist. Param-Werte
(Sparer-PB 1000/2000, Satz 25 %) aus params/<vz>, Wertkonstanz 2024-26.
"""
import os
import sys

import pytest
import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "golden"))

VZ = (2024, 2025, 2026)


def _runner():
    try:
        import runner
        return runner
    except Exception as e:
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")


def _seeds(rule_id):
    doc = yaml.safe_load(open(os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")))
    return next(r for r in doc["regeln"] if r["rule_id"] == rule_id)["test_seed"]


# ---- Params + Wertkonstanz ----------------------------------------------------

def test_param_sparer_pb_1000_alle_vz():
    runner = _runner()
    for vz in VZ:
        assert runner._sparer_pauschbetrag(vz) == 1000, vz


def test_param_abgeltungssatz_25_alle_vz():
    runner = _runner()
    for vz in VZ:
        assert runner._abgeltungssatz(vz) == 25, vz


# ---- Konsistenz-Gate runner↔registry (rot bei Divergenz) ----------------------

def test_sparer_pb_konsistenz_runner_registry():
    runner = _runner()
    for c in _seeds("p20_9_sparer_pauschbetrag"):
        got = runner.catala_sparer_pb({**c["inputs"], "veranlagungszeitraum": 2025})
        assert got == int(c["expected"]), (c["inputs"], got, c["expected"])


def test_verrechnung_konsistenz_runner_registry():
    runner = _runner()
    for c in _seeds("p20_6_verlustverrechnung"):
        got = runner.catala_kapital_verrechnung(c["inputs"])
        assert got == int(c["expected"]), (c["inputs"], got, c["expected"])


def test_kapital_steuer_konsistenz_runner_registry():
    runner = _runner()
    for c in _seeds("p32d_1_abgeltung"):
        got = runner.catala_kapital_steuer({**c["inputs"], "veranlagungszeitraum": 2025})
        assert got == int(c["expected"]), (c["inputs"], got, c["expected"])


# ---- Topf-Trennung + Sparer-PB-Doppelung + Günstiger-Vorrang ------------------

def test_topf_trennung_kein_uebergreifen():
    """§ 20 Abs. 6: sonstiger Verlust mindert NICHT den Aktien-Gewinn (getrennte Töpfe)."""
    runner = _runner()
    # Aktien +5000/−2000 = 3000; sonstige +1000/−3000 = 0 (nicht −2000) -> Summe 3000
    assert runner.catala_kapital_verrechnung({"gewinn_aktien": 5000, "verlust_aktien": 2000,
                                              "gewinn_sonstige": 1000, "verlust_sonstige": 3000}) == 3000


def test_sparer_pb_verdoppelt_zusammen():
    runner = _runner()
    assert runner.catala_sparer_pb({"veranlagungszeitraum": 2025, "kapitalertraege": 5000,
                                    "zusammenveranlagung": True}) == 3000   # 5000 − 2000
    assert runner.catala_sparer_pb({"veranlagungszeitraum": 2025, "kapitalertraege": 5000,
                                    "zusammenveranlagung": False}) == 4000  # 5000 − 1000


def test_guenstiger_vs_abgeltung():
    """min(Abgeltung, delta): Grenzsteuer > 25 % -> Abgeltung; < 25 % -> Günstiger (delta)."""
    runner = _runner()
    st = lambda kap, mit, ohne: runner.catala_kapital_steuer(
        {"veranlagungszeitraum": 2025, "kapitaleinkuenfte": kap,
         "est_regulaer_mit_kap": mit, "est_regulaer_ohne_kap": ohne})
    assert st(9000, 17537, 13924) == 2250   # delta 3613 > abgeltung 2250 -> Abgeltung (K4)
    assert st(9000, 15000, 13924) == 1076    # delta 1076 < abgeltung 2250 -> Günstiger
    assert st(9000, 13924, 13924) == 0       # delta 0 (0-Grundtarif) -> 0 (reiner Kapital-Fall)
