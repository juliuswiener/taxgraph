#!/usr/bin/env python3
"""Read-only Parity-Dump: die Kz-Tabellen aus `produkt/mapping/est_mapping.py` als JSON, damit
`rust/elster/src/tabellen.rs` ohne `PARITY=1` und ohne Python-Orakel dagegen geprueft werden kann
(`rust/fixtures/kz_tabellen.json`, Rust-Test `tabellen::tests`, Python-Test
`tests/test_kz_tabellen_fixture.py`).

Gedumpt wird, was ein Kz oder die Art-Weiche zu einem Kz festlegt: `KONSTANTE_KZ`,
`IBAN_TRANSFORM_ZIEL_KZ`, `NEGATION`, `DOKUMENTIERT_AGGREGAT`, § 23 (`P23_*`), `VERZWEIGUNG`,
`PARTNER_VERZWEIGUNG`, `PARTNER_INSTANZ`, `PFLEGE_KZ`, `WERTEKODIERUNG` (Kz und Codes, ohne
Hinweistext). Mengen stehen sortiert. Nicht darin: Feldnamen-Listen ohne Kz (`KAP_FELDER_*`,
`MULTIPLIKATION`, `PFLICHTFELDER`) und die Kz-Listen in `rust/elster/src/kz_format.rs`.

Run: python3 tools/parity/dump_kz_tabellen.py   (schreibt rust/fixtures/kz_tabellen.json)
"""
from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
ZIEL = ROOT / "rust" / "fixtures" / "kz_tabellen.json"
sys.path.insert(0, str(ROOT / "produkt" / "mapping"))

import est_mapping as M  # noqa: E402  (importiert nur os und re)


def _verzweigung(tabelle: dict) -> dict:
    return {feld: {"art_feld": v["art_feld"], "kz": dict(v["kz"])} for feld, v in tabelle.items()}


def tabellen() -> dict:
    """Die Tabellen in der Form, die der Rust-Test aus `tabellen.rs` nachbaut."""
    # ponytail: nur Tabellen mit Kz. Feldnamen-Listen ohne Kz (KAP_FELDER_*, MULTIPLIKATION, PFLICHTFELDER),
    # die Hinweistexte der WERTEKODIERUNG und die Kz-Listen in kz_format.rs fehlen. Upgrade: hier einen
    # Schluessel ergaenzen und in `tabellen::tests::aus_tabellen` denselben.
    return {
        "konstante_kz": sorted(M.KONSTANTE_KZ),
        "iban_transform_ziel_kz": sorted(M.IBAN_TRANSFORM_ZIEL_KZ),
        "negation": dict(M.NEGATION),
        "dokumentiert_aggregat": {kz: sorted(felder) for kz, felder in M.DOKUMENTIERT_AGGREGAT.items()},
        "p23": {
            "betragsfelder": sorted(M.P23_BETRAGSFELDER),
            "art_feld": M.P23_GEWINN["art_feld"],
            "gewinn_kz": dict(M.P23_GEWINN["kz"]),
        },
        "verzweigung": _verzweigung(M.VERZWEIGUNG),
        "partner_verzweigung": _verzweigung(M.PARTNER_VERZWEIGUNG),
        "partner_instanz": dict(M.PARTNER_INSTANZ),
        "pflege_kz": sorted(M.PFLEGE_KZ),
        "wertekodierung": {
            feld: {"kz": v["kz"], "code": dict(v["code"])} for feld, v in M.WERTEKODIERUNG.items()
        },
    }


def text() -> str:
    return json.dumps(tabellen(), ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def main() -> None:
    ZIEL.parent.mkdir(parents=True, exist_ok=True)
    ZIEL.write_text(text(), encoding="utf-8")
    print(f"{len(tabellen())} Tabellen -> {ZIEL}")


if __name__ == "__main__":
    main()
