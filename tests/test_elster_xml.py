"""P3.2 XML-Writer: Deklaration -> ELSTER-Submission-XML.

Gate ist das amtliche E10-XSD (via xmllint). Die Positivtests beweisen, dass der Writer
schema-valides XML baut; die Negativtests, dass er fail-closed bleibt (unvollständige
Deklaration, fehlende Hersteller-ID, unbekannte Kz -> kein XML statt kaputtes XML).
"""
from __future__ import annotations

import os
import subprocess
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "import"))
sys.path.insert(0, os.path.join(ROOT, "elster", "submission"))

import elster_xml as EX      # noqa: E402
import validate_xsd as VX    # noqa: E402

HID = "74931"                # ERiC-Test-Hersteller-ID aus dem amtlichen Beispiel-XML

_schema_da = VX.find_schema("2025") is not None
_xmllint_da = bool(__import__("shutil").which("xmllint"))
braucht_xsd = pytest.mark.skipif(
    not (_schema_da and _xmllint_da),
    reason="E10-2025.xsd oder xmllint fehlt — XSD-Gate nicht lauffähig")


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
