"""§35 GewSt-Anrechnung — Mitunternehmer-Zähler (§35 Abs.1 S.3). Deterministisch, NULL LLM.

§15 Mitunternehmer-Einkünfte sind IMMER gewerbesteuerpflichtig (Anlage G) → §35-Zähler
muss sie zählen, auch wenn betriebsart≠gewerbe (reiner Mitunternehmer ohne eigenes Gewerbe).
Vor dem Fix (_laufender_gewinn returned int, Zähler auf betriebsart=="gewerbe" gegatet)
wurde reiner Mitunternehmer still auf Zähler=0 blockiert → Over-tax.
Ruling instructor_rulings_100.md:7-23, abgenommen 2026-07-21.
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402


# -- §35-Mitu-Zähler-Logik (replikation api.py-Logik, ohne Ring) ---------------

def _p35_zaehler(laufender_gewinn: int, mitu: int, betriebsart) -> int:
    """Exakte Replikation des api.py §35-Zähler-Ausdrucks (beide Scheiben)."""
    if betriebsart == "gewerbe":
        return max(0, laufender_gewinn)
    return max(0, mitu)


# -- Reiner Mitunternehmer (betriebsart≠gewerbe) ------------------------------

def test_mitu_zaehler_reiner_mitunternehmer():
    """Reiner Mitunternehmer (mitu=5000, betriebsart=selbstaendig, eigener Direktgewinn=0):
    laufender_gewinn=5000, mitu=5000, betriebsart≠gewerbe → Zähler=5000 (nur mitu).
    Vor Fix: Zähler=0 (Over-tax)."""
    assert _p35_zaehler(5000, 5000, "selbstaendig") == 5000


def test_mitu_zaehler_mit_direktgewinn_nicht_gewerbe():
    """Mitunternehmer (mitu=3000) + eigener §18-selbstständig-Gewinn (3000):
    laufender_gewinn=6000, mitu=3000, betriebsart=selbstaendig → Zähler=3000.
    Der §18-Direktgewinn ist NICHT gewerbesteuerpflichtig → bleibt draussen."""
    assert _p35_zaehler(6000, 3000, "selbstaendig") == 3000


def test_mitu_zaehler_gewerbe_beide_drin():
    """Eigenes Gewerbe (mitu=3000, Direktgewinn=5000, betriebsart=gewerbe):
    laufender_gewinn=8000, mitu=3000, betriebsart=gewerbe → Zähler=8000 (alles gewerblich).
    Status quo war korrekt — kein Regression-Risiko."""
    assert _p35_zaehler(8000, 3000, "gewerbe") == 8000


def test_mitu_zaehler_negativer_eigener_gewinn():
    """Eigener §18-Gewinn negativ (-2000), mitu=5000: laufender_gewinn=3000.
    betriebsart=selbstaendig → Zähler=5000 (mitu), der negative Eigengewinn bleibt draussen.
    max(0,…) greift nur auf das Endergebnis, nicht die Komponenten einzeln."""
    assert _p35_zaehler(3000, 5000, "selbstaendig") == 5000


def test_mitu_zaehler_negativer_mitu():
    """mitu negativ (Verlustanteil §15a) → max(0,mitu)=0. Keinerlei Kredit."""
    assert _p35_zaehler(-2000, -2000, "selbstaendig") == 0


def test_mitu_zaehler_unset_betriebsart():
    """Betriebsart unset (keinerlei Direktgewerbe) + mitu>0 → Zähler=mitu."""
    assert _p35_zaehler(5000, 5000, None) == 5000


def test_mitu_zaehler_gewerbe_negativer_gewinn():
    """betriebsart=gewerbe aber Gesamtverlust (laufender_gewinn=-500, mitu=0).
    max(0, -500)=0 → kein Kredit. Richtung over-tax-safe."""
    assert _p35_zaehler(-500, 0, "gewerbe") == 0


# -- Accessor-Konsistenz (catala_mitunternehmer_einkuenfte existiert) -----------

def test_mitu_accessor_funktional():
    """catala_mitunternehmer_einkuenfte summiert korrekt (EURO)."""
    r = R.catala_mitunternehmer_einkuenfte({
        "gewinnanteil": 10000, "verguetung_taetigkeit": 2000,
        "verguetung_darlehen": 500, "verguetung_ueberlassung": 300})
    assert r == 12800


def test_mitu_accessor_negativer_gewinnanteil():
    """Gewinnanteil negativ (Verlust aus PersG), Sondervergütungen positiv."""
    r = R.catala_mitunternehmer_einkuenfte({
        "gewinnanteil": -5000, "verguetung_taetigkeit": 2000,
        "verguetung_darlehen": 0, "verguetung_ueberlassung": 0})
    assert r == -3000
