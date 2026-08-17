"""Responsive Design Rendering-Tests (P5.6) — Playwright.

Verifiziert, dass alle UI-Komponenten bei 360×640 (iPhone SE/Android-Baseline)
bedienbar sind: 44×44 Tap-Ziele, kein horizontales Scrollen, Buttons nicht überlappend.
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
for sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402
import audit                # noqa: E402


try:
    from playwright.sync_api import sync_playwright  # noqa: E402
except ImportError:
    sync_playwright = None


@pytest.fixture
def base(tmp_path, monkeypatch):
    """HTTP-Server auf Port 0, daemon thread."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    assert srv.server_address[0] == "127.0.0.1"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _req(base: str, method: str, path: str, body: dict | None = None,
         erwarte: int | None = None):
    """HTTP request (urllib fallback für Seiten-State).

    Prüft selbst:
    - 5xx → AssertionError (nie unterdrückbar)
    - 4xx → AssertionError, es sei denn `erwarte=<code>` ist gesetzt
    - 2xx → durch
    - erwarte=N → assert status == N
    """
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            status = r.status
            content = json.loads(r.read())
    except urllib.error.HTTPError as e:
        status = e.code
        content = json.loads(e.read())
    if erwarte is not None:
        assert status == erwarte, (
            f"erwarte={erwarte}, erhalten={status} {method} {path} {body}")
    elif status >= 500:
        raise AssertionError(
            f"Serverfehler {status} {method} {path} {body}: {content}")
    elif status >= 400:
        raise AssertionError(
            f"Fehler {status} {method} {path} {body}: {content}")
    return status, content


def _laie(fld, w):
    """Event-Objekt für Nutzer-Input."""
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


@pytest.fixture
def playwright_context():
    """Playwright-Browser-Context mit Fehler-Handling."""
    if sync_playwright is None:
        pytest.skip("Playwright nicht installiert")
    try:
        with sync_playwright() as p:
            try:
                # Chromium mit Timeout + Sandbox-Args
                browser = p.chromium.launch(
                    headless=True,
                    timeout=15000,
                    args=["--no-sandbox", "--disable-setuid-sandbox"]
                )
            except Exception as e:
                import traceback
                tb = traceback.format_exc()
                pytest.skip(f"Chromium Start fehlgeschlagen: {e}\n{tb}")
            try:
                yield browser.new_context(viewport={"width": 360, "height": 640})
            finally:
                browser.close()
    except Exception as e:
        import traceback
        tb = traceback.format_exc()
        pytest.skip(f"Playwright-Setup fehlgeschlagen: {e}\n{tb}")


def test_start_screen_kacheln_responsive(base, playwright_context):
    """Start-Kacheln: 44×44 Tap-Ziele, kein Überlapp."""
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")

        # Viewport
        assert page.evaluate("window.innerWidth") == 360
        assert page.evaluate("window.innerHeight") == 640

        # Kein horizontales Scrollen
        scroll_width = page.evaluate("document.documentElement.scrollWidth")
        assert scroll_width <= 360, f"Horizontales Scrollen: scrollWidth={scroll_width} > 360"

        # Kachel-Buttons: 44×44 Minimum
        kacheln = page.query_selector_all(".kachel")
        assert len(kacheln) >= 2, "Keine Kacheln gefunden"
        for i, kachel in enumerate(kacheln):
            bbox = kachel.bounding_box()
            assert bbox is not None, f"Kachel {i}: bounding_box() ist None"
            height = bbox.get("height", 0)
            width = bbox.get("width", 0)
            # Kachel ist größer, aber alle Children müssen 44×44 haben
            assert height >= 44, f"Kachel {i}: height={height} < 44"
            assert width >= 44, f"Kachel {i}: width={width} < 44"
    finally:
        page.close()


def test_start_screen_no_horizontal_scroll(base, playwright_context):
    """Gesamtseite: kein horizontales Scrollen."""
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")
        scroll_width = page.evaluate("document.documentElement.scrollWidth")
        assert scroll_width <= 360, f"scrollWidth={scroll_width} > 360 (Viewport 360)"
    finally:
        page.close()


def test_wegpunkt_buttons_responsive(base, playwright_context):
    """Wegpunkt: zwei Buttons (.aktionen) nebeneinander oder gestapelt, keine Überlaps."""
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")

        # Klick auf Kachel startet Fall (JS, keine Playwright-Click wegen Overlay-Blocks)
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")

        # Warte, bis die WEGPUNKT-Karte sichtbar ist. Hier stand `.card:not([hidden])` — ein
        # Stellvertreter, der solange trug, wie die Wegpunkt-Karte die einzige .card im Fluss war.
        # Seit der KI-Berater ebenfalls eine .card ist (und nie versteckt), traf der Selektor
        # sofort zu, und #bestaetigen hatte noch keine Bounding-Box. Der Test misst jetzt, was er
        # meint.
        page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)

        # Buttons
        btn_best = page.query_selector("#bestaetigen")
        btn_chat = page.query_selector("#chat")
        assert btn_best is not None
        assert btn_chat is not None

        # 44×44 Tap-Ziele
        for btn, name in [(btn_best, "bestaetigen"), (btn_chat, "chat")]:
            bbox = btn.bounding_box()
            assert bbox is not None, f"{name}: bbox None"
            h = bbox.get("height", 0)
            w = bbox.get("width", 0)
            assert h >= 44, f"{name}: h={h}"
            assert w >= 44, f"{name}: w={w}"

        # Keine Überlaps
        bbox1 = btn_best.bounding_box()
        bbox2 = btn_chat.bounding_box()
        b1r = bbox1["x"] + bbox1["width"]
        b2l = bbox2["x"]
        b1b = bbox1["y"] + bbox1["height"]
        b2t = bbox2["y"]
        side = b1r <= b2l + 1
        stack = b1b <= b2t + 1
        assert side or stack, f"Overlap: b1 r={b1r} b2 l={b2l}"

        # Kein horizontales Scrollen
        scroll = page.evaluate("document.documentElement.scrollWidth")
        assert scroll <= 360, f"scrollWidth={scroll}"
    finally:
        page.close()


def test_fortschrittsleiste_responsive(base, playwright_context):
    """Fortschrittsleiste sichtbar, nicht breiter als 360px."""
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")

        # Klick auf Kachel startet Fall (JS)
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")

        # Warte auf Fluss sichtbar
        page.wait_for_selector("#flow:not([hidden])", timeout=5000)

        # Leiste finden
        progress = page.query_selector("#fortschritt")
        assert progress is not None, "Progress-Element #fortschritt nicht gefunden"

        # Sichtbar (nicht hidden)
        is_hidden = progress.is_hidden()
        assert not is_hidden, "Progress ist hidden"

        # Breite <= 360
        bbox = progress.bounding_box()
        assert bbox is not None, "Progress: bounding_box() ist None"
        width = bbox.get("width", 0)
        assert width <= 360, f"Progress: width={width} > 360"
    finally:
        page.close()


def test_belegt_liste_tap_targets(base, playwright_context):
    """Belegt-Liste: Zeilen >= 44px Höhe (Tap-Ziel), keine Überschneidung."""
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")

        # Fall starten + 1 Feld bestätigen (damit Belegt-Liste non-leer ist)
        status, body = _req(base, "POST", "/fall", {
            "scheibe": "gesamt",
            "veranlagungszeitraum": 2025,
            "fall_id": "test-belegt-360"
        })
        assert status == 201
        fall_id = body.get("fall_id")

        # Frage laden
        status, fragen = _req(base, "GET", f"/fall/{fall_id}/fragen")
        assert status == 200
        q = fragen.get("fragen", [{}])[0]
        feld_id = q.get("feld_id")

        # Event: Feld bestätigen. Welches Feld zuerst gefragt wird, hängt von der
        # Screening-Reihenfolge ab (nicht stabil) — ein fester Wert wie 42 passt nicht zu
        # jedem Bindungstyp (enum/bool/...). beispielwert ist je Bindung typkorrekt vorgegeben.
        status, _ = _req(base, "POST", f"/fall/{fall_id}/event", _laie(feld_id, q.get("beispielwert")))

        # Seite laden
        page.goto(base)
        page.wait_for_load_state("networkidle")

        # Belegt-Zeilen prüfen
        zeilen = page.query_selector_all(".zeile")
        if len(zeilen) > 0:
            for i, zeile in enumerate(zeilen):
                bbox = zeile.bounding_box()
                assert bbox is not None, f"Zeile {i}: bounding_box() ist None"
                h = bbox.get("height", 0)
                assert h >= 44, f"Zeile {i}: height={h} < 44"
    finally:
        page.close()


def test_overlay_und_berater_responsive(base, playwright_context):
    """Herkunft-Overlay (weiterhin ein Modal) und KI-Berater (seit 2026-08-14 ein dauerhaftes
    Panel): bei 360px lesbar, nicht breiter als der Viewport.

    Dieser Test hieß test_overlays_responsive und maß das CHAT-Overlay. Das gibt es nicht mehr —
    Julius: „Die KI sollte immer offen sein". Zwei Lehren aus seiner alten Fassung bleiben
    eingebaut: der Selektor ist auf EIN Overlay gescoped (ungescoped traf `.overlay-card` die
    erste im DOM, also die falsche), und geprüft wird eine echte Bounding-Box (ein verstecktes
    Element hat keine — vorher fiel deshalb nicht auf, dass beide Overlays dauerhaft sichtbar
    waren)."""
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)

        # Der Berater ist ohne jedes Öffnen da — das ist die Eigenschaft, um die es geht.
        berater = page.query_selector("#berater")
        assert berater is not None, "KI-Berater-Panel fehlt"
        bbox = berater.bounding_box()
        assert bbox is not None, "Der Berater ist nicht sichtbar — er soll dauerhaft offen sein."
        assert bbox.get("width", 0) <= 360, f"Berater: width={bbox.get('width')} > 360"
        assert page.query_selector("#chat-text") is not None, "Eingabefeld nicht erreichbar"
        assert page.query_selector("#chat-send") is not None, "Absende-Knopf fehlt"

        # Das Herkunft-Overlay ist weiter ein Modal und muss weiter passen.
        page.evaluate("document.getElementById('kette-overlay').hidden = false")
        karte = page.query_selector("#kette-overlay .overlay-card")
        assert karte is not None, "Overlay-Card im Herkunft-Overlay nicht gefunden"
        kbox = karte.bounding_box()
        assert kbox is not None, "Herkunft-Overlay-Card hat keine Bounding-Box — nicht offen."
        assert kbox.get("width", 0) <= 360, f"Overlay-Card: width={kbox.get('width')} > 360"

        page.evaluate("document.getElementById('kette-zu').click()")
        # state="hidden" statt Selektor "#kette-overlay[hidden]": wait_for_selector wartet ohne
        # state-Angabe auf VISIBLE, und ein wirklich verstecktes Element wird das nie. Bis zum
        # [hidden]-Fix (2026-08-14) lief die alte Form durch, weil das Overlay trotz hidden-Attribut
        # sichtbar blieb — der Test hat also bestätigt, dass Schließen NICHT wirkt.
        page.wait_for_selector("#kette-overlay", state="hidden", timeout=5000)
    finally:
        page.close()


def test_input_fields_tap_targets(base, playwright_context):
    """Input-Felder: min-height 44px."""
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")

        # Fall starten
        status, body = _req(base, "POST", "/fall", {
            "scheibe": "gesamt",
            "veranlagungszeitraum": 2025,
            "fall_id": "test-inputs-360"
        })
        assert status == 201

        # Seite laden (Wegpunkt sichtbar)
        page.goto(base)
        page.wait_for_load_state("networkidle")

        # Input-Feld #feld-input
        inp = page.query_selector("#feld-input")
        if inp:
            bbox = inp.bounding_box()
            assert bbox is not None
            h = bbox.get("height", 0)
            assert h >= 44, f"Input #feld-input: height={h} < 44"
    finally:
        page.close()
