"""Die Zwischenstufe des dreistufigen KI-Dialogs, sichtbar in der Oberfläche (2026-08-21).

ANLASS, gemessen: Julius schrieb 222 Zeichen mit FÜNF Fakten (ledig, gesetzlich krankenversichert,
Jahresbrutto, seit Juli arbeitslos, Gesundheitsausgaben). Zurück kamen ZWEI Vorschläge. Über die
drei übrigen Fakten stand nirgends ein Wort — der Nutzer merkte nur, dass etwas fehlt, und hatte
keinen Anhaltspunkt, was. Ursache war nicht ein Filter, sondern die Größe: der Prompt trug 301
Feldbeschreibungen, 87.000 von 90.000 Zeichen waren Katalog.

Der Umbau auf drei Stufen (produkt/haut/api_llm.py) liefert der Oberfläche eine ZWISCHENSTUFE, und
die ist zeigbar. Hier wird geprüft, dass sie auch wirklich gezeigt wird:

  1. Jede Aussage erscheint mit ihrem wörtlichen Beleg-Zitat. Ohne den Beleg ist eine Aussage eine
     Behauptung der KI über den Satz des Nutzers, die er nur glauben kann.
  2. Eine Aussage ohne Feldzuordnung sieht SICHTBAR anders aus — und zwar nicht bloß in der Farbe,
     sondern in Form (gestrichelter Rand) und Text (eigene Marke, eigener Hinweissatz).
  3. Eine Rückfrage ist beantwortbar, über DENSELBEN Kanal wie jeder andere Satz, und danach läuft
     der Ablauf weiter. Anlass: aus „bis Juni 100k p.a." schloss die KI auf ein Jahresbrutto von
     100.000 EUR und war sich sicher — 70.000 EUR zu viel.
  4. Fehlen die neuen Felder ganz, verhält sich alles wie vorher. Der Endpunkt liefert sie noch
     nicht; die Oberfläche darf davon nicht abhängen.
  5. Rückfrage UND Vorschlag am selben Feld ist laut Auflage ein Fehler im Backend. Die Oberfläche
     hält den Wert dann zurück und SAGT das — die Rückfrage gewinnt.

KEIN LLM: /chat wird per page.route abgefangen. Der Server sieht diesen Aufruf nie, es fließt kein
Key und kein Cent. Im Handler wird nicht geschlafen — genau das (und nur das) würde Playwrights
sync-Schleife blockieren, s. die Warnung in tests/test_ui_chat_wartezustand.py.
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


def _antwort(antwort="", vorschlaege=None, aussagen=None, rueckfragen=None, neue_felder=True):
    """Eine Server-Antwort in der Form, die api.chat() zurückgibt.

    `neue_felder=False` lässt `aussagen`/`rueckfragen` WEG — das ist der heutige Stand des
    Endpunkts und damit der Fall, den Punkt 4 prüft.
    """
    vs = []
    for v in (vorschlaege or []):
        vs.append({"feld_id": v["feld_id"], "event_id": v.get("event_id", "EV-STUB"),
                   "wert": v["wert"], "beleg": v.get("beleg", ""),
                   "frage": v.get("frage", "Frage zu " + v["feld_id"]),
                   "typ": v.get("typ", "cent"), "frage_invertiert": False,
                   "einheit": v.get("einheit"), "enum_labels": None,
                   "aussage": v.get("aussage", 0)})
    body = {"vorschlaege": vs, "abgelehnt": [], "abgelehnt_gruende": {}, "konflikte": [],
            "antwort": antwort, "unsicher": False,
            "hinweis": "Vorschläge erfasst — bitte jeden einzeln bestätigen (die KI setzt nichts)."}
    if neue_felder:
        body["aussagen"] = aussagen or []
        body["rueckfragen"] = rueckfragen or []
    return body


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
    """Seite mit laufendem Fall. Sammelt nebenbei Konsolenfehler ein — Punkt 4 behauptet
    ausdrücklich „kein Fehler in der Konsole", und das ist nur mit Mitschnitt nachprüfbar.

    EINE Meldung wird übergangen, und zwar namentlich: initAuth() fragt beim Start bewusst einen
    nicht existierenden Fall ab (`/fall/auth-sonde-taxgraph/stand`), um herauszufinden, ob diese
    Instanz eine Anmeldung verlangt. Der 404 ist die Antwort, nicht der Fehler. Ausgeblendet wird
    deshalb genau diese URL — jeder andere Ressourcenfehler lässt den Test weiter rot werden.
    """
    gestartet = []
    SONDE = "/fall/auth-sonde-taxgraph/stand"

    def _mach(breite=360, hoehe=780):
        p = sync_playwright().start()
        gestartet.append(p)
        browser = p.chromium.launch()
        ctx = browser.new_context(viewport={"width": breite, "height": hoehe})
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
        # P0b (2026-08-23): zwischen Fallart und Fluss liegt jetzt die Wegwahl (Fragebogen / erst KI).
        page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
        page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)
        return page

    try:
        yield _mach
    finally:
        for p in gestartet:
            p.stop()


@pytest.fixture
def seite(seite_factory):
    return seite_factory()


def _stub(page, *antworten):
    """/chat abfangen. Mehrere Antworten werden der Reihe nach ausgeliefert — die letzte bleibt
    stehen, damit ein Folgeaufruf (Punkt 3: „danach läuft es weiter") eine Antwort bekommt."""
    rest = list(antworten)

    def _handler(route):
        daten = rest.pop(0) if len(rest) > 1 else rest[0]
        route.fulfill(status=200, content_type="application/json", body=json.dumps(daten))

    page.route("**/chat", _handler)


def _senden(page, text="ich bin ledig und hatte 620 euro arztkosten"):
    """Absenden und warten, bis der Aufruf durch ist.

    Bewusst `wait_for_selector` mit CSS statt `wait_for_function`: letzteres wertet eine
    Zeichenkette als JavaScript aus, was die scharfe CSP der Seite (script-src 'self') verbietet —
    man müsste sie fürs Werkzeug abschalten (bypass_csp), wie es
    tests/test_ui_chat_wartezustand.py für seine Zeitfenster-Messungen tun muss. Hier geht es
    ohne, also läuft diese Datei unter der Richtlinie, die auch im Betrieb gilt.

    Kein Wettlauf: chatSperren(false) im finally und die Auswertung darunter laufen in DEMSELBEN
    synchronen Block (das erste `await` steht erst hinter zeigeVerstanden). Ist der Knopf wieder
    frei, steht die Anzeige bereits vollständig im DOM.
    """
    page.fill("#chat-text", text)
    page.click("#chat-send")
    page.wait_for_selector("#chat-send:not([disabled])", timeout=5000)


def _zeilen(page):
    return page.evaluate("""() => [...document.querySelectorAll('#chat-body .a-zeile')].map(li => ({
        status: li.dataset.status,
        klasse: li.className,
        text: (li.querySelector('.a-text') || {}).textContent || '',
        marke: (li.querySelector('.a-marke') || {}).textContent || '',
        beleg: (li.querySelector('.a-beleg') || {}).textContent || '',
        frage: (li.querySelector('.a-frage') || {}).textContent || '',
    }))""")


# --- 1) Die Aussagen erscheinen, mit Beleg ---------------------------------------------------

def test_aussagen_erscheinen_mit_ihrem_beleg(seite):
    """Punkt 1: was die KI aus dem Satz gelesen hat, steht auf dem Schirm — jede Aussage mit dem
    wörtlichen Satzteil, auf den sie sich stützt. Erst der Beleg macht sie nachprüfbar."""
    page = seite
    _stub(page, _antwort(
        aussagen=[
            {"text": "Nutzer ist ledig", "beleg": "ich bin ledig", "status": "vorschlag"},
            {"text": "Arztkosten 620 EUR", "beleg": "620 euro arztkosten", "status": "vorschlag"},
        ],
        vorschlaege=[{"feld_id": "aussergewoehnliche_belastungen", "wert": 62000,
                      "beleg": "620 euro arztkosten", "aussage": 1}]))
    _senden(page)

    z = _zeilen(page)
    assert len(z) == 2, f"Nicht beide Aussagen angezeigt: {z}"
    assert z[0]["text"] == "Nutzer ist ledig"
    assert z[1]["text"] == "Arztkosten 620 EUR"
    for zeile in z:
        assert "weil du sagtest" in zeile["beleg"], (
            f"Aussage ohne Beleg-Zitat: {zeile} — dann ist sie eine Behauptung der KI, die der "
            "Nutzer nur glauben kann.")
    assert "ich bin ledig" in z[0]["beleg"] and "620 euro arztkosten" in z[1]["beleg"], (
        f"Der Beleg gehört zur falschen Aussage: {z}")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_aussagen_stehen_vor_der_antwort(seite):
    """Reihenfolge ist hier Inhalt: erst was gelesen wurde, dann was daraus folgt. Stünde die
    Zwischenstufe unter der Antwort, läse man erst das Ergebnis und prüfte die Lesart nie."""
    page = seite
    _stub(page, _antwort(
        antwort="Arztkosten zählen zu den außergewöhnlichen Belastungen.",
        aussagen=[{"text": "Arztkosten 620 EUR", "beleg": "620 euro", "status": "kein_feld"}]))
    _senden(page)

    reihenfolge = page.evaluate("""() => {
        const box = document.querySelector('#chat-body .chat-aussagen');
        const ant = document.querySelector('#chat-body .chat-antwort');
        return (box && ant) ? (box.compareDocumentPosition(ant) & Node.DOCUMENT_POSITION_FOLLOWING) > 0 : null;
    }""")
    assert reihenfolge is True, "Die gelesenen Aussagen stehen nicht vor der Antwort."


# --- 2) Was NICHT zugeordnet werden konnte ---------------------------------------------------

def test_eine_aussage_ohne_feld_ist_sichtbar_als_solche(seite):
    """Punkt 2, der eigentliche Gewinn: heute verschwindet so etwas ersatzlos. Geprüft wird nicht
    die Klasse (die ließe sich vergeben, ohne dass man etwas sieht), sondern der GEMESSENE
    Unterschied im Bild — Rahmenform und eigener Text."""
    page = seite
    _stub(page, _antwort(
        aussagen=[
            {"text": "Nutzer ist ledig", "beleg": "ich bin ledig", "status": "vorschlag"},
            {"text": "Seit Juli arbeitslos", "beleg": "seit juli arbeitslos",
             "status": "kein_feld"},
        ],
        vorschlaege=[{"feld_id": "veranlagungsart", "wert": "einzel", "typ": "enum",
                      "beleg": "ich bin ledig", "aussage": 0}]))
    _senden(page)

    z = _zeilen(page)
    assert len(z) == 2, f"Die nicht zugeordnete Aussage fehlt ganz: {z}"
    ohne = [x for x in z if x["status"] == "kein_feld"]
    assert len(ohne) == 1, f"kein_feld-Zeile nicht als solche ausgezeichnet: {z}"
    assert ohne[0]["text"] == "Seit Juli arbeitslos"
    assert ohne[0]["marke"] and ohne[0]["marke"] != z[0]["marke"], (
        f"Die Marke unterscheidet die Zeilen nicht: {[x['marke'] for x in z]}")

    stile = page.evaluate("""() => [...document.querySelectorAll('#chat-body .a-zeile')].map(li => {
        const s = getComputedStyle(li);
        return {status: li.dataset.status, art: s.borderLeftStyle, farbe: s.borderLeftColor};
    })""")
    ohne_stil = [s for s in stile if s["status"] == "kein_feld"][0]
    mit_stil = [s for s in stile if s["status"] == "vorschlag"][0]
    assert ohne_stil["art"] == "dashed" and mit_stil["art"] == "solid", (
        f"Die Rahmenform unterscheidet die beiden nicht ({ohne_stil} vs {mit_stil}) — Farbe allein "
        "trägt die Unterscheidung für niemanden mit Farbsehschwäche.")
    assert ohne_stil["farbe"] != mit_stil["farbe"], f"Auch die Farbe ist gleich: {stile}"

    hinweis = page.evaluate(
        "(document.querySelector('#chat-body .a-kein_feld .a-hinweis') || {}).textContent || ''")
    assert "kein passendes Feld" in hinweis, (
        f"Kein Hinweis, was der Nutzer jetzt tun kann: {hinweis!r}")


# --- 3) Die Rückfrage ------------------------------------------------------------------------

def _rueckfrage_antwort(**kw):
    return _antwort(
        aussagen=[{"text": "Jahresbrutto 100.000 EUR", "beleg": "bis Juni 100k p.a.",
                   "status": "rueckfrage"}],
        rueckfragen=[{"frage": "Meinst du 100.000 für das ganze Jahr oder anteilig bis Juni?",
                      "aussage": 0, "feld_id": "bruttoarbeitslohn"}], **kw)


def test_rueckfrage_erscheint_an_ihrer_aussage(seite):
    """Die Rückfrage steht bei der Aussage, um die es geht — sonst muss der Nutzer raten, worauf
    sie sich bezieht, und das war der Ausgangsfehler."""
    page = seite
    _stub(page, _rueckfrage_antwort())
    _senden(page, "bis Juni 100k p.a.")

    z = _zeilen(page)
    assert len(z) == 1, f"Genau eine Aussage erwartet: {z}"
    assert z[0]["status"] == "rueckfrage"
    assert "anteilig bis Juni" in z[0]["frage"], f"Rückfrage nicht an ihrer Aussage: {z}"
    assert "bis Juni 100k p.a." in z[0]["beleg"], "Auch die Rückfrage-Aussage braucht ihren Beleg."
    assert page.query_selector("#chat-body .a-antworten") is not None, (
        "Eine Rückfrage ohne Antwortmöglichkeit ist keine Rückfrage.")


def test_rueckfrage_ist_beantwortbar_und_der_ablauf_laeuft_weiter(seite):
    """Punkt 3, der Nutzerpfad: klicken, antworten, absenden — und danach geht es normal weiter.
    Der Chat IST der Kanal; es entsteht kein zweiter Weg und kein eigener Zustand."""
    page = seite
    _stub(page, _rueckfrage_antwort(),
          _antwort(antwort="Danke — dann rechne ich mit dem anteiligen Betrag."))
    _senden(page, "bis Juni 100k p.a.")

    page.click("#chat-body .a-antworten")
    vorspann = page.input_value("#chat-text")
    assert vorspann.startswith("Zu deiner Rückfrage"), (
        f"Der Knopf füllt das Eingabefeld nicht: {vorspann!r}")
    assert "anteilig bis Juni" in vorspann, (
        "Der Bezug fehlt im abgeschickten Text — die nächste Runde müsste ihn erraten.")
    assert page.evaluate("document.activeElement.id") == "chat-text", (
        "Der Fokus liegt nicht im Eingabefeld — der Nutzer müsste erst hinklicken.")

    # Und jetzt der eigentliche Beweis: der Ablauf läuft weiter.
    page.fill("#chat-text", vorspann + "anteilig, ich habe nur bis Juni gearbeitet")
    page.click("#chat-send")
    page.wait_for_selector("#chat-body .chat-antwort", timeout=5000)
    assert "anteiligen Betrag" in page.text_content("#chat-body .chat-antwort")
    g = page.evaluate("""() => ({
        wegpunkt: document.getElementById('wegpunkt').hasAttribute('inert'),
        verstanden: document.getElementById('verstanden').hasAttribute('inert'),
        belegt: document.getElementById('belegt-liste').hasAttribute('inert'),
        importbox: document.querySelector('.vorjahr-box').hasAttribute('inert'),
        chattext: document.getElementById('chat-text').disabled,
        sendknopf: document.getElementById('chat-send').disabled,
    })""")
    assert not any(g.values()), f"Nach der Rückfrage-Runde sind noch Sperren gesetzt: {g}"
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_antworten_wirft_schon_getippten_text_nicht_weg(seite):
    """Wer schon angefangen hat zu schreiben, verliert das nicht durch einen Klick auf
    „Antworten". Das Feld ist der einzige Kanal — was dort steht, ist Nutzerarbeit."""
    page = seite
    _stub(page, _rueckfrage_antwort())
    _senden(page, "bis Juni 100k p.a.")

    page.fill("#chat-text", "ausserdem habe ich eine Brille gekauft")
    page.click("#chat-body .a-antworten")
    wert = page.input_value("#chat-text")
    assert "Brille" in wert, f"Getippter Text überschrieben: {wert!r}"
    assert "Zu deiner Rückfrage" in wert, f"Vorspann fehlt: {wert!r}"


def test_eine_rueckfrage_ohne_zuordnung_verschwindet_nicht(seite):
    """Zeigt der Index ins Leere, darf die Rückfrage nicht still unter den Tisch fallen — sie ist
    der eine Ort, an dem die KI zugibt, dass sie sonst raten müsste."""
    page = seite
    _stub(page, _antwort(
        aussagen=[{"text": "Nutzer ist ledig", "beleg": "ich bin ledig", "status": "kein_feld"}],
        rueckfragen=[{"frage": "Ganzes Jahr oder anteilig?", "aussage": 7,
                      "feld_id": "bruttoarbeitslohn"}]))
    _senden(page)

    fragen = page.evaluate(
        "[...document.querySelectorAll('#chat-body .a-frage')].map(e => e.textContent)")
    assert fragen == ["Ganzes Jahr oder anteilig?"], (
        f"Die heimatlose Rückfrage ist verschwunden: {fragen}")
    assert page.query_selector("#chat-body .a-antworten") is not None, (
        "Sie ist zwar da, aber nicht beantwortbar.")


def test_rueckfrage_und_vorschlag_am_selben_feld_haelt_den_wert_zurueck(seite):
    """Auflage: zu einer Rückfrage gibt es KEINEN Vorschlag am selben Feld. Kommt beides, hat die
    KI gleichzeitig gefragt und geraten — der Vorschlag IST die Vermutung, nach der sie fragt.
    Genau dieser Fall kostete am 2026-08-21 fast 70.000 EUR zu viel Bruttolohn. Also: die
    Rückfrage gewinnt, der Wert wird zurückgehalten, und dass er zurückgehalten wurde, steht da."""
    page = seite
    _stub(page, _rueckfrage_antwort(
        vorschlaege=[{"feld_id": "bruttoarbeitslohn", "wert": 10000000,
                      "beleg": "100k p.a.", "aussage": 0}]))
    _senden(page, "bis Juni 100k p.a.")

    assert page.is_hidden("#verstanden"), (
        "Der Vorschlag wurde trotz offener Rückfrage zur Bestätigung angeboten — ein Klick darauf "
        "schriebe genau den Wert, nach dem die KI noch fragt.")
    assert page.query_selector("#chat-body .a-antworten") is not None, "Rückfrage fehlt."
    text = page.text_content("#chat-body")
    assert "zurückgehalten" in text, (
        "Der Wert verschwand still — das ist derselbe Fehler in die andere Richtung.")


# --- 4) Ohne die neuen Felder: alles wie heute ------------------------------------------------

def test_ohne_aussagen_verhaelt_sich_alles_wie_heute(seite):
    """Punkt 4 und keine Bequemlichkeit: der Endpunkt liefert die neuen Felder noch nicht, der
    Umbau kann scheitern oder später kommen. Fehlen sie, darf keine leere Überschrift stehen und
    kein Fehler in der Konsole landen — und die Vorschläge müssen ganz normal ankommen."""
    page = seite
    _stub(page, _antwort(antwort="Das kommt in Zeile 61 der Anlage.",
                         vorschlaege=[{"feld_id": "aussergewoehnliche_belastungen",
                                       "wert": 62000, "beleg": "620 euro"}],
                         neue_felder=False))
    _senden(page)

    assert page.query_selector("#chat-body .chat-aussagen") is None, (
        "Ohne `aussagen` steht trotzdem ein (leerer) Aussagen-Kasten da.")
    assert "Das kommt in Zeile 61" in page.text_content("#chat-body .chat-antwort")
    page.wait_for_selector("#verstanden:not([hidden])", timeout=5000)
    assert page.query_selector("#verstanden-liste .v-ok") is not None, (
        "Der Vorschlag kam nicht mehr zur Bestätigung durch.")
    assert not page.fehler, f"Konsolenfehler ohne die neuen Felder: {page.fehler}"


def test_ohne_aussagen_bleibt_die_leermeldung_stehen(seite):
    """Gegenprobe zur einen Zeile, die in chatSenden() geändert wurde: liefert die KI weder Wert
    noch Antwort noch Rückfrage, muss die alte Meldung weiter erscheinen."""
    page = seite
    _stub(page, _antwort(neue_felder=False))
    _senden(page)
    assert "weder einen Wert" in page.text_content("#chat-body"), (
        "Die Leermeldung ist mit der neuen Bedingung ganz verschwunden.")


def test_bei_offener_rueckfrage_keine_leermeldung(seite):
    """Die Kehrseite: steht eine Rückfrage auf dem Schirm, hat die KI sehr wohl etwas erkannt.
    „Weder einen Wert noch eine Frage" schickte den Nutzer dort zum Umformulieren statt zum
    Antworten."""
    page = seite
    _stub(page, _rueckfrage_antwort())
    _senden(page, "bis Juni 100k p.a.")
    assert "weder einen Wert" not in page.text_content("#chat-body"), (
        "Neben der Rückfrage steht, die KI habe keine Frage erkannt.")


def test_antworten_waehrend_eines_laufenden_aufrufs_frisst_den_vorspann_nicht(seite):
    """Der Knopf steht im Chat-Verlauf, und der ist während eines laufenden Aufrufs bewusst NICHT
    gesperrt (chatSperren fasst nur #wegpunkt, #verstanden, #belegt-liste und .vorjahr-box an) —
    man soll weiterlesen können. Ein Klick auf „Antworten“ dürfte dort aber Text in ein Feld
    schreiben, das chatSenden() am Ende leert: der Vorspann wäre still verschwunden, und der Nutzer
    hätte auf einen Knopf gedrückt, der nichts tat.

    Hier hält die SEITE den Aufruf offen (window.fetch-Stub wie in
    tests/test_ui_chat_wartezustand.py) — page.route kann ein Zeitfenster nicht offen halten, ohne
    Playwrights eigene Schleife zu blockieren."""
    page = seite
    page.evaluate("""() => {
      window.__CHAT_OFFEN = null;
      window.__ECHT = window.fetch;
      window.fetch = (url, opt) => String(url).endsWith('/chat')
        ? new Promise((res, rej) => { window.__CHAT_OFFEN = { res, rej }; })
        : window.__ECHT(url, opt);
    }""")
    aufloesen = """(daten) => { const o = window.__CHAT_OFFEN;
        setTimeout(() => o.res(new Response(JSON.stringify(daten),
            { status: 200, headers: { 'Content-Type': 'application/json' } })), 0); }"""

    page.fill("#chat-text", "bis Juni 100k p.a.")
    page.click("#chat-send")
    page.wait_for_selector("#chat-send[disabled]", timeout=5000)
    page.evaluate(aufloesen, _rueckfrage_antwort())
    page.wait_for_selector("#chat-send:not([disabled])", timeout=5000)

    page.fill("#chat-text", "und eine Brille")
    page.click("#chat-send")
    page.wait_for_selector("#chat-send[disabled]", timeout=5000)   # zweiter Aufruf läuft
    page.evaluate("document.querySelector('#chat-body .a-antworten').click()")
    assert page.input_value("#chat-text") == "und eine Brille", (
        "Der Vorspann landete in einem Feld, das gleich geleert wird — der Klick wäre folgenlos "
        "gewesen, ohne dass der Nutzer das merkt.")

    page.evaluate(aufloesen, _antwort(antwort="Verstanden."))
    page.wait_for_selector("#chat-send:not([disabled])", timeout=5000)
    assert page.input_value("#chat-text") == "", "Das Feld wurde nach dem Aufruf nicht geleert."
    # Und danach greift der Knopf wieder — die Sperre war ein Zustand auf Zeit, keine Sackgasse.
    page.click("#chat-body .a-antworten")
    assert page.input_value("#chat-text").startswith("Zu deiner Rückfrage"), (
        "Nach dem Aufruf füllt „Antworten“ das Feld nicht mehr — der Knopf ist tot.")


# --- 5) Auflagen: Sperre und schmaler Schirm --------------------------------------------------

def test_die_chat_sperre_greift_auch_mit_der_neuen_anzeige(seite):
    """Auflage 2, geprüft statt angenommen: die Sperre hängt an #wegpunkt, #verstanden,
    #belegt-liste und .vorjahr-box. Die neue Anzeige hängt bewusst im Chat-Verlauf — griffe sie in
    einen dieser Bereiche, sperrte ein laufender Aufruf sein eigenes Eingabefeld mit."""
    page = seite
    _stub(page, _rueckfrage_antwort())
    _senden(page, "bis Juni 100k p.a.")

    drin = page.evaluate("""() => {
        const box = document.querySelector('#chat-body .chat-aussagen');
        if (!box) return 'kein Kasten';
        for (const id of ['wegpunkt', 'verstanden', 'belegt-liste']) {
            if (document.getElementById(id).contains(box)) return id;
        }
        if (document.querySelector('.vorjahr-box').contains(box)) return 'vorjahr-box';
        return document.getElementById('berater').contains(box) ? 'berater' : 'sonstwo';
    }""")
    assert drin == "berater", f"Die Aussagen-Anzeige liegt in {drin!r} statt im Berater-Panel."
    assert page.evaluate("!document.getElementById('berater').closest('[inert]')"), (
        "Das Berater-Panel liegt in einem gesperrten Bereich.")


def test_die_anzeige_passt_auf_den_schmalen_schirm(seite_factory):
    """Auflage 4: 360px ist die Messlatte (tests/test_ui_responsive.py). Lange Modelltexte dürfen
    die Anzeige nicht seitwärts schieben.

    NACHGEMESSEN (Mutationsprobe): `document.scrollWidth` allein prüft das NICHT. #chat-body trägt
    `overflow-y:auto`, und damit rechnet der Browser auch overflow-x auf `auto` hoch — überlaufender
    Text erzeugt dort einen eigenen waagerechten Scrollbalken und erreicht das Dokument nie. Die
    Seite bliebe also 360px breit, während der Nutzer die Aussage nur zur Hälfte sieht und
    seitwärts wischen muss. Gemessen wird deshalb der Überlauf IM Kasten (scrollWidth gegen
    clientWidth), und der Prüftext enthält ein unteilbares Wort — genau daran, und nur daran,
    hängt `overflow-wrap:anywhere`. Modelltext liefert so etwas laufend (IBANs, Feld-IDs)."""
    page = seite_factory(breite=360, hoehe=640)
    lang = "Der Nutzer erzielte Einkuenfte aus nichtselbstaendiger Arbeit, gebucht auf " \
           "DE00500105175407324931unterkontoLohnUndGehaltJanuarBisJuni2025Sammelbuchung"
    _stub(page, _antwort(
        aussagen=[{"text": lang, "beleg": lang, "status": "kein_feld"}],
        rueckfragen=[{"frage": lang, "aussage": 0, "feld_id": "bruttoarbeitslohn"}]))
    _senden(page)

    assert page.evaluate("document.documentElement.scrollWidth") <= 360, (
        "Horizontales Scrollen bei 360px durch die neue Anzeige.")
    ueberlauf = page.evaluate("""() => ['#chat-body', '#chat-body .chat-aussagen',
                                        '#chat-body .a-zeile', '#chat-body .a-rueckfrage-box']
        .map(s => { const e = document.querySelector(s);
                    return {sel: s, ueber: e ? e.scrollWidth - e.clientWidth : -1}; })
        .filter(x => x.ueber > 0)""")
    assert ueberlauf == [], (
        f"Der Text läuft aus seinem Kasten heraus: {ueberlauf} — der Nutzer sieht die Aussage nur "
        "zur Hälfte und muss seitwärts wischen, ohne dass die Seite es anzeigt.")
    kasten = page.query_selector("#chat-body .chat-aussagen").bounding_box()
    assert kasten["x"] >= 0 and kasten["x"] + kasten["width"] <= 360, (
        f"Der Aussagen-Kasten ragt aus dem Bild: {kasten}")
    assert page.query_selector("#chat-body .a-antworten").bounding_box()["height"] >= 44, (
        "Tap-Ziel „Antworten“ unter 44px.")
