"""Mutation-Marker-Gate — Backlog mutation-marker-gate-fehlt.md, Entscheidung
zwei-ratschen-bei-null-angefangen.md (Julius 2026-08-26).

Ein früherer Fund forderte ein Gate gegen `MUTATION`-Marker im ausgelieferten Code —
gemeint sind Reste von Mutationsproben (Testbau), die still liegen bleiben. Gefordert,
nie gebaut. Gemessen 2026-08-26 an 31be4a7: 0 Treffer, aber nichts hielt sie leer.

Bauform laut Entscheidung: reiner Zeichenkettenscan, nur unter produkt/, KEINE
Ausnahmeliste — bei null Markern kostet die Wache nichts, bis jemand einen hinzufügt.
Braucht wirklich jemand eine Ausnahme, ist DAS der Beleg für einen Fristmechanismus
(eigene Entscheidung), keine Zeile in einer Liste. Bewusst eng (nur `MUTATION`, nur
produkt/) — `FIXME`/`XXX`/`scripts/` ausdrücklich verworfen, s. Entscheidung.
"""
from __future__ import annotations

import os
import pathlib

ROOT = pathlib.Path(os.path.dirname(os.path.abspath(__file__))).parent
PRODUKT = ROOT / "produkt"


def test_kein_mutation_marker_unter_produkt():
    """`MUTATION` darf nirgends in produkt/*.py liegen bleiben — kein Ausnahmefall."""
    treffer = []
    for pfad in sorted(PRODUKT.rglob("*.py")):
        if "__pycache__" in str(pfad):
            continue
        for zeilennr, zeile in enumerate(pfad.read_text(encoding="utf-8").splitlines(), start=1):
            if "MUTATION" in zeile:
                treffer.append(f"{pfad.relative_to(ROOT)}:{zeilennr}: {zeile.strip()}")
    assert not treffer, (
        "MUTATION-Marker unter produkt/ gefunden — Baustellen-Rest im ausgelieferten Code, "
        "nicht in tests/ verschoben oder aufgeräumt:\n  " + "\n  ".join(treffer))


def test_scan_menge_ist_nicht_leer_und_enthaelt_reale_module():
    """Selbstprüfung des Scans: PRODUKT.rglob("*.py") darf nicht leer sein und muss mindestens
    die zum Zeitpunkt dieses Tests bekannten realen Module unter produkt/ enthalten. Ohne diese
    Zusicherung wäre test_kein_mutation_marker_unter_produkt bei einem stumm kollabierten Scan
    (verschobenes produkt/-Verzeichnis, falscher PRODUKT-Pfad) grün, obwohl er nichts geprüft
    hätte — bei null Markern kostet die Wache nichts, aber sie muss auch wirklich etwas
    scannen (Muster: test_llm_import_boundary.py::
    test_scan_menge_ist_nicht_leer_und_enthaelt_reale_module)."""
    gescannt = {p.relative_to(ROOT) for p in PRODUKT.rglob("*.py") if "__pycache__" not in str(p)}
    UNTERGRENZE = 30  # gemessen 2026-09-08: 36 .py-Dateien unter produkt/
    assert len(gescannt) >= UNTERGRENZE, (
        f"Nur {len(gescannt)} .py-Dateien unter produkt/ gefunden, erwartet mindestens "
        f"{UNTERGRENZE} — die Scan-Menge ist unerwartet klein geworden"
    )
    bekannte_module = {pathlib.Path("produkt/haut/api.py"), pathlib.Path("produkt/store/store.py")}
    fehlend = bekannte_module - gescannt
    assert not fehlend, f"Bekannte Module fehlen in der Scan-Menge: {sorted(fehlend)}"
