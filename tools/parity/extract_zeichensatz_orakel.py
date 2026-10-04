#!/usr/bin/env python3
"""Hermetisches Orakel fuer `domain::zeichensatz` aus `produkt/store/zeichensatz.py`.

Der Rust-Paritaetstest `rust/parity/tests/store_zeichensatz_paritaet.rs` braucht `PARITY=1` und Python. Dieses
Skript haelt dasselbe Orakel als Fixture, damit `rust/domain/tests/zeichensatz_hermetisch.rs` im Standardlauf laeuft.

Je Codepunkt (ohne Surrogate) steht fest: erlaubt oder nicht, wie die Meldung das Zeichen nennt (Name oder das Zeichen
selbst), und der Rat. Gleiche Nachbarn sind zu Laeufen zusammengefasst.

Run: python3 tools/parity/extract_zeichensatz_orakel.py   (schreibt rust/fixtures/zeichensatz_orakel.json)
"""
from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from produkt.store import zeichensatz as z  # noqa: E402

ZIEL = ROOT / "rust" / "fixtures" / "zeichensatz_orakel.json"
MAX = 0x10FFFF


def codepunkte():
    return (cp for cp in range(MAX + 1) if not 0xD800 <= cp <= 0xDFFF)


def main() -> None:
    erlaubt: list[list[int]] = []
    laeufe: list[list] = []
    for cp in codepunkte():
        c = chr(cp)
        if z.erstes_unerlaubtes_zeichen(c) is None:
            if erlaubt and erlaubt[-1][1] + 1 == cp:
                erlaubt[-1][1] = cp
            else:
                erlaubt.append([cp, cp])
        anzeige, punkt = z.zeichen_anzeige(c), f"U+{cp:04X}"
        if anzeige == f'„{c}" ({punkt})':
            name = None  # die Meldung zeigt das Zeichen selbst
        else:
            assert anzeige.endswith(f" ({punkt})"), anzeige
            name = anzeige[: -len(f" ({punkt})")]
        rat = z.zeichen_rat(c)
        if laeufe and laeufe[-1][1] + 1 == cp and laeufe[-1][2] == name and laeufe[-1][3] == rat:
            laeufe[-1][1] = cp
        else:
            laeufe.append([cp, cp, name, rat])

    proben = [
        ["stammdaten_nachname", "ł", z.feld_meldung("stammdaten_nachname", "ł")],
        ["x", "\t", z.feld_meldung("x", "\t")],
        ["x", "́", z.feld_meldung("x", "́")],
        ["x", "–", z.feld_meldung("x", "–")],
        ["x", "ж", z.feld_meldung("x", "ж")],
    ]
    elemente = [
        [el, text, z.element_meldung(el, text)]
        for el, text in [("Nachname", "Müller"), ("Nachname", "Miłler"), ("Ort", "a b\tc"), ("Ort", ""), ("Ort", "€ß")]
    ]
    def zeilen(name: str, eintraege: list) -> str:
        return f'"{name}": [\n' + ",\n".join(json.dumps(e, ensure_ascii=False) for e in eintraege) + "\n]"

    ZIEL.write_text(
        "{\n"
        + ",\n".join(
            zeilen(n, e)
            for n, e in [("erlaubt", erlaubt), ("laeufe", laeufe), ("feld_meldung", proben), ("element_meldung", elemente)]
        )
        + "\n}\n",
        encoding="utf-8",
    )
    print(f"{len(erlaubt)} erlaubte Bereiche, {len(laeufe)} Laeufe -> {ZIEL}")


main()
