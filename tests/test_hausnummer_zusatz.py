"""Der Hausnummerzusatz hat ein Bindungsfeld (Vault backlog/taxgraph/hausnummer-zusatz-versprochen-aber-
nicht-einloesbar, Entscheidung hausnummer-zusatz-bekommt-ein-bindungsfeld).

Vorher: `stammdaten_hausnummer` verspricht in `hilfe_kurz` ein „Zusatzfeld" (ELSTER E0101207, Laenge 1-6),
das es nicht gab. Der XML-Zweig `_leite_absender_ab` hing den Zusatz an `absender_strasse` — mit Kz
`E0101207`, das kein Feld je lieferte (toter Zweig). Wer „12a" tippte, wurde mit 422 abgewiesen
(`stammdaten_hausnummer` nimmt nur `[0-9]{1,4}`).

Jetzt traegt `stammdaten_hausnummerzusatz` den Kz `E0101207`, optional, aus dem Vorjahr uebernehmbar.
Ein Zusatz ist keine Hausnummer, kein Pflicht-Gate, und der alte Wert in `stammdaten_hausnummer` wird nicht
umgeschrieben.

    AK1  das Feld steht in der Tabelle (Typ, Kz, Muster, Vorjahr) und in den Scheiben, die Stammdaten fuehren
    AK2  Zusatz „a" neben Hausnummer „12": deklariere() und XML tragen E0101207='a' neben E0101206='12'
    AK3  Grenzen: „abcdef" geht, „abcdefg", der leere Text und ein Zeilenumbruch gehen nicht
    AK4  der Zusatz erreicht `absender_strasse` (<AbsStr>) ueber den Weg Store -> deklariere -> erzeuge_xml

Das Rust-Gegenstueck mit denselben Faellen: rust/elster/tests/hausnummer_zusatz.rs (hermetisch, ohne PARITY)."""
from __future__ import annotations

import json
import os
import re
import sys
import urllib.error
import urllib.request

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/eingang", "produkt/mapping", "produkt/store", "produkt/traverser", "produkt/haut",
             "tools/parity"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api_constants as AC  # noqa: E402
import elster_xml as EX  # noqa: E402
import est_mapping as EM  # noqa: E402
import store as ST  # noqa: E402
import traverser as TR  # noqa: E402
import xsd_verify as XV  # noqa: E402
from test_paket_b_e2e_http import _laie, base  # noqa: E402,F401 — Fixture

FELD = "stammdaten_hausnummerzusatz"
KZ = "E0101207"
HID = "74931"          # ERiC-Test-Hersteller-ID aus dem amtlichen Beispiel-XML
STNR = "9181081508155"
BINDUNG = TR.lade_bindung()
KATALOG = ST.lade_katalog(BINDUNG)
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

requires_real_schema = pytest.mark.skipif(
    XV._find_schema(2025) is None, reason="lokales ERiC-E10-2025.xsd nicht gefunden ($ERIC_DIR/~/02_Software/eric)")


def _b(s, feld_id, wert):
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand="bestaetigt", herkunft=H, schreiber="ui:laie",
                    signal={"signal_1": None, "signal_2": f"ok@{feld_id}"}, ts="2026-10-03T09:00:00+00:00",
                    bindung=BINDUNG, katalog=KATALOG)


def _fall(zusatz):
    """Leerer Fall mit den Stammdaten; `zusatz=None` heisst: das Feld bleibt unbeantwortet."""
    s = ST.leerer_store(2025, fall_id="hnz")
    for feld, wert in [("stammdaten_nachname", "Maier"), ("stammdaten_vorname", "Hans"),
                       ("stammdaten_geburtsdatum", "05.05.1955"), ("stammdaten_strasse", "Musterstr."),
                       ("stammdaten_hausnummer", "12"), ("stammdaten_plz", "55555"),
                       ("stammdaten_wohnort", "Musterort"), ("stammdaten_keine_bankverbindung", True),
                       ("stammdaten_art_est_erklaerung", True), ("kist_konfession", "keine"), ("veranlagung", "einzel"),
                       ("kein_gewinn", True),
                       ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True)]:
        _b(s, feld, wert)
    if zusatz is not None:
        _b(s, FELD, zusatz)
    return s


def _deklariere(s):
    felder, _ = ST.materialisiere(s)
    return felder, EM.deklariere(felder, BINDUNG, vz=2025)


# ----------------------------------------------------------------------------------------------- AK1

def test_ak1_bindungsfeld_steht_in_der_tabelle():
    b = BINDUNG.get(FELD)
    assert b is not None, f"{FELD} fehlt in der Bindungstabelle"
    assert b["typ"] == "text"
    assert b["elster_kz"] == KZ
    assert b["vorjahr"] == "uebernehmbar"
    assert b["askable"] is True
    assert b.get("pflicht") is not True            # optional: ein leerer Zusatz ist kein Pflichtkonflikt
    assert re.fullmatch(b["muster"], "abcdef") and not re.fullmatch(b["muster"], "abcdefg")


def test_ak1_scheiben_mit_stammdaten_fuehren_das_feld():
    """Ohne den Eintrag in `STAMMDATEN_FELDER` lehnt POST /event das Feld mit 400 „nicht in dieser Scheibe"
    ab; mit ihm wird auch einreichen() es von jeder abgabefaehigen Scheibe verlangen koennen."""
    assert FELD in AC.STAMMDATEN_FELDER
    for scheibe in ("gesamt", "rentner_gesamt"):
        assert FELD in AC.SCHEIBEN[scheibe]["felder"], scheibe


@requires_real_schema
def test_ak1_muster_deckt_sich_mit_dem_schema():
    """E10-2025.xsd: E0101207 ist String_MinL1_MaxL6 mit dem Muster `[^\\n\\r]+`. Das Muster der Bindung
    urteilt auf jedem Probewert wie das Schema (Laenge 1..6 und Muster), nicht strenger und nicht laxer."""
    meta = XV._resolve_kz_meta(XV._find_schema(2025), "E10")[KZ]
    lo, hi = map(int, re.search(r"MinL(\d+)_MaxL(\d+)", meta["type_name"]).groups())
    assert (lo, hi) == (1, 6) and meta["patterns"] == ["[^\n\r]+"]
    muster = BINDUNG[FELD]["muster"]
    for wert in ["", "a", "abcdef", "abcdefg", "a\nb", "a\rb", "a\n", "\n", "Äöü ß-1", "123456", "1234567"]:
        schema_ok = lo <= len(wert) <= hi and all(re.fullmatch(p, wert) for p in meta["patterns"])
        assert bool(re.fullmatch(muster, wert)) == schema_ok, repr(wert)


# ----------------------------------------------------------------------------------------------- AK2

def test_ak2_zusatz_a_erscheint_neben_der_hausnummer_in_deklaration_und_xml():
    felder, result = _deklariere(_fall("a"))
    assert result["deklaration"]["E0101206"] == "12"
    assert result["deklaration"][KZ] == "a"
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID).replace("ns0:", "").replace("ns1:", "")
    assert "<E0101206>12</E0101206>" in xml
    assert "<E0101207>a</E0101207>" in xml
    assert xml.index("<E0101206>") < xml.index("<E0101207>")      # die Reihenfolge des Schemas


def test_ak2_ohne_zusatz_steht_kein_e0101207_im_xml():
    """Optional: wer nichts antwortet, bekommt kein leeres Element (XSD: MinL1)."""
    _, result = _deklariere(_fall(None))
    assert KZ not in result["deklaration"]
    assert "E0101207" not in EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)


# ----------------------------------------------------------------------------------------------- AK3

def test_ak3_sechs_zeichen_gehen():
    assert FELD in BINDUNG                  # ein unbekanntes Feld liesse der Store ohne Pruefung durch
    s = _fall(None)
    _b(s, FELD, "abcdef")
    assert ST.materialisiere(s)[0][FELD]["wert"] == "abcdef"


@pytest.mark.parametrize("wert", ["abcdefg", "", "a\nb", "a\rb", "a\n"])
def test_ak3_sieben_zeichen_leer_und_zeilenumbruch_gehen_nicht(wert):
    """Die Abweisung kommt vom Muster („Format"); den leeren Text lehnt schon die Typpruefung ab, die jedes
    Textfeld trifft („Typ"). Ein unbekanntes Feld liesse der Store durch."""
    assert FELD in BINDUNG
    s = _fall(None)
    with pytest.raises(ValueError, match=r"fail-closed \((Format|Typ)\)"):
        _b(s, FELD, wert)
    assert FELD not in ST.materialisiere(s)[0]                   # der abgewiesene Wert liegt nicht in der Akte


def _roh(base, method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def test_ak3_post_event_nimmt_sechs_zeichen_und_weist_den_rest_ab(base):
    """Die Grenzen am Weg des Nutzers: POST /event auf der Scheibe `gesamt`. Ein Fall je Wert, damit ein
    zweites Schreiben nicht an Auflage B (Ueberschreiben braucht `ersetzt`) scheitert."""
    faelle = [("abcdef", 201), ("a", 201), ("abcdefg", 422), ("", 422), ("a\nb", 422), ("a\rb", 422)]
    for i, (wert, soll) in enumerate(faelle):
        fid = f"hnz{i}"
        assert _roh(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})[0] == 201
        status, antwort = _roh(base, "POST", f"/fall/{fid}/event", _laie(FELD, wert))
        assert status == soll, (wert, status, antwort)
        if soll == 422:
            assert re.search(r"fail-closed \((Format|Typ)\)", json.dumps(antwort, ensure_ascii=False)), (wert, antwort)


# ----------------------------------------------------------------------------------------------- AK4

@requires_real_schema
def test_ak4_der_zusatz_erreicht_absender_strasse():
    """Der Weg von api.einreichen(): Store -> deklariere -> erzeuge_xml(abgabefaehig=True, snapshot=felder).
    Der XML-Zweig in elster_xml._leite_absender_ab war bis dahin tot: kein Feld lieferte E0101207."""
    felder, result = _deklariere(_fall("a"))
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID, abgabefaehig=True, snapshot=felder,
                         absender_steuernummer=STNR)
    assert "<AbsStr>Musterstr. 12a</AbsStr>" in xml
    felder_ohne, ohne = _deklariere(_fall(None))
    xml_ohne = EX.erzeuge_xml(ohne, vz=2025, hersteller_id=HID, abgabefaehig=True, snapshot=felder_ohne,
                              absender_steuernummer=STNR)
    assert "<AbsStr>Musterstr. 12</AbsStr>" in xml_ohne
