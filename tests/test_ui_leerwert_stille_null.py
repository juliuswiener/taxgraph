"""Stille-Null-Klasse (Variante Leerwert) — Befund B, team-lead-Auftrag 2026-08-16.

Vorher: leseWert() machte aus einem LEEREN oder browser-ungültigen Zahlenfeld über
`parseFloat(el.value || "0")` / `parseInt(el.value || "0", 10)` kommentarlos eine 0 —
ununterscheidbar von einer echten, absichtlich eingegebenen 0. Der Aufrufer bestaetigen()
schrieb diese 0 anstandslos als zustand=bestaetigt in den Store (derselbe Stille-Null-Effekt
wie Befund A, nur am anderen Ende: nicht ein falscher Typ, sondern eine erfundene Abwesenheit).

Fix (produkt/haut/static/app.js): leseWert() liefert bei leer/ungültig jetzt `undefined`
(Number.isFinite-Check nach parseFloat/parseInt); bestaetigen() bricht bei `wert === undefined`
VOR dem Netzwerk-Aufruf ab, zeigt die Fehlermeldung über den schon vorhandenen netz-banner
(zeigeNetzFehler — derselbe Mechanismus wie jede andere Ablehnung in dieser Funktion) und
schreibt KEIN Event. Eine ausdrücklich eingegebene "0" bleibt eine echte, gültige 0.

Testmuster identisch zu test_ui_wahl_buttons.py (base/seite-Fixtures, direktes Rendern einer
beliebigen Frage über zeigeFrage(q, STAND) statt den ganzen Klickweg nachzubauen).

NULL LLM.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import audit             # noqa: E402
import server as SRV     # noqa: E402
from ui_hilfen import zum_fragebogen  # noqa: E402

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None

pytestmark = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
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
        # bypass_csp: NICHT weil die Anwendung unter der Richtlinie Probleme hätte, sondern weil
        # das WERKZEUG sie hat. `page.wait_for_function()` pollt, indem es eine Zeichenkette als
        # JavaScript auswertet — das verbietet jede CSP ohne 'unsafe-eval', und Chrome meldet
        # genau das ("Refused to evaluate a string as JavaScript"). Nachgemessen 2026-08-17: derselbe
        # Ablauf (Feld rendern, 0 eintragen, bestätigen) läuft unter der scharfen Richtlinie ohne
        # eine einzige Verletzung durch, sobald man ihn ohne wait_for_function fährt.
        # Dass die Richtlinie die Oberfläche NICHT bricht, prüft deshalb ein eigener Test ohne
        # diesen Schalter: tests/test_idor_und_csp.py::test_die_oberflaeche_laeuft_unter_der_csp.
        # 'unsafe-eval' ins Produkt aufzunehmen, damit ein Testwerkzeug pollen kann, wäre die
        # falsche Richtung — dann schützte die Richtlinie gegen genau nichts mehr.
        kontext = browser.new_context(viewport={"width": 360, "height": 780}, bypass_csp=True)
        page = kontext.new_page()
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        # P0b (2026-08-23): zwischen Fallart und Fluss liegt jetzt die Wegwahl (Fragebogen / erst KI).
        page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
        zum_fragebogen(page)   # Ankreuzliste am Anfang, s. tests/ui_hilfen.py
        yield page, base
        browser.close()


def _aktuelle_frage(page):
    return page.evaluate("AKTUELL && AKTUELL.feld_id")


def _rendere(page, fid):
    """Ein beliebiges cent-Feld direkt als aktuelle Frage rendern — wie test_ui_wahl_buttons.py:
    alle davorliegenden Fragen über die UI durchzuklicken wäre für diesen Test unnötiger Aufwand."""
    page.evaluate("""async (fid) => {
        const r = await jget(`/fall/${FALL}/fragen`);
        const q = r.body.fragen.find(f => f.feld_id === fid);
        if (q) { AKTUELL = q; zeigeFrage(q, STAND); }
    }""", fid)
    assert _aktuelle_frage(page) == fid, f"{fid} in dieser Scheibe nicht erreichbar"


def test_leeres_feld_liest_undefined_nicht_null(seite):
    """Der Kern des Fixes: parseFloat/parseInt auf einem leeren Feld darf NICHT mehr 0 ergeben."""
    page, _ = seite
    _rendere(page, "bruttoarbeitslohn")
    page.fill("#feld-input", "")
    wert = page.evaluate("leseWert(AKTUELL)")
    assert wert is None, f"leseWert() bei leerem Feld muss undefined liefern, ist {wert!r}"


def test_komma_statt_punkt_liest_undefined(seite):
    """Komma statt Punkt ('12,5') ist für <input type="number"> ein ungültiges Format — der
    Browser sanitisiert das Feld selbst auf el.value="" (Playwright.fill() verweigert das Tippen
    sogar direkt für type=number — darum hier der DOM-Property-Weg, der dasselbe browser-eigene
    Sanitizing auslöst wie echtes Tippen). Ohne den Fix wäre daraus über
    `parseFloat(el.value || "0")` still eine 0 geworden."""
    page, _ = seite
    _rendere(page, "bruttoarbeitslohn")
    page.evaluate("document.getElementById('feld-input').value = '12,5'")
    assert page.input_value("#feld-input") == "", (
        "Erwartung widerlegt: der Browser lässt ein Komma im Zahlenfeld doch durch — Testannahme prüfen.")
    wert = page.evaluate("leseWert(AKTUELL)")
    assert wert is None, f"leseWert() bei browser-ungültigem Text muss undefined liefern, ist {wert!r}"


def test_explizite_null_bleibt_eine_echte_null(seite):
    """Die Kehrseite des Fixes: eine ausdrücklich eingetippte 0 darf NICHT als 'leer' behandelt
    werden — roh='0' ist nicht roh=''."""
    page, _ = seite
    _rendere(page, "bruttoarbeitslohn")
    page.fill("#feld-input", "0")
    wert = page.evaluate("leseWert(AKTUELL)")
    assert wert == 0, f"leseWert() bei explizit eingegebener 0 muss 0 liefern, ist {wert!r}"


def test_bestaetigen_schreibt_bei_leerem_feld_kein_event_und_zeigt_fehler(seite):
    """Der Aufrufer (bestaetigen()) muss den Leerwert VOR dem Schreiben abfangen: kein POST, kein
    refresh() (also bleibt AKTUELL auf demselben Feld stehen), und die Fehlermeldung erscheint im
    schon vorhandenen netz-banner statt dass eine falsche 0 im Store landet."""
    page, _ = seite
    _rendere(page, "bruttoarbeitslohn")
    page.fill("#feld-input", "")
    page.evaluate("versteckeNetzFehler()")   # Ausgangszustand definiert, falls ein früherer Test das Banner zeigte
    page.click("#bestaetigen")
    page.wait_for_timeout(200)   # bestaetigen() ist async; kein Netzwerk-Await nötig, aber Microtask-Zeit geben
    assert _aktuelle_frage(page) == "bruttoarbeitslohn", (
        "Die Frage hat gewechselt — es wurde offenbar doch ein Event geschrieben und refresh() gerufen.")
    banner_versteckt = page.evaluate("document.getElementById('netz-banner').hidden")
    banner_text = page.evaluate("document.getElementById('netz-banner').textContent")
    assert banner_versteckt is False, "Fehlerbanner erscheint nicht — der Nutzer erfährt nichts von der Ablehnung."
    assert "gültigen Wert" in banner_text, f"Unerwarteter Banner-Text: {banner_text!r}"
    assert page.is_enabled("#bestaetigen"), "Bestätigen-Button bleibt gesperrt nach der Ablehnung."


def test_bestaetigen_schreibt_explizite_null_erfolgreich(seite, base):
    """Gegenprobe zum vorigen Test: eine echte 0 wird ganz normal bestätigt (kein Fehlerbanner,
    Store trägt danach wert=0) — der Fix darf legitime Nullen nicht mit-blockieren."""
    page, _ = seite
    _rendere(page, "bruttoarbeitslohn")
    page.fill("#feld-input", "0")
    page.evaluate("versteckeNetzFehler()")
    page.click("#bestaetigen")
    page.wait_for_function("!document.getElementById('bestaetigen').disabled", timeout=5000)
    banner_versteckt = page.evaluate("document.getElementById('netz-banner').hidden")
    assert banner_versteckt is True, "Fehlerbanner erscheint bei einer gültigen 0 — falsch abgelehnt."

    # `STAND` ist der BROWSER-Zwischenstand, und der wird von refresh() nachgezogen — nach dem
    # Freigeben des Knopfes. Gemessen 2026-08-27 mit 350 ms je Netzaufruf: hier stand `None`,
    # obwohl die 0 sauber geschrieben war. Der freigegebene Knopf sagt eben nicht, dass der
    # Vorgang durch ist (dieselbe Verwechslung wie in 52fa22a).
    #
    # Gefragt wird deshalb der STORE über echtes HTTP — das ist ohnehin die bessere Quelle: was
    # die Oberfläche zwischengespeichert hat, ist eine Behauptung, was hier steht, ist das
    # Ergebnis. Gewartet wird auf den Zustand, nicht auf eine Zeitspanne.
    page.wait_for_function(
        "STAND && STAND.felder && STAND.felder['bruttoarbeitslohn'] !== undefined", timeout=8000)
    with urllib.request.urlopen(f"{base}/fall/{page.evaluate('FALL')}/stand", timeout=10) as r:
        felder = json.loads(r.read().decode("utf-8")).get("felder", {})
    gespeichert = felder.get("bruttoarbeitslohn")
    assert gespeichert is not None and gespeichert.get("wert") == 0, (
        f"bruttoarbeitslohn steht nicht mit wert=0 im Store: {gespeichert!r}")
