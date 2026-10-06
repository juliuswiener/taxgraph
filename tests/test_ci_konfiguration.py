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
import tomllib

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


def _versteckt(p: pathlib.Path, root: pathlib.Path = ROOT) -> bool:
    """Punkt-Verzeichnis oder __pycache__ INNERHALB des Repos, geprüft am Pfad relativ zur
    Wurzel. Am absoluten Pfad fiel unter jedem Punkt-Verzeichnis (~/.cache/…,
    .claude/worktrees/…) JEDE Datei heraus: gemessen 2026-09-26 0 statt 496 eigene Module,
    und 55 projekteigene Module galten als fehlende Fremdpakete. Beide Filter unten brauchen
    die Regel: nur mit dem ersten blieb der Test grün, auch ohne `requests` im Manifest.

    `root` ist ein Parameter, damit ein Test die Regel gegen eine ANDERE Wurzel stellen kann —
    die historische Falle war ein Repo, das selbst unter einem Punkt-Verzeichnis lag."""
    return any(t.startswith(".") or t == "__pycache__" for t in p.relative_to(root).parts)


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
        if _versteckt(p):
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

    # Gescannt wird die TRANSITIVE HÜLLE der Modulebene-Importe ab tests/ und pipeline/tests/ — genau diese, aus
    # zwei gemessenen Gründen:
    #
    # (1) NICHT NUR tests/. `requests`, dessen Fehlen die CI monatelang lahmlegte, wird von
    #     keiner Testdatei direkt importiert; es kommt über pipeline/client.py herein. Die
    #     erste Fassung scannte nur tests/ und blieb grün, als die Mutationsprobe `requests`
    #     aus dem Manifest nahm — sie konnte den Fall nicht sehen, für den sie gebaut war.
    #
    # (2) NICHT ALLES. Die zweite Fassung scannte pipeline/ vollständig und meldete fastapi
    #     und pydantic aus pipeline/ui/app.py. Die brechen nichts: pipeline/tests/test_ui_backend.py
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
        pipeline/tests/test_gettsim_crosscheck.py ruft in Zeile 22 `pytest.importorskip("gettsim")` und
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
        if _versteckt(p) or not p.is_file():
            continue
        modul_datei.setdefault(p.stem, p)

    fehlend: dict[str, str] = {}
    # pipeline/tests/ seit 2026-10-06: die Stufe-B-Tests (CI-Job `stufe-b`) liegen nicht mehr unter tests/.
    offen = sorted((ROOT / "tests").glob("*.py")) + sorted((ROOT / "pipeline" / "tests").glob("*.py"))
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


def _gepinnte_version(paket: str) -> str:
    zeilen = [z.split("#")[0].strip() for z in REQ_ORACLE.read_text(encoding="utf-8").splitlines()]
    treffer = [z for z in zeilen if re.split(r"[<>=!~\[]", z)[0].strip().lower() == paket]
    assert treffer, f"{paket} steht nicht in requirements-oracle.txt"
    gepinnt = re.fullmatch(rf"{re.escape(paket)}\s*==\s*([\w.+!-]+)", treffer[0], re.I)
    assert gepinnt, (
        f"{paket} ist nicht exakt festgenagelt: {treffer[0]!r} — bei einem Oracle ist die "
        f"Version ein Zahlenwert, kein Ablaufdetail")
    return gepinnt.group(1)


def test_gettsim_ist_exakt_festgenagelt():
    """Das Vergleichs-Oracle bestimmt, welche Abweichung als bekannt gilt. Eine neue Fassung
    verschiebt die Vergleichsbasis stillschweigend — schlimmstenfalls zeigt eine
    Allowlist-Zeile ins Leere und deckt fortan eine ECHTE Abweichung mit ab.

    Dasselbe gilt für ttsim-backend, das Rechenwerk unter GETTSIM: gettsim 1.2 verlangt es nur
    nach unten begrenzt, und die Fassung vom 2026-08-20 liess den Crosscheck ohne Änderung bei
    uns rot werden. Hier prüfte erst nur die gettsim-Zeile — ein offenes `>=` beim Rechenwerk
    blieb grün."""
    for paket in ("gettsim", "ttsim-backend"):
        assert _gepinnte_version(paket)


# Ein Versions-Spezifizierer an gettsim/ttsim-backend, z.B. `gettsim==1.2` oder `ttsim-backend<1.3`.
# Die Ziffer danach ist Pflicht: ohne sie traf `echo "=== install gettsim ==="` (gemessen).
# ponytail: eine Version aus einer Variable (`gettsim==$V`) sieht das Muster nicht; reicht, solange
# kein Weg so installiert — sonst die Variable mitprüfen.
_NEBEN_PIN = re.compile(r"\b(gettsim|ttsim[-_.]backend)\s*(===?|[<>!~]=|[<>])\s*\d", re.I)


def test_kein_installationsweg_nennt_eine_eigene_oracle_version():
    """Die Pins standen an DREI Orten — requirements-oracle.txt, docs/setup.md und
    scripts/install-gettsim.sh —, und nichts hielt sie gleich. Der Doku-Weg zog deshalb eine
    andere GETTSIM-Fassung als die CI und scheiterte mit einem anderen Fehler. Jetzt installiert
    jeder Weg `-r requirements-oracle.txt` wie ci.yml; eine Version steht nur noch dort."""
    wege = [CI, REQ_CI, ROOT / "docs" / "setup.md", ROOT / "Makefile",
            *sorted((ROOT / "scripts").glob("*.sh"))]
    neben = [f"{p.relative_to(ROOT)}:{n}: {z.strip()}"
             for p in wege
             for n, z in enumerate(p.read_text(encoding="utf-8").splitlines(), 1)
             if _NEBEN_PIN.search(z)]
    assert not neben, (
        "Version für gettsim/ttsim-backend ausserhalb von requirements-oracle.txt — dieser Weg "
        "installiert sonst eine andere Vergleichsbasis als die CI. Statt eines eigenen Pins "
        "`-r requirements-oracle.txt` installieren:\n  " + "\n  ".join(neben))


def test_gepinnte_gettsim_version_ist_die_installierbare():
    """Die gepinnte Zahl muss die DISTRIBUTIONS-Version sein, nicht die des Moduls.

    Hier stand erst 1.2.1, und der CI-Lauf scheiterte mit "no version of gettsim==1.2.1". Der
    Grund: das Paket meldet zwei verschiedene Versionen. `gettsim.__version__` sagt 1.2.1, die
    Metadaten sagen 1.2 — und pip/uv kennen nur die Metadaten. Zwei Repräsentationen derselben
    Zahl, die auseinanderfallen; genau die Bauart, an der hier schon Geld verlorengegangen ist,
    diesmal in der Werkzeugkette.

    Der Test liest die INSTALLIERTE Metadaten-Version aus dem venv312 und vergleicht. Ohne
    venv312 (jeder CI-Job ausser dem Crosscheck, frischer Checkout) übersprungen — die
    Alternative wäre eine Netzabfrage bei PyPI in einer Unit-Suite.

    Dasselbe gilt für ttsim-backend, das Rechenwerk unter GETTSIM, seit es gepinnt ist. Hier
    prüfte erst nur gettsim — ein ttsim-backend-Pin neben der installierten Fassung blieb grün
    (gemessen 2026-10-02 mit verstelltem Pin 1.2 statt 1.2.1)."""
    venv_py = ROOT / "oracle" / ".venv312" / "bin" / "python"
    if not venv_py.exists():
        pytest.skip("oracle/.venv312 nicht vorhanden — Metadaten nicht lesbar")
    import subprocess
    for paket in ("gettsim", "ttsim-backend"):
        ergebnis = subprocess.run(
            [str(venv_py), "-c",
             f"import importlib.metadata as m; print(m.version({paket!r}))"],
            capture_output=True, text=True, timeout=30)
        if ergebnis.returncode != 0:
            pytest.skip(f"{paket} im venv312 nicht installiert: {ergebnis.stderr.strip()[:120]}")
        installiert = ergebnis.stdout.strip()
        gepinnt = _gepinnte_version(paket)
        assert gepinnt == installiert, (
            f"requirements-oracle.txt pinnt {paket}=={gepinnt}, installiert ist laut Metadaten "
            f"{installiert}. Wurde die Zahl aus dem `__version__` des Moduls abgeschrieben? Die "
            f"weicht ab — pip und uv kennen nur die Metadaten-Version, und ein Pin auf die andere "
            f"lässt den CI-Job mit 'no version of {paket}=={gepinnt}' scheitern.")


# ------------------------------------------------------------ Rust: Optimierungsstufe der Tests

RUST_WURZEL = ROOT / "rust" / "Cargo.toml"


def _rust_dev_profil() -> dict:
    return tomllib.loads(RUST_WURZEL.read_text(encoding="utf-8")).get("profile", {}).get("dev", {})


def test_rust_tests_laufen_mit_opt_level_1_und_pruefungen_bleiben_an():
    """Ohne `opt-level = 1` laufen die Rust-Parity-Läufe um den Faktor 2 bis 5 langsamer (Zahlen
    im Kommentar in rust/Cargo.toml). Die Stufe darf nicht still verschwinden — und sie darf
    die Laufzeitprüfungen nicht mitnehmen: `debug-assertions` und `overflow-checks` fangen
    Überläufe und verletzte Annahmen in Geldrechnung. Cargo schaltet sie bei opt-level 1
    nicht von selbst aus; ein `= false` im Profil wäre eine ausdrückliche Entscheidung."""
    dev = _rust_dev_profil()
    assert dev.get("opt-level") == 1, (
        f"[profile.dev] opt-level ist {dev.get('opt-level')!r}, nicht 1 — die Parity-Läufe "
        f"werden wieder 2- bis 5-mal so langsam, und der rust-Job der CI baut ohne die Stufe")
    for schalter in ("debug-assertions", "overflow-checks"):
        assert dev.get(schalter, True) is True, (
            f"[profile.dev] {schalter} ist ausgeschaltet — Überlauf und verletzte Annahmen "
            f"in der Geldrechnung bleiben in den Tests unbemerkt")


def test_der_rust_cache_schluessel_kennt_die_optimierungsstufe():
    """Zwei Repräsentationen derselben Einstellung, die auseinanderlaufen können — dieselbe
    Bauart wie der catala-Cache-Schlüssel oben.

    Der rust-Job cacht `rust/target` unter einem Schlüssel. Steht darin nur `Cargo.lock`, kennt
    der Schlüssel das Profil nicht: nach einer Änderung der Stufe findet der Job den alten,
    mit opt-level 0 gebauten Cache, cargo baut alles neu (die Stufe steckt im Hash jedes
    Artefakts), und bei einem Treffer schreibt actions/cache nie neu. Der Cache bliebe für
    immer wertlos, und jeder Lauf zahlte Herunterladen UND vollen Neubau."""
    schritte = _jobs()["rust"]["steps"]
    caches = [s for s in schritte if str(s.get("uses", "")).startswith("actions/cache@")]
    assert len(caches) == 1, f"der rust-Job hat {len(caches)} Cache-Schritte statt einem"
    cache = caches[0]
    assert "rust/target" in str(cache["with"]["path"]), "der rust-Job cacht rust/target nicht mehr"
    schluessel = str(cache["with"]["key"])
    assert "rust/Cargo.toml" in schluessel, (
        f"der Cache-Schlüssel {schluessel!r} hängt nicht an rust/Cargo.toml, wo das Profil steht — "
        f"nach einer Änderung der Optimierungsstufe bleibt der alte Cache stehen")


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


def test_eric_skip_greift_nur_mit_dem_flag():
    """Die Regel vom 2026-10-01 (Log #142), festgenagelt: fehlendes ERiC-Schema ist LAUT ROT.

    Uebersprungen wird nur, wenn TAXGRAPH_OHNE_XSD=1 ausdruecklich gesetzt ist. Der Grund steht
    in conftest: ein stiller Skip macht die ELSTER-Pruefung fuer immer unsichtbar, weil das
    Schema nie im Repo liegt und CI es nie hat. Bis 2026-10-01 pinnte dieser Test die alte
    Regel (ohne Schema IMMER Skip).

    Die Vorbedingung ERIC_SCHEMA_FEHLT wird GESETZT, nicht gelesen: sonst prueft der Test auf
    jedem Rechner mit Schema nur die eine Haelfte und in CI nur die andere. Ein Skip darf nicht
    aus dem Test entweichen: in CI (Flag=1) meldete sich der Test sonst selbst als
    uebersprungen, genau dann, wenn das Gate kaputt ist."""
    sys.path.insert(0, HERE)
    import conftest                       # noqa: E402

    class _XmlFehler(Exception):
        pass
    _XmlFehler.__name__ = "XmlFehler"     # der Hook prüft den Typnamen, nicht die Klasse
    echte = _XmlFehler("E10-2025.xsd nicht gefunden — $ERIC_DIR setzen / ERiC-Doku entpacken.")

    def ausgang(schema_fehlt, flag, fehler=echte):
        with pytest.MonkeyPatch.context() as mp:
            mp.setattr(conftest, "ERIC_SCHEMA_FEHLT", schema_fehlt)
            if flag is None:
                mp.delenv("TAXGRAPH_OHNE_XSD", raising=False)
            else:
                mp.setenv("TAXGRAPH_OHNE_XSD", flag)
            try:
                conftest._fehlendes_schema_ueberspringen(fehler)
            except pytest.skip.Exception:
                return "skip"
            except AssertionError:
                return "rot"
            return "durch"

    for flag in (None, "1"):
        assert ausgang(False, flag) == "durch", (
            "das Schema liegt vor, aber das Gate hat die echte ERiC-Meldung angefasst — so "
            "verschwindet die ELSTER-Pruefung lautlos, und ein echter XSD-Fehler saehe aus wie "
            "eine fehlende Lizenzdatei")
    assert ausgang(True, None) == "rot", (
        "ohne TAXGRAPH_OHNE_XSD kam kein lautes Rot — der stille Skip ist zurueck")
    assert ausgang(True, "1") == "skip", "mit TAXGRAPH_OHNE_XSD=1 wurde nicht uebersprungen"
    # Nur "1" zaehlt, wie auf der Rust-Seite (elster::testhilfe::schemas_da).
    for wert in ("true", "yes", "0", ""):
        assert ausgang(True, wert) == "rot", f"TAXGRAPH_OHNE_XSD={wert!r} hat uebersprungen"
    # Ein inhaltlicher XmlFehler wird nie angefasst, auch mit Flag: der Aufrufer reicht die
    # ORIGINAL-Ausnahme durch.
    assert ausgang(True, "1", _XmlFehler("Pflichtfeld E0100082 fehlt im Container")) == "durch", (
        "ein inhaltlicher XmlFehler wurde angefasst — er muss unveraendert durchkommen")


def test_eric_skip_frisst_keine_fremden_fehler():
    """Die Gegenprobe zur Weite des Musters: ein anderer XmlFehler und eine andere Ausnahme mit
    derselben Meldung dürfen NICHT übersprungen werden. Ein Hook, der jede Ausnahme in einen
    Skip verwandelt, macht die Suite grün, indem er sie abschaltet."""
    sys.path.insert(0, HERE)
    import conftest                       # noqa: E402

    class _XmlFehler(Exception):
        pass
    _XmlFehler.__name__ = "XmlFehler"

    assert not conftest._ist_fehlendes_eric_schema(
        _XmlFehler("Pflichtfeld E0100082 fehlt im Container")), \
        "ein inhaltlicher XmlFehler wird übersprungen — das versteckt echte Abgabe-Blocker"
    assert not conftest._ist_fehlendes_eric_schema(
        ValueError("E10-2025.xsd nicht gefunden — $ERIC_DIR setzen")), \
        "eine beliebige Ausnahme mit der Meldung wird übersprungen — zu weit gefasst"

    # AssertionError IST zugelassen, aber nur mit der Lizenz-Meldung: 12 Tests fangen den
    # Fehler mit pytest.raises(match=...) ab, dort kommt nie ein XmlFehler beim Test an.
    # Ein gewöhnlicher Assertion-Fehler darf davon nicht profitieren.
    assert not conftest._ist_fehlendes_eric_schema(
        AssertionError("erwartet 4711, war 0")), \
        "ein gewöhnlicher AssertionError wird übersprungen — so wird jede rote Zahl grün"


def test_skip_grund_erkennt_das_schema_und_nichts_sonst():
    """Die zweite Haelfte des Gates: Skips, die eine TESTDATEI selbst setzt.

    Ohne Schema entstehen 47 solche Skips aus `@pytest.mark.skipif(not _schema_da)`. Sie
    entstehen, bevor Code laeuft — kein Ausnahme-Hook sieht sie. `_skip_grund_ist_schema`
    erkennt sie am Grund, `pytest_runtest_makereport` macht sie ohne Flag rot."""
    sys.path.insert(0, HERE)
    import conftest                       # noqa: E402

    erkannt = [
        "E10-2025.xsd oder xmllint fehlt — XSD-Gate nicht lauffähig",
        "lokales ERiC-E10-2025.xsd nicht gefunden ($ERIC_DIR/~/02_Software/eric)",
        "E10-2025.xsd nicht gefunden — $ERIC_DIR setzen",
        "ERiC-Schemaverzeichnis nicht vorhanden ($ERIC_DIR / ~/02_Software/eric)",
        "elster11_E10_2025_extern.xsd nicht gefunden",
    ]
    for grund in erkannt:
        assert conftest._skip_grund_ist_schema(grund), f"nicht erkannt: {grund!r}"

    # Die Hersteller-ID ist ein ANDERES fehlendes Stueck und darf hier nicht mitlaufen: sie
    # haengt an ihrem eigenen Muster und wird von diesem Gate nicht angefasst.
    nicht_erkannt = [
        "ERiC oder Hersteller-ID fehlt — amtliche Pruefung nicht lauffaehig "
        "(credential-freies CI)",
        "keine Eingangsfrage vorhanden",
        "oracle/.venv312 nicht vorhanden — Metadaten nicht lesbar",
        "bescheid_deklaration.py:837-838 (gewinn_quelle_offen) prueft _positiv('einkuenfte')",
    ]
    for grund in nicht_erkannt:
        assert not conftest._skip_grund_ist_schema(grund), f"falsch erkannt: {grund!r}"


def test_hersteller_id_skip_ist_genauso_eng():
    """Zweiter Fall derselben Art (neun Tests im CI-Lauf danach): die Hersteller-ID ist ein
    Geheimnis aus der gitignored .env und fehlt in CI zurecht. Gleiche Enge wie beim Schema —
    und dieselbe wichtigere Richtung: ist sie gesetzt, darf nichts übersprungen werden.

    Die Bedingung wird zur LAUFZEIT gelesen, damit ein Test, der die Variable per monkeypatch
    entfernt, um die fail-closed-Antwort zu prüfen, weiterhin seinen Fehler bekommt."""
    sys.path.insert(0, HERE)
    import conftest                       # noqa: E402

    class _XmlFehler(Exception):
        pass
    _XmlFehler.__name__ = "XmlFehler"
    echte = _XmlFehler("keine Hersteller-ID — $ELSTER_HERSTELLER_ID setzen "
                       "(nie im Repo, nie im Code).")

    if conftest._hersteller_id_fehlt():
        assert conftest._ist_fehlende_hersteller_id(echte)
    else:
        assert not conftest._ist_fehlende_hersteller_id(echte), (
            "die Hersteller-ID ist gesetzt, der Hook würde trotzdem überspringen")

    # Enge in beide Richtungen, unabhängig vom Umgebungszustand:
    assert not conftest._ist_fehlende_hersteller_id(
        _XmlFehler("Pflichtfeld E0100082 fehlt im Container")), \
        "ein inhaltlicher XmlFehler wird übersprungen"
    assert not conftest._ist_fehlende_hersteller_id(
        ValueError("keine Hersteller-ID — $ELSTER_HERSTELLER_ID setzen")), \
        "eine beliebige Ausnahme mit der Meldung wird übersprungen — zu weit gefasst"


# ---- Umgebungslecks beim Testimport: Waechter und seine Selbstratsche ------------------------
#
# Der Scan steht als Modulfunktion, nicht in der Testfunktion: die Selbstratsche darunter muss
# DIESELBE Menge sehen wie der Waechter. Eine eigene Kopie der Schleife waere genau der blinde
# Fleck, den sie schliessen soll — der Waechter koennte leer laufen und die Kopie bliebe voll.

def _literal(wert) -> str | None:
    """Der Zeichenketten-Literal, wenn `wert` einer ist, sonst None.

    None heisst "nicht statisch vergleichbar" — `os.environ[...] = None` waere zur Laufzeit
    ein TypeError, kann also nicht mit einem echten Wert verwechselt werden."""
    return wert.value if isinstance(wert, ast.Constant) and isinstance(wert.value, str) else None


def _env_ziel(knoten):
    """(Name, Zeilennummer, gesetzter Wert) fuer eine Modulebene-Schreiboperation, sonst None.

    Der Wert kam am 2026-09-26 dazu: der Waechter prueft damit nicht mehr nur den NAMEN.
    Ein erlaubter Name ist kein erlaubter Wert — `TAXGRAPH_NO_AUTH = "0"` schaltet die
    Authentifizierung wieder ein und war unter der Namenspruefung unsichtbar."""
    if isinstance(knoten, ast.Assign):
        for t in knoten.targets:
            if (isinstance(t, ast.Subscript) and isinstance(t.value, ast.Attribute)
                    and t.value.attr == "environ"
                    and isinstance(t.value.value, ast.Name) and t.value.value.id == "os"):
                s = t.slice
                if isinstance(s, ast.Constant) and isinstance(s.value, str):
                    return s.value, knoten.lineno, _literal(knoten.value)
    if isinstance(knoten, ast.Expr) and isinstance(knoten.value, ast.Call):
        a = knoten.value.func
        if (isinstance(a, ast.Attribute) and a.attr in ("setdefault", "setenv", "putenv")
                and isinstance(a.value, ast.Attribute) and a.value.attr == "environ"
                and isinstance(a.value.value, ast.Name) and a.value.value.id == "os"
                and knoten.value.args
                and isinstance(knoten.value.args[0], ast.Constant)
                and isinstance(knoten.value.args[0].value, str)):
            # setdefault(name, wert) — der Wert ist das ZWEITE Argument. putenv traegt ihn im
            # ersten ("X=1") und ist kein environ-Verfahren; hier bleibt er darum None.
            zweites = knoten.value.args[1] if len(knoten.value.args) > 1 else None
            return knoten.value.args[0].value, knoten.value.lineno, _literal(zweites)
    return None


def _gescannte_testdateien() -> list[pathlib.Path]:
    """Die Menge, die der Waechter liest: jede Testdatei unter tests/."""
    return sorted((ROOT / "tests").glob("test_*.py"))


def _erlaubte_conftest_variablen() -> dict[str, str | None]:
    """Was conftest auf Modulebene setzt, ist Sitzungs-Eigentum — nachsetzen ist wirkungslos.

    Rueckgabe Name -> Wert, und zwar der WERT AUS DER CONFTEST, nicht ein Literal hier. Ein
    Literal im Test waere eine zweite Wahrheit: aendert conftest seine `TAXGRAPH_NO_AUTH`, waere
    das Nachsetzen mit dem alten Wert sofort ein Treffer, obwohl es gleichwertig ist. Der
    erwartete Wert ist der, den die Sitzung tatsaechlich hat."""
    conftest_baum = ast.parse((ROOT / "tests" / "conftest.py").read_text(encoding="utf-8"))
    return {r[0]: r[2] for k in conftest_baum.body if (r := _env_ziel(k))}


def test_kein_testmodul_veraendert_die_umgebung_beim_import():
    """Ein Testmodul, das beim IMPORT die Umgebung setzt, veraendert fremde Tests.

    pytest importiert beim Sammeln jede Datei, bevor der erste Test laeuft — und unter
    `-n 6 --dist loadfile` importiert jeder Worker ALLE ihm zugeteilten Dateien. Ein
    `os.environ[...] = ...` auf Modulebene gilt damit fuer fremde Dateien im selben Worker.

    Gemessen 2026-09-26: vier Module setzten `ELSTER_HERSTELLER_ID` auf einen Platzhalter.
    Folge in test_checkest_feldmatrix.py: die ERiC-Tests hielten sich fuer lauffaehig, liefen
    mit dem Platzhalter gegen ERiC und scheiterten mit rc=610301200 statt zu skippen —
    `make unit` war rot, und der rote Test (test_p35a_einzelaufstellung...) sah nach einem
    §35a-Fehler aus, war aber ein Umgebungsleck. Der Fix war eine modulgescopte Fixture.

    ERLAUBT: nur Variablen, die conftest selbst auf Modulebene setzt, UND NUR MIT DEM WERT, DEN
    CONFTEST SETZT — `TAXGRAPH_NO_AUTH` ist der einzige solche Fall. Alles andere (eine
    Hersteller-ID, ein Schluessel, ein Pfad) gehoert in eine Fixture.

    Erlaubt bleibt der Import in einer Funktion (dort laeuft er beim Test, nicht beim Sammeln).

    WAS DIESER WAECHTER WEITERHIN NICHT SIEHT — alles am 2026-09-26 ueber alle 283 Dateien
    gemessen, jede Zeile heute 0 Fundstellen, also heute kein Schaden, aber auch keine Deckung:
    1. `os.environ.pop(...)`, `.update(...)`, `.clear()`, `del os.environ[...]` — ein Modul
       koennte eine Variable ENTFERNEN; geprueft werden nur Zuweisungen und die drei
       Setz-Aufrufe in `_env_ziel`.
    2. Schreibzugriffe NICHT direkt auf Modulebene (in `if`/`try`/`with`) — der Scan liest
       `baum.body`, also nur die oberste Ebene. Zur Laufzeit des Imports feuern sie trotzdem.
    3. `from os import environ` als Alias — `_env_ziel` sucht `os.environ`.
    4. Ein Wert, der erst zur Laufzeit entsteht (`os.environ[x] = f(...)`): der Vergleich
       bekommt None und meldet. Das ist gewollt — lieber ein Fehlalarm als ein stilles Leck.
    Ein Gate, das seine Blindstelle nicht nennt, wird fuer vollstaendig gehalten.
    """
    erlaubt = _erlaubte_conftest_variablen()

    treffer = []
    for pfad in _gescannte_testdateien():
        baum = ast.parse(pfad.read_text(encoding="utf-8"))
        for knoten in baum.body:       # NUR Modulebene, wie bei den runner-Importen oben
            r = _env_ziel(knoten)
            if not r:
                continue
            name, zeile, wert = r
            if name not in erlaubt:
                treffer.append(f"{pfad.name}:{zeile}  {name}")
            elif wert != erlaubt[name]:
                # Der Name ist erlaubt, der Wert nicht. Gemessen 2026-09-26: die Mutation
                # `os.environ["TAXGRAPH_NO_AUTH"] = "1"` -> `"0"` liess den Waechter gruen
                # (19 passed) — Anwesenheit ist kein Uebergeben, ein erlaubter Name ist kein
                # erlaubter Wert. Dieselbe Klasse wie `_hat_bindung` bei bindung=None.
                treffer.append(
                    f"{pfad.name}:{zeile}  {name} = {wert!r} statt {erlaubt[name]!r} "
                    f"(wie in conftest)")

    assert not treffer, (
        "Testmodule setzen beim Import Umgebungsvariablen, die conftest NICHT fuer die Sitzung "
        "setzt — das leckt in fremde Tests im selben xdist-Worker. Nimm eine funktionsgescopte "
        "Fixture (Vorbild: tests/test_checkest_durchstich.py::hid_attrappe).\n  "
        + "\n  ".join(treffer))


def test_env_ziel_liest_den_wert_auch_im_setdefault_zweig():
    """Der setdefault-Zweig hat in allen 283 Dateien heute 0 Fundstellen (gemessen 2026-09-26).
    Ungedeckt bliebe er trotzdem falsch verdrahtbar: stuende der Wert-Index daneben, meldete der
    Waechter still nichts — genau die Klasse, gegen die er antritt. Der Zweig wird darum direkt
    gegen einen AST gestellt, ohne Fixtur und ohne Datei."""
    mit_wert = ast.parse('os.environ.setdefault("TAXGRAPH_NO_AUTH", "1")').body[0]
    assert _env_ziel(mit_wert) == ("TAXGRAPH_NO_AUTH", 1, "1")

    # Ohne zweites Argument gibt es keinen Wert: None, und der Waechter meldet (fail-closed).
    ohne_wert = ast.parse('os.environ.setdefault("TAXGRAPH_NO_AUTH")').body[0]
    assert _env_ziel(ohne_wert) == ("TAXGRAPH_NO_AUTH", 1, None)

    # Ein Nicht-String-Wert ist kein vergleichbarer Wert — ebenfalls None statt stiller Deckung.
    zahl = ast.parse('os.environ["TAXGRAPH_NO_AUTH"] = 0').body[0]
    assert _env_ziel(zahl) == ("TAXGRAPH_NO_AUTH", 1, None)


def test_umgebungsleck_waechter_sieht_ueberhaupt_dateien():
    """Selbstratsche zum Waechter darueber: er ist gruen, WEIL er nichts findet — wird seine
    Schleife leer, sagt er dasselbe. Gemessen 2026-09-26 (Befund main): Dateischleife auf
    `glob(...)[:0]` gesetzt -> der Waechter meldete weiter 1 passed. Die Zahlen unten sind
    gemessen, nicht geschaetzt; sinkt die Zahl, ist das eine Entscheidung, keine Nebenwirkung."""
    dateien = _gescannte_testdateien()
    DATEIEN_UNTEN = 283    # gemessen 2026-09-26: tests/test_*.py
    assert len(dateien) >= DATEIEN_UNTEN, (
        f"Der Waechter liest nur {len(dateien)} Testdateien, erwartet mindestens {DATEIEN_UNTEN} — "
        f"eine leere oder beschnittene Scan-Menge ist gruen, ohne etwas geprueft zu haben")

    erlaubt = _erlaubte_conftest_variablen()
    assert len(erlaubt) >= 1, (
        "conftest setzt auf Modulebene keine Umgebungsvariable mehr — dann ist JEDER Modulebene-"
        "Schreibzugriff ein Treffer, oder der Scan liest die conftest nicht mehr")

    # Die Werte-Dimension darf nicht selbst blind werden: liest _env_ziel den Wert der conftest
    # nicht als Literal, vergliche der Waechter gegen None. Gemessen 2026-09-26 ist der Wert ein
    # Literal ('1') — die Zusicherung haelt das fest, statt es anzunehmen.
    assert all(w is not None for w in erlaubt.values()), (
        f"conftest setzt eine erlaubte Variable mit einem Wert, den der Scan nicht als Literal "
        f"liest: {[k for k, w in erlaubt.items() if w is None]} — der Werte-Vergleich liefe dann "
        f"gegen None. Erwartet wird ein String-Literal in conftest")

    VARIABLEN_OBEN = 1     # gemessen 2026-09-26: TAXGRAPH_NO_AUTH, der einzige solche Fall
    assert len(erlaubt) <= VARIABLEN_OBEN, (
        f"Die Ausnahmemenge ist auf {len(erlaubt)} gewachsen ({sorted(erlaubt)}): "
        f"{sorted(set(erlaubt) - {'TAXGRAPH_NO_AUTH'})} ist neu. Jede Variable hier schaltet den "
        f"Waechter fuer sie ab — das braucht eine Entscheidung, nicht eine Zeile mehr in conftest")


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


# ---- Ein Lauf ohne Catala-Engine nennt seine Ursache in der Schlusszeile ------------------------
# Entscheidung ein-lauf-ohne-catala-engine-bleibt-rot-und-nennt-seine-ursache (Vault), Backlog
# conftest-guard-prueft-importzeit-statt-laufzeit. Der Lauf bleibt rot; nur die Ursache kommt dazu.

def test_schlusszeile_nennt_die_zahlen_und_fehlt_mit_engine():
    sys.path.insert(0, HERE)
    import conftest                       # noqa: E402

    # Engine da -> keine Zeile.
    assert conftest._catala_schlusszeile(False, 5, 3, 24) is None
    # Engine fehlt, kein Fehler -> Zeile mit der Zahl der nicht gesammelten Dateien.
    z = conftest._catala_schlusszeile(True, 0, 0, 24)
    assert "Catala-Engine fehlt in diesem Baum" in z and "24 Testdateien NICHT gesammelt" in z
    assert "kein Fehler" in z
    # Engine fehlt, F = 5, N = 3 -> beide Zahlen, dazu die Abhilfe.
    z = conftest._catala_schlusszeile(True, 5, 3, 24)
    assert "5 Fehler, davon 3 mit Importversuch" in z and "24 Testdateien NICHT gesammelt" in z
    assert "make build-python" in z and "_catala" in z


def test_catala_fehlt_stimmt_mit_einem_eigenen_importversuch_ueberein():
    """Die Schlusszeile hängt an `_ENGINE_FEHLT`. Ohne diese Gegenprobe könnte `_catala_fehlt()` immer
    False liefern, und alle Tests der Zeilenlogik blieben grün: sie bekämen ihre Bedingung gestellt."""
    sys.path.insert(0, HERE)
    import conftest                       # noqa: E402

    try:
        import runner                     # noqa: F401, E402 — conftest hat die Pfade gesetzt
        engine_da = True
    except Exception:
        engine_da = False
    assert conftest._ENGINE_FEHLT == (not engine_da), (
        f"conftest sagt Engine fehlt={conftest._ENGINE_FEHLT}, ein eigener Import von `runner` sagt "
        f"engine_da={engine_da}")


def test_lauf_ohne_engine_endet_mit_der_ursache_nach_der_fehlerliste(tmp_path):
    """Ende-zu-Ende in einem eigenen pytest-Prozess: ein Test, der die Engine laden will, und einer,
    der es nicht tut. Die Zeile steht NACH der `FAILED`-Zeile und zählt genau einen Importversuch."""
    sys.path.insert(0, HERE)
    import conftest                       # noqa: E402

    if not conftest._ENGINE_FEHLT:
        pytest.skip("mit Catala-Engine — die Schlusszeile erscheint dort nie, nichts zu messen")
    import subprocess

    (tmp_path / "test_probe_engine.py").write_text(
        "def test_laedt_die_engine():\n    import pkg  # noqa: F401\n\n\n"
        "def test_ohne_engine():\n    assert 1 + 1 == 2\n", encoding="utf-8")
    env = dict(os.environ, PYTHONPATH=os.pathsep.join([HERE, os.environ.get("PYTHONPATH", "")]),
               PYTHONDONTWRITEBYTECODE="1")
    r = subprocess.run(
        [sys.executable, "-m", "pytest", "-q", "-p", "conftest", "-p", "no:cacheprovider",
         "-p", "no:randomly", "-o", "addopts=", "--rootdir", str(tmp_path), str(tmp_path)],
        cwd=tmp_path, env=env, capture_output=True, text=True, timeout=300)
    zeilen = r.stdout.splitlines()
    ursache = [i for i, z in enumerate(zeilen) if "Catala-Engine fehlt in diesem Baum" in z]
    assert len(ursache) == 1, f"erwartet genau eine Ursache-Zeile:\n{r.stdout[-2500:]}\n{r.stderr[-800:]}"
    assert "1 Fehler, davon 1 mit Importversuch" in zeilen[ursache[0]]
    failed = [i for i, z in enumerate(zeilen) if z.startswith("FAILED")]
    assert failed and ursache[0] > failed[-1], (
        "die Ursache-Zeile muss NACH der Fehlerliste stehen, sonst geht sie wie die Anfangszeile unter")
    assert "1 failed, 1 passed" in r.stdout        # nichts umgewandelt: rot bleibt rot


# ---------------------------------------------- was CI und Makefile starten, muss startbar sein
#
# DER FUND (Abnahme-Audit 2026-08-19): der Umzug des Rechenkerns nach produkt/engine/ liess
# `golden/golden_lauf.py` ohne den Pfad zurück, unter dem `runner` seither zu finden ist. Das
# Skript war damit nicht mehr startbar — `make golden` und der CI-Job `golden` brachen sofort
# ab. Gemeldet hat es das Audit, nicht die Suite.
#
# WARUM DIE SUITE BLIND WAR, und das ist der eigentliche Punkt: tests/conftest.py legt
# produkt/engine für JEDEN pytest-Lauf in den Pfad. Unter pytest war der Import also immer in
# Ordnung. Und mein eigener Beleg "Golden-Lauf 135/135" kam aus einem Aufruf, der denselben
# Pfad mitbrachte — das Gate bekam seine Vorbedingung von dem geliefert, der es prüfen wollte.
# Dieselbe Klasse wie der conftest-Guard, der ohne golden/ im Pfad immer "Catala fehlt" sagte.
#
# Deshalb: eigener Prozess, `-I` (weder PYTHONPATH noch site-packages des Nutzers), und die
# Liste der Skripte wird aus ci.yml + Makefile GEMESSEN statt gepflegt. Ein neues Skript im
# Workflow ist automatisch dabei.
_SKRIPT_AUFRUF = re.compile(r"python3?\s+(?:-\S+\s+)*([\w./-]+\.py)")


def test_versteckt_urteilt_ueber_das_repo_nicht_ueber_seinen_ort(tmp_path):
    """Der Filter `_versteckt` muss den Pfad INNERHALB des Repos beurteilen.

    Anlass (gemessen 2026-09-26, backlog/arbeitsbaum-unter-punktverzeichnis-erfindet-fehlschlaege):
    lag der Arbeitsbaum selbst unter einem Punkt-Verzeichnis (~/.cache/…, .claude/worktrees/…,
    ~/.local/state/orch/…), pruefte der Filter ueber den ABSOLUTEN Pfad und warf JEDE Datei
    heraus — 0 statt 496 eigene Module, 55 projekteigene Module galten als fehlende
    Fremdpakete. Der Test oben meldete daraufhin jeden eigenen Import als fehlendes
    Fremdpaket: ein Fehlschlag, der am Ort des Baums hing, nicht am Code.

    Zwei Faelle, und der zweite ist der entscheidende:
    1. ein Punkt-Verzeichnis INNERHALB des Repos -> die Datei darin ist zu Recht versteckt
    2. ein Repo, das SELBST unter einem Punkt-Verzeichnis liegt -> darin ist NICHTS versteckt

    Fall 2 unterscheidet die beiden Fassungen wirklich: in einem dot-freien Baum wie diesem
    liefern `p.parts` und `p.relative_to(ROOT).parts` dasselbe Ergebnis, eine Probe an Fall 1
    allein bliebe bei der Mutation also gruen. Die Wurzel wird darum als Parameter gestellt.
    """
    wurzel = tmp_path / "repo"
    (wurzel / ".versteckt").mkdir(parents=True)
    (wurzel / "sichtbar.py").write_text("", encoding="utf-8")
    drin = wurzel / ".versteckt" / "drin.py"
    drin.write_text("", encoding="utf-8")

    # 1. Punkt-Verzeichnis IM Repo bleibt versteckt — auch relativ zur Wurzel.
    assert _versteckt(drin, wurzel), "ein Punkt-Verzeichnis im Repo muss versteckt bleiben"
    assert not _versteckt(wurzel / "sichtbar.py", wurzel), "eine normale Datei ist nicht versteckt"

    # 2. Der Ort des Repos darf nicht mitzaehlen. Die Wurzel liegt hier explizit unter einem
    #    Punkt-Verzeichnis; unter der Mutation (p.parts) faellt `sichtbar.py` damit heraus,
    #    obwohl im Repo nichts versteckt ist. Nachgemessen 2026-09-26: laege die Wurzel
    #    dot-FREI, bliebe Fall 1 unter der Mutation gruen (faenge sie also NICHT) — erst
    #    Fall 2 wird dort rot. Deshalb steht Fall 2 hier und nicht nur Fall 1.
    dot_wurzel = tmp_path / ".cache" / "repo"
    (dot_wurzel / ".versteckt").mkdir(parents=True)
    sichtbar_dot = dot_wurzel / "sichtbar.py"
    sichtbar_dot.write_text("", encoding="utf-8")

    assert not _versteckt(sichtbar_dot, dot_wurzel), (
        "das Repo liegt unter einem Punkt-Verzeichnis, die Datei darin ist trotzdem nicht "
        "versteckt — der Filter urteilt ueber den Pfad IM Repo, nicht ueber seinen Ort")
    assert _versteckt(dot_wurzel / ".versteckt" / "x.py", dot_wurzel), (
        "ein Punkt-Verzeichnis im Repo bleibt auch dann versteckt, wenn die Wurzel selbst "
        "unter einem Punkt-Verzeichnis liegt")


def _gestartete_skripte() -> set[str]:
    treffer = set()
    for quelle in (CI, ROOT / "Makefile"):
        for zeile in quelle.read_text(encoding="utf-8").splitlines():
            if zeile.strip().startswith("#"):
                continue                      # ein Kommentar startet nichts
            treffer.update(_SKRIPT_AUFRUF.findall(zeile))
    return treffer


def _repo_eigene_module() -> set[str]:
    """Jeder Modulname, der als .py im Repo liegt. Die Trennlinie des Gates unten."""
    namen = set()
    for p in ROOT.rglob("*.py"):
        if any(t in p.parts for t in ("__pycache__", ".git", "_catala", "node_modules")):
            continue
        namen.add(p.stem)
    return namen


def _importiert_isoliert(skript: pathlib.Path) -> tuple[int, str]:
    import subprocess
    r = subprocess.run(
        [sys.executable, "-I", "-c",
         f"import sys; sys.path.insert(0, {str(skript.parent)!r}); import {skript.stem}"],
        capture_output=True, text=True, cwd=str(ROOT), timeout=180)
    return r.returncode, r.stderr


def test_jedes_von_ci_gestartete_skript_ist_importierbar():
    """Ein Skript, das der Workflow aufruft, muss sich ohne fremde Hilfe importieren lassen.

    ENG GEFASST, sonst wäre das Gate unbrauchbar: fünf der zwölf Skripte scheitern hier an
    `gettsim` bzw. `psycopg` — Fremdpakete, die im venv312 bzw. in der Datenbank-Umgebung
    stehen und in diesem Interpreter zurecht fehlen. Das ist eine Umgebungsfrage, kein Fund.
    Ein Fund ist NUR ein Modul, das als .py IM REPO liegt und trotzdem nicht gefunden wird —
    genau die Signatur des Umzugs-Fehlers (`No module named 'runner'`, während
    produkt/engine/runner.py danebenliegt).

    Dieselbe Trennlinie wie bei den drei anderen Fällen dieser Art (runner+pkg, ERiC-XSD,
    Hersteller-ID): lokal vorhanden / in CI zurecht nicht ist kein Fehler — im Repo vorhanden
    und trotzdem nicht auffindbar ist einer."""
    skripte = _gestartete_skripte()

    # Leere-Menge-Falle, und heute schon einmal zugeschlagen: eine Umzugsliste, die leerläuft,
    # wird still grün. Hier hinge alles am Regex — ändert jemand die Aufrufform in ci.yml
    # (`uv run`, ein Shell-Wrapper, ein Makefile-Variablenname), findet es nichts mehr und der
    # Test meldet Erfolg über eine leere Menge. Die Untergrenze ist absichtlich stumpf: sie
    # muss nur merken, dass die MESSUNG kaputt ist, nicht wie viele Skripte es geben sollte.
    assert len(skripte) >= 8, (
        f"Nur {len(skripte)} gestartete Skripte in ci.yml/Makefile gefunden (erwartet >= 8) — "
        f"vermutlich passt _SKRIPT_AUFRUF nicht mehr auf die Aufrufform. Gefunden: "
        f"{sorted(skripte)}")
    assert "golden/golden_lauf.py" in skripte, (
        "golden/golden_lauf.py wird nicht mehr erkannt — das ist genau das Skript, dessen "
        "fehlender Pfad-Prolog dieses Gate ausgelöst hat")

    eigene = _repo_eigene_module()
    fehlt_re = re.compile(r"No module named '([\w.]+)'")
    verstoss, umgebung = [], []

    for rel in sorted(skripte):
        pfad = ROOT / rel
        assert pfad.exists(), (
            f"{rel} wird von ci.yml/Makefile gestartet, existiert aber nicht — toter Aufruf "
            f"(genau so blieb der ci.yml-Kommentar auf golden/runner.py stehen, nachdem die "
            f"Datei umgezogen war)")
        rc, stderr = _importiert_isoliert(pfad)
        if rc == 0:
            continue
        m = fehlt_re.search(stderr)
        fehlend = m.group(1).split(".")[0] if m else None
        if fehlend and fehlend in eigene:
            verstoss.append(f"{rel}: findet '{fehlend}' nicht, obwohl es im Repo liegt")
        elif fehlend:
            umgebung.append(f"{rel} ({fehlend})")
        else:
            verstoss.append(f"{rel}: Import scheitert — {stderr.strip().splitlines()[-1][:120]}")

    assert not verstoss, (
        "Von CI/Makefile gestartete Skripte sind nicht startbar, und zwar an einem Modul, das "
        "IM REPO LIEGT — d.h. ein Pfad-Prolog fehlt oder zeigt nach einem Umzug ins Leere:\n  "
        + "\n  ".join(verstoss)
        + f"\n(Umgebungsbedingt übersprungen, kein Fund: {', '.join(umgebung) or 'keine'})")


def test_die_probe_faengt_den_umzugs_fehler():
    """Negativprobe: ohne sie ist nicht belegt, dass das Gate darüber den Fund vom 2026-08-19
    überhaupt sehen KANN — und ein Test, der seinen eigenen Fehlerfall nicht kennt, ist eine
    Behauptung. Nachgestellt wird golden_lauf.py ohne die Pfadzeile für produkt/engine."""
    import subprocess
    import tempfile

    quelle = (ROOT / "golden" / "golden_lauf.py").read_text(encoding="utf-8")
    ohne = quelle.replace(
        'sys.path.insert(0, os.path.join(ROOT, "produkt", "engine"))', "")
    assert ohne != quelle, (
        "Die Pfadzeile für produkt/engine steht nicht mehr wörtlich in golden_lauf.py — "
        "wurde sie umgebaut, gehört diese Probe nachgezogen, sonst prüft sie nichts")

    with tempfile.TemporaryDirectory(dir=str(ROOT / "golden")) as d:
        # im selben Verzeichnis, damit der relative ROOT-Aufstieg gleich viele Ebenen hat
        kaputt = pathlib.Path(d) / "golden_lauf_ohne_pfad.py"
        kaputt.write_text(ohne.replace(
            'os.path.join(os.path.dirname(__file__), "..")',
            'os.path.join(os.path.dirname(__file__), "..", "..")'), encoding="utf-8")
        r = subprocess.run(
            [sys.executable, "-I", "-c",
             f"import sys; sys.path.insert(0, {d!r}); import golden_lauf_ohne_pfad"],
            capture_output=True, text=True, cwd=str(ROOT), timeout=180)

    assert r.returncode != 0 and "No module named 'runner'" in r.stderr, (
        f"Ohne die Pfadzeile gelingt der Import trotzdem — dann prüft das Gate darüber nichts. "
        f"(Läge ein zweites `runner` im Pfad, wäre genau das die Doppel-Identitäts-Falle.)\n"
        f"rc={r.returncode}\n{r.stderr[-800:]}")


# ---- ERiC-Aufrufer: jeder traegt den Marker @braucht_eric --------------------------------
#
# ABGRENZUNG zum Waechter oben (bewusst NICHT zusammengebaut, zwei verschiedene Aussagen):
#   test_kein_testmodul_veraendert_die_umgebung_beim_import  prueft die ENGE des Skips
#       (darf hier ueberhaupt etwas uebersprungen werden?) und liest dafuer `os.environ`.
#   hier unten: die DEKORATEURE (ist beim ERiC-Aufrufer ueberhaupt ein Marker da?).
#
# ANLASS, gemessen 2026-09-26: test_checkest_feldmatrix.py::test_p35a_einzelaufstellung_alle_
# drei_toepfe_amtlich_plausibel trug den Marker nicht, obwohl seine vier Geschwister in derselben
# Datei ihn trugen. Solange eine fremde Testdatei ihren Hersteller-ID-Platzhalter global in die
# Umgebung schrieb (10baecb), lief er mit dem Platzhalter gegen ERiC und fiel mit rc=610301200 —
# die Suite rot, und der rote Test sah nach einem 35a-Fehler aus. Nach dem Leck-Fix faellt er
# korrekt in den Skip. Der Marker ist inzwischen an beiden Stellen gesetzt (bab2ec5, 4cce87b) —
# ohne ein Gate kommt er beim naechsten neuen ERiC-Aufrufer wieder weg.
#
# WAS ALS "ERiC-AUFRUFER" ZAEHLT, und was ausdruecklich nicht:
#   JA:  `CE.validate(...)` — CE ist `import checkest_gate as CE`, und checkest_gate.validate()
#        laedt die ERiC-Bibliothek (elster/checkest_gate.py:197 ueber _load_and_init).
#   NEIN: `VX.validate(...)` — VX ist `import validate_xsd as VX`, ein reiner XSD-Check ohne ERiC.
#        Beide heissen `validate`; wer nur auf den Methodennamen sieht, verwechselt sie. Das
#        waere genau die Klasse "ueber X gemessen, ueber Y behauptet".

# Der Modulname, unter dem checkest_gate in tests/ importiert wird. Gemessen 2026-09-26 ueber
# alle 283 Dateien: genau EIN Alias, `import checkest_gate as CE` — kein `from ... import`.
ERIC_MODUL = "CE"
ERIC_FUNKTION = "validate"

# Aufrufer, die ERiC erreichen und trotzdem KEINEN Marker tragen duerfen — mit Begruendung.
# LEER, und das ist ein Messergebnis: alle Aufrufer, die ERiC heute erreichen, tragen den Marker.
ERIC_OHNE_MARKER: dict[tuple[str, str], str] = {}

MARKER = "braucht_eric"


def _dekorator_namen(fn) -> set[str]:
    """Alle Decorator-Namen einer Funktion, ohne Argumente (@braucht_eric, @pytest.mark.x)."""
    namen = set()
    for d in fn.decorator_list:
        ziel = d.func if isinstance(d, ast.Call) else d
        if isinstance(ziel, ast.Name):
            namen.add(ziel.id)
        elif isinstance(ziel, ast.Attribute):
            namen.add(ziel.attr)
    return namen


def _funktionen(baum: ast.Module) -> dict:
    return {f.name: f for f in ast.walk(baum)
            if isinstance(f, (ast.FunctionDef, ast.AsyncFunctionDef))}


def _ruft_eric(fn) -> bool:
    """True, wenn die Funktion CE.validate(...) DIREKT aufruft."""
    return any(isinstance(k, ast.Call) and isinstance(k.func, ast.Attribute)
               and k.func.attr == ERIC_FUNKTION
               and isinstance(k.func.value, ast.Name)
               and k.func.value.id == ERIC_MODUL
               for k in ast.walk(fn))


def _ruft(fn) -> set[str]:
    """Namen aller direkt aufgerufenen Funktionen — die Kanten fuer den Erreichbarkeitslauf."""
    return {k.func.id for k in ast.walk(fn)
            if isinstance(k, ast.Call) and isinstance(k.func, ast.Name)}


def _eric_aufrufer() -> tuple[set, int]:
    """((Datei, Testfunktion) die ERiC transitiv erreichen; Zahl der CE.validate-Stellen).

    TRANSITIV, und das ist der Punkt: von den CE.validate-Aufrufstellen unter tests/ stehen
    sieben in Helfern (_block_scharf, _pruefe, _scharf, _xml_und_rc, _dekl_ohne,
    _pruefe_kernfeld, _ohne_feld), nicht in Tests. Ein Gate, das nur die aufrufende Funktion
    ansieht, uebersaehe jeden Test, der ERiC ueber einen solchen Helfer erreicht.
    """
    aufrufer = set()
    stellen = 0
    for pfad in _gescannte_testdateien():
        baum = ast.parse(pfad.read_text(encoding="utf-8"))
        funs = _funktionen(baum)
        stellen += sum(1 for f in funs.values() for k in ast.walk(f)
                       if isinstance(k, ast.Call) and isinstance(k.func, ast.Attribute)
                       and k.func.attr == ERIC_FUNKTION
                       and isinstance(k.func.value, ast.Name) and k.func.value.id == ERIC_MODUL)
        erreicht = {n for n, f in funs.items() if _ruft_eric(f)}
        # Fixpunkt: wer einen ERiC-Erreicher ruft, erreicht ERiC ebenfalls.
        # ponytail: O(n^2) je Datei bei ~25 Funktionen — reicht weit, bei Wachstum Worklist.
        geaendert = True
        while geaendert:
            geaendert = False
            for n, f in funs.items():
                if n not in erreicht and (_ruft(f) & erreicht):
                    erreicht.add(n)
                    geaendert = True
        aufrufer |= {(pfad.name, n) for n in funs if n.startswith("test_") and n in erreicht}
    return aufrufer, stellen


def test_jeder_eric_aufrufer_traegt_den_marker():
    """Kern-Gate: wer ERiC erreicht, traegt @braucht_eric — sonst laeuft er ohne ERiC-Zugang
    gegen eine fehlende Bibliothek statt zu skippen."""
    aufrufer, _ = _eric_aufrufer()
    fehlend = []
    for pfad in _gescannte_testdateien():
        baum = ast.parse(pfad.read_text(encoding="utf-8"))
        for fn in _funktionen(baum).values():
            if not fn.name.startswith("test_"):
                continue
            if (pfad.name, fn.name) in aufrufer and MARKER not in _dekorator_namen(fn):
                if (pfad.name, fn.name) not in ERIC_OHNE_MARKER:
                    fehlend.append(f"{pfad.name}:{fn.lineno}  {fn.name}")
    assert not fehlend, (
        f"Testfunktionen erreichen ERiC (CE.validate), tragen aber kein @{MARKER} — ohne ERiC "
        f"laufen sie nicht in den Skip, sondern in einen Fehler, der nach einem Fachfehler "
        f"aussieht:\n  " + "\n  ".join(sorted(fehlend)))


def test_eric_gate_sieht_ueberhaupt_aufrufer():
    """Selbstratsche in beide Richtungen.

    Ohne die Untergrenze ist das Gate still gruen, sobald es keine Aufrufer mehr findet — etwa
    weil der Modul-Alias sich aendert (dann sucht es `CE.validate` und keiner ruft mehr so) oder
    weil die Dateischleife leer wird. Beide Faelle sind als Mutation gemessen.
    """
    aufrufer, stellen = _eric_aufrufer()
    AUFRUFER_UNTEN = 24   # gemessen 2026-09-26: Testfunktionen, die ERiC transitiv erreichen
    STELLEN_UNTEN = 15    # gemessen 2026-09-26: CE.validate-Aufrufstellen unter tests/
    assert len(aufrufer) >= AUFRUFER_UNTEN, (
        f"Nur {len(aufrufer)} ERiC-erreichende Testfunktionen gefunden, erwartet mindestens "
        f"{AUFRUFER_UNTEN} — sucht das Gate noch den richtigen Modulnamen?")
    assert stellen >= STELLEN_UNTEN, (
        f"Nur {stellen} CE.validate-Aufrufstellen unter tests/ gefunden, erwartet mindestens "
        f"{STELLEN_UNTEN} — der Scan sucht womoeglich einen Namen, den keiner mehr ruft")

    assert len(ERIC_OHNE_MARKER) == 0, (
        f"ERIC_OHNE_MARKER ist auf {len(ERIC_OHNE_MARKER)} gewachsen "
        f"({sorted(ERIC_OHNE_MARKER)}): jeder Eintrag nimmt einen ERiC-Aufrufer vom Gate aus — "
        f"das braucht eine Entscheidung mit Begruendung, nicht eine Zeile mehr")
    for schluessel, grund in ERIC_OHNE_MARKER.items():
        assert len(grund) > 20, f"{schluessel}: Ausnahme ohne (ausreichende) Begruendung"

    # Gegenrichtung: die Ausnahmeliste darf nicht auf tote Eintraege zeigen.
    tot = sorted(set(ERIC_OHNE_MARKER) - aufrufer)
    assert not tot, f"ERIC_OHNE_MARKER nennt Aufrufer, die es nicht (mehr) gibt: {tot}"


# ---- .gitignore: auch ein VERWEIS auf ein Build-Verzeichnis wird ignoriert -----------------------
# Entscheidung ignorier-regeln-der-build-verzeichnisse-gelten-auch-fuer-verweise (Vault), Backlog
# gitignore-nimmt-den-catala-symlink-nicht-aus. Ein Arbeitsbaum bekommt statt eines Build-Verzeichnisses
# einen Symlink auf den Hauptbaum (arbeitsbaum-misst-wie-der-hauptbaum, Punkt 3). Eine Regel mit `/`
# am Ende gilt nur für echte Verzeichnisse: der Verweis wäre für Git eine neue Datei, und `git add -A`
# (die Auto-Speicherung von orch) nähme ihn mit. Gemessen 2026-10-03: drei Zweige tragen so einen Verweis.
_BUILD_NAMEN = ("_build", "_target", "_targets", "oracle/.venv", "oracle/.venv312",
                "oracle/gettsim/_catala")


def _nicht_ignoriert(repo: pathlib.Path, pfade: list[str]) -> list[str]:
    import subprocess
    env = dict(os.environ, GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1")  # kein fremdes excludesfile
    return [p for p in pfade
            if subprocess.run(["git", "check-ignore", "-q", p], cwd=repo, env=env).returncode != 0]


def test_gitignore_nimmt_verweise_auf_build_verzeichnisse_aus(tmp_path):
    """Je Name zwei Fälle in einem Wegwerf-Repo mit der echten `.gitignore` dieses Baums: ein VERWEIS
    (der Fall, der fehlte) und ein echtes Verzeichnis (muss ignoriert bleiben). Kein absoluter Pfad:
    die `.gitignore` kommt aus der Wurzel des Baums, in dem der Test läuft."""
    import shutil
    import subprocess

    ziel = tmp_path / "ziel"
    ziel.mkdir()
    ergebnis = {}
    for fall in ("verweis", "verzeichnis"):
        repo = tmp_path / fall
        repo.mkdir()
        subprocess.run(["git", "init", "-q", "."], cwd=repo, check=True)
        shutil.copy(ROOT / ".gitignore", repo / ".gitignore")
        pfade = []
        for name in _BUILD_NAMEN:
            (repo / name).parent.mkdir(parents=True, exist_ok=True)
            if fall == "verweis":
                (repo / name).symlink_to(ziel, target_is_directory=True)
                pfade.append(name)
            else:
                (repo / name).mkdir()
                (repo / name / "datei.txt").write_text("x", encoding="utf-8")
                pfade.append(f"{name}/datei.txt")
        ergebnis[fall] = _nicht_ignoriert(repo, pfade)
    meldungen = []
    if ergebnis["verweis"]:
        meldungen.append(
            "Git ignoriert diese Build-Namen nicht, wenn sie ein VERWEIS sind (die Regel in .gitignore "
            "endet auf `/` und gilt nur für Verzeichnisse): " + ", ".join(ergebnis["verweis"]))
    if ergebnis["verzeichnis"]:
        meldungen.append("Git ignoriert ein echtes Verzeichnis unter diesen Namen nicht mehr: "
                         + ", ".join(ergebnis["verzeichnis"]))
    assert not meldungen, "\n".join(meldungen)
