"""pytest-Plugin `ui_rust_plugin`: die Playwright-Tests der Oberfläche gegen den RUST-Server.

REWRITE_PLAN §6/§7 (9c): "Playwright gegen Rust". Die 22 UI-Dateien in `tests/` starten den Python-Server
im Prozess (`SRV.make_server(0)`). Dieses Plugin ersetzt genau diese EINE Funktion durch den Start von
`taxgraph-api` mit demselben Datenverzeichnis, das die Fixture der Datei vorher in `API.FAELLE` und
`AUTH.USER_STORE` gesetzt hat. Der Test-Code bleibt unverändert. Aufruf: `make ui-rust`.

Eine zweite Fixture gibt es nicht. Was das Plugin sonst tut:

* `TAXGRAPH_NO_AUTH`: Rust liest die Variable beim Start, die Tests schalten sie nach dem Start um
  (`monkeypatch.delenv`). `os.environ` meldet das Umschalten, das Plugin startet Rust auf demselben
  Port neu. Der Test sieht davon nichts.
* Der Rust-Server schreibt nur unter dem `tmp_path` des Laufs. Zeigt `API.FAELLE` anderswohin
  (der echte Bestand!), bricht der Test ab, bevor Rust startet.
* `ausschluss.tsv` (neben dieser Datei) nennt die Tests, die gegen Rust NICHT grün werden können, mit
  einer Ursache je Zeile. Sie laufen als `xfail(strict=True)`: wird ein gelisteter Test grün, ist der
  Eintrag veraltet und der Lauf rot. Fehlt ein gelisteter Test in seiner Datei, ist der Eintrag veraltet
  und der Lauf rot (vor dem ersten Test). Ein Test OHNE Eintrag, der rot wird, bleibt rot.
* Die Schlusszeile zählt, wie viele grüne Tests einen Rust-Server gestartet haben. Ein grüner Test ohne
  Server (er ruft Python im Prozess auf) beweist nichts über Rust und steht darum getrennt.
* `UI_RUST_GEGENPROBE=1` beendet jeden Rust-Prozess gleich nach dem Start (`make ui-rust-gegenprobe`):
  die Tests müssen rot werden, sonst redeten sie nicht mit Rust.

Umgebung: `UI_RUST_BIN` (Standard `$CARGO_TARGET_DIR/debug/taxgraph-api`, sonst `rust/target/...`).
"""
from __future__ import annotations

import ast
import os
import subprocess
import threading
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
AUSSCHLUSS = Path(__file__).with_name("ausschluss.tsv")
URSACHEN = ("LLM-Stub", "ERiC-Stub")
EIGENSCHAFT = "ui_rust_server"

_LEBENDE: list["_RustServer"] = []


def _binary() -> Path:
    gesetzt = os.environ.get("UI_RUST_BIN")
    if gesetzt:
        return Path(gesetzt)
    ziel = os.environ.get("CARGO_TARGET_DIR") or str(ROOT / "rust" / "target")
    return Path(ziel) / "debug" / "taxgraph-api"


def lies_ausschluss(pfad: Path = AUSSCHLUSS) -> dict[str, str]:
    """`{nodeid: ursache}`. Eine Zeile: `tests/test_x.py::test_name<TAB>Ursache`. `#` beginnt einen Kommentar."""
    eintraege: dict[str, str] = {}
    for nr, roh in enumerate(pfad.read_text(encoding="utf-8").splitlines(), 1):
        zeile = roh.split("#", 1)[0].strip()
        if not zeile:
            continue
        teile = zeile.split("\t")
        if len(teile) != 2 or teile[1] not in URSACHEN:
            raise pytest.UsageError(f"{pfad.name}:{nr}: erwartet `nodeid<TAB>Ursache`, Ursache aus {URSACHEN}: {roh!r}")
        if teile[0] in eintraege:
            raise pytest.UsageError(f"{pfad.name}:{nr}: {teile[0]} steht zweimal in der Liste")
        eintraege[teile[0]] = teile[1]
    return eintraege


def _veraltete_eintraege(eintraege: dict[str, str]) -> list[str]:
    """Eintraege, deren Datei oder Test es nicht mehr gibt (statisch gelesen, ohne Sammeln)."""
    veraltet = []
    for nodeid in eintraege:
        datei, _, rest = nodeid.partition("::")
        name = rest.split("::")[-1].split("[")[0]
        pfad = ROOT / datei
        if not pfad.is_file():
            veraltet.append(f"{nodeid}: Datei fehlt")
            continue
        baum = ast.parse(pfad.read_text(encoding="utf-8"))
        if not any(isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)) and n.name == name for n in ast.walk(baum)):
            veraltet.append(f"{nodeid}: Test fehlt in der Datei")
    return veraltet


class _RustServer:
    """Ein `taxgraph-api`-Prozess, der sich wie `http.server.HTTPServer` bedienen lässt (so ruft ihn die Fixture)."""

    def __init__(self, daten: Path, users: str | None):
        self.daten, self.users, self.port, self.zu = daten, users, 0, False
        self._halt = threading.Event()
        self._start()
        _LEBENDE.append(self)

    def _start(self) -> None:
        env = dict(os.environ)
        env.update(
            TAXGRAPH_ROOT=str(ROOT),
            TAXGRAPH_DATEN=str(self.daten),
            TAXGRAPH_AUDIT_DIR=str(self.daten / "faelle"),
            TAXGRAPH_USER_STORE=self.users or str(self.daten / "users.json"),
            TAXGRAPH_JWT_SECRET="ui-rust-geheimnis",
            TAXGRAPH_FLOW="0",
            LLM_API_KEY="", LLM_API_BASE="", LLM_MODEL="", ORS_API_KEY="",
            ELSTER_HERSTELLER_ID="", ERIC_DIR="/nicht/vorhanden",
        )
        env.pop("XDG_DATA_HOME", None)
        (self.daten / "faelle").mkdir(parents=True, exist_ok=True)
        self.proc = subprocess.Popen(
            [str(_binary()), str(self.port)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, env=env
        )
        zeile = self.proc.stdout.readline()
        if "http://127.0.0.1:" not in zeile:
            raise RuntimeError(f"taxgraph-api meldet keinen Port (Startzeile {zeile!r}, rc={self.proc.poll()})")
        self.port = int(zeile.split("http://127.0.0.1:")[1].split()[0])
        self.server_address = ("127.0.0.1", self.port)
        proc = self.proc

        def leer() -> None:  # stdout leerlesen, sonst blockiert der Dienst bei vollem Pipe-Puffer
            for _ in proc.stdout:
                pass

        threading.Thread(target=leer, daemon=True).start()
        if os.environ.get("UI_RUST_GEGENPROBE") == "1":
            self.proc.terminate()
            self.proc.wait(timeout=10)

    def neustart(self) -> None:
        if self.zu:
            return
        self.proc.terminate()
        self.proc.wait(timeout=10)
        self._start()

    # --- die drei Aufrufe der Fixtures (`http.server`-Schnittstelle)
    def serve_forever(self) -> None:
        self._halt.wait()

    def shutdown(self) -> None:
        self.zu = True
        self._halt.set()
        self.proc.terminate()

    def server_close(self) -> None:
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()


class _Umgebung(os._Environ):
    """`os.environ`, das das Umschalten von `TAXGRAPH_NO_AUTH` an die laufenden Rust-Server meldet."""

    def _melde(self, key: str) -> None:
        if key == "TAXGRAPH_NO_AUTH":
            for server in list(_LEBENDE):
                server.neustart()

    def __setitem__(self, key, value):
        alt = self.get(key)
        super().__setitem__(key, value)
        if alt != value:
            self._melde(key)

    def __delitem__(self, key):
        super().__delitem__(key)
        self._melde(key)


def pytest_configure(config) -> None:
    if not _binary().is_file():
        raise pytest.UsageError(f"Rust-Binary fehlt: {_binary()} (`make ui-rust` baut es, oder UI_RUST_BIN setzen)")
    eintraege = lies_ausschluss()
    veraltet = _veraltete_eintraege(eintraege)
    if veraltet:
        raise pytest.UsageError("tools/ui_rust/ausschluss.tsv ist veraltet:\n  " + "\n  ".join(veraltet))
    config._ui_rust_ausschluss = eintraege
    os.environ.__class__ = _Umgebung


def pytest_unconfigure(config) -> None:
    os.environ.__class__ = os._Environ
    for server in _LEBENDE:
        server.zu = True
        if server.proc.poll() is None:
            server.proc.kill()


def pytest_collection_modifyitems(config, items) -> None:
    for item in items:
        ursache = config._ui_rust_ausschluss.get(item.nodeid)
        if ursache:
            item.add_marker(pytest.mark.xfail(
                strict=True,
                reason=f"{ursache} (tools/ui_rust/ausschluss.tsv; wird der Test grün, ist der Eintrag veraltet)",
            ))


@pytest.fixture(autouse=True)
def _server_ist_rust(request, monkeypatch, tmp_path_factory):
    import api as API
    import auth as AUTH
    import server as SRV

    erlaubt = tmp_path_factory.getbasetemp().resolve()

    def make_server(port=0, *args, **kwargs):
        faelle = Path(API.FAELLE).resolve()
        if not faelle.is_relative_to(erlaubt):
            pytest.fail(f"API.FAELLE={faelle} liegt nicht unter {erlaubt}: Rust würde den echten Bestand beschreiben")
        server = _RustServer(faelle.parent, getattr(AUTH, "USER_STORE", None))
        request.node.user_properties.append((EIGENSCHAFT, server.port))
        return server

    monkeypatch.setattr(SRV, "make_server", make_server)
    yield


def pytest_terminal_summary(terminalreporter) -> None:
    gruen = terminalreporter.stats.get("passed", [])
    gruen = [r for r in gruen if r.when == "call"]
    mit = [r for r in gruen if any(k == EIGENSCHAFT for k, _ in r.user_properties)]
    ohne = [r.nodeid for r in gruen if r not in mit]
    ausgeschlossen = terminalreporter.stats.get("xfailed", [])
    nach_ursache = {u: sum(1 for r in ausgeschlossen if str(getattr(r, "wasxfail", "")).startswith(u)) for u in URSACHEN}
    terminalreporter.write_sep("-", "ui-rust")
    terminalreporter.write_line(
        f"ui-rust: {len(gruen)} grün, davon {len(mit)} mit Rust-Server und {len(ohne)} ohne Server (Python im Prozess); "
        f"{len(ausgeschlossen)} laut Liste ausgeschlossen {nach_ursache}"
    )
    for nodeid in ohne:
        terminalreporter.write_line(f"  ohne Rust-Server grün: {nodeid}")
