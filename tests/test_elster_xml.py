"""P3.2 XML-Writer: Deklaration -> ELSTER-Submission-XML.

Gate ist das amtliche E10-XSD (via xmllint). Die Positivtests beweisen, dass der Writer
schema-valides XML baut; die Negativtests, dass er fail-closed bleibt (unvollständige
Deklaration, fehlende Hersteller-ID, unbekannte Kz -> kein XML statt kaputtes XML).
"""
from __future__ import annotations

import hashlib
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "eingang"))
sys.path.insert(0, os.path.join(ROOT, "elster", "submission"))
sys.path.insert(0, os.path.join(ROOT, "elster"))
for _sub in ("produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import elster_xml as EX      # noqa: E402
import est_mapping as EM     # noqa: E402
import store as ST           # noqa: E402
import traverser as TR       # noqa: E402
import validate_xsd as VX    # noqa: E402
import checkest_gate as CE   # noqa: E402
import test_checkest_durchstich as D   # noqa: E402  (Rentner-Bauer, _ABSENDER)

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
    return {"eingaben_konsistent": True, "pflichtfelder_vollstaendig": True,
            "deklaration": dict(kz)}


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
    result = {"eingaben_konsistent": False, "deklaration": {"E0100201": "Maier"},
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


def test_steuerzeichen_im_textwert_ist_harter_fehler():
    """Ticket elster-xml-steuerzeichen-im-textwert: ElementTree maskiert nur &, <, > — ein NUL
    landete roh im XML, und ELSTER weist die ganze Abgabe ab. Zweite Linie hinter der
    Store-Pruefung (Alt-Stores, Importe): fail-closed statt kaputtem XML."""
    with pytest.raises(EX.XmlFehler, match="Steuerzeichen") as exc:
        EX.erzeuge_xml(_dekl(E0100201="Maier\x00"), vz=2025, hersteller_id=HID)
    meldung = str(exc.value)
    assert "E0100201" in meldung, meldung                                  # nennt die Kz ...
    assert "Maier" not in meldung and "\x00" not in meldung, meldung      # ... nie den Wert (PII)
    # jede Kz, nicht nur typ=text: der Fuzz-Fund war die bool-Kz E0161806 mit NUL
    with pytest.raises(EX.XmlFehler, match="Element E0161806 enthält ein Steuerzeichen"):
        EX.erzeuge_xml(_dekl(E0161806="\x00"), vz=2025, hersteller_id=HID)


@pytest.mark.parametrize("wert,zeichen,vorschlag", [
    ("Kowalski Anna", "geschütztes Leerzeichen (U+00A0)", "ein normales Leerzeichen"),
    ("Müller–Straße", "„–\" (U+2013)", "„-\""),
    ("Wałesa", "„ł\" (U+0142)", "„l\""),
    ("Maier\tMüller", "Tabulator (U+0009)", "ein normales Leerzeichen"),
    ("Maier\nMüller", "Zeilenumbruch (U+000A)", "ein normales Leerzeichen"),
], ids=["nbsp", "gedankenstrich", "l-mit-strich", "tabulator", "zeilenumbruch"])
def test_zeichen_ausserhalb_des_elster_zeichensatzes_ist_harter_fehler(wert, zeichen, vorschlag):
    """Ticket elster-zeichensatz-strenger-als-xml, AK5: die zweite Sperre an der XML-Erzeugung. Ein Wert,
    der vor der Store-Regel gespeichert wurde (Alt-Store, Import), kommt nicht ins XML — er scheiterte
    sonst erst bei ELSTER. Die Meldung nennt Element, Zeichen und Vorschlag, nie den Wert (PII)."""
    with pytest.raises(EX.XmlFehler, match="ELSTER in Textfeldern nicht annimmt") as exc:
        EX.erzeuge_xml(_dekl(E0100201=wert), vz=2025, hersteller_id=HID)
    meldung = str(exc.value)
    assert "Element E0100201" in meldung and zeichen in meldung and vorschlag in meldung, meldung
    assert "Müller" not in meldung and "Maier" not in meldung and "Kowalski" not in meldung, meldung


def test_altbestand_mit_unerlaubtem_zeichen_passiert_die_xml_erzeugung_nicht():
    """AK5 durch den ganzen Weg: ein Wert, den ein Alt-Store ohne Zeichensatz-Regel angenommen hat
    (append_event OHNE bindung=, wie vor der Regel), wird deklariert und scheitert erst am Writer."""
    s = ST.leerer_store(2025, fall_id="zs-altbestand")
    for fid, wert in (("stammdaten_nachname", "Wałesa"), ("veranlagung", "einzel")):
        ST.append_event(s, feld_id=fid, wert=wert, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"},
                        ts="2026-10-01T12:00:00+00:00")
    snap, sid = ST.materialisiere(s)
    dekl = EM.deklariere(snap, TR.lade_bindung(), snapshot_id=sid, vz=2025)
    assert dekl["deklaration"]["E0100201"] == "Wałesa"
    with pytest.raises(EX.XmlFehler, match=r"Element E0100201 enthält das Zeichen „ł\" \(U\+0142\)"):
        EX.erzeuge_xml(dekl, vz=2025, hersteller_id=HID)


def test_erlaubte_zeichen_kommen_ins_xml():
    """Gegenprobe zu AK5: Umlaute, ß, € und Œ gehen durch die zweite Sperre und stehen im XML."""
    xml = EX.erzeuge_xml(_dekl(E0100201="Müller-Größe ß € Œuvre"), vz=2025, hersteller_id=HID)
    assert "<E0100201>Müller-Größe ß € Œuvre</E0100201>" in xml


# ----------------------------------------------------------------- XSD-Gate

@braucht_xsd
def test_minimalfall_ist_xsd_valide(tmp_path):
    pfad = _schreibe(tmp_path, _dekl(E0100201="Maier", E0100301="Hans",
                                     E0100401="05.05.1955"))
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, meldung


# ------------------------------------------------- §35a: mehrere Posten je Topf (AK1-AK3)
#
# Haushaltsnahe Aufwendungen (§ 35a EStG) haben drei Töpfe: Minijob, Dienstleistung, Handwerker.
# Wer in einem Topf zwei Posten angibt, bekam bis 2026-10-01 ein ungültiges XML: <HA_35a> trägt
# im Schema maxOccurs="1" (E10-2025.xsd:8236), die Posten wiederholen sich über <Einz> darunter
# (maxOccurs="99", :10048). Ohne Eintrag in INSTANZ_CONTAINER_TIEFER fiel der Writer auf
# `kz_path[:2]` zurück — also auf <HA_35a> selbst — und wiederholte den ganzen Abschnitt.
# xmllint: „Element HA_35a: This element is not expected". Von mehreren Posten erreichte nur
# einer die Datei. Entschieden in decisions/p35a-posten-als-einz-unter-einem-ha35a.md.

_HH_TOEPFE = {
    # gruppe: (Art-Kz, Betrag-Kz) — beide sitzen unter <St_Erm>/<Topf>/<Einz>
    "hh_minijob": ("E0104206", "E0104108"),
    "hh_dienstleistung": ("E0104206", "E0104108"),
    "hh_handwerker": ("E0111217", "E0111214"),
}


def _hh_result(gruppe: str, posten: list[dict]) -> dict:
    """Ein §35a-Topf mit N Posten, wie est_mapping.deklariere() ihn liefert."""
    return {
        "eingaben_konsistent": True,
        "deklaration": {"E0100001": True},
        "person_b": {},
        "anlage_instanzen": {
            gruppe: [{"index": i + 1, "felder": dict(p)} for i, p in enumerate(posten)]
        },
        "kind_anlagen": [],
    }


def _zaehle(xml: str, tag: str) -> int:
    return len(re.findall(rf"<{tag}[ >]", xml))


def test_hh_top_zwei_posten_ein_ha35a():
    """AK1: ein §35a-Topf mit zwei Posten erzeugt genau EIN <HA_35a> mit ZWEI <Einz>-Kindern.

    Geprüft für alle drei Töpfe: die Gruppe steht in INSTANZ_CONTAINER_TIEFER, nicht der
    Elementname — „Einz" allein wäre mehrdeutig (er kehrt im E10-Schema vielfach wieder)."""
    for gruppe, (art_kz, betrag_kz) in _HH_TOEPFE.items():
        xml = EX.erzeuge_xml(_hh_result(gruppe, [
            {art_kz: "1", betrag_kz: 120000},
            {art_kz: "2", betrag_kz: 80000},
        ]), vz=2025, hersteller_id=HID)
        assert _zaehle(xml, "HA_35a") == 1, f"{gruppe}: {_zaehle(xml, 'HA_35a')} HA_35a"
        assert _zaehle(xml, "Einz") == 2, f"{gruppe}: {_zaehle(xml, 'Einz')} Einz"
        # Beide Posten stehen wirklich drin — nicht zwei Hüllen um einen Wert.
        assert "120000" in xml and "80000" in xml, gruppe


def test_hh_top_ein_posten_bleibt_unveraendert():
    """AK2: Gegenprobe zu AK1 — ein Posten ergibt weiter genau ein <HA_35a> mit einem <Einz>.
    Ohne diese Probe belegt AK1 nur, dass sich etwas geändert hat."""
    for gruppe, (art_kz, betrag_kz) in _HH_TOEPFE.items():
        xml = EX.erzeuge_xml(_hh_result(gruppe, [{art_kz: "1", betrag_kz: 120000}]), vz=2025,
                             hersteller_id=HID)
        assert _zaehle(xml, "HA_35a") == 1, gruppe
        assert _zaehle(xml, "Einz") == 1, gruppe


@braucht_xsd
def test_hh_top_mehrere_posten_ist_xsd_valide(tmp_path):
    """AK5 (Python-Seite): das erzeugte XML hält das amtliche Schema. Vor dem Fix scheiterte
    genau das an <HA_35a> (maxOccurs=1) — der Test wäre damals rot gewesen."""
    pfad = str(tmp_path / "hh.xml")
    EX.schreibe_xml(_hh_result("hh_minijob", [
        {"E0104206": "1", "E0104108": 120000},
        {"E0104206": "2", "E0104108": 80000},
    ]), pfad, vz=2025, hersteller_id=HID)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, meldung


def test_instanz_container_tiefer_verdraengt_p23_nicht():
    """AK3: `p23_veraeusserung` muss seinen Eintrag behalten. Der Fix fügt drei Gruppen hinzu,
    er ersetzt keine — ein `dict`-Literal, das den bestehenden Schlüssel überschriebe, fiele
    hier auf."""
    assert EX.INSTANZ_CONTAINER_TIEFER["p23_veraeusserung"] == "Einz"
    for gruppe in _HH_TOEPFE:
        assert EX.INSTANZ_CONTAINER_TIEFER[gruppe] == "Einz"


# ------------------------------------------------- Anlage R: mehrere Renten einer Person
#
# Wer allein veranlagt wird und zwei Renten hat, bekam bis 2026-10-03 ein XML mit ZWEI <R>: die zweite
# Rente (Instanz `__2` der Gruppe `rente`) legte ein zweites <R> an, und Index 1 am <R> heisst PersonB.
# ERiC: rc=610001002, „Es handelt sich um eine Einzelveranlagung, daher darf fuer die Ehefrau /
# Person B keine Anlage R ausgefuellt werden." Das Schema kennt <R> je Person (maxOccurs=2); die Renten
# einer Person stehen als <Einz> (maxOccurs=99) in <Leibr_gesetzl>|<Leibr_priv>|<Leibr_sonst>.
# Eine zweite Stelle: der Index im <Einz> ist der RANG der Instanz im Container, nicht ihre Nummer —
# gesetzlich (1) plus privat (2) legte vor dem privaten Posten ein leeres <Einz> an (ERiC: „Kontext
# '/R[1]/Leibr_priv[1]/Einz[1]' ist leer"). Entschieden in
# decisions/zweite-rente-einer-person-steht-als-weiteres-einz-in-derselben-anlage-r.md.

def _anlagen_r(xml: str) -> list:
    """[(Person, {Container: [{Kz: Text} je <Einz>]})] je <R>, in Dokumentreihenfolge.

    Eine Kz, die in EINEM <Einz> zweimal steht, ist ein Fehler des Schreibers und bricht hier ab —
    ein dict wuerde sie verschlucken."""
    def lok(tag: str) -> str:
        return tag.rsplit("}", 1)[-1]
    ergebnis = []
    for r in (e for e in ET.fromstring(xml).iter() if lok(e.tag) == "R"):
        person = next(c.text for c in r if lok(c.tag) == "Person")
        container: dict = {}
        for c in r:
            if not lok(c.tag).startswith("Leibr"):
                continue
            for einz in c:
                kinder = [(lok(k.tag), k.text) for k in einz]
                assert len({t for t, _ in kinder}) == len(kinder), f"{lok(c.tag)}: Kz doppelt im Einz: {kinder}"
                container.setdefault(lok(c.tag), []).append(dict(kinder))
        ergebnis.append((person, container))
    return ergebnis


def _rente_xml(veranlagung: str, zweite=(), partner=()) -> str:
    snap, sid = ST.materialisiere(D._fall_rente(veranlagung, zweite, partner))
    dekl = EM.deklariere(snap, TR.lade_bindung(), snapshot_id=sid, vz=2025)
    return EX.erzeuge_xml(dekl, vz=2025, hersteller_id=HID, abgabefaehig=True, **D._ABSENDER)


# Rente 1 (`_BASIS_RENTNER_MIT_RENTE`), Rente `__2` (`D._rente_2`), Rente von Person B (`D._RENTE_PARTNER`)
_GESETZL_1 = {"E1800301": "18000", "E1800501": "01.01.2015"}
_GESETZL_2 = {"E1800301": "9000", "E1800501": "01.01.2012"}
_PRIV_2 = {"E1801601": "9000", "E1801701": "01.01.2012"}
_GESETZL_B = {"E1800301": "7000", "E1800501": "01.01.2016"}

_RENTEN_FAELLE = {
    # AK1: Einzelveranlagung, zwei gesetzliche Renten -> EIN <R>, EIN <Leibr_gesetzl>, ZWEI <Einz>
    "einzel gesetzlich+gesetzlich": (
        ("einzel", D._rente_2("gesetzliche_rente"), ()),
        [("PersonA", {"Leibr_gesetzl": [_GESETZL_1, _GESETZL_2]})]),
    # AK2: gesetzlich + privat -> je EIN <Einz> in <Leibr_gesetzl> und <Leibr_priv>, kein leeres <Einz>
    "einzel gesetzlich+privat": (
        ("einzel", D._rente_2("private_leibrente"), ()),
        [("PersonA", {"Leibr_gesetzl": [_GESETZL_1], "Leibr_priv": [_PRIV_2]})]),
    # AK3: zusammen, zweite Rente von A + Rente von B -> <R>[1] (A) zwei <Einz>, <R>[2] (B) eines
    "zusammen zweite Rente A + Rente B": (
        ("zusammen", D._rente_2("gesetzliche_rente"), D._RENTE_PARTNER),
        [("PersonA", {"Leibr_gesetzl": [_GESETZL_1, _GESETZL_2]}),
         ("PersonB", {"Leibr_gesetzl": [_GESETZL_B]})]),
}


@pytest.mark.parametrize("name", list(_RENTEN_FAELLE))
def test_mehrere_renten_einer_person_stehen_als_einz_in_ihrer_anlage_r(name):
    """AK1-AK3: Aufbau des <R>-Baums je Fall — Zahl und Reihenfolge der <R>, der Container, der <Einz>
    und ihr Inhalt. Nur die Zahl der <Einz> zu zaehlen liesse das leere <Einz> von AK2 durch."""
    (veranlagung, zweite, partner), erwartet = _RENTEN_FAELLE[name]
    assert _anlagen_r(_rente_xml(veranlagung, zweite, partner)) == erwartet, name


@braucht_xsd
@pytest.mark.parametrize("name", list(_RENTEN_FAELLE))
def test_mehrere_renten_einer_person_sind_xsd_valide(name, tmp_path):
    (veranlagung, zweite, partner), _ = _RENTEN_FAELLE[name]
    pfad = str(tmp_path / "rente.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(_rente_xml(veranlagung, zweite, partner))
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, f"{name}: {meldung}"


# Luecke im Instanzindex: Instanz 1 und 3, keine 2. Der Store erlaubt das (`eingaben_konsistent` bleibt
# wahr, es fehlt nur `__2`). Die Gruppen-Nummer 3 legte VOR dem Posten ein leeres <Einz> an
# (ERiC: „Kontext ist leer"); der Rang zaehlt dicht. Das ist die EINZIGE Abweichung vom alten XML der
# vier bestehenden Gruppen (gemessen: alle byte-gleichen Faelle unten) und sie ist gewollt.
_HH_FELDER = {
    "hh_minijob": ("hh_minijob_art", "hh_minijob_betrag"),
    "hh_dienstleistung": ("hh_dienstleistung_art", "hh_dienstleistung_betrag"),
    "hh_handwerker": ("hh_handwerker_art", "hh_handwerker_betrag"),
}


@pytest.mark.parametrize("gruppe", list(_HH_FELDER))
def test_luecke_im_instanzindex_zaehlt_dicht_ohne_leeres_einz(gruppe):
    art, betrag = _HH_FELDER[gruppe]
    s = ST.leerer_store(2025, fall_id=f"luecke_{gruppe}")
    for feld, wert in ((art, "1"), (betrag, 120_000), (f"{art}__3", "2"), (f"{betrag}__3", 80_000)):
        D._b(s, feld, wert)
    D._b(s, "veranlagung", "einzel")
    snap, sid = ST.materialisiere(s)
    dekl = EM.deklariere(snap, TR.lade_bindung(), snapshot_id=sid, vz=2025)
    assert dekl["eingaben_konsistent"] is True and [i["index"] for i in dekl["anlage_instanzen"][gruppe]] == [3], \
        "Vorbedingung: Instanz 3 liegt in anlage_instanzen, Instanz 2 fehlt"
    xml = EX.erzeuge_xml(dekl, vz=2025, hersteller_id=HID)
    einz = [e for e in ET.fromstring(xml).iter() if e.tag.rsplit("}", 1)[-1] == "Einz"]
    assert len(einz) == 2, f"{gruppe}: {len(einz)} Einz statt 2"
    assert all(len(e) > 0 for e in einz), f"{gruppe}: leeres Einz im XML"
    # beide Posten stehen drin, in Instanz-Reihenfolge (Betrag in Euro: 120000 Cent = 1200)
    assert [[k.text for k in e] for e in einz] == [["1", "1200"], ["2", "800"]], gruppe


# Pin (AK8): die vier bestehenden Gruppen und die Rente mit EINER Rente erzeugen vor und nach dem Umbau
# dasselbe XML, Byte fuer Byte. Hash = sha256 des XML-Textes (UTF-8), gemessen auf 604022c8 (vorher).
# Faelle: p23 (1-3 Verkaeufe, Partner), die drei §35a-Toepfe mit 1, 2, 4 Posten, Rente einzel mit einer
# Rente, Rente zusammen mit Rente von Person B. Wird einer rot, hat sich das XML einer bestehenden Gruppe
# geaendert -- der Rang im Container wirkt dann ueber `rente` hinaus.
_HASH_VORHER = {
    "p23 1 Verkaeufe": "221571009eb89abfc26a003dbb3ae56550f46cf76949ce33f3806f0a9d75c20d",
    "p23 2 Verkaeufe": "34c063104d0146535c54a28ddc4c93be1c75f6d17083cf793234bb8c93bead74",
    "p23 3 Verkaeufe": "c9589cdf09b1ecbf9450f39e1500d9cc70f926b35c3675def0486f14e6843e21",
    "p23 Partner-Verkauf": "57b6b711aab73a77559b4ca05115795bd1f60b8c148c5a3fe7570f85d7b00cd3",
    "hh_minijob 1 Posten": "d351e12a89d6e99454084884eb07a260d3b5bd9c66a5c236206dee98c74d738f",
    "hh_minijob 2 Posten": "f36a5b55bd8ce968af3faf2ec12ac10479efe9344f2a62916c84ca33f6fe4af3",
    "hh_minijob 4 Posten": "1b3e299cebdbb20d049f6fcd0c66e4918469d62ba53922914cfd9bed583fbecd",
    "hh_dienstleistung 1 Posten": "d351e12a89d6e99454084884eb07a260d3b5bd9c66a5c236206dee98c74d738f",
    "hh_dienstleistung 2 Posten": "f36a5b55bd8ce968af3faf2ec12ac10479efe9344f2a62916c84ca33f6fe4af3",
    "hh_dienstleistung 4 Posten": "1b3e299cebdbb20d049f6fcd0c66e4918469d62ba53922914cfd9bed583fbecd",
    "hh_handwerker 1 Posten": "28f1f0642c1f1a252d9e1f51695aa9b3035189af48f2f61c32c1d0fc4d9f2666",
    "hh_handwerker 2 Posten": "4c8672a7ad43d0d61c47b42500b0810911b9b1f4d5390e8c9d849280ce6c6221",
    "hh_handwerker 4 Posten": "2c7917d7b1bc17760b7634364e421f8708aa03dfd35b678be6a23f365babd6de",
    "Rente einzel, eine Rente": "4300823a0933db89847e71b8360c4dc5b4dc7ded84b5bf6181513d5e7033c45c",
    "Rente zusammen, Rente von B": "9604e70b449f51dab3eb95a09881cc5676ade75b64f8dd6a9a2b8d253ef0f149",
}


def _bytegleich_faelle() -> dict:
    import test_p23_mehrfachverkauf_bricht_so_maxoccurs as P23
    import test_p23_partner_verkauf_still_unter_person_a_eingereicht as P23P
    bind = TR.lade_bindung()

    def p23(n):
        snap, sid = ST.materialisiere(P23._fall(f"pin_p23_{n}", n))
        res = EM.deklariere(snap, bind, snapshot_id=sid, vz=2025)
        return EX.erzeuge_xml(res, vz=2025, hersteller_id="74931", snapshot=snap)

    def hh(gruppe, n):
        art, betrag = _HH_TOEPFE[gruppe]
        res = _hh_result(gruppe, [{art: str(i + 1), betrag: 100_000 * (i + 1)} for i in range(n)])
        return EX.erzeuge_xml(res, vz=2025, hersteller_id="74931")

    faelle: dict = {f"p23 {n} Verkaeufe": (lambda n=n: p23(n)) for n in (1, 2, 3)}
    faelle["p23 Partner-Verkauf"] = lambda: P23P._xml_bauen(
        bind, {**P23P._VERKAUF_A, **P23P._VERKAUF_PARTNER_UEBER_INSTANZ_2})[1]
    faelle.update({f"{g} {n} Posten": (lambda g=g, n=n: hh(g, n)) for g in _HH_TOEPFE for n in (1, 2, 4)})
    faelle["Rente einzel, eine Rente"] = lambda: _rente_xml("einzel")
    faelle["Rente zusammen, Rente von B"] = lambda: _rente_xml("zusammen", (), D._RENTE_PARTNER)
    return faelle


def test_vier_gruppen_und_eine_rente_erzeugen_unveraendertes_xml():
    faelle = _bytegleich_faelle()
    assert set(faelle) == set(_HASH_VORHER), "Fallliste und Hash-Tabelle weichen ab"
    anders = {}
    for name, bau in faelle.items():
        h = hashlib.sha256(bau().encode("utf-8")).hexdigest()
        if h != _HASH_VORHER[name]:
            anders[name] = h
    assert not anders, f"XML geaendert gegenueber 604022c8 (sha256): {anders}"


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
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier", E0102002=True), vz=2025, hersteller_id=HID,
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
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier", E0102002=True), vz=2025, hersteller_id=HID,
                         abgabefaehig=True, **ABSENDER)
    vorsatz = xml[xml.index("<Vorsatz>"):xml.index("</Vorsatz>")]
    reihenfolge = ["Unterfallart", "Vorgang", "StNr", "Zeitraum", "AbsName", "AbsStr",
                   "AbsPlz", "AbsOrt", "Copyright", "OrdNrArt", "Bescheid"]
    positionen = [vorsatz.index(f"<{tag}>") for tag in reihenfolge]
    assert positionen == sorted(positionen), "Vorsatz-Kinder nicht in Schema-Reihenfolge"


def test_vorsatz_ist_das_letzte_kind_von_e10():
    """Laut Schema (E10-2025.xsd:8403) das letzte E10-Kind — muss NACH allen Kz-Containern stehen."""
    xml = EX.erzeuge_xml(_dekl(E0100201="Maier", E0102002=True), vz=2025, hersteller_id=HID,
                         abgabefaehig=True, **ABSENDER)
    assert xml.index("<ESt1A>") < xml.index("<Vorsatz>")
    assert xml.index("</Vorsatz>") < xml.index("</E10>")


def test_abgabefaehig_ohne_absender_ist_fail_closed():
    """Crash statt stiller Null — die 9 Absender-Fehler sollen nie unbemerkt zurueckkehren."""
    with pytest.raises(EX.XmlFehler, match="Absender-Stammdaten"):
        EX.erzeuge_xml(_dekl(E0100201="Maier", E0102002=True), vz=2025, hersteller_id=HID,
                       abgabefaehig=True)


def test_abgabefaehig_nennt_jedes_fehlende_absender_feld():
    with pytest.raises(EX.XmlFehler, match="absender_ort"):
        EX.erzeuge_xml(_dekl(E0100201="Maier", E0102002=True), vz=2025, hersteller_id=HID,
                       abgabefaehig=True,
                       absender_name="Maier Hans", absender_strasse="Musterstr. 55",
                       absender_plz="55555", absender_steuernummer="9181081508155")


def test_steuernummer_praefix_muss_zur_finanzamtsnummer_passen():
    """Gemessener Befund: OrdNrArt='S' ohne konsistente StNr ersetzt die 9 Original-Fehler NICHT
    durch 0, sondern durch 2 andere ('Bundesfinanzamtsnummer ... unterscheiden sich')."""
    absender = dict(ABSENDER, absender_steuernummer="1234081508155")  # Praefix != "9181"
    with pytest.raises(EX.XmlFehler, match="Finanzamtsnummer"):
        EX.erzeuge_xml(_dekl(E0100201="Maier", E0102002=True), vz=2025, hersteller_id=HID,
                       empfaenger_finanzamt="9181", abgabefaehig=True, **absender)


@braucht_xsd
def test_abgabefaehiges_xml_ist_xsd_valide(tmp_path):
    pfad = _schreibe(tmp_path, _dekl(E0100201="Maier", E0100301="Hans",
                                     E0100401="05.05.1955", E0102002=True),
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
# produkt/eingang/elster_xml.py) — eigene Tests dafuer in tests/test_stammdaten_steuernummer.py.

# E0102002 (keine Bankverbindung): seit dem Bankverbindungs-Baustein (2026-08-10) verlangt
# abgabefaehig=True zusaetzlich eine Bankverbindungs-Entscheidung (s. erzeuge_xml()); hier dabei,
# damit diese Tests weiter isoliert nur die Absender-Ableitung pruefen.
_STAMM_KZ = dict(E0100201="Maier", E0100301="Hans", E0101104="Musterstr.",
                 E0101206="55", E0100601="55555", E0100602="Musterort", E0102002=True)


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
    for rc, seite in ((rc1, "abgeleitet"), (rc2, "explizit")):
        klasse = CE.klassifiziere_rc(rc)
        if klasse in CE.NICHT_GEPRUEFT_KLASSEN:
            pytest.skip(f"{seite}: rc={rc} [{klasse}]: nicht geprueft, kein Vergleich "
                        f"moeglich. Leerer Puffer heisst hier NICHT fehlerfrei.")
    n1 = len(re.findall(r"<Text>", antwort1 or ""))
    n2 = len(re.findall(r"<Text>", antwort2 or ""))
    assert n1 == n2, (f"Ableitung liefert eine ANDERE Fehlerzahl als explizite Parameter "
                      f"({n1} vs {n2}) — Ableitung ist keine echte Aequivalenz.")
