"""§ 10 Abs. 1 Nr. 7 EStG — Aufwendungen für die eigene Berufsausbildung, Accessor-Einheitstest (module
Berufsausbildungsaufwendungen). Prüft catala_p10_1_7_berufsausbildung: abziehbare Sonderausgaben = min(aufwendungen,
6000) — Höchstbetrag je Person (§ 10 Abs. 1 Nr. 7 S. 1). Die Fold-seitige additive Einbettung in den gesamt-Ring
(sonderausgaben) ist e2e getestet (test_gesamt_p10_1_7_*). Toolchain-frei übersprungen."""
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "golden"))


@pytest.fixture(scope="module")
def R():
    try:
        import runner
        runner.catala_p10_1_7_berufsausbildung({"berufsausbildung_aufwendungen": 0})
    except Exception:
        pytest.skip("Catala-Toolchain / pkg nicht verfügbar")
    return runner


@pytest.mark.parametrize("aufwendungen,erwartet", [
    (0, 0),          # keine Aufwendungen → 0
    (4000, 4000),    # < Höchstbetrag → voll
    (5999, 5999),    # knapp unter HB → voll
    (6000, 6000),    # genau HB → voll
    (6001, 6000),    # knapp über HB → gedeckelt
    (8000, 6000),    # > HB → auf 6000 gedeckelt
    (100000, 6000),  # weit über HB → 6000
])
def test_p10_1_7_hoechstbetrag(R, aufwendungen, erwartet):
    assert R.catala_p10_1_7_berufsausbildung({"berufsausbildung_aufwendungen": aufwendungen}) == erwartet


def test_p10_1_7_deckel_greift(R):
    """Der Höchstbetrag (§ 10 Abs. 1 Nr. 7 S. 1, 6000 €) deckelt: Aufwendungen über 6000 werden nicht voll
    abgezogen (kein unbegrenzter Sonderausgabenabzug) — 8000 → 6000, nicht 8000."""
    gedeckelt = R.catala_p10_1_7_berufsausbildung({"berufsausbildung_aufwendungen": 8000})
    assert gedeckelt == 6000 and gedeckelt < 8000
