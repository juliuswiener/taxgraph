"""`make serve` / `make serve-python` (Cutover, REWRITE_PLAN Schritt 10): was der Produktstart tut.

`make -n` druckt die Befehle und fuehrt keinen aus (kein Bau, keine Sicherung, kein Start). Geprueft
wird, was nur ein Befehlstext belegen kann und was bei einer Aenderung still kippen wuerde:
  - der Bestand wird vor dem Start gesichert, ausser SICHERN=0;
  - Auth ist an: ein geerbtes TAXGRAPH_NO_AUTH wird abgeraeumt;
  - der Bau ist der gepruefte (dev-Profil), nicht `--release` (dort fehlen overflow-checks);
  - `serve-python` startet den Python-Dienst, den Rueckfall.
Kein Catala, kein Rust, kein Netz.
"""
import os
import subprocess

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def _trocken(*args: str) -> str:
    env = {k: v for k, v in os.environ.items() if k not in ("MAKEFLAGS", "MFLAGS")}
    r = subprocess.run(["make", "-n", *args], cwd=ROOT, env=env, capture_output=True, text=True)
    assert r.returncode == 0, r.stderr
    return r.stdout


def test_serve_sichert_den_bestand_und_baut_den_gepruefen_dienst():
    aus = _trocken("serve")
    assert "tar czf" in aus, "serve sichert den Bestand nicht (backup fehlt als Voraussetzung)"
    assert aus.index("tar czf") < aus.index("cargo build"), "die Sicherung muss vor dem Bau und Start laufen"
    assert "cargo build -p api --bin taxgraph-api" in aus
    assert "--release" not in aus, "ein Release-Bau hat keine overflow-checks und wurde nie gemessen"
    assert "env -u TAXGRAPH_NO_AUTH" in aus, "Auth muss an sein, auch wenn die Shell das Opt-out erbt"
    assert "debug/taxgraph-api" in aus


def test_sichern_null_ueberspringt_nur_die_sicherung():
    aus = _trocken("serve", "SICHERN=0")
    assert "tar czf" not in aus
    assert "debug/taxgraph-api" in aus


def test_serve_python_ist_der_rueckfall_mit_derselben_sicherung():
    aus = _trocken("serve-python")
    assert "tar czf" in aus
    assert "produkt/haut/server.py" in aus
    assert "taxgraph-api" not in aus
    assert "env -u TAXGRAPH_NO_AUTH" in aus
