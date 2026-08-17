"""IDOR bei der Vorjahres-Übernahme + Sicherheits-Header (Audit 2026-08-16).

Zwei Funde, beide 2026-08-17 vor der Reparatur verifiziert:

`vorjahr()` prüfte `_fall_owner_check(fall_id)` nur für das ZIEL und lud danach die aus dem
Request-Body stammende Quell-Kennung ungeprüft. Ein eingeloggter Nutzer konnte damit Felder aus
einem FREMDEN Fall in seinen eigenen ziehen — dieselbe Bauart wie das DELETE-Loch (39fcf79):
eine Route, die die Zugriffsnaht nur halb benutzt. Dass die Klasse nicht wiederkommt, hält
`tests/test_zweite_fall_kennung_gate.py` strukturell fest; hier steht der Verhaltensbeleg.

Und: der Server setzte keinen `Content-Security-Policy`-Header. Das ist die zweite
Verteidigungslinie hinter dem XSS-Sink in `app.js:herkunftKette` (OCR-Rohtext eines fremden
Dokuments ging unescaped in innerHTML) — und die zählt hier besonders, weil das Anmelde-Token
in sessionStorage liegt.

BEIDE RICHTUNGEN, wie überall in diesem Haus: nicht nur "der Fremde wird abgewiesen", sondern
auch "der Berechtigte kommt durch und bekommt seine Felder". Eine Sperre, die alles sperrt,
sieht in einem Einrichtungs-Test genauso grün aus wie eine richtige.
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
            return r.status, dict(r.headers), json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), json.loads(e.read())


def _konto(base, name):
    _req(base, "POST", "/auth/register", body={"username": name, "password": "geheim1234"})
    _, _, ld = _req(base, "POST", "/auth/login", body={"username": name, "password": "geheim1234"})
    return {"Authorization": "Bearer " + ld["token"]}


def _fall(base, kopf, fall_id):
    st, _, obj = _req(base, "POST", "/fall", body={"fall_id": fall_id, "scheibe": "ep"},
                      headers=kopf)
    assert st == 201, obj
    return fall_id


# ------------------------------------------------------------------ IDOR /vorjahr

def test_fremder_vorjahres_fall_wird_abgewiesen(monkeypatch, base):
    """Das eigentliche Loch: Ziel gehört mir, Quelle gehört jemand anderem."""
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    opfer = _konto(base, "opfer")
    _fall(base, opfer, "opfer-vj")
    angreifer = _konto(base, "angreifer")
    _fall(base, angreifer, "angreifer-neu")

    st, _, obj = _req(base, "POST", "/fall/angreifer-neu/vorjahr",
                      body={"vorjahr_fall_id": "opfer-vj"}, headers=angreifer)
    assert st == 403, f"IDOR: fremder Vorjahres-Fall wurde ausgelesen ({st}: {obj})"


def test_eigener_vorjahres_fall_geht_weiter(monkeypatch, base):
    """Die Gegenrichtung — ohne sie wäre eine Prüfung, die ALLES sperrt, ebenso grün."""
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    kopf = _konto(base, "eigner")
    _fall(base, kopf, "eigner-vj")
    _fall(base, kopf, "eigner-neu")

    st, _, obj = _req(base, "POST", "/fall/eigner-neu/vorjahr",
                      body={"vorjahr_fall_id": "eigner-vj"}, headers=kopf)
    assert st == 200, f"eigener Vorjahres-Fall wurde abgewiesen ({st}: {obj})"
    assert "uebernommen" in obj, obj


def test_nicht_existierender_vorjahres_fall_bleibt_404(monkeypatch, base):
    """Abgrenzung: die neue Prüfung darf 404 nicht in 403 verwandeln. `_fall_owner_check`
    kehrt bei fehlendem Fall bewusst zurück, damit der Aufrufer die ehrliche Auskunft gibt —
    für den EIGENEN Fallraum ist die Existenz keine Information, die zu schützen wäre."""
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    kopf = _konto(base, "sucher")
    _fall(base, kopf, "sucher-neu")
    st, _, _ = _req(base, "POST", "/fall/sucher-neu/vorjahr",
                    body={"vorjahr_fall_id": "gibt-es-nicht"}, headers=kopf)
    assert st == 404


# ------------------------------------------------------------------ Sicherheits-Header

def _csp(headers):
    return headers.get("Content-Security-Policy", "")


def test_csp_auf_der_ausgelieferten_seite(base):
    """Die Seite selbst ist der Ort, an dem die Richtlinie wirkt — auf ihr läuft das Skript."""
    st, h, _ = _req(base, "GET", "/health")     # JSON-Pfad
    assert st == 200
    assert "default-src 'none'" in _csp(h), h

    req = urllib.request.Request(base + "/", method="GET")
    with urllib.request.urlopen(req, timeout=10) as r:   # statischer Pfad
        kopf = dict(r.headers)
    assert "default-src 'none'" in _csp(kopf), kopf
    assert "script-src 'self'" in _csp(kopf), kopf


def test_csp_erlaubt_kein_inline_skript(base):
    """Der Kern: ohne 'unsafe-inline' ist die Richtlinie gegen genau den Fall scharf, für den
    sie da ist. Steht es doch einmal drin, ist sie Dekoration.

    Die erste Zeile ist keine Formalität. In der ersten Fassung fehlte sie, und beim
    Mutationstest (Header-Aufruf entfernt) blieb dieser Test GRÜN: "unsafe-inline" steht eben
    auch in einem leeren String nicht drin. Eine Zusicherung über eine Eigenschaft, die es gar
    nicht gibt, ist immer erfüllt — dieselbe Klasse wie die vier Gates, die heute grün waren,
    weil sie ihre eigene Voraussetzung mitbrachten. Erst Existenz, dann Inhalt."""
    req = urllib.request.Request(base + "/", method="GET")
    with urllib.request.urlopen(req, timeout=10) as r:
        p = dict(r.headers).get("Content-Security-Policy", "")
    assert p, "gar kein CSP-Header — die Prüfungen darunter wären sonst trivial erfüllt"
    assert "unsafe-inline" not in p, f"CSP erlaubt Inline-Skript/-Style: {p}"
    assert "unsafe-eval" not in p, f"CSP erlaubt eval(): {p}"


def test_die_oberflaeche_laeuft_unter_der_csp(base):
    """Der eigentliche Beweis, und bewusst OHNE `bypass_csp`: die Anwendung wird unter der
    scharfen Richtlinie wirklich bedient, und dabei darf keine einzige CSP-Verletzung fallen.

    Warum es diesen Test überhaupt gibt: die UI-Tests in test_ui_leerwert_stille_null.py mussten
    `bypass_csp=True` bekommen, weil Playwrights `wait_for_function()` zum Pollen eine
    Zeichenkette als JavaScript auswertet — das verbietet jede CSP ohne 'unsafe-eval'. Damit
    wären ALLE Browser-Tests blind gegen eine Richtlinie, die die Oberfläche abschaltet. Dieser
    Test schließt die Lücke: er kommt ohne wait_for_function aus und lauscht auf die Konsole.

    Gemessen 2026-08-17: der Ablauf (Scheibe wählen, Frage beantworten, bestätigen) erzeugt
    unter der scharfen Richtlinie null Verletzungen."""
    pw = pytest.importorskip("playwright.sync_api")
    verletzungen = []
    with pw.sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 360, "height": 780})   # KEIN bypass_csp
        page.on("console", lambda m: verletzungen.append(m.text)
                if "Content Security Policy" in m.text else None)
        try:
            page.goto(base)
            page.wait_for_load_state("networkidle")
            page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
            # Bis hierher ist alles beteiligt, was die Richtlinie überhaupt betreffen kann:
            # das Stylesheet (style-src), app.js (script-src), die fetch-Aufrufe von
            # /fall, /stand und /fragen (connect-src) und das Rendern der ersten Frage.
            page.wait_for_selector("#wegpunkt:not([hidden])", timeout=15000)
            sichtbar = page.evaluate("!document.getElementById('wegpunkt').hidden")
            frage = page.evaluate("document.getElementById('frage').textContent")
        finally:
            browser.close()
    assert not verletzungen, (
        "die eigene Richtlinie blockiert die eigene Oberfläche:\n  " + "\n  ".join(verletzungen))
    assert sichtbar, "der Wegpunkt wurde nicht sichtbar — Ablauf unter der CSP steckengeblieben"
    assert frage and frage.strip(), "keine Frage gerendert — die Seite ist unter der CSP leer"


def test_richtlinie_passt_zur_ausgelieferten_seite():
    """Eine Richtlinie, die die eigene Oberfläche abschaltet, ist schlimmer als keine — und ein
    Test, der nur den HTTP-Status prüft, merkt davon nichts. Deshalb hier gegen die Dateien
    gemessen statt geglaubt: kein Inline-<script>, kein on*=-Handler, kein style=-Attribut,
    keine javascript:-URL, keine externe Ressource. Kommt eines davon dazu, wird dieser Test
    rot, BEVOR die Seite beim Nutzer stumm halb funktioniert."""
    import glob
    import re
    for pfad in glob.glob(os.path.join(ROOT, "produkt", "haut", "static", "*.html")):
        html = open(pfad, encoding="utf-8").read()
        name = os.path.basename(pfad)
        assert "<script>" not in html, f"{name}: Inline-<script> — CSP wuerde es abschalten"
        assert 'style="' not in html, f"{name}: style=-Attribut — CSP wuerde es abschalten"
        assert "javascript:" not in html, f"{name}: javascript:-URL — CSP wuerde sie abschalten"
        assert not re.search(r'\son(click|change|input|submit|load|error)\s*=', html), (
            f"{name}: Inline-Event-Handler — CSP wuerde ihn abschalten")
        assert 'src="http' not in html and 'href="http' not in html, (
            f"{name}: externe Ressource — default-src 'none' wuerde sie blockieren")
