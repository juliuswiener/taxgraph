"""tests/conftest.py lenkt das Fehlerprotokoll um, solange es die .env laedt (AK2 von logging-fehlt-noch-beim-env-lesen).

Der Sitzungsstart ruft `server._lade_env_dateien(_ROOT)` auf. Ist eine vorhandene `.env` nicht lesbar,
schreibt diese Funktion eine WARNING nach `audit.AUDIT_DIR/fehler.log`. Ohne die Umlenkung
(`_audit.AUDIT_DIR = _env_ablage`) landet sie in der ECHTEN Datei des Nutzers: unter `-n 6` sieben Zeilen je
Sitzungsstart, nicht zu unterscheiden von einem Serverstart. `tests/test_env_loader.py` fing das nicht
(Mutante n25 der Backlog-Inventur 2026-10-04: Umlenkung raus, 8 passed), weil die Fixture
`_kein_schreiben_ins_echte_fehler_log` erst NACH dem conftest-Import greift.

Dieser Waechter importiert tests/conftest.py in einem Unterprozess, dessen "echte" Ablage ein Temp-Verzeichnis ist
(TAXGRAPH_AUDIT_DIR), und lenkt den Aufruf auf einen Temp-Root mit einer .env aus ungueltigem UTF-8 um. Er prueft
zweierlei: die echte `fehler.log` bleibt leer, und `audit.AUDIT_DIR` steht nach dem Import wieder auf dem
Importwert (das Zuruecksetzen ist Pflicht, sonst bewachen die zwei Wachen in conftest die falsche Datei).
Die Positivkontrolle ruft dieselbe Funktion OHNE conftest und erwartet genau eine Zeile: ohne sie bewiese
"leer" nichts.
"""
from __future__ import annotations

import json
import os
import pathlib
import subprocess
import sys

import pytest

ROOT = pathlib.Path(__file__).resolve().parent.parent

PROBE = r'''
import importlib.util, json, os, sys
root, fremd, modus = sys.argv[1], sys.argv[2], sys.argv[3]
for sub in ("produkt/store", "produkt/haut"):
    sys.path.insert(0, os.path.join(root, sub))
import audit
import server
echt = audit.AUDIT_DIR
if modus == "conftest":
    echte_funktion = server._lade_env_dateien
    # conftest ruft mit dem Repo-Root; hier liest sie stattdessen den Temp-Root mit der kaputten .env.
    server._lade_env_dateien = lambda _root: echte_funktion(fremd)
    spec = importlib.util.spec_from_file_location("conftest_probe", os.path.join(root, "tests", "conftest.py"))
    spec.loader.exec_module(importlib.util.module_from_spec(spec))
else:
    server._lade_env_dateien(fremd)
print("PROBE=" + json.dumps({"echt": echt, "danach": audit.AUDIT_DIR}))
'''


def _lauf(tmp_path: pathlib.Path, modus: str) -> dict:
    echt = tmp_path / modus / "echt"
    fremd = tmp_path / modus / "root"
    echt.mkdir(parents=True)
    fremd.mkdir()
    (fremd / ".env").write_bytes(b"\xff\xfeTG_UMLENKUNG=\x80\n")  # kein gueltiges UTF-8
    umgebung = {k: v for k, v in os.environ.items() if k != "PYTEST_ADDOPTS"}
    umgebung["TAXGRAPH_AUDIT_DIR"] = str(echt)
    lauf = subprocess.run([sys.executable, "-c", PROBE, str(ROOT), str(fremd), modus],
                          env=umgebung, capture_output=True, text=True, timeout=120)
    zeilen = [z for z in lauf.stdout.splitlines() if z.startswith("PROBE=")]
    assert lauf.returncode == 0 and len(zeilen) == 1, (
        f"Probe {modus!r} rc={lauf.returncode}\n{lauf.stdout[-2000:]}\n{lauf.stderr[-2000:]}")
    log = echt / "fehler.log"
    befund = json.loads(zeilen[0][len("PROBE="):])
    befund["zeilen"] = log.read_text(encoding="utf-8").splitlines() if log.exists() else []
    befund["echt_soll"] = str(echt)
    return befund


@pytest.fixture(scope="module")
def direkt(tmp_path_factory):
    return _lauf(tmp_path_factory.mktemp("direkt"), "direkt")


@pytest.fixture(scope="module")
def mit_conftest(tmp_path_factory):
    return _lauf(tmp_path_factory.mktemp("conftest"), "conftest")


def test_positivkontrolle_ohne_umlenkung_steht_eine_zeile_in_der_echten_datei(direkt):
    assert len(direkt["zeilen"]) == 1 and "server.env_datei_lesen" in direkt["zeilen"][0], (
        f"Die kaputte .env schreibt ohne conftest nicht genau eine Zeile: {direkt['zeilen']!r}. "
        f"Dann misst der Test unten nichts.")


def test_conftest_import_schreibt_nichts_in_die_echte_fehler_log(mit_conftest):
    assert mit_conftest["zeilen"] == [], (
        f"tests/conftest.py hat beim Laden der .env ins ECHTE fehler.log geschrieben "
        f"({mit_conftest['echt_soll']}/fehler.log): {mit_conftest['zeilen']!r}. "
        f"Fehlt `_audit.AUDIT_DIR = _env_ablage` vor `_server._lade_env_dateien(_ROOT)`?")


def test_conftest_stellt_die_ablage_nach_dem_laden_wieder_her(mit_conftest):
    assert mit_conftest["danach"] == mit_conftest["echt"] == mit_conftest["echt_soll"], (
        f"audit.AUDIT_DIR steht nach dem conftest-Import auf {mit_conftest['danach']!r} statt auf "
        f"{mit_conftest['echt']!r}. Fehlt das Zuruecksetzen im `finally` der Umlenkung?")
