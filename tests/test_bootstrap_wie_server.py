"""Das Produkt startet ohne das Repo-Wurzelverzeichnis im Pfad — geprüft in einem EIGENEN Prozess.

DER FUND (Audit 2026-08-16, arch-sys-path-side-effect-ordering): der § 23-Rechenweg löste
seinen Import nur auf, weil drei Zeilen vorher `import runner` stand und golden/runner.py beim
Import `sys.path.insert(0, ROOT)` ausführt. Ein naheliegender Aufräumschritt am Golden-Runner —
den ROOT-Insert nach main() verschieben, damit das Werkzeug isoliert importierbar wird — hätte
den Produktpfad zur Laufzeit still gebrochen, während das golden-Gate grün bleibt.

Der konkrete Auslöser ist seit dem 2026-08-18 weg (der lokale `from produkt.mapping import
est_mapping` in api.py wurde entfernt, er erzeugte ausserdem eine zweite Modul-Identität).
Diese Datei sorgt dafür, dass er nicht zurückkommt — und dass kein NEUER Import derselben Art
entsteht.

WARUM EIN EIGENER PROZESS, und warum das der ganze Punkt ist: der Audit hat auch notiert,
weshalb die Suite die Fragilität nie sah — CI ruft `python3 -m pytest`, und `python -m` stellt
das Arbeitsverzeichnis an den Anfang von sys.path. Die Testumgebung liefert genau den
Pfadeintrag, den der Produktionsstart NICHT hat. Ein Test, der im pytest-Prozess läuft, kann
diesen Fund also gar nicht finden; er wäre grün und nichts wert.

Gestartet wird deshalb wie server.py: nur produkt/haut, produkt/auth, produkt/store auf dem
Pfad, ROOT ausdrücklich NICHT — und dann ein echter Rechenweg, nicht nur ein Import.

NULL LLM. Braucht Catala (der Rechenweg ist der Zweck); ohne Toolchain übersprungen.
"""
from __future__ import annotations

import os
import subprocess
import sys
import textwrap

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)


def _catala_da() -> bool:
    try:
        sys.path.insert(0, os.path.join(ROOT, "golden"))
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _in_frischem_prozess(quelltext: str, timeout: int = 180) -> subprocess.CompletedProcess:
    """Führt `quelltext` in einem NEUEN Interpreter aus, dessen sys.path nicht von pytest
    stammt. `-I` (isoliert) hält PYTHONPATH und das Nutzer-site-Verzeichnis draussen — sonst
    schmuggelt die Umgebung des Testlaufs genau den Pfadeintrag herein, dessen Abwesenheit
    hier geprüft wird. `cwd=ROOT`, weil der Produktionsstart auch von dort kommt."""
    return subprocess.run([sys.executable, "-I", "-c", quelltext],
                          capture_output=True, text=True, timeout=timeout, cwd=ROOT)


_BOOTSTRAP = """
import os, sys
ROOT = os.getcwd()
# genau server.py:18-27 — drei Verzeichnisse, ROOT ist NICHT dabei
for teil in ("produkt/haut", "produkt/auth", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, teil))
assert ROOT not in sys.path, "ROOT liegt schon im Pfad — die Probe prüft nichts"
"""


def test_api_importiert_ohne_repo_wurzel():
    """Die schwächere Hälfte: der blosse Import. Er war auch vor dem Fix in Ordnung — hier
    festgehalten, damit ein neuer Modulebene-Import aus dem Repo-Wurzelverzeichnis auffällt,
    bevor er im Betrieb auffällt."""
    r = _in_frischem_prozess(_BOOTSTRAP + textwrap.dedent("""
        import api
        print("OK", ROOT in sys.path)
    """))
    assert r.returncode == 0, (
        f"`import api` scheitert unter dem Produktions-Bootstrap (ROOT nicht im Pfad):\n"
        f"{r.stderr[-1500:]}")
    assert r.stdout.strip() == "OK False", (
        f"unerwartete Ausgabe: {r.stdout!r} — hat etwas ROOT in den Pfad gelegt?")


@pytest.mark.skipif(not _catala_da(), reason="Catala-Toolchain nicht verfügbar")
def test_rechenweg_laeuft_ohne_repo_wurzel():
    """Die eigentliche Prüfung. Der Fund lag nicht im Import, sondern im RECHENWEG: dort stand
    der Import, der ROOT brauchte, und er lief nur, weil `import runner` zwei Zeilen vorher
    den Pfad mutiert hatte. Ein Test, der nur importiert, hätte ihn nie gesehen."""
    r = _in_frischem_prozess(_BOOTSTRAP + textwrap.dedent("""
        import tempfile
        import api, api_auth, audit
        d = tempfile.mkdtemp()
        api.FAELLE = d
        audit.AUDIT_DIR = d
        os.environ["TAXGRAPH_NO_AUTH"] = "1"
        api.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "boot"})
        status, _ = api.stand("boot")
        print("STATUS", status)
    """))
    assert r.returncode == 0, (
        f"Der Rechenweg scheitert unter dem Produktions-Bootstrap:\n{r.stderr[-2000:]}")
    assert "STATUS 200" in r.stdout, f"unerwartete Ausgabe: {r.stdout!r}\n{r.stderr[-800:]}"


def test_kein_produkt_modul_importiert_ueber_die_repo_wurzel():
    """Die statische Hälfte — und die einzige, die vollständig ist.

    WARUM SIE NÖTIG IST, gemessen am 2026-08-18: der Rechenweg-Test oben blieb GRÜN, als die
    alte fragile Zeile (`from produkt.mapping import est_mapping`) versuchsweise wieder in
    bescheid.py eingebaut wurde. Grund: `api.stand()` auf einem leeren Fall erreicht die
    betroffene Funktion nicht. Ein dynamischer Test prüft nur, was seine Fixtur anfasst, und
    kann prinzipiell nicht jeden Pfad erreichen — der Fund von damals sass in einer Funktion,
    die zwei bestimmte Zweige aufrufen.

    Diese Prüfung liest stattdessen den Quelltext: KEIN Modul unter produkt/ darf `produkt.…`
    oder ein Wurzel-Modul importieren — weder auf Modulebene noch innerhalb einer Funktion,
    denn der Fund sass ausdrücklich in einem Funktions-Import.

    Beide Hälften zusammen: die statische sieht jede Zeile, aber nicht die Laufzeit; die
    dynamische sieht die Laufzeit, aber nicht jede Zeile."""
    import ast
    import pathlib

    # Module, die im Repo-Wurzelverzeichnis liegen und deshalb ROOT im Pfad brauchen.
    wurzel_module = {p.stem for p in pathlib.Path(ROOT).glob("*.py")}

    # produkt/store/__init__.py und die Migration darunter sprechen sich selbst als PAKET an
    # (`from produkt.store.sql_backend import …`). Dieser Weg wird ausschliesslich von Tests
    # benutzt — der Produktionscode lädt store.py flach über sys.path, und beide Wege führen zu
    # verschiedenen DATEIEN (store.py vs __init__.py), nicht zu zwei Instanzen derselben.
    # Nachgemessen 2026-08-18; wird der Paketweg je in den Produktionsstart gezogen, muss er
    # hier heraus und die Importe müssen umgestellt werden.
    ausnahmen = {
        "produkt/store/__init__.py": "Paket-Init, nur über den Testweg `import produkt.store` geladen",
        "produkt/store/migrations/json_to_sql.py": "Migrationsskript, wird von Hand/Tests gefahren",
        "produkt/store/file_backend.py": "Backend-Klasse, delegiert per `import produkt.store.store`",
        "produkt/store/sql_backend.py": "Backend-Klasse, delegiert per `import produkt.store.store`",
    }

    verstoss = []
    for pfad in sorted(pathlib.Path(ROOT, "produkt").rglob("*.py")):
        if "__pycache__" in pfad.parts or not pfad.is_file():
            continue
        rel = str(pfad.relative_to(ROOT))
        if rel in ausnahmen:
            continue
        baum = ast.parse(pfad.read_text(encoding="utf-8"), filename=rel)
        for n in ast.walk(baum):        # ast.walk: auch Importe INNERHALB von Funktionen
            namen = []
            if isinstance(n, ast.Import):
                namen = [a.name.split(".")[0] for a in n.names]
            elif isinstance(n, ast.ImportFrom) and n.module and n.level == 0:
                namen = [n.module.split(".")[0]]
            for name in namen:
                if name == "produkt" or name in wurzel_module:
                    verstoss.append(f"{rel}:{n.lineno}: {name}")

    assert not verstoss, (
        "Module unter produkt/ importieren über das Repo-Wurzelverzeichnis. Das löst sich nur "
        "auf, wenn ROOT in sys.path liegt — beim Produktionsstart (server.py) tut es das NICHT, "
        "und unter pytest fällt es nie auf, weil `python -m` das Arbeitsverzeichnis "
        "voranstellt:\n  " + "\n  ".join(verstoss))


def test_ausnahmen_leben_noch():
    """Kein toter Eintrag: eine Ausnahme für eine Datei, die es nicht mehr gibt, täuscht eine
    geprüfte Entscheidung vor."""
    import pathlib
    for rel in ("produkt/store/__init__.py", "produkt/store/migrations/json_to_sql.py",
                "produkt/store/file_backend.py", "produkt/store/sql_backend.py"):
        assert (pathlib.Path(ROOT) / rel).exists(), f"Ausnahme {rel} existiert nicht mehr"


def test_die_store_doppelidentitaet_bleibt_aus_dem_produktionspfad():
    """Die Ausnahmen oben erlauben etwas, das man kennen muss: `import produkt.store.store`
    lädt DIESELBE DATEI wie das flache `import store` — unter einem zweiten Modulnamen.

    Gemessen 2026-08-18: `flach.__file__ == ueber_paket.__file__` ist True, `flach is
    ueber_paket` ist False, und ein Attribut, das auf dem einen gesetzt wird, ist auf dem
    anderen nicht sichtbar. Dieselbe Klasse, die bei est_mapping schon einmal aufgeräumt werden
    musste (Audit arch-dual-module-identity) und hinter der die fffd7c8-Lehre steht: ein Patch
    auf dem einen Modul erreicht das andere nicht.

    HEUTE FOLGENLOS, und genau das hält dieser Test fest: der Produktionsstart lädt kein
    backend-Modul (nachgemessen — nach `import api` ist keines in sys.modules), und der
    audit-Wächter in conftest.py bleibt scharf, weil store.py audit gar nicht importiert.
    Wandert der Paketweg je in den Produktionspfad, wird dieser Test rot — dann sind es zwei
    Instanzen im selben Prozess, und Monkeypatches greifen nur auf einer davon."""
    r = _in_frischem_prozess(_BOOTSTRAP + textwrap.dedent("""
        import api        # noqa: F401
        geladen = sorted(m for m in sys.modules
                         if m.startswith("produkt.") or m.endswith("_backend"))
        print("PAKETMODULE", geladen)
    """))
    assert r.returncode == 0, r.stderr[-1200:]
    assert "PAKETMODULE []" in r.stdout, (
        f"Der Produktionsstart lädt Module über den Paketweg: {r.stdout.strip()}\n"
        f"Damit liegt store.py unter zwei Modulnamen im selben Prozess — ein Monkeypatch "
        f"erreicht dann nur eine der beiden Instanzen.")


@pytest.mark.skipif(not _catala_da(), reason="Catala-Toolchain nicht verfügbar")
def test_die_probe_wuerde_den_alten_fund_finden():
    """Negativprobe: ohne sie wäre nicht belegt, dass dieser Aufbau den Fehler überhaupt
    sichtbar machen KANN — und ein Test, der seinen eigenen Fehlerfall nicht kennt, ist eine
    Behauptung.

    Nachgestellt wird genau die alte Zeile aus api.py:318, die den Fund ausmachte."""
    r = _in_frischem_prozess(_BOOTSTRAP + textwrap.dedent("""
        try:
            from produkt.mapping import est_mapping    # die alte, fragile Fassung
            print("IMPORT GELANG")
        except ModuleNotFoundError as e:
            print("ERWARTETER FEHLER", e.name)
    """))
    assert "ERWARTETER FEHLER produkt" in r.stdout, (
        f"Der Paket-Import aus dem Repo-Wurzelverzeichnis gelingt unter diesem Bootstrap — "
        f"dann prüfen die Tests darüber nichts.\nstdout={r.stdout!r}\nstderr={r.stderr[-600:]}")


@pytest.mark.skipif(not _catala_da(), reason="Catala-Toolchain nicht verfügbar")
def test_runner_legt_die_repo_wurzel_in_den_pfad():
    """Festgehalten, was HEUTE gilt — nicht als Billigung, sondern damit die Kopplung sichtbar
    bleibt: golden/runner.py mutiert beim Import sys.path. Das Produkt hängt nicht mehr daran
    (die Tests darüber beweisen es), aber der Seiteneffekt existiert weiter und macht die
    Reihenfolge von Importen bedeutsam.

    Wird der Insert entfernt, gehört dieser Test gestrichen — dann ist Phase 4 an der Stelle
    fertig. Er soll dabei auffallen, statt stillschweigend grün zu bleiben."""
    r = _in_frischem_prozess(_BOOTSTRAP + textwrap.dedent("""
        sys.path.insert(0, os.path.join(ROOT, "golden"))
        vorher = ROOT in sys.path
        import runner        # noqa: F401
        print("VORHER", vorher, "NACHHER", ROOT in sys.path)
    """))
    assert r.returncode == 0, r.stderr[-1200:]
    assert "VORHER False NACHHER True" in r.stdout, (
        f"golden/runner.py legt die Repo-Wurzel nicht mehr in den Pfad ({r.stdout!r}) — falls "
        f"das Absicht war: prüfen, dass nichts daran hing, und diesen Test streichen.")
