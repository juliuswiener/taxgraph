"""P7.1 — DBA-Methodenwahl je (Staat, Einkunftsart).

Kein Abkommen wendet eine Methode auf alle Einkunftsarten an. Art. 24 Abs. 1 DBA-Polen 2003
stellt frei (Buchst. a), rechnet aber an für Dividenden (b aa) und für die Einkünfte nach
Art. 11 Abs. 2, 12 Abs. 2, 13 Abs. 2, 15 Abs. 3, 16 Abs. 1 und 17 (b bb).

Quelle: sources/dba/dba_pl_abkommen_2003.txt

Polen ist das ausgearbeitete Muster; die übrigen zehn Länder laufen weiter über die
pauschale DBA_METHOD_MAP. Diese Tests pinnen beides — die Polen-Einträge und die
Rückwärtskompatibilität für alles andere.
"""
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))

from api_constants import (  # noqa: E402
    DBA_EINKUNFTSARTEN, DBA_METHOD_MAP, DBA_METHOD_MAP_ART, DBA_STAAT_ISO,
    dba_methode_fuer, dba_staat_iso)


# ----------------------------------------------------------------- Polen (Muster)

@pytest.mark.parametrize("einkunftsart,erwartet", [
    ("unbewegliches_vermoegen", "freistellung"),     # Art. 24 Abs. 1 a
    ("unternehmensgewinne", "freistellung"),         # Art. 24 Abs. 1 a
    ("dividenden", "anrechnung"),                    # Art. 24 Abs. 1 b aa
    ("zinsen", "anrechnung"),                        # b bb (Art. 11 Abs. 2)
    ("lizenzgebuehren", "anrechnung"),               # b bb (Art. 12 Abs. 2)
    ("veraeusserungsgewinne", "freistellung"),       # a (nur Abs. 2 → b bb)
    ("unselbstaendige_arbeit", "freistellung"),      # a (nur Abs. 3 → b bb)
    ("aufsichtsratsverguetungen", "anrechnung"),     # b bb (Art. 16 Abs. 1)
    ("kuenstler_sportler", "anrechnung"),            # b bb (Art. 17)
    ("ruhegehaelter", "freistellung"),               # a
])
def test_polen_je_einkunftsart(einkunftsart, erwartet):
    assert dba_methode_fuer("pl", einkunftsart) == erwartet


def test_polen_weicht_von_pauschaler_methode_ab():
    """Ohne das Feature wäre für Polen alles 'anrechnung' — die Tabelle muss das ändern."""
    assert DBA_METHOD_MAP["pl"] == "anrechnung"
    assert dba_methode_fuer("pl", "unselbstaendige_arbeit") == "freistellung"
    assert dba_methode_fuer("pl") == "anrechnung", "ohne Einkunftsart bleibt es pauschal"


# ----------------------------------------------------------------- Rückwärtskompatibilität

@pytest.mark.parametrize("staat", sorted(DBA_METHOD_MAP))
def test_ohne_einkunftsart_bleibt_pauschale_methode(staat):
    assert dba_methode_fuer(staat) == DBA_METHOD_MAP[staat]


@pytest.mark.parametrize("staat", [s for s in sorted(DBA_METHOD_MAP) if s != "pl"])
def test_nicht_ausgearbeitete_laender_unveraendert(staat):
    """Für die zehn noch nicht adjudizierten Länder ändert die Einkunftsart nichts."""
    for art in DBA_EINKUNFTSARTEN:
        assert dba_methode_fuer(staat, art) == DBA_METHOD_MAP[staat]


def test_unbekannte_einkunftsart_faellt_auf_pauschal_zurueck():
    assert dba_methode_fuer("pl", "gibtsnicht") == DBA_METHOD_MAP["pl"]


def test_unbekannter_staat_ist_anrechnung():
    """Ohne Abkommensgrundlage gilt § 34c Abs. 1 EStG: Anrechnung, keine Freistellung."""
    assert dba_methode_fuer("xx") == "anrechnung"
    assert dba_methode_fuer("xx", "zinsen") == "anrechnung"


def test_kein_staat_ist_anrechnung():
    assert dba_methode_fuer(None) == "anrechnung"
    assert dba_methode_fuer("") == "anrechnung"


# ----------------------------------------------------------------- Normalisierung

def test_grossschreibung_und_leerzeichen_egal():
    assert dba_methode_fuer("PL", "ZINSEN") == "anrechnung"
    assert dba_methode_fuer("  pl  ", "  unselbstaendige_arbeit  ") == "freistellung"


# ----------------------------------------------------------------- Tabellen-Integrität

def test_tabelle_kennt_nur_deklarierte_einkunftsarten():
    for (_staat, art) in DBA_METHOD_MAP_ART:
        assert art in DBA_EINKUNFTSARTEN, f"{art!r} fehlt in DBA_EINKUNFTSARTEN"


def test_tabelle_kennt_nur_bekannte_laender():
    for (staat, _art) in DBA_METHOD_MAP_ART:
        assert staat in DBA_METHOD_MAP, f"{staat!r} hat keine pauschale Methode als Fallback"


def test_nur_gueltige_methoden():
    assert set(DBA_METHOD_MAP_ART.values()) <= {"anrechnung", "freistellung"}


def test_polen_ist_vollstaendig_abgedeckt():
    """Ein Teil-Mapping wäre irreführend — für das Muster-Land müssen alle Arten belegt sein."""
    fehlend = [a for a in DBA_EINKUNFTSARTEN if ("pl", a) not in DBA_METHOD_MAP_ART]
    assert not fehlend, f"Polen ohne Eintrag für: {fehlend}"


def test_nur_polen_ausgearbeitet():
    """Pinnt den dokumentierten Stand — neue Länder brauchen Adjudikation, nicht nur Code."""
    assert {s for (s, _a) in DBA_METHOD_MAP_ART} == {"pl"}


# ----------------------------------------------------------------- Enum ↔ ISO-Brücke
#
# REGRESSION: dba_staat führt deutsche Ländernamen ("Polen"), die Methoden-Tabellen
# ISO-Codes ("pl"). Ohne Auflösung traf KEIN Enum-Wert die Map — alles fiel auf den
# Anrechnungs-Default, auch Österreich und die USA, die Freistellungs-DBA sind.
# Der alte Test prüfte nur die Map-Struktur, nie den Weg vom Enum-Wert zur Methode.

ENUM_MIT_ADJUDIZIERTER_METHODE = [
    ("Oesterreich", "freistellung"),
    ("USA", "freistellung"),
    ("Polen", "anrechnung"),
    ("Frankreich", "anrechnung"),
    ("Schweiz", "anrechnung"),
    ("Dänemark", "anrechnung"),
    ("Spanien", "anrechnung"),
    ("Grossbritannien", "anrechnung"),
    ("Luxemburg", "anrechnung"),
    ("Niederlande", "anrechnung"),
    ("Türkei", "anrechnung"),
]


@pytest.mark.parametrize("enum_wert,erwartet", ENUM_MIT_ADJUDIZIERTER_METHODE)
def test_enum_wert_trifft_die_methode(enum_wert, erwartet):
    """Der Enum-Wert des Feldes muss die Methode treffen, nicht nur der ISO-Code."""
    assert dba_methode_fuer(enum_wert) == erwartet


def test_freistellungslaender_fallen_nicht_auf_anrechnung():
    """Der eigentliche Bug: Freistellungs-DBA als Anrechnung zu rechnen ist eine Fehlberechnung."""
    for land in ("Oesterreich", "USA"):
        assert dba_methode_fuer(land) == "freistellung", (
            f"{land} ist ein Freistellungs-DBA — Anrechnung wäre falsch gerechnet")


def test_enum_werte_ohne_adjudizierte_methode_sind_anrechnung():
    """Italien, Tschechien, Kanada, Deutschland: kein DBA-Eintrag → Default § 34c Abs. 1."""
    for land in ("Italien", "Tschechien", "Kanada", "Deutschland", "sonstiger_staat"):
        assert dba_methode_fuer(land) == "anrechnung"


def test_iso_code_funktioniert_weiterhin():
    """Aufrufer dürfen auch den ISO-Code übergeben."""
    assert dba_methode_fuer("at") == "freistellung"
    assert dba_methode_fuer("pl", "zinsen") == "anrechnung"


@pytest.mark.parametrize("name,iso", sorted(DBA_STAAT_ISO.items()))
def test_iso_brücke_zeigt_auf_bekannte_laender(name, iso):
    assert iso in DBA_METHOD_MAP, f"{name!r} → {iso!r} hat keine Methode"


def test_umlaut_varianten_werden_aufgeloest():
    """Das Enum schreibt 'Dänemark' und 'Türkei' — beide Schreibweisen müssen greifen."""
    assert dba_staat_iso("Dänemark") == dba_staat_iso("Daenemark") == "dk"
    assert dba_staat_iso("Türkei") == dba_staat_iso("Tuerkei") == "tr"
    assert dba_staat_iso("Österreich") == dba_staat_iso("Oesterreich") == "at"


def test_alle_enum_werte_des_feldes_sind_abgedeckt():
    """Jeder Enum-Wert von dba_staat muss eine definierte Methode ergeben — kein KeyError,
    kein stiller Sonderfall."""
    import pathlib
    import yaml
    pfad = pathlib.Path(ROOT) / "produkt" / "bindung" / "bindung_p34c_gesamt.yaml"
    bindung = yaml.safe_load(pfad.read_text(encoding="utf-8"))
    enum_werte = [b for b in bindung["bindungen"] if b["feld_id"] == "dba_staat"][0]["enum_werte"]
    for wert in enum_werte:
        assert dba_methode_fuer(wert) in {"anrechnung", "freistellung"}
