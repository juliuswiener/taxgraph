#!/usr/bin/env python
"""Struktur-bewusster Kz-Extraktor aus der ESt-Schemadokumentation (E10-<vz>.html).

PRIMAERQUELLE fuer das Feldmapping (Instructor 2026-07-12): die E10-Schemadok ist ein
SVG-gerendertes XSD-Diagramm; die Anker-IDs kodieren den SEKTIONS-Pfad
(`#<SEKTION>_<hash>_CType-E<Kz>`), der die Kz-Labels disambiguiert (Label allein ist
Vordruckzeilen-quervernetzt und mehrdeutig). Dieser Extraktor liefert je Kz:
Sektions-Pfad + woertliches Label. Er RAET NICHT welche Kz zu einem Regel-Input passt
— das ist Kuratierung + Instructor-Review (Zitatanker-Doktrin auf Kz-Ebene).

Konfidenz je Kandidaten-Zeile (Instructor): XSD-Sektionspfad allein = MITTEL; erst der
amtliche Vordruck-Kreuzcheck (formulare-bfinv.de, Zeile<->Konzept) hebt auf STARK.

ERiC-Schemadok liegt extern (~/02_Software/eric); Pfad aus $EST_SCHEMA_HTML oder
Default-Suche unter $ERIC_DIR / ~/02_Software/eric.

    python elster/kz_extract.py --suche kirchensteuer
    python elster/kz_extract.py --sektion AgB
"""
from __future__ import annotations

import argparse
import glob
import html as htmllib
import os
import re
import sys

_ANCHOR = re.compile(r'href="#([A-Za-z0-9_]+)_m?-?\d{6,}_CType-(E\d{7})"')
_DOC = re.compile(
    r'name="(E\d{7})"[^>]*>\s*<xs:annotation>\s*<xs:documentation>([^<]{3,240})</xs:documentation>',
    re.S)


# Datenart -> Schema-Datei. E10 = ESt-Erklaerung (gerendertes HTML mit Sektions-Ankern,
# Anlagen N/AUS/S/G etc.); E77 = Anlage EUER (eigene ELSTER-Datenart, nur rohe XSD - KEIN
# gerendertes HTML). Fuer E77 fehlt der Sektions-Anker (0 _ANCHOR-Treffer, s.
# reports/review/.../feldmapping-*euer*.md), die Sektion bleibt leer: die Vordruck-Zeilen-
# ordnung ist Primaer-Disambiguator (Einzelformular, keine cross-Anlage-Label-Kollision).
_SCHEMA_DATEI = {"e10": "E10-2025.html", "e77": "E77-2025.xsd"}


def _find_schema(datenart: str = "e10") -> str | None:
    if datenart == "e10" and os.environ.get("EST_SCHEMA_HTML"):
        p = os.path.expanduser(os.environ["EST_SCHEMA_HTML"])
        return p if os.path.exists(p) else None
    name = _SCHEMA_DATEI.get(datenart)
    if not name:
        return None
    roots = [os.environ.get("ERIC_DIR", ""), os.path.expanduser("~/02_Software/eric")]
    for r in roots:
        if not r:
            continue
        hits = sorted(glob.glob(os.path.join(os.path.expanduser(r), "**", name),
                                recursive=True))
        if hits:
            return hits[0]
    return None


def lade(html_path: str) -> dict:
    """{Kz: {"sektion": str, "label": str}} aus der E10-Schemadok."""
    raw = open(html_path, encoding="utf-8", errors="replace").read()
    kz_sec = {}
    for m in _ANCHOR.finditer(raw):
        kz_sec.setdefault(m.group(2), m.group(1))
    labels = {}
    for m in _DOC.finditer(htmllib.unescape(raw)):
        labels.setdefault(m.group(1), " ".join(m.group(2).split()))
    out = {}
    for kz in set(kz_sec) | set(labels):
        out[kz] = {"sektion": kz_sec.get(kz, ""), "label": labels.get(kz, "")}
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--suche", help="Substring im Label (case-insensitive)")
    ap.add_argument("--sektion", help="Substring im Sektions-Pfad")
    ap.add_argument("--schema", help="Pfad zur Schemadatei (sonst $EST_SCHEMA_HTML / Default)")
    ap.add_argument("--datenart", choices=("e10", "e77"), default="e10",
                    help="e10 = ESt-Erklaerung (Anlagen N/AUS/S/G), e77 = Anlage EUER (eigene "
                         "Datenart, rohe XSD, Sektion leer = Vordruck-primaer)")
    args = ap.parse_args()

    path = args.schema or _find_schema(args.datenart)
    if not path:
        print("[kz] E10-2025.html nicht gefunden — $EST_SCHEMA_HTML setzen oder ERiC-Schemadok "
              "unter ~/02_Software/eric entpacken.")
        return 1
    daten = lade(path)
    print(f"[kz] Quelle: {path}  ({len(daten)} Kz)")
    treffer = []
    for kz, d in sorted(daten.items()):
        if args.suche and args.suche.lower() not in d["label"].lower():
            continue
        if args.sektion and args.sektion.lower() not in d["sektion"].lower():
            continue
        treffer.append((kz, d))
    for kz, d in treffer[:60]:
        print(f"  {kz} | [{d['sektion']}] | {d['label'][:100]}")
    print(f"[kz] {len(treffer)} Treffer" + (" (auf 60 gekuerzt)" if len(treffer) > 60 else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
