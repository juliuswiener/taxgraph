"""§ 21 Abs. 2 EStG — verbilligte Wohnraumvermietung WK-Kürzung, Accessor-Einheitstest (module
VerbilligteVermietungWk). Prüft catala_p21_2_verbilligt: entgelt_quote ≥ 66 % → volle WK; < 66 % → WK ×
(quote/100). Der Fold-seitige Tatbestand-Gate (Wohnzwecke/Dauer) + die per-Objekt-Anwendung sind e2e getestet
(test_gesamt_p21_2_*). Toolchain-frei übersprungen."""
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "golden"))


@pytest.fixture(scope="module")
def R():
    try:
        import runner
        runner.catala_p21_2_verbilligt({"werbungskosten": 0, "entgelt_quote_prozent": 100})
    except Exception:
        pytest.skip("Catala-Toolchain / pkg nicht verfügbar")
    return runner


@pytest.mark.parametrize("wk,quote,erwartet", [
    (10000, 100, 10000),    # ≥ 66 % → voll
    (10000, 66, 10000),     # genau 66 % → voll (>= 66)
    (10000, 65, 6500),      # < 66 % → × 0,65
    (10000, 50, 5000),      # < 50 % → × 0,50 (Aufteilung; hier konservativ anteilig)
    (10000, 40, 4000),      # < 50 % → × 0,40
    (10000, 0, 0),          # quote 0 → keine abziehbaren WK
    (0, 50, 0),             # keine WK → 0
])
def test_p21_2_kuerzung(R, wk, quote, erwartet):
    assert R.catala_p21_2_verbilligt({"werbungskosten": wk, "entgelt_quote_prozent": quote}) == erwartet


def test_p21_2_kuerzung_erhoeht_einkuenfte(R):
    """Die Kürzung senkt die abziehbaren WK → erhöht die § 21-Einkünfte (Unter-Besteuerungs-Fix, K2-sichere
    Richtung): bei quote 50 % ist der abziehbare WK-Betrag kleiner als voll."""
    voll = R.catala_p21_2_verbilligt({"werbungskosten": 8000, "entgelt_quote_prozent": 100})
    gekuerzt = R.catala_p21_2_verbilligt({"werbungskosten": 8000, "entgelt_quote_prozent": 50})
    assert gekuerzt < voll and gekuerzt == 4000
