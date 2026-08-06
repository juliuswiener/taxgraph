"""Deterministischer XBRL-Linkbase-Parser fuer die E-Bilanz-Kerntaxonomie
(de-gaap-ci, § 5b EStG). Liest ausschliesslich die committeten Freeze-Dateien
unter sources/ebilanz/<v>/xbrl/:

- reference-fiscal.xml : Mussfeld-Klassifikation (hgbref:fiscalRequirement) +
  Rechtsform-Geltung (hgbref:legalFormEU/PG/KSt) je Concept.
- label-de.xml         : deutsche Standard-Bezeichner (role/label) je Concept.
- <taxonomie>.xsd      : die Menge aller benannten Concepts (Existenz-Nachweis).
- presentation-*.xml   : Baum-Elternschaft (presentationArc), fuer Pfad-Kontext.

Reine stdlib (xml.etree), kein LLM, deterministisch. Die Werte werden NICHT aus
den Excel-Visualisierungen oder meta-Notizen uebernommen (unverbindlich) - allein
die XBRL-Linkbase ist amtlich verbindlich."""

from __future__ import annotations

import xml.etree.ElementTree as ET

LINK = "http://www.xbrl.org/2003/linkbase"
XLINK = "http://www.w3.org/1999/xlink"
ROLE_STD_LABEL = "http://www.xbrl.org/2003/role/label"

# fiscalRequirement-Kategorien mit Muss-Charakter (§ 5b): Mussfeld + rechnerische
# Oberpositionen (Summenmussfeld) + Pflichtfeld-mit-Konten-Wunsch. "Rechnerisch
# notwendig, soweit vorhanden" ist KEIN Muss (bedingt).
MUSS_KATEGORIEN = (
    "Mussfeld",
    "Summenmussfeld",
    "Mussfeld, Kontennachweis erwünscht",
)


def _q(ns: str, tag: str) -> str:
    return f"{{{ns}}}{tag}"


def _concept_from_href(href: str, prefix: str) -> str:
    frag = href.split("#", 1)[1]
    return frag[len(prefix):] if frag.startswith(prefix) else frag


def parse_reference_fiscal(path: str, prefix: str = "de-gaap-ci_") -> dict:
    """concept -> {fiscalRequirement, legalFormEU, legalFormPG, legalFormKSt}.

    Aufloesung strikt ueber loc/arc/reference (nicht ueber die Namens-Konvention):
    loc-Label -> Concept (aus href), Arc from->to, reference-Resource -> hgbref-Felder.
    """
    root = ET.parse(path).getroot()
    loc: dict = {}
    arcs: list = []
    refs: dict = {}
    for rl in root.iter(_q(LINK, "referenceLink")):
        for el in rl:
            tag = el.tag.split("}")[-1]
            if tag == "loc":
                loc[el.get(_q(XLINK, "label"))] = _concept_from_href(
                    el.get(_q(XLINK, "href")), prefix)
            elif tag == "referenceArc":
                arcs.append((el.get(_q(XLINK, "from")), el.get(_q(XLINK, "to"))))
            elif tag == "reference":
                d = {}
                for c in el:
                    d[c.tag.split("}")[-1]] = (c.text or "").strip()
                refs[el.get(_q(XLINK, "label"))] = d
    out: dict = {}
    for frm, to in arcs:
        concept = loc.get(frm)
        if concept is None:
            continue
        f = refs.get(to, {})
        out[concept] = {
            "fiscalRequirement": f.get("fiscalRequirement") or None,
            "legalFormEU": f.get("legalFormEU") == "true",
            "legalFormPG": f.get("legalFormPG") == "true",
            "legalFormKSt": f.get("legalFormKSt") == "true",
        }
    return out


def parse_labels(path: str, prefix: str = "de-gaap-ci_",
                 role: str = ROLE_STD_LABEL) -> dict:
    """concept -> deutscher Standard-Label (erste role/label-Zuordnung)."""
    root = ET.parse(path).getroot()
    labels: dict = {}
    arcs: list = []
    for el in root.iter():
        tag = el.tag.split("}")[-1]
        if tag == "label" and el.get(_q(XLINK, "role")) == role:
            labels[el.get(_q(XLINK, "label"))] = (el.text or "").strip()
        elif tag == "labelArc":
            arcs.append((el.get(_q(XLINK, "from")), el.get(_q(XLINK, "to"))))
    out: dict = {}
    for frm, to in arcs:
        if to in labels:
            concept = frm[len(prefix):] if frm.startswith(prefix) else frm
            out.setdefault(concept, labels[to])
    return out


def parse_concepts_xsd(path: str) -> set:
    """Menge aller benannten Concepts (xsd:element name=...)."""
    root = ET.parse(path).getroot()
    names = set()
    for el in root.iter():
        if el.tag.split("}")[-1] == "element":
            n = el.get("name")
            if n:
                names.add(n)
    return names


def categorize(refmap: dict) -> dict:
    """fiscalRequirement-Kategorie -> Menge der Concepts."""
    cat: dict = {}
    for concept, f in refmap.items():
        fr = f["fiscalRequirement"]
        if fr:
            cat.setdefault(fr, set()).add(concept)
    return cat


def muss_weit(cat: dict) -> set:
    """Mussfeld + Summenmussfeld + Kontennachweis-erwuenscht."""
    s: set = set()
    for k in MUSS_KATEGORIEN:
        s |= cat.get(k, set())
    return s


def muss_eng(cat: dict) -> set:
    """Ohne Summenmussfeld (= Instructor-''514/525''-Definition)."""
    return cat.get("Mussfeld", set()) | cat.get("Mussfeld, Kontennachweis erwünscht", set())


def w2_nenner(refmap: dict, concepts: set) -> set:
    """Muss-Concepts, die fuer legalFormEU (Einzelunternehmer) ODER legalFormPG
    (Personengesellschaft) gelten - der W2-Nenner (natuerliche Person/PersG).
    KSt-only-Positionen fallen heraus."""
    return {c for c in concepts
            if refmap.get(c, {}).get("legalFormEU") or refmap.get(c, {}).get("legalFormPG")}
