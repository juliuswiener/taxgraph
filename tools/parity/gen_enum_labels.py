#!/usr/bin/env python3
"""Erzeugt `rust/api/src/enum_labels.rs` aus `ENUM_LABELS` in `produkt/haut/api_constants.py`.

Warum ein Generator statt Handarbeit: die Anzeigetexte stehen nur dort, nicht in der Bindung
(`api_constants.py:939-950`), und die abgeleiteten Schluessel (`<basis>_partner`, die
`setdefault`-Zeilen am Ende) entstehen erst beim Laden des Moduls. Erzeugt wird die FERTIGE Tabelle,
so wie `GET /stand` und `GET /feld/<fid>/frage` sie ausliefern.

Drift faengt `enum_labels_gleich` in `rust/parity/tests/api_http_paritaet.rs`: es laedt dieselbe
Python-Tabelle und vergleicht Schluessel, Werte und Reihenfolge. Aendert sich `api_constants.py`,
wird der Test rot, bis der Generator neu laeuft.

Run: python3 tools/parity/gen_enum_labels.py
"""
from __future__ import annotations

import importlib.util
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
QUELLE = ROOT / "produkt" / "haut" / "api_constants.py"
ZIEL = ROOT / "rust" / "api" / "src" / "enum_labels.rs"


def _lit(s: str) -> str:
    """Ein Rust-Zeichenkettenliteral; Steuerzeichen, `"` und `\\` gibt es in der Tabelle nicht."""
    if any(c in s for c in '"\\') or any(ord(c) < 0x20 for c in s):
        sys.exit(f"Zeichenkette {s!r}: Escaping noetig, der Generator kennt es nicht")
    return f'"{s}"'


def main() -> None:
    spec = importlib.util.spec_from_file_location("ac", QUELLE)
    assert spec is not None and spec.loader is not None
    ac = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ac)
    tabelle = ac.ENUM_LABELS
    for fid, labels in tabelle.items():
        if not isinstance(fid, str) or not isinstance(labels, dict):
            sys.exit(f"{fid!r}: kein dict[str, dict]")
        for wert, text in labels.items():
            if not isinstance(wert, str) or not isinstance(text, str):
                sys.exit(f"{fid}/{wert!r}: kein str")

    zeilen = [
        "//! GENERIERT von `tools/parity/gen_enum_labels.py` aus `produkt/haut/api_constants.py`.",
        "//! NICHT von Hand pflegen. Neu erzeugen:",
        "//!",
        "//! ```text",
        "//! python3 tools/parity/gen_enum_labels.py",
        "//! ```",
        "//!",
        "//! `ENUM_LABELS` (`api_constants.py:951`): je Enum-Feld die Anzeigetexte seiner Werte, in",
        "//! Pythons Einfuegereihenfolge, samt der abgeleiteten Schluessel am Ende des Moduls.",
        "//! `#[rustfmt::skip]` haelt das Generator-Layout, damit ein erneuter Lauf byte-stabil bleibt.",
        "//!",
        "//! Drift faengt `enum_labels_gleich` (`rust/parity/tests/api_http_paritaet.rs`).",
        "",
        "/// `feld_id -> [(enum-wert, anzeigetext)]`.",
        "#[rustfmt::skip]",
        f"pub const ENUM_LABELS: [(&str, &[(&str, &str)]); {len(tabelle)}] = [",
    ]
    for fid, labels in tabelle.items():
        zeilen.append(f"    ({_lit(fid)}, &[")
        for wert, text in labels.items():
            zeilen.append(f"        ({_lit(wert)}, {_lit(text)}),")
        zeilen.append("    ]),")
    zeilen.append("];")
    zeilen.append("")
    ZIEL.write_text("\n".join(zeilen), encoding="utf-8")
    print(f"{ZIEL.relative_to(ROOT)}: {len(tabelle)} Felder, "
          f"{sum(len(v) for v in tabelle.values())} Werte")


if __name__ == "__main__":
    main()
