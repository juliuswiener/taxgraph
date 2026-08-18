"""Die CI-Konfiguration selbst prüfen — sie ist der einzige Teil des Repos ohne Netz darunter.

AUSGANGSLAGE (nachgemessen 2026-08-18, nicht aus dem Audit übernommen): JEDER CI-Lauf war rot,
auch die vom selben Tag. Nicht ein Test schlug fehl — die Sammlung brach ab. Der volle Gate
scheiterte an fünf Collection-Fehlern, alle mit derselben Ursache: `No module named 'requests'`.
Ein einziges fehlendes Paket, und die 2226 Tests liefen monatelang nirgends ausser lokal.

Der schnelle Gate hatte zusätzlich 22 Fehler mit `No module named 'pkg'` — dem
Catala-Compiler-Output. Der Workflow-Kommentar sagte dazu "verifiziert gegen ALLE tests/*.py,
Stand 2026-07-21: EINE Luecke gefunden". Aus einer sind 23 geworden, und nichts hat es gemeldet.

WARUM DIESE DATEI EXISTIERT: eine kaputte CI meldet sich nicht. Sie ist rot, und Rot sieht nach
einer Weile aus wie der Normalzustand — genau das ist hier passiert. Die Suite prüft alles im
Repo ausser dem, was die Suite startet. Diese Datei schliesst die Lücke, so weit sie sich ohne
laufenden Runner schliessen lässt: sie kann nicht prüfen, ob die CI grün ist, aber sie kann
prüfen, dass die Zusagen der Konfiguration mit dem Repo übereinstimmen.

NULL LLM, kein Netz.
"""
from __future__ import annotations

import ast
import os
import pathlib
import re
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = pathlib.Path(os.path.dirname(HERE))
CI = ROOT / ".github" / "workflows" / "ci.yml"
REQ_CI = ROOT / "requirements-ci.txt"
REQ_ORACLE = ROOT / "requirements-oracle.txt"


def _workflow() -> dict:
    return yaml.safe_load(CI.read_text(encoding="utf-8"))


def _jobs() -> dict:
    return _workflow()["jobs"]


def _pakete(pfad: pathlib.Path) -> set[str]:
    """Paketnamen aus einer requirements-Datei, ohne Version und Kommentare."""
    namen = set()
    for zeile in pfad.read_text(encoding="utf-8").splitlines():
        zeile = zeile.split("#")[0].strip()
        if zeile:
            namen.add(re.split(r"[<>=!~\[]", zeile)[0].strip().lower())
    return namen


# ------------------------------------------------------------ die Ursache des roten Laufs

def test_alle_importierten_fremdpakete_stehen_im_manifest():
    """Der Fund selbst, strukturell: `requests` fehlte, und die Collection brach ab.

    Geprüft wird nicht gegen eine Liste, sondern gegen die tatsächlichen Importe im Repo —
    sonst wiederholt sich genau der Fall, in dem jemand einen Import hinzufügt und die
    Paketliste nicht kennt."""
    stdlib = set(sys.stdlib_module_names)
    # Eigene Module: JEDE .py-Datei und jedes Verzeichnis im Repo. Die erste Fassung zählte
    # nur eine Handvoll bekannter sys.path-Wurzeln auf und hielt daraufhin fünf projekteigene
    # Module (validate_xsd, linkbase, katalog, run, generate_abzinsungsfaktor) für fehlende
    # Fremdpakete. Eine Liste von Verzeichnissen ist hier dieselbe Pflegeliste wie die
    # Paketliste im Workflow, gegen die dieser Test gerade gerichtet ist.
    eigene = set()
    for p in ROOT.rglob("*"):
        if any(teil.startswith(".") or teil == "__pycache__" for teil in p.parts):
            continue
        if p.suffix == ".py":
            eigene.add(p.stem)
        elif p.is_dir():
            eigene.add(p.name)

    im_manifest = _pakete(REQ_CI)
    # Namen, unter denen ein Paket importiert wird, weichen manchmal vom Paketnamen ab.
    alias = {"pil": "pillow", "yaml": "pyyaml", "jwt": "pyjwt", "dateutil": "python-dateutil"}

    # Erzeugnisse des Catala-Übersetzers, keine PyPI-Pakete: sie entstehen erst durch
    # `clerk build p32a-python` + assemble_catala.sh unter oracle/gettsim/_catala/ und gehören
    # in kein Manifest.
    #
    # OHNE DIESE AUSNAHME URTEILT DER TEST UMGEBUNGSABHÄNGIG — gemessen am 2026-08-18 im ersten
    # CI-Lauf nach seiner Einführung: lokal liegt _catala/pkg auf der Platte, also fand die
    # Modulsuche es und verfolgte es als projekteigenes Modul weiter; in CI fehlt es vor dem
    # Build, also galt es als fehlendes Fremdpaket und der Test wurde rot. Ein Test, der
    # lokal und in CI verschieden urteilt, ist schlimmer als keiner: er lehrt, seinen roten
    # Zustand zu ignorieren, und genau davon hat diese CI schon zu viel gesehen.
    erzeugt = {"pkg", "runner", "catala_runtime", "rt"}

    # Gescannt wird die TRANSITIVE HÜLLE der Modulebene-Importe ab tests/ — genau diese, aus
    # zwei gemessenen Gründen:
    #
    # (1) NICHT NUR tests/. `requests`, dessen Fehlen die CI monatelang lahmlegte, wird von
    #     keiner Testdatei direkt importiert; es kommt über pipeline/client.py herein. Die
    #     erste Fassung scannte nur tests/ und blieb grün, als die Mutationsprobe `requests`
    #     aus dem Manifest nahm — sie konnte den Fall nicht sehen, für den sie gebaut war.
    #
    # (2) NICHT ALLES. Die zweite Fassung scannte pipeline/ vollständig und meldete fastapi
    #     und pydantic aus pipeline/ui/app.py. Die brechen nichts: tests/test_ui_backend.py
    #     importiert fastapi INNERHALB von Funktionen, und ein Funktions-Import lässt die
    #     Collection unberührt — er wird zum Skip, nicht zum Abbruch. Sie ins Manifest zu
    #     zwingen hiesse, eine schwere Entwicklungs-Abhängigkeit in jeden CI-Job zu ziehen,
    #     gegen einen Fehler, den es nicht gibt.
    #
    # Modulebene-Import bricht die Collection, Funktions-Import nicht. Das ist die Grenze,
    # und die Hülle unten bildet genau sie ab.
    def _modulebene_importe(datei: pathlib.Path) -> list[str]:
        """Modulebene-Importe bis zum ersten `pytest.importorskip(...)`.

        Der Abbruch dort ist der dritte Anlauf dieses Tests und kam wieder aus einer Messung:
        tests/test_gettsim_crosscheck.py ruft in Zeile 22 `pytest.importorskip("gettsim")` und
        importiert erst in Zeile 27 `golden_crosscheck` — das über harness.py numpy zieht.
        Ohne diese Regel meldete der Test numpy als fehlend, obwohl die Sammlung dieser Datei
        längst mit einem Skip beendet ist, bevor sie dorthin kommt.

        `importorskip` sieht nicht aus wie ein Guard (kein try/except), wirkt aber genau so:
        alles danach wird nur erreicht, wenn das genannte Paket da ist."""
        try:
            baum = ast.parse(datei.read_text(encoding="utf-8"))
        except (SyntaxError, OSError):
            return []
        namen = []
        for n in baum.body:
            if isinstance(n, ast.Expr) and isinstance(n.value, ast.Call):
                f = n.value.func
                if isinstance(f, ast.Attribute) and f.attr == "importorskip":
                    break                    # ab hier ist alles geguardet
            if isinstance(n, ast.Assign) and isinstance(n.value, ast.Call):
                f = n.value.func
                if isinstance(f, ast.Attribute) and f.attr == "importorskip":
                    break
            if isinstance(n, ast.Import):
                namen += [a.name.split(".")[0] for a in n.names]
            elif isinstance(n, ast.ImportFrom) and n.module and n.level == 0:
                namen.append(n.module.split(".")[0])
        return namen

    # Modulname -> Datei. is_file() filtert kaputte Symlinks: golden/catala_runtime.py zeigt
    # auf ein pkg/catala_runtime.py, das es nicht gibt — getrackt und tot (Audit
    # cq-broken-tracked-symlink-golden, hier beim Bau dieses Tests über die Füsse gelaufen).
    modul_datei: dict[str, pathlib.Path] = {}
    for p in ROOT.rglob("*.py"):
        if any(t.startswith(".") or t == "__pycache__" for t in p.parts) or not p.is_file():
            continue
        modul_datei.setdefault(p.stem, p)

    fehlend: dict[str, str] = {}
    offen = sorted((ROOT / "tests").glob("*.py"))
    gesehen: set[pathlib.Path] = set()
    while offen:
        datei = offen.pop()
        if datei in gesehen:
            continue
        gesehen.add(datei)
        for name in _modulebene_importe(datei):
            k = name.lower()
            if name in stdlib or name in erzeugt:
                continue
            if name in modul_datei:                 # projekteigenes Modul: weiterverfolgen
                offen.append(modul_datei[name])
                continue
            if name in eigene or k in eigene:       # Paketverzeichnis ohne gleichnamige .py
                continue
            if alias.get(k, k) in im_manifest:
                continue
            fehlend.setdefault(name, str(datei.relative_to(ROOT)))

    assert not fehlend, (
        "Testdateien importieren auf Modulebene Pakete, die nicht in requirements-ci.txt "
        "stehen — die CI-Collection bricht daran ab, so wie am 2026-08-18 an `requests`:\n  "
        + "\n  ".join(f"{p} (zuerst in {d})" for p, d in sorted(fehlend.items())))


def test_der_workflow_installiert_ueber_das_manifest():
    """Die Paketliste stand dreimal wortgleich im YAML. `requests` fehlte in allen drei
    Kopien — eine Liste, die man an drei Stellen pflegen muss, wird an null Stellen gepflegt."""
    text = CI.read_text(encoding="utf-8")
    assert "requirements-ci.txt" in text, "der Workflow installiert nicht über das Manifest"
    assert "pip install pytest pyyaml" not in text, (
        "es steht wieder eine handgepflegte Paketliste im Workflow — sie läuft dem Manifest "
        "davon, das war die Ursache des monatelang roten Laufs")


# ------------------------------------------------------------ Rechenbasis festgenagelt

def test_catala_version_ist_festgenagelt_und_passt_zum_cache_schluessel():
    """Zwei Repräsentationen derselben Zahl, die auseinanderlaufen können — dieselbe Bauart
    wie die Nähte, an denen hier schon Geld verlorenging.

    `opam install catala` (ohne Version) holte die jeweils neueste, während der Cache-Schlüssel
    eine bestimmte behauptete. Bei Cache-Treffer läuft dann eine ANDERE Version als die, die
    der Schlüssel benennt — und catala erzeugt die Rechenregeln, aus denen Steuerbeträge
    entstehen. Gemessen 2026-08-18: Schlüssel sagte 1.2.0, lokal lief 1.2.1."""
    text = CI.read_text(encoding="utf-8")
    installiert = set(re.findall(r"opam install -y catala(?:\.([0-9.]+))?", text))
    assert installiert, "kein opam-install-Aufruf für catala gefunden"
    assert "" not in installiert and None not in installiert, (
        "`opam install -y catala` ohne Version — holt die jeweils neueste, unabhängig davon, "
        "was der Cache-Schlüssel behauptet")
    assert len(installiert) == 1, f"verschiedene catala-Versionen in einem Workflow: {installiert}"
    version = installiert.pop()

    schluessel = set(re.findall(r"opam-taxgraph-catala-([0-9.]+)-ocaml", text))
    assert schluessel == {version}, (
        f"Cache-Schlüssel nennt {schluessel}, installiert wird {version} — bei Cache-Treffer "
        f"läuft eine andere Version als die, die der Schlüssel benennt.")


def _gepinnte_gettsim_version() -> str:
    zeilen = [z.split("#")[0].strip() for z in REQ_ORACLE.read_text(encoding="utf-8").splitlines()]
    gettsim = [z for z in zeilen if z.lower().startswith("gettsim")]
    assert gettsim, "gettsim steht nicht in requirements-oracle.txt"
    assert "==" in gettsim[0], (
        f"gettsim ist nicht exakt festgenagelt: {gettsim[0]!r} — bei einem Oracle ist die "
        f"Version ein Zahlenwert, kein Ablaufdetail")
    return gettsim[0].split("==", 1)[1].strip()


def test_gettsim_ist_exakt_festgenagelt():
    """Das Vergleichs-Oracle bestimmt, welche Abweichung als bekannt gilt. Eine neue Fassung
    verschiebt die Vergleichsbasis stillschweigend — schlimmstenfalls zeigt eine
    Allowlist-Zeile ins Leere und deckt fortan eine ECHTE Abweichung mit ab."""
    assert _gepinnte_gettsim_version()


def test_gepinnte_gettsim_version_ist_die_installierbare():
    """Die gepinnte Zahl muss die DISTRIBUTIONS-Version sein, nicht die des Moduls.

    Hier stand erst 1.2.1, und der CI-Lauf scheiterte mit "no version of gettsim==1.2.1". Der
    Grund: das Paket meldet zwei verschiedene Versionen. `gettsim.__version__` sagt 1.2.1, die
    Metadaten sagen 1.2 — und pip/uv kennen nur die Metadaten. Zwei Repräsentationen derselben
    Zahl, die auseinanderfallen; genau die Bauart, an der hier schon Geld verlorengegangen ist,
    diesmal in der Werkzeugkette.

    Der Test liest die INSTALLIERTE Metadaten-Version aus dem venv312 und vergleicht. Ohne
    venv312 (jeder CI-Job ausser dem Crosscheck, frischer Checkout) übersprungen — die
    Alternative wäre eine Netzabfrage bei PyPI in einer Unit-Suite."""
    venv_py = ROOT / "oracle" / ".venv312" / "bin" / "python"
    if not venv_py.exists():
        pytest.skip("oracle/.venv312 nicht vorhanden — Metadaten nicht lesbar")
    import subprocess
    ergebnis = subprocess.run(
        [str(venv_py), "-c",
         "import importlib.metadata as m; print(m.version('gettsim'))"],
        capture_output=True, text=True, timeout=30)
    if ergebnis.returncode != 0:
        pytest.skip(f"gettsim im venv312 nicht installiert: {ergebnis.stderr.strip()[:120]}")
    installiert = ergebnis.stdout.strip()
    gepinnt = _gepinnte_gettsim_version()
    assert gepinnt == installiert, (
        f"requirements-oracle.txt pinnt gettsim=={gepinnt}, installiert ist laut Metadaten "
        f"{installiert}. Wurde die Zahl aus `gettsim.__version__` abgeschrieben? Die weicht ab "
        f"— pip und uv kennen nur die Metadaten-Version, und ein Pin auf die andere lässt den "
        f"CI-Job mit 'no version of gettsim=={gepinnt}' scheitern.")


# ------------------------------------------------------------ Betrieb: Grenzen und Rechte

def test_jeder_job_hat_ein_zeitlimit():
    """Ohne timeout-minutes läuft ein hängender Job bis zum GitHub-Standard von sechs Stunden.
    Der opam-Schritt kompiliert OCaml aus dem Quelltext — genau die Sorte Schritt, die hängen
    bleibt, statt abzubrechen."""
    ohne = [name for name, job in _jobs().items() if "timeout-minutes" not in job]
    assert not ohne, f"Jobs ohne Zeitlimit: {ohne}"


def test_laeufe_werden_nicht_verdoppelt():
    """`on: push` UND `on: pull_request` lassen jeden Commit auf einem Branch mit offenem PR
    zweimal komplett durchlaufen. Ohne cancel-in-progress läuft ausserdem der überholte Lauf
    weiter, dessen Ergebnis niemanden mehr interessiert."""
    wf = _workflow()
    nebenlaeufig = wf.get("concurrency")
    assert nebenlaeufig, "kein concurrency-Block — jeder PR-Commit fährt zwei volle Läufe"
    assert nebenlaeufig.get("cancel-in-progress") is True, (
        "cancel-in-progress fehlt — überholte Läufe laufen weiter")


def test_token_darf_nur_lesen():
    """Kein Job hier schreibt ins Repository. Das Standard-Token darf es trotzdem, solange
    nichts anderes dasteht — zusammen mit Actions an beweglichen Tags (@v4) ist das der
    Unterschied zwischen 'ein Schritt liest Code' und 'ein Schritt kann Code ändern'."""
    wf = _workflow()
    rechte = wf.get("permissions")
    assert rechte, "kein permissions-Block — das Standard-Token darf schreiben"
    assert rechte.get("contents") == "read", f"contents ist nicht read-only: {rechte}"
    assert not any(v == "write" for v in rechte.values()), f"Schreibrecht erteilt: {rechte}"


def test_das_schnelle_gate_faehrt_keine_tests_ohne_toolchain():
    """Hält die Entscheidung vom 2026-08-18 fest, damit sie nicht rückgängig gemacht wird, ohne
    dass jemand die Messung dahinter kennt.

    Der Vorgänger dieses Jobs wollte die ganze Suite ohne Catala fahren. Gemessen: 1759 grün,
    221 übersprungen, 30 GESCHEITERT — und zwar nicht an fehlenden Importen (2 von 30), sondern
    mit AssertionError, weil der Code das fehlende `runner` intern abfängt und ehrlich "kein
    Wert" liefert, während der Test einen Betrag erwartet. Diese 30 sind kein Mechanismus-
    Problem; jeder von ihnen behauptet inhaltlich etwas, das nur mit Catala gilt.

    Ein Job, der 30 Fehlschläge produziert, die niemand beheben will, ist dauerhaft rot — und
    dauerhaft rot sieht nach kurzer Zeit aus wie der Normalzustand. Genau so hat diese CI
    monatelang niemandem gefehlt."""
    jobs = _jobs()
    schnell = jobs.get("sammelbarkeit")
    assert schnell, (
        "der Job `sammelbarkeit` fehlt — ohne ihn gibt es kein Signal, das ohne Toolchain "
        "verlässlich ist")
    schritte = " ".join(str(s.get("run", "")) for s in schnell["steps"])
    assert "--collect-only" in schritte, "der Sammelbarkeits-Schritt fehlt"
    voll = re.search(r"pytest\s+tests/\s+-q(?!\s*--collect-only)", schritte)
    assert not voll, (
        "der schnelle Job fährt wieder die ganze Suite ohne Toolchain — das ergibt 30 "
        "Fehlschläge, die niemand beheben will, und der Job wird dauerhaft rot.\n"
        f"gefunden: {voll.group(0) if voll else ''}")


def test_der_volle_gate_faehrt_die_ganze_suite():
    """Die Gegenrichtung: der Job MIT Toolchain muss die Suite wirklich fahren. Ohne diesen
    Test wäre die grünste Fassung der CI eine, die nur noch sammelt und nichts mehr ausführt —
    dieselbe Falle wie ein Gate, das seine eigene Voraussetzung mitbringt."""
    job = _jobs().get("catala-toolchain")
    assert job, "der volle Gate-Job fehlt"
    schritte = " ".join(str(s.get("run", "")) for s in job["steps"])
    assert re.search(r"pytest\s+tests/\s+-q", schritte), (
        "der catala-toolchain-Job fährt die Suite nicht mehr — dann prüft die CI gar keine "
        "Testergebnisse mehr, nur noch Sammelbarkeit")
    assert "--collect-only" not in schritte, (
        "der volle Job sammelt nur noch, statt auszuführen")


# ------------------------------------------------------------ der conftest-Guard selbst

def test_guard_findet_die_catala_gebundenen_dateien():
    """Der Guard misst per AST statt eine Liste zu pflegen — der Grund, warum aus einer Lücke
    23 werden konnten. Hier wird geprüft, dass die Messung überhaupt etwas findet: eine leere
    Menge wäre stillschweigend grün und der Guard wirkungslos."""
    sys.path.insert(0, HERE)
    import conftest                       # noqa: E402 — genau der Guard, der geprüft wird

    gefunden = conftest._dateien_die_catala_brauchen()
    assert len(gefunden) >= 20, (
        f"der Guard findet nur {len(gefunden)} catala-gebundene Dateien — gemessen waren es 23; "
        f"eine zu kleine Menge lässt die Collection in CI wieder abbrechen")
    assert all(n.startswith("test_") and n.endswith(".py") for n in gefunden)


def test_guard_greift_nur_ohne_toolchain():
    """Die andere Richtung, und die wichtigere: mit verfügbarer Toolchain darf NICHTS
    übersprungen werden. Ein Guard, der immer greift, versteckt die halbe Suite."""
    sys.path.insert(0, HERE)
    import conftest                       # noqa: E402

    if conftest._catala_fehlt():
        pytest.skip("ohne Catala-Toolchain — diese Richtung ist hier nicht prüfbar")
    assert conftest.collect_ignore == [], (
        f"Catala ist verfügbar, aber {len(conftest.collect_ignore)} Dateien werden trotzdem "
        f"übersprungen — der Guard greift zu früh und versteckt echte Tests")
