"""Unit-Golden für catala_kist (§ 51a EStG i.V.m. Landes-KiStG, KiSt-Festsetzung).

Deterministisch, NULL LLM. est_mit_fb = EURO (Maßstabsteuer), Rückgabe = CENT (EURO×Hebesatz-int).
Kernrisiko: BW-Hebesatz 8 % (nicht 9 %) — Typo im Länder-Tupel würde still über-versteuern (Over-tax).
Deshalb pinnt Branch (2) BW=8 % explizit. Pipeline-verifiziert (snapshot p51a_kirchensteuer).
"""
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "golden"))

import runner as R  # noqa: E402


def _kist(est_mit_fb, konfession, bundesland):
    return R.catala_kist({"est_mit_fb": est_mit_fb, "konfession": konfession,
                          "bundesland": bundesland})


def test_ev_bayern_8_prozent():
    # ev in Bayern: 10.000 € × 8 % = 800,00 € = 80000 Cent
    assert _kist(10000, "evangelisch", "bayern") == 80000


def test_rk_baden_wuerttemberg_8_prozent_typo_guard():
    # rk in Baden-Württemberg: 8 % (NICHT 9 %). Falsche Tupel-Schreibweise → 9 % → Over-tax.
    assert _kist(10000, "roemisch-katholisch", "baden_wuerttemberg") == 80000


def test_rk_nrw_9_prozent():
    # rk in NRW (übriges Land): 10.000 € × 9 % = 900,00 € = 90000 Cent
    assert _kist(10000, "roemisch-katholisch", "nordrhein_westfalen") == 90000


def test_ev_nrw_9_prozent():
    # evangelisch ist ebenfalls steuererhebend → 9 % in NRW
    assert _kist(10000, "evangelisch", "nordrhein_westfalen") == 90000


def test_konfessionslos_null():
    assert _kist(10000, "keine", "nordrhein_westfalen") == 0


def test_andere_religionsgemeinschaft_null():
    # andere (nicht steuererhebende) Religionsgemeinschaft → satz 0
    assert _kist(10000, "andere", "bayern") == 0


def test_fehlende_konfession_fail_closed():
    # Default (Feld nie gesetzt) → keine KiSt (fail-closed, kein erfundener Betrag)
    assert R.catala_kist({"est_mit_fb": 10000}) == 0


def test_cent_exakt_kein_rundungsschnitt():
    # ungerade EURO-Basis: 12345 € × 9 % = 1.111,05 € = 111105 Cent (exakt, kein Schnitt)
    assert _kist(12345, "roemisch-katholisch", "hessen") == 111105
