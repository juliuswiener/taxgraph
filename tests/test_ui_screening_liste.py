"""Zehn Kreuze statt zehn Einzelfragen — und 147 Folgefragen weniger.

ANLASS, Julius am 2026-08-25: „wenn wir eine liste von unüblichen/seltenen dingen haben können wir
die auch schnell in einer checkbox abfrage abhandeln … also behindert, einkünfte aus vermietung,
kapitalerträge usw."

GEMESSEN, vorher: die zehn Fragen, die je die EXISTENZ eines ganzen Themas erheben, standen einzeln
über die Queue verteilt — auf den Positionen 2, 4, 5, 8, 9, 18, 19, 27, 33 und 38 von 321. Zwischen
ihnen die Detailfragen genau der Themen, nach denen noch gar nicht gefragt war (das war der Befund
„frage hat keine daseinsberechtigung" derselben Woche, von der anderen Seite gesehen).

Werden alle zehn verneint, sinkt die Queue von 321 auf 174.

WELCHE Fragen dazugehören, sagt die Bindung (`screening: true`) — nicht die Oberfläche und nicht
der Feldname. Ein Filter über „kein_…" wäre dieselbe Heuristik, die hier schon einmal zwei Feldern
das Gegenteil der Nutzerantwort entlockt hat; dafür gibt es `frage_invertiert`, und aus demselben
Grund gibt es jetzt `screening`.

KEIN LLM.
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

import api as API          # noqa: E402
import audit               # noqa: E402
import server as SRV       # noqa: E402
import traverser as TR     # noqa: E402

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
def seite_factory(base):
    gestartet = []
    SONDE = "/fall/auth-sonde-taxgraph/stand"

    def _mach(weg="fragebogen"):
        p = sync_playwright().start()
        gestartet.append(p)
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1280, "height": 900})
        page.fehler = []
        page.on("pageerror", lambda e: page.fehler.append(str(e)))
        page.on("console", lambda m: (
            page.fehler.append(m.text)
            if m.type == "error" and not (m.location or {}).get("url", "").endswith(SONDE)
            else None))
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        page.wait_for_selector("#wegwahl:not([hidden])", timeout=5000)
        page.evaluate(f"document.getElementById('weg-{weg}').click()")
        return page

    try:
        yield _mach
    finally:
        for p in gestartet:
            p.stop()


def _offene(base, fall):
    with urllib.request.urlopen(f"{base}/fall/{fall}/fragen") as a:
        return [q["feld_id"] for q in json.loads(a.read())["fragen"]]


def _stand(base, fall):
    with urllib.request.urlopen(f"{base}/fall/{fall}/stand") as a:
        return json.loads(a.read())["felder"]


# ---------------------------------------------------------------- die Bindung

def test_die_bindung_erklaert_welche_fragen_dazugehoeren():
    """Untergrenze gegen den stillen Leerlauf: ohne `screening`-Felder wäre die ganze Datei grün
    und die Liste im Betrieb leer."""
    b = TR.lade_bindung()
    s = [f for f, e in b.items() if e.get("screening")]
    assert len(s) >= 10, f"Nur {len(s)} screening-Felder: {sorted(s)}"
    # Julius' drei Beispiele müssen dabei sein.
    for f in ("keine_behinderung_pflege", "kein_vuv", "kein_kap"):
        assert f in s, f"{f} fehlt in der Ankreuzliste."
    # Alle sind bool und fragen entgegengesetzt zum Feldnamen — daran hängt die Umkehr beim
    # Speichern. Ein screening-Feld ohne das wäre eine Falle.
    for f in s:
        assert b[f].get("typ") == "bool", f"{f} ist kein bool"
        assert b[f].get("frage_invertiert"), (
            f"{f} ist nicht `frage_invertiert` — dann bedeutet ein Kreuz das Gegenteil.")


# ---------------------------------------------------------------- die Seite

def test_die_liste_steht_am_anfang_des_fragebogens(seite_factory):
    """Der Fragebogen beginnt mit ihr — nicht irgendwo auf Position 2, 4, 5, 8 …"""
    page = seite_factory("fragebogen")
    page.wait_for_selector("#screening:not([hidden])", timeout=8000)
    assert page.is_hidden("#wegpunkt"), (
        "Die Frage-Karte steht daneben — dann sind es zwei Aufforderungen gleichzeitig.")
    n = page.evaluate("document.querySelectorAll('#screening-liste .sc-box').length")
    assert n >= 10, f"Nur {n} Kästchen in der Liste."
    text = page.text_content("#screening")
    for wort in ("Vermietung", "Kapitalerträge", "Behinderung"):
        assert wort in text, f"„{wort}“ kommt in der Liste nicht vor."
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_ein_kreuz_heisst_ja_und_ein_leeres_kaestchen_nein(seite_factory, base):
    """DIE POLARITÄT, und sie ist die Stelle, an der diese Seite still falsch werden könnte: das
    Feld heisst `kein_kap`, die Frage lautet „Hattest du Kapitalerträge?". Ein Kreuz bedeutet
    „ja, hatte ich" — im Store also `kein_kap = false`.

    Geprüft wird der Store, nicht die Anzeige."""
    page = seite_factory("fragebogen")
    page.wait_for_selector("#screening:not([hidden])", timeout=8000)
    fall = page.evaluate("FALL")

    page.evaluate("""() => {
      const b = document.querySelector('#screening-liste .sc-box[data-feld="kein_kap"]');
      b.checked = true;
    }""")
    page.click("#screening-weiter")
    page.wait_for_selector("#screening", state="hidden", timeout=8000)

    felder = _stand(base, fall)
    assert felder["kein_kap"]["wert"] is False, (
        f"Angekreuzt („ja, hatte ich“) wurde als {felder['kein_kap']['wert']!r} gespeichert — der "
        f"Nutzer verliert damit seine Kapitalerträge aus der Erklärung.")
    assert felder["kein_vuv"]["wert"] is True, (
        f"Nicht angekreuzt wurde als {felder['kein_vuv']['wert']!r} gespeichert.")
    assert felder["kein_kap"]["zustand"] == "bestaetigt"


def test_die_liste_nimmt_die_folgefragen_weg(seite_factory, base):
    """Der eigentliche Zweck, in Zahlen: zehn Kreuze statt 147 Einzelfragen."""
    page = seite_factory("fragebogen")
    page.wait_for_selector("#screening:not([hidden])", timeout=8000)
    fall = page.evaluate("FALL")
    vorher = len(_offene(base, fall))

    page.click("#screening-weiter")          # nichts angekreuzt = alles verneint
    page.wait_for_selector("#screening", state="hidden", timeout=8000)

    nachher = _offene(base, fall)
    assert len(nachher) < vorher - 100, (
        f"Die Liste nimmt nur {vorher - len(nachher)} Fragen weg (gemessen: 147). Entweder ist die "
        f"Verbindung zu den Folge-Regeln verloren oder die Antworten kamen nicht an.")
    for f in ("kap_kapitalertraege", "vv_einnahmen", "kind_vorname"):
        assert f not in nachher, f"{f} steht noch offen, obwohl das Thema verneint ist."


def test_nach_der_liste_kommt_der_fragebogen(seite_factory):
    """Kein Sackgassen-Bildschirm: danach steht die erste echte Frage da."""
    page = seite_factory("fragebogen")
    page.wait_for_selector("#screening:not([hidden])", timeout=8000)
    page.click("#screening-weiter")
    page.wait_for_selector("#wegpunkt:not([hidden])", timeout=8000)
    assert page.is_hidden("#screening")
    assert page.text_content("#frage").strip(), "Der Fragebogen zeigt keine Frage."


def test_beim_zweiten_mal_kommt_sie_nicht_wieder(seite_factory, base):
    """Sie fragt nach OFFENEN Screening-Feldern. Sind alle beantwortet, hat sie nichts zu zeigen —
    sonst stünde sie bei jedem Neuladen wieder da und der Nutzer beantwortete dasselbe erneut."""
    page = seite_factory("fragebogen")
    page.wait_for_selector("#screening:not([hidden])", timeout=8000)
    page.click("#screening-weiter")
    page.wait_for_selector("#wegpunkt:not([hidden])", timeout=8000)

    da = page.evaluate("async () => await zeigeScreening()")
    assert da is False, "Die Liste erscheint ein zweites Mal, obwohl alles beantwortet ist."


def test_auf_dem_ki_weg_kommt_sie_nicht_sofort(seite_factory):
    """Wer „erst von der KI ausfüllen lassen" wählt, will schreiben — nicht ankreuzen. Die KI
    erhebt dieselben Dinge aus seinem Satz; die Liste kommt erst beim Wechsel in den Fragebogen
    (zumFragebogen), und dann nur noch mit dem, was offen geblieben ist."""
    page = seite_factory("ki")
    page.wait_for_selector("#chat-body .chat-erklaer", timeout=8000)
    assert page.is_hidden("#screening"), (
        "Die Ankreuzliste steht auf dem KI-Weg im Weg — dort ist NUR das KI-Fenster.")

    page.click("#zum-fragebogen")
    page.wait_for_selector("#screening:not([hidden])", timeout=8000)
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_tap_ziele_und_schmaler_schirm(base):
    """360px ist die Messlatte (tests/test_ui_responsive.py). Zehn Zeilen mit Kästchen und
    Hilfetext sind genau die Art Liste, die dort seitwärts ausbricht."""
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 360, "height": 780})
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        page.wait_for_selector("#wegwahl:not([hidden])", timeout=5000)
        page.evaluate("document.getElementById('weg-fragebogen').click()")
        page.wait_for_selector("#screening:not([hidden])", timeout=8000)

        m = page.evaluate("""() => {
          const zeilen = [...document.querySelectorAll('#screening-liste .sc-label')];
          const ueber = ['#screening', '#screening-liste']
            .map(s => { const e = document.querySelector(s);
                        return {sel: s, ueber: e ? e.scrollWidth - e.clientWidth : -1}; })
            .filter(x => x.ueber > 0);
          return {breite: document.documentElement.scrollWidth,
                  kleinste: Math.min(...zeilen.map(e => e.getBoundingClientRect().height)),
                  ueberlauf: ueber};
        }""")
        browser.close()

    assert m["breite"] <= 360, f"Seitwärts-Scrollen bei 360px: {m['breite']}px"
    assert m["ueberlauf"] == [], f"Text läuft aus seinem Kasten: {m['ueberlauf']}"
    assert m["kleinste"] >= 44, f"Kleinstes Tap-Ziel nur {m['kleinste']:.0f}px (nötig: 44)."
