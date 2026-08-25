"""Was der Nutzer über seine eigenen Angaben liest, muss seine Sprache sein — nicht die des Stores.

ANLASS, gemessen 2026-08-24 an Julius' echtem Durchgang. Nach den ersten KI-Vorschlägen stand in
der linken Spalte:

    bruttoarbeitslohn    2500000
    ep_eigenes_kfz          true
    veranlagung         "einzel"

Drei Feld-Kennungen und drei Rohwerte. `2500000` sind 25.000 Euro in Cent — der Nutzer liest die
Zahl aber als seinen Betrag. Die Umwandlung existierte längst (`verstandenWertText` für die
Bestätigungsliste, `_wert_klartext` im Server); diese Liste benutzte sie nur nicht, weil `/stand`
die dafür nötigen Metadaten nicht mitschickte.

Im selben Bild, an derselben Stelle: die Überschrift „Schon beantwortet" stand über einer LEEREN
Liste — eine Zusage, die nichts einlöst.

Und im Kopf: „Bescheid: —" mit dem Untertitel „steht". Beides zusammen ist eine Behauptung über
einen Wert, den es nicht gibt: `euro(null)` ergibt „—", und die Bedingung „min === max" ist für
zwei null-Grenzen erfüllt. Die Software sagte „steht" an einer Stelle, an der die Rechnung noch
gar nichts hergibt.

KEIN LLM: geschrieben wird über /event, wie bei einer Antwort im Fragebogen.
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
def page(base):
    with sync_playwright() as p:
        browser = p.chromium.launch()
        seite = browser.new_page(viewport={"width": 1280, "height": 900})
        seite.goto(base)
        seite.wait_for_load_state("networkidle")
        seite.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        seite.wait_for_selector("#weg-fragebogen", timeout=5000).click()
        zum_fragebogen(seite)   # Ankreuzliste am Anfang, s. tests/ui_hilfen.py
        yield seite
        browser.close()


def _schreib(base, fall, feld_id, wert):
    ev = {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
          "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
          "schreiber": "ui:laie",
          "signal": {"signal_1": None, "signal_2": f"test@{feld_id}"}}
    r = urllib.request.Request(f"{base}/fall/{fall}/event", method="POST",
                               data=json.dumps(ev).encode(),
                               headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(r) as a:
        return a.status


def _zeilen(page):
    return page.evaluate("""() => [...document.querySelectorAll('#belegt-liste .zeile')].map(li => ({
        name: (li.querySelector('.z-name') || {}).textContent || '',
        kennung: (li.querySelector('.z-name') || {}).title || '',
        wert: (li.querySelector('.z-wert') || {}).textContent || '',
    }))""")


# ---------------------------------------------------------------- die Liste selbst

def test_die_liste_zeigt_frage_und_lesbaren_wert(page, base):
    """Der Befund. Ein Cent-Betrag als Euro, ein Ja/Nein als Ja/Nein, und davor die FRAGE."""
    fall = page.evaluate("FALL")
    assert _schreib(base, fall, "bruttoarbeitslohn", 2500000) in (200, 201)
    page.evaluate("() => refresh()")
    page.wait_for_selector("#belegt-liste .zeile", timeout=5000)

    z = _zeilen(page)
    assert len(z) == 1, f"Genau eine Zeile erwartet: {z}"
    assert z[0]["wert"] != "2500000", (
        "Der Cent-Rohwert steht wieder da — der Nutzer liest 2500000 als seinen Betrag.")
    assert "25.000" in z[0]["wert"] and "€" in z[0]["wert"], (
        f"Kein lesbarer Euro-Betrag: {z[0]['wert']!r}")
    assert z[0]["name"] != "bruttoarbeitslohn", "Die Feld-Kennung steht wieder als Name da."
    assert len(z[0]["name"]) > 15 and "?" in z[0]["name"], (
        f"Der Name ist keine Frage: {z[0]['name']!r}")
    # Die Kennung bleibt erreichbar — sie ist für Fehlersuche nützlich, nur nicht im Weg.
    assert z[0]["kennung"] == "bruttoarbeitslohn", (
        f"Die Feld-Kennung ist ganz verschwunden: {z[0]['kennung']!r}")


def test_ein_bool_steht_als_ja_oder_nein_da(page, base):
    """`true` ist kein deutsches Wort. Und bei den `kein_`-Feldern ist die ANTWORT das Gegenteil
    des gespeicherten Werts — die Anzeige muss dieselbe Umkehr machen wie der Schreibpfad, sonst
    liest der Nutzer das Gegenteil dessen, was er geantwortet hat."""
    fall = page.evaluate("FALL")
    assert _schreib(base, fall, "kein_kap", True) in (200, 201)
    page.evaluate("() => refresh()")
    page.wait_for_selector("#belegt-liste .zeile", timeout=5000)

    z = _zeilen(page)
    assert z[0]["wert"] in ("Ja", "Nein"), f"Rohwert statt Ja/Nein: {z[0]['wert']!r}"
    # kein_kap ist invertiert: gespeichert true ("keine Kapitalerträge") heisst als ANTWORT auf
    # „Hattest du Kapitalerträge?" -> Nein.
    assert z[0]["wert"] == "Nein", (
        f"Die Umkehr der kein_-Felder fehlt in der Anzeige: {z[0]['wert']!r} — der Nutzer liest "
        f"das Gegenteil seiner eigenen Antwort.")


def test_ohne_antworten_steht_die_ueberschrift_nicht_da(page, base):
    """„Schon beantwortet" über einer leeren Liste sagt dem Nutzer, es gäbe schon etwas zu sehen."""
    assert page.is_hidden(".belegt"), (
        "Die Überschrift „Schon beantwortet“ steht da, obwohl nichts beantwortet ist.")

    fall = page.evaluate("FALL")
    _schreib(base, fall, "bruttoarbeitslohn", 2500000)
    page.evaluate("() => refresh()")
    page.wait_for_selector("#belegt-liste .zeile", timeout=5000)
    assert not page.is_hidden(".belegt"), (
        "Und jetzt fehlt sie, obwohl etwas beantwortet ist — die Gegenrichtung.")


def test_ein_langer_wert_quetscht_die_frage_nicht_zu_tode(base):
    """GEMESSEN IM BILD, nicht im Test: der Fragetext brach BUCHSTABENWEISE um — „G / i / b / s /
    t / d / u / …", eine Spalte aus einzelnen Zeichen.

    Ursache war mein eigener erster Fix: `overflow-wrap:anywhere` auf beiden Spalten. `anywhere`
    erlaubt nicht nur den Bruch mitten im Wort, es setzt auch die intrinsische Mindestbreite auf 0 —
    der lange Wert „Zusammenveranlagung mit Ehe- oder Lebenspartner" drückte die Frage daneben auf
    Nullbreite.

    WARUM DER BESTEHENDE TEST DAS NICHT FING: er prüft, dass die Seite nicht seitwärts scrollt. Das
    ist erfüllt, wenn jedes Zeichen auf einer eigenen Zeile steht — die Seite passt dann perfekt und
    ist trotzdem unlesbar. „Passt in die Breite" und „ist lesbar" sind zwei verschiedene Fragen.

    Gemessen werden beide: die Breite (die Ursache) und die Höhe (die Wirkung)."""
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 360, "height": 780})
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
        zum_fragebogen(page)   # Ankreuzliste am Anfang, s. tests/ui_hilfen.py
        fall = page.evaluate("FALL")
        # Genau der Fall aus dem Bild: langes enum-Label neben langem Fragetext.
        _schreib(base, fall, "veranlagung", "zusammen")
        page.evaluate("() => refresh()")
        page.wait_for_selector("#belegt-liste .zeile", timeout=5000)

        m = page.evaluate("""() => {
          const li = document.querySelector('#belegt-liste .zeile');
          const n = li.querySelector('.z-name'), w = li.querySelector('.z-wert');
          const zeilenhoehe = parseFloat(getComputedStyle(n).lineHeight) || 20;
          return {name_breite: n.getBoundingClientRect().width,
                  name_hoehe: n.getBoundingClientRect().height,
                  zeilen: Math.round(n.getBoundingClientRect().height / zeilenhoehe),
                  name_text: n.textContent, wert_breite: w.getBoundingClientRect().width,
                  wert_text: w.textContent};
        }""")
        browser.close()

    assert len(m["name_text"]) > 20, f"Vorbedingung: der Fragetext muss lang sein — {m['name_text']!r}"
    assert m["name_breite"] >= 80, (
        f"Der Fragetext hat nur {m['name_breite']:.0f}px — bei 360px Schirm bricht er dann "
        f"buchstabenweise um. Der Wert daneben nimmt {m['wert_breite']:.0f}px.")
    assert m["zeilen"] <= 6, (
        f"Der Fragetext läuft über {m['zeilen']} Zeilen ({m['name_hoehe']:.0f}px hoch) — das ist "
        f"kein Umbruch mehr, das ist eine Buchstabensäule.")


# ---------------------------------------------------------------- der Kopf

def test_ohne_zahl_behauptet_der_kopf_nicht_dass_etwas_feststeht(page):
    """DER SCHIEFE SATZ. „Bescheid: —" mit „steht" darunter: `euro(null)` ergibt den Gedankenstrich,
    und `min === max` ist für zwei null-Grenzen wahr. Die Oberfläche behauptete damit, ein Wert
    stünde fest, wo die Rechnung noch nichts hergibt.

    Geprüft wird der frische Fall — genau der Zustand aus Julius' Screenshot."""
    kopf = page.evaluate("""() => ({
        spanne: document.getElementById('spanne').textContent,
        hint: document.getElementById('spanne-hint').textContent,
    })""")
    assert "—" not in kopf["spanne"], (
        f"Ein Gedankenstrich als Wert: {kopf!r}")
    assert kopf["hint"] != "steht", (
        f"„steht“ ohne Zahl daneben: {kopf!r} — das behauptet etwas, das nicht stimmt.")
    # Und es muss trotzdem etwas dastehen: ein leerer Kopf sagt dem Nutzer gar nichts.
    assert kopf["spanne"].strip(), f"Der Kopf ist leer: {kopf!r}"
