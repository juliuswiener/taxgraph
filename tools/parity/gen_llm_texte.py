"""Erzeugt `rust/llm/src/texte.rs` aus `produkt/haut/api_llm.py` und `kontoauszug_writer.py`.

Die festen Textteile der Prompts und die drei JSON-Schemas werden NICHT abgetippt, sondern aus der
Python-Referenz geschnitten: die Prompt-Funktionen laufen mit Marker-Eingaben, und die Stuecke
zwischen den Markern sind die Konstanten. `llm_paritaet` vergleicht danach jeden gebauten Prompt
byte-genau mit Python — ein veraltetes Generat wird dort rot.

Start: python3 tools/parity/gen_llm_texte.py   (von der Repo-Wurzel)
"""
from __future__ import annotations

import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
for sub in ("produkt/store", "produkt/traverser", "produkt/haut", "produkt/eingang", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api_llm as AL  # noqa: E402
import kontoauszug_writer as KW  # noqa: E402

M = "\u0000MARKE\u0000"


def _teile(text: str, n: int) -> list[str]:
    stuecke = text.split(M)
    assert len(stuecke) == n, (len(stuecke), n)
    return stuecke


def _rust(name: str, wert: str) -> str:
    return f"pub(crate) const {name}: &str = {json.dumps(wert, ensure_ascii=False)};\n"


def main() -> None:
    out = ["//! GENERIERT von `tools/parity/gen_llm_texte.py` aus `produkt/haut/api_llm.py` und\n",
           "//! `produkt/eingang/kontoauszug_writer.py` — nicht von Hand aendern. `llm_paritaet` prueft\n",
           "//! jeden daraus gebauten Prompt byte-genau gegen Python.\n",
           "#![allow(clippy::doc_markdown)]\n\n"]
    # Stufe 1
    s = AL._aussagen_prompt(M)[0]["content"]
    out.append(_rust("AUSSAGEN_SYSTEM", s))
    # Stufe 2: nummeriert und regel_zeilen als Marker
    s = AL._themen_prompt("x", [{"text": M}], M)[0]["content"]
    a, b, c = _teile(s, 3)
    assert b.startswith("\n\nREGELN:\n") and a.endswith("AUSSAGEN:\n[0] ")
    out.append(_rust("THEMEN_KOPF", a[: -len("[0] ")]))
    out.append(_rust("THEMEN_MITTE", b))
    out.append(_rust("THEMEN_ENDE", c))
    # Stufe 3: felder=Marker (ein Feld mit Marker-feld_id), Aussagen-Block, Kontext
    kat = [{"feld_id": M}]
    s = AL._dialog_prompt("x", kat, M, [{"text": M}])[0]["content"]
    teile = _teile(s, 4)
    kopf, feld_rest, block_rest, ende = teile
    assert kopf.endswith("(keine anderen):\n- ")
    kopf = kopf[: -len("- ")]
    # feld_rest = ":  (Typ )" + "\n\n...RÜCKFRAGEN...\n\n" + Aussagen-Block-Kopf + "[0] "
    feld_zeile_ende = ":  (Typ )"
    assert feld_rest.startswith(feld_zeile_ende)
    mitte = feld_rest[len(feld_zeile_ende):]
    block_kopf = AL._aussagen_block([{"text": M}]).split(M)[0]
    assert mitte.endswith(block_kopf)
    out.append(_rust("DIALOG_KOPF", kopf))
    out.append(_rust("DIALOG_REGELN", mitte[: -len(block_kopf)]))
    out.append(_rust("AUSSAGEN_BLOCK_KOPF", block_kopf[: -len("[0] ")]))
    block_ende = AL._aussagen_block([{"text": M}]).split(M)[1]
    assert block_rest.startswith(block_ende)
    out.append(_rust("AUSSAGEN_BLOCK_ENDE", block_ende))
    kontext_ende = block_rest[len(block_ende):]
    assert kontext_ende == "", repr(kontext_ende)
    assert ende.startswith("\n\n")
    out.append(_rust("DIALOG_ENDE", ende[len("\n\n"):]))
    # Kontoauszug-Klassifikator
    out.append(_rust("KONTOAUSZUG_SYSTEM", KW._LLM_PROMPT))
    # Schemas als JSON-Text
    for name, sch in (("DIALOG_SCHEMA", AL.DIALOG_SCHEMA), ("AUSSAGEN_SCHEMA", AL.AUSSAGEN_SCHEMA),
                      ("ZUORDNUNG_SCHEMA", AL.ZUORDNUNG_SCHEMA)):
        out.append(_rust(f"{name}_JSON", json.dumps(sch, ensure_ascii=False)))
    ziel = os.path.join(ROOT, "rust", "llm", "src", "texte.rs")
    with open(ziel, "w", encoding="utf-8") as f:
        f.write("".join(out))
    print(ziel)


if __name__ == "__main__":
    main()
