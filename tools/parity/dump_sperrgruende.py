#!/usr/bin/env python3
"""Read-only Parity-Dump: `SPERRGRUND_KLARTEXT` + `UNBEKANNTER_SPERRGRUND` aus
`produkt/bescheid/bescheid_deklaration.py` als JSON, damit `domain::Sperrgrund::klartext()`
byte-identisch dagegen getestet werden kann (`REWRITE_PLAN.md` Schritt 2, Sperrgrund-Parity).

Nur AST-Parse, kein Import: `bescheid_deklaration.py` zieht schwere Abhaengigkeiten (Ring,
Traverser, ...), die dieses Skript nicht braucht und die die Bindung an dieses eine Modul
unnoetig fragil machen wuerden. Schreibt NICHTS unter `produkt/` -- reine Lesehilfe fuer die
Rust-Portierung.

Run: python3 tools/parity/dump_sperrgruende.py   (schreibt rust/fixtures/sperrgrund_klartext.json)
"""
from __future__ import annotations

import ast
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
QUELLE = ROOT / "produkt" / "bescheid" / "bescheid_deklaration.py"
ZIEL = ROOT / "rust" / "fixtures" / "sperrgrund_klartext.json"


def main() -> None:
    baum = ast.parse(QUELLE.read_text(encoding="utf-8"), filename=str(QUELLE))
    klartext = None
    unbekannt = None
    for node in ast.walk(baum):
        paare: list[tuple[str, ast.expr]] = []
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            paare.append((node.targets[0].id, node.value))
        elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name) and node.value is not None:
            paare.append((node.target.id, node.value))
        for name, value in paare:
            if name == "SPERRGRUND_KLARTEXT":
                klartext = ast.literal_eval(value)
            elif name == "UNBEKANNTER_SPERRGRUND":
                unbekannt = ast.literal_eval(value)

    if klartext is None or unbekannt is None:
        sys.exit("SPERRGRUND_KLARTEXT oder UNBEKANNTER_SPERRGRUND nicht gefunden -- Datei geaendert?")

    ZIEL.parent.mkdir(parents=True, exist_ok=True)
    ZIEL.write_text(
        json.dumps({"klartext": klartext, "unbekannt": unbekannt}, ensure_ascii=False, indent=2, sort_keys=True)
        + "\n",
        encoding="utf-8",
    )
    print(f"{len(klartext)} Sperrgruende -> {ZIEL}")


if __name__ == "__main__":
    main()
