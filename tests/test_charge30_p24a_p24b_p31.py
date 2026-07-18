"""§ 24a Altersentlastungsbetrag + § 24b Entlastungsbetrag Alleinerziehende + § 31 Familienleistungsausgleich —
Accessor-Einheitstests (charge30, Stage 2). Prüft catala_p24a_altersentlastung / catala_p24b_entlastung /
catala_p31_familienleistung gegen die dev-2-Module (Altersentlastungsbetrag/Entlastungsbetrag/
Familienleistungsausgleich) inkl. der Andock-Auflagen: §24a Kohorten-Lookup (geburtsjahr+65) + prozentsatz-als-
Prozent, §24b Bool()-Typ + alleinstehend-Gate, §31 Günstigerprüfung (Kindergeld vs Kinderfreibetrag). Toolchain-frei übersprungen."""
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "golden"))


@pytest.fixture(scope="module")
def R():
    try:
        import runner
        runner.catala_p24b_entlastung({"alleinstehend": False, "anzahl_kinder": 0})
    except Exception:
        pytest.skip("Catala-Toolchain / pkg nicht verfügbar")
    return runner


# ---- § 24a: Kohorten-Lookup (geburtsjahr+65 = maßgebendes Folgejahr) + prozentsatz × Bemessung, gedeckelt ----

def test_p24a_kohorten_deckel(R):
    """Hohe Bemessung (25000) → 14 % = 3500 > Höchstbetrag der Kohorte 2023 (665) → gedeckelt auf 665.
    (geburtsjahr 1958 → Vollendung 64. Lj 2022 → Folgejahr 2023 → 14,0 %/665.)"""
    assert R.catala_p24a_altersentlastung(
        {"geburtsjahr": 1958, "arbeitslohn": 20000, "positive_andere_einkuenfte": 5000}) == 665


def test_p24a_unter_deckel(R):
    """Niedrige Bemessung (2000) → 14 % = 280 ≤ 665 → ungedeckelt 280."""
    assert R.catala_p24a_altersentlastung({"geburtsjahr": 1958, "arbeitslohn": 2000}) == 280


def test_p24a_kohorte_juenger_kleinerer_satz(R):
    """Jüngere Kohorte → kleinerer Satz/Deckel: geburtsjahr 1965 → Folgejahr 2030 → 11,2 %/532; 25000 × 11,2 % =
    2800 > 532 → 532 (kleiner als die 665 der älteren Kohorte)."""
    assert R.catala_p24a_altersentlastung(
        {"geburtsjahr": 1965, "arbeitslohn": 20000, "positive_andere_einkuenfte": 5000}) == 532


def test_p24a_kein_geburtsjahr_ist_null(R):
    """geburtsjahr nicht erfasst (0) → Betrag 0 (fail-safe, kein Phantom-Abzug ohne Kohorte)."""
    assert R.catala_p24a_altersentlastung({"geburtsjahr": 0, "arbeitslohn": 20000}) == 0


# ---- § 24b: Grundbetrag + Erhöhung je weiterem Kind, nur bei alleinstehend + mind. 1 Kind ----

@pytest.mark.parametrize("allein,kinder,erwartet", [
    (True, 1, 4260),      # Grundbetrag 1 Kind
    (True, 2, 4500),      # + 240 Erhöhung je weiterem Kind
    (True, 3, 4740),      # + 240 je weiterem
    (True, 0, 0),         # kein Kind → kein Entlastungsbetrag
    (False, 1, 0),        # nicht alleinstehend → 0 (§ 24b Abs. 1/3)
])
def test_p24b_grundbetrag_und_erhoehung(R, allein, kinder, erwartet):
    assert R.catala_p24b_entlastung(
        {"alleinstehend": allein, "anzahl_kinder": kinder, "monate_ohne_voraussetzung": 0}) == erwartet


def test_p24b_monate_kuerzung(R):
    """Anteilige Kürzung um die Monate ohne Voraussetzung (§ 24b Abs. 4): 6 Monate ohne → halber Betrag."""
    voll = R.catala_p24b_entlastung({"alleinstehend": True, "anzahl_kinder": 1, "monate_ohne_voraussetzung": 0})
    halb = R.catala_p24b_entlastung({"alleinstehend": True, "anzahl_kinder": 1, "monate_ohne_voraussetzung": 6})
    assert 0 < halb < voll


# ---- § 31: Günstigerprüfung Kindergeld vs Kinderfreibetrag ----

def test_p31_kindergeld_guenstiger(R):
    """Kindergeld (3000) > Steuerersparnis des Freibetrags (est_ohne 10000 − est_mit 8000 = 2000) → Kindergeld
    gewinnt → festzusetzende ESt bleibt est_ohne 10000 (der Freibetrag wird NICHT angesetzt)."""
    assert R.catala_p31_familienleistung(
        {"est_ohne_freibetraege": 10000, "est_mit_freibetraegen": 8000, "kindergeld": 3000}) == 10000


def test_p31_freibetrag_guenstiger(R):
    """Freibetrag-Ersparnis (30000 − 25000 = 5000) > Kindergeld (3000) → Freibetrag gewinnt → est_mit 25000 +
    Hinzurechnung des Kindergelds (§ 31 S. 4) 3000 = 28000."""
    assert R.catala_p31_familienleistung(
        {"est_ohne_freibetraege": 30000, "est_mit_freibetraegen": 25000, "kindergeld": 3000}) == 28000
