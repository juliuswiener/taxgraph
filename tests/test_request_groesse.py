"""Der Anfrage-Rumpf hat ein Höchstmaß — und die Grenze liegt VOR dem Lesen.

Der Fund (Audit 2026-08-16, sec-unbounded-request-body), am 2026-08-18 nachgesehen:

    laenge = int(self.headers.get("Content-Length") or 0)
    roh = self.rfile.read(laenge) if laenge else b""

`rfile.read` liest so viel, wie der Client BEHAUPTET zu senden. Der Server ist einfädig
(server.py:make_server nutzt HTTPServer, nicht ThreadingHTTPServer — bewusst, catala_runtime
ist nicht threadsicher). Ein angekündigtes Gigabyte belegt deshalb nicht nur Speicher, sondern
hält den ganzen Dienst auf, solange gelesen wird. Dieselbe Angriffsfläche wie ein Unterprozess
ohne Zeitlimit (tests/test_unterprozess_zeitlimit.py), nur eine Schicht früher: dort hängt der
Server an der Verarbeitung, hier schon am Einlesen.

WARUM DIE PRÜFUNG VOR DEM LESEN STEHT: nachträglich zu prüfen, wie groß der gelesene Rumpf war,
hilft nicht — dann liegt er bereits im Speicher, und genau das war der Schaden. Der Header wird
geprüft, bevor ein einziges Byte des Rumpfes angefasst wird. test_grosser_rumpf_wird_nicht_
gelesen belegt das, statt sich mit dem Statuscode zufriedenzugeben: ein 413, das erst NACH dem
Einlesen kommt, sieht von aussen genauso aus und schützt nichts.

Dazu ein zweiter, kleinerer Fund derselben Zeile: `int(...)` auf einen nicht-numerischen
Content-Length-Header warf eine ValueError, die bis in die 500-Behandlung durchschlug. Eine
kaputte Anfrage ist kein Serverfehler.

NULL LLM.
"""
from __future__ import annotations

import json
import os
import socket
import sys
import threading

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/import", "produkt/bescheid",
             "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api as API       # noqa: E402
import audit            # noqa: E402
import server as SRV    # noqa: E402


@pytest.fixture
def basis(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    assert srv.server_address[0] == "127.0.0.1", "Auflage B: niemals 0.0.0.0"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield srv.server_address
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _roh_anfrage(adresse, kopfzeilen: str, rumpf: bytes = b"", lese_zeitlimit=5.0) -> str:
    """Eine Anfrage von Hand über den Socket — nötig, weil urllib den Content-Length-Header
    selbst setzt und eine LÜGE darin (mehr ankündigen als senden) gar nicht erst zulässt.
    Genau diese Lüge ist hier der Angriff."""
    s = socket.create_connection(adresse, timeout=lese_zeitlimit)
    try:
        s.sendall(kopfzeilen.encode() + rumpf)
        s.settimeout(lese_zeitlimit)
        antwort = b""
        while b"\r\n\r\n" not in antwort:
            stueck = s.recv(4096)
            if not stueck:
                break
            antwort += stueck
        return antwort.decode("utf-8", "replace")
    finally:
        s.close()


def test_zu_grosser_rumpf_wird_mit_413_abgewiesen(basis):
    """Der angekündigte Rumpf liegt über dem Höchstmaß — der Server muss ablehnen.

    Sendet wie der Test darunter KEINEN Rumpf; die beiden unterscheiden sich nur im Anspruch
    (hier: der Statuscode stimmt, dort: die Antwort kommt sofort, also vor dem Lesen). Der
    Vollständigkeit halber beides, weil ein späterer Umbau eines von beiden brechen kann."""
    zu_gross = SRV.MAX_BODY_BYTES + 1
    kopf = (f"POST /fall HTTP/1.1\r\nHost: 127.0.0.1\r\n"
            f"Content-Type: application/json\r\nContent-Length: {zu_gross}\r\n"
            f"Origin: http://127.0.0.1:{basis[1]}\r\nConnection: close\r\n\r\n")
    antwort = _roh_anfrage(basis, kopf)          # KEIN Rumpf gesendet — nur angekündigt
    assert "413" in antwort.split("\r\n")[0], (
        f"erwartet 413, bekommen: {antwort.splitlines()[0] if antwort else '(keine Antwort)'}")


def test_grosser_rumpf_wird_nicht_gelesen(basis):
    """Der eigentliche Schutz, und der Grund, warum der Statuscode allein nicht genügt: die
    Anfrage kündigt viel an und sendet NICHTS. Läse der Server erst und prüfte danach, bliebe er
    hier hängen, bis das Lese-Zeitlimit zuschlägt — bei einem einfädigen Server heisst das: der
    Dienst steht. Kommt die Antwort dagegen sofort, ist bewiesen, dass vor dem Lesen geprüft
    wird."""
    zu_gross = SRV.MAX_BODY_BYTES * 4
    kopf = (f"POST /fall HTTP/1.1\r\nHost: 127.0.0.1\r\n"
            f"Content-Type: application/json\r\nContent-Length: {zu_gross}\r\n"
            f"Origin: http://127.0.0.1:{basis[1]}\r\nConnection: close\r\n\r\n")
    # Kurzes Zeitlimit: wer erst liest und dann prüft, läuft hier in einen Timeout-Fehler
    # statt in eine Antwort. Der Test schlägt dann mit socket.timeout fehl — das ist die
    # gewünschte Aussage, nicht ein Testfehler.
    antwort = _roh_anfrage(basis, kopf, lese_zeitlimit=3.0)
    assert antwort, "keine Antwort — der Server wartet auf einen Rumpf, den er nie bekommt"
    assert "413" in antwort.split("\r\n")[0]


def test_content_length_ist_keine_zahl(basis):
    """Kaputte Anfrage, kein Serverfehler: `int("abc")` schlug vorher bis in die 500-Behandlung
    durch."""
    kopf = ("POST /fall HTTP/1.1\r\nHost: 127.0.0.1\r\n"
            "Content-Type: application/json\r\nContent-Length: abc\r\n"
            f"Origin: http://127.0.0.1:{basis[1]}\r\nConnection: close\r\n\r\n")
    antwort = _roh_anfrage(basis, kopf)
    kopfzeile = antwort.split("\r\n")[0]
    assert "400" in kopfzeile, f"erwartet 400, bekommen: {kopfzeile}"
    assert "500" not in kopfzeile


def test_normale_anfrage_geht_weiter_durch(basis):
    """Der Normalfall muss unbehelligt bleiben — ohne diesen Test wäre ein Höchstmaß von 0 die
    grünste Lösung (dieselbe Klasse wie ein Gate, das seine eigene Voraussetzung mitbringt)."""
    rumpf = json.dumps({"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                        "fall_id": "groessen_probe"}).encode()
    kopf = (f"POST /fall HTTP/1.1\r\nHost: 127.0.0.1\r\n"
            f"Content-Type: application/json\r\nContent-Length: {len(rumpf)}\r\n"
            f"Origin: http://127.0.0.1:{basis[1]}\r\nConnection: close\r\n\r\n")
    antwort = _roh_anfrage(basis, kopf, rumpf)
    kopfzeile = antwort.split("\r\n")[0]
    assert "413" not in kopfzeile and "400" not in kopfzeile, (
        f"eine gewöhnliche Anfrage wird abgewiesen: {kopfzeile}")


def test_hoechstmass_ist_nicht_wegdefiniert():
    """Gegenprobe zur Konstante selbst: ein absurd hohes Höchstmaß wäre so gut wie keines, ein
    absurd niedriges bräche den PDF-Upload. Die Spanne hält beide Richtungen offen genug für
    eine bewusste Änderung und eng genug, dass sie auffällt."""
    assert 1 * 1024 * 1024 <= SRV.MAX_BODY_BYTES <= 128 * 1024 * 1024, (
        f"MAX_BODY_BYTES={SRV.MAX_BODY_BYTES} liegt ausserhalb des sinnvollen Bereichs — ein "
        f"PDF kommt base64-kodiert im JSON-Rumpf (+ ein Drittel), zu klein bricht den Upload, "
        f"zu gross hebt den Schutz auf.")
