"""XSD-Kz-Verifikationspass (Task #11, Design reports/review/2026-07-20-xsd-kz-verifikationspass-design.md).

READ-ONLY-Report-Baustein: prüft je bindung-Feld mit `elster_kz != null`, ob die Kz im lokalen
ERiC-E10-Schema existiert + an welchem Sektionspfad. Top-Down-Walk über LOKALE xs:element-Namen
(kein Typ-Namen-Reverse-Lookup, Soundness-Begründung s. Design §1) — NIE Auto-Edit an Schema/bindung.

H1 (Gate-tauglich): `main()`/`pruefe_bindung(...)["exit_code"]` liefert 0 nur wenn ALLE geprüften
Felder OK sind, 1 sobald ein Feld AMBIGUOUS/NOT_FOUND/SCHEMA_UNVERFUEGBAR ist oder ein
MAX_DEPTH-Abbruch vorkam (H3).
"""
from __future__ import annotations

import glob
import os
import re
import sys
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
ROOT = os.path.dirname(PRODUKT)
sys.path.insert(0, os.path.join(PRODUKT, "traverser"))
sys.path.insert(0, HERE)

from traverser import lade_bindung  # noqa: E402
import est_mapping  # noqa: E402

XS = "{http://www.w3.org/2001/XMLSchema}"
MAX_DEPTH = 80  # Backstop; im echten E10-Schema nie erreicht (max. Pfadtiefe 8, Design §4)
_KZ_RE = re.compile(r"^E\d{7}$")

STATUS_OK = "OK"
STATUS_AMBIGUOUS = "AMBIGUOUS"
STATUS_NOT_FOUND = "NOT_FOUND"
STATUS_UNVERFUEGBAR = "SCHEMA_UNVERFUEGBAR"


# ---------------------------------------------------------------- Schema-Lookup

def _find_schema(jahr: int, schema_name_pattern: str = "E10-{jahr}.xsd") -> str | None:
    """Lokaler Schema-Pfad via $ERIC_DIR / ~/02_Software/eric, oder None (kein lokales Schema)."""
    name = schema_name_pattern.format(jahr=jahr)
    for r in (os.environ.get("ERIC_DIR", ""), os.path.expanduser("~/02_Software/eric")):
        if not r:
            continue
        hits = sorted(glob.glob(os.path.join(os.path.expanduser(r), "**", name), recursive=True))
        if hits:
            return hits[0]
    return None


def _resolve_schema_pfad(jahr: int, overrides: dict[int, str | None] | None,
                          schema_name_pattern: str) -> str | None:
    if overrides is not None and jahr in overrides:
        return overrides[jahr]
    return _find_schema(jahr, schema_name_pattern)


# ---------------------------------------------------------------- Datenart-Routing (Task #8)
# elster_kz-Präfix -> Ziel-Datenart. E10 (ESt-Erklaerung) ist der Default; E60xx-Präfix routet
# auf E77 (Anlage EÜR, eigene Datenart) — dieselbe Konvention, nach der ERiC selbst die Submission
# routet (s. elster_kz_grund in bindung_an_gesamt.yaml/bindung_n_vor_gwg.yaml). Root-Element E77,
# self-contained (0 xs:include/xs:import, Jahre 2021-2025), empirisch analog zu E10 geprüft.
_DATENART_DEFAULT = ("E10-{jahr}.xsd", "E10")
_DATENART_ROUTING = {
    "60": ("E77-{jahr}.xsd", "E77"),
}


def _datenart_fuer_kz(kz: str) -> tuple[str, str]:
    """Kz -> (schema_name_pattern, start_element) nach Kz-Präfix (2 Ziffern nach 'E')."""
    return _DATENART_ROUTING.get(kz[1:3], _DATENART_DEFAULT)


# ---------------------------------------------------------------- XSD-Parsing (mit defensiver
# xs:include/xs:import-Auflösung, Design §3 Punkt 4 — im aktuellen E10-Datenbestand toter Code)

def _parse_top_level_children(schema_pfad: str, _seen: set | None = None) -> list:
    seen = _seen if _seen is not None else set()
    schema_pfad = os.path.abspath(schema_pfad)
    if schema_pfad in seen:
        return []
    seen.add(schema_pfad)
    root = ET.parse(schema_pfad).getroot()
    children = list(root)
    for child in root:
        if child.tag in (XS + "include", XS + "import"):
            loc = child.get("schemaLocation")
            if loc:
                ref_pfad = os.path.join(os.path.dirname(schema_pfad), loc)
                if os.path.exists(ref_pfad):
                    children.extend(_parse_top_level_children(ref_pfad, seen))
    return children


def _load_indices(top_level_children: list) -> tuple[dict, dict, dict]:
    type_index, group_index, element_index = {}, {}, {}
    for child in top_level_children:
        name = child.get("name")
        if not name:
            continue
        if child.tag == XS + "complexType":
            type_index[name] = child
        elif child.tag == XS + "group":
            group_index[name] = child
        elif child.tag == XS + "element":
            element_index[name] = child
    return type_index, group_index, element_index


def _flatten_content(container) -> list:
    """Rekursiv durch xs:sequence/xs:choice/xs:all (Struktur-Wrapper, keine eigene Pfad-Ebene)."""
    out = []
    for child in container:
        if child.tag in (XS + "sequence", XS + "choice", XS + "all"):
            out.extend(_flatten_content(child))
        elif child.tag in (XS + "element", XS + "group"):
            out.append(child)
    return out


def _content_of(elem_node, type_index: dict):
    """Inline complexType-Kind ODER referenzierter benannter Typ. None = Blatt (primitiver Typ)."""
    for child in elem_node:
        if child.tag == XS + "complexType":
            return child
    typ = elem_node.get("type")
    if typ and typ in type_index:
        return type_index[typ]
    return None


def walk(schema_pfad: str, start_element: str = "E10") -> tuple[dict[str, list[tuple]], int]:
    """Top-Down-Walk ab `start_element`. Rückgabe: (kz -> Liste[Pfad-Tupel], max_depth_hits).

    Reine Walker-Primitive (kein Report-Konsument) — jede Kz-Fundstelle einzeln, keine Dedup.
    """
    top_level = _parse_top_level_children(schema_pfad)
    type_index, group_index, element_index = _load_indices(top_level)
    if start_element not in element_index:
        raise ValueError(f"Start-Element '{start_element}' nicht im Schema {schema_pfad} gefunden")

    kz_index: dict[str, list[tuple]] = {}
    max_depth_hits = 0

    def recurse(elem_node, path: tuple, depth: int) -> None:
        nonlocal max_depth_hits
        if depth > MAX_DEPTH:
            max_depth_hits += 1
            return
        local_name = elem_node.get("name") or elem_node.get("ref")
        new_path = path + (local_name,)
        if _KZ_RE.match(local_name):
            kz_index.setdefault(local_name, []).append(new_path)

        content = _content_of(elem_node, type_index)
        if content is None:
            return
        for child in _flatten_content(content):
            if child.tag == XS + "element":
                ref = child.get("ref")
                if ref:
                    target = element_index.get(ref)
                    if target is not None:
                        recurse(target, new_path, depth + 1)
                else:
                    recurse(child, new_path, depth + 1)
            elif child.tag == XS + "group":
                grp = group_index.get(child.get("ref"))
                if grp is not None:
                    for gc in _flatten_content(grp):
                        if gc.tag == XS + "element":
                            recurse(gc, new_path, depth + 1)

    recurse(element_index[start_element], (), 0)
    return kz_index, max_depth_hits


# ---------------------------------------------------------------- Report-Treiber (Konsument, H1/H2/H3)

def _rollup(jahre_ergebnis: dict[int, dict]) -> str:
    stati = {v["status"] for v in jahre_ergebnis.values()}
    if STATUS_AMBIGUOUS in stati:
        return STATUS_AMBIGUOUS
    if STATUS_NOT_FOUND in stati:
        return STATUS_NOT_FOUND
    if STATUS_OK in stati:
        return STATUS_OK
    return STATUS_UNVERFUEGBAR  # alle geprüften Jahre ohne lokales Schema


def pruefe_bindung(bindung: dict, schema_pfade: dict[int, str | None] | None = None,
                    start_element: str | None = None) -> dict:
    """Je bindung-Feld mit elster_kz != null: OK/AMBIGUOUS/NOT_FOUND/SCHEMA_UNVERFUEGBAR je vz_gueltigkeit-Jahr
    + Feld-Rollup. `schema_pfade` überschreibt die Jahr->Pfad-Auflösung (Test-Injektion echter/synthetischer
    Schemata); fehlt ein Jahr im Override, wird real via `$ERIC_DIR`/`~/02_Software/eric` gesucht.

    `start_element` explizit gesetzt (Tests, synthetische Schemata) gilt für ALLE Felder unverändert
    (Backward-Compat). `start_element=None` (Produktion, `main()`) aktiviert Pro-Feld-Datenart-Routing
    per Kz-Präfix (`_datenart_fuer_kz`, Task #8) — E60xx-Felder laufen gegen E77 statt E10.

    Rückgabe: {"felder": {feld_id: {"status", "jahre": {jahr: {"status","pfade"}}}},
               "unverfuegbare_jahre": sorted[...], "max_depth_hits_gesamt": int, "exit_code": 0|1}
    """
    schema_cache: dict[tuple[str, str], tuple[dict, int]] = {}
    felder: dict[str, dict] = {}
    unverfuegbare_jahre: set[int] = set()

    for feld_id, b in bindung.items():
        kz = b.get("elster_kz")
        if not kz:
            continue
        schema_name_pattern, feld_start_element = _datenart_fuer_kz(kz)
        if start_element is not None:
            feld_start_element = start_element
        jahre_ergebnis: dict[int, dict] = {}
        for jahr in b.get("vz_gueltigkeit", []):
            pfad = _resolve_schema_pfad(jahr, schema_pfade, schema_name_pattern)
            if pfad is None:
                jahre_ergebnis[jahr] = {"status": STATUS_UNVERFUEGBAR, "pfade": []}
                unverfuegbare_jahre.add(jahr)
                continue
            cache_key = (pfad, feld_start_element)
            if cache_key not in schema_cache:
                schema_cache[cache_key] = walk(pfad, feld_start_element)
            kz_index, _hits = schema_cache[cache_key]
            kandidaten = kz_index.get(kz, [])
            if len(kandidaten) == 0:
                status = STATUS_NOT_FOUND
            elif len(kandidaten) == 1:
                status = STATUS_OK
            else:
                status = STATUS_AMBIGUOUS
            jahre_ergebnis[jahr] = {"status": status, "pfade": ["/".join(p) for p in kandidaten]}
        felder[feld_id] = {"status": _rollup(jahre_ergebnis), "jahre": jahre_ergebnis}

    max_depth_hits_gesamt = sum(hits for _kz_index, hits in schema_cache.values())
    alle_ok = all(f["status"] == STATUS_OK for f in felder.values())
    exit_code = 0 if (alle_ok and max_depth_hits_gesamt == 0) else 1

    return {
        "felder": felder,
        "unverfuegbare_jahre": sorted(unverfuegbare_jahre),
        "max_depth_hits_gesamt": max_depth_hits_gesamt,
        "exit_code": exit_code,
    }


# ---------------------------------------------------------------- Verzweigungs-Kz-Ernte (Task #13)
# Klasse d/f/g×f (NEGATION/VERZWEIGUNG/PARTNER_VERZWEIGUNG, est_mapping.py) tragen ihr Ziel-Kz NUR als
# Python-String-Literal in den Transform-Tabellen — nie als `elster_kz:` in bindung yaml (dort steht
# elster_kz: null + Prosa-Grund). lade_bindung() sieht diese Kz daher NIE (blinder Fleck, Task #11-Audit).
# Ernte sie hier zu synthetischen bindung-Einträgen (feld_id klar als "verzweigung:"/"negation:"/
# "aggregation:"-präfigiert markiert, keine Kollision mit echten feld_ids) und hängt sie in main() an
# lade_bindung() an, BEVOR pruefe_bindung läuft — vz_gueltigkeit wird vom zugehörigen Wert-Feld in
# bindung yaml übernommen (nie geraten).

def ernte_est_mapping_kz(bindung: dict) -> dict:
    synth: dict[str, dict] = {}

    def _vz(wert_feld: str) -> list:
        return bindung[wert_feld]["vz_gueltigkeit"]

    for wert_feld, kz in est_mapping.NEGATION.items():
        synth[f"negation:{wert_feld}"] = {"elster_kz": kz, "vz_gueltigkeit": _vz(wert_feld)}

    for tabelle in (est_mapping.VERZWEIGUNG, est_mapping.PARTNER_VERZWEIGUNG):
        for wert_feld, cfg in tabelle.items():
            vz = _vz(wert_feld)
            for art_wert, kz in cfg["kz"].items():
                synth[f"verzweigung:{wert_feld}:{art_wert}"] = {"elster_kz": kz, "vz_gueltigkeit": vz}

    for kz, quell_felder in est_mapping.DOKUMENTIERT_AGGREGAT.items():
        # Union der vz_gueltigkeit über ALLE Quellfelder (nicht nur [0]): falls die Quellfelder je
        # divergierende Gültigkeit tragen, wird der Aggregat-Kz in JEDEM Jahr geprüft, in dem
        # irgendein Quellfeld gilt — kein still verpasstes Jahr (dev-1-Härtung Task #18).
        jahre = sorted({j for qf in quell_felder for j in _vz(qf)})
        synth[f"aggregation:{kz}"] = {"elster_kz": kz, "vz_gueltigkeit": jahre}

    return synth


def _print_summary(ergebnis: dict) -> None:
    n = len(ergebnis["felder"])
    schlecht = {fid: f for fid, f in ergebnis["felder"].items() if f["status"] != STATUS_OK}
    print(f"[xsd_verify] {n} Felder geprüft, {len(schlecht)} nicht OK, "
          f"max_depth_hits_gesamt={ergebnis['max_depth_hits_gesamt']}, "
          f"unverfuegbare_jahre={ergebnis['unverfuegbare_jahre']}")
    for fid, f in sorted(schlecht.items()):
        print(f"  {fid}: {f['status']}  {f['jahre']}")


def main(argv: list[str] | None = None) -> int:
    bindung = lade_bindung()
    bindung = {**bindung, **ernte_est_mapping_kz(bindung)}
    ergebnis = pruefe_bindung(bindung)
    _print_summary(ergebnis)
    return ergebnis["exit_code"]


if __name__ == "__main__":
    sys.exit(main())
