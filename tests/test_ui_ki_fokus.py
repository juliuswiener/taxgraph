"""Wer über die KI arbeitet, braucht den Platz dort — nicht in einer 360px-Leiste.

ANLASS, wörtlich (Julius, 2026-08-23, im echten Durchgang): „die kis sidebar ist zu klein wenn das
aber der bereich ist in dem der user gerade arbeitet … es sollte den main fokus bekommen. der rest
ist in dem moment nicht wichtig bekommt aber den größten real estate."

Davor war die Wegwahl eine Wahl ohne Unterschied: beide Knöpfe öffneten denselben Bildschirm, der
eine setzte bloss den Textcursor woanders hin. „Erst von der KI ausfüllen lassen" versprach etwas
anderes und lieferte den Fragebogen mit einer schmalen Leiste daneben.

DIE KORREKTUR IST EIN SPALTENTAUSCH, KEIN ZWEITER MODUS. Der Fragebogen bleibt sichtbar — er ist
der Zielort, und er zeigt weiter Ring, Spanne und Fortschritt —, tritt aber auf rund ein Drittel
zurück. Mein ursprünglicher Einwand gegen einen „KI-Modus" war, er überlebe kein Neuladen; für eine
reine Anzeige-Klasse ist das gerade richtig: wer neu lädt, steht nicht mehr am Anfang, und dann ist
die Leiste wieder die passende Form.

GEMESSEN WERDEN PIXEL, NICHT DIE KLASSE. Eine Klasse lässt sich vergeben, ohne dass man etwas
sieht — dieselbe Falle wie bei borderLeftStyle in tests/test_ui_chat_aussagen.py. Hier wird deshalb
die tatsächliche Breite von #berater gegen die des Fragebogens gestellt.

KEIN LLM: /chat wird per page.route abgefangen.
"""
from __future__ import annotations

import json
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

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None

pytestmark = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")

# Breit genug, dass die Zweispalten-Regel (@media min-width:900px) überhaupt greift.
BREIT, HOCH = 1280, 900


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

    def _mach(weg="ki"):
        p = sync_playwright().start()
        gestartet.append(p)
        browser = p.chromium.launch()
        ctx = browser.new_context(viewport={"width": BREIT, "height": HOCH})
        page = ctx.new_page()
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
        page.wait_for_selector("#flow:not([hidden])", timeout=5000)
        # Auf den ZUSTAND warten, nicht auf eine Zeitspanne. Hier standen 300ms; die reichten, bis
        # /stand am 2026-08-24 zusätzlich die Anzeige-Metadaten je Feld baute — danach war nach
        # 300ms `AKTUELL` noch null und #wegpunkt noch versteckt (gemessen: fertig zwischen 300 und
        # 1000ms). Eine feste Wartezeit misst die Geschwindigkeit des Rechners mit; unter Last
        # kippt sie.
        #
        # Je Weg ein anderer Marker, und zwar zwangsläufig: auf dem KI-Weg ist #wegpunkt seit
        # 2026-08-25 gar nicht mehr im Bild („KI fenster Only"). Dort ist der Marker die
        # Einführungszeile im Verlauf — wegWaehlen() hängt sie NACH `await refresh()` an, sie ist
        # also genau dann da, wenn der Fluss steht.
        if weg == "ki":
            page.wait_for_selector("#chat-body .chat-erklaer", timeout=8000)
        else:
            page.wait_for_selector("#wegpunkt:not([hidden])", timeout=8000)
        page.wait_for_timeout(120)   # nur noch das Layout setzen lassen
        return page

    try:
        yield _mach
    finally:
        for p in gestartet:
            p.stop()


def _breiten(page):
    """Die tatsächlichen Spaltenbreiten von #flow: links der Fragebogen, rechts der Berater.

    GEMESSEN WIRD DAS GRID, NICHT EIN EINZELNES KIND. Die frühere Fassung suchte das sichtbare
    Element der linken Spalte (#rueckfragen, #verstanden, #wegpunkt, #fertig) und stellte dessen
    Breite gegen #berater. Das trug zwei Fallen, beide gemessen:

      1. Ist gerade KEINES davon sichtbar — genau der Fall während der Rückfragen, die #wegpunkt
         ausblenden —, misst man 0px gegen die Beraterbreite und ist tautologisch grün.
      2. Seit dem Umzug (2026-08-24) liegt #rueckfragen IM Berater. Es als „Inhalt" zu messen
         hiesse, den Berater gegen sich selbst zu stellen.

    Die Spaltenbreiten hängen an keinem Schritt und an keinem Kind: sie sind genau das, was Julius
    „real estate" nennt.
    """
    return page.evaluate("""() => {
      const f = document.getElementById('flow');
      const s = getComputedStyle(f).gridTemplateColumns.split(' ').map(parseFloat)
                  .filter(x => !Number.isNaN(x));
      const b = document.getElementById('berater');
      return {links: s[0] || 0, rechts: s.length > 1 ? s[1] : 0, spalten: s.length,
              berater: b ? b.getBoundingClientRect().width : 0};
    }""")
    # Anmerkung zur Wartung: #rueckfragen UND #verstanden liegen seit 2026-08-24 im Berater. Sie
    # dürfen in keiner „Inhaltsspalte"-Messung auftauchen — sonst stünde der Berater gegen sich
    # selbst. Die Spaltenmessung oben ist genau deshalb von den Kindern unabhängig.


def _sichtbar(page):
    """Was vom Fluss tatsächlich im Bild steht — die direkten Kinder von #flow, gefiltert.

    Seit „KI fenster Only" (2026-08-25) ist DAS die entscheidende Messung: auf dem KI-Weg ist
    #flow gar kein Grid mehr, eine Spaltenmessung liefe dort ins Leere. Gemessen wird die
    tatsächliche Sichtbarkeit (Breite > 0 UND display), nicht das `hidden`-Attribut: das
    Ausblenden geschieht per CSS, `hidden` bliebe unverändert und die Prüfung wäre blind.
    """
    return page.evaluate("""() => {
      const da = e => !!(e && e.getBoundingClientRect().width > 0
                         && e.getBoundingClientRect().height > 0
                         && getComputedStyle(e).display !== 'none');
      const flow = document.getElementById('flow');
      return {
        kinder: [...flow.children].filter(da)
                  .map(e => e.id || String(e.className).split(' ')[0] || e.tagName.toLowerCase()),
        berater: Math.round(document.getElementById('berater').getBoundingClientRect().width),
        fenster: window.innerWidth,
        fragebogen: da(document.getElementById('wegpunkt')),
        ring: da(document.getElementById('ring')),
        belegt: da(document.querySelector('.belegt')),
      };
    }""")


def _antwort(rueckfragen=None, vorschlaege=None):
    def _v(v):
        return {"feld_id": v["feld_id"], "wert": v["wert"], "beleg": v.get("beleg", ""),
                "frage": "Frage zu " + v["feld_id"], "typ": "cent", "frage_invertiert": False,
                "einheit": None, "enum_labels": None, "aussage": 0, "rechenweg": None,
                "event_id": "EV-STUB"}
    return {"vorschlaege": [_v(v) for v in (vorschlaege or [])], "abgelehnt": [],
            "abgelehnt_gruende": {}, "konflikte": [], "antwort": "", "unsicher": False,
            "hinweis": "", "aussagen": [], "rueckfragen": rueckfragen or []}


def _stub(page, daten):
    page.route("**/chat", lambda route: route.fulfill(
        status=200, content_type="application/json", body=json.dumps(daten)))


def _senden(page, text="ich hatte 62000 euro brutto"):
    page.fill("#chat-text", text)
    page.evaluate("document.getElementById('chat-send').click()")
    page.wait_for_timeout(1200)


# ---------------------------------------------------------------- der Kern

def test_auf_dem_ki_weg_steht_NUR_das_ki_fenster(seite_factory):
    """DER BEFUND, in seiner dritten und endgültigen Fassung.

    Erst gab die Wegwahl dem Chat gar keinen Platz (2026-08-23). Dann bekam er zwei Drittel, und
    der Fragebogen trat auf ein Drittel zurück (2026-08-24). Julius im echten Durchgang am
    2026-08-25, mit Screenshot: „hier ist immernoch der fragebogen. das ist der screen den ich sehe
    nachdem ich angeklickt habe: ich will mit ki loslegen. dem user dann trotzdem direkt den
    fragebogen zu zeigen ist gegen den willen des users. KI fenster Only."

    Zweimal war meine Annahme zu zaghaft: „zurücktreten, nicht verschwinden". Wer „erst von der KI
    ausfüllen lassen" wählt, hat den Fragebogen gerade ABGEWÄHLT. Ihn trotzdem hinzustellen macht
    die Wahl bedeutungslos."""
    page = seite_factory("ki")
    s = _sichtbar(page)
    assert s["kinder"] == ["berater"], (
        f"Neben dem KI-Fenster steht noch etwas: {s['kinder']} — auf diesem Weg gibt es nur das "
        f"Panel.")
    assert not s["fragebogen"], "Die Frage-Karte des Fragebogens steht im Bild."
    assert not s["ring"], "Der Ring des Fragebogens steht im Bild."
    assert not s["belegt"], "Die Liste der beantworteten Felder steht im Bild."
    assert s["berater"] > s["fenster"] * 0.5, (
        f"berater={s['berater']}px von {s['fenster']}px — das ist nicht der „größte real estate\".")


def test_auf_dem_fragebogen_weg_bleibt_die_leiste_schmal(seite_factory):
    """Die Gegenrichtung, und sie ist genauso wichtig: begleitet die KI nur, gehört ihr die
    Leiste. Ohne diesen Test wäre „immer breit" eine bestandene Lösung."""
    page = seite_factory("fragebogen")
    b = _breiten(page)
    assert b["rechts"] < b["links"], (
        f"Auch ohne KI-Weg ist der Chat der größte Bereich: links={b['links']:.0f}px, "
        f"rechts={b['rechts']:.0f}px")


def test_bei_rueckfragen_bleibt_der_ki_fokus(seite_factory):
    """UMGEDREHT AM 2026-08-24, und die alte Fassung war MEINE falsche Annahme, nicht bloss eine
    zu enge Messung: hier stand „bei Rückfragen wandert der Platz zurück", weil ich Rückfragen für
    Arbeit am Fragebogen hielt. Sie sind das Gegenteil — der Nutzer hat „erst von der KI ausfüllen
    lassen" gewählt und wartet auf die KI. Julius aus dem echten Durchgang: „der user wartet auf
    die rückfragen bzw zustimmungen."

    Wirkung des alten Standes, im Screenshot sichtbar: der Spaltentausch war genau im wichtigsten
    Moment wieder aus — Rückfrage links breit, KI-Panel rechts auf 360px."""
    page = seite_factory("ki")
    assert _sichtbar(page)["kinder"] == ["berater"], "Vorbedingung: der KI-Fokus muss anliegen"
    _stub(page, _antwort(rueckfragen=[{"frage": "Wie viele Tage?",
                                       "feld_id": "ep_arbeitstage", "aussage": 0}]))
    _senden(page)
    page.wait_for_selector("#rueckfragen:not([hidden])", timeout=8000)
    page.wait_for_timeout(300)
    s = _sichtbar(page)
    assert s["kinder"] == ["berater"], (
        f"Bei den Rückfragen taucht der Fragebogen wieder auf: {s['kinder']} — der Nutzer ist noch "
        f"auf dem KI-Weg.")


def test_bei_bestaetigungen_bleibt_der_ki_fokus(seite_factory):
    """Dasselbe für die Verstanden-Seite, aus demselben Grund: was die KI vorgeschlagen hat zu
    bestätigen, ist die letzte Stufe des KI-Wegs — nicht Arbeit im Fragebogen."""
    page = seite_factory("ki")
    _stub(page, _antwort(vorschlaege=[{"feld_id": "bruttoarbeitslohn", "wert": 6200000,
                                       "beleg": "62000 euro brutto"}]))
    _senden(page)
    page.wait_for_selector("#verstanden:not([hidden])", timeout=8000)
    page.wait_for_timeout(300)
    s = _sichtbar(page)
    assert s["kinder"] == ["berater"], (
        f"Beim Bestätigen taucht der Fragebogen wieder auf: {s['kinder']} — auch das ist noch der "
        f"KI-Weg.")


def test_es_gibt_einen_weg_zurueck_in_den_fragebogen(seite_factory):
    """DIE GEGENRICHTUNG, und seit „KI fenster Only" ist sie überlebenswichtig: wenn NUR das Panel
    dasteht, ist ein Weg zurück keine Bequemlichkeit mehr, sondern die einzige Tür.

    Vorher prüfte dieser Test, dass der Fokus endet, sobald der Nutzer im Fragebogen antwortet —
    das ging über die Frage-Karte, die es auf diesem Weg nicht mehr gibt. Jetzt über den Knopf.

    Ohne diesen Test wäre „immer nur das Panel" eine bestandene Lösung, und der Nutzer säße fest."""
    page = seite_factory("ki")
    assert _sichtbar(page)["kinder"] == ["berater"], "Vorbedingung: nur das Panel"

    knopf = page.query_selector("#zum-fragebogen")
    assert knopf is not None and knopf.bounding_box() is not None, (
        "Es gibt keinen sichtbaren Weg zurück in den Fragebogen — der Nutzer sitzt im Panel fest.")
    page.click("#zum-fragebogen")
    page.wait_for_selector("#wegpunkt:not([hidden])", timeout=8000)
    page.wait_for_timeout(200)

    s = _sichtbar(page)
    assert s["fragebogen"], f"Nach dem Klick ist der Fragebogen nicht da: {s}"
    assert s["ring"], f"Und der Ring fehlt auch: {s}"
    assert "berater" in s["kinder"], (
        f"Der Berater ist verschwunden: {s['kinder']} — er soll Begleiter werden, nicht weg sein.")
    b = _breiten(page)
    assert b["rechts"] < b["links"], (
        f"Der Chat ist im Fragebogen immer noch der Hauptbereich: links={b['links']:.0f}px, "
        f"rechts={b['rechts']:.0f}px")


def test_der_rueckweg_ist_nur_auf_dem_ki_weg_da(seite_factory):
    """Im Fragebogen wäre „Weiter im Fragebogen" ein Knopf, der nichts tut — und der Nutzer würde
    einmal darauf klicken, um herauszufinden, was er soll."""
    page = seite_factory("fragebogen")
    knopf = page.query_selector("#zum-fragebogen")
    assert knopf is None or knopf.bounding_box() is None, (
        "Der Rückweg-Knopf steht auch im Fragebogen, wo er nichts bedeutet.")


def test_der_wizard_steht_im_ki_fenster(seite_factory):
    """DIE INTEGRATION SELBST (Julius 2026-08-24: „wizard und ki fenster miteinander integrieren").

    Vorher war die Rückfrage eine eigene Karte in der LINKEN Spalte, während der Verlauf rechts
    dieselbe Frage ein zweites Mal zeigte. Jetzt ist es ein Fenster: Verlauf oben, die Frage
    darunter, das Eingabefeld darunter — in dieser Reihenfolge, denn genau so läuft das Gespräch.

    Gemessen wird die LAGE im Dokument und die Reihenfolge, nicht eine Klasse: `#rueckfragen`
    könnte im Berater hängen und trotzdem über dem Verlauf oder unter der Eingabe stehen."""
    page = seite_factory("ki")
    _stub(page, _antwort(rueckfragen=[{"frage": "Wie viele Tage?",
                                       "feld_id": "ep_arbeitstage", "aussage": 0}]))
    _senden(page)
    page.wait_for_selector("#rueckfragen:not([hidden])", timeout=8000)

    lage = page.evaluate("""() => {
      const rf = document.getElementById('rueckfragen');
      const berater = document.getElementById('berater');
      const verlauf = document.getElementById('chat-body');
      const eingabe = document.querySelector('.chat-eingabe');
      const nach = (a, b) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) > 0;
      return {im_berater: berater.contains(rf),
              nach_verlauf: nach(verlauf, rf),
              vor_eingabe: nach(rf, eingabe),
              rf_oben: rf.getBoundingClientRect().top,
              eingabe_oben: eingabe.getBoundingClientRect().top};
    }""")
    assert lage["im_berater"], "Der Wizard hängt nicht im KI-Fenster — er ist wieder eine eigene Karte."
    assert lage["nach_verlauf"], "Der Wizard steht ÜBER dem Verlauf; das Gespräch liest sich rückwärts."
    assert lage["vor_eingabe"], "Der Wizard steht unter dem Eingabefeld."
    assert lage["rf_oben"] < lage["eingabe_oben"], (
        f"Auch im Bild steht der Wizard nicht über der Eingabe: {lage}")


def test_die_bestaetigungen_stehen_auch_im_ki_fenster(seite_factory):
    """Nach den Rückfragen kommen die Bestätigungen — dieselbe Kette, derselbe Ort. Stünden sie
    wieder links, spränge der Nutzer mitten im KI-Weg von rechts nach links und zurück."""
    page = seite_factory("ki")
    _stub(page, _antwort(vorschlaege=[{"feld_id": "bruttoarbeitslohn", "wert": 6200000,
                                       "beleg": "62000 euro brutto"}]))
    _senden(page)
    page.wait_for_selector("#verstanden:not([hidden])", timeout=8000)

    lage = page.evaluate("""() => {
      const v = document.getElementById('verstanden');
      const b = document.getElementById('berater');
      const e = document.querySelector('.chat-eingabe');
      const nach = (a, z) => (a.compareDocumentPosition(z) & Node.DOCUMENT_POSITION_FOLLOWING) > 0;
      return {im_berater: b.contains(v), vor_eingabe: nach(v, e),
              nach_verlauf: nach(document.getElementById('chat-body'), v)};
    }""")
    assert lage["im_berater"], "Die Bestätigungen stehen wieder ausserhalb des KI-Fensters."
    assert lage["nach_verlauf"] and lage["vor_eingabe"], (
        f"Sie stehen nicht zwischen Verlauf und Eingabefeld: {lage}")


def test_im_ki_weg_ist_kein_stueck_fragebogen_im_bild(seite_factory):
    """Einzeln benannt, was im Screenshot vom 2026-08-25 noch dastand — die erste Fassung dieses
    Tests liess Ring und Spanne ausdrücklich stehen („der Ring bleibt"). Auch das war zu zaghaft.

    Der Fortschrittsbalken war dabei nicht nur überflüssig, sondern schief: „0 von 301" stand da,
    während die KI gerade vier Vorschläge gemacht hatte."""
    page = seite_factory("ki")
    sicht = page.evaluate("""() => {
      const da = s => { const e = document.querySelector(s);
        return !!(e && e.getBoundingClientRect().width > 0
                  && getComputedStyle(e).display !== 'none'); };
      return {balken: da('.fortschritt'), balkentext: da('.fortschritt-text'),
              import: da('.vorjahr-box'), ring: da('#ring'), spanne: da('#spanne'),
              frage: da('#wegpunkt'), bestaetigen: da('#bestaetigen'), belegt: da('.belegt'),
              panel: da('#berater')};
    }""")
    steht_da = [k for k, v in sicht.items() if v and k != "panel"]
    assert steht_da == [], f"Vom Fragebogen steht noch im Bild: {steht_da}"
    assert sicht["panel"], "Und das Panel selbst ist auch weg — dann ist gar nichts mehr da."


def test_auf_dem_fragebogen_weg_bleibt_der_kopf_vollstaendig(seite_factory):
    """Die Gegenrichtung, ohne die „immer ausblenden" eine bestandene Lösung wäre. Wer den
    Fragebogen gewählt hat, braucht Fortschritt und Import-Wege."""
    page = seite_factory("fragebogen")
    sicht = page.evaluate("""() => {
      const da = s => { const e = document.querySelector(s);
        return !!(e && e.getBoundingClientRect().width > 0 && getComputedStyle(e).display !== 'none'); };
      return {balken: da('.fortschritt'), import: da('.vorjahr-box')};
    }""")
    assert sicht["balken"] and sicht["import"], (
        f"Auf dem Fragebogen-Weg fehlt der Kopf: {sicht}")


def test_der_verlauf_tritt_zurueck_solange_der_wizard_steht(seite_factory):
    """Beides zusammen muss ins Bild passen. Behielte der Verlauf seine volle Höhe (68vh im
    KI-Fokus), stünde die Frage, die gerade beantwortet werden soll, unterhalb des Fensterrands —
    dann wäre der Wizard zwar integriert, aber unsichtbar.

    Gemessen wird die tatsächliche Höhe des Verlaufs, nicht die Klasse: `.rf-laeuft` liesse sich
    setzen, ohne dass eine Regel greift (genau so ging das verschachtelte minmax() am 2026-08-23
    still ins Leere)."""
    page = seite_factory("ki")
    # Der Verlauf muss LANG sein, sonst misst der Test nichts: ein leerer Verlauf ist 65px hoch,
    # und ob der Deckel bei 68vh oder 30vh steht, ändert daran nichts. Genau das war der erste
    # Anlauf dieses Tests — er verglich 65px gegen 94px und meldete einen Fehler, der keiner war.
    page.evaluate("""() => {
      const b = document.getElementById('chat-body');
      for (let i = 0; i < 40; i++) {
        const p = document.createElement('p');
        p.textContent = 'Verlaufszeile ' + i + ' — Fülltext, damit der Verlauf seinen Deckel erreicht.';
        b.appendChild(p);
      }
    }""")
    vorher = page.evaluate(
        "parseFloat(getComputedStyle(document.getElementById('chat-body')).maxHeight)")
    _stub(page, _antwort(rueckfragen=[{"frage": "Wie viele Tage?",
                                       "feld_id": "ep_arbeitstage", "aussage": 0}]))
    _senden(page)
    page.wait_for_selector("#rueckfragen:not([hidden])", timeout=8000)
    page.wait_for_timeout(300)

    m = page.evaluate("""() => {
      const cb = document.getElementById('chat-body');
      const rf = document.getElementById('rueckfragen').getBoundingClientRect();
      return {deckel: parseFloat(getComputedStyle(cb).maxHeight),
              verlauf: cb.getBoundingClientRect().height,
              rf_unten: rf.bottom, fenster: window.innerHeight};
    }""")
    assert m["deckel"] < vorher, (
        f"Der Deckel des Verlaufs ist beim Wizard nicht gesunken ({vorher:.0f}px → "
        f"{m['deckel']:.0f}px) — die Regel .berater.rf-laeuft greift nicht.")
    assert m["verlauf"] <= m["deckel"] + 1, (
        f"Der Verlauf ist höher als sein eigener Deckel ({m['verlauf']:.0f} > {m['deckel']:.0f}).")
    assert m["rf_unten"] <= m["fenster"], (
        f"Der Wizard endet unterhalb des Fensterrands ({m['rf_unten']:.0f}px bei "
        f"{m['fenster']:.0f}px) — der Nutzer sieht die Knöpfe nicht, ohne zu scrollen.")


def test_kein_konsolenfehler(seite_factory):
    page = seite_factory("ki")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"
