"""Test: checkest_gate pure rc-Klassifikation — KEIN ERiC noetig.

Die Falsch-Gruen-Regel (api.py:einreichen): "ein leerer Fehlerpuffer bei rc!=0 heisst
'nicht geprueft', nicht 'fehlerfrei'". klassifiziere_rc muss das halten:
RC_IO_KEIN_TICKET (610301200) liefert 0 Fehler im Puffer, darf aber NIE als gruen
durch - es ist "io_gate_nicht_geprueft", nicht "plausibel".

Nur deterministische Pure-Logik (rc->klasse, Kappungs-Verdacht) wird hier getestet;
validate() selbst braucht ERiC und wird in tests/test_einreichen.py ueber die
Mock-Naht abgedeckt.
"""
import sys
import os

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "elster"))

import checkest_gate as CE  # noqa: E402


# -- klassifiziere_rc: die 4 bekannten Klassen + sonstig ---------------------

def test_rc_ok_ist_plausibel():
    assert CE.klassifiziere_rc(CE.RC_OK) == "plausibel"


def test_rc_plausibilitaet():
    assert CE.klassifiziere_rc(CE.RC_PLAUSIBILITAET) == "plausibilitaet_fehler"


def test_rc_io_kein_ticket_nie_gruen():
    """Falsch-Gruen-Kern: rc=610301200 liefert 0 Fehler im Puffer, ist aber NICHT geprueft.
    Muss 'io_gate_nicht_geprueft' sein, nie 'plausibel'."""
    assert CE.klassifiziere_rc(CE.RC_IO_KEIN_TICKET) == "io_gate_nicht_geprueft"
    assert CE.klassifiziere_rc(CE.RC_IO_KEIN_TICKET) != "plausibel"


def test_rc_hersteller_gesperrt():
    assert CE.klassifiziere_rc(CE.RC_HERSTELLER_GESPERRT) == "hersteller_id_gesperrt"


def test_unbekannter_rc_sonstig():
    """Unbekannter rc -> 'sonstig' (fail-closed, kein stilles gruen)."""
    assert CE.klassifiziere_rc(999) == "sonstig"
    assert CE.klassifiziere_rc(-1) == "sonstig"


# -- gekappt_verdacht: Trunkierungs-Doktrin ---------------------------------

def test_gekappt_verdacht_unter_cap():
    assert CE.gekappt_verdacht("<FehlerRegelpruefung>" * 999) is False


def test_gekappt_verdacht_am_cap():
    """Genau 1000 Fehler = am angehobenen Cap -> Verdacht (kann weiter gekappt sein)."""
    assert CE.gekappt_verdacht("<FehlerRegelpruefung>" * 1000) is True


def test_gekappt_verdacht_ueber_cap():
    assert CE.gekappt_verdacht("<FehlerRegelpruefung>" * 1001) is True


def test_gekappt_verdacht_leer():
    assert CE.gekappt_verdacht("") is False


def test_gekappt_verdacht_ohne_fehler():
    assert CE.gekappt_verdacht("<Hinweise/>") is False


# -- rc-Konstanten sind stabil (Falsch-Gruen-Sperre bricht sonst) ------------

def test_rc_konstanten_distinkt():
    """Alle rc-Konstanten muessen verschieden sein, sonst kollidiert die Klasse."""
    s = {CE.RC_OK, CE.RC_PLAUSIBILITAET, CE.RC_IO_KEIN_TICKET, CE.RC_HERSTELLER_GESPERRT,
         CE.RC_DATENARTVERSION_UNBEKANNT, CE.RC_IO_UNERWARTETE_ELEMENTE}
    assert len(s) == 6


def test_validate_braucht_eric_oder_skip():
    """validate() ohne ERiC darf nicht still nichts tun — es laedt oder wirft.

    Mit ERiC vorhanden laeuft es; ohne ERiC wirft es RuntimeError (nie still 0,0).
    Wir testen, dass der ERiC-Load-Pfad existiert und robust skip-baid ist.
    """
    lib = CE.find_eric_lib()
    if lib is None:
        pytest.skip("kein lokales ERiC (find_eric_lib leer) — validate() nicht testbar")
    # validate() wird via _load_and_init indirekt getestet; hier nur der Pfad.
    assert os.path.exists(lib), f"find_eric_lib meldet {lib}, existiert aber nicht"

# -- ERIC_GLOBAL_DATENARTVERSION_UNBEKANNT: kein Pruefmodul fuer den VZ -------

def test_rc_datenartversion_unbekannt_ist_eigene_klasse():
    """ESt_2026 hat in ERiC 44.2.4.0 kein Pruefmodul — das ist NICHT "sonstig".

    Gemessen 2026-08-09 (reports/adjudikation/vz_matrix_2026-08-09.md): das Produkt
    rechnet VZ 2026 (params/2026 vollstaendig, fall_anlegen nimmt jede Jahreszahl),
    ERiC liefert dafuer aber kein libcheckESt_2026.so und antwortet mit 610001042.
    Ohne eigene Klasse fiel der Code auf "sonstig", und der Endpunkt meldete
    "plausibilitaet_verletzt" — eine Falschaussage ueber die Erklaerung des Nutzers,
    die in Wahrheit nie geprueft wurde.
    """
    assert CE.klassifiziere_rc(CE.RC_DATENARTVERSION_UNBEKANNT) == "datenartversion_unbekannt"
    assert CE.klassifiziere_rc(CE.RC_DATENARTVERSION_UNBEKANNT) != "sonstig"
    assert CE.klassifiziere_rc(CE.RC_DATENARTVERSION_UNBEKANNT) != "plausibilitaet_fehler"


def test_datenartversion_unbekannt_ist_nicht_geprueft():
    """Wie RC_IO_KEIN_TICKET: leerer Fehlerpuffer, aber kein Pruefergebnis."""
    assert CE.klassifiziere_rc(CE.RC_DATENARTVERSION_UNBEKANNT) != "plausibel"
    assert CE.RC_DATENARTVERSION_UNBEKANNT != CE.RC_OK


# -- ERIC_IO_READER_UNERWARTETE_ELEMENTE (610301106): eigener Eintrag, kein "sonstig" mehr --

def test_rc_io_unerwartete_elemente_ist_eigene_klasse():
    """Gemessen (Vollsweep, scripts/_vollsweep_rohdaten.json): person_b_idnr (E0100082)
    -> rc=610301106, ERiC lehnt die Eingabedatei am XML-Reader ab. Ohne eigenen Eintrag
    fiel der Code auf "sonstig", genau die Blindstelle wie bei RC_DATENARTVERSION_UNBEKANNT."""
    assert CE.klassifiziere_rc(CE.RC_IO_UNERWARTETE_ELEMENTE) == "io_reader_unerwartete_elemente"
    assert CE.klassifiziere_rc(CE.RC_IO_UNERWARTETE_ELEMENTE) != "sonstig"
    assert CE.klassifiziere_rc(CE.RC_IO_UNERWARTETE_ELEMENTE) != "plausibilitaet_fehler"
    assert CE.klassifiziere_rc(CE.RC_IO_UNERWARTETE_ELEMENTE) != "plausibel"


# -- unerwarteter_rc_hinweis: Fail-closed-Generalisierung fuer JEDEN Nicht-Plausibilitaets-rc --

def test_unerwarteter_rc_hinweis_nennt_rc_und_klasse_roh():
    """Der rohe rc und die Klasse muessen im Klartext stehen — auch fuer einen rc, den
    klassifiziere_rc gar nicht kennt. Das ist der Kern von "faellt nie in einen Sammeleimer,
    der wie 'nichts Schlimmes' aussieht"."""
    hinweis = CE.unerwarteter_rc_hinweis(999999, "")
    assert "999999" in hinweis
    assert "sonstig" in hinweis
    assert "nicht inhaltlich geprueft" in hinweis


def test_unerwarteter_rc_hinweis_leerer_puffer_nennt_eric_log():
    """rc!=0 mit leerem Puffer -> eric.log muss im Ergebnis benannt sein (Auftrag Punkt 2).
    Ohne laufendes ERiC ist der Pfad nicht ermittelbar, aber der Hinweis muss das SAGEN,
    statt eric.log einfach zu verschweigen."""
    hinweis = CE.unerwarteter_rc_hinweis(CE.RC_IO_UNERWARTETE_ELEMENTE, "")
    assert "eric.log" in hinweis


def test_unerwarteter_rc_hinweis_gefuellter_puffer_kein_eric_log_zwang():
    """Steht der Grund schon im Rueckgabepuffer (<Text>...), muss nicht extra auf eric.log
    verwiesen werden — der Puffer selbst ist dann die Quelle."""
    hinweis = CE.unerwarteter_rc_hinweis(CE.RC_HERSTELLER_GESPERRT, "<Text>Hersteller-ID gesperrt</Text>")
    assert "610301202" in hinweis
    assert "hersteller_id_gesperrt" in hinweis


def test_eric_log_pfad_ohne_init_none():
    """Vor jedem validate()-Aufruf ist kein log_dir bekannt -> None, nicht raten."""
    CE._STATE["log_dir"] = None
    assert CE.eric_log_pfad() is None
