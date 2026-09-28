#!/usr/bin/env python3
"""Extrahiert `golden/cases/*.yaml` (135 Faelle) nach `rust/fixtures/golden_cases.json` fuer
den kuenftigen Rust-Golden-Runner (REWRITE_PLAN.md §7, Schritt 4+).

ponytail: Die Werte bleiben in EURO, exakt wie im YAML (`sachverhalt`/`erwartung` sind ein
freies dict je Fall -- ohne ein Feld-Schema, das Geld- von Nicht-Geld-Schluesseln
unterscheidet (z. B. `veranlagungszeitraum: 2025` vs. `zu_versteuerndes_einkommen: 100000`),
waere ein blindes ×100 auf jeden Integer falsch. Die Cent-Umrechnung gehoert dorthin, wo das
Schema bekannt ist -- der Rust-Golden-Runner ab Schritt 4, der die `domain`-Typen (Cent vs.
Jahr vs. Flag) schon hat.

Run: python3 tools/parity/extract_golden.py   (von der Repo-Wurzel)
"""

from __future__ import annotations

import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, ROOT)

from yamlstrict import load_str  # noqa: E402

CASES_DIR = os.path.join(ROOT, "golden", "cases")
OUT_PATH = os.path.join(ROOT, "rust", "fixtures", "golden_cases.json")


def main() -> None:
    faelle = []
    for name in sorted(os.listdir(CASES_DIR)):
        if not name.endswith(".yaml"):
            continue
        path = os.path.join(CASES_DIR, name)
        with open(path, encoding="utf-8") as fh:
            fall = load_str(fh.read(), herkunft=path)
        faelle.append({
            "id": fall["id"],
            "beschreibung": fall.get("beschreibung", ""),
            "sachverhalt": fall["sachverhalt"],
            "erwartung": fall["erwartung"],
            "quelle": fall.get("quelle", {}),
        })

    os.makedirs(os.path.dirname(OUT_PATH), exist_ok=True)
    with open(OUT_PATH, "w", encoding="utf-8") as fh:
        json.dump(faelle, fh, ensure_ascii=False, indent=2, sort_keys=True)
        fh.write("\n")

    print(f"{len(faelle)} Faelle -> {OUT_PATH}")


if __name__ == "__main__":
    main()
