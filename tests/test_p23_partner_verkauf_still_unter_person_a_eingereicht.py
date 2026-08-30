"""§ 23 Anlage SO, Zusammenveranlagung: ein Verkauf des Partners wird nicht abgelehnt und nicht als
Partner-Verkauf erkannt, sondern still als zusaetzlicher eigener Verkauf der Person A eingereicht --
kein Fehler, kein Hinweis, unter der Unterschrift der Person A.

Vorgeschichte (Instructor-Auftrag 2026-08-30, "§23 Partner-Eingabepfad"): es gibt kein
`_partner`-Feld fuer einen privaten Veraeusserungsgewinn und keinen Eintrag in
PARTNER_INSTANZ/PARTNER_VERZWEIGUNG (est_mapping.py) -- die einzige verfuegbare Mechanik fuer einen
zweiten Verkauf ist die generische, zaehlbasierte Instanz (`p23_veraeusserungspreis__2` usw.), die
keine Person unterscheidet. `PARTNER_INSTANZ` ist TaxGraph's eigener Bindungsschicht-Begriff, KEIN
XSD-Begriff.

XSD-Befund (E10-2025.xsd, ERiC 44.2.4.0, Zeilenangaben nur zur Einordnung -- Zeilennummern altern,
deshalb hier im Docstring, nicht in der Assertion):
  - `SO` (Anlage SO) selbst: maxOccurs="1" (:8298) -- "SO doppelt" ist keine Option.
  - Innerhalb von SO/Priv_VA_G sind FUENF Bloecke maxOccurs="2": Grdst (:22217), Virt_Waehr,
    And_WG (:22219), Ant_Ek, Begr_V_Rue -- jeder mit einem PFLICHT-Kindfeld `Person`
    (Enum PersonA/PersonB) als erstes Kind (Grdst :22226, And_WG :22364). Das ist dasselbe
    Person-Unterscheidungsmerkmal, das Anlage N an ihrer eigenen maxOccurs="2"-Stelle traegt.
  - JEDER der fuenf Bloecke traegt zusaetzlich ein xs:unique auf (Block, Person): Grdst
    (:21891-21894), Virt_Waehr (:21895-21898), And_WG (:21899-21902), Ant_Ek (:21903-21906),
    Begr_V_Rue (:21907-21910) -- das Schema VERBIETET zwei Instanzen desselben Blocks mit
    demselben Person-Wert, wenn sie Geschwister unter derselben Priv_VA_G sind.
  - `Einz` innerhalb von Grdst ist zusaetzlich maxOccurs="99" (:22231) -- Person und Mehrfachheit
    sind zwei GETRENNTE Achsen im Schema: mehrere Verkaufe DERSELBEN Person laufen ueber `Einz`,
    ein zweiter FILER laeuft ueber eine zweite Grdst-Instanz mit anderem Person-Wert. Ein Testfall
    fuer "zwei Verkaeufe derselben Person" ist deshalb bewusst NICHT Teil dieser Datei (Instructor
    2026-08-30: eine Nachbarinstanz vermisst dieses Muster separat, Ergebnis offen).

Live gemessen (HEAD 1a96285b62e38a4cc3db9b9bee816996f22c03b6, direkt ueber
est_mapping.deklariere()+elster_xml.erzeuge_xml(), NICHT ueber API.ergebnis()/den Guard -- diese
Pipeline ist von der zeitgleich im Baum liegenden, unfertigen kein_p23_verkauf-Aenderung
unberuehrt, gemessen: eingaben_konsistent bleibt in beiden Faellen unten True):

  EIN Verkauf (Person A, 45.000 EUR):
    #<SO> im XML = 1, genau ein (Person, E0306801)-Paar: (PersonA, 45000).
    xmllint gegen das amtliche E10-2025.xsd: VALIDE.

  ZWEI Verkaeufe (Person A 45.000 EUR + "Partner" 8.000 EUR, ueber die einzige verfuegbare
  Zaehl-Instanz __2 -- der einzige heute existierende Eingabeweg fuer einen zweiten Verkauf):
    #<SO> im XML = 2 (ZWEI separate <SO>-Elemente, nicht ein <SO> mit zwei Grdst-Geschwistern),
    beide mit Person=PersonA, PersonB kommt kein einziges Mal vor. Beide Betraege stehen im XML,
    aber jeweils in einem eigenen, gegen das Schema ungueltigen zweiten <SO>-Block.
    xmllint gegen das amtliche E10-2025.xsd: UNGUELTIG --
      "Element '...SO': This element is not expected." (SO hat maxOccurs=1; das eigentliche
      xs:unique(Grdst,Person) wird gar nicht erst erreicht, weil die zwei Grdst-Instanzen heute
      keine Geschwister unter derselben Priv_VA_G sind, sondern in getrennten SO-Bloecken liegen --
      ein noch grundlegenderer Schema-Verstoss als die eingangs vermutete xs:unique-Verletzung.)

Ursache (produkt/import/elster_xml.py, erzeuge_xml()): der generische Instanz-Mechanismus fuer
`anlage_instanzen` verankert eine neue Zaehl-Instanz am direkten E10-Kind des Kz-Pfads
(`kz_path[:2]`, hier also immer `("E10","SO")`) -- richtig fuer Gruppen wie kind/gwg/vv_objekt/
rente, wo genau diese Ebene die tatsaechlich wiederholbare ist, falsch fuer p23/SO, dessen
wiederholbare Ebene drei Stufen tiefer liegt (Grdst/And_WG). Der bereits vorhandene, generische
Person-B-Mechanismus des Writers (pflicht_kinder()/_bestimme_person_container(), erkennt
Person-Container ueber die im Schema als Pflicht markierten Kinder) faende Grdst/And_WG schon
heute korrekt -- der gesamte Mangel liegt in est_mapping.py, das den zweiten Verkauf nie in den
person_b-Bucket schreibt, sondern ausschliesslich in anlage_instanzen.

Reparaturrichtung bewusst offen gelassen (neues `_partner`-Feld vs. Person-Enum an der bestehenden
Instanz -- nicht entschieden). Die Erwartung unten prueft deshalb NICHT auf einen Feldnamen,
sondern auf die Struktur, die das Schema selbst verlangt: genau EIN <SO>, mit zwei
Person-unterschiedenen Verkaufsblocken und den zwei unterscheidbaren Betraegen darin.

Unterscheidbare Betraege (45.000 / 8.000 EUR) sind Pflicht -- gleiche Betraege wuerden einen
verschluckten Partner-Verkauf wie eine funktionierende Zuordnung aussehen lassen.
"""
from __future__ import annotations

import os
import sys
import xml.etree.ElementTree as ET

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/import", "produkt/store", "produkt/traverser", "produkt/mapping"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import elster_xml as EX      # noqa: E402
import est_mapping as EM     # noqa: E402
import store as ST           # noqa: E402
import traverser as TR       # noqa: E402

try:
    ROOT_XSD = os.path.dirname(HERE)
    sys.path.insert(0, os.path.join(ROOT_XSD, "elster", "submission"))
    import validate_xsd as VX  # noqa: E402
    _schema_da = VX.find_schema("2025") is not None
    _xmllint_da = bool(__import__("shutil").which("xmllint"))
except Exception:
    VX = None
    _schema_da = _xmllint_da = False

braucht_xsd = pytest.mark.skipif(
    not (_schema_da and _xmllint_da),
    reason="E10-2025.xsd oder xmllint fehlt -- XSD-Gate nicht lauffaehig")

TS = "2026-08-30T12:00:00Z"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

_STAMM = {
    "stammdaten_nachname": "Meier", "stammdaten_vorname": "Klaus",
    "stammdaten_geburtsdatum": "01.01.1970", "stammdaten_strasse": "Teststr.",
    "stammdaten_hausnummer": "1", "stammdaten_plz": "10115", "stammdaten_wohnort": "Berlin",
    "stammdaten_keine_bankverbindung": True, "stammdaten_art_est_erklaerung": True,
    "kist_konfession": "keine",
}
_BASIS = {
    "veranlagung": "zusammen", "kein_gewinn": True, "kein_kap": True, "kein_vuv": True,
    "kein_sonstige": True, "bruttoarbeitslohn": 0, "vor_an_anteil_rv": 0,
    "vor_ag_anteil_rv": 0, "vor_rv_ausserhalb_lstb": 0,
}

# Verkauf 1 (Person A): 45.000 EUR Gewinn (90.000 - 45.000 Anschaffungskosten).
_VERKAUF_A = {
    "p23_veraeusserungspreis": 9_000_000, "p23_anschaffung_herstellungskosten": 4_500_000,
    "p23_werbungskosten": 0, "p23_veraeusserungs_typ": "grundstueck",
}
# Verkauf 2 (Partner, einzige heute verfuegbare Eingabe: die Zaehl-Instanz __2): 8.000 EUR Gewinn --
# bewusst ein ANDERER Betrag als Verkauf 1, sonst waere ein verschluckter Partner-Verkauf von einer
# funktionierenden Zuordnung nicht zu unterscheiden.
_VERKAUF_PARTNER_UEBER_INSTANZ_2 = {
    "p23_veraeusserungspreis__2": 2_000_000, "p23_anschaffung_herstellungskosten__2": 1_200_000,
    "p23_werbungskosten__2": 0, "p23_veraeusserungs_typ__2": "grundstueck",
}

E0306801_PERSON_A_EUR = "45000"
E0306801_PARTNER_EUR = "8000"


def _b(s, feld_id, wert):
    ST.append_event(store=s, feld_id=feld_id, wert=wert, zustand="bestaetigt", herkunft=H,
                     schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"ok@{feld_id}"}, ts=TS)


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def _lokalname(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def _xml_bauen(bindung, felder: dict) -> tuple[dict, str]:
    """Baut den Store, deklariert und schreibt das ECHTE Uebermittlungs-XML -- nicht den flachen
    `deklaration`-Dict lesen (der zeigt E0306801 fuer die zweite Instanz nicht, s. Docstring):
    est_mapping.deklariere() legt den zweiten Verkauf in `anlage_instanzen` ab, nicht in
    `deklaration`. Erst erzeuge_xml() zeigt, wo er im tatsaechlich abgegebenen Dokument landet."""
    s = ST.leerer_store(2025, fall_id="p23-partner-test")
    for fid, wert in {**_STAMM, **_BASIS, **felder}.items():
        _b(s, fid, wert)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    xml_text = EX.erzeuge_xml(result, vz=2025, hersteller_id="74931",
                              empfaenger_finanzamt="9181", abgabefaehig=False)
    return result, xml_text


def _so_grdst_person_betrag_paare(xml_text: str) -> tuple[int, set[tuple[str, str]]]:
    """(#<SO>-Elemente, {(Person, E0306801-Betrag), ...}) -- strukturell ueber den echten XML-Baum,
    nicht per Substring-Zaehlung. Absichtlich generisch ueber ALLE <SO>-Elemente hinweg: das
    Schema verlangt genau EIN <SO> (maxOccurs=1) -- ob eine Reparatur zwei Grdst-Geschwister in
    EINEM SO erzeugt oder einen anderen amtlich validen Weg waehlt, prueft dieses Paar-Set nicht."""
    root = ET.fromstring(xml_text)
    so_liste = [e for e in root.iter() if _lokalname(e.tag) == "SO"]
    paare: set[tuple[str, str]] = set()
    for so in so_liste:
        for grdst in [e for e in so.iter() if _lokalname(e.tag) == "Grdst"]:
            person = None
            for kind in grdst.iter():
                if _lokalname(kind.tag) == "Person":
                    person = kind.text
                    break
            for kind in grdst.iter():
                if _lokalname(kind.tag) == "E0306801":
                    paare.add((person, kind.text))
    return len(so_liste), paare


# --------------------------------------------------------------- 1) gruene Kontrolle (Person A)

def test_person_a_verkauf_landet_korrekt_im_echten_xml(bindung):
    """Positivbeleg, damit ein spaeterer Fix den heute funktionierenden Normalfall nicht mitnimmt:
    EIN Verkauf von Person A -> genau ein <SO>, Person=PersonA, E0306801=45000 im echten XML."""
    result, xml_text = _xml_bauen(bindung, _VERKAUF_A)
    assert result.get("eingaben_konsistent") is True
    anzahl_so, paare = _so_grdst_person_betrag_paare(xml_text)
    assert anzahl_so == 1, f"erwartet genau ein <SO>-Element, gefunden {anzahl_so}"
    assert paare == {("PersonA", E0306801_PERSON_A_EUR)}, (
        f"erwartet {{('PersonA', {E0306801_PERSON_A_EUR!r})}}, gefunden {paare}")


@braucht_xsd
def test_person_a_verkauf_ist_xsd_valide(bindung, tmp_path):
    """Zusatzbeleg zur Kontrolle oben: das amtliche E10-2025.xsd (via xmllint) akzeptiert den
    Normalfall unveraendert -- ohne diesen Test koennte ein spaeterer Fix versehentlich auch den
    funktionierenden Ein-Verkauf-Fall kaputt machen, ohne dass die Struktur-Assertion oben es merkt."""
    _, xml_text = _xml_bauen(bindung, _VERKAUF_A)
    pfad = str(tmp_path / "kontrolle.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml_text)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, meldung


# --------------------------------------------------------------- 2) der Defekt (Person A + Partner)

@pytest.mark.xfail(
    strict=True,
    reason="est_mapping.py schreibt den zweiten (Partner-)Verkauf nur in anlage_instanzen, nie in "
           "person_b -- elster_xml.erzeuge_xml() verankert jede anlage_instanzen-Instanz am "
           "direkten E10-Kind ('E10','SO') statt an der tatsaechlich wiederholbaren Ebene "
           "(Grdst/And_WG, je maxOccurs=2 im Schema). Gemessen: zwei SEPARATE <SO>-Elemente, "
           "beide Person=PersonA, PersonB kommt nicht vor -- das Schema erlaubt nur EIN <SO> "
           "(maxOccurs=1). Marker faellt am Tag des Fixes (XPASS) und zwingt dazu, ihn zu entfernen.")
def test_person_a_und_partner_verkauf_zwei_instanzen_mit_unterschiedlichem_person_kennzeichen(bindung):
    """Erwartung nach Fix (reparaturrichtungsneutral -- ob ueber ein neues _partner-Feld oder ueber
    das Person-Enum an der bestehenden Instanz, ist nicht entschieden): genau EIN <SO>-Element mit
    zwei Person-unterschiedenen Verkaufsbloecken, je einem der beiden unterscheidbaren Betraege.
    Heute (gemessen): zwei <SO>-Elemente, beide Person=PersonA -- der Partner-Verkauf wird still
    als zweiter eigener Verkauf der Person A eingereicht, kein Fehler, kein Hinweis."""
    result, xml_text = _xml_bauen(bindung, {**_VERKAUF_A, **_VERKAUF_PARTNER_UEBER_INSTANZ_2})
    assert result.get("eingaben_konsistent") is True
    anzahl_so, paare = _so_grdst_person_betrag_paare(xml_text)
    erwartet = {("PersonA", E0306801_PERSON_A_EUR), ("PersonB", E0306801_PARTNER_EUR)}
    assert anzahl_so == 1, f"erwartet genau ein <SO>-Element (Schema: maxOccurs=1), gefunden {anzahl_so}"
    assert paare == erwartet, f"erwartet {erwartet}, gefunden {paare}"


@braucht_xsd
@pytest.mark.xfail(
    strict=True,
    reason="Direkte amtliche Bestaetigung desselben Befunds: xmllint gegen E10-2025.xsd lehnt das "
           "heute erzeugte Zwei-Verkaeufe-XML ab (zweites <SO> nicht erwartet, SO hat maxOccurs=1). "
           "Marker faellt am Tag des Fixes (XPASS) und zwingt dazu, ihn zu entfernen.")
def test_person_a_und_partner_verkauf_ist_heute_xsd_ungueltig(bindung, tmp_path):
    """Staerkster amtlicher Beleg: das amtliche Schema selbst lehnt das heutige XML ab. Gemessen
    (HEAD 1a96285b62e38a4cc3db9b9bee816996f22c03b6): xmllint meldet 'Element ...SO: This element
    is not expected' -- das zweite <SO> verletzt maxOccurs=1, bevor das eigentlich vermutete
    xs:unique(Grdst,Person) ueberhaupt geprueft wird."""
    _, xml_text = _xml_bauen(bindung, {**_VERKAUF_A, **_VERKAUF_PARTNER_UEBER_INSTANZ_2})
    pfad = str(tmp_path / "defekt.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml_text)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, meldung
