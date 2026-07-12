#!/usr/bin/env python
"""Stufe (a): Offline-XSD-Schema-Validierung eines ESt-Submission-XML (Struktur-Gate).

Validiert gegen das amtliche ELSTER-Schema `elster11_E10_<vz>_extern.xsd` aus der ERiC-
Auslieferung via `xmllint` — KEIN Netz, KEINE Hersteller-ID, KEIN Versand. Das ist die
erste, sofort laufende Verifikationsstufe des Submission-Bausteins. Stufe (b) checkESt-
Plausibilitaet (elster/checkest_gate.py) dockt spaeter mit der Hersteller-ID an.

Schema-Pfad aus $ERIC_DIR / ~/02_Software/eric (Doku-Zip entpackt unter doc_extract).

    python elster/submission/validate_xsd.py <xml> [--vz 2025]
    python elster/submission/validate_xsd.py --prove
"""
from __future__ import annotations

import argparse
import glob
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def find_schema(vz: str) -> str | None:
    roots = [os.environ.get("ERIC_DIR", ""), os.path.expanduser("~/02_Software/eric")]
    for r in roots:
        if not r:
            continue
        hits = sorted(glob.glob(os.path.join(os.path.expanduser(r), "**",
                                f"elster11_E10_{vz}_extern.xsd"), recursive=True))
        if hits:
            return hits[0]
    return None


def validate(xml_path: str, vz: str = "2025") -> tuple[bool, str]:
    """(ok, meldung). ok=True iff xmllint das XML gegen das E10-<vz>-Schema akzeptiert."""
    if not shutil.which("xmllint"):
        return False, "xmllint nicht gefunden (libxml2-utils installieren)."
    schema = find_schema(vz)
    if not schema:
        return False, (f"Schema elster11_E10_{vz}_extern.xsd nicht gefunden — ERIC_DIR setzen "
                       f"/ ERiC-Doku entpacken.")
    r = subprocess.run(["xmllint", "--noout", "--schema", schema, xml_path],
                       capture_output=True, text=True)
    return r.returncode == 0, (r.stderr.strip() or "validates")


def _prove() -> int:
    valid = os.path.join(HERE, "testfall_est2025_minimal.xml")
    ok, msg = validate(valid, "2025")
    print(f"[xsd] Valider Testfall ({os.path.basename(valid)}) -> "
          f"{'PASS' if ok else 'FAIL'}: {msg[:80]}")
    # Negativfall: ein Element durch ein schema-fremdes ersetzen -> muss fallen.
    import tempfile
    with open(valid, encoding="utf-8") as f:
        broken = f.read().replace("<E0100201>", "<E9999999>").replace("</E0100201>", "</E9999999>")
    tmp = tempfile.NamedTemporaryFile("w", suffix=".xml", delete=False, encoding="utf-8")
    tmp.write(broken)
    tmp.close()
    bad_ok, _ = validate(tmp.name, "2025")
    os.unlink(tmp.name)
    print(f"[xsd] Kaputter Testfall (schema-fremdes Element) -> "
          f"{'FAIL (erwartet)' if not bad_ok else 'PASS (FEHLER: Gate wirkungslos!)'}")
    passed = ok and not bad_ok
    print(f"[xsd] Verdikt Stufe (a): {'STRUKTUR-GATE FUNKTIONSFAEHIG' if passed else 'FEHLGESCHLAGEN'} "
          f"(valide -> PASS, kaputt -> FAIL). Offline, keine Hersteller-ID. VZ 2025 = Pipe-Proof; "
          f"amtliche 2026-Werte spaeter, wenn das VZ-2026-Modul vorliegt.")
    return 0 if passed else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("xml", nargs="?")
    ap.add_argument("--vz", default="2025")
    ap.add_argument("--prove", action="store_true")
    args = ap.parse_args()
    if args.prove:
        return _prove()
    if not args.xml:
        print("Nutzung: validate_xsd.py <xml> [--vz 2025]  |  --prove")
        return 2
    ok, msg = validate(args.xml, args.vz)
    print(f"{'PASS' if ok else 'FAIL'}: {msg}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
