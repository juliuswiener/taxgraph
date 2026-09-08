"""SIGTERM muss den Server-PROZESS beenden — Vault-Ticket server-shutdown-haengt-sigterm.

DER DEFEKT (Handmessung 2026-08-21/25 am eigenen Prozess, `ps` zeigt `wchan
poll_schedule_timeout`): `produkt/haut/server.py:main()` schickte SIGTERM über einen Umweg an
KeyboardInterrupt weiter und rief danach `srv.shutdown()` im `finally` auf — beides aus
DEMSELBEN Thread, in dem `serve_forever()` lief. Betrieblich: `systemctl stop` hängt, der Port
bleibt belegt, der Neustart scheitert, während der ALTE Server unbemerkt weiterläuft und mit
HTTP 200 antwortet.

WARUM NUR EIN EIGENER PROZESS DEN FEHLER SIEHT: die 69 vorhandenen Server-Tests
(`tests/test_paket_b_e2e_http.py` u.a.) starten den Server per `threading.Thread(target=
srv.serve_forever)` und rufen `srv.shutdown()` aus dem TEST-Thread auf — das ist der korrekte
Cross-Thread-Aufruf, bei dem der Fehler gar nicht auftritt. `main()` wird von keinem dieser
Tests je aufgerufen. Nur wer den Server als eigenen Prozess startet und ihm SIGTERM schickt,
durchläuft den kaputten Pfad.

ZWEI FASSUNGEN GEMESSEN, NUR EINE TRÄGT: der naheliegende Einzeiler (`shutdown()` im `finally`
durch `server_close()` ersetzen) wurde gebaut und gegen GENAU DIESEN Test gemessen — er blieb
ROT, derselbe Hänger, weil `serve_forever()` seine Blockade nach SIGTERM nie verlässt (der
Defekt sitzt vor dem `finally`, nicht darin). Erst der Fix im Handler selbst (Event statt
Signal-Weiterleitung, `shutdown()` aus einem eigenen Watcher-Thread) macht diesen Test grün.
Die genaue CPython-Mechanik, warum das Signal-Relay (`os.kill(SIGINT)` aus dem SIGTERM-Handler)
serve_forever() nicht zuverlässig unterbricht, ist NICHT abschließend geklärt — belegt ist nur
das Verhalten an diesem Test, vor und nach beiden Fassungen.

ZWEITER DEFEKT, gefunden 2026-09-08 unter Parallellast (5 von 25 Läufen rot, solo 0 von 10):
kein Hänger, sondern `rc=-15` — der Prozess starb am STANDARDVERHALTEN von SIGTERM. Die
Handler-Installation stand nach der Bereit-Meldung, SIGINT hatte gar keinen eigenen Handler.
Zwischen "Server ist gebunden und sagt es" und "Server hört auf Signale" lag also ein Fenster,
das unter Last breit genug wurde, um getroffen zu werden. Mechanismus deterministisch belegt,
nicht erschlossen: eine 0,5-s-Sonde in genau dieses Fenster gelegt macht BEIDE Tests 5 von 5
rot (SIGTERM rc=-15, SIGINT rc=-2); dieselbe Sonde hinter die Handler gelegt lässt sie 5 von 5
grün. Betriebliche Folge war dieselbe wie beim ersten Defekt und deshalb leicht zu verwechseln:
kein `server_close()`, Port bleibt belegt, Neustart scheitert.
"""
from __future__ import annotations

import os
import queue
import signal
import subprocess
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SERVER = os.path.join(ROOT, "produkt", "haut", "server.py")

# Prozessstart (Interpreter + Modul-Importe von api.py & Co.) unter Parallellast (-n 6);
# großzügig, damit CPU-Kontention beim gleichzeitigen Testlauf keinen Fehlalarm auslöst.
STARTUP_TIMEOUT = 15.0

# Ein korrekter Shutdown kehrt binnen Millisekunden zurück (Bind schließen, kein Warten).
# Der kaputte Pfad hängt dagegen UNBEGRENZT (Event, das nie gesetzt wird) — 8s in der
# Handmessung war nur der Beobachtungszeitpunkt, kein Ablauf. 5s liegt weit über jeder
# realistischen Verzögerung durch Scheduling/GC unter Last und weit unter "unbegrenzt".
SIGNAL_TIMEOUT = 5.0


def _starte_server():
    """Startet server.py als eigenen Prozess (port=0 -> freier Port) und wartet, bis er
    gebunden hat. Gibt (proc, zeilen-Queue) zurück; Aufrufer räumt proc im finally auf."""
    env = dict(os.environ, PYTHONUNBUFFERED="1")
    proc = subprocess.Popen(
        [sys.executable, SERVER, "0"],
        cwd=ROOT, env=env,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
    )
    zeilen: "queue.Queue[str]" = queue.Queue()

    def _lies_stdout():
        for zeile in proc.stdout:
            zeilen.put(zeile)

    threading.Thread(target=_lies_stdout, daemon=True).start()

    gebunden = False
    gesehen = []
    deadline = time.monotonic() + STARTUP_TIMEOUT
    while time.monotonic() < deadline:
        try:
            zeile = zeilen.get(timeout=0.2)
        except queue.Empty:
            continue
        gesehen.append(zeile)
        if "TaxGraph-Haut auf" in zeile:
            gebunden = True
            break
    if not gebunden:
        proc.kill()
        proc.wait(timeout=5)
        raise AssertionError(
            f"Server hat nicht binnen {STARTUP_TIMEOUT}s gestartet, Ausgabe bisher: {gesehen!r}"
        )
    return proc, zeilen


def test_sigterm_beendet_prozess_binnen_zeitlimit():
    proc, _ = _starte_server()
    try:
        proc.send_signal(signal.SIGTERM)
        try:
            rc = proc.wait(timeout=SIGNAL_TIMEOUT)
        except subprocess.TimeoutExpired:
            raise AssertionError(
                f"Prozess (pid={proc.pid}) lebt noch {SIGNAL_TIMEOUT}s nach SIGTERM — "
                "haengt (Ticket server-shutdown-haengt-sigterm)"
            ) from None
        assert rc == 0, f"Server-Prozess endete mit Exit-Code {rc}, erwartet 0"
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait(timeout=5)


def test_sigint_beendet_prozess_binnen_zeitlimit():
    """Auflage: der Fix darf das gewöhnliche Strg-C (SIGINT) im Terminal nicht kaputt machen.
    Der Watcher-Thread aus dem SIGTERM-Fix darf `serve_forever()`s eigenen
    KeyboardInterrupt-Pfad (Strg-C) weder blockieren noch den Prozess am Leben halten, wenn
    dieser andere Pfad greift statt des Watcher-Threads."""
    proc, _ = _starte_server()
    try:
        # KEINE Ruhezeit vor dem Signal (2026-09-08): hier stand ein `time.sleep(0.3)` mit der
        # Begründung, ein Signal direkt nach der Bereit-Meldung sei "ein Setup-Detail dieses
        # Tests, kein Verhalten von serve_forever() selbst". Das war falsch, und der Satz deckte
        # den Defekt zu: die Signal-Handler wurden erst NACH der Bereit-Meldung installiert, also
        # behielten beide Signale in genau diesem Fenster ihr Standardverhalten (SIGTERM tötet,
        # rc=-15; ungefangener KeyboardInterrupt -> CPython schickt sich selbst SIGINT, rc=-2).
        # Ohne die Ruhezeit trifft der Test das Fenster -- das ist der Nutzerpfad "warte auf die
        # Bereit-Meldung, dann stoppe den Dienst", nicht ein Testartefakt.
        proc.send_signal(signal.SIGINT)
        try:
            rc = proc.wait(timeout=SIGNAL_TIMEOUT)
        except subprocess.TimeoutExpired:
            raise AssertionError(
                f"Prozess (pid={proc.pid}) lebt noch {SIGNAL_TIMEOUT}s nach SIGINT (Strg-C)"
            ) from None
        assert rc == 0, f"Server-Prozess endete mit Exit-Code {rc}, erwartet 0"
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait(timeout=5)
