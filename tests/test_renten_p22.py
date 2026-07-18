"""§ 22 Nr. 1 Renten-Einkünfte-Accessor (Weg A) + Konsistenz-Gate runner↔registry.

catala_renten_einkuenfte, 4 Zweige (Instructor-Ruling): bb Ertragsanteil (exakt), aa Erstjahr
(Besteuerungsanteil-Kohorte), aa Folgejahr mit fixiertem Rentenfreibetrag, aa Folgejahr ohne → fail-closed
(K2 Rentenfreibetrag-Fixierung). Prozente aus params/kohorten (dev-2, VZ-agnostisch), WK-PB § 9a S. 1 Nr. 3.
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


def _renten(runner, **kw):
    return runner.catala_renten_einkuenfte({"veranlagungszeitraum": 2025, **kw})


# ---- Konsistenz-Gate runner↔registry (die p22_1-Arithmetik, prozent-agnostisch) ----

def test_stpfl_konsistenz_runner_registry():
    """KONSISTENZ-GATE: _renten_stpfl reproduziert p22_1_leibrente_besteuerungsanteil (jahresrente ×
    prozent/100). Deckt aa UND bb (identische Arithmetik, nur %-Quelle unterscheidet). Divergenz → ROT."""
    runner = _runner()
    seeds = next(r for r in yaml.safe_load(open(os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")))
                 ["regeln"] if r["rule_id"] == "p22_1_leibrente_besteuerungsanteil")["test_seed"]
    for c in seeds:
        got = runner._renten_stpfl(int(c["inputs"]["jahresrente"]), c["inputs"]["besteuerungsanteil_prozent"])
        assert got == int(c["expected"]), (c["inputs"], got, c["expected"])


def test_stpfl_nicht_ganzzahliger_prozent_exakt():
    """Kohorte 2025 = 83,5 % (Halbjahres-Anhebung) — die 1-Nachkomma-Präzision muss exakt sein
    (round(%×10)//1000), NICHT int(%) truncieren."""
    runner = _runner()
    assert runner._renten_stpfl(12000, 83.5) == 10020        # 12000 × 0,835
    assert runner._renten_stpfl(10000, 83.5) == 8350


def test_param_wk_pb_102():
    runner = _runner()
    assert runner._renten_wk_pb(2025) == 102


# ---- Die 4 Zweige ----

def test_bb_ertragsanteil_exakt():
    """bb private Leibrente: jahresrente × Ertragsanteil%(Alter) − WK-PB, exakt alle Jahre (keine Fixierung)."""
    runner = _runner()
    assert _renten(runner, renten_art="private_leibrente", jahresrente=12000,
                   alter_bei_rentenbeginn=65) == 2058       # 12000 × 18 % − 102
    assert _renten(runner, renten_art="sonstige_leibrente", jahresrente=12000,
                   alter_bei_rentenbeginn=64) == 2178       # 12000 × 19 % − 102


def test_aa_erstjahr_besteuerungsanteil():
    """aa Erstjahr (renten_beginn_jahr == VZ): jahresrente × Besteuerungsanteil%(Kohorte) − WK-PB."""
    runner = _runner()
    assert _renten(runner, renten_art="gesetzliche_rente", jahresrente=12000,
                   renten_beginn_jahr=2025) == 9918          # 12000 × 83,5 % − 102


def test_aa_folgejahr_mit_rentenfreibetrag():
    """aa Folgejahr MIT fixiertem Rentenfreibetrag (EUR): stpfl = jahresrente − rentenfreibetrag − WK-PB."""
    runner = _runner()
    assert _renten(runner, renten_art="gesetzliche_rente", jahresrente=12000,
                   renten_beginn_jahr=2020, rentenfreibetrag=6000) == 5898


def test_aa_folgejahr_ohne_rentenfreibetrag_fail_closed():
    """K2: aa Folgejahr OHNE Rentenfreibetrag → RentenfreibetragFixierungOffen (kein Rate-Bescheid;
    %×erhöhte-Jahresrente würde die Rentenerhöhung unterbesteuern)."""
    runner = _runner()
    with pytest.raises(runner.RentenfreibetragFixierungOffen):
        _renten(runner, renten_art="gesetzliche_rente", jahresrente=12000, renten_beginn_jahr=2020)


def test_wk_pb_floor_nicht_negativ():
    """Kleine Rente unter WK-PB → einkuenfte_sonstige 0 (kein negativer § 22-Beitrag)."""
    runner = _runner()
    assert _renten(runner, renten_art="private_leibrente", jahresrente=500,
                   alter_bei_rentenbeginn=65) == 0           # 90 − 102 -> max(0, …)


def test_unbekannte_renten_art_gap():
    """MVP = aa+bb; jede andere renten_art ist nicht ring-fähig (benannter GAP, kein stiller 0)."""
    runner = _runner()
    with pytest.raises(ValueError):
        _renten(runner, renten_art="betriebsrente_direktzusage", jahresrente=12000)
