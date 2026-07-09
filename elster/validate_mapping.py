"""Validate an ELSTER field-mapping file against the format (elster/feldmapping_schema.md).

Checks required fields, enum values, the regel_output dotted-name shape, and
uniqueness of elster_feld_id within (anlage, veranlagungszeitraum). Exit 1 on any
violation. Does not require ELSTER access (format check only).

Run: python elster/validate_mapping.py [mapping.yaml]   (default: feldmapping.stub.yaml)
"""

from __future__ import annotations

import os
import re
import sys

import yaml  # noqa: F401
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))
from yamlstrict import load_str


def load_yaml_fh(fh):
    """Strikt laden: doppelte Schluessel sind ein Fehler, kein Ueberschreiben."""
    with fh:
        return load_str(fh.read(), herkunft=getattr(fh, 'name', '<yaml>'))


ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

REQUIRED = ("regel_output", "elster_feld_id", "anlage", "typ", "pflicht", "status")
ANLAGEN = {"Mantelbogen", "Anlage N", "Anlage Vorsorgeaufwand", "Anlage Kind"}
TYPEN = {"euro", "integer", "bool", "string", "prozent"}
STATUS = {"stub", "mapped", "verified"}
REGEL_OUTPUT_RE = re.compile(r"^[a-zA-Z0-9_]+(\.[a-zA-Z0-9_]+){1,2}$")


def main(path: str) -> int:
    data = load_yaml_fh(open(path, encoding="utf-8"))
    rows = data.get("mapping", [])
    errors = []
    seen = {}
    for i, row in enumerate(rows):
        loc = f"row {i} ({row.get('regel_output', '?')})"
        for f in REQUIRED:
            if f not in row or row[f] in (None, ""):
                errors.append(f"{loc}: missing required field '{f}'")
        if row.get("anlage") not in ANLAGEN:
            errors.append(f"{loc}: invalid anlage '{row.get('anlage')}'")
        if row.get("typ") not in TYPEN:
            errors.append(f"{loc}: invalid typ '{row.get('typ')}'")
        if row.get("status") not in STATUS:
            errors.append(f"{loc}: invalid status '{row.get('status')}'")
        if not isinstance(row.get("pflicht"), bool):
            errors.append(f"{loc}: pflicht must be bool")
        if not REGEL_OUTPUT_RE.match(str(row.get("regel_output", ""))):
            errors.append(f"{loc}: regel_output must be dotted <modul>.<...>")
        key = (row.get("anlage"), row.get("veranlagungszeitraum"), row.get("elster_feld_id"))
        if key in seen:
            errors.append(f"{loc}: duplicate elster_feld_id in {key[:2]}")
        seen[key] = i

    if errors:
        print(f"INVALID ({len(errors)} Fehler):")
        for e in errors:
            print("  -", e)
        return 1
    print(f"OK: {len(rows)} Mapping-Zeilen, Format gueltig ({os.path.relpath(path, ROOT)}).")
    return 0


if __name__ == "__main__":
    p = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "elster", "feldmapping.stub.yaml")
    sys.exit(main(p))
