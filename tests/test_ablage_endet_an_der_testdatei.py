"""Eine Umlenkung von Falldaten und Pruefprotokoll endet an der Grenze ihrer Testdatei.

Befund 2026-10-01: fuenf Modul-Fixtures setzten api.FAELLE und audit.AUDIT_DIR per Zuweisung
und nie zurueck. pytest-randomly mischt die Dateien, und jede spaetere Datei im selben Prozess
sah das Temp-Verzeichnis des Verursachers -- test_datenwurzel_ausserhalb_repo.py war in 5 von
8 Laeufen rot. Das Ruecksetzen sitzt EINMAL in tests/conftest.py (_ablage_endet_an_der_testdatei).

Dieser Waechter legt die Reihenfolge fest, in der das Leck beisst: erst die Attrappe
tests/_ablage_leck_probe.py (lenkt um, setzt nie zurueck), dann der Pruefer unten -- in einem
eigenen pytest-Prozess, ohne Zufallsreihenfolge und ohne xdist. So haengt das Ergebnis weder
von der Reihenfolge des aeusseren Laufs ab noch davon, ob die fuenf Verursacher noch lecken.
"""
from __future__ import annotations

import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
for _sub in ("produkt/haut", "produkt/store"):
    sys.path.insert(0, str(ROOT / _sub))

import api                                # noqa: E402
import audit                              # noqa: E402
from _ablage_leck_probe import SENTINEL   # noqa: E402


def test_ablage_ist_nicht_die_probe():
    """Im Unterprozess laeuft dieser Test NACH der Attrappe; im normalen Lauf gilt er trivial."""
    assert SENTINEL not in (audit.AUDIT_DIR, api.FAELLE), (
        f"Die Umlenkung aus tests/_ablage_leck_probe.py lebt in einer fremden Testdatei weiter: "
        f"audit.AUDIT_DIR={audit.AUDIT_DIR!r}, api.FAELLE={api.FAELLE!r}. "
        f"Setzt _ablage_endet_an_der_testdatei in tests/conftest.py beide zurueck?"
    )


def test_umlenkung_endet_an_der_testdatei():
    # Ohne PYTEST_ADDOPTS: ein `-n` von aussen verteilte Attrappe und Pruefer auf zwei Prozesse,
    # und der Pruefer waere gruen, ohne je hinter der Attrappe gelaufen zu sein.
    umgebung = {k: v for k, v in os.environ.items() if k != "PYTEST_ADDOPTS"}
    lauf = subprocess.run(
        [sys.executable, "-m", "pytest", "-q", "-p", "no:randomly", "-p", "no:cacheprovider",
         "tests/_ablage_leck_probe.py",
         "tests/test_ablage_endet_an_der_testdatei.py::test_ablage_ist_nicht_die_probe"],
        cwd=ROOT, env=umgebung, capture_output=True, text=True, timeout=120,
    )
    # "2 passed", nicht nur der Exit-Code: ein Lauf, der die Attrappe nicht sammelt, waere
    # ebenfalls gruen -- und haette nichts geprueft.
    assert lauf.returncode == 0 and "2 passed" in lauf.stdout, (
        f"rc={lauf.returncode}\n{lauf.stdout[-3000:]}\n{lauf.stderr[-2000:]}"
    )
