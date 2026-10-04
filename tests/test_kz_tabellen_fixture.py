"""Die Kz-Tabellen stehen in `rust/fixtures/kz_tabellen.json` so, wie `est_mapping.py` sie führt.

`tools/parity/dump_kz_tabellen.py` schreibt `KONSTANTE_KZ`, `IBAN_TRANSFORM_ZIEL_KZ`, `NEGATION`,
`DOKUMENTIERT_AGGREGAT`, `P23_*`, `VERZWEIGUNG` (darin die neun Sanierungsarten § 35c),
`PARTNER_VERZWEIGUNG`, `PARTNER_INSTANZ`, `PFLEGE_KZ`, `WERTEKODIERUNG` und 21 weitere Schlüssel (Mengen, Regeln, `proben`)
in diese Datei. Rust prüft sie mit `tabellen::tests` (Regal: `regal.rs`; Bankverbindung: `xml::tests`).
Die Datei ist ein Abbild. Ändert jemand eine Kz in `est_mapping.py` und erzeugt sie nicht neu, merkte das
bisher nur `PARITY=1` (`elster_paritaet`, braucht das Python-Orakel): der Standardlauf blieb grün, und Rust
prüfte gegen einen veralteten Stand (gemessen 2026-10-03: Kz-Tausch in `tabellen.rs`, `cargo test
--workspace --exclude parity` grün, auch bei einer Kz, die es nicht gibt).

Dieser Test braucht weder Rust noch `PARITY=1`. Er vergleicht `est_mapping.py` mit der Datei, Tabelle für
Tabelle und Eintrag für Eintrag; jede Abweichung in beide Richtungen ist rot. Rot heißt: `python3
tools/parity/dump_kz_tabellen.py` laufen lassen, die Datei committen und, wenn der Rust-Test dann rot
wird, `rust/elster/src/tabellen.rs` nachziehen.

NULL LLM."""
from __future__ import annotations

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "tools", "parity"))

import dump_kz_tabellen as DUMP  # noqa: E402

FIXTURE = os.path.join(ROOT, "rust", "fixtures", "kz_tabellen.json")
HINWEIS = ("Fixture neu erzeugen: python3 tools/parity/dump_kz_tabellen.py, dann rust/fixtures/ committen "
           "und rust/elster/src/tabellen.rs nachziehen.")


def _fixture() -> dict:
    with open(FIXTURE, encoding="utf-8") as f:
        return json.load(f)


def _abweichungen(pfad: str, py, fix) -> list[str]:
    """Jeder Eintrag, der in `est_mapping.py` und in der Fixture verschieden steht (Pfad + beide Werte)."""
    if isinstance(py, dict) and isinstance(fix, dict):
        aus: list[str] = []
        for k in sorted(set(py) | set(fix)):
            if k not in fix:
                aus.append(f"{pfad}/{k}: nur in est_mapping.py ({py[k]!r})")
            elif k not in py:
                aus.append(f"{pfad}/{k}: nur in der Fixture ({fix[k]!r})")
            else:
                aus += _abweichungen(f"{pfad}/{k}", py[k], fix[k])
        return aus
    return [] if py == fix else [f"{pfad}: est_mapping.py {py!r}, Fixture {fix!r}"]


def test_die_fixture_fuehrt_dieselben_tabellen_wie_est_mapping():
    py, fix = set(DUMP.tabellen()), set(_fixture())
    assert py == fix, f"nur in est_mapping: {sorted(py - fix)}; nur in der Fixture: {sorted(fix - py)}. {HINWEIS}"


def test_jeder_eintrag_der_fixture_ist_gleich_dem_aus_est_mapping():
    abw = _abweichungen("", DUMP.tabellen(), _fixture())
    assert not abw, "Abweichungen:\n  " + "\n  ".join(abw) + f"\n{HINWEIS}"


def test_der_vergleich_sieht_einen_unterschied_und_die_fixture_ist_nicht_leer():
    """Gegenprobe gegen „beide Seiten leer“ oder einen Vergleich, der nie etwas findet (nur Fixture und Helfer,
    nicht die Tabellen: eine Abweichung dort macht diesen Test nicht zusätzlich rot)."""
    fix = _fixture()
    p35c = fix["verzweigung"]["p35c_massnahme_einzelbetrag"]["kz"]
    assert len(p35c) == 9 and "heizung" in p35c, "neun Sanierungsarten"
    assert fix["negation"] and fix["p23"]["gewinn_kz"], "NEGATION und P23_GEWINN_KZ nicht leer"
    geaendert = json.loads(json.dumps(fix))
    geaendert["verzweigung"]["p35c_massnahme_einzelbetrag"]["kz"]["heizung"] = "E0241599"
    abw = _abweichungen("", fix, geaendert)
    assert len(abw) == 1 and "heizung" in abw[0], abw
