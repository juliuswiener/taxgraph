"""§ 33 außergewöhnliche Belastungen (+ § 33 Abs. 3 zumutbare Belastung) + § 10 Abs. 1 Nr. 4 KiSt —
Accessor-Einheitstests (charge29/#8). Prüft catala_p33_zumutbar / catala_p33_agb / catala_p10_kist gegen
die dev-2-Modul-Seeds (ZumutbareBelastung/AgbAbzug/Kirchensteuerabzug) + die Ketten-Semantik: §33 zumutbar
(Staffelung anzahl_kinder/splitting) → agB-Abzug (min 0) → aussergewoehnliche_belastungen; KiSt gezahlt−erstattet
→ sonderausgaben. Toolchain-frei übersprungen."""
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "golden"))


@pytest.fixture(scope="module")
def R():
    try:
        import runner
        runner.catala_p10_kist({"gezahlte_kirchensteuer": 0, "erstattete_kirchensteuer": 0})
    except Exception:
        pytest.skip("Catala-Toolchain / pkg nicht verfügbar")
    return runner


# ---- § 33 Abs. 3 zumutbare Belastung (Tranchen-Methode, Staffelung nach Kinderzahl/Splitting) ----

def test_zumutbar_staffelt_nach_kindern(R):
    """Mehr Kinder → niedrigere zumutbare Belastung (§ 33 Abs. 3 S. 1 Nr. 1-3); Splitting senkt zusätzlich."""
    gde = 50000
    z0 = R.catala_p33_zumutbar({"gesamtbetrag_der_einkuenfte": gde, "anzahl_kinder": 0, "splitting": False})
    z2 = R.catala_p33_zumutbar({"gesamtbetrag_der_einkuenfte": gde, "anzahl_kinder": 2, "splitting": False})
    z3 = R.catala_p33_zumutbar({"gesamtbetrag_der_einkuenfte": gde, "anzahl_kinder": 3, "splitting": False})
    zs = R.catala_p33_zumutbar({"gesamtbetrag_der_einkuenfte": gde, "anzahl_kinder": 0, "splitting": True})
    assert z0 > z2 > z3          # je mehr Kinder, desto niedriger
    assert zs < z0               # Splitting senkt die zumutbare Belastung


# ---- § 33 Abs. 1 agB-Abzug = agB − zumutbar, min 0 ----

def test_agb_abzug_zieht_zumutbar_ab(R):
    """agB über der zumutbaren Belastung → Abzug = agB − zumutbar (Money-genau, EIN Rundungsschritt)."""
    gde = 58770
    zum = R.catala_p33_zumutbar({"gesamtbetrag_der_einkuenfte": gde, "anzahl_kinder": 0, "splitting": False})
    abz = R.catala_p33_agb({"aussergewoehnliche_belastungen": 5000, "gesamtbetrag_der_einkuenfte": gde,
                            "anzahl_kinder": 0, "splitting": False})
    assert abz == 5000 - zum or abz == 5000 - zum - 1     # ±1 € durch die Money→Euro-Rundung der Tranche


def test_agb_unter_zumutbar_ist_null(R):
    """agB unterhalb der zumutbaren Belastung → Abzug 0 (kein Negativabzug, § 33 Abs. 1)."""
    abz = R.catala_p33_agb({"aussergewoehnliche_belastungen": 2000, "gesamtbetrag_der_einkuenfte": 58770,
                            "anzahl_kinder": 0, "splitting": False})
    assert abz == 0


def test_agb_kinder_erhoehen_abzug(R):
    """Dieselbe agB, mehr Kinder → niedrigere zumutbare Belastung → höherer Abzug."""
    base = dict(aussergewoehnliche_belastungen=5000, gesamtbetrag_der_einkuenfte=58770, splitting=False)
    assert R.catala_p33_agb({**base, "anzahl_kinder": 2}) > R.catala_p33_agb({**base, "anzahl_kinder": 0})


# ---- § 10 Abs. 1 Nr. 4 KiSt = gezahlt − erstattet ----

@pytest.mark.parametrize("gezahlt,erstattet,erwartet", [
    (1200, 200, 1000),          # regulär: gezahlt − erstattet
    (1200, 0, 1200),            # keine Erstattung
    (1200, 1200, 0),            # voll erstattet → 0 abziehbar
])
def test_kist_gezahlt_minus_erstattet(R, gezahlt, erstattet, erwartet):
    assert R.catala_p10_kist({"gezahlte_kirchensteuer": gezahlt, "erstattete_kirchensteuer": erstattet}) == erwartet


def test_kist_ueberhang_modul_gibt_null(R):
    """Erstattungsüberhang (erstattet > gezahlt): das Modul liefert 0 abziehbar. Das allein wäre
    Unterbesteuerung — die fehlende Gegenrichtung liefert catala_p10_4b_erstattungsueberhang
    (§ 10 Abs. 4b S. 3), siehe unten."""
    assert R.catala_p10_kist({"gezahlte_kirchensteuer": 200, "erstattete_kirchensteuer": 1200}) == 0


# ---- § 10 Abs. 4b S. 3 Erstattungsüberhang → GdE-Hinzurechnung ----

@pytest.mark.parametrize("gezahlt,erstattet,erwartet", [
    (200, 1200, 1000),      # Überhang: erstattet − gezahlt
    (1200, 200, 0),         # kein Überhang (der Normalfall)
    (1200, 1200, 0),        # exakt aufgezehrt — Grenze, kein Zuschlag
    (1200, 1201, 1),        # ein Euro darüber → Zuschlag beginnt
    (0, 500, 500),          # gar nichts gezahlt, nur erstattet
    (0, 0, 0),
])
def test_p10_4b_ueberhang(R, gezahlt, erstattet, erwartet):
    assert R.catala_p10_4b_erstattungsueberhang(
        {"gezahlte_kirchensteuer": gezahlt, "erstattete_kirchensteuer": erstattet}) == erwartet


def test_p10_4b_und_p10_kist_ueberschneiden_sich_nicht(R):
    """Die beiden Accessoren teilen sich denselben Sachverhalt und dürfen ihn nicht doppelt
    verwerten: entweder es gibt einen Abzug (gezahlt > erstattet) ODER eine Hinzurechnung.
    Nie beides — sonst würde derselbe Euro zweimal wirken."""
    for gezahlt, erstattet in [(1200, 200), (200, 1200), (1200, 1200), (0, 0), (500, 0)]:
        s = {"gezahlte_kirchensteuer": gezahlt, "erstattete_kirchensteuer": erstattet}
        abzug = R.catala_p10_kist(s)
        zuschlag = R.catala_p10_4b_erstattungsueberhang(s)
        assert not (abzug > 0 and zuschlag > 0), f"beides bei {s}: {abzug}/{zuschlag}"


def test_p10_4b_absente_felder_geben_null(R):
    """Ein Fall ohne Kirchensteuer-Angaben darf keinen Zuschlag erfinden."""
    assert R.catala_p10_4b_erstattungsueberhang({}) == 0
