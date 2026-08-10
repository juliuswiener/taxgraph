"""Abnahme: Bankverbindungs-Baustein (Team-Lead-Briefing 2026-08-10).

Problem, das dieser Baustein beseitigt: das Produkt erklaerte bisher UNGEFRAGT und
UNBEDINGT fuer jeden Nutzer "keine Bankverbindung vorhanden" (E0102002) -- faktisch falsch
fuer jeden mit Konto. Gemessen (checkESt, ERiC 44.2.4.0): ELSTER akzeptiert kein Schweigen
(weder-noch -> rc=610001002) und keinen Widerspruch (beides gleichzeitig -> rc=610001002).

Exklusivitaets-Mechanik, zweigeteilt (s. Kommentare in est_mapping.py / elster_xml.py):
  - "beides gleichzeitig bestaetigt" ist ein WIDERSPRUCH IN VORHANDENEN DATEN -> gehoert in
    deklariere() (unvollstaendig/vollstaendig), praesenzsicher wie der Rest der Funktion.
  - "weder noch bestaetigt" ist eine ABWESENHEITS-Pruefung -- die darf NICHT unconditional in
    deklariere() sitzen (sonst wird jeder Snapshot ohne Bankverbindungs-Bezug faelschlich
    unvollstaendig, s. Regression an test_multi_objekt_vv_zwei_objekte & Co. beim ersten
    Versuch). Sie sitzt deshalb auf dem echten Abgabe-Pfad in erzeuge_xml(abgabefaehig=True),
    symmetrisch zum dortigen Absender-Pflichtfelder-Gate.

IBAN-Pruefziffer (ISO 13616, Modulo 97) ist eine eigene Validierung ueber das XSD-Muster
hinaus -- eine strukturell XSD-valide IBAN mit falscher Pruefziffer ist amtlich trotzdem
falsch. PII-Disziplin wie bei der Steuernummer (_STNR_PATTERN): der IBAN-Wert selbst darf in
keiner Fehlermeldung auftauchen.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import elster_xml as EX       # noqa: E402
import est_mapping as EM      # noqa: E402
import store as ST            # noqa: E402
import traverser as TR        # noqa: E402

HID = "74931"
TS = "2026-08-10T09:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

# Offizielles Beispiel (Bundesbank/Wikipedia), Pruefziffer korrekt (gemessen via _iban_pruefziffer_gueltig).
IBAN_INLAND = "DE02120300000000202051"
# Oesterreich-Beispiel, Pruefziffer korrekt -- fuer den Ausland-Zweig (E0102603).
IBAN_AUSLAND = "AT611904300234573201"
BIC_AUSLAND = "PBNKDEFF370"


def _b(s, feld_id, wert, zustand="bestaetigt"):
    sig = ({"signal_1": None, "signal_2": f"ok@{feld_id}"} if zustand == "bestaetigt"
           else {"signal_1": None, "signal_2": None})
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand=zustand,
                    herkunft=H, schreiber="ui:laie", signal=sig, ts=TS)


def _flags_einzel(s):
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)


def _stammdaten_ohne_bankverbindung(s):
    """Alle Stammdaten ausser der Bankverbindungs-Entscheidung -- die setzt jeder Test selbst."""
    _b(s, "stammdaten_nachname", "Maier")
    _b(s, "stammdaten_vorname", "Hans")
    _b(s, "stammdaten_geburtsdatum", "05.05.1955")
    _b(s, "stammdaten_strasse", "Musterstr.")
    _b(s, "stammdaten_hausnummer", "55")
    _b(s, "stammdaten_plz", "55555")
    _b(s, "stammdaten_wohnort", "Musterort")
    _b(s, "stammdaten_art_est_erklaerung", True)


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


# ---------------------------------------------------------------- IBAN-Variante (inland)

def test_iban_inland_wird_deklariert(bindung):
    s = ST.leerer_store(2025, fall_id="bv_iban_inland")
    _b(s, "stammdaten_iban", IBAN_INLAND)
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    dekl = result["deklaration"]

    assert dekl["E0102102"] == IBAN_INLAND
    assert "E0102603" not in dekl
    assert dekl["E0101601"] is True, "Kontoinhaber wird bei bestaetigter IBAN auto-abgeleitet"
    assert result["vollstaendig"] is True


def test_iban_leerzeichen_werden_entfernt_und_normalisiert(bindung):
    """Laien tippen IBANs oft mit Leerzeichen -- die hilfe_kurz verspricht 'werden automatisch
    entfernt'; wenn die Normalisierung fehlt, blockiert das XSD-Muster jede so eingegebene IBAN."""
    s = ST.leerer_store(2025, fall_id="bv_iban_leerzeichen")
    _b(s, "stammdaten_iban", "DE02 1203 0000 0000 2020 51")
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert result["deklaration"]["E0102102"] == IBAN_INLAND
    assert result["vollstaendig"] is True


# ---------------------------------------------------------------- IBAN-Variante (ausland + BIC)

def test_iban_ausland_mit_bic(bindung):
    s = ST.leerer_store(2025, fall_id="bv_iban_ausland")
    _b(s, "stammdaten_iban", IBAN_AUSLAND)
    _b(s, "stammdaten_bic", BIC_AUSLAND)
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    dekl = result["deklaration"]

    assert dekl["E0102603"] == IBAN_AUSLAND
    assert "E0102102" not in dekl
    assert dekl["E0102201"] == BIC_AUSLAND
    assert dekl["E0101601"] is True
    assert result["vollstaendig"] is True


def test_iban_ausland_ohne_bic_bleibt_vollstaendig(bindung):
    """Gemessen (checkESt, ERiC 44.2.4.0, scripts/measure_bankverbindung.py Fall 3b): eine
    Auslands-IBAN OHNE BIC wird von checkESt anstandslos akzeptiert (rc=0). BIC ist bei
    auslaendischer IBAN KEIN Pflichtfeld -- checkESt prueft nur, WENN ein BIC mitkommt, ob er
    zur IBAN passt (Laendercode Stellen 5-6). Kein fail-closed hier, das waere unbegruendete
    Blockade eines von ELSTER akzeptierten Falls."""
    s = ST.leerer_store(2025, fall_id="bv_iban_ausland_ohne_bic")
    _b(s, "stammdaten_iban", IBAN_AUSLAND)
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)

    assert result["vollstaendig"] is True
    assert result["deklaration"]["E0102603"] == IBAN_AUSLAND
    assert "E0102201" not in result["deklaration"]


# ---------------------------------------------------------------- Sonderfeld-Variante

def test_keine_bankverbindung_variante(bindung):
    s = ST.leerer_store(2025, fall_id="bv_keine")
    _b(s, "stammdaten_keine_bankverbindung", True)
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert result["deklaration"]["E0102002"] is True
    assert "E0102102" not in result["deklaration"]
    assert "E0102603" not in result["deklaration"]
    assert result["vollstaendig"] is True


# ---------------------------------------------------------------- Exklusivitaet: beides gleichzeitig

def test_iban_und_keine_bankverbindung_gleichzeitig_fail_closed(bindung):
    """Gemessen (checkESt, ERiC 44.2.4.0): IBAN UND 'keine Bankverbindung' gleichzeitig
    bestaetigt -> rc=610001002 (Widerspruch). deklariere() muss das VOR jedem XML-Versuch fangen."""
    s = ST.leerer_store(2025, fall_id="bv_widerspruch")
    _b(s, "stammdaten_iban", IBAN_INLAND)
    _b(s, "stammdaten_keine_bankverbindung", True)
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)

    assert result["vollstaendig"] is False
    gruende = [u["feld_id"] for u in result["unvollstaendig"]]
    assert "stammdaten_iban" in gruende


# ---------------------------------------------------------------- Exklusivitaet: weder noch (Abgabe-Pfad)

def test_weder_iban_noch_keine_bankverbindung_deklariere_bleibt_unberuehrt(bindung):
    """deklariere() selbst darf NICHT unconditional auf Abwesenheit pruefen (s. Moduldoc) --
    ein Snapshot, der Bankverbindung gar nicht anfasst, bleibt bei deklariere() vollstaendig.
    Die 'kein Schweigen'-Pflicht sitzt auf dem Abgabe-Pfad (naechster Test)."""
    s = ST.leerer_store(2025, fall_id="bv_schweigen_dekl")
    _stammdaten_ohne_bankverbindung(s)
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert result["vollstaendig"] is True
    assert "E0102002" not in result["deklaration"]
    assert "E0102102" not in result["deklaration"]
    assert "E0102603" not in result["deklaration"]


def test_weder_iban_noch_keine_bankverbindung_blockiert_abgabe(bindung):
    """Gemessen (checkESt, ERiC 44.2.4.0): weder IBAN noch 'keine Bankverbindung' bestaetigt
    -> rc=610001002 ('Bitte geben Sie Ihre Bankverbindungsdaten an oder erklaeren Sie in dem
    vorgesehenen Sonderfeld, dass keine Bankverbindung vorhanden ist'). abgabefaehig=True muss
    das VOR jedem checkESt-Aufruf abfangen -- kein stiller Default."""
    s = ST.leerer_store(2025, fall_id="bv_schweigen_xml")
    _stammdaten_ohne_bankverbindung(s)
    _b(s, "kist_konfession", "keine")
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert result["vollstaendig"] is True  # deklariere() selbst sieht keinen Fehler (s.o.)

    with pytest.raises(EX.XmlFehler, match="Bankverbindungs-Entscheidung"):
        EX.erzeuge_xml(result, vz=2025, hersteller_id=HID, abgabefaehig=True,
                       absender_name="Hans Maier", absender_strasse="Musterstr. 55",
                       absender_plz="55555", absender_ort="Musterort",
                       absender_steuernummer="9198012340010")


def test_nicht_abgabefaehig_erlaubt_schweigen(bindung):
    """Ohne abgabefaehig=True (Vorschau/Entwicklung) bleibt Schweigen ueber Bankverbindung
    erlaubt -- die Pflicht gilt nur fuer den echten Abgabe-Pfad, analog zu den
    Absender-Stammdaten (die ebenfalls nur mit abgabefaehig=True Pflicht sind)."""
    s = ST.leerer_store(2025, fall_id="bv_schweigen_preview")
    _stammdaten_ohne_bankverbindung(s)
    _b(s, "kist_konfession", "keine")
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)  # abgabefaehig default False
    assert "<E0102002>" not in xml.replace("ns0:", "").replace("ns1:", "")


# ---------------------------------------------------------------- Format- und Pruefziffer-Validierung

def test_iban_falsches_muster_fail_closed(bindung):
    s = ST.leerer_store(2025, fall_id="bv_iban_kaputtes_muster")
    _b(s, "stammdaten_iban", "1E02120300000000202051")  # Laendercode kein Buchstabe -> Musterverstoss
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert result["vollstaendig"] is False
    gruende = [u["grund"] for u in result["unvollstaendig"] if u["feld_id"] == "stammdaten_iban"]
    assert gruende and "Muster" in gruende[0]


def test_iban_falsche_pruefziffer_fail_closed(bindung):
    """Strukturell XSD-valide (2 Buchstaben + 2 Ziffern + alphanumerisch), aber Modulo-97-
    Pruefziffer falsch (letzte Ziffer von IBAN_INLAND mutiert) -- muss trotzdem fail-closed sein,
    das XSD-Muster allein faengt das NICHT (s. _iban_pruefziffer_gueltig-Docstring)."""
    kaputt = IBAN_INLAND[:-1] + ("0" if IBAN_INLAND[-1] != "0" else "1")
    s = ST.leerer_store(2025, fall_id="bv_iban_kaputte_pruefziffer")
    _b(s, "stammdaten_iban", kaputt)
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert result["vollstaendig"] is False
    gruende = [u["grund"] for u in result["unvollstaendig"] if u["feld_id"] == "stammdaten_iban"]
    assert gruende and "Pruefziffer" in gruende[0]
    assert kaputt not in gruende[0], "PII: der IBAN-Wert selbst darf nicht in der Fehlermeldung stehen"


def test_iban_pii_nie_in_fehlermeldung(bindung):
    """Wie bei der Steuernummer (_STNR_PATTERN-Fehlermeldungen): der eingegebene IBAN-Wert darf
    in KEINER Fehlermeldung (unvollstaendig/nicht_deklariert) auftauchen, auch nicht teilweise --
    weder bei Muster- noch bei Pruefziffer-Verstoss."""
    faelle = ["XX-offensichtlich-falsches-muster", IBAN_INLAND[:-1] + "9"]
    for wert in faelle:
        s = ST.leerer_store(2025, fall_id=f"bv_pii_{hash(wert)}")
        _b(s, "stammdaten_iban", wert)
        _flags_einzel(s)
        snap, _ = ST.materialisiere(s)
        result = EM.deklariere(snap, bindung)
        alle_gruende = " ".join(u["grund"] for u in result["unvollstaendig"])
        alle_gruende += " ".join(n["grund"] for n in result["nicht_deklariert"])
        assert wert.upper().replace(" ", "") not in alle_gruende.upper(), (
            f"IBAN-Wert ist in einer Fehlermeldung gelandet: {wert!r}")


# ---------------------------------------------------------------- Mutations-Waechter

def test_iban_pruefziffer_funktion_mutationswaechter():
    """Mutations-Beweis: die bekannte Test-IBAN muss gueltig sein, eine einstellig mutierte
    Pruefziffer (Positionen 2-3) muss ungueltig sein. Haelt _iban_pruefziffer_gueltig() ehrlich,
    falls jemand die Modulo-97-Logik versehentlich zu 'immer True' vereinfacht."""
    assert EM._iban_pruefziffer_gueltig(IBAN_INLAND) is True
    mutiert = IBAN_INLAND[:2] + "03" + IBAN_INLAND[4:]
    assert EM._iban_pruefziffer_gueltig(mutiert) is False


def test_iban_muster_regex_mutationswaechter():
    assert EM._IBAN_PATTERN.match(IBAN_INLAND) is not None
    assert EM._IBAN_PATTERN.match("D302120300000000202051") is None  # Laendercode nicht 2 Buchstaben
    assert EM._IBAN_PATTERN.match("DEAB120300000000202051") is None  # Pruefziffern nicht 2 Ziffern
