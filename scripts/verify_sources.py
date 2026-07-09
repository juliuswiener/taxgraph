"""Verify the frozen source archive: each text file must match the SHA256 in
its .meta.yaml. Deterministic integrity gate for sources/ (precursor of the
citation-anchor gate). Exit code 1 on any mismatch.

Run: python scripts/verify_sources.py   (or: make sources-check)
"""

from __future__ import annotations

import glob
import hashlib
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
SRC = os.path.join(ROOT, "sources")

# Gemessen wird der WORTLAUT nach dem Marker, nicht die Datei: der Metadaten-Kopf
# allein bringt schon rund 300 Bytes mit. § 9 Abs. 1 S. 3 Nr. 6 ist ein einziger
# Satz und damit legitim kurz.
MARKER = "--- Wortlaut"
MIN_WORTLAUT = 60


def wortlaut_laenge(path: str) -> int:
    t = open(path, encoding="utf-8", errors="replace").read()
    i = t.find(MARKER)
    body = t[i + len(MARKER):] if i >= 0 else t
    return len(body.strip())


def sha256_of(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        h.update(f.read())
    return h.hexdigest()


def field(text: str, name: str) -> str | None:
    m = re.search(rf'^\s*{name}:\s*"?([^"\n]+)"?\s*$', text, re.MULTILINE)
    return m.group(1).strip() if m else None


def main() -> int:
    metas = sorted(glob.glob(os.path.join(SRC, "**", "*.meta.yaml"), recursive=True))
    if not metas:
        print("no source metadata found")
        return 1
    ok = True
    for meta in metas:
        text = open(meta, encoding="utf-8").read()
        datei = field(text, "datei")
        expected = field(text, "sha256")
        if not datei or not expected:
            print(f"MISSING FIELDS: {os.path.relpath(meta, ROOT)}")
            ok = False
            continue
        target = os.path.join(os.path.dirname(meta), datei)
        if not os.path.exists(target):
            print(f"MISSING FILE:  {datei} (referenced by {os.path.relpath(meta, ROOT)})")
            ok = False
            continue
        actual = sha256_of(target)
        if actual != expected:
            ok = False
            print(f"{'MISMATCH':8} {os.path.relpath(target, ROOT)}")
            continue
        # Eine leere Quelle passt zu ihrem sha256 - der Integritaetscheck allein
        # haette zwei versehentlich leer eingefrorene Paragraphen durchgewunken.
        # Sie faellt sonst erst auf, wenn Wochen spaeter ein Zitatanker ins Leere
        # zeigt. Ein Gesetzesausschnitt unter 200 Zeichen ist kein Wortlaut.
        laenge = wortlaut_laenge(target)
        if laenge < MIN_WORTLAUT:
            ok = False
            print(f"{'LEER':8} {os.path.relpath(target, ROOT)} "
                  f"(Wortlaut nur {laenge} Zeichen)")
            continue
        print(f"{'OK':8} {os.path.relpath(target, ROOT)}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
