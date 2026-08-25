"""P1.1-Verdrahtung — Login in der Oberfläche (team-lead-Auftrag 2026-08-17).

Vorher existierte das Auth-Backend (produkt/auth/auth.py, /auth/register, /auth/login,
/auth/logout, /auth/session) vollständig, wurde aber von der Oberfläche NIE erreicht — kein
Authorization-Header irgendwo in produkt/haut/static/. Die ganze Zugriffskontrolle stand auf
dem Papier.

Fix (produkt/haut/static/app.js): jget/jpost hängen den Header an (TOKEN, sessionStorage-
gehalten), fangen 401 zentral ab (_401Abfangen) und zeigen dann die Anmeldemaske
(zeigeAnmeldemaske) statt des Netzwerkfehler-Banners. initAuth() klärt beim Start per Sonde
gegen einen garantiert nicht existierenden Fall (/fall/auth-sonde-taxgraph/stand, ohne Token),
ob diese Instanz überhaupt Auth verlangt — /auth/session allein kann das nicht (kennt
TAXGRAPH_NO_AUTH nicht, s. server.py _session_check), deshalb die Sonde über einen Endpunkt,
der _fall_owner_check durchläuft (produkt/haut/api.py).

Testmuster: Server-Fixture aus test_authz_fail_closed.py (isolierter USER_STORE, damit
register() nicht in die echte produkt/auth/users.json schreibt), Playwright-Fixture aus
test_ui_leerwert_stille_null.py. conftest.py setzt TAXGRAPH_NO_AUTH=1 für die gesamte Suite —
Tests, die den Auth-Pflicht-Pfad prüfen, löschen die Variable gezielt (wie
test_authz_fail_closed.py es vormacht).

NULL LLM.
"""
from __future__ import annotations

import os
import sys
import threading

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import audit             # noqa: E402
import server as SRV     # noqa: E402
import auth as AUTH      # noqa: E402
from ui_hilfen import zum_fragebogen  # noqa: E402

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None

pytestmark = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")

NUTZER = "testuser1"
PASSWORT = "geheim1234"


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    # Eigener User-Store pro Test (wie test_authz_fail_closed.py:base) — sonst schreibt
    # register() über die Oberfläche in die ECHTE produkt/auth/users.json.
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


@pytest.fixture
def seite(base):
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 360, "height": 780})
        yield page, base
        browser.close()


def _registrieren_und_anmelden(page, user=NUTZER, pw=PASSWORT):
    """Kompletter UI-Weg: Maske -> Registrieren-Modus -> Absenden -> eingeloggt."""
    page.wait_for_selector("#login:not([hidden])", timeout=5000)
    page.click("#login-umschalten")   # "Neu hier? Registrieren"
    page.fill("#login-user", user)
    page.fill("#login-pw", pw)
    page.click("#login-go")
    page.wait_for_selector("#start:not([hidden])", timeout=5000)


# --------------------------------------------------------------------- Auth-Pflicht (kein NO_AUTH)

def test_anmeldemaske_erscheint_wenn_auth_pflicht_ist(seite, monkeypatch):
    """Ohne Token und ohne TAXGRAPH_NO_AUTH muss die Maske VOR dem Start-Screen stehen — die Sonde
    in initAuth() (GET /fall/auth-sonde-taxgraph/stand ohne Header) muss 401 sauber erkennen."""
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    page, base = seite
    page.goto(base)
    page.wait_for_selector("#login:not([hidden])", timeout=5000)
    assert page.is_hidden("#start"), "Start-Screen sichtbar, obwohl Auth Pflicht ist und kein Token existiert."
    banner_versteckt = page.evaluate("document.getElementById('netz-banner').hidden")
    assert banner_versteckt is True, "Die Sonde darf keinen technischen Fehlerbanner auslösen."


def test_registrierung_und_anmeldung_fuehrt_in_die_anwendung(seite, monkeypatch):
    """Kernanforderung: gültige Registrierung+Anmeldung über die Maske führt in die App."""
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    page, base = seite
    page.goto(base)
    _registrieren_und_anmelden(page)
    token = page.evaluate("TOKEN")
    assert token, "Nach erfolgreichem Login muss TOKEN gesetzt sein."
    assert page.evaluate("sessionStorage.getItem('taxgraph_token')") == token, (
        "Token steht nicht in sessionStorage — würde einen Reload nicht überleben.")
    assert page.is_hidden("#login"), "Anmeldemaske bleibt nach erfolgreichem Login sichtbar."
    # Konto-Leiste zeigt den angemeldeten Nutzer (kein hartes UI-Pflichtstück im Auftrag, aber
    # billige Zusatzprobe, dass AUTH_USER wirklich gesetzt wurde).
    assert page.is_hidden("#konto-leiste") is False


def test_request_nach_login_traegt_authorization_header(seite, monkeypatch):
    """Der eigentliche Kern der Verdrahtung: ein GET nach dem Login trägt 'Authorization: Bearer …'."""
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    page, base = seite
    page.goto(base)
    _registrieren_und_anmelden(page)

    with page.expect_request(lambda req: "/fall/" in req.url and req.url.endswith("/stand")) as req_info:
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        # P0b (2026-08-23): zwischen Fallart und Fluss liegt jetzt die Wegwahl (Fragebogen / erst KI).
        page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
    req = req_info.value
    headers = {k.lower(): v for k, v in req.headers.items()}
    auth_header = headers.get("authorization")
    assert auth_header, f"Kein Authorization-Header am Request {req.url}: {headers}"
    assert auth_header.startswith("Bearer "), f"Unerwartetes Header-Format: {auth_header!r}"
    # Der Header muss auch wirklich AKZEPTIERT werden — sonst misst der Test nur, dass irgendein
    # Header geschickt wird, nicht dass die Verdrahtung stimmt. (Dazwischen liegt seit 2026-08-25
    # die Ankreuzliste; auch sie kommt nur, wenn /fragen den Header akzeptiert hat.)
    zum_fragebogen(page)


def test_abmelden_entfernt_das_token(seite, monkeypatch):
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    page, base = seite
    page.goto(base)
    _registrieren_und_anmelden(page)
    assert page.evaluate("TOKEN") is not None

    page.click("#logout-btn")
    page.wait_for_selector("#login:not([hidden])", timeout=5000)
    assert page.evaluate("TOKEN") is None, "TOKEN-Variable nach Abmelden nicht geleert."
    assert page.evaluate("sessionStorage.getItem('taxgraph_token')") is None, (
        "sessionStorage trägt nach Abmelden noch das Token.")
    assert page.is_hidden("#konto-leiste"), "Konto-Leiste bleibt nach Abmelden sichtbar."


def test_ungueltiges_token_zeigt_maske_nicht_fehlerbanner(seite, monkeypatch):
    """Requirement aus dem Auftrag: abgelaufenes/fehlendes Token -> Anmeldemaske, NICHT der
    technische Netzwerkfehler-Banner. Simuliert per manipuliertem TOKEN + echtem Request
    (refresh() gegen einen Fall, den es gar nicht braucht — nur der 401 zählt)."""
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    page, base = seite
    page.goto(base)
    _registrieren_und_anmelden(page)
    page.click(".kachel[data-scheibe='gesamt']")
    # P0b (2026-08-23): zwischen Fallart und Fluss liegt jetzt die Wegwahl (Fragebogen / erst KI).
    page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
    zum_fragebogen(page)   # Ankreuzliste am Anfang, s. tests/ui_hilfen.py

    page.evaluate("versteckeNetzFehler()")   # Ausgangszustand definiert
    page.evaluate("TOKEN = 'kaputt.und.abgelaufen'")   # Token verfälschen, ohne neu zu laden
    page.evaluate("refresh()")
    page.wait_for_selector("#login:not([hidden])", timeout=5000)
    banner_versteckt = page.evaluate("document.getElementById('netz-banner').hidden")
    assert banner_versteckt is True, (
        "Ein 401 darf den technischen Fehlerbanner nicht zeigen — die Anmeldemaske ersetzt ihn.")


# --------------------------------------------------------------------- Einzelnutzer-Modus (Default)

def test_einzelnutzer_modus_zeigt_nie_die_anmeldemaske(seite):
    """Gegenprobe: OHNE monkeypatch.delenv bleibt TAXGRAPH_NO_AUTH=1 (Suite-Default aus
    conftest.py) aktiv. Die Sonde muss dann auf 404 laufen (Fall existiert nicht, aber der
    Besitz-Check kommt durch) statt 401 — die Maske darf nie erscheinen, der komplette
    bisherige Ablauf (Kachel -> Wegpunkt) muss unverändert funktionieren."""
    page, base = seite
    page.goto(base)
    page.wait_for_load_state("networkidle")
    assert page.is_hidden("#login"), "Anmeldemaske erscheint im Einzelnutzer-Modus — darf nicht."
    assert not page.is_hidden("#start"), "Start-Screen bleibt versteckt im Einzelnutzer-Modus."
    assert page.is_hidden("#konto-leiste"), "Konto-Leiste erscheint ohne Login — falsch."

    page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
    # P0b (2026-08-23): zwischen Fallart und Fluss liegt jetzt die Wegwahl (Fragebogen / erst KI).
    page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
    zum_fragebogen(page)   # Ankreuzliste am Anfang, s. tests/ui_hilfen.py
    assert page.is_hidden("#login"), "Anmeldemaske erscheint mitten im Einzelnutzer-Fluss — darf nicht."
    assert page.evaluate("TOKEN") is None, "Im Einzelnutzer-Modus darf nie ein Token existieren."
