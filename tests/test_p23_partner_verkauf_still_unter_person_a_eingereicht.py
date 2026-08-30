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
    sind zwei GETRENNTE Achsen im Schema: mehrere Verkaeufe DERSELBEN Person laufen ueber `Einz`,
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
    Damals (bis fa9453d): #<SO> im XML = 2 (ZWEI separate <SO>-Elemente), beide mit
    Person=PersonA, PersonB kommt kein einziges Mal vor -- xmllint gegen E10-2025.xsd: UNGUELTIG
    ("Element '...SO': This element is not expected", SO hat maxOccurs=1).

NACHTRAG 2026-08-31 (fa9453d, "fix(p23): Mehrfachverkauf EINER Person an <Einz> statt an <SO>
gehaengt"): eine Nachbarinstanz hat die xmllint-Ablehnung oben beseitigt, indem der zweite
Verkauf jetzt am schon vorhandenen Wiederholungssegment <Grdst>/<Einz> (maxOccurs=99) haengt statt
an einem zweiten <SO>-Geschwister. Live nachgemessen (dieser Datei, gegen einen sauberen Klon von
HEAD fa9453d979890a81bc164c3d45c32ddc47cf76c4):

  ZWEI Verkaeufe, JETZT (fa9453d): #<SO> im XML = 1 (nur noch EIN <SO>, xmllint: VALIDE -- der
  Fix wirkt, das Schema akzeptiert das Dokument). Trotzdem: beide Person-Werte im Dokument sind
  weiterhin "PersonA" -- gefunden {(PersonA,45000), (PersonA,8000)}, erwartet
  {(PersonA,45000), (PersonB,8000)}. PersonB kommt weiterhin kein einziges Mal vor.

DER KERN (warum die xmllint-Fassung den Befund verloren hat, nicht der Befund selbst verschwunden
ist): Schema-Gueltigkeit war hier ein STELLVERTRETERMERKMAL. Die alte xmllint-Ablehnung maass die
Personen-Achse nur ZUFAELLIG mit, weil der damalige Fehler (Instanz-Verankerung am generischen
E10-Direktkind 'SO' statt an der gruppenspezifischen Wiederholungsebene) gleichzeitig maxOccurs=1
verletzte. Die Person-Achse (Grdst/And_WG, Pflicht-Kindfeld 'Person', maxOccurs=2) und die
Mehrfachheits-Achse (Einz, maxOccurs=99, KEIN Person-Feld) sind im Schema zwei GETRENNTE
Konstrukte (s. XSD-Befund oben) -- fa9453d hat nur die Mehrfachheits-Achse repariert (Einz statt
SO), die Person-Achse (welche der beiden Grdst-Geschwister-Kandidaten -- hier: gar keiner, beide
Verkaeufe teilen sich dieselbe Grdst-Instanz -- welchen Person-Wert traegt) bleibt unveraendert
falsch. Weil <Einz> strukturell GAR KEIN Person-Feld traegt, kann das Schema diesen Fehler an
dieser Stelle grundsaetzlich nicht mehr sehen -- nicht "noch nicht", sondern prinzipiell nicht,
solange der zweite Verkauf ueber Einz statt ueber eine zweite Grdst-Instanz mit Person=PersonB
eingehaengt wird. Ein spaeterer Aufraeumer, der nur "der Marker ist jetzt XPASS, also ist der
Fehler behoben" liest, wuerde den Melder mit dem Defekt verwechseln.

Ursache (produkt/import/elster_xml.py, erzeuge_xml()) bleibt unveraendert gegenueber der obigen
Analyse: est_mapping.py schreibt den zweiten Verkauf nach wie vor ausschliesslich in
anlage_instanzen, nie in person_b. fa9453d hat NUR den Verankerungs-Level der Instanz geaendert
(INSTANZ_CONTAINER_TIEFER: Gruppe 'p23_veraeusserung' -> 'Einz' statt 'SO'), nicht WELCHE Person
die Instanz traegt -- die Person bleibt implizit die des Grdst-Blocks, in den Einz eingehaengt
wird, und es existiert weiterhin nur EIN Grdst-Block (Person A).

Reparaturrichtung bewusst offen gelassen (neues `_partner`-Feld vs. Person-Enum an der bestehenden
Instanz -- nicht entschieden). Die Erwartung unten prueft deshalb NICHT auf einen Feldnamen,
sondern auf die Struktur, die das Schema selbst verlangt: genau EIN <SO>, mit zwei
Person-unterschiedenen Verkaufsblocken und den zwei unterscheidbaren Betraegen darin.

Unterscheidbare Betraege (45.000 / 8.000 EUR) sind Pflicht -- gleiche Betraege wuerden einen
verschluckten Partner-Verkauf wie eine funktionierende Zuordnung aussehen lassen.

WAS DIESE DATEI NICHT BEHAUPTET:
  - keine Aussage, welcher Reparaturmechanismus (neues Feld vs. Person-Enum an bestehender
    Instanz) richtig ist -- s. oben.
  - keine Aussage ueber die Betragshoehe/-berechnung der Verkaeufe selbst -- nur ueber die
    Person-Zuordnung der bereits berechneten Betraege im XML.
  - keine Aussage ueber ERiC/die amtliche Plausibilitaetspruefung: diese Datei prueft nur
    XSD-Schema-Validitaet (xmllint) und die Struktur des erzeugten XML, NICHT den ERiC-Testmerker-
    Pfad. fa9453d's eigene Commit-Message nennt zusaetzlich einen unabhaengigen ERiC-Befund
    (FachlicheFehlerId=zuGrosseLfdNummer, rc=610001002) fuer den ALTEN Zustand -- ob ERiC die
    NEUE (Einz-basierte) Fassung ebenfalls durchwinkt, ist hier NICHT gemessen.
  - keine Aussage, dass das Schema selbst falsch waere: Einz ohne Person-Feld ist eine legitime,
    amtliche Konstruktion fuer "mehrere Verkaeufe derselben Person" -- der Fehler liegt in
    TaxGraph, das den zweiten Verkauf faelschlich ueber Einz statt ueber eine zweite,
    Person-B-markierte Grdst-Instanz einhaengt.
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
# Verkauf 2 (Partner, einzige heute verfuegbare Eingabe: die Zaehl-Instanz __2) -- bewusst ein
# ANDERER Betrag als Verkauf 1, sonst waere ein verschluckter Partner-Verkauf von einer
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
    reason="fa9453d (Mehrfachverkauf haengt jetzt an <Einz> statt an einem zweiten <SO>) hat die "
           "SICHTBARE Symptomatik verschoben, den Defekt selbst nicht behoben: anzahl_so ist jetzt "
           "1 (schema-konform, vorher 2), aber beide Verkaeufe tragen weiterhin Person=PersonA -- "
           "gemessen {('PersonA','45000'), ('PersonA','8000')} statt der erwarteten "
           "{('PersonA','45000'), ('PersonB','8000')}. Vor fa9453d scheiterte diese Assertion "
           "schon frueher, an anzahl_so==2 (zwei separate <SO>-Elemente) -- die Ursache "
           "(est_mapping.py schreibt den zweiten Verkauf nur in anlage_instanzen, nie in "
           "person_b) ist unveraendert dieselbe, nur die zuerst sichtbare Konsequenz hat sich "
           "verschoben. Marker faellt am Tag des Fixes (XPASS) und zwingt dazu, ihn zu entfernen.")
def test_person_a_und_partner_verkauf_zwei_instanzen_mit_unterschiedlichem_person_kennzeichen(bindung):
    """Erwartung nach Fix (reparaturrichtungsneutral -- ob ueber ein neues _partner-Feld oder ueber
    das Person-Enum an der bestehenden Instanz, ist nicht entschieden): genau EIN <SO>-Element mit
    zwei Person-unterschiedenen Verkaufsbloecken, je einem der beiden unterscheidbaren Betraege.
    Heute (gemessen, HEAD fa9453d): ein <SO>-Element (Schema-Achse repariert), beide Verkaeufe
    weiterhin Person=PersonA (Personen-Achse unveraendert falsch) -- der Partner-Verkauf wird
    still als zweiter eigener Verkauf der Person A eingereicht, kein Fehler, kein Hinweis."""
    result, xml_text = _xml_bauen(bindung, {**_VERKAUF_A, **_VERKAUF_PARTNER_UEBER_INSTANZ_2})
    assert result.get("eingaben_konsistent") is True
    anzahl_so, paare = _so_grdst_person_betrag_paare(xml_text)
    erwartet = {("PersonA", E0306801_PERSON_A_EUR), ("PersonB", E0306801_PARTNER_EUR)}
    assert anzahl_so == 1, f"erwartet genau ein <SO>-Element (Schema: maxOccurs=1), gefunden {anzahl_so}"
    assert paare == erwartet, f"erwartet {erwartet}, gefunden {paare}"


@braucht_xsd
@pytest.mark.xfail(
    strict=True,
    reason="Ersetzt die alte xmllint-Ablehnung (bis fa9453d): das Schema lehnt das Zwei-Verkaeufe-"
           "XML jetzt NICHT mehr ab (gemessen unten, xmllint akzeptiert es -- der SO-maxOccurs-Fix "
           "wirkt). Trotzdem bleibt die Personen-Achse falsch: beide Verkaeufe stehen weiterhin "
           "unter Person=PersonA. Schema-Gueltigkeit war ein Stellvertretermerkmal -- sie maass "
           "die Personen-Zuordnung nur zufaellig mit, solange derselbe Fehler gleichzeitig "
           "maxOccurs=1 auf <SO> verletzte. <Einz> (wo der zweite Verkauf jetzt haengt) traegt "
           "KEIN Person-Feld -- das Schema kann diesen Fehler an dieser Stelle grundsaetzlich "
           "nicht mehr erkennen, nicht nur heute nicht. Marker faellt, sobald der zweite Verkauf "
           "tatsaechlich unter PersonB einsortiert wird.")
def test_person_a_und_partner_verkauf_ist_heute_xsd_valide_aber_personenachse_falsch(bindung, tmp_path):
    """Vorher (bis fa9453d) lehnte xmllint das Zwei-Verkaeufe-XML strukturell ab -- diese
    Ablehnung war der staerkste amtliche Beleg fuer den Defekt, ist aber KEIN direkter Beleg der
    Personen-Achse gewesen (s. Moduldocstring "DER KERN"). Diese Fassung prueft beides getrennt:
    zuerst die (jetzt WAHRE) Schema-Validitaet als Tatsachenfeststellung, danach die Personen-
    Achse direkt (wie test_..._zwei_instanzen_mit_unterschiedlichem_person_kennzeichen) als die
    Assertion, an der der Test tatsaechlich xfailt."""
    _, xml_text = _xml_bauen(bindung, {**_VERKAUF_A, **_VERKAUF_PARTNER_UEBER_INSTANZ_2})
    pfad = str(tmp_path / "defekt.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml_text)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, f"erwartet (fa9453d): xmllint akzeptiert das Dokument jetzt -- {meldung}"
    anzahl_so, paare = _so_grdst_person_betrag_paare(xml_text)
    erwartet = {("PersonA", E0306801_PERSON_A_EUR), ("PersonB", E0306801_PARTNER_EUR)}
    assert anzahl_so == 1, f"erwartet genau ein <SO>-Element, gefunden {anzahl_so}"
    assert paare == erwartet, f"erwartet {erwartet}, gefunden {paare}"
