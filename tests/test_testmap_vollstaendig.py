"""Jede Testdatei hat eine Zeile in rust/TESTMAP.tsv.

Die Karte sagt je Testdatei, was sie prueft und was sie ersetzt oder dass sie bleibt. Eine Datei ohne
Zeile steht in keiner Zaehlung des Cutover-Plans. Der Test sucht die Dateien in `tests/` (Python) und
in `rust/*/tests/` (Rust, dazu Hilfsquellen `.py`/`.c`), nicht deren Inhalt, und braucht nichts ausser
dem Baum. Daten (yaml, txt, proptest-regressions) und `#[cfg(test)]` im Quelltext zaehlen nicht.

`python3 tests/test_testmap_vollstaendig.py` nennt die Zahl und die Liste der fehlenden Dateien.
"""
from __future__ import annotations

import glob
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
KARTE = "rust/TESTMAP.tsv"
MUSTER = ("tests/**/*.py", "rust/*/tests/**/*.rs", "rust/*/tests/**/*.py", "rust/*/tests/**/*.c")


def testdateien(wurzel: str = ROOT) -> list[str]:
    """Relative Pfade (mit `/`) aller Testdateien unter `wurzel`."""
    treffer: set[str] = set()
    for muster in MUSTER:
        for pfad in glob.glob(os.path.join(wurzel, muster), recursive=True):
            if os.path.isfile(pfad) and "__pycache__" not in pfad.split(os.sep):
                treffer.add(os.path.relpath(pfad, wurzel).replace(os.sep, "/"))
    return sorted(treffer)


def luecken(wurzel: str = ROOT, karte: str = KARTE) -> list[str]:
    """Testdateien ohne Zeile in der Karte (Spalte 1 = Pfad, Zeile 1 = Kopf)."""
    with open(os.path.join(wurzel, karte), encoding="utf-8") as f:
        eingetragen = {zeile.split("\t", 1)[0] for zeile in f.read().splitlines()[1:]}
    return [d for d in testdateien(wurzel) if d not in eingetragen]


def test_jede_testdatei_hat_eine_zeile_in_der_karte():
    fehlend = luecken()
    assert not fehlend, (
        f"{len(fehlend)} Testdateien ohne Zeile in {KARTE} (Spalten: file, lines, n_tests, xfail, "
        "category, target_module, replacing_guarantee_or_note):\n  " + "\n  ".join(fehlend)
    )


def test_der_finder_sieht_beide_seiten():
    """Ein Finder, der nichts findet, meldet keine Luecke. Er muss Python und Rust sehen."""
    dateien = testdateien()
    assert any(d.startswith("tests/test_") and d.endswith(".py") for d in dateien)
    assert any(d.startswith("rust/") and d.endswith(".rs") for d in dateien)
    assert "tests/test_testmap_vollstaendig.py" in dateien


def test_der_finder_meldet_eine_fehlende_zeile(tmp_path):
    for pfad in ("tests/test_a.py", "tests/test_b.py", "rust/x/tests/y.rs", "rust/x/tests/sub/z.rs"):
        datei = tmp_path / pfad
        datei.parent.mkdir(parents=True, exist_ok=True)
        datei.write_text("", encoding="utf-8")
    (tmp_path / "tests/__pycache__").mkdir()
    (tmp_path / "tests/__pycache__/test_a.py").write_text("", encoding="utf-8")
    (tmp_path / "tests/daten.txt").write_text("", encoding="utf-8")
    (tmp_path / "rust").mkdir(exist_ok=True)
    kopf = "file\tlines\tn_tests\txfail\tcategory\ttarget_module\tnote\n"
    zeile = "{}\t0\t0\t0\tGOLDEN\t-\tx\n"
    (tmp_path / KARTE).write_text(kopf + zeile.format("tests/test_a.py") + zeile.format("rust/x/tests/y.rs"),
                                  encoding="utf-8")
    assert luecken(str(tmp_path)) == ["rust/x/tests/sub/z.rs", "tests/test_b.py"]


if __name__ == "__main__":
    fehlend = luecken()
    print(f"{len(testdateien())} Testdateien, {len(fehlend)} ohne Zeile in {KARTE}")
    print("\n".join(fehlend))
    sys.exit(1 if fehlend else 0)
