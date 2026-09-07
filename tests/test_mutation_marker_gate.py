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
