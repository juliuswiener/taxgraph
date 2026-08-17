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
import signal
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
if HERE not in sys.path:
    sys.path.insert(0, HERE)
_AUTH_DIR = os.path.join(PRODUKT, "auth")
if _AUTH_DIR not in sys.path:
    sys.path.insert(0, _AUTH_DIR)
_STORE_DIR = os.path.join(PRODUKT, "store")
if _STORE_DIR not in sys.path:
    sys.path.insert(0, _STORE_DIR)

import api  # noqa: E402
import api_auth  # noqa: E402 — Request-Auth (Modul-Attribute)
import auth  # noqa: E402 — P1.1 Authentifizierung
import audit  # noqa: E402 — P1.6 Audit-Log


def _session_check():
    """GET /auth/session — JWT schon von _dispatch via api_auth._AUTH_USER verifiziert."""
    uid = api_auth._AUTH_USER
    if uid is None:
        raise auth.AuthError(401, "ungültiges oder abgelaufenes Token")
    return 200, {"username": uid, "authenticated": True}

HOST = "127.0.0.1"          # Auflage B — niemals 0.0.0.0
STATIC = os.path.join(HERE, "static")

_CTYPE = {".html": "text/html; charset=utf-8", ".js": "application/javascript; charset=utf-8",
          ".css": "text/css; charset=utf-8", ".json": "application/json; charset=utf-8"}

# (Methode, kompiliertes Muster) -> Handler(match, body) -> (status, obj)
_ID = r"(?P<id>[A-Za-z0-9_-]{1,64})"
_FID = r"(?P<fid>[A-Za-z0-9_]{1,64})"


def _routes():
    return [
        # P8.3 Health / Ready
        ("GET", re.compile(r"^/health$"), lambda m, b: api.health()),
        ("GET", re.compile(r"^/ready$"), lambda m, b: api.ready()),
        # P1.1 Auth (kein _fall_owner_check — öffentlich)
        ("POST", re.compile(r"^/auth/register$"), lambda m, b: auth.register(b)),
        ("POST", re.compile(r"^/auth/login$"),
         lambda m, b: auth.login(b, audit_fn=lambda uid, act, fid, det: audit.append(uid, act, fid, det))),
        ("POST", re.compile(r"^/auth/logout$"),
         lambda m, b: auth.logout(b, audit_fn=lambda uid, act, fid, det: audit.append(uid, act, fid, det))),
        ("GET", re.compile(r"^/auth/session$"),
         lambda m, b: _session_check()),
        # Fall-Endpunkte
        ("POST", re.compile(r"^/fall$"), lambda m, b: api.fall_anlegen(b)),
        ("GET", re.compile(rf"^/fall/{_ID}/fragen$"), lambda m, b: api.fragen(m["id"])),
        ("GET", re.compile(rf"^/fall/{_ID}/stand$"), lambda m, b: api.stand(m["id"])),
        ("POST", re.compile(rf"^/fall/{_ID}/event$"), lambda m, b: api.event(m["id"], b)),
        ("GET", re.compile(rf"^/fall/{_ID}/feld/{_FID}/warum$"), lambda m, b: api.warum(m["id"], m["fid"])),
        ("GET", re.compile(rf"^/fall/{_ID}/ergebnis$"), lambda m, b: api.ergebnis(m["id"])),
        ("GET", re.compile(rf"^/fall/{_ID}/preflight$"), lambda m, b: api.preflight_check(m["id"])),
        ("GET", re.compile(rf"^/fall/{_ID}/deklaration$"), lambda m, b: api.deklaration(m["id"])),
        # P3.2b Einreichen: Deklaration → ELSTER-XML → checkESt (ERIC_VALIDIERE, offline).
        # Prüft nur; Versand (ERIC_ENCRYPT_AND_SEND) ist bewusst nicht verdrahtet.
        ("POST", re.compile(rf"^/fall/{_ID}/einreichen$"), lambda m, b: api.einreichen(m["id"], b)),
        ("GET", re.compile(rf"^/fall/{_ID}/graph$"), lambda m, b: api.graph(m["id"])),
        ("POST", re.compile(rf"^/fall/{_ID}/elster-ampel$"), lambda m, b: (503, api.AMPEL_503)),
        ("POST", re.compile(rf"^/fall/{_ID}/chat$"), lambda m, b: api.chat(m["id"], b)),
        # Arbeitsweg-Entfernung über Karten-Dienst (ORS): Vorschlag-Fluss; kein Key/Fehler → 503-Fallback.
        ("POST", re.compile(rf"^/fall/{_ID}/entfernung$"), lambda m, b: api.entfernung(m["id"], b)),
        # Vorjahr-Übernahme: Vorjahres-Fall → vorläufige Vorschläge (herkunft=vorjahr) im aktuellen Fall.
        ("POST", re.compile(rf"^/fall/{_ID}/vorjahr$"), lambda m, b: api.vorjahr(m["id"], b)),
        # Kontoauszug-Upload (csv/json/pdf, alle det+LLM-Fallback): Transaktion-Vorschläge (herkunft=kontoauszug).
        ("POST", re.compile(rf"^/fall/{_ID}/kontoauszug$"), lambda m, b: api.kontoauszug(m["id"], b)),
        # P1.7 DSGVO-Löschung: Fall-Datei entfernen, Audit behalten.
        ("DELETE", re.compile(rf"^/fall/{_ID}$"), lambda m, b: api.fall_loeschen(m["id"])),
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
    def _extract_user(self) -> str | None:
        """Liest Authorization-Header, verifiziert JWT → user_id oder None."""
        raw = (self.headers.get("Authorization") or self.headers.get("authorization") or "").strip()
        if raw.startswith("Bearer "):
            raw = raw[7:]
        if not raw:
            return None
        return auth.verify_token(raw)

    def _dispatch(self, method: str) -> None:
        pfad = self.path.split("?", 1)[0]
        if method == "GET" and (pfad == "/" or pfad.startswith("/static/")):
            self._static(pfad)
            return
        # Boundary-Checks (Audit 2026-08-16, sec-csrf-simple-request-write): eine besuchte
        # Webseite kann sonst per CORS-Simple-Request (text/plain, kein Preflight) in den
        # lokalen Fall schreiben. Fremder Host = DNS-Rebinding; Content-Type-Zwang auf
        # application/json erzwingt den Preflight, den kein Access-Control-Allow-Origin beantwortet.
        host = (self.headers.get("Host") or "").split(":")[0]
        if host not in ("127.0.0.1", "localhost", ""):
            self._json(421, {"fehler": "unerwarteter_host"})
            return
        origin = self.headers.get("Origin")
        if method in ("POST", "DELETE") and origin is not None:
            o_host = origin.split("//", 1)[-1].split("/")[0].split(":")[0]
            if o_host not in ("127.0.0.1", "localhost"):
                self._json(403, {"fehler": "cross_origin_verboten"})
                return
        body = {}
        if method == "POST":
            laenge = int(self.headers.get("Content-Length") or 0)
            if laenge and not (self.headers.get("Content-Type") or "").startswith("application/json"):
                self._json(415, {"fehler": "Content-Type muss application/json sein"})
                return
            roh = self.rfile.read(laenge) if laenge else b""
            if roh:
                try:
                    body = json.loads(roh)
                except json.JSONDecodeError:
                    self._json(400, {"fehler": "ungültiges JSON im Body"})
                    return
        # JWT-Kontext für DIESEN Request setzen (single-threaded → Modul-Variable sicher)
        api_auth._AUTH_USER = self._extract_user()
        try:
            for m, muster, fn in ROUTES:
                if m != method:
                    continue
                treffer = muster.match(pfad)
                if treffer:
                    status = 500
                    try:
                        status, obj = fn(treffer.groupdict(), body)
                    except (api.ApiError, auth.AuthError) as e:
                        self._json(e.status, {"fehler": str(e)})
                    except Exception as e:  # nie eine nackte Exception nach aussen lecken
                        self._json(500, {"fehler": f"{type(e).__name__}: {e}"})
                    else:
                        self._json(status, obj)
                    if pfad.startswith("/fall"):
                        uid = api_auth._AUTH_USER or "dev"
                        fid = treffer.groupdict().get("id")
                        if pfad == "/fall":
                            audit.append(uid, "fall_create", fid, f"status={status}")
                        elif fid:
                            act = pfad.split("/")[-1]
                            audit.append(uid, f"fall_{act}", fid, f"status={status}")
                    return
            self._json(404, {"fehler": "route_not_found", "methode": method, "pfad": pfad})
        finally:
            api_auth._AUTH_USER = None  # Request-Kontext sauber räumen

    def do_GET(self):
        self._dispatch("GET")

    def do_POST(self):
        self._dispatch("POST")

    def do_DELETE(self):
        self._dispatch("DELETE")


def make_server(port: int = 8000) -> HTTPServer:
    """Bindet 127.0.0.1:port (port=0 -> freier Port, für Tests). Aufrufer ruft serve_forever().

    SINGLE-THREADED (HTTPServer, NICHT ThreadingHTTPServer) — bewusst: catala_runtime ist NICHT
    thread-safe (globaler `max_decimals`/`log`-State, catala_runtime.py, steuert die Money-Rundung).
    Die Haut feuert /stand + /ergebnis parallel (Browser serialisiert XHRs nicht); mit echten Request-
    Threads würden zwei catala-Berechnungen am Global racen → falsche Rundung → FALSCHER BESCHEID (K2).
    Ein Handler je Zeit killt die Concurrency an der Quelle; für die 127.0.0.1-Einzelnutzer-App ist die
    Serialisierungs-Latenz irrelevant. NICHT auf ThreadingHTTPServer zurückstellen ohne catala-Lock."""
    return HTTPServer((HOST, port), Handler)


def _lade_env_dateien(root: str) -> None:
    """Lädt gitignored Env-Dateien (`.env.maps` für $ORS_API_KEY, `.env.llm` für den LLM-Key, `.env` für
    Sonstiges wie $ELSTER_HERSTELLER_ID) aus `root` in os.environ — NUR Schlüssel, die noch NICHT gesetzt
    sind (das echte Prozess-Env gewinnt IMMER, kein Override).
    Fehlt/unlesbar → still übersprungen (nie Crash beim Start). Keine externe Dependency (kein python-dotenv).
    Zeilenformat KEY=VALUE, `#` = Kommentar, Anführungszeichen werden getrimmt. WERTE werden NIE geloggt (Secrets).
    Macht die externe Live-Schaltung schlüsselfertig: Key in die (gitignored) Datei legen — kein Shell-Export nötig.
    NUR in main() aufgerufen (nicht beim Import), damit Test-Importe von server.py keine echten Keys laden.

    Reichweite: tests/conftest.py ruft diese Funktion zusätzlich direkt (nicht über main()) beim
    pytest-Sessionstart auf, und das Makefile-Ziel `eric-gate` laedt `.env` per Shell-`source` vor dem
    Python-Aufruf (s. Makefile) — beide Wege bewusst, damit `$ELSTER_HERSTELLER_ID` nicht mehr blind aus
    einer .env liegt (Naht 2026-08: HERSTELLER_ID vs. ELSTER_HERSTELLER_ID, s. BACKLOG). `eric_gate.py`
    selbst liest .env weiterhin NIEMALS (s. dortige Doku) — dort muss die Variable schon gesetzt sein."""
    for name in (".env.maps", ".env.llm", ".env"):
        pfad = os.path.join(root, name)
        if not os.path.isfile(pfad):
            continue
        try:
            with open(pfad, encoding="utf-8") as f:
                zeilen = f.readlines()
        except OSError:
            continue
        for zeile in zeilen:
            zeile = zeile.strip()
            if not zeile or zeile.startswith("#") or "=" not in zeile:
                continue
            schluessel, _, wert = zeile.partition("=")
            schluessel, wert = schluessel.strip(), wert.strip().strip('"').strip("'")
            if schluessel and schluessel not in os.environ:
                os.environ[schluessel] = wert


def main(argv):
    # Turnkey externe Live-Schaltung: gitignored .env.maps/.env.llm aus dem Repo-Root laden (Prozess-Env gewinnt).
    _lade_env_dateien(os.path.dirname(os.path.dirname(HERE)))
    port = int(argv[1]) if len(argv) > 1 else 8000
    srv = make_server(port)
    host, gebunden = srv.server_address[0], srv.server_address[1]
    print(f"TaxGraph-Haut auf http://{host}:{gebunden}  (Ctrl-C zum Beenden)")
    # P8.5 Graceful Shutdown: SIGTERM → KeyboardInterrupt → laufende Requests zu Ende
    signal.signal(signal.SIGTERM, lambda _sig, _frame: sys.stderr.write("\nSIGTERM empfangen, fahre herunter...\n") or os.kill(os.getpid(), signal.SIGINT))
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        srv.shutdown()
        print("Server heruntergefahren.")


if __name__ == "__main__":
    main(sys.argv)
