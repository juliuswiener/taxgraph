"""Import-Identitäts-Gate (Audit 2026-08-16, arch-dual-module-identity +
arch-sys-path-side-effect-ordering).

Zwei Fehlerklassen, beide unter dem ECHTEN server.py-Bootstrap geprüft (Subprozess,
NICHT pytest-sys.path — `python -m pytest` legt ROOT auf den Pfad und maskiert genau
diese Klasse; der Produktionsstart tut das nicht):

1. Doppelte Modul-Identität: `import est_mapping` (flat) + `from produkt.mapping
   import est_mapping` (package) erzeugten ZWEI Modulobjekte derselben Datei —
   Monkeypatches und lru_caches der einen Identität sind für die andere unsichtbar.
   Der conftest-Sicherheitsmechanismus ("ein Modul-Attribut-Patch fängt jeden
   Aufrufer") ist damit für das Doppel-Modul falsch.
2. Import-Reihenfolge-Abhängigkeit: der §23-Pfad resolvte nur, weil golden/runner.py
   als Import-Nebenwirkung ROOT auf sys.path legte.
"""
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# Repliziert server.py:18-27 exakt: produkt/haut + produkt/auth + produkt/store,
# ROOT NICHT auf dem Pfad ('' / cwd wird entfernt — python -c legt cwd auf sys.path[0]).
_BOOTSTRAP = """
import os, sys
sys.path = [p for p in sys.path if p not in ("", os.getcwd())]
HERE = os.path.abspath(os.path.join("produkt", "haut"))
PRODUKT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(PRODUKT, "auth"))
sys.path.insert(0, os.path.join(PRODUKT, "store"))
import api
"""


def _lauf(code: str) -> subprocess.CompletedProcess:
    return subprocess.run([sys.executable, "-c", code], cwd=ROOT,
                          capture_output=True, text=True, timeout=120)


def test_api_importiert_unter_server_bootstrap():
    r = _lauf(_BOOTSTRAP + "\nprint('OK')\n")
    assert r.returncode == 0, f"api-Import unter Server-Bootstrap bricht:\n{r.stderr}"
    assert "OK" in r.stdout


def test_keine_doppelte_modul_identitaet():
    code = _BOOTSTRAP + """
for name in ("est_mapping", "traverser", "store"):
    ids = sorted(k for k in sys.modules if k == name or k.endswith("." + name))
    assert len(ids) == 1, f"{name}: doppelte Modul-Identitaet {ids}"
print("EINDEUTIG")
"""
    r = _lauf(code)
    assert r.returncode == 0, f"Doppel-Identität oder Importfehler:\n{r.stderr}"
    assert "EINDEUTIG" in r.stdout


def test_p23_pfad_ohne_runner_seiteneffekt():
    # § 23 lief nur, weil `import runner` vorher ROOT auf sys.path legte. Nach dem Fix
    # nutzt _p23_ansonsten_einkuenfte das Modul-Level-EM — der Pfad muss OHNE vorherigen
    # runner-Import auflösen (leerer Store: Ergebnis 0, aber der Import-Weg wird betreten).
    code = _BOOTSTRAP + """
try:
    erg = api._p23_ansonsten_einkuenfte({}, {"events": [], "fall_id": "t", "version": 1}, {})
except ModuleNotFoundError as e:
    # 'No module named produkt' war der Originalfehler (Import haengt an runner-Seiteneffekt).
    # Fehlende catala-Module dagegen = Toolchain nicht gebaut -> nicht Gegenstand dieses Gates.
    if "produkt" in str(e):
        raise SystemExit(f"PRODUKT-IMPORT KAPUTT: {e}")
    print("TOOLCHAIN-ABSENT")
except Exception as e:
    print(f"FACHFEHLER-OK ({type(e).__name__})")   # Import loeste auf, Leerdaten-Fehler egal
else:
    assert isinstance(erg, int)
    print("P23-PFAD-OK")
"""
    r = _lauf(code)
    assert r.returncode == 0, f"§23-Pfad hängt weiter am runner-Seiteneffekt:\n{r.stderr}"
