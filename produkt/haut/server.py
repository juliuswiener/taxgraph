"""Paket-B Haut — HTTP-Transport (stdlib, kein Framework). Dünne Schale um produkt/haut/api.py.

Auflagen: (A) POST …/chat -> 501 mit erklärendem Body, nie 200-Fake; (B) bindet AUSSCHLIESSLICH
127.0.0.1 (Nutzerdaten, kein 0.0.0.0); die Transport-Schicht ist absichtlich austauschbar
(Upgrade-Pfad stdlib -> uvicorn), weil api.py reine (request)->(status,obj)-Funktionen liefert.

Start:  python -m produkt.haut.server   (oder: python produkt/haut/server.py [port])
"""
from __future__ import annotations

import json
import os
import re
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import api  # noqa: E402

HOST = "127.0.0.1"          # Auflage B — niemals 0.0.0.0
STATIC = os.path.join(HERE, "static")

_CTYPE = {".html": "text/html; charset=utf-8", ".js": "application/javascript; charset=utf-8",
          ".css": "text/css; charset=utf-8", ".json": "application/json; charset=utf-8"}

# (Methode, kompiliertes Muster) -> Handler(match, body) -> (status, obj)
_ID = r"(?P<id>[A-Za-z0-9_-]{1,64})"
_FID = r"(?P<fid>[A-Za-z0-9_]{1,64})"


def _routes():
    return [
        ("POST", re.compile(r"^/fall$"), lambda m, b: api.fall_anlegen(b)),
        ("GET", re.compile(rf"^/fall/{_ID}/fragen$"), lambda m, b: api.fragen(m["id"])),
        ("GET", re.compile(rf"^/fall/{_ID}/stand$"), lambda m, b: api.stand(m["id"])),
        ("POST", re.compile(rf"^/fall/{_ID}/event$"), lambda m, b: api.event(m["id"], b)),
        ("GET", re.compile(rf"^/fall/{_ID}/feld/{_FID}/warum$"), lambda m, b: api.warum(m["id"], m["fid"])),
        ("GET", re.compile(rf"^/fall/{_ID}/ergebnis$"), lambda m, b: api.ergebnis(m["id"])),
        ("GET", re.compile(rf"^/fall/{_ID}/deklaration$"), lambda m, b: api.deklaration(m["id"])),
        ("GET", re.compile(rf"^/fall/{_ID}/graph$"), lambda m, b: api.graph(m["id"])),
        ("POST", re.compile(rf"^/fall/{_ID}/elster-ampel$"), lambda m, b: (503, api.AMPEL_503)),
        ("POST", re.compile(rf"^/fall/{_ID}/chat$"), lambda m, b: (501, api.CHAT_501)),
        # Arbeitsweg-Entfernung über Karten-Dienst (ORS): Vorschlag-Fluss; kein Key/Fehler → 503-Fallback.
        ("POST", re.compile(rf"^/fall/{_ID}/entfernung$"), lambda m, b: api.entfernung(m["id"], b)),
        # Vorjahr-Übernahme: Vorjahres-Fall → vorläufige Vorschläge (herkunft=vorjahr) im aktuellen Fall.
        ("POST", re.compile(rf"^/fall/{_ID}/vorjahr$"), lambda m, b: api.vorjahr(m["id"], b)),
    ]


ROUTES = _routes()


class Handler(BaseHTTPRequestHandler):
    server_version = "TaxGraphHaut/1.0"

    def log_message(self, *a):        # keine Nutzerdaten in die Server-Logzeile
        pass

    # -- Antworten --
    def _json(self, status: int, obj) -> None:
        payload = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def _static(self, pfad: str) -> None:
        rel = pfad[len("/static/"):] if pfad.startswith("/static/") else "index.html"
        root = os.path.realpath(STATIC)
        voll = os.path.realpath(os.path.join(root, rel))
        # Path-Traversal hart abfangen: der aufgelöste Pfad MUSS unter static/ liegen.
        if os.path.commonpath([root, voll]) != root or not os.path.isfile(voll):
            self._json(404, {"fehler": "not_found", "pfad": pfad})
            return
        ext = os.path.splitext(voll)[1]
        with open(voll, "rb") as f:
            data = f.read()
        self.send_response(200)
        self.send_header("Content-Type", _CTYPE.get(ext, "application/octet-stream"))
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    # -- Dispatch --
    def _dispatch(self, method: str) -> None:
        pfad = self.path.split("?", 1)[0]
        if method == "GET" and (pfad == "/" or pfad.startswith("/static/")):
            self._static(pfad)
            return
        body = {}
        if method == "POST":
            laenge = int(self.headers.get("Content-Length") or 0)
            roh = self.rfile.read(laenge) if laenge else b""
            if roh:
                try:
                    body = json.loads(roh)
                except json.JSONDecodeError:
                    self._json(400, {"fehler": "ungültiges JSON im Body"})
                    return
        for m, muster, fn in ROUTES:
            if m != method:
                continue
            treffer = muster.match(pfad)
            if treffer:
                try:
                    status, obj = fn(treffer.groupdict(), body)
                except api.ApiError as e:
                    self._json(e.status, {"fehler": str(e)})
                except Exception as e:  # nie eine nackte Exception nach aussen lecken
                    self._json(500, {"fehler": f"{type(e).__name__}: {e}"})
                else:
                    self._json(status, obj)
                return
        self._json(404, {"fehler": "route_not_found", "methode": method, "pfad": pfad})

    def do_GET(self):
        self._dispatch("GET")

    def do_POST(self):
        self._dispatch("POST")


def make_server(port: int = 8000) -> HTTPServer:
    """Bindet 127.0.0.1:port (port=0 -> freier Port, für Tests). Aufrufer ruft serve_forever().

    SINGLE-THREADED (HTTPServer, NICHT ThreadingHTTPServer) — bewusst: catala_runtime ist NICHT
    thread-safe (globaler `max_decimals`/`log`-State, catala_runtime.py, steuert die Money-Rundung).
    Die Haut feuert /stand + /ergebnis parallel (Browser serialisiert XHRs nicht); mit echten Request-
    Threads würden zwei catala-Berechnungen am Global racen → falsche Rundung → FALSCHER BESCHEID (K2).
    Ein Handler je Zeit killt die Concurrency an der Quelle; für die 127.0.0.1-Einzelnutzer-App ist die
    Serialisierungs-Latenz irrelevant. NICHT auf ThreadingHTTPServer zurückstellen ohne catala-Lock."""
    return HTTPServer((HOST, port), Handler)


def main(argv):
    port = int(argv[1]) if len(argv) > 1 else 8000
    srv = make_server(port)
    host, gebunden = srv.server_address[0], srv.server_address[1]
    print(f"TaxGraph-Haut auf http://{host}:{gebunden}  (Ctrl-C zum Beenden)")
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        srv.shutdown()


if __name__ == "__main__":
    main(sys.argv)
