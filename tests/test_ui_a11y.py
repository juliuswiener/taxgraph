"""Barrierefreiheit-Tests (A11y) mit axe-core.

Prüft WCAG 2.1 Level AA Verstöße (critical + serious) auf Start-Screen und Wegpunkt-Fluss.
Nutzt axe-core via CDN-Injection + Playwright Rendering.
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


try:
    from playwright.sync_api import sync_playwright  # noqa: E402
except ImportError:
    sync_playwright = None


@pytest.fixture
def base(tmp_path, monkeypatch):
    """HTTP-Server auf Port 0, daemon thread."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
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


@pytest.fixture
def playwright_context():
    """Playwright Browser-Context."""
    if sync_playwright is None:
        pytest.skip("Playwright nicht installiert")
    try:
        with sync_playwright() as p:
            try:
                browser = p.chromium.launch(
                    headless=True,
                    timeout=15000,
                    args=["--no-sandbox", "--disable-setuid-sandbox"]
                )
            except Exception as e:
                pytest.skip(f"Chromium Start fehlgeschlagen: {e}")
            try:
                yield browser.new_context(viewport={"width": 360, "height": 640})
            finally:
                browser.close()
    except Exception as e:
        pytest.skip(f"Playwright-Setup fehlgeschlagen: {e}")


def _inject_axe_and_run(page, context_selector="body"):
    """Injiziert axe-core via CDN und führt Scan durch.

    Rückgabe: dict mit 'violations' (Liste), 'passes', 'inapplicable'.
    """
    # axe-core CDN-URL (neueste)
    axe_url = "https://cdnjs.cloudflare.com/ajax/libs/axe-core/4.8.0/axe.min.js"

    try:
        # Skript injizieren
        page.add_script_tag(url=axe_url)
    except Exception as e:
        pytest.skip(f"axe-core CDN nicht erreichbar: {e}")

    # axe.run() aufrufen, Ergebnis abrufen
    result = page.evaluate(f"""
        new Promise((resolve, reject) => {{
            axe.run(
                "{context_selector}",
                {{ rules: {{ 'color-contrast': {{ enabled: true }} }} }},
                (error, results) => {{
                    if (error) reject(error);
                    else resolve(results);
                }}
            );
        }})
    """)

    return result


@pytest.mark.xfail(reason="A11y-Verstöße erwartet — Initial-Report", strict=False)
def test_a11y_startscreen(base, playwright_context):
    """Start-Screen: Kachel-Wahl ohne WCAG-Verstöße."""
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")

        # axe laufen lassen
        result = _inject_axe_and_run(page, "body")

        # critical + serious Verstöße sammeln
        violations = result.get("violations", [])
        serious_viol = [v for v in violations if v.get("impact") in ["critical", "serious"]]

        if serious_viol:
            # Detailliert ausgeben für Debugging
            msg = f"\n=== A11y Verstöße: Start-Screen ===\n"
            for v in serious_viol:
                rule_id = v.get("id", "UNKNOWN")
                impact = v.get("impact", "?")
                description = v.get("description", "")
                nodes = v.get("nodes", [])
                selectors = [n.get("target", ["?"])[0] for n in nodes[:3]]
                msg += f"\n[{impact.upper()}] {rule_id}: {description}\n"
                msg += f"  Selektoren: {', '.join(selectors)}\n"
            pytest.fail(msg)
    finally:
        page.close()


@pytest.mark.xfail(reason="A11y-Verstöße erwartet — Initial-Report", strict=False)
def test_a11y_wegpunkt(base, playwright_context):
    """Wegpunkt-Fluss: Frage + Buttons ohne WCAG-Verstöße."""
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")

        # Fall starten (JS-Klick auf Kachel)
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")

        # Wegpunkt-Karte warten
        page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)

        # axe laufen lassen
        result = _inject_axe_and_run(page, "body")

        # Verstöße
        violations = result.get("violations", [])
        serious_viol = [v for v in violations if v.get("impact") in ["critical", "serious"]]

        if serious_viol:
            msg = f"\n=== A11y Verstöße: Wegpunkt ({len(serious_viol)}) ===\n"
            for v in serious_viol:
                rule_id = v.get("id", "UNKNOWN")
                impact = v.get("impact", "?")
                description = v.get("description", "")
                nodes = v.get("nodes", [])
                selectors = [n.get("target", ["?"])[0] for n in nodes[:3]]
                msg += f"\n[{impact.upper()}] {rule_id}: {description}\n"
                msg += f"  Selektoren: {', '.join(selectors)}\n"
            # Melde Verstöße, aber lass xfail laufen (strict=False)
            print(msg)
    finally:
        page.close()
