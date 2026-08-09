"""ELSTER-Submission-XML aus einer Deklaration (P3.2).

Nimmt die flache Kz-Deklaration aus `est_mapping.deklariere()` und baut daraus das
verschachtelte ELSTER-XML — die Verschachtelung kommt NICHT aus einer Handtabelle,
sondern aus dem amtlichen E10-XSD (`xsd_verify.walk()` liefert je Kz den vollen
Element-Pfad). Damit gibt es keine zweite Wahrheit über die Sektionsstruktur.

Fail-closed: eine unvollständige Deklaration (`vollstaendig=False`) wird NICHT
serialisiert — ohne Bestätigung aller Pflichtfelder entsteht kein Submission-XML.
Kz ohne Pfad im Schema sind ein harter Fehler (nie stilles Weglassen).

    from elster_xml import erzeuge_xml
    xml = erzeuge_xml(result, vz=2025, empfaenger_land="BY")

Hersteller-ID kommt aus $ELSTER_HERSTELLER_ID (nie im Code, nie im Repo).
Testmerker default 700000004 (ERiC-Testfall) — für echten Versand explizit None.
"""
from __future__ import annotations

import os
import re
import sys
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(PRODUKT, "mapping"))
sys.path.insert(0, os.path.join(PRODUKT, "traverser"))
import xsd_verify as XV   # noqa: E402  (amtlicher Kz -> Element-Pfad aus dem E10-XSD)

NS_ELSTER = "http://www.elster.de/elsterxml/schema/v11"
NS_E10 = "http://finkonsens.de/elster/elstererklaerung/est/e10/v{vz}"
TESTMERKER_ERIC = "700000004"     # ERiC-Testfall: wird nie ans Finanzamt zugestellt

_PFAD_CACHE: dict[int, dict[str, tuple]] = {}
_PFLICHT_CACHE: dict[int, dict[tuple, list[str]]] = {}

# Pflicht-Diskriminatoren ohne Kz, die das Schema in Personen-Containern verlangt.
# Der Walk findet sie nicht (er indiziert nur Kz), der Writer muss sie setzen.
# Für Person B wird der Wert dynamisch aus dem Container-Index abgeleitet.
PFLICHT_DEFAULT = {"Person": "PersonA"}

# Kz, die in anlage_instanzen auftauchen, aber NICHT ins E10-XML gehören (andere Datenart).
# Jeder Eintrag ist ein EXPLIZITER, benannter Ausschluss mit Begründung — kein stilles
# continue. Fehlt ein Kz hier UND im E10-XSD, fliegt XmlFehler (fail-closed).
# Datenart-Routing: E60xx → E77 (Anlage EÜR, eigene Datenart).
E10_AUSSCHLUSS_DATENART: dict[str, str] = {
    "E6002301": "E77/EÜR (§ 4 Abs. 3 — Gewinnermittlung) Datenart, kein E10-Element",
    "E6004901": "E77/EÜR (§ 4 Abs. 3 — Gewinnermittlung) Datenart, kein E10-Element",
}


class XmlFehler(Exception):
    """Deklaration lässt sich nicht schema-konform serialisieren."""


def kz_pfade(vz: int) -> dict[str, tuple]:
    """{Kz: Element-Pfad-Tupel} aus dem amtlichen E10-<vz>-XSD. Gecacht pro VZ.

    Die Einfüge-Reihenfolge des dict IST die Schema-Reihenfolge (`walk()` läuft top-down
    durch die xs:sequence) — der Writer nutzt sie, um Geschwister schema-konform zu ordnen.
    Alphabetisch sortieren wäre falsch: xs:sequence ist ordnungsempfindlich.

    Mehrfach-Fundstellen (dieselbe Kz an mehreren Schema-Stellen) sind für den Writer
    mehrdeutig — hier gewinnt der kürzeste Pfad, was der ESt1A-Hauptvordruck-Position
    entspricht. Anlagen-Instanzen laufen über `anlage_instanzen`, nicht hierüber.
    """
    if vz in _PFAD_CACHE:
        return _PFAD_CACHE[vz]
    schema = XV._find_schema(vz)
    if not schema:
        raise XmlFehler(f"E10-{vz}.xsd nicht gefunden — $ERIC_DIR setzen / ERiC-Doku entpacken.")
    roh, _ = XV.walk(schema, "E10")
    pfade = {kz: min(fundstellen, key=len) for kz, fundstellen in roh.items()}
    _PFAD_CACHE[vz] = pfade
    return pfade


def _hat_element_kinder(node, type_index: dict) -> bool:
    """True wenn `node` ein echter Container ist (xs:element-Kinder hat).

    Nicht dasselbe wie „hat Content": ein enum-getypter Skalar wie `<Person>` trägt einen
    complexType/simpleType, aber keine Element-Kinder — für den Writer ist er ein Blatt.
    """
    content = XV._content_of(node, type_index)
    if content is None:
        return False
    return any(c.tag == XV.XS + "element" for c in XV._flatten_content(content))


def pflicht_kinder(vz: int) -> dict[tuple, list[str]]:
    """{Container-Pfad: [Pflicht-Nicht-Kz-Kinder]} aus dem E10-<vz>-XSD. Gecacht pro VZ.

    Manche Container verlangen einen skalaren Diskriminator, der keine Kz ist — in Anlage N
    etwa `<Person>PersonA</Person>` vor `<Wk>`. `xsd_verify.walk()` indiziert nur Kz und
    kennt diese Elemente nicht, also scannt der Writer sie hier selbst (minOccurs>=1,
    kein Kz-Name, kein eigener Content = skalares Pflichtfeld).
    """
    if vz in _PFLICHT_CACHE:
        return _PFLICHT_CACHE[vz]
    schema = XV._find_schema(vz)
    if not schema:
        raise XmlFehler(f"E10-{vz}.xsd nicht gefunden — $ERIC_DIR setzen / ERiC-Doku entpacken.")
    top_level = XV._parse_top_level_children(schema)
    type_index, group_index, element_index = XV._load_indices(top_level)
    treffer: dict[tuple, list[str]] = {}
    gesehen: set[tuple] = set()

    def recurse(node, pfad: tuple, tiefe: int) -> None:
        name = node.get("name") or node.get("ref")
        neu = pfad + (name,)
        if tiefe > XV.MAX_DEPTH or neu in gesehen:
            return
        gesehen.add(neu)
        content = XV._content_of(node, type_index)
        if content is None:
            return
        pflicht: list[str] = []
        for child in XV._flatten_content(content):
            if child.tag != XV.XS + "element":
                continue
            ref = child.get("ref")
            kind_name = ref or child.get("name")
            ziel = element_index.get(ref) if ref else child
            if child.get("minOccurs", "1") != "0" and not XV._KZ_RE.match(kind_name or ""):
                if ziel is not None and not _hat_element_kinder(ziel, type_index):
                    pflicht.append(kind_name)      # skalares Pflichtfeld (kein Container)
            if ziel is not None:
                recurse(ziel, neu, tiefe + 1)
        if pflicht:
            treffer[neu] = pflicht

    recurse(element_index["E10"], (), 0)
    _PFLICHT_CACHE[vz] = treffer
    return treffer


def _find_nth_element(parent, tag, n):
    """Das n-te Vorkommen von 'tag' unter 'parent' (0-basiert), None wenn weniger vorhanden."""
    count = 0
    for child in parent:
        if child.tag == tag:
            if count == n:
                return child
            count += 1
    return None


def _bestimme_person_container(pfad: tuple, pflicht: dict[tuple, list[str]]) -> tuple | None:
    """Tiefster Container-Pfad im `pfad`, der <Person> als Pflicht-Kind hat."""
    for i in range(len(pfad), 1, -1):
        if "Person" in pflicht.get(pfad[:i], []):
            return pfad[:i]
    return None


def _wert_text(wert) -> str:
    """Kz-Wert -> XML-Text. Bool ist in ELSTER 'X' (gesetzt) bzw. weggelassen (nicht gesetzt)."""
    if isinstance(wert, bool):
        return "X"
    return str(wert)


def _einhaengen(wurzel: ET.Element, pfad: tuple, wert, ns: str,
                pflicht: dict[tuple, list[str]],
                instanz: dict[tuple, int] | None = None,
                kz_meta: dict[str, dict] | None = None) -> None:
    """Legt den Pfad unterhalb `wurzel` an (idempotent) und setzt das Blatt auf `wert`.

    `pfad` beginnt mit dem Namen der Wurzel selbst (z.B. ('E10','ESt1A','Art_Erkl','E0100001'));
    das erste Segment wird übersprungen, weil `wurzel` genau dieses Element ist. Beim erstmaligen
    Anlegen eines Containers werden dessen skalare Pflichtfelder direkt mitgeschrieben — sie
    stehen im Schema vor den Kz-Kindern, müssen also VOR dem Weiterlaufen gesetzt werden.

    Mit `instanz`: ein Dict {Container-Pfad: Index}, das die zu verwendende Instanz-Nummer
    (0-basiert) für wiederholbare Container angibt — statt `knoten.find(tag)` wird das n-te
    Vorkommen gesucht und ggf. angelegt. Person-Diskriminatoren werden je nach Index gesetzt:
    Index 0 = PersonA, Index >= 1 = PersonB.

    Ja-Typ-Handling (mit `kz_meta`): True → enum-Wert ("1"/"X"), False → Element weglassen.
    Kz ohne Ja-Typ oder ohne kz_meta-Info werden normal über _wert_text gerendert.
    """
    knoten = wurzel
    inst = instanz or {}
    for i, name in enumerate(pfad[1:-1], start=1):
        tag = f"{{{ns}}}{name}"
        container_pfad = pfad[:i + 1]
        inst_idx = inst.get(container_pfad)
        if inst_idx is not None:
            # Instanz-Achse: n-tes Vorkommen suchen oder anlegen
            kind = _find_nth_element(knoten, tag, inst_idx)
            if kind is None:
                # Vorhandene Instanzen bis zur Lücke füllen
                last = None
                for j in range(inst_idx + 1):
                    last = _find_nth_element(knoten, tag, j)
                    if last is None:
                        last = ET.SubElement(knoten, tag)
                        for pflicht_name in pflicht.get(container_pfad, []):
                            if pflicht_name == "Person":
                                wert_person = "PersonB" if j >= 1 else "PersonA"
                                ET.SubElement(last, f"{{{ns}}}{pflicht_name}").text = wert_person
                            else:
                                vorgabe = PFLICHT_DEFAULT.get(pflicht_name)
                                if vorgabe is None:
                                    raise XmlFehler(
                                        f"Container {'/'.join(container_pfad)} verlangt Pflichtfeld "
                                        f"'{pflicht_name}', für das keine Vorgabe hinterlegt ist "
                                        f"(PFLICHT_DEFAULT erweitern).")
                                ET.SubElement(last, f"{{{ns}}}{pflicht_name}").text = vorgabe
                kind = last
            knoten = kind
        else:
            kind = knoten.find(tag)
            if kind is None:
                kind = ET.SubElement(knoten, tag)
                for pflicht_name in pflicht.get(container_pfad, []):
                    vorgabe = PFLICHT_DEFAULT.get(pflicht_name)
                    if vorgabe is None:
                        raise XmlFehler(
                            f"Container {'/'.join(container_pfad)} verlangt Pflichtfeld "
                            f"'{pflicht_name}', für das keine Vorgabe hinterlegt ist "
                            f"(PFLICHT_DEFAULT erweitern).")
                    ET.SubElement(kind, f"{{{ns}}}{pflicht_name}").text = vorgabe
            knoten = kind

    kz_name = pfad[-1]
    # Ja-Typ: True → enum-Wert, False → Element weglassen
    if kz_meta and kz_name in kz_meta:
        km = kz_meta[kz_name]
        if km["is_ja"] and isinstance(wert, bool):
            if wert is True:
                blatt = ET.SubElement(knoten, f"{{{ns}}}{kz_name}")
                blatt.text = km["enums"][0]  # "1" für Ja1, "X" für JaX
            # False → Element weglassen (nicht "X" oder "1" setzen)
            return

    blatt = ET.SubElement(knoten, f"{{{ns}}}{kz_name}")
    blatt.text = _wert_text(wert)


# Absender-Werte fuer den Vorsatz-Block sind KEINE zweite Wahrheit: dieselben Angaben stehen
# bereits als Kz im Hauptvordruck ESt 1 A / Stammdaten Person A (s. _vorsatz()-Docstring).
# {absender_feld: [(Kz, Label), ...]} -- ALLE Kz eines Feldes muessen vorhanden sein, sonst
# bleibt das Feld unabgeleitet (None). absender_steuernummer hat KEINEN Kz-Spiegel -- der
# Vorsatz-Block ist kein E10-Kz-Element (s. _vorsatz()-Docstring) und `stammdaten_steuernummer`
# ist deshalb selbst als bindungslose Bindung (elster_kz: null, kz_status: endgueltig) gebaut.
# Die Ableitung laeuft deshalb nicht ueber diese Kz-Tabelle, sondern direkt ueber den
# Store-Snapshot (s. `_leite_steuernummer_ab()`).
_ABSENDER_HERKUNFT: dict[str, list[tuple[str, str]]] = {
    "absender_name": [("E0100201", "Nachname"), ("E0100301", "Vorname")],
    "absender_strasse": [("E0101104", "Straße"), ("E0101206", "Hausnummer")],
    "absender_plz": [("E0100601", "PLZ")],
    "absender_ort": [("E0100602", "Wohnort")],
}
_ABSENDER_STRASSE_ZUSATZ_KZ = "E0101207"   # optional, wird an die Hausnummer angehaengt

# StNr-Format laut amtlichem Schema: SteuernummerBaseCType, E10-2025.xsd:1779-1786 --
# ([0-9]{4})0[0-9]{8}: 4-stellige Finanzamtsnummer, Ziffer "0", 8 weitere Ziffern (13 Ziffern
# gesamt). Fuehrende Nullen sind bedeutungstragend -- deshalb typ=text, nicht int/cent.
_STNR_PATTERN = re.compile(r"^[0-9]{4}0[0-9]{8}$")


def _leite_absender_ab(deklaration: dict) -> dict[str, str | None]:
    """AbsName/AbsStr/AbsPlz/AbsOrt aus den Stammdaten-Kz der Deklaration (Person A) ableiten.

    Fehlt eines der zugrunde liegenden Kz eines Feldes, bleibt das Feld None -- nie eine
    erratene Ersatzangabe. Format gemessen (reports/adjudikation/absender_ableitung_2026-08-10.md):
    checkESt prueft AbsStr NICHT auf Trennzeichen/Hausnummer/Zusatz-Format (XSD-Typ ist ein
    freier String_MinL1_MaxL30 ohne Muster) -- "Straße Hausnummer[Zusatz]" (Leerzeichen vor
    der Nummer, Zusatz direkt angehaengt) ist eine unauffaellige, aber nicht die einzig
    moegliche Wahl.
    """
    abgeleitet: dict[str, str | None] = {}
    for feld, quellen in _ABSENDER_HERKUNFT.items():
        teile = [deklaration.get(kz) for kz, _ in quellen]
        abgeleitet[feld] = " ".join(str(t) for t in teile) if all(teile) else None
    if abgeleitet["absender_strasse"] is not None:
        zusatz = deklaration.get(_ABSENDER_STRASSE_ZUSATZ_KZ)
        if zusatz:
            abgeleitet["absender_strasse"] += str(zusatz)
    return abgeleitet


def _leite_steuernummer_ab(snapshot: dict) -> str | None:
    """absender_steuernummer aus dem Store-Snapshot ableiten (`stammdaten_steuernummer`).

    Kein Kz-Spiegel moeglich (Vorsatz traegt kein Kz) -- die Quelle ist deshalb nicht
    `deklaration` (Kz -> Wert, wie bei `_leite_absender_ab()`), sondern der materialisierte
    Snapshot selbst ({feld_id -> {wert, zustand, herkunft}}, dieselbe Ebene, die auch
    `est_mapping.deklariere()` als erstes Argument nimmt). Nur ein BESTAETIGTER Wert zaehlt
    (Zwei-Signal) -- nie eine erratene Ersatzangabe.
    """
    feld = snapshot.get("stammdaten_steuernummer")
    if not feld or feld.get("zustand") != "bestaetigt":
        return None
    wert = feld.get("wert")
    return str(wert) if wert else None


def _vorsatz(vz: int, absender_name: str, absender_strasse: str, absender_plz: str,
            absender_ort: str, absender_steuernummer: str, datenlieferant: str,
            ns_e10: str) -> ET.Element:
    """<Vorsatz>-Block: Vordruck-Verwaltungsdaten. KEIN Kz-Element (kein `E\\d{7}`-Name) —
    `kz_pfade()` indiziert ihn deshalb nie; er muss hier von Hand gebaut werden, genau wie
    `_transfer_header()`. Lebt im E10-Namespace (`ns_e10`), NICHT im Elster-Rahmen-Namespace,
    weil er Kind von `<E10>` ist (s. Referenz-XML, Zeile 108 — kein eigenes Praefix).

    Struktur, Reihenfolge und zulaessige Werte kommen aus dem amtlichen Schema
    (`Vorsatz_67907_CType`, E10-2025.xsd:25286-25360) und der amtlichen Referenz
    (elster/submission/testfall_est2025_minimal.xml, validiert rc=0, Zeilen 108-122) —
    nichts geraten:
      - Unterfallart: XSD-Pattern laesst nur "10" zu (String_MinL2_MaxL2_Muster1567_CType,
        E10-2025.xsd:8079-8087; Doku "bei ESt ist der Wert 10 zu verwenden").
      - Vorgang: Enum_Vorsatz_Vorgang_CType kennt "01" (Veranlagung) / "04" (Veranlagung +
        Festsetzung von Vorauszahlungen), E10-2025.xsd:7466-7479. "01" ist der Normalfall
        (reine Veranlagung, keine Vorauszahlungsfestsetzung) — auch der Referenz-Wert.
      - Zeitraum: = vz, kein neuer Wert (der Parameter existiert schon).
      - Copyright: XSD verlangt nur einen freien String, max. 50 Zeichen ("Copyrightvermerk
        des Herstellers der Steuersoftware", E10-2025.xsd:25343-25346) — NICHT der Referenz-
        Literal "ELSTER" (der bezeichnet den Ersteller der amtlichen Beispieldatei, nicht
        TaxGraph als Hersteller). Hier der bereits vorhandene `datenlieferant`-Parameter
        (Default "TaxGraph"), der denselben Sachverhalt im TransferHeader traegt.
      - OrdNrArt: XSD-Pattern "S|O" (String_MinL1_MaxL1_Muster83686_CType,
        E10-2025.xsd:7974-7982); Referenz nutzt "S" (Ordnung nach Steuernummer).
      - StNr: GEMESSENER Befund (2026-08-09, s. reports/adjudikation/vorsatz_block_2026-08-09.md):
        anfangs weggelassen, weil sie nicht in den 9 Original-Fehlern stand — checkESt akzeptierte
        das nicht klaglos, sondern ersetzte die 9 Fehler durch 2 ANDERE: „Es wurde ... angegeben,
        dass die Steuererklaerung mit einer vorhandenen Steuernummer abgegeben wird [...]" und
        „Die Bundesfinanzamtsnummer und die ersten 4 Stellen der Steuernummer unterscheiden
        sich." OrdNrArt="S" VERLANGT also eine StNr, deren erste 4 Stellen mit der
        Finanzamtsnummer (`empfaenger_finanzamt`) uebereinstimmen — kein Konstante, sondern ein
        weiterer Absender-Parameter, den `erzeuge_xml()` genauso fail-closed erzwingt und dessen
        Praefix es gegen `empfaenger_finanzamt` prueft.
      - Rueckuebermittlung/Bescheid: JaNein12BaseCType, "1"=Ja/"2"=Nein
        (E10-2025.xsd:1634-1649). Referenz nutzt "2" (keine elektronische Bescheid-Abholung
        angefordert) — konservativer Default, bis das Produkt diesen Workflow anbietet.

    AbsName/AbsStr/AbsPlz/AbsOrt/StNr kommen aus dem Fall (Absender = steuerpflichtige Person).
    Es sind PARAMETER dieser Funktion — der Aufrufer (`erzeuge_xml`) leitet alle fuenf im
    Normalfall ab (die ersten vier aus der Deklaration, s. `_leite_absender_ab()`, dieselben
    Angaben stehen bereits als Kz im Hauptvordruck; StNr aus dem Store-Snapshot, s.
    `_leite_steuernummer_ab()` — kein Kz-Spiegel moeglich) und erzwingt fail-closed, wenn
    weder Parameter noch Ableitung vorliegen. Ein expliziter Parameter (z.B. aus Tests) hat
    Vorrang vor der Ableitung.
    """
    v = ET.Element(f"{{{ns_e10}}}Vorsatz")
    ET.SubElement(v, f"{{{ns_e10}}}Unterfallart").text = "10"
    ET.SubElement(v, f"{{{ns_e10}}}Vorgang").text = "01"
    ET.SubElement(v, f"{{{ns_e10}}}StNr").text = absender_steuernummer
    ET.SubElement(v, f"{{{ns_e10}}}Zeitraum").text = str(vz)
    ET.SubElement(v, f"{{{ns_e10}}}AbsName").text = absender_name
    ET.SubElement(v, f"{{{ns_e10}}}AbsStr").text = absender_strasse
    ET.SubElement(v, f"{{{ns_e10}}}AbsPlz").text = absender_plz
    ET.SubElement(v, f"{{{ns_e10}}}AbsOrt").text = absender_ort
    ET.SubElement(v, f"{{{ns_e10}}}Copyright").text = datenlieferant
    ET.SubElement(v, f"{{{ns_e10}}}OrdNrArt").text = "S"
    rueck = ET.SubElement(v, f"{{{ns_e10}}}Rueckuebermittlung")
    ET.SubElement(rueck, f"{{{ns_e10}}}Bescheid").text = "2"
    return v


def _transfer_header(vz: int, empfaenger_land: str, hersteller_id: str,
                     datenlieferant: str, testmerker: str | None) -> ET.Element:
    th = ET.Element(f"{{{NS_ELSTER}}}TransferHeader", {"version": "11"})
    ET.SubElement(th, f"{{{NS_ELSTER}}}Verfahren").text = "ElsterErklaerung"
    ET.SubElement(th, f"{{{NS_ELSTER}}}DatenArt").text = "ESt"
    ET.SubElement(th, f"{{{NS_ELSTER}}}Vorgang").text = "send-Auth"
    if testmerker:
        ET.SubElement(th, f"{{{NS_ELSTER}}}Testmerker").text = testmerker
    empf = ET.SubElement(th, f"{{{NS_ELSTER}}}Empfaenger", {"id": "L"})
    ET.SubElement(empf, f"{{{NS_ELSTER}}}Ziel").text = empfaenger_land
    ET.SubElement(th, f"{{{NS_ELSTER}}}HerstellerID").text = hersteller_id
    ET.SubElement(th, f"{{{NS_ELSTER}}}DatenLieferant").text = datenlieferant
    datei = ET.SubElement(th, f"{{{NS_ELSTER}}}Datei")
    ET.SubElement(datei, f"{{{NS_ELSTER}}}Verschluesselung").text = "CMSEncryptedData"
    ET.SubElement(datei, f"{{{NS_ELSTER}}}Kompression").text = "GZIP"
    ET.SubElement(datei, f"{{{NS_ELSTER}}}TransportSchluessel")
    return th


def erzeuge_xml(result: dict, *, vz: int = 2025, empfaenger_land: str = "BY",
                empfaenger_finanzamt: str = "9181", hersteller_id: str | None = None,
                datenlieferant: str = "TaxGraph", testmerker: str | None = TESTMERKER_ERIC,
                nutzdaten_ticket: str = "taxgraph-0001", abgabefaehig: bool = False,
                absender_name: str | None = None, absender_strasse: str | None = None,
                absender_plz: str | None = None, absender_ort: str | None = None,
                absender_steuernummer: str | None = None,
                snapshot: dict | None = None) -> str:
    """Deklaration (aus est_mapping.deklariere()) -> ELSTER-Submission-XML als String.

    Fail-closed: `result["vollstaendig"] is False` -> XmlFehler. Kz ohne Schema-Pfad -> XmlFehler.

    Verarbeitet die drei Buckets aus est_mapping.deklariere():
    - `deklaration`: {Kz: Wert} — Person A / Basis-Instanzen (Index 0)
    - `person_b`: {Kz: Wert} — Person B (Zusammenveranlagung, Index 1)
    - `anlage_instanzen`: {gruppe: [{index, felder{Kz: Wert}}]} — Instanz 2..N
    - `kind_anlagen`: Zählliste, dient als Konsistenz-Wächter

    Äußere Schleife über Kz in Schema-Reihenfolge, innere über Instanzen — so bleiben
    N(PersonA) und N(PersonB) direkt benachbart, VOR dem VOR-Container.

    `abgabefaehig=True` haengt zusaetzlich den <Vorsatz>-Block an (s. `_vorsatz()`) — ohne ihn
    weist checkESt jedes Produkt-XML mit 9 zusaetzlichen Vorsatz-Plausibilitaetsfehlern ab
    (gemessen 2026-08-09 an ZWEI Faellen, Einzel- UND Zusammenveranlagung: dieselben 9
    Vorsatz-Fehler in beiden, unabhaengig von den uebrigen, fallabhaengigen Fehlern —
    reports/adjudikation/vorsatz_block_2026-08-09.md). Default bleibt False: Vorsatz ist KEIN
    Kz-Element, kann also nie ueber `deklaration` befuellt werden.

    `absender_name/_strasse/_plz/_ort` sind deshalb eigene Parameter — werden aber, wenn nicht
    explizit gesetzt, aus `deklaration` abgeleitet (s. `_leite_absender_ab()`): dieselben
    Angaben stehen ohnehin als Kz im Hauptvordruck ESt 1 A, eine zweite Repraesentation ueber
    Pflicht-Parameter waere eine Naht mit zwei Wahrheiten. `absender_steuernummer` hat keinen
    Kz-Spiegel (der Vorsatz-Block traegt kein Kz-Element) — sie wird deshalb, wenn nicht
    explizit gesetzt, aus dem optionalen `snapshot`-Parameter abgeleitet (s.
    `_leite_steuernummer_ab()`; `snapshot` ist dieselbe {feld_id -> {wert, zustand}}-Ebene, die
    auch `est_mapping.deklariere()` als erstes Argument nimmt — der Aufrufer haelt sie ohnehin
    schon, wenn er `result` gebaut hat). Ohne `snapshot` UND ohne expliziten Parameter bleibt
    `absender_steuernummer` unabgeleitet. Mit `abgabefaehig=True` und weder Parameter noch
    Ableitung fuer eines der fuenf Absender-Felder bricht dieser Aufruf fail-closed ab — die
    Meldung nennt das fehlende Kz bzw. Feld, nicht nur den Parameternamen —, statt ein XML zu
    erzeugen, das checkESt spaeter still ablehnt. Die Steuernummer muss zusaetzlich mit der
    Finanzamtsnummer des Empfaengers beginnen und dem amtlichen StNr-Muster entsprechen
    (`_STNR_PATTERN`, gemessen — s. dort); beides gilt fuer Parameter UND Ableitung gleich,
    die Pruefung sitzt nach der Ableitung, nicht davor.
    """
    if not result.get("vollstaendig", False):
        offen = result.get("unvollstaendig", [])
        raise XmlFehler(f"Deklaration unvollständig ({len(offen)} offene Pflichtfelder) — "
                        f"kein Submission-XML. Erste: {offen[:3]}")
    deklaration = result.get("deklaration") or {}
    if not deklaration:
        raise XmlFehler("leere Deklaration — nichts zu übermitteln.")
    hid = hersteller_id or os.environ.get("ELSTER_HERSTELLER_ID", "")
    if not hid:
        raise XmlFehler("keine Hersteller-ID — $ELSTER_HERSTELLER_ID setzen "
                        "(nie im Repo, nie im Code).")

    pfade = kz_pfade(vz)
    # Benannte Ausschlüsse (E10_AUSSCHLUSS_DATENART, z.B. E60xx->E77/EÜR) sind dokumentiert,
    # kein stilles Weglassen — sie dürfen einen anderen Datenart-Kz nicht als E10-Fehler crashen.
    # Konsistent mit dem anlage_instanzen-Pfad unten. Fehlt ein Kz hier UND im Schema UND im
    # Ausschluss -> fail-closed (nie stilles Weglassen).
    fehlend = sorted(kz for kz in deklaration
                     if kz not in pfade and kz not in E10_AUSSCHLUSS_DATENART)
    if fehlend:
        raise XmlFehler(f"{len(fehlend)} Kz ohne Pfad im E10-{vz}-Schema: {fehlend[:5]}")

    # Extrahiere die drei Buckets
    person_b = result.get("person_b") or {}
    anlage_instanzen = result.get("anlage_instanzen") or {}
    kind_anlagen = result.get("kind_anlagen") or []

    # kind_anlagen-Konsistenzprüfung (fail-closed, nur kind-spezifisch)
    if kind_anlagen:
        kind_instanzen = anlage_instanzen.get("kind", [])
        n_behauptet = len(kind_anlagen)
        n_vorhanden = 1 + len(kind_instanzen)
        if n_behauptet != n_vorhanden:
            raise XmlFehler(
                f"Kinderzahl behauptet {n_behauptet}, Kind-Daten für {n_vorhanden} vorhanden — "
                f"kind_anlagen vs anlage_instanzen[kind] inkonsistent.")

    # Vorbereitung: generische Instanz-Map über ALLE Gruppen
    # instanz_map[kz] = [(container_pfad, idx_0, wert), ...]
    # Container-Pfad wird aus dem Kz-Pfad abgeleitet: prefix bis E10-Direktkind.
    # Kz, die nicht im E10-Schema liegen (z.B. E60xx->E77), sind EXPLIZIT in
    # E10_AUSSCHLUSS_DATENART benannt — sonst fail-closed (nie stilles Weglassen).
    instanz_map: dict[str, list[tuple[tuple, int, object]]] = {}
    for gruppe, instanzen in anlage_instanzen.items():
        for inst in instanzen:
            k_idx = inst["index"]
            inst_idx_0 = k_idx - 1
            for kz, wert in inst.get("felder", {}).items():
                if kz not in pfade:
                    grund = E10_AUSSCHLUSS_DATENART.get(kz)
                    if not grund:
                        raise XmlFehler(
                            f"Gruppe '{gruppe}': Kz {kz} nicht im E10-{vz}-Schema — "
                            f"kann nicht deklariert werden (weder E10-Kz noch "
                            f"expliziter Ausschluss in E10_AUSSCHLUSS_DATENART).")
                    # Expliziter Ausschluss (andere Datenart, z.B. E77/EÜR): Instanz-Wert
                    # kann nicht ins ESt-XML — das ist dokumentiert, kein stilles Weglassen.
                    continue
                kz_path = pfade[kz]
                if len(kz_path) < 3:
                    raise XmlFehler(
                        f"Gruppe '{gruppe}': Kz {kz} Pfad {kz_path} zu kurz — "
                        f"kein E10-Container ableitbar.")
                container = kz_path[:2]   # ("E10", "<Direktkind>")
                instanz_map.setdefault(kz, []).append((container, inst_idx_0, wert))

    ns_e10 = NS_E10.format(vz=vz)
    ET.register_namespace("", NS_ELSTER)

    wurzel = ET.Element(f"{{{NS_ELSTER}}}Elster")
    wurzel.append(_transfer_header(vz, empfaenger_land, hid, datenlieferant, testmerker))

    datenteil = ET.SubElement(wurzel, f"{{{NS_ELSTER}}}DatenTeil")
    block = ET.SubElement(datenteil, f"{{{NS_ELSTER}}}Nutzdatenblock")
    nh = ET.SubElement(block, f"{{{NS_ELSTER}}}NutzdatenHeader", {"version": "11"})
    ET.SubElement(nh, f"{{{NS_ELSTER}}}NutzdatenTicket").text = nutzdaten_ticket
    ET.SubElement(nh, f"{{{NS_ELSTER}}}Empfaenger", {"id": "F"}).text = empfaenger_finanzamt
    nutzdaten = ET.SubElement(block, f"{{{NS_ELSTER}}}Nutzdaten")

    e10 = ET.SubElement(nutzdaten, f"{{{ns_e10}}}E10", {"version": str(vz)})
    pflicht = pflicht_kinder(vz)
    schema_pfad = XV._find_schema(vz)
    kz_meta = XV._resolve_kz_meta(schema_pfad) if schema_pfad else {}

    # Vorbereitung: Person-Container-Bestimmung fuer person_b
    person_container_cache: dict[str, tuple | None] = {}

    def _person_container(kz: str) -> tuple | None:
        if kz not in person_container_cache:
            p = pfade.get(kz)
            person_container_cache[kz] = _bestimme_person_container(p, pflicht) if p else None
        return person_container_cache[kz]

    # Prüfe person_b-Kz auf Schema-Pfade
    fehlend_b = sorted(kz for kz in person_b if kz not in pfade)
    if fehlend_b:
        raise XmlFehler(f"{len(fehlend_b)} Kz in person_b ohne Pfad: {fehlend_b[:5]}")

    # Äußere Schleife über Kz in Schema-Reihenfolge, innere über Instanzen
    for kz in pfade:
        # Person A (Haupt-Deklaration, Index 0)
        if kz in deklaration:
            _einhaengen(e10, pfade[kz], deklaration[kz], ns_e10, pflicht, kz_meta=kz_meta)

        # Person B (Instanz-Achse Index 1, nur Person-Container)
        if kz in person_b:
            cp = _person_container(kz)
            if cp is None:
                raise XmlFehler(
                    f"person_b-Kz {kz} liegt in Container ohne Person-Diskriminator "
                    f"(Pfad: {'/'.join(pfade[kz])}) — kann nicht als Instanz 1 (PersonB) geschrieben werden.")
            _einhaengen(e10, pfade[kz], person_b[kz], ns_e10, pflicht,
                        instanz={cp: 1}, kz_meta=kz_meta)

        # Anlage-Instanzen 2..N (Gruppen-kind, gwg, vv_objekt, rente, ...)
        for container, inst_idx_0, wert in instanz_map.get(kz, []):
            _einhaengen(e10, pfade[kz], wert, ns_e10, pflicht,
                        instanz={container: inst_idx_0}, kz_meta=kz_meta)

    if abgabefaehig:
        # Ableitung ist der Normalfall (dieselben Angaben stehen als Kz im Hauptvordruck bzw.
        # — bei der Steuernummer, die kein Kz hat — im Store-Snapshot) — ein expliziter
        # Parameter (z.B. aus Tests) hat Vorrang und wird nie überschrieben.
        abgeleitet = _leite_absender_ab(deklaration)
        absender_name = absender_name or abgeleitet["absender_name"]
        absender_strasse = absender_strasse or abgeleitet["absender_strasse"]
        absender_plz = absender_plz or abgeleitet["absender_plz"]
        absender_ort = absender_ort or abgeleitet["absender_ort"]
        if not absender_steuernummer and snapshot is not None:
            absender_steuernummer = _leite_steuernummer_ab(snapshot)

        fehlend_absender = []
        for param_name, wert in (
                ("absender_name", absender_name), ("absender_strasse", absender_strasse),
                ("absender_plz", absender_plz), ("absender_ort", absender_ort)):
            if wert:
                continue
            fehlende_kz = ", ".join(f"{kz} ({label})" for kz, label in _ABSENDER_HERKUNFT[param_name]
                                    if not deklaration.get(kz))
            fehlend_absender.append(f"{param_name} (fehlendes Kz: {fehlende_kz})")
        if not absender_steuernummer:
            fehlend_absender.append(
                "absender_steuernummer (kein Kz-Spiegel — weder als Parameter übergeben noch "
                "aus snapshot['stammdaten_steuernummer'] ableitbar, s. _leite_steuernummer_ab())")
        if fehlend_absender:
            raise XmlFehler(
                f"abgabefaehig=True verlangt Absender-Stammdaten, es fehlen:\n" +
                "\n".join(f"  - {f}" for f in fehlend_absender) +
                f"\nsonst lehnt checkESt das XML wegen fehlender Vorsatz-Pflichtfelder ab "
                f"(still fuer den Nutzer; gemessen 2026-08-09: 9 Vorsatz-Fehlermeldungen, "
                f"gleich bei Einzel- und Zusammenveranlagung — die uebrigen, fallabhaengigen "
                f"Fehler bleiben davon unberuehrt).")
        # Format zuerst (amtliches Muster, s. _STNR_PATTERN), dann die Finanzamts-Kopplung —
        # eine Steuernummer mit falscher Form kann sonst zufaellig mit dem richtigen Praefix
        # beginnen und den Kopplungs-Check bestehen, obwohl sie kein gueltiges StNr-XML-Element
        # waere (PII bleibt aus der Fehlermeldung: nur ob das Muster passt, nie der Wert).
        if not _STNR_PATTERN.match(absender_steuernummer):
            raise XmlFehler(
                "absender_steuernummer entspricht nicht dem amtlichen Muster "
                "([0-9]{4}0[0-9]{8}, SteuernummerBaseCType, E10-2025.xsd:1779-1786) — "
                "13 Ziffern, 5. Ziffer '0'. Wert nicht geloggt.")
        # Gemessener Befund (reports/adjudikation/vorsatz_block_2026-08-09.md): OrdNrArt="S"
        # ohne konsistente StNr ersetzt die urspruenglichen Fehler NICHT durch Null, sondern
        # durch 2 ANDERE ("Bundesfinanzamtsnummer ... unterscheiden sich"). Praefix-Check hier
        # schlaegt beim Bau fehl statt erst bei checkESt / beim Finanzamt.
        if absender_steuernummer[:4] != empfaenger_finanzamt:
            raise XmlFehler(
                f"absender_steuernummer beginnt nicht mit der Finanzamtsnummer "
                f"{empfaenger_finanzamt!r} (Praefix {absender_steuernummer[:4]!r}) — "
                f"checkESt lehnt das als 'Bundesfinanzamtsnummer ... unterscheiden sich' ab.")
        # Vorsatz ist laut Schema (E10-2025.xsd:8403) das LETZTE Kind von E10 — Anhaengen
        # NACH der Kz-Schleife ist deshalb schema-korrekt, unabhaengig davon, welche
        # Kz-Container zuvor entstanden sind.
        e10.append(_vorsatz(vz, absender_name, absender_strasse, absender_plz, absender_ort,
                            absender_steuernummer, datenlieferant, ns_e10))

    ET.indent(wurzel, space="\t")
    return '<?xml version="1.0" encoding="UTF-8"?>\n' + _serialisiere(wurzel, ns_e10)


def _serialisiere(wurzel: ET.Element, ns_e10: str) -> str:
    """Serialisiert so, dass ERiC den Rahmen annimmt: E10-Namespace LOKAL am <E10>.

    ElementTree kennt nur EINEN Default-Namespace. Da `register_namespace("", NS_ELSTER)`
    den Elster-Rahmen belegt, bekommt der E10-Teilbaum ein generiertes Praefix (ns0/ns1/...)
    und dessen Deklaration wandert an den <Elster>-Root. ERiC lehnt das ab — gemessen am
    2026-08-09 gegen ERiC 44.2.4.0, ESt_2025:

        rc=610301200 ERIC_IO_READER_SCHEMA_VALIDIERUNGSFEHLER
        eric.log: EDS-XML in Zeile 2 in Spalte 130: 'Der XML-Datensatz enthaelt an einer
        nicht erlaubten Stelle eine Namespace-Praefix-Definition: xmlns:ns1'

    Das amtliche Referenz-XML (elster/submission/testfall_est2025_minimal.xml, rc=0)
    deklariert den E10-Namespace stattdessen als Default direkt am <E10>. Genau das wird
    hier hergestellt. Das XSD faengt den Unterschied NICHT — beide Fassungen validieren
    gegen elster11_E10_2025_extern.xsd; nur checkESt sieht ihn.
    """
    roh = ET.tostring(wurzel, encoding="unicode")
    m = re.search(rf'\s+xmlns:([A-Za-z0-9_.-]+)="{re.escape(ns_e10)}"', roh)
    if m is None:
        return roh                      # bereits praefixfrei serialisiert
    praefix = m.group(1)
    roh = roh[:m.start()] + roh[m.end():]
    roh, n = re.subn(rf"<{re.escape(praefix)}:E10(\s|>)",
                     lambda mm: f'<E10 xmlns="{ns_e10}"{mm.group(1)}', roh, count=1)
    if n != 1:
        raise XmlFehler(
            f"E10-Wurzel mit Praefix '{praefix}' nicht gefunden — Serialisierung "
            f"haette eine unerlaubte Praefix-Deklaration am Elster-Root hinterlassen.")
    roh = re.sub(rf"</?{re.escape(praefix)}:", lambda mm: mm.group(0).replace(praefix + ":", ""), roh)
    return roh


def schreibe_xml(result: dict, ziel: str, **kw) -> str:
    """erzeuge_xml() + atomar nach `ziel` schreiben. Gibt den Pfad zurück."""
    xml = erzeuge_xml(result, **kw)
    tmp = ziel + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        f.write(xml)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, ziel)
    return ziel
