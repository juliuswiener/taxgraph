"""Authz-Boundary fail-closed + CSRF/Origin/Host/Content-Type-Checks (Audit 2026-08-16).

Deckt die Funde sec-authz-fail-open-no-token und sec-csrf-simple-request-write:
- Ohne Token ist JEDER /fall-Zugriff 401 (vorher: Vollzugriff, live-Probe löschte
  den Fall eines registrierten Nutzers anonym).
- Cross-Origin-POST (CORS-Simple-Request) wird an der Dispatch-Naht abgewiesen,
  bevor ein Handler läuft.
- TAXGRAPH_NO_AUTH=1 ist der EXPLIZITE Einzelnutzer-Opt-out (conftest setzt ihn für
  die Bestandssuite); diese Tests löschen ihn gezielt, um den Default zu prüfen.
- Die scharfe Richtung wird auf BEIDEN Seiten geprüft: 403 für einen fremden
  Besitzer UND 200 für den eigenen Fall mit gültigem Token — sonst merkt niemand,
  wenn die Prüfung eines Tages ALLES sperrt statt nur das Unberechtigte.
"""
import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden", "produkt/store", "produkt/traverser",
            "produkt/unsicherheit", "produkt/mapping", "produkt/konsistenz",
            "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API           # noqa: E402
import server as SRV        # noqa: E402
import audit                # noqa: E402
import auth as AUTH         # noqa: E402


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    # Eigener User-Store pro Test (wie test_auth_integration.py:base) — sonst schreiben
    # register()-Aufrufe in dieser Datei in die ECHTE produkt/auth/users.json.
    monkeypatch.setattr(AUTH, "USER_STORE", str(tmp_path / "users.json"))
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _req(base, method, path, body=None, headers=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    h = {"Content-Type": "application/json"}
    if headers:
        h.update(headers)
    req = urllib.request.Request(base + path, data=data, method=method, headers=h)
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def test_ohne_token_ist_fall_zugriff_401(base, monkeypatch):
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    status, obj = _req(base, "GET", "/fall/irgendein-fall/stand")
    assert status == 401, f"fail-open: anonymer Lesezugriff kam durch ({status}: {obj})"


def _angemeldeter_besitzer(base, username, fall_id):
    """Registriert+loggt username ein und legt fall_id auf dessen Namen an. Gibt das Token
    zurück. Setup läuft bewusst MIT TAXGRAPH_NO_AUTH gesetzt (Suite-Default) — das Token
    macht den Request ohnehin authentifiziert, der Opt-out greift hier gar nicht."""
    _req(base, "POST", "/auth/register", body={"username": username, "password": "geheim1234"})
    _, ld = _req(base, "POST", "/auth/login", body={"username": username, "password": "geheim1234"})
    st, obj = _req(base, "POST", "/fall",
                   body={"fall_id": fall_id, "scheibe": "ep", "veranlagungszeitraum": 2025},
                   headers={"Authorization": f"Bearer {ld['token']}"})
    assert st == 201, f"Setup gescheitert: Fall {fall_id!r} nicht angelegt ({st}: {obj})"
    return ld["token"]


def test_ohne_token_ist_delete_401(base, monkeypatch):
    # Das Audit-PoC-Szenario: anonymer DELETE löschte den Fall eines registrierten Nutzers.
    # Der Fall MUSS real existieren und einen echten Besitzer haben — sonst prüft dieser
    # Test nur "Fall gibt's nicht" (404 vom Handler), nicht die eigentliche Lücke.
    _angemeldeter_besitzer(base, "opfer", "opferfall")

    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    status, obj = _req(base, "DELETE", "/fall/opferfall")
    assert status == 401, f"fail-open: anonymer DELETE kam durch ({status}: {obj})"


def test_fremder_token_ist_delete_403(base, monkeypatch):
    # Eingeloggt, aber nicht Besitzer -> 403 (weder 401 noch 200).
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    _angemeldeter_besitzer(base, "opfer2", "opferfall2")
    _req(base, "POST", "/auth/register", body={"username": "angreifer", "password": "geheim1234"})
    _, ld = _req(base, "POST", "/auth/login", body={"username": "angreifer", "password": "geheim1234"})

    status, obj = _req(base, "DELETE", "/fall/opferfall2",
                       headers={"Authorization": f"Bearer {ld['token']}"})
    assert status == 403, f"fremder Nutzer konnte löschen ({status}: {obj})"


def test_eigener_token_ist_delete_200(base, monkeypatch):
    # Gegenprobe zu den beiden Sperr-Tests oben: der rechtmäßige Besitzer darf weiterhin
    # löschen. Ohne diese Richtung fiele eine Prüfung, die irgendwann ALLES sperrt
    # (auch berechtigte Zugriffe), in keinem Test auf.
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    token = _angemeldeter_besitzer(base, "besitzer", "eigenerfall")

    status, obj = _req(base, "DELETE", "/fall/eigenerfall",
                       headers={"Authorization": f"Bearer {token}"})
    assert status == 200, f"rechtmäßiger Besitzer konnte nicht löschen ({status}: {obj})"


def test_cross_origin_post_wird_403(base):
    # CORS-Simple-Request einer besuchten Webseite: Origin fremd -> Abweisung an der
    # Dispatch-Naht, unabhängig vom Auth-Zustand (conftest: TAXGRAPH_NO_AUTH=1 aktiv).
    status, obj = _req(base, "POST", "/fall/x/event",
                       body={"feld_id": "steuerklasse", "wert": 1},
                       headers={"Origin": "https://evil.example"})
    assert status == 403, f"Cross-Origin-Schreibzugriff kam durch ({status}: {obj})"
    assert obj.get("fehler") == "cross_origin_verboten"


def test_eigener_origin_bleibt_erlaubt(base):
    port = base.rsplit(":", 1)[1]
    status, _ = _req(base, "GET", "/health",
                     headers={"Origin": f"http://127.0.0.1:{port}"})
    assert status == 200


def test_fremder_host_wird_421(base):
    # DNS-Rebinding: Browser löst evil.example auf 127.0.0.1 auf, Host-Header trägt den Angreifer.
    status, obj = _req(base, "GET", "/health", headers={"Host": "evil.example"})
    assert status == 421, f"fremder Host kam durch ({status}: {obj})"


def test_text_plain_post_wird_415(base):
    # text/plain ist der CORS-preflight-freie Weg — Content-Type-Zwang erzwingt den Preflight.
    req = urllib.request.Request(base + "/fall/x/event",
                                 data=b'{"feld_id": "steuerklasse", "wert": 1}',
                                 method="POST", headers={"Content-Type": "text/plain"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            status = r.status
    except urllib.error.HTTPError as e:
        status = e.code
    assert status == 415, f"text/plain-POST kam durch ({status})"


def test_no_auth_flag_ist_expliziter_opt_out(base, monkeypatch):
    # Mit Flag (Suite-Default aus conftest): kein 401 — der Einzelnutzer-Modus bleibt nutzbar,
    # aber als BENANNTE Entscheidung. Fall existiert nicht -> 404 vom Handler, nicht 401.
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    status, _ = _req(base, "GET", "/fall/gibt-es-nicht/stand")
    assert status == 404
