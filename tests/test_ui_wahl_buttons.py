"""Ja/Nein und kurze Auswahllisten sind Buttons, kein Dropdown.

Julius beim Durchklicken (2026-08-14): "Außerdem sollten wir Buttons für Ja und
Antwortmöglichkeiten oder wo es eine fixe Anzahl an Antwortmöglichkeiten gibt."

Vorher war jede bool- und enum-Frage ein <select>: zwei Interaktionen für eine Information, auf
dem Handy ein Systemdialog über der ganzen Seite. Jetzt Buttons bis WAHL_MAX Optionen, darüber
weiter ein Dropdown (16 Bundesländer als Button-Reihe wären eine Tapete).

Der heikle Teil ist NICHT das Aussehen, sondern dass der Wert unverändert ankommt. Zwei Fallen:

  1. leseWert() liest `#feld-input`.value. Bei der Button-Gruppe trägt ein verstecktes input im
     Container diese id — der Container darf sie nicht überschreiben, sonst findet leseWert() ein
     <div> ohne .value und schickt undefined.
  2. app.js invertiert bool-Antworten bei Feldern mit "kein_"-Präfix (feld_id.startsWith("kein_")
     ? !ja : ja). Die Screening-Fragen sind positiv gestellt ("Hattest du Kapitalerträge?") und
     negativ benannt (kein_kap). Ein Button-Umbau, der diese Inversion verliert, dreht lautlos
     jede Einkunftsart um — und das sieht man dem Bildschirm nicht an.

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
        page = browser.new_page(viewport={"width": 360, "height": 780})
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


def test_erste_frage_ist_eine_button_wahl(seite):
    """veranlagung steht seit der Graph-Sortierung an erster Stelle und ist ein enum mit zwei
    Werten — also der Normalfall für die Button-Darstellung."""
    page, _ = seite
    assert _aktuelle_frage(page) == "veranlagung", (
        f"Erwartet veranlagung als erste Frage, ist {_aktuelle_frage(page)!r} — "
        f"die Reihenfolge hat sich geändert, der Test misst dann etwas anderes.")
    assert page.query_selector(".wahl .wahl-opt") is not None, "Keine Auswahl-Buttons gerendert"
    assert page.query_selector("#eingabe select") is None, "Es ist weiter ein Dropdown"


def test_buttons_tragen_den_anzeigetext_nicht_den_rohwert(seite):
    page, _ = seite
    texte = page.eval_on_selector_all(".wahl-opt", "els => els.map(e => e.textContent)")
    assert any("Zusammenveranlagung" in t for t in texte), (
        f"Anzeigetexte fehlen auf den Buttons: {texte}")
    assert not any(t.strip() in ("einzel", "zusammen") for t in texte), (
        f"Rohwert steht auf einem Button: {texte}")


def test_klick_setzt_den_wert_und_markiert_den_button(seite):
    page, _ = seite
    page.click(".wahl-opt[data-wert='zusammen']")
    assert page.input_value("#feld-input") == "zusammen", (
        "Der Klick landet nicht im versteckten Feld — leseWert() bekäme nichts.")
    aktiv = page.eval_on_selector_all(".wahl-opt.aktiv", "els => els.map(e => e.dataset.wert)")
    assert aktiv == ["zusammen"], f"Genau ein Button darf aktiv sein, aktiv: {aktiv}"
    assert page.get_attribute(".wahl-opt[data-wert='zusammen']", "aria-pressed") == "true"


def test_vorauswahl_folgt_dem_beispielwert_nicht_der_ersten_option(seite):
    """Gemessen: ALLE 85 askable bool/enum-Felder tragen einen beispielwert, es gibt keins ohne.
    Die Vorbelegung ist also gewollt (Kommentar in app.js: "beispielwert als Vorauswahl statt fix
    'Ja'") — sie muss aber dem beispielwert folgen und nicht einfach der ersten Option.

    kein_kap ist der aussagekräftige Fall: beispielwert False, und "Nein" ist die ZWEITE Option.
    Bei veranlagung fielen beide zusammen, der Test würde dort auch bei kaputter Logik grün.

    Dass überhaupt vorbelegt wird, ist unbedenklich: gespeichert wird erst mit dem Hold-Confirm,
    also nie ohne bewusste Handlung des Nutzers."""
    page, _ = seite
    page.evaluate("""async (fid) => {
        const r = await jget(`/fall/${FALL}/fragen`);
        const q = r.body.fragen.find(f => f.feld_id === fid);
        if (q) { AKTUELL = q; zeigeFrage(q, STAND); }
    }""", "kein_kap")
    assert _aktuelle_frage(page) == "kein_kap", "kein_kap nicht erreichbar"
    assert page.input_value("#feld-input") == "false", (
        "Vorauswahl folgt nicht dem beispielwert (False) — vermutlich steht die erste Option da.")
    aktiv = page.eval_on_selector_all(".wahl-opt.aktiv", "els => els.map(e => e.dataset.wert)")
    assert aktiv == ["false"], f"Markiert sein muss der beispielwert-Button, aktiv: {aktiv}"


def test_bool_inversion_bleibt_erhalten(seite, base):
    """Die eigentliche Gefahr des Umbaus. kein_kap ist negativ benannt, die Frage positiv
    gestellt ("Hattest du Kapitalerträge?"); app.js invertiert beim Speichern. Wer "Ja" klickt,
    muss kein_kap=false im Store haben — sonst kippt die Einkunftsart lautlos ins Gegenteil."""
    page, basis = seite
    fall = page.evaluate("FALL")
    # zu kein_kap navigieren: alle davor liegenden Fragen per API beantworten wäre aufwendig —
    # stattdessen die Frage direkt rendern und den Klickweg messen.
    page.evaluate("""async (fid) => {
        const r = await jget(`/fall/${FALL}/fragen`);
        const q = r.body.fragen.find(f => f.feld_id === fid);
        if (q) { AKTUELL = q; zeigeFrage(q, STAND); }
    }""", "kein_kap")
    assert _aktuelle_frage(page) == "kein_kap", "kein_kap nicht erreichbar"
    page.click(".wahl-opt[data-wert='true']")          # Nutzer klickt "Ja, ich hatte Kapitalerträge"
    wert = page.evaluate("leseWert(AKTUELL)")
    assert wert is False, (
        f"'Ja' auf 'Hattest du Kapitalerträge?' muss kein_kap=False ergeben, ist {wert!r} — "
        f"die kein_-Inversion aus app.js ist beim Button-Umbau verloren gegangen.")


def test_lange_auswahl_bleibt_ein_dropdown(seite):
    """Ab WAHL_MAX Optionen wäre die Button-Reihe länger als der Bildschirm. dba_staat hat 16
    Werte — dort muss das Dropdown bleiben."""
    page, _ = seite
    page.evaluate("""async (fid) => {
        const r = await jget(`/fall/${FALL}/fragen`);
        const q = r.body.fragen.find(f => f.feld_id === fid);
        if (q) { AKTUELL = q; zeigeFrage(q, STAND); }
    }""", "dba_staat")
    if _aktuelle_frage(page) != "dba_staat":
        pytest.skip("dba_staat in dieser Scheibe nicht offen")
    assert page.query_selector("#eingabe select") is not None, (
        "16 Staaten werden als Button-Reihe gerendert — das sprengt die Ansicht.")


def test_keine_horizontale_scrollbar_bei_360px(seite):
    """Dieselbe Falle wie bei den enum-Labels (scrollWidth 458 > 360): Buttons mit langen Texten
    dürfen die Mobilansicht nicht sprengen, sie müssen umbrechen."""
    page, _ = seite
    breite = page.evaluate("document.documentElement.scrollWidth")
    assert breite <= 360, f"scrollWidth={breite} > 360 — die Seite scrollt seitlich."
