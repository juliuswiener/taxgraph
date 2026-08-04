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
                instanz: dict[tuple, int] | None = None) -> None:
    """Legt den Pfad unterhalb `wurzel` an (idempotent) und setzt das Blatt auf `wert`.

    `pfad` beginnt mit dem Namen der Wurzel selbst (z.B. ('E10','ESt1A','Art_Erkl','E0100001'));
    das erste Segment wird übersprungen, weil `wurzel` genau dieses Element ist. Beim erstmaligen
    Anlegen eines Containers werden dessen skalare Pflichtfelder direkt mitgeschrieben — sie
    stehen im Schema vor den Kz-Kindern, müssen also VOR dem Weiterlaufen gesetzt werden.

    Mit `instanz`: ein Dict {Container-Pfad: Index}, das die zu verwendende Instanz-Nummer
    (0-basiert) für wiederholbare Container angibt — statt `knoten.find(tag)` wird das n-te
    Vorkommen gesucht und ggf. angelegt. Person-Diskriminatoren werden je nach Index gesetzt:
    Index 0 = PersonA, Index >= 1 = PersonB.
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
    blatt = ET.SubElement(knoten, f"{{{ns}}}{pfad[-1]}")
    blatt.text = _wert_text(wert)


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
                nutzdaten_ticket: str = "taxgraph-0001") -> str:
    """Deklaration (aus est_mapping.deklariere()) -> ELSTER-Submission-XML als String.

    Fail-closed: `result["vollstaendig"] is False` -> XmlFehler. Kz ohne Schema-Pfad -> XmlFehler.

    Verarbeitet die drei Buckets aus est_mapping.deklariere():
    - `deklaration`: {Kz: Wert} — Person A / Basis-Instanzen (Index 0)
    - `person_b`: {Kz: Wert} — Person B (Zusammenveranlagung, Index 1)
    - `anlage_instanzen`: {gruppe: [{index, felder{Kz: Wert}}]} — Instanz 2..N
    - `kind_anlagen`: Zählliste, dient als Konsistenz-Wächter

    Äußere Schleife über Kz in Schema-Reihenfolge, innere über Instanzen — so bleiben
    N(PersonA) und N(PersonB) direkt benachbart, VOR dem VOR-Container.
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
    fehlend = sorted(kz for kz in deklaration if kz not in pfade)
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
            _einhaengen(e10, pfade[kz], deklaration[kz], ns_e10, pflicht)

        # Person B (Instanz-Achse Index 1, nur Person-Container)
        if kz in person_b:
            cp = _person_container(kz)
            if cp is None:
                raise XmlFehler(
                    f"person_b-Kz {kz} liegt in Container ohne Person-Diskriminator "
                    f"(Pfad: {'/'.join(pfade[kz])}) — kann nicht als Instanz 1 (PersonB) geschrieben werden.")
            _einhaengen(e10, pfade[kz], person_b[kz], ns_e10, pflicht,
                        instanz={cp: 1})

        # Anlage-Instanzen 2..N (Gruppen-kind, gwg, vv_objekt, rente, ...)
        for container, inst_idx_0, wert in instanz_map.get(kz, []):
            _einhaengen(e10, pfade[kz], wert, ns_e10, pflicht,
                        instanz={container: inst_idx_0})

    ET.indent(wurzel, space="\t")
    return '<?xml version="1.0" encoding="UTF-8"?>\n' + ET.tostring(wurzel, encoding="unicode")


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
