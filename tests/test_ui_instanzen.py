"""Zwei Kinder brauchen zwei Eingabefelder — nicht zweimal dieselbe Frage.

ANLASS, Julius im echten Durchgang am 2026-08-25:

    „Wie heißen deine Kinder mit Vornamen? … wie hier mehrere angeben bei nur einem feld??"

und, als Lösungsvorschlag, besser als N Fragen hintereinander:

    „oder man gibt direkt n inputfelder anstatt jedesmal einen neue frage."

GEMESSEN: **69 Felder** tragen eine `instanz_gruppe`, 31 davon für Kinder, 22 für
Vermietungsobjekte. Store, ELSTER-Mapping und Bescheid kennen die Achse seit langem
(`est_mapping.parse_instanz`: `base__2`, `base__3` …, die Basis ohne Suffix ist Instanz 1). Der
FRAGEBOGEN kannte sie nicht und fragte jedes Feld genau einmal — wer zwei Kinder hatte, konnte
einen Vornamen eintragen, und für das zweite gab es kein Feld. Das ist ein Abgabe-Blocker: die
Anlage Kind wird je Kind einmal ausgefüllt.

DER TRAVERSER BLEIBT UNANGETASTET. Er liefert das Basisfeld wie bisher EINMAL; die Zahl steht als
`instanz_anzahl` daneben, und die Oberfläche baut daraus die Felder. Das war der eigentliche
Gewinn an Julius' Vorschlag — die Fragen-Queue, der Ring und die Relevanz-Rechnung mussten dafür
nicht angefasst werden.

Nur die Gruppe `kind` ist bisher gepflegt: sie hat als einzige ein Zählfeld im Fragebogen
(`fam_anzahl_kinder`). Die übrigen sieben verhalten sich wie bisher, und das ist ehrlicher als
eine geratene Anzahl.
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

import api as API           # noqa: E402
import audit                # noqa: E402
import est_mapping as EM    # noqa: E402
import server as SRV        # noqa: E402
import store as ST          # noqa: E402
import traverser as TR      # noqa: E402
from ui_hilfen import zum_fragebogen  # noqa: E402

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None


# ---------------------------------------------------------------- die Zählung (ohne Browser)

def test_die_bindung_erklaert_die_gruppe_kind():
    """Untergrenze: ohne den Eintrag wäre alles darunter grün und die Achse im Betrieb tot."""
    g = TR.lade_instanz_gruppen()
    assert "kind" in g, f"Gruppe `kind` fehlt: {sorted(g)}"
    k = g["kind"]
    assert k["anzahl_feld"] == "fam_anzahl_kinder"
    assert k["etikett"] and k["max"] >= 2
    b = TR.lade_bindung()
    assert b[k["anzahl_feld"]].get("askable"), "Das Zählfeld wird gar nicht gefragt."
    assert b[k["anzahl_feld"]].get("typ") == "int"
    # Und die Gruppe muss auch wirklich Felder haben.
    felder = [f for f, e in b.items() if e.get("instanz_gruppe") == "kind"]
    assert len(felder) >= 20, f"Nur {len(felder)} Kind-Felder — Messung prüfen."


def _store_mit(kinder=None):
    b = TR.lade_bindung()
    s = ST.leerer_store(veranlagungszeitraum=2025, fall_id="inst")
    if kinder is not None:
        ST.append_event(s, feld_id="fam_anzahl_kinder", wert=kinder, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                        schreiber="t", signal={"signal_1": None, "signal_2": "t"}, bindung=b)
    return s, b


@pytest.mark.parametrize("kinder,erwartet", [(None, 1), (1, 1), (2, 2), (3, 3), (99, 9)])
def test_die_anzahl_kommt_aus_dem_zaehlfeld(kinder, erwartet):
    """Inklusive Obergrenze: 99 Kinder sind ein Vertipper, nicht 99 Eingabefelder."""
    s, b = _store_mit(kinder)
    n, etikett = TR.instanz_anzahl(s, b, "kind_vorname")
    assert n == erwartet, f"fam_anzahl_kinder={kinder} -> {n} Felder statt {erwartet}"
    assert etikett == "Kind"


def test_ein_vorlaeufiger_wert_zaehlt_nicht():
    """Ein KI-Vorschlag („2 kinder") ist noch nicht bestätigt. Er darf nicht darüber entscheiden,
    wie viele Felder der Nutzer ausfüllen soll — er hat ihn ja noch gar nicht gesehen."""
    b = TR.lade_bindung()
    s = ST.leerer_store(veranlagungszeitraum=2025, fall_id="inst")
    # `katalog=` ist Pflicht für Vorschlags-Schreiber (fail-closed: die KI darf nur Felder
    # vorschlagen, die dafür freigegeben sind). Ohne ihn weist der Store schon das Schreiben ab —
    # der Test käme gar nicht bis zur Messung.
    ST.append_event(s, feld_id="fam_anzahl_kinder", wert=3, zustand="vorlaeufig",
                    herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft",
                              "haftung": "nutzer"},
                    schreiber="llm:chat", signal={"signal_1": None, "signal_2": None},
                    katalog=ST.lade_katalog(b), bindung=b)
    n, _ = TR.instanz_anzahl(s, b, "kind_vorname")
    assert n == 1, f"Ein vorläufiger Vorschlag hat {n} Eingabefelder aufgespannt."


def test_ein_feld_ohne_achse_bleibt_bei_einem():
    """Die Gegenrichtung — ohne sie wäre „immer N Felder" eine bestandene Lösung."""
    s, b = _store_mit(3)
    for f in ("bruttoarbeitslohn", "veranlagung", "ep_arbeitstage"):
        n, etikett = TR.instanz_anzahl(s, b, f)
        assert n == 1 and etikett == "", f"{f} hat plötzlich {n} Instanzen"


def test_eine_gruppe_ohne_zaehlfeld_bleibt_bei_einem():
    """Sieben der acht Gruppen sind (noch) nicht gepflegt. Sie müssen sich verhalten wie bisher —
    eine geratene Anzahl wäre schlimmer als keine."""
    s, b = _store_mit(3)
    ohne = [f for f, e in b.items()
            if e.get("instanz_gruppe") and e["instanz_gruppe"] not in TR.lade_instanz_gruppen()]
    assert ohne, "Alle Gruppen gepflegt — dann prüft dieser Test nichts mehr (dann bitte löschen)."
    n, _ = TR.instanz_anzahl(s, b, ohne[0])
    assert n == 1, f"{ohne[0]} spannt {n} Instanzen auf, obwohl seine Gruppe kein Zählfeld hat."


def test_das_feldformat_ist_dasselbe_das_der_store_liest():
    """`base` ist Instanz 1, ab 2 mit Suffix. Fiele die Oberfläche hier auseinander, schriebe sie
    Werte an Feld-Kennungen, die der ELSTER-Writer nie findet — still, und erst beim Einreichen."""
    assert EM.parse_instanz("kind_vorname") is None, "Die Basis ist Instanz 1, ohne Suffix."
    assert EM.parse_instanz("kind_vorname__2") == ("kind_vorname", 2)
    assert EM.parse_instanz("kind_vorname__3") == ("kind_vorname", 3)


# ---------------------------------------------------------------- die Kosten der Zählung

def test_die_gruppen_werden_einmal_von_platte_gelesen_nicht_je_feld():
    """DIE TEUERSTE ZEILE DIESES BAUSTEINS, und sie stand am 2026-08-25 zwei Stunden lang drin.

    `instanz_anzahl()` fragt `lade_instanz_gruppen()` — und `/fragen` ruft `instanz_anzahl()` für
    JEDE Frage der Queue auf. 69 Felder tragen eine `instanz_gruppe`. Ohne Cache parste ein
    einziger Fragen-Aufruf damit sämtliche bindung_*.yaml 69-mal von Platte.

    GEMESSEN, dieselbe Schleife über 400 Felder: **28,75 s ohne Cache, 0,41 s mit** — Faktor 70.
    Am Endpunkt: `/fragen` 24,7 s, und zwar bei JEDEM Aufruf, also nach jeder beantworteten Frage.
    147 UI-Tests liefen daran in den Timeout; keiner davon zeigte auf diese Funktion, alle sagten
    nur „Timeout beim Warten auf #wegpunkt".

    Der Test misst deshalb die Platten-Reads, nicht die Sekunden: eine Zeitschranke wäre auf einer
    langsamen Maschine mal so, mal so, und genau hier soll nichts wackeln."""
    TR.lade_instanz_gruppen.cache_clear()
    b = TR.lade_bindung()
    s, _ = _store_mit(2)
    mit_achse = [f for f, e in b.items() if e.get("instanz_gruppe")]
    assert len(mit_achse) >= 20, f"Nur {len(mit_achse)} Felder mit Achse — Messung prüfen."
    for f in mit_achse:
        TR.instanz_anzahl(s, b, f)
    info = TR.lade_instanz_gruppen.cache_info()
    assert info.misses == 1, (
        f"{info.misses} Platten-Reads für {len(mit_achse)} Felder statt 1 — der Cache greift "
        f"nicht, und jeder /fragen-Aufruf parst die Bindung erneut.")


def test_der_cache_liefert_dasselbe_wie_der_frische_read():
    """Ein Cache, der etwas anderes liefert als die Datei, wäre schlimmer als der langsame Weg:
    die Zahl der Eingabefelder käme dann aus einem Zustand, den niemand mehr sieht."""
    TR.lade_instanz_gruppen.cache_clear()
    gecacht = TR.lade_instanz_gruppen()
    frisch = TR.lade_instanz_gruppen.__wrapped__()
    assert gecacht == frisch


# ---------------------------------------------------------------- die Oberfläche

pytestmark_browser = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")


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


def _stand(base, fall):
    with urllib.request.urlopen(f"{base}/fall/{fall}/stand") as a:
        return json.loads(a.read())["felder"]


@pytest.fixture
def seite(base):
    if sync_playwright is None:
        pytest.skip("playwright fehlt")
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1280, "height": 900})
        page.fehler = []
        page.on("pageerror", lambda e: page.fehler.append(str(e)))
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        page.wait_for_selector("#wegwahl:not([hidden])", timeout=5000)
        # Klick per evaluate, nicht per Playwright-Klick: letzterer wartet auf „stable", und die
        # Wegwahl trägt die fade-Animation von `.screen`. Gemessen 2026-08-25 — mit `.click()` des
        # Locators lief wegWaehlen() nicht an, mit evaluate sofort.
        page.evaluate("document.getElementById('weg-fragebogen').click()")
        # „Kinder" bejahen — sonst nimmt die Ankreuzliste den ganzen Block aus dem Dialog und es
        # gibt gar kein Kind-Feld mehr zu prüfen.
        # „Kinder" bejahen — sonst nimmt die Ankreuzliste den ganzen Block aus dem Dialog und es
        # gibt gar kein Kind-Feld mehr zu prüfen.
        #
        zum_fragebogen(page, ankreuzen=["kein_kind"])
        yield page, base
        browser.close()


def _zeige(page, feld_id):
    """Die Frage `feld_id` im Fragebogen anzeigen — unabhängig davon, wo sie in der Queue steht."""
    return page.evaluate("""async (fid) => {
      const r = await jget(`/fall/${FALL}/fragen`);
      const q = (r.body.fragen || []).find(x => x.feld_id === fid);
      if (!q) return null;
      AKTUELL = q;
      zeigeFrage(q, STAND);
      return {anzahl: q.instanz_anzahl, etikett: q.instanz_etikett};
    }""", feld_id)


def _setze_kinderzahl(page, n):
    return page.evaluate("""async (n) => {
      const r = await jpost(`/fall/${FALL}/event`, {
        feld_id: 'fam_anzahl_kinder', wert: n, zustand: 'bestaetigt',
        herkunft: {herkunft: 'laie', pruef_tiefe: 'ungeprueft', haftung: 'nutzer'},
        schreiber: 'ui:laie', signal: {signal_1: null, signal_2: 'klick@fam_anzahl_kinder'}});
      if (r.status < 200 || r.status >= 300) return r.status;
      await refresh();
      return r.status;
    }""", n)


def test_drei_kinder_geben_drei_eingabefelder(seite):
    """DER BEFUND. Ein Feld für drei Kinder war die Lücke; drei Felder sind die Antwort."""
    page, _ = seite
    assert _setze_kinderzahl(page, 3) in (200, 201)
    q = _zeige(page, "kind_vorname")
    assert q and q["anzahl"] == 3, f"Der Server meldet {q} statt drei Instanzen."

    m = page.evaluate("""() => {
      const felder = [...document.querySelectorAll('#eingabe .instanz-zeile')];
      return {zeilen: felder.length,
              marken: felder.map(z => z.querySelector('.instanz-marke').textContent),
              ids: felder.map(z => (z.querySelector('input, select') || {}).id)};
    }""")
    assert m["zeilen"] == 3, f"Nur {m['zeilen']} Eingabefelder: {m}"
    assert m["marken"] == ["Kind 1", "Kind 2", "Kind 3"], (
        f"Ohne Nummer weiss der Nutzer nicht, welches Kind gemeint ist: {m['marken']}")
    assert all(m["ids"]), f"Ein Feld ohne id — leseInstanzWerte findet es nicht: {m['ids']}"
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_ein_kind_bleibt_ein_schlichtes_feld(seite):
    """Die Gegenrichtung: bei einem Kind darf nichts anders aussehen als bisher — keine Marke
    „Kind 1", kein Rahmen um eine einzelne Zeile."""
    page, _ = seite
    assert _setze_kinderzahl(page, 1) in (200, 201)
    q = _zeige(page, "kind_vorname")
    assert q["anzahl"] == 1
    n = page.evaluate("document.querySelectorAll('#eingabe .instanz-zeile').length")
    assert n == 0, f"Bei einem Kind stehen {n} Instanz-Zeilen da."


def test_die_werte_landen_unter_base_und_base__n(seite):
    """Das Format ist die Naht zu ELSTER: `kind_vorname`, `kind_vorname__2`, `kind_vorname__3`.
    Schriebe die Oberfläche hier anders, fände der Writer die Werte nie — still, und erst beim
    Einreichen."""
    page, base_url = seite
    fall = page.evaluate("FALL")
    assert _setze_kinderzahl(page, 3) in (200, 201)
    _zeige(page, "kind_vorname")

    page.evaluate("""() => {
      const namen = ['Anna', 'Ben', 'Cem'];
      [...document.querySelectorAll('#eingabe .instanz-zeile input')].forEach((el, i) => {
        el.value = namen[i];
      });
    }""")
    page.click("#bestaetigen")
    page.wait_for_timeout(1200)

    felder = _stand(base_url, fall)
    assert felder.get("kind_vorname", {}).get("wert") == "Anna", (
        f"Instanz 1 fehlt oder steht falsch: {felder.get('kind_vorname')}")
    assert felder.get("kind_vorname__2", {}).get("wert") == "Ben", (
        f"Instanz 2 fehlt — genau das war die Lücke: {sorted(felder)}")
    assert felder.get("kind_vorname__3", {}).get("wert") == "Cem"
    for i in (1, 2, 3):
        fid = "kind_vorname" if i == 1 else f"kind_vorname__{i}"
        assert felder[fid]["zustand"] == "bestaetigt"


def test_eine_leere_instanz_wird_uebersprungen_nicht_verworfen(seite):
    """Wer drei Kinder angegeben hat, aber nur zwei Namen zur Hand hat, soll die zwei speichern
    können. Die dritte Instanz bleibt leer — und damit offen, nicht falsch."""
    page, base_url = seite
    fall = page.evaluate("FALL")
    assert _setze_kinderzahl(page, 3) in (200, 201)
    _zeige(page, "kind_vorname")

    page.evaluate("""() => {
      const els = [...document.querySelectorAll('#eingabe .instanz-zeile input')];
      els[0].value = 'Anna';
      els[2].value = 'Cem';        // die MITTLERE bleibt leer
    }""")
    page.click("#bestaetigen")
    page.wait_for_timeout(1200)

    felder = _stand(base_url, fall)
    assert felder.get("kind_vorname", {}).get("wert") == "Anna"
    assert "kind_vorname__2" not in felder, (
        "Die leere Instanz wurde geschrieben — ein leeres Feld ist keine Antwort (Stille-Null).")
    assert felder.get("kind_vorname__3", {}).get("wert") == "Cem", (
        "Die dritte Instanz ging mit der leeren zweiten verloren.")


def test_alle_leer_schreibt_nichts_und_sagt_es(seite):
    """Gar keine Eingabe ist keine Antwort — dieselbe Regel wie beim einzelnen leeren Feld. Und der
    Nutzer erfährt es, statt auf einen Knopf zu drücken, der nichts tut."""
    page, base_url = seite
    fall = page.evaluate("FALL")
    assert _setze_kinderzahl(page, 2) in (200, 201)
    _zeige(page, "kind_vorname")

    vorher = set(_stand(base_url, fall))
    page.click("#bestaetigen")
    page.wait_for_timeout(800)

    assert set(_stand(base_url, fall)) == vorher, "Leere Felder haben etwas geschrieben."
    banner = page.evaluate(
        "(document.getElementById('netz-banner') || {}).textContent || ''")
    assert banner.strip(), "Der Klick tat nichts und sagte nichts."
