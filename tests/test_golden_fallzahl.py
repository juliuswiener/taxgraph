"""Gate: der Golden-Korpus hat genau N_FAELLE Faelle -- im Verzeichnis und in dem Lauf, der ihn rechnet.

DIE LUECKE. `golden/golden_lauf.py` (`make golden`, CI-Job `golden`) kennt seine Soll-Zahl nicht:
`main()` zaehlt `golden/cases/*.yaml` selbst und druckt `<bestanden>/<alle> Faelle bestanden.` mit Exit 0,
sobald alles Gezaehlte besteht. Ein verschwundener Fall ist fuer den Lauf kein Fehler, nur ein kleinerer
Nenner. Gemessen 2026-10-04 auf 2ec08c52 mit `python3 -I golden/golden_lauf.py`: 135 Dateien -> `135/135`,
Exit 0; ein Fall weniger -> `134/134`, Exit 0; ein Fall mehr -> `136/136`, Exit 0.

DIE ZAHL STEHT DREIFACH. Jede Stelle haelt sie fuer einen anderen Lauf:
  1. rust/bescheid/tests/golden_kopf.rs  N_FAELLE  die Fixture rust/fixtures/golden_cases.json zaehlt so
     viel, und golden/cases/ zaehlt so viel wie die Fixture                                  (cargo test)
  2. rust/engine/tests/golden_werte.rs   N_FAELLE  die Fixture zaehlt so viel, bevor der Rust-Lauf jeden
     Fall rechnet                                                                            (cargo test)
  3. dieser Test                         N_FAELLE  golden/cases/ zaehlt so viel, und der Python-Lauf
     meldet so viele Faelle                                                                  (make unit)
Wer einen Fall bewusst anlegt oder streicht, zieht alle drei Zahlen im selben Commit nach und schreibt die
Fixture mit `python3 tools/parity/extract_golden.py` neu.

REICHT EINE QUELLE? Nein, solange zwei Laeufe den Korpus aus verschiedenen Dateien lesen:
  - Die Rust-Pins sehen die Fixture (golden_kopf.rs zusaetzlich das Verzeichnis). Ein verschwundener Fall
    macht `cargo test` rot, nicht aber `make golden` und den CI-Job `golden` (dort `134/134`, Exit 0);
    auf der Python-Seite faengt ihn erst dieser Test.
  - Kein Rust-Pin sieht, was `golden_lauf.py` wirklich rechnet. Wird dessen Glob enger, bleiben Verzeichnis
    und Fixture bei 135, und der Python-Lauf meldet weniger. Das faengt nur der zweite Test.
  - Dieser Test sieht die Fixture nicht; sie bleibt bei golden_kopf.rs.
Entfaellt der Python-Lauf, entfaellt dieser Test mit ihm.

Was der Test nicht prueft, steht bei N_FAELLE.
"""
from __future__ import annotations

import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

# ponytail: Die Zahl steht hart hier und in den beiden Rust-Dateien. Eine gemeinsame Quelle gaebe die
# Kreuzpruefung auf; der Test zaehlt nur. Ein Tausch bei gleicher Zahl (Fall A raus, Fall B rein) bleibt
# gruen -- die IDs gegen die Dateien prueft golden_kopf.rs (ids_sind_die_yaml_dateien_in_ihrer_reihenfolge).
# Ein Fall, den `main()` uebergeht, ohne `len(cases)` zu senken, bleibt hier ebenfalls unsichtbar.
N_FAELLE = 135

# `print(f"\n{len(cases) - len(failures)}/{len(cases)} Faelle bestanden.")` in golden_lauf.main()
SCHLUSSZEILE = re.compile(r"^(\d+)/(\d+) Faelle bestanden\.$", re.MULTILINE)


def test_das_verzeichnis_hat_genau_n_faelle():
    """Gezaehlt wird wie in `golden_lauf.main()`: `golden/cases/*.yaml`, nur diese Ebene."""
    dateien = sorted((ROOT / "golden" / "cases").glob("*.yaml"))
    assert len(dateien) == N_FAELLE, (
        f"golden/cases/ hat {len(dateien)} Faelle statt {N_FAELLE}: ein Fall fehlt oder ist dazugekommen. "
        f"Gewollt? Dann die Zahl in den DREI Dateien aus dem Kopf dieser Datei nachziehen und die Fixture "
        f"mit `python3 tools/parity/extract_golden.py` neu schreiben, alles im selben Commit.")


def test_der_python_lauf_meldet_genau_n_faelle():
    """Der echte Lauf als eigener Prozess: gepinnt wird die Zahl, die er SELBST meldet.

    Die Zahl der Dateien allein sagt nichts darueber, was `golden_lauf.py` rechnet (siehe Kopf, zweiter
    Punkt). Eigener Prozess mit `-I` wie die Gegenprobe in tests/test_ci_konfiguration.py: das Skript legt
    seine Pfade selbst, der Test liefert ihm keine Vorbedingung.

    Der Exit-Code bleibt unbeurteilt. Ob ein Fall seinen Wert verfehlt, entscheidet der Lauf selbst
    (`make golden`, CI-Job `golden`); verfehlt einer, steht dort `134/135`, und dieser Test bleibt gruen --
    er soll nur rot werden, wenn die Zahl der Faelle nicht mehr stimmt.
    """
    r = subprocess.run([sys.executable, "-I", str(ROOT / "golden" / "golden_lauf.py")],
                       capture_output=True, text=True, cwd=str(ROOT), timeout=180)
    treffer = SCHLUSSZEILE.findall(r.stdout)
    assert len(treffer) == 1, (
        f"Der Golden-Lauf hat keine Schlusszeile '<n>/<m> Faelle bestanden.' gedruckt (Exit {r.returncode}): "
        f"er ist vor dem Ende abgebrochen, etwa weil die Catala-Engine fehlt (`make build-python`), oder der "
        f"Wortlaut in golden_lauf.main() hat sich geaendert und SCHLUSSZEILE hier gehoert nachgezogen.\n"
        f"stdout (Ende): {r.stdout[-300:]!r}\nstderr (Ende): {r.stderr[-600:]!r}")
    bestanden, gesamt = (int(z) for z in treffer[0])
    assert gesamt == N_FAELLE, (
        f"Der Golden-Lauf rechnet {gesamt} Faelle statt {N_FAELLE} ({bestanden} bestanden): ein Fall fehlt, "
        f"ist dazugekommen, oder der Glob in golden_lauf.main() ist enger oder weiter geworden. "
        f"Gewollt? Dann die Zahl in den DREI Dateien aus dem Kopf dieser Datei nachziehen.")
