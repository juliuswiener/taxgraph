"""P3.2 XML-Writer: Deklaration -> ELSTER-Submission-XML.

Gate ist das amtliche E10-XSD (via xmllint). Die Positivtests beweisen, dass der Writer
schema-valides XML baut; die Negativtests, dass er fail-closed bleibt (unvollständige
Deklaration, fehlende Hersteller-ID, unbekannte Kz -> kein XML statt kaputtes XML).
"""
from __future__ import annotations

import os
import re
import subprocess
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "import"))
sys.path.insert(0, os.path.join(ROOT, "elster", "submission"))
sys.path.insert(0, os.path.join(ROOT, "elster"))

import elster_xml as EX      # noqa: E402
import validate_xsd as VX    # noqa: E402
import checkest_gate as CE   # noqa: E402

HID = "74931"                # ERiC-Test-Hersteller-ID aus dem amtlichen Beispiel-XML

_schema_da = VX.find_schema("2025") is not None
_xmllint_da = bool(__import__("shutil").which("xmllint"))
braucht_xsd = pytest.mark.skipif(
    not (_schema_da and _xmllint_da),
    reason="E10-2025.xsd oder xmllint fehlt — XSD-Gate nicht lauffähig")


def _hid_eric() -> str | None:
    """Echte Hersteller-ID fuer checkESt-Aufrufe (NICHT die feste XSD-Test-ID oben) — aus der
    Umgebung, sonst aus der gitignoreten .env. Nie loggen. Gleiches Muster wie
    tests/test_checkest_durchstich.py (dort nicht importierbar geteilt, s. dessen Docstring)."""
    hid = os.environ.get("ELSTER_HERSTELLER_ID")
    if hid:
        return hid
    pfad = os.path.join(ROOT, ".env")
    if not os.path.exists(pfad):
        return None
    for zeile in open(pfad, encoding="utf-8"):
        if zeile.startswith(("ELSTER_HERSTELLER_ID=", "HERSTELLER_ID=")):
            return zeile.split("=", 1)[1].strip().strip('"').strip("'") or None
    return None


_HID_ERIC = _hid_eric()
_ERIC_DA = bool(_HID_ERIC) and os.path.isdir(
    os.environ.get("ERIC_DIR", os.path.expanduser("~/02_Software/eric")))
braucht_eric = pytest.mark.skipif(
    not _ERIC_DA,
    reason="ERiC oder Hersteller-ID fehlt — amtliche Pruefung nicht lauffaehig "
           "(credential-freies CI)")


def _dekl(**kz) -> dict:
    return {"vollstaendig": True, "deklaration": dict(kz)}


def _schreibe(tmp_path, result, **kw) -> str:
    ziel = str(tmp_path / "submission.xml")
    return EX.schreibe_xml(result, ziel, vz=2025, hersteller_id=HID, **kw)


# ----------------------------------------------------------------- Pfad-Quelle (XSD)

def test_kz_pfade_kommen_aus_dem_schema():
    pfade = EX.kz_pfade(2025)
    assert len(pfade) > 2000, "E10-2025 hat >2000 Kz — Walk liefert zu wenig"
    assert pfade["E0100201"] == ("E10", "ESt1A", "Allg", "A", "E0100201")
    assert pfade["E0203504"] == ("E10", "N", "Wk", "EP", "Erste_Taetig", "E0203504")


def test_kz_pfade_sind_in_schema_reihenfolge():
    """xs:sequence ist ordnungsempfindlich — E0100401 steht im Schema VOR E0100201."""
    pfade = EX.kz_pfade(2025)
    reihenfolge = list(pfade)
    assert reihenfolge.index("E0100401") < reihenfolge.index("E0100201")


def test_pflicht_kinder_findet_person_diskriminator():
    """Anlage N verlangt <Person> vor <Wk> — kein Kz, also vom Walk nicht erfasst."""
    pflicht = EX.pflicht_kinder(2025)
    assert pflicht[("E10", "N")] == ["Person"]
    assert len(pflicht) > 50, "Es gibt viele Personen-Container, nicht nur Anlage N"


# ----------------------------------------------------------------- Struktur

def test_transfer_header_traegt_hersteller_id():
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID)
    assert f"<HerstellerID>{HID}</HerstellerID>" in xml
    assert "<Verfahren>ElsterErklaerung</Verfahren>" in xml
    assert "<DatenArt>ESt</DatenArt>" in xml


def test_testmerker_default_ist_eric_testfall():
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID)
    assert f"<Testmerker>{EX.TESTMERKER_ERIC}</Testmerker>" in xml


def test_testmerker_none_erzeugt_echtfall_ohne_merker():
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID, testmerker=None)
    assert "<Testmerker>" not in xml


def test_bool_wird_zu_x():
    xml = EX.erzeuge_xml(_dekl(E0100001=True), vz=2025, hersteller_id=HID)
    assert "E0100001>X<" in xml.replace("ns0:", "").replace("ns1:", "")


def test_person_diskriminator_wird_gesetzt():
    xml = EX.erzeuge_xml(_dekl(E0203504=20), vz=2025, hersteller_id=HID)
    entnamed = xml.replace("ns0:", "").replace("ns1:", "")
    assert "<Person>PersonA</Person>" in entnamed
    assert entnamed.index("<Person>") < entnamed.index("<Wk>")


def test_geschwister_in_schema_reihenfolge():
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier", E0100401="05.05.1955"),
                         vz=2025, hersteller_id=HID)
    entnamed = xml.replace("ns0:", "").replace("ns1:", "")
    assert entnamed.index("E0100401") < entnamed.index("E0100201")


# ----------------------------------------------------------------- fail-closed

def test_unvollstaendige_deklaration_wird_nicht_serialisiert():
    result = {"vollstaendig": False, "deklaration": {"E0100201": "Maier"},
              "unvollstaendig": [{"feld_id": "x", "grund": "vorlaeufig"}]}
    with pytest.raises(EX.XmlFehler, match="unvollständig"):
        EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)


def test_leere_deklaration_wird_abgelehnt():
    with pytest.raises(EX.XmlFehler, match="leere Deklaration"):
        EX.erzeuge_xml(_dekl(), vz=2025, hersteller_id=HID)


def test_ohne_hersteller_id_kein_xml(monkeypatch):
    monkeypatch.delenv("ELSTER_HERSTELLER_ID", raising=False)
    with pytest.raises(EX.XmlFehler, match="Hersteller-ID"):
        EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025)


def test_hersteller_id_aus_env(monkeypatch):
    monkeypatch.setenv("ELSTER_HERSTELLER_ID", "12345")
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025)
    assert "<HerstellerID>12345</HerstellerID>" in xml


def test_unbekannte_kz_ist_harter_fehler():
    with pytest.raises(EX.XmlFehler, match="ohne Pfad"):
        EX.erzeuge_xml(_dekl(E9999999="x"), vz=2025, hersteller_id=HID)


# ----------------------------------------------------------------- XSD-Gate

@braucht_xsd
def test_minimalfall_ist_xsd_valide(tmp_path):
    pfad = _schreibe(tmp_path, _dekl(E0100201="Maier", E0100301="Hans",
                                     E0100401="05.05.1955"))
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, meldung


@braucht_xsd
def test_mehrere_anlagen_sind_xsd_valide(tmp_path):
    """ESt1A + Anlage N gleichzeitig — prüft Container-Anlage über Anlagengrenzen."""
    pfad = _schreibe(tmp_path, _dekl(
        E0100201="Maier", E0100301="Hans", E0100401="05.05.1955",
        E0100402="03", E0101104="Musterstr.", E0100001=True, E0203504=20))
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, meldung


@braucht_xsd
def test_geschriebene_datei_ist_wohlgeformt(tmp_path):
    import xml.etree.ElementTree as ET
    pfad = _schreibe(tmp_path, _dekl(E0100201="Maier"))
    wurzel = ET.parse(pfad).getroot()
    assert wurzel.tag == "{http://www.elster.de/elsterxml/schema/v11}Elster"


def test_schreibe_xml_ist_atomar(tmp_path):
    """Kein .tmp-Rest nach erfolgreichem Schreiben."""
    pfad = _schreibe(tmp_path, _dekl(E0100201="Maier"))
    assert os.path.exists(pfad)
    assert not os.path.exists(pfad + ".tmp")


# ------------------------------------------------- ERiC-Rahmengate (XSD sieht es NICHT)

def test_kein_namespace_praefix_am_elster_root():
    """ERiC lehnt jede Praefix-Deklaration am <Elster>-Root ab — das XSD nicht.

    Gemessen 2026-08-09 gegen ERiC 44.2.4.0 / ESt_2025: die von ElementTree erzeugte
    Fassung mit `xmlns:ns1=...` am Root fiel mit

        rc=610301200 ERIC_IO_READER_SCHEMA_VALIDIERUNGSFEHLER
        eric.log: 'Der XML-Datensatz enthaelt an einer nicht erlaubten Stelle eine
                   Namespace-Praefix-Definition: xmlns:ns1'

    obwohl `xmllint --schema elster11_E10_2025_extern.xsd` sie als valide durchwinkt.
    Deshalb ist dieser Test NICHT durch das XSD-Gate abgedeckt und steht separat.
    """
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID)
    kopf = xml[:xml.index(">", xml.index("<Elster")) + 1]
    assert "xmlns:" not in kopf, (
        f"Praefix-Deklaration am Elster-Root — ERiC weist das XML ab (610301200):\n{kopf}")


def test_e10_traegt_seinen_namespace_lokal():
    """Der E10-Namespace muss am <E10> haengen, so wie im amtlichen Referenz-XML."""
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID)
    ns = EX.NS_E10.format(vz=2025)
    assert f'<E10 xmlns="{ns}"' in xml, "E10 ohne lokale Default-Namespace-Deklaration"
    assert "ns0:" not in xml and "ns1:" not in xml, "Praefix-Reste im Nutzdatenteil"


# ----------------------------------------------------------------- Vorsatz-Block
#
# <Vorsatz> traegt KEIN Kz (kein E\d{7}-Name) — kz_pfade() indiziert ihn deshalb nie, egal was
# in `deklaration` steht. Ohne ihn lehnt checkESt jedes Produkt-XML mit 9 Plausibilitaetsfehlern
# ab (gemessen 2026-08-09, reports/adjudikation/vorsatz_block_2026-08-09.md). Die Absender-
# Stammdaten liegen heute nicht im Fall vor -> `abgabefaehig=True` erzwingt sie fail-closed.

ABSENDER = dict(absender_name="Maier Hans", absender_strasse="Musterstr. 55",
                absender_plz="55555", absender_ort="Musterort",
                absender_steuernummer="9181081508155")


def test_ohne_abgabefaehig_gibt_es_keinen_vorsatz_block():
    """Default-Verhalten (abgabefaehig=False) bleibt unveraendert — keine stille Erweiterung."""
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID)
    assert "<Vorsatz>" not in xml


def test_abgabefaehig_haengt_vorsatz_mit_allen_pflichtfeldern_an():
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID,
                         abgabefaehig=True, **ABSENDER)
    for tag, wert in [("Unterfallart", "10"), ("Vorgang", "01"),
                      ("StNr", "9181081508155"), ("Zeitraum", "2025"),
                      ("AbsName", "Maier Hans"), ("AbsStr", "Musterstr. 55"),
                      ("AbsPlz", "55555"), ("AbsOrt", "Musterort"),
                      ("OrdNrArt", "S"), ("Bescheid", "2")]:
        assert f"<{tag}>{wert}</{tag}>" in xml, f"{tag} fehlt oder falscher Wert"
    assert "<Copyright>TaxGraph</Copyright>" in xml
    assert "<Rueckuebermittlung>" in xml


def test_vorsatz_kinder_in_schema_reihenfolge():
    """xs:sequence in Vorsatz_67907_CType (E10-2025.xsd:25286): Unterfallart, Vorgang, StNr,
    Zeitraum, AbsName, AbsStr, AbsPlz, AbsOrt, Copyright, OrdNrArt, Rueckuebermittlung/Bescheid.

    `<Vorgang>` kommt zweimal vor (auch im TransferHeader) — deshalb erst auf den Vorsatz-
    Teilstring einschraenken, sonst misst man versehentlich den falschen Treffer.
    """
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID,
                         abgabefaehig=True, **ABSENDER)
    vorsatz = xml[xml.index("<Vorsatz>"):xml.index("</Vorsatz>")]
    reihenfolge = ["Unterfallart", "Vorgang", "StNr", "Zeitraum", "AbsName", "AbsStr",
                   "AbsPlz", "AbsOrt", "Copyright", "OrdNrArt", "Bescheid"]
    positionen = [vorsatz.index(f"<{tag}>") for tag in reihenfolge]
    assert positionen == sorted(positionen), "Vorsatz-Kinder nicht in Schema-Reihenfolge"


def test_vorsatz_ist_das_letzte_kind_von_e10():
    """Laut Schema (E10-2025.xsd:8403) das letzte E10-Kind — muss NACH allen Kz-Containern stehen."""
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID,
                         abgabefaehig=True, **ABSENDER)
    assert xml.index("<ESt1A>") < xml.index("<Vorsatz>")
    assert xml.index("</Vorsatz>") < xml.index("</E10>")


def test_abgabefaehig_ohne_absender_ist_fail_closed():
    """Crash statt stiller Null — die 9 Absender-Fehler sollen nie unbemerkt zurueckkehren."""
    with pytest.raises(EX.XmlFehler, match="Absender-Stammdaten"):
        EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID, abgabefaehig=True)


def test_abgabefaehig_nennt_jedes_fehlende_absender_feld():
    with pytest.raises(EX.XmlFehler, match="absender_ort"):
        EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID, abgabefaehig=True,
                       absender_name="Maier Hans", absender_strasse="Musterstr. 55",
                       absender_plz="55555", absender_steuernummer="9181081508155")


def test_steuernummer_praefix_muss_zur_finanzamtsnummer_passen():
    """Gemessener Befund: OrdNrArt='S' ohne konsistente StNr ersetzt die 9 Original-Fehler NICHT
    durch 0, sondern durch 2 andere ('Bundesfinanzamtsnummer ... unterscheiden sich')."""
    absender = dict(ABSENDER, absender_steuernummer="1234081508155")  # Praefix != "9181"
    with pytest.raises(EX.XmlFehler, match="Finanzamtsnummer"):
        EX.erzeuge_xml(_dekl(E0100201="Maier"), vz=2025, hersteller_id=HID,
                       empfaenger_finanzamt="9181", abgabefaehig=True, **absender)


@braucht_xsd
def test_abgabefaehiges_xml_ist_xsd_valide(tmp_path):
    pfad = _schreibe(tmp_path, _dekl(E0100201="Maier", E0100301="Hans",
                                     E0100401="05.05.1955"),
                     abgabefaehig=True, **ABSENDER)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, meldung


# --------------------------------------------------------- Absender-Ableitung aus der Deklaration
#
# absender_name/_strasse/_plz/_ort sind KEINE zweite Wahrheit: dieselben Angaben stehen bereits
# als Kz im Hauptvordruck ESt 1 A (Stammdaten Person A). Statt sie als Pflicht-Parameter zu
# verlangen (zweite Repraesentation), leitet erzeuge_xml() sie aus `deklaration` ab, wenn kein
# expliziter Parameter kommt. absender_steuernummer hat KEINEN Kz-Spiegel (Vorsatz ist kein
# Kz-Element) — ohne `snapshot`-Parameter bleibt sie deshalb Pflicht-Parameter, s.
# test_absender_steuernummer_bleibt_ohne_kz_ableitung_pflicht_parameter() unten. Mit `snapshot`
# leitet erzeuge_xml() sie aus `stammdaten_steuernummer` ab (s. _leite_steuernummer_ab() in
# produkt/import/elster_xml.py) — eigene Tests dafuer in tests/test_stammdaten_steuernummer.py.

_STAMM_KZ = dict(E0100201="Maier", E0100301="Hans", E0101104="Musterstr.",
                 E0101206="55", E0100601="55555", E0100602="Musterort")


def test_absender_wird_aus_deklaration_abgeleitet():
    """Kein absender_name/_strasse/_plz/_ort uebergeben -> aus den Stammdaten-Kz gebaut."""
    xml = EX.erzeuge_xml(_dekl(**_STAMM_KZ), vz=2025, hersteller_id=HID, abgabefaehig=True,
                         absender_steuernummer="9181081508155")
    for tag, wert in [("AbsName", "Maier Hans"), ("AbsStr", "Musterstr. 55"),
                      ("AbsPlz", "55555"), ("AbsOrt", "Musterort")]:
        assert f"<{tag}>{wert}</{tag}>" in xml, f"{tag} nicht abgeleitet"


def test_explizit_absender_hat_vorrang_vor_ableitung():
    """Ein gesetzter Parameter gewinnt — Ableitung ist Fallback, keine Ueberschreibung.

    Mutationsprobe: `absender_name or abgeleitet[...]` zu `abgeleitet[...] or absender_name`
    vertauscht macht diesen Test bei abweichenden Werten rot.
    """
    xml = EX.erzeuge_xml(_dekl(**_STAMM_KZ), vz=2025, hersteller_id=HID, abgabefaehig=True,
                         absender_name="Explizit Vorrang", absender_steuernummer="9181081508155")
    assert "<AbsName>Explizit Vorrang</AbsName>" in xml
    assert "<AbsName>Maier Hans</AbsName>" not in xml


def test_hausnummernzusatz_wird_an_die_strasse_angehaengt():
    kz = dict(_STAMM_KZ, E0101207="a")
    xml = EX.erzeuge_xml(_dekl(**kz), vz=2025, hersteller_id=HID, abgabefaehig=True,
                         absender_steuernummer="9181081508155")
    assert "<AbsStr>Musterstr. 55a</AbsStr>" in xml


def test_fehlende_ableitung_nennt_das_kz_nicht_nur_den_parameternamen():
    """Wohnort-Kz (E0100602) fehlt, kein absender_ort-Parameter -> Meldung nennt E0100602."""
    kz = {k: v for k, v in _STAMM_KZ.items() if k != "E0100602"}
    with pytest.raises(EX.XmlFehler, match="E0100602"):
        EX.erzeuge_xml(_dekl(**kz), vz=2025, hersteller_id=HID, abgabefaehig=True,
                       absender_steuernummer="9181081508155")


def test_teilweise_fehlendes_kz_nennt_nur_das_fehlende():
    """Nachname (E0100201) vorhanden, Vorname (E0100301) fehlt -> nur E0100301 in der Meldung,
    nicht E0100201 (das ist ja da)."""
    kz = {k: v for k, v in _STAMM_KZ.items() if k != "E0100301"}
    with pytest.raises(EX.XmlFehler) as exc:
        EX.erzeuge_xml(_dekl(**kz), vz=2025, hersteller_id=HID, abgabefaehig=True,
                       absender_steuernummer="9181081508155")
    assert "E0100301" in str(exc.value)
    assert "E0100201" not in str(exc.value)


def test_absender_steuernummer_bleibt_ohne_kz_ableitung_pflicht_parameter():
    """Kein Kz-Spiegel fuer StNr — die Ableitung darf sie nicht ersetzen, Meldung sagt warum."""
    with pytest.raises(EX.XmlFehler, match="absender_steuernummer.*kein Kz-Spiegel"):
        EX.erzeuge_xml(_dekl(**_STAMM_KZ), vz=2025, hersteller_id=HID, abgabefaehig=True)


@braucht_eric
def test_ableitung_liefert_checkest_dieselbe_fehlerzahl_wie_explizite_parameter():
    """Beweis: ein XML mit abgeleiteten Absenderdaten ist fuer checkESt UNUNTERSCHEIDBAR von
    einem mit expliziten Parametern — keine der beiden Bauarten hinterlaesst eine eigene Spur
    von Beanstandungen."""
    kz_voll = dict(_STAMM_KZ, E0100401="05.05.1955")
    xml_abgeleitet = EX.erzeuge_xml(_dekl(**kz_voll), vz=2025, hersteller_id=_HID_ERIC,
                                    abgabefaehig=True, absender_steuernummer="9181081508155")
    xml_explizit = EX.erzeuge_xml(_dekl(**kz_voll), vz=2025, hersteller_id=_HID_ERIC,
                                  abgabefaehig=True, **ABSENDER)
    rc1, antwort1 = CE.validate(xml_abgeleitet, "ESt_2025")
    rc2, antwort2 = CE.validate(xml_explizit, "ESt_2025")
    n1 = len(re.findall(r"<Text>", antwort1 or ""))
    n2 = len(re.findall(r"<Text>", antwort2 or ""))
    assert n1 == n2, (f"Ableitung liefert eine ANDERE Fehlerzahl als explizite Parameter "
                      f"({n1} vs {n2}) — Ableitung ist keine echte Aequivalenz.")
