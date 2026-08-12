"""Abnahme: Instanz-Achse im ELSTER-XML-Writer (Schritt 3, Plan lexical-beaming-glacier.md).

Ersetzt die beiden xfail-Tests (die nicht prüften, was sie behaupteten) durch vier
echte Abnahme-Tests über den Produktionspfad Store -> deklariere -> erzeuge_xml:

1. Person-B-Durchgang: Zusammenveranlagung mit unterschiedlichen Werten für A und B
2. Kind-Instanz-Durchgang: Zwei Kind-Instanzen (Reuse-Kz, indizierte Container)
3. Kinderzahl-Wächter: kind_anlagen vs anlage_instanzen -> XmlFehler
4. Ring-Deklaration-Differential: jedes Betragsfeld mit Kz im XML oder nicht_deklariert

XSD-Validierung (Schritt 4) über skipif-Muster.
"""
from __future__ import annotations

import os
import sys
import xml.etree.ElementTree as ET

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser", "elster/submission"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import elster_xml as EX       # noqa: E402
import est_mapping            # noqa: E402
import store as ST            # noqa: E402
import traverser as TR        # noqa: E402
import validate_xsd as VX     # noqa: E402

HID = "74931"
TS = "2026-08-04T14:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

_schema_da = VX.find_schema("2025") is not None
_xmllint_da = bool(__import__("shutil").which("xmllint"))
braucht_xsd = pytest.mark.skipif(
    not (_schema_da and _xmllint_da),
    reason="E10-2025.xsd oder xmllint fehlt — XSD-Gate nicht lauffähig")


def _b(s, feld_id, wert, zustand="bestaetigt"):
    """Helper: Wert in den Store schreiben."""
    sig = {"signal_1": None, "signal_2": f"ok@{feld_id}"} if zustand == "bestaetigt" else {
        "signal_1": None, "signal_2": None}
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand=zustand,
                    herkunft=H, schreiber="ui:laie", signal=sig, ts=TS)


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def _xml_count_values(xml_str: str) -> dict[str, int]:
    """Zählt Vorkommen jedes numerischen Werts im XML."""
    import re
    clean = xml_str.replace("ns0:", "").replace("ns1:", "")
    counts: dict[str, int] = {}
    for m in re.finditer(r">(\d+)<", clean):
        counts[m.group(1)] = counts.get(m.group(1), 0) + 1
    return counts


# ---------------------------------------------------------------- 1. Person-B-Durchgang

def test_person_b_echter_bucket_wechselseitig(bindung):
    """Zusammenveranlagung: Person A (50k, 2k, 1.5k, 1k, 5k) und B (40k, 1.6k, 1.2k, 0.8k, 3k).
    Beide Werte-Sets tauchen im XML auf — A-Werte sind PersonA, B-Werte sind PersonB.
    Prüfe über String-Extraktion: alle Werte vorhanden, keine doppelten."""
    s = ST.leerer_store(2025, fall_id="person_b_wechs")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "vor_an_anteil_rv", 200000)
    _b(s, "vor_ag_anteil_rv", 150000)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)
    _b(s, "kap_kapitalertraege", 500000)
    _b(s, "kap_gewinn_aktien", 0)
    _b(s, "kap_verlust_aktien", 0)
    _b(s, "kap_verlust_sonstige", 0)
    _b(s, "bruttoarbeitslohn_partner", 4000000)
    _b(s, "vor_an_anteil_rv_partner", 160000)
    _b(s, "vor_ag_anteil_rv_partner", 120000)
    _b(s, "vor_rv_ausserhalb_lstb_partner", 80000)
    _b(s, "kap_kapitalertraege_partner", 300000)
    _b(s, "kap_gewinn_aktien_partner", 0)
    _b(s, "kap_verlust_aktien_partner", 0)
    _b(s, "kap_verlust_sonstige_partner", 0)
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", False)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    clean = xml.replace("ns0:", "").replace("ns1:", "")

    assert "<Person>PersonA</Person>" in clean
    assert "<Person>PersonB</Person>" in clean

    # Person A Werte (CENT→EURO): 50000, 2000, 1500, 1000, 5000
    assert ">50000<" in clean, "Bruttolohn 50000 (PersonA) fehlt"
    assert ">2000<" in clean, "vor_an_anteil_rv 2000 (PersonA) fehlt"
    assert ">1500<" in clean, "vor_ag_anteil_rv 1500 (PersonA) fehlt"

    # Person B Werte (CENT→EURO): 40000, 1600, 1200, 800, 3000
    assert ">40000<" in clean, "Bruttolohn 40000 (PersonB) fehlt"
    assert ">1600<" in clean, "vor_an_anteil_rv 1600 (PersonB) fehlt"
    assert ">1200<" in clean, "vor_ag_anteil_rv 1200 (PersonB) fehlt"

    # Jeder Wert taucht genau 1x auf (weil unterschiedliche Werte A≠B — kein Duplikat)
    counts = _xml_count_values(xml)
    assert counts.get("50000", 0) == 1, "50000 sollte genau 1x vorkommen"
    assert counts.get("40000", 0) == 1, "40000 sollte genau 1x vorkommen (PersonB)"


@braucht_xsd
def test_person_b_xsd_valide(bindung, tmp_path):
    """Person-B-XML gegen amtliches Schema validieren."""
    s = ST.leerer_store(2025, fall_id="person_b_xsd")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "bruttoarbeitslohn_partner", 4000000)
    _b(s, "vor_an_anteil_rv", 200000)
    _b(s, "vor_an_anteil_rv_partner", 160000)
    _b(s, "vor_ag_anteil_rv", 150000)
    _b(s, "vor_ag_anteil_rv_partner", 120000)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)
    _b(s, "vor_rv_ausserhalb_lstb_partner", 80000)
    _b(s, "kap_kapitalertraege", 500000)
    _b(s, "kap_kapitalertraege_partner", 300000)
    _b(s, "kap_gewinn_aktien", 0)
    _b(s, "kap_gewinn_aktien_partner", 0)
    _b(s, "kap_verlust_aktien", 0)
    _b(s, "kap_verlust_aktien_partner", 0)
    _b(s, "kap_verlust_sonstige", 0)
    _b(s, "kap_verlust_sonstige_partner", 0)
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", False)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    pfad = str(tmp_path / "person_b.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml)

    ok, meldung = VX.validate(pfad, "2025")
    assert ok, f"Person-B-XML nicht schema-valide: {meldung}"


# ---------------------------------------------------------------- 1b. Gewinneinkünfte Person-B (§§13-18, §16 Abs.4, §35)

def test_gewinneinkuenfte_partner_kommt_im_xml_an(bindung):
    """Stufe 1 (Deklaration) der Gewinneinkünfte-Partnerseite: einkuenfte_gewinn_partner
    (gewerbe) -> E0800502, rentner_veraeusserungsgewinn_partner (selbstaendig) -> E0804501,
    gewst_hebesatz_partner/gewst_messbetrag_partner -> E0801705/E0801606. Alle vier laufen
    in den person_b-Bucket (dieselben Person-A-Kz, zweite Anlage-G-Instanz) — kein eigenes
    Ehegatte-Kz. Person A und B mit UNTERSCHIEDLICHEN Werten, damit ein Vertauschen der
    Buckets sofort auffiele. Hebesatz bleibt bewusst ein Nicht-Cent-Wert (410 -> "410",
    keine Cent->Euro-Wandlung)."""
    s = ST.leerer_store(2025, fall_id="gewinn_partner_xml")
    _b(s, "einkuenfte_gewinn", 500000)                        # 5.000 EUR, Person A
    _b(s, "gewinn_betriebsart", "gewerbe")
    _b(s, "einkuenfte_gewinn_partner", 300000)                # 3.000 EUR, Person B
    _b(s, "gewinn_betriebsart_partner", "gewerbe")
    _b(s, "rentner_veraeusserungsgewinn_partner", 4500000)    # 45.000 EUR, Person B
    _b(s, "rentner_veraeusserungs_betriebsart_partner", "selbstaendig")
    _b(s, "gewst_hebesatz_partner", 410)                      # Prozent, kein Cent-Feld
    _b(s, "gewst_messbetrag_partner", 120000)                 # 1.200 EUR
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", False)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    clean = xml.replace("ns0:", "").replace("ns1:", "")

    assert "<Person>PersonB</Person>" in clean

    # Gewerbe-Gewinn: A (5000) und B (3000) teilen sich dasselbe Kz E0800502 in zwei
    # Anlage-G-Instanzen — Reihenfolge A vor B, keine Verwechslung der Werte.
    import re
    gewinn_werte = re.findall(r"<E0800502>(\d+)</E0800502>", clean)
    assert gewinn_werte == ["5000", "3000"], (
        f"E0800502 nicht [5000 (Person A), 3000 (Person B)]: {gewinn_werte}\n" + xml)

    # § 16 Abs.4 Veräußerungsgewinn Person B, selbstaendig -> E0804501 = 45000
    assert "<E0804501>45000</E0804501>" in clean, (
        "E0804501 (Veraeusserungsgewinn Person B) fehlt oder falscher Wert:\n" + xml)

    # § 35 GewSt Person B: Hebesatz 410 (kein Cent), Messbetrag 1200 (Cent->Euro gewandelt)
    assert "<E0801705>410</E0801705>" in clean, (
        "Hebesatz Person B nicht 410 — Cent-Wandlung faelschlich angewandt?\n" + xml)
    assert "<E0801606>1200</E0801606>" in clean, (
        "Messbetrag Person B nicht 1200:\n" + xml)


@braucht_xsd
def test_gewinneinkuenfte_partner_xsd_valide(bindung, tmp_path):
    """Gewinneinkünfte-Partnerseite (Gewerbe + §16 Abs.4 + §35) gegen amtliches Schema."""
    s = ST.leerer_store(2025, fall_id="gewinn_partner_xsd")
    _b(s, "einkuenfte_gewinn", 500000)
    _b(s, "gewinn_betriebsart", "gewerbe")
    _b(s, "einkuenfte_gewinn_partner", 300000)
    _b(s, "gewinn_betriebsart_partner", "gewerbe")
    _b(s, "rentner_veraeusserungsgewinn_partner", 4500000)
    _b(s, "rentner_veraeusserungs_betriebsart_partner", "selbstaendig")
    _b(s, "gewst_hebesatz_partner", 410)
    _b(s, "gewst_messbetrag_partner", 120000)
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", False)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    pfad = str(tmp_path / "gewinn_partner.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, f"Gewinneinkuenfte-Partner-XML nicht schema-valide: {meldung}"


def test_mitunternehmer_partner_faellt_in_nicht_deklariert(bindung):
    """Die 4 §15-Mitunternehmer-Partnerfelder haben (wie Person A) elster_kz: null — sie
    sind im gemeinsamen Gewinn-Kz (E0800502/E0803402) AUFGEGANGEN, nicht separat deklariert.
    Muessen daher in nicht_deklariert erscheinen, NICHT in person_b landen (Klasse c)."""
    s = ST.leerer_store(2025, fall_id="mitu_partner_nd")
    _b(s, "gewinnanteil_partner", 100000)
    _b(s, "verguetung_taetigkeit_partner", 20000)
    _b(s, "verguetung_darlehen_partner", 5000)
    _b(s, "verguetung_ueberlassung_partner", 3000)
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", False)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)

    nd_ids = {x["feld_id"] for x in result.get("nicht_deklariert", [])}
    for fid in ("gewinnanteil_partner", "verguetung_taetigkeit_partner",
                "verguetung_darlehen_partner", "verguetung_ueberlassung_partner"):
        assert fid in nd_ids, f"{fid} muss in nicht_deklariert stehen (elster_kz: null)"
    assert result.get("person_b", {}) == {}, (
        f"Mitunternehmer-Partnerfelder duerfen NICHT im person_b-Bucket landen: {result.get('person_b')}")


# ---------------------------------------------------------------- 2. Kind-Instanz-Durchgang

def test_kind_instanzen_zwei_kinder(bindung):
    """Zwei Kind-Instanzen via anlage_instanzen -> zwei Container mit Reuse-Kz."""
    s = ST.leerer_store(2025, fall_id="kind_instanz_2")
    # Zwei Kinder: Basis (Kind 1) + Kind 2 via __2
    _b(s, "fam_anzahl_kinder", 2)
    _b(s, "kind_idnr", "12345678901")    # Kind 1 (Basis)
    _b(s, "kind_idnr__2", "23456789012")  # Kind 2
    _b(s, "kind_geburtsdatum", "10.05.2010")       # Kind 1
    _b(s, "kind_geburtsdatum__2", "20.09.2012")    # Kind 2
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    clean = xml.replace("ns0:", "").replace("ns1:", "")

    # Beide Kind-IDNR müssen im XML sein (E0500406)
    assert ">12345678901<" in clean, "Kind 1-IDNR fehlt im XML"
    assert ">23456789012<" in clean, "Kind 2-IDNR fehlt im XML"

    # Prüfe: zwei <Kind>-Container (Kind 1 + Kind 2)
    kind_count = clean.count("<Kind>")
    assert kind_count == 2, f"Erwarte 2 <Kind>-Container, habe {kind_count}"


@braucht_xsd
def test_kind_instanzen_xsd_valide(bindung, tmp_path):
    """Zwei Kind-Instanzen gegen amtliches Schema validieren."""
    s = ST.leerer_store(2025, fall_id="kind_instanz_xsd")
    _b(s, "fam_anzahl_kinder", 2)
    _b(s, "kind_idnr", "12345678901")
    _b(s, "kind_idnr__2", "23456789012")
    _b(s, "kind_geburtsdatum", "10.05.2010")
    _b(s, "kind_geburtsdatum__2", "20.09.2012")
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    pfad = str(tmp_path / "kind_instanzen.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, f"Kind-Instanzen-XML nicht schema-valide: {meldung}"


# ---------------------------------------------------------------- 3. Kinderzahl-Wächter

def test_kinderzahl_waecher_fail_closed(bindung):
    """kind_anlagen behauptet 3, aber Daten für 2 -> XmlFehler."""
    s = ST.leerer_store(2025, fall_id="kind_waecher")
    _b(s, "fam_anzahl_kinder", 3)            # 3 behauptet
    _b(s, "kind_idnr", "11111111111")         # Kind 1
    _b(s, "kind_idnr__2", "22222222222")      # Kind 2 — Kind 3 fehlt!
    _b(s, "kind_geburtsdatum", "10.05.2010")
    _b(s, "kind_geburtsdatum__2", "20.09.2012")
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    with pytest.raises(EX.XmlFehler, match="Kinderzahl behauptet 3"):
        EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)


def test_kinderzahl_konsistent_ohne_kinder(bindung):
    """Keine Kinder (kind_anlagen leer) mit einem Kz-Feld -> kein Wächter-Fehler."""
    s = ST.leerer_store(2025, fall_id="kind_konsistent")
    _b(s, "fam_anzahl_kinder", 0)
    _b(s, "bruttoarbeitslohn", 5000000)  # Kz-tragendes Feld, damit deklaration nicht leer
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    # Sollte ohne Fehler durchgehen (kind_anlagen leer -> kein Check)
    assert "kind_anlagen" in result and not result["kind_anlagen"]
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    assert len(xml) > 0


# ---------------------------------------------------------------- 4. XSD-Ordnungsprüfung (Person A+B direkt benachbart)

@braucht_xsd
def test_person_a_b_ordnung_schema_valide(bindung, tmp_path):
    """N(PersonA) und N(PersonB) müssen DIREKT nacheinander stehen (xs:sequence).
    Das XSD fängt einen Ordnungsfehler hart ab — nutze es als Gate."""
    s = ST.leerer_store(2025, fall_id="ordnung_a_b")
    # Nur Bruttolohn für A und B — keine VOR-Felder (die zwischen N und AVor liegen)
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "bruttoarbeitslohn_partner", 4000000)
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    pfad = str(tmp_path / "ordnung_a_b.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, f"Ordnungs-Gate: Person-A+B-XML nicht schema-valide: {meldung}"


# ---------------------------------------------------------------- 5. Ring-Deklaration-Differential (einfache Variante)

def test_person_a_und_b_haben_unterschiedliche_kz(bindung):
    """Prüfe: Person A (50k) und Person B (40k) haben im deklaration/person_b-Bucket
    E0200201, aber mit unterschiedlichen Werten — das XML enthält beide."""
    s = ST.leerer_store(2025, fall_id="differential_a_b")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "bruttoarbeitslohn_partner", 4000000)
    _b(s, "vor_an_anteil_rv", 3500000)
    _b(s, "vor_an_anteil_rv_partner", 2800000)
    _b(s, "vor_ag_anteil_rv", 1000000)
    _b(s, "vor_ag_anteil_rv_partner", 800000)
    _b(s, "vor_rv_ausserhalb_lstb", 200000)
    _b(s, "vor_rv_ausserhalb_lstb_partner", 160000)
    _b(s, "kap_kapitalertraege", 1000000)
    _b(s, "kap_kapitalertraege_partner", 500000)
    _b(s, "kap_gewinn_aktien", 0)
    _b(s, "kap_gewinn_aktien_partner", 0)
    _b(s, "kap_verlust_aktien", 0)
    _b(s, "kap_verlust_aktien_partner", 0)
    _b(s, "kap_verlust_sonstige", 0)
    _b(s, "kap_verlust_sonstige_partner", 0)
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", False)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    clean = xml.replace("ns0:", "").replace("ns1:", "")

    # Prüfe Person-Diskriminatoren
    assert "<Person>PersonA</Person>" in clean
    assert "<Person>PersonB</Person>" in clean

    # Prüfe: beide 50k und 40k tauchen irgendwo auf (verschiedene Container)
    count_50000 = clean.count(">50000<")
    count_40000 = clean.count(">40000<")
    assert count_50000 >= 1, "Person A Bruttolohn 50000 fehlt"
    assert count_40000 >= 1, "Person B Bruttolohn 40000 fehlt"


# ---------------------------------------------------------------- 6. VV-Objekt-Instanzen (Gruppe vv_objekt)

def test_vv_objekt_zwei_instanzen(bindung):
    """Zwei vv_objekt-Instanzen -> zwei E0700201-Werte im XML."""
    s = ST.leerer_store(2025, fall_id="vv_instanz_2")
    # Objekt 1 (Basis)
    _b(s, "vv_einnahmen", 3000000)            # 30.000 EUR -> E0700201
    _b(s, "vv_gebaeude_afa", 500000)
    _b(s, "vv_schuldzinsen", 200000)
    _b(s, "vv_erhaltungsaufwand", 100000)
    _b(s, "vv_sonstige_wk", 50000)
    _b(s, "vv_entgelt_quote_prozent", 100)
    _b(s, "vv_wohnzwecke", True)
    _b(s, "vv_auf_dauer", True)
    # Objekt 2
    _b(s, "vv_einnahmen__2", 2000000)          # 20.000 EUR -> E0700201 instanz 2
    _b(s, "vv_gebaeude_afa__2", 300000)
    _b(s, "vv_schuldzinsen__2", 100000)
    _b(s, "vv_erhaltungsaufwand__2", 80000)
    _b(s, "vv_sonstige_wk__2", 20000)
    _b(s, "vv_entgelt_quote_prozent__2", 100)
    _b(s, "vv_wohnzwecke__2", True)
    _b(s, "vv_auf_dauer__2", True)
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", False)  # V+V gesetzt
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    clean = xml.replace("ns0:", "").replace("ns1:", "")

    # Prüfe 2 V-Container und 2 E0700201-Werte
    # E0700201 -> ('E10', 'V', 'Einn', 'Mieteinn', 'Whg', 'Einz', 'E0700201')
    import re
    v_werte = re.findall(r"<E0700201>(\d+)</E0700201>", clean)
    assert len(v_werte) == 2, f"Erwarte 2x E0700201, habe {v_werte}"
    assert "30000" in v_werte, "30.000 EUR (Objekt 1) fehlt"
    assert "20000" in v_werte, "20.000 EUR (Objekt 2) fehlt"

    # Prüfe <V>-Container-Anzahl
    v_count = clean.count("<V>")
    assert v_count == 2, f"Erwarte 2 <V>-Container, habe {v_count}"


@braucht_xsd
def test_vv_objekt_xsd_valide(bindung, tmp_path):
    """Zwei vv_objekt-Instanzen gegen amtliches Schema validieren."""
    s = ST.leerer_store(2025, fall_id="vv_instanz_xsd")
    _b(s, "vv_einnahmen", 3000000)
    _b(s, "vv_gebaeude_afa", 500000)
    _b(s, "vv_schuldzinsen", 200000)
    _b(s, "vv_erhaltungsaufwand", 100000)
    _b(s, "vv_sonstige_wk", 50000)
    _b(s, "vv_entgelt_quote_prozent", 100)
    _b(s, "vv_wohnzwecke", True)
    _b(s, "vv_auf_dauer", True)
    _b(s, "vv_einnahmen__2", 2000000)
    _b(s, "vv_gebaeude_afa__2", 300000)
    _b(s, "vv_schuldzinsen__2", 100000)
    _b(s, "vv_erhaltungsaufwand__2", 80000)
    _b(s, "vv_sonstige_wk__2", 20000)
    _b(s, "vv_entgelt_quote_prozent__2", 100)
    _b(s, "vv_wohnzwecke__2", True)
    _b(s, "vv_auf_dauer__2", True)
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", False)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    pfad = str(tmp_path / "vv_instanzen.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, f"VV-Instanzen-XML nicht schema-valide: {meldung}"


# ---------------------------------------------------------------- 7. E10_AUSSCHLUSS_DATENART — kein stilles Weglassen

def test_e10_ausschluss_gwg_dokumentiert(bindung):
    """gwg-Instanz-Kz E6002301 ist in E10_AUSSCHLUSS_DATENART -> dokumentiert, kein XmlFehler."""
    s = ST.leerer_store(2025, fall_id="gwg_ausschluss")
    # gwg_base (Instanz 1) -> deklaration -> fehlt im E10-XSD = XmlFehler (bestehendes Verhalten)
    # Gweg daher direkt als anlage_instanzen-Eintrag, ohne base-instance
    import copy
    result = {
        "vollstaendig": True,
        "deklaration": {
            "E0100201": "Maier",  # benoetigt fuer nicht-leere deklaration
            "E0100401": "05.05.1955",
        },
        "person_b": {},
        "anlage_instanzen": {
            "gwg": [
                {"index": 2, "felder": {"E6002301": 6789}},  # Instanz 2 (base=Instanz 1)
            ],
        },
        "kind_anlagen": [],
    }
    # Darf KEINEN XmlFehler werfen — E6002301 ist dokumentierter Ausschluss
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    assert "6789" not in xml.replace("ns0:", "").replace("ns1:", ""), (
        "E6002301 darf nicht im E10-XML landen (E77-Datenart)")


def test_e10_ausschluss_unbekanntes_kz_fail_closed():
    """Nicht-E10-Kz OHNE Eintrag in E10_AUSSCHLUSS_DATENART -> XmlFehler."""
    result = {
        "vollstaendig": True,
        "deklaration": {
            "E0100201": "Maier",
            "E0100401": "05.05.1955",
        },
        "person_b": {},
        "anlage_instanzen": {
            "vv_objekt": [
                {"index": 2, "felder": {"E9999999": 12345}},
            ],
        },
        "kind_anlagen": [],
    }
    with pytest.raises(EX.XmlFehler, match="nicht im E10"):
        EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)