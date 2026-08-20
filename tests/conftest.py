"""Gate: kein Test darf ins ECHTE audit.jsonl schreiben.

Grund: 32 Testdateien haben genau das versehentlich getan (Audit-Log auf 582 MB
angewachsen, s. Backlog-Eintrag "audit-jsonl-wucherung", Vault backlog/taxgraph.md). Ein Grep nach dem Patch-
Muster (`monkeypatch.setattr(audit, "AUDIT_DIR", ...)`) faengt die Faelle NICHT,
die den Fix noetig machten: unconditionaler audit.append-Aufruf ohne jeden Patch,
eine Schwesterklasse in derselben Datei ohne Patch, eine von zwei Fixtures in
derselben Datei ohne Patch. Deshalb misst dieses Gate den echten Schreibpfad
zur Laufzeit statt ein Namensmuster.

Fruehere Fassung mass die Dateigroesse der echten audit.jsonl vorher/nachher
(os.path.getsize). Im Parallelbetrieb (mehrere Worker/Skripte im selben
Checkout, dieselbe Datei) beschuldigt das einen unbeteiligten Test, sobald ein
FREMDER Prozess in derselben Sekunde in dieselbe echte Datei schreibt — die
Groesse ist geteilter Zustand, kein Signal ueber DIESEN Testprozess (Vorfall
2026-08-09 23:22, test_bindungstabelle.py faelschlich beschuldigt).

Jetzt: audit.append wird pro Test umwickelt, jeder Aufruf wird am tatsaechlich
aufgeloesten Pfad geprueft (kein `from audit import append` im Repo — ein
Modul-Attribut-Patch faengt jeden Aufrufer, egal aus welcher Datei/Klasse/
Fixture er kommt, exakt dieselbe Faellklasse wie beim Namensmuster-Grep oben).
Sieht nur, was DIESER Prozess in DIESEM Test tatsaechlich aufruft — kein
Datei-I/O, kein geteilter Zustand, kein Wettlauf mit fremden Schreibern.
"""
from __future__ import annotations

import os
import sys

import pytest

_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.dirname(_HERE)
_STORE = os.path.join(_ROOT, "produkt", "store")
if _STORE not in sys.path:
    sys.path.insert(0, _STORE)
_HAUT = os.path.join(_ROOT, "produkt", "haut")
if _HAUT not in sys.path:
    sys.path.insert(0, _HAUT)


# --------------------------------------------------- Sammel-Guard ohne Catala-Toolchain
#
# OHNE gebaute Catala-Toolchain ist `runner` nicht importierbar (es zieht den
# Compiler-Output `pkg` nach). 23 Testdateien importieren ihn auf Modulebene ohne try —
# ihre Collection scheitert dann mit ModuleNotFoundError, und pytest bricht die GESAMTE
# Sammlung ab: gemessen am 2026-08-18 in CI 27 Collection-Fehler, 1 skipped, NULL Tests
# gelaufen. Der schnelle CI-Job war damit seit Monaten rot und hat nie etwas geprüft.
#
# WARUM ZENTRAL, obwohl .github/workflows/ci.yml ausdrücklich "kein zentrales conftest"
# festhält: die dort beschriebene Regel — jede Datei guardet sich selbst — hat nicht
# getragen. Derselbe Kommentar sagt "verifiziert gegen ALLE tests/*.py, Stand 2026-07-21:
# EINE Luecke gefunden" und nennt test_kapital_accessoren.py, das im YAML per --ignore
# ausgenommen ist. Aus dieser einen sind 23 geworden, ohne dass irgendetwas es meldete.
# Eine Konvention, die 22-mal hintereinander gebrochen wird, ist keine Konvention; eine
# --ignore-Liste im YAML wäre dieselbe Pflegeliste mit demselben Schicksal.
#
# Die Menge wird deshalb GEMESSEN statt gepflegt: der AST sagt, welche Datei `runner`
# oder `pkg` auf Modulebene importiert. Eine neue solche Datei ist automatisch dabei.
#
# WAS DAS NICHT TUT: Fehler verstecken. Übersprungen wird NUR, wenn `runner` wirklich
# nicht importierbar ist — mit Toolchain (der volle CI-Job, jeder lokale Lauf) läuft
# jede Datei echt. Ein ImportError aus einem ANDEREN Grund bleibt sichtbar, weil er die
# Probe unten nicht betrifft.
def _catala_fehlt() -> bool:
    """Ist `runner` importierbar?

    ACHTUNG, HIER LAG BEIM ERSTEN ENTWURF EIN SCHWERER FEHLER (gefunden 2026-08-18 von
    test_ci_konfiguration.test_guard_greift_nur_ohne_toolchain, nicht beim Schreiben): die
    Prüfung stand OHNE den golden-Pfad. `runner` liegt in golden/, und den fügen sonst die
    Testdateien selbst hinzu, kurz bevor sie ihn importieren — zum conftest-Zeitpunkt ist er
    noch nicht in sys.path. Die Probe scheiterte also IMMER, und der Guard hätte 23 Dateien
    auch dort übersprungen, wo Catala vollständig verfügbar ist: lokal und im vollen CI-Job.
    Ergebnis wäre eine grüne Suite mit einem Viertel weniger Tests gewesen — ein Guard, der
    genau den Schaden anrichtet, gegen den er gebaut ist."""
    # produkt/engine zuerst: dort liegt der Rechenkern seit 2026-08-19. `golden` bleibt
    # in der Liste, weil dort weiterhin Daten und der Golden-Lauf selbst liegen.
    for teil in ("produkt/engine", "golden", "produkt/mapping", "produkt/traverser",
                 "produkt/unsicherheit"):
        p = os.path.join(_ROOT, teil)
        if p not in sys.path:
            sys.path.insert(0, p)
    try:
        import runner  # noqa: F401
        return False
    except Exception:
        return True


def _dateien_die_catala_brauchen() -> list[str]:
    import ast
    treffer = []
    for name in sorted(os.listdir(_HERE)):
        if not (name.startswith("test_") and name.endswith(".py")):
            continue
        try:
            baum = ast.parse(open(os.path.join(_HERE, name), encoding="utf-8").read())
        except SyntaxError:
            continue                     # soll die Collection selbst melden, nicht wir
        for knoten in baum.body:         # NUR top-level: ein Import in try/except ist geguardet
            if isinstance(knoten, ast.Import) and any(
                    a.name in ("runner", "pkg") for a in knoten.names):
                treffer.append(name)
                break
            if isinstance(knoten, ast.ImportFrom) and knoten.module in ("runner", "pkg"):
                treffer.append(name)
                break
    return treffer


# ------------------------------------------- Fehlendes ERiC-Schema ist ein Skip, kein Fehler
#
# Das ERiC-XSD ist lizenzpflichtig: kein öffentlicher Download, in CI nicht vorhanden. Der
# Workflow hält das seit jeher fest — "alle @requires_real_schema-Tests skippen hier graceful
# weiter". Gemessen am 2026-08-18, als die Suite dort zum ersten Mal wirklich lief: das gilt
# NUR für tests/test_xsd_verify.py, wo das Muster steht. 14 weitere Dateien kennen es nicht,
# und 102 Tests scheiterten mit
#     XmlFehler: E10-2025.xsd nicht gefunden — $ERIC_DIR setzen / ERiC-Doku entpacken.
#
# Dieselbe Klasse wie die 23 ungeguardeten runner-Importe darüber: eine Konvention, die an
# genau einer Stelle durchgehalten wurde. Deshalb auch hier zentral und gemessen statt in 14
# Dateien nachgezogen — eine Liste, die man an 15 Stellen pflegen muss, wird an einer gepflegt.
#
# ENG GEFASST, damit er keine echten Fehler frisst:
#   - nur wenn das Schema WIRKLICH nirgends liegt (dieselbe Suche wie test_xsd_verify),
#   - nur die Ausnahme XmlFehler,
#   - nur mit genau dieser Meldung.
# Ist das Schema da (jeder lokale Lauf mit ERIC_DIR, `make eric-gate`), greift nichts davon,
# und ein echter XSD-Fehler schlägt durch wie bisher — geprüft in
# tests/test_ci_konfiguration.py::test_eric_skip_greift_nur_ohne_schema.
def _eric_schema_fehlt() -> bool:
    for teil in ("produkt/mapping", "produkt/traverser"):
        p = os.path.join(_ROOT, teil)
        if p not in sys.path:
            sys.path.insert(0, p)
    try:
        import xsd_verify
        return xsd_verify._find_schema(2025) is None
    except Exception:
        return True          # ohne den Sucher ist das Schema erst recht nicht prüfbar


ERIC_SCHEMA_FEHLT = _eric_schema_fehlt()
_ERIC_MUSTER = "nicht gefunden — $ERIC_DIR setzen"

# Zweiter Fall derselben Art, gefunden im nächsten CI-Lauf: neun Tests scheiterten mit
#     XmlFehler: keine Hersteller-ID — $ELSTER_HERSTELLER_ID setzen (nie im Repo, nie im Code).
# Die Hersteller-ID ist ein Geheimnis, kein Download — sie steht in der gitignored .env, die
# conftest hier lädt, und existiert in CI zurecht nicht. Anders geartet als die Lizenzdatei,
# gleiche Folge: der Test kann nicht prüfen, was er prüfen soll, und das ist ein Skip.
#
# Die Bedingung wird ZUR LAUFZEIT gelesen, nicht beim Import: die .env wird oben in dieser
# Datei geladen, und ein Test darf sie per monkeypatch entfernen, um genau die
# fail-closed-Antwort zu prüfen — dann muss der Hook wegbleiben und den Fehler durchlassen.
_HERSTELLER_MUSTER = "keine Hersteller-ID — $ELSTER_HERSTELLER_ID setzen"


def _hersteller_id_fehlt() -> bool:
    return not os.environ.get("ELSTER_HERSTELLER_ID", "").strip()


def _ist_fehlendes_eric_schema(fehler: BaseException) -> bool:
    """Trägt diese Ausnahme das fehlende ERiC-Schema als Ursache?

    ZWEI Typen, und der zweite ist nicht Bequemlichkeit, sondern gemessen: 12 der betroffenen
    Tests fangen den Fehler mit `pytest.raises(..., match=...)` ab, weil sie ein fail-closed-
    Verhalten prüfen. Dort kommt beim Test nie ein XmlFehler an — pytest verwandelt den
    verfehlten Vergleich in einen AssertionError ("Regex pattern did not match"), der die
    tatsächliche Meldung mitführt. Ohne den zweiten Typ blieben genau diese 12 rot, und zwar
    ausgerechnet die Sicherheitstests.

    Die Meldung selbst ist der enge Teil: "$ERIC_DIR setzen / ERiC-Doku entpacken" steht an
    genau zwei Stellen in elster_xml.py und sagt eindeutig, dass die Lizenzdatei fehlt. Ein
    inhaltlicher XmlFehler (fehlendes Pflicht-Kz, falscher Container) trägt sie nicht und wird
    weiterhin rot — geprüft in test_eric_skip_frisst_keine_fremden_fehler."""
    if not ERIC_SCHEMA_FEHLT or _ERIC_MUSTER not in str(fehler):
        return False
    return type(fehler).__name__ in ("XmlFehler", "AssertionError")


def _ist_fehlende_hersteller_id(fehler: BaseException) -> bool:
    """Wie _ist_fehlendes_eric_schema, für das zweite lokal-vorhandene / in-CI-fehlende Stück."""
    if not _hersteller_id_fehlt() or _HERSTELLER_MUSTER not in str(fehler):
        return False
    return type(fehler).__name__ in ("XmlFehler", "AssertionError")


@pytest.hookimpl(wrapper=True)
def pytest_runtest_call(item):
    try:
        return (yield)
    except Exception as e:
        if _ist_fehlendes_eric_schema(e):
            pytest.skip("ERiC-XSD nicht verfügbar (lizenzpflichtig, $ERIC_DIR nicht gesetzt) — "
                        "dieser Test braucht das echte Schema")
        if _ist_fehlende_hersteller_id(e):
            pytest.skip("$ELSTER_HERSTELLER_ID nicht gesetzt (Geheimnis, nie im Repo) — "
                        "dieser Test baut ein vollständiges Übermittlungs-XML")
        raise


collect_ignore = []
if _catala_fehlt():
    collect_ignore = _dateien_die_catala_brauchen()
    # Sichtbar machen, nicht stillschweigend weglassen: eine übersprungene Datei ist eine
    # NICHT gelaufene Prüfung. Die Zahl gehört ins Protokoll, wie jede Skip-Zahl — sonst
    # liest sich ein grüner Lauf wie ein vollständiger (Lehre 2026-08-17: eine CSP schaltete
    # die Barrierefreiheits-Tests still ab, bemerkt allein an der Skip-Zahl 5 -> 6).
    print(f"\n[conftest] Catala-Toolchain nicht verfügbar — {len(collect_ignore)} von "
          f"{len([n for n in os.listdir(_HERE) if n.startswith('test_') and n.endswith('.py')])} "
          f"Testdateien werden NICHT gesammelt. Voller Lauf braucht `clerk build p32a-python`.")

import audit as _audit  # echtes Modul-Attribut, gelesen VOR jedem Test-Monkeypatch
import fehler_log as _fehler_log  # dito — zweite Datei in derselben Ablage, zweite Wache unten
import server as _server  # noqa: E402 — teilt server._lade_env_dateien mit dem Server-Start

# .env-Naht (s. tests/test_env_loader.py): pytest rief server._lade_env_dateien() bisher NIE auf
# (nur server.main() tat das) — dadurch blieb z.B. $ELSTER_HERSTELLER_ID aus einer lokalen `.env`
# fuer die gesamte Suite unsichtbar. Reuse der bestehenden Funktion, kein neuer Mechanismus.
# Bestehendes Prozess-Env gewinnt IMMER (kein Override, s. Doku dort); fehlende .env = no-op.
_server._lade_env_dateien(_ROOT)

# ...ABER der LLM-Schlüssel wird für die Suite sofort wieder entfernt (2026-08-14).
# Seit .env.llm existiert, lud die Zeile darüber einen ECHTEN Key in jeden Testlauf. Folge:
# test_chat_501 bekam 200 statt der erwarteten Cap-Grenze — und, schwerer wiegend, jeder
# Suite-Durchlauf hätte echte, kostenpflichtige LLM-Calls abgesetzt (~20-40 s pro Aufruf).
# Tests, die den Chat prüfen, patchen llm_client.complete und brauchen den Key nicht; Tests,
# die die Cap-Grenze prüfen, brauchen seine ABWESENHEIT. Wer wirklich live testen will, setzt
# LLM_API_KEY im Testkörper selbst — dann ist es eine bewusste Entscheidung und steht im Test.
# Base und Modell bleiben stehen: ohne Key passiert damit ohnehin nichts (llm_client._key()
# wirft zuerst), und ihre Werte sind kein Geheimnis.
os.environ.pop("LLM_API_KEY", None)

# Zertifikats-PIN gehört nicht in die Umgebung jedes Testprozesses (Audit 2026-08-16,
# sec-elster-pin-in-every-test-process). Kein Test liest ihn (grep tests/ leer); der
# Versand (elster/versand.py) läuft manuell, nie unter pytest.
os.environ.pop("ELSTER_ZERTIFIKAT_PIN", None)

# Die Suite läuft im EXPLIZITEN Einzelnutzer-Modus: _fall_owner_check ist seit dem
# Audit-Fix (sec-authz-fail-open-no-token) fail-closed — ohne Token 401. Die 2000+
# Bestandstests fahren die API bewusst ohne Login; das ist jetzt eine sichtbare
# Entscheidung statt eines stillen Defaults. Negativtests (test_authz_fail_closed)
# löschen die Variable gezielt per monkeypatch.
os.environ["TAXGRAPH_NO_AUTH"] = "1"

_REAL_AUDIT_PFAD = os.path.abspath(os.path.join(_audit.AUDIT_DIR, "audit.jsonl"))


@pytest.fixture(autouse=True)
def _kein_schreiben_ins_echte_audit_log(request, monkeypatch):
    """Umwickelt audit.append fuer JEDEN Test und prueft bei jedem Aufruf den
    zu diesem Zeitpunkt aufgeloesten Pfad — direkt oder indirekt ueber Produktcode.
    Prozesslokal: sieht nur eigene Aufrufe, nicht das Wachstum der geteilten Datei
    (s. Docstring oben, Grund fuer den Wechsel weg von Dateigroesse).

    Ein Treffer wird GEBLOCKT statt durchgereicht (kein Aufruf von echtes_append):
    die echte Datei bleibt so auch bei einem Test, der es versucht, unangetastet.
    Der Verstoss wird erst NACH yield als assert gemeldet, nicht per raise im
    Wrapper selbst — server.py:165 faengt Exceptions aus dem Request-Dispatch
    breit ab ("nie eine nackte Exception nach aussen lecken"); ein raise dort
    wuerde als generisches 500 verschluckt statt den Test klar rot zu machen.
    """
    treffer: list[str] = []
    echtes_append = _audit.append

    def _wache(*args, **kwargs):
        pfad = os.path.abspath(_audit._audit_pfad())
        if pfad == _REAL_AUDIT_PFAD:
            treffer.append(f"pfad={pfad!r} args={args!r} kwargs={kwargs!r}")
            return None  # geblockt — echtes_append() NICHT aufgerufen, Datei unangetastet
        return echtes_append(*args, **kwargs)

    monkeypatch.setattr(_audit, "append", _wache)
    yield
    assert not treffer, (
        f"{request.node.nodeid} hat versucht, ins ECHTE audit.jsonl zu schreiben "
        f"({_REAL_AUDIT_PFAD}): " + "; ".join(treffer) + ". Schreibvorgang wurde geblockt, "
        'Datei ist unangetastet. Fix: monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path)) '
        "im Test/Fixture setzen."
    )


_REAL_FEHLER_PFAD = os.path.abspath(os.path.join(_audit.AUDIT_DIR, "fehler.log"))


@pytest.fixture(autouse=True)
def _kein_schreiben_ins_echte_fehler_log(request, monkeypatch):
    """Dieselbe Wache fuer das Fehler-Protokoll (produkt/store/fehler_log.py).

    Es liegt in derselben Ablage wie audit.jsonl und faellt deshalb NICHT unter die
    Wache darueber: die umwickelt audit.append, und protokolliere() ist ein anderer
    Aufruf. Ohne diese zweite Fixture schreibt jeder Test, der einen 500er ausloest,
    in die echte fehler.log des Nutzers -- genau die Fehlklasse, die das Audit-Log
    einmal auf 582 MB hat wachsen lassen.

    Gleiche Mechanik, gleiche Gruende: Pfad zur AUFRUFZEIT aufloesen (nicht beim
    Import), Treffer BLOCKEN statt durchreichen, und erst nach yield asserten --
    ein raise im Wrapper wuerde von server.py:229 als generisches 500 verschluckt.

    Ein Test, der das Protokoll pruefen WILL, lenkt audit.AUDIT_DIR auf tmp_path um;
    dann zeigt _pfad() woandershin und die Wache laesst ihn durch.
    """
    treffer: list[str] = []
    echtes_protokolliere = _fehler_log.protokolliere

    def _wache(*args, **kwargs):
        pfad = os.path.abspath(_fehler_log._pfad())
        if pfad == _REAL_FEHLER_PFAD:
            treffer.append(f"pfad={pfad!r} ort={(args[0] if args else kwargs.get('ort'))!r}")
            return None  # geblockt -- echte Datei unangetastet
        return echtes_protokolliere(*args, **kwargs)

    monkeypatch.setattr(_fehler_log, "protokolliere", _wache)
    yield
    assert not treffer, (
        f"{request.node.nodeid} hat versucht, ins ECHTE fehler.log zu schreiben "
        f"({_REAL_FEHLER_PFAD}): " + "; ".join(treffer) + ". Schreibvorgang wurde geblockt, "
        'Datei ist unangetastet. Fix: monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path)) '
        "im Test/Fixture setzen -- fehler_log folgt derselben Ablage."
    )
