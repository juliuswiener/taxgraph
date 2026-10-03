"""Die Klartexte der Sperrgründe stehen in `rust/fixtures/sperrgrund_klartext.json` so, wie Python sie führt.

`tools/parity/dump_sperrgruende.py` schreibt `SPERRGRUND_KLARTEXT` und `UNBEKANNTER_SPERRGRUND` aus
`bescheid_deklaration.py` in diese Datei; Rust (`domain::Sperrgrund::klartext`) wird gegen sie geprüft
(`klartext_ist_byte_identisch_zur_python_quelle`). Die Datei ist ein Abbild. Ändert jemand einen Klartext oder
fügt einen Grund hinzu und erzeugt sie nicht neu, merkte das bisher nur `PARITY=1`
(`bescheid_deklaration_paritaet::sperrgrund_klartext_literale`): der Standardlauf blieb grün, und Rust prüfte
gegen einen veralteten Text (gemessen 2026-10-03: Mutation eines Python-Klartexts, `cargo test -p domain` grün).

Dieser Test braucht weder Rust noch `PARITY=1`: er vergleicht das Python-Dict mit der Datei, Schlüsselmenge UND
Texte (Byte für Byte). Rot heißt: `python3 tools/parity/dump_sperrgruende.py` laufen lassen und die Datei
committen.

NULL LLM."""
from __future__ import annotations

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/eingang", "produkt/mapping", "produkt/bescheid", "golden",
             "elster/submission"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import bescheid_deklaration as BD  # noqa: E402

FIXTURE = os.path.join(ROOT, "rust", "fixtures", "sperrgrund_klartext.json")
HINWEIS = "Fixture neu erzeugen: python3 tools/parity/dump_sperrgruende.py, dann rust/fixtures/ committen."


def _fixture() -> dict:
    with open(FIXTURE, encoding="utf-8") as f:
        return json.load(f)


def test_die_fixture_fuehrt_dieselben_sperrgruende_wie_python():
    py, fix = set(BD.SPERRGRUND_KLARTEXT), set(_fixture()["klartext"])
    assert py == fix, (f"nur in Python: {sorted(py - fix)}; nur in der Fixture: {sorted(fix - py)}. {HINWEIS}")


def test_jeder_klartext_der_fixture_ist_byte_gleich_dem_python_text():
    fix = _fixture()["klartext"]
    abweichend = sorted(k for k, text in BD.SPERRGRUND_KLARTEXT.items() if fix.get(k) != text)
    assert not abweichend, f"Text weicht ab bei {abweichend}. {HINWEIS}"


def test_der_text_fuer_unbekannte_gruende_ist_byte_gleich():
    assert _fixture()["unbekannt"] == BD.UNBEKANNTER_SPERRGRUND, HINWEIS


def test_die_fixture_ist_nicht_leer_und_kennt_den_neuesten_grund():
    """Gegenprobe gegen „beide Seiten leer“: die Vergleiche oben wären sonst grün."""
    fix = _fixture()["klartext"]
    assert len(fix) >= 57 and fix["abs3_partner_gewinn_offen"].strip()
