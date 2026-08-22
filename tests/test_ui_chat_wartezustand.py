"""Wartezustand des KI-Chats — Befund B1 aus einem echten Nutzerlauf (Julius 2026-08-20).

Wörtlich: „habe einen text an die ki gefüttert. es sollte in der zwischenzeit ein spinner o.ä.
kommen. ansonsten kann der nutzer etwas klicken und torpediert u.U. die KI ausgabe"

Drei getrennte Lücken in chatSenden() (produkt/haut/static/app.js), hier alle drei als
NUTZERPFAD geprüft — nicht als Funktionsaufruf:

  1. Die Wartemeldung war als Zustand unsichtbar. „Die KI liest mit …“ entstand über
     beraterZeile() — dieselbe Funktion wie für jede Antwort — und stand danach als unbewegter
     Absatz im Antwortverlauf. Jetzt: chatWarteZeile() mit CSS-Animation (.chat-warte-punkte),
     unter prefers-reduced-motion mit sichtbarer Ersatz-Kennzeichnung (.chat-warte-still).

  2. Gesperrt war NUR der Absendeknopf. Der Fragebereich, die Verstanden-Liste, die Belegt-Liste
     und die Import-Box blieben bedienbar — und aus jedem dieser Bereiche heraus ändert sich
     `AKTUELL`. Jetzt sperrt chatSperren() sie per `inert` für die Dauer des Aufrufs.

  3. Kein try/finally. Die Freigabe stand allein am Ende des Rumpfs; jede Ausnahme davor hätte
     Knopf und Wartemeldung dauerhaft stehen lassen. Jetzt steht die Freigabe im finally — und
     das ist für die NEUEN Sperren aus (2) zwingend, denn die umfassen die halbe Oberfläche.

MESSERGEBNIS zu (2), 2026-08-20 nachgemessen und hier unten als Test festgehalten
(test_vorschlag_landet_unter_seiner_eigenen_feld_id): der Wechsel von `AKTUELL` während des
Aufrufs ist KEIN Datenfehler. `feld_id` aus der Oberfläche erreicht serverseitig nur
_erklaer_kontext() (Prompt-Kontext für die Antwort-Hälfte); geschrieben wird auf der feld_id, die
im Vorschlag selbst steht, und die Oberfläche bestätigt ebenfalls über `v.feld_id`. Falsch
zuordnen kann sich nur die ANTWORT — deshalb kennzeichnet sie chatSenden() jetzt, statt sie still
unter eine andere Frage zu hängen.

KEIN LLM: /chat wird im Browser abgefangen (window.fetch-Stub, den der Test offen hält und
gezielt auflöst oder scheitern lässt). Der Server sieht diesen Aufruf nie, es fließt kein Key
und kein Cent. Der Stub ist damit auch das einzige Mittel, das Zeitfenster „während des Aufrufs“
überhaupt beobachtbar zu machen.
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
    from playwright.sync_api import TimeoutError as PWTimeout
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None
    PWTimeout = Exception

pytestmark = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")


# --- Der fetch-Stub -------------------------------------------------------------------------
# Warum nicht page.route(): dessen Handler läuft in der sync-API auf Playwrights eigener
# Schleife. Ein `time.sleep()` darin blockiert sie — alle „während des Aufrufs“-Abfragen liefen
# dann erst NACH der Antwort und maßen das Gegenteil dessen, was sie behaupteten (2026-08-20 so
# passiert). Hier hält die SEITE den Aufruf offen: window.fetch gibt für /chat ein Promise
# zurück, dessen res/rej der Test in der Hand behält.
_STUB = """() => {
  window.__CHAT_OFFEN = null;
  if (window.__FETCH_ECHT) return;          // nur einmal einhängen
  window.__FETCH_ECHT = window.fetch;
  window.fetch = (url, opt) => {
    if (String(url).endsWith('/chat')) {
      return new Promise((res, rej) => { window.__CHAT_OFFEN = { res, rej }; });
    }
    return window.__FETCH_ECHT(url, opt);
  };
}"""

# setTimeout, nicht direkt auflösen: sonst läuft die gesamte Antwort-Auswertung noch als
# Microtask INNERHALB von page.evaluate() — eine Ausnahme darin käme dann als Fehler des
# evaluate-Aufrufs zurück statt in der Seite zu landen, und der Test misst sein eigenes Werkzeug.
# Eine echte Netzantwort trifft ebenfalls nie im selben Task ein.
_AUFLOESEN = """(daten) => {
  const offen = window.__CHAT_OFFEN;
  setTimeout(() => offen.res(new Response(JSON.stringify(daten),
      { status: 200, headers: { 'Content-Type': 'application/json' } })), 0);
}"""

# Netzabbruch, so wie ihn der Browser meldet: fetch() lehnt mit TypeError ab (Server weg,
# Tunnel zu, WLAN aus). Genau der Fall aus Punkt 3.
_ABBRECHEN = """() => {
  const offen = window.__CHAT_OFFEN;
  setTimeout(() => offen.rej(new TypeError('Failed to fetch')), 0);
}"""


def _antwort(feld_id=None, antwort="", vorschlaege=None):
    """Eine Server-Antwort in der Form, die api.chat() tatsächlich zurückgibt."""
    vs = []
    for v in (vorschlaege or []):
        vs.append({"feld_id": v["feld_id"], "event_id": v.get("event_id", "EV-STUB"),
                   "wert": v["wert"], "beleg": v.get("beleg", ""),
                   "frage": v.get("frage", "Frage zu " + v["feld_id"]),
                   "typ": v.get("typ", "cent"), "frage_invertiert": False,
                   "einheit": v.get("einheit"), "enum_labels": None})
    return {"vorschlaege": vs, "abgelehnt": [], "abgelehnt_gruende": {}, "konflikte": [],
            "antwort": antwort, "unsicher": False,
            "hinweis": "Vorschläge erfasst — bitte jeden einzeln bestätigen (die KI setzt nichts)."}


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
    """Seite mit laufendem Fall. `bewegung_reduziert` schaltet prefers-reduced-motion scharf."""
    gestartet = []

    def _mach(bewegung_reduziert=False):
        p = sync_playwright().start()
        gestartet.append(p)
        browser = p.chromium.launch()
        # bypass_csp: nur fürs WERKZEUG (page.wait_for_function wertet eine Zeichenkette als
        # JavaScript aus, was jede CSP ohne 'unsafe-eval' verbietet). Dass die Oberfläche unter
        # der scharfen Richtlinie läuft, prüft tests/test_idor_und_csp.py — siehe die
        # ausführliche Begründung in tests/test_ui_leerwert_stille_null.py.
        ctx = browser.new_context(viewport={"width": 360, "height": 780}, bypass_csp=True,
                                  reduced_motion="reduce" if bewegung_reduziert else "no-preference")
        page = ctx.new_page()
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        # P0b (2026-08-23): zwischen Fallart und Fluss liegt jetzt die Wegwahl (Fragebogen / erst KI).
        page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
        page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)
        page.evaluate(_STUB)
        return page

    try:
        yield _mach
    finally:
        for p in gestartet:
            p.stop()


@pytest.fixture
def seite(seite_factory):
    return seite_factory()


# --- Helfer ---------------------------------------------------------------------------------

def _rendere(page, fid):
    """Ein beliebiges Feld direkt als aktuelle Frage setzen — denselben Weg nimmt
    tests/test_ui_leerwert_stille_null.py. Alle davorliegenden Fragen durchzuklicken wäre für
    diese Prüfung reiner Aufwand."""
    page.evaluate("""async (fid) => {
        const r = await jget(`/fall/${FALL}/fragen`);
        const q = r.body.fragen.find(f => f.feld_id === fid);
        if (q) { AKTUELL = q; zeigeFrage(q, STAND); }
    }""", fid)
    assert page.evaluate("AKTUELL && AKTUELL.feld_id") == fid, f"{fid} in dieser Scheibe nicht erreichbar"


def _erstes_cent_feld(page, n=0):
    felder = page.evaluate("""async () => {
        const r = await jget(`/fall/${FALL}/fragen`);
        return r.body.fragen.filter(f => f.typ === 'cent').map(f => f.feld_id);
    }""")
    return felder[n]


def _absenden(page, text="ich hatte 620 euro solche ausgaben"):
    """Absenden und warten, bis der Aufruf wirklich offen ist (nicht: eine Zeit lang schlafen)."""
    page.evaluate("window.__CHAT_OFFEN = null")
    page.fill("#chat-text", text)
    page.click("#chat-send")
    page.wait_for_function("window.__CHAT_OFFEN !== null", timeout=5000)


def _klick_geht_durch(page, sel, timeout=1200):
    """Ein ECHTER Klick — das ist der Nutzerpfad. `is_enabled()` taugt hier nicht: ein inertes
    Element meldet sich weiter als 'enabled', obwohl kein Klick es erreicht."""
    try:
        page.click(sel, timeout=timeout)
        return True
    except PWTimeout:
        return False


def _gesperrt(page):
    return page.evaluate("""() => ({
        wegpunkt: document.getElementById('wegpunkt').hasAttribute('inert'),
        verstanden: document.getElementById('verstanden').hasAttribute('inert'),
        belegt: document.getElementById('belegt-liste').hasAttribute('inert'),
        importbox: document.querySelector('.vorjahr-box').hasAttribute('inert'),
        chattext: document.getElementById('chat-text').disabled,
        sendknopf: document.getElementById('chat-send').disabled,
    })""")


def _warte_sichtbar(page):
    return page.evaluate("!!document.querySelector('#chat-body .chat-warte')")


# --- 1) Die Laufanzeige ---------------------------------------------------------------------

def test_waehrend_des_aufrufs_laeuft_eine_sichtbare_anzeige(seite):
    """Punkt 1: während der Aufruf läuft, steht im Verlauf eine BEWEGTE Anzeige — kein
    stillstehender Absatz, der sich als Antwort liest."""
    page = seite
    _absenden(page)
    assert _warte_sichtbar(page), "Keine Laufanzeige im Chat-Verlauf, während der Aufruf läuft."
    animation = page.evaluate(
        "getComputedStyle(document.querySelector('.chat-warte-punkte i')).animationName")
    assert animation == "chat-warte-puls", (
        f"Die Laufanzeige bewegt sich nicht (animation-name={animation!r}) — genau das war der "
        "Befund: ein Zustand, der wie eine Aussage aussieht.")
    assert page.evaluate("document.querySelector('.chat-warte').getAttribute('role')") == "status", (
        "Die Laufanzeige ist für Screen-Reader nicht als Zustand ausgezeichnet (role=status fehlt).")


def test_reduzierte_bewegung_zeigt_statt_animation_eine_ruhige_kennzeichnung(seite_factory):
    """Punkt 1, Kehrseite: bei prefers-reduced-motion darf die Anzeige nicht einfach
    verschwinden. Die globale Regel `*{animation:none}` nimmt ihr die Bewegung — dann muss die
    Kennzeichnung sichtbar an deren Stelle treten."""
    page = seite_factory(bewegung_reduziert=True)
    _absenden(page)
    assert _warte_sichtbar(page), "Laufanzeige fehlt unter prefers-reduced-motion ganz."
    animation = page.evaluate(
        "getComputedStyle(document.querySelector('.chat-warte-punkte i')).animationName")
    assert animation == "none", (
        f"Unter prefers-reduced-motion läuft trotzdem eine Animation ({animation!r}).")
    marke = page.evaluate("getComputedStyle(document.querySelector('.chat-warte-still')).display")
    assert marke != "none", (
        "Ohne Bewegung UND ohne Ersatz-Kennzeichnung ist der Wartezustand unsichtbar — "
        "die Anzeige wäre dort wieder ein stillstehender Absatz.")
    assert page.evaluate("document.querySelector('.chat-warte-still').textContent") == "läuft"


# --- 2) Die Sperre --------------------------------------------------------------------------

def test_waehrend_des_aufrufs_ist_der_fragenbereich_gesperrt(seite):
    """Punkt 2, Julius' eigentlicher Punkt: nicht nur der Absendeknopf. Gesperrt sind genau die
    Bereiche, aus denen heraus sich `AKTUELL` ändern lässt."""
    page = seite
    feld = _erstes_cent_feld(page)
    _rendere(page, feld)
    _absenden(page)

    g = _gesperrt(page)
    assert g["sendknopf"], "Absendeknopf nicht gesperrt."
    assert g["wegpunkt"], "Fragebereich nicht gesperrt — der Nutzer kann die Frage wechseln."
    assert g["verstanden"], "Verstanden-Liste nicht gesperrt."
    assert g["belegt"], "Belegt-Liste nicht gesperrt — ein Klick dort ruft korrigiereBestaetigt()."
    assert g["importbox"], "Import-Box nicht gesperrt — Vorjahr/Kontoauszug rufen refresh()."
    assert g["chattext"], "Chat-Eingabefeld nicht gesperrt (sein Inhalt wird am Ende geleert)."


def test_waehrend_des_aufrufs_veraendert_ein_klick_die_aktuelle_frage_nicht(seite):
    """Punkt 2 als NUTZERPFAD: ein echter Klick auf „Bestätigen“ bzw. „Erklär mir“ kommt nicht
    durch, und `AKTUELL` steht danach unverändert da."""
    page = seite
    feld = _erstes_cent_feld(page)
    _rendere(page, feld)
    _absenden(page)

    assert not _klick_geht_durch(page, "#bestaetigen"), (
        "Der Klick auf „Bestätigen“ ging während des laufenden KI-Aufrufs durch.")
    assert not _klick_geht_durch(page, "#chat"), (
        "Der Klick auf „Erklär mir“ ging durch — er leert den Chat-Verlauf (erklaereFeld: "
        "body.innerHTML = \"\") und hätte die noch ausstehende Antwort mit weggeräumt.")
    assert page.evaluate("AKTUELL && AKTUELL.feld_id") == feld, (
        "Die aktuelle Frage hat sich während des Aufrufs geändert.")


def test_nach_der_antwort_ist_alles_wieder_frei(seite):
    """Die Sperre ist ein Zustand auf Zeit: nach der Antwort ist die Oberfläche wieder
    vollständig bedienbar und die Laufanzeige weg."""
    page = seite
    feld = _erstes_cent_feld(page)
    _rendere(page, feld)
    _absenden(page)
    page.evaluate(_AUFLOESEN, _antwort(antwort="Das kommt in Zeile 61 der Anlage."))
    page.wait_for_function("!document.getElementById('chat-send').disabled", timeout=5000)

    assert not _warte_sichtbar(page), "Die Laufanzeige steht nach der Antwort noch da."
    g = _gesperrt(page)
    assert not any(g.values()), f"Nach der Antwort sind noch Sperren gesetzt: {g}"
    assert _klick_geht_durch(page, "#bestaetigen"), (
        "„Bestätigen“ ist nach der Antwort nicht wieder anklickbar.")


# --- 3) Der fehlgeschlagene Aufruf ----------------------------------------------------------

def test_nach_einem_fehlgeschlagenen_aufruf_ist_alles_wieder_frei(seite):
    """Punkt 3 — der eigentliche Fund. Bricht der Aufruf ab (Server weg, Tunnel zu, WLAN aus),
    muss JEDE Sperre wieder fallen. Ohne das finally bliebe die halbe Oberfläche gesperrt und die
    Laufanzeige stünde für immer da; ohne Neuladen der Seite käme der Nutzer da nicht heraus."""
    page = seite
    feld = _erstes_cent_feld(page)
    _rendere(page, feld)
    _absenden(page)
    page.evaluate(_ABBRECHEN)
    page.wait_for_function("!document.getElementById('chat-send').disabled", timeout=5000)

    assert not _warte_sichtbar(page), "Die Laufanzeige steht nach dem Abbruch noch da."
    g = _gesperrt(page)
    assert not any(g.values()), f"Nach dem Abbruch sind noch Sperren gesetzt: {g}"
    assert _klick_geht_durch(page, "#bestaetigen"), (
        "Nach einem abgebrochenen KI-Aufruf ist die Oberfläche dauerhaft gesperrt.")
    assert page.evaluate(
        "[...document.querySelectorAll('#chat-body p')].some(e => e.textContent.includes('unerwartet'))"), (
        "Der Nutzer erfährt nichts über den fehlgeschlagenen Aufruf.")


def test_eine_ausnahme_in_der_auswertung_sperrt_die_oberflaeche_nicht(seite):
    """Punkt 3 als Zusage, nicht als Einzelfall: was gesperrt wurde, wird freigegeben, BEVOR die
    Antwort ausgewertet wird. Hier wirft zeigeVerstanden() — stellvertretend für jede Ausnahme in
    der Auswertung. Die Oberfläche muss trotzdem bedienbar bleiben."""
    page = seite
    feld = _erstes_cent_feld(page)
    _rendere(page, feld)
    # Die äußere Hülle ist nötig: Playwright RUFT einen Ausdruck auf, dessen Wert eine Funktion
    # ist — die nackte Zuweisung hätte den Fehler sofort hier geworfen statt in chatSenden().
    page.evaluate(
        "() => { window.zeigeVerstanden = () => { throw new Error('Testfehler in der Auswertung'); }; }")
    _absenden(page)
    page.evaluate(_AUFLOESEN, _antwort(vorschlaege=[{"feld_id": feld, "wert": 62000}]))
    page.wait_for_timeout(500)

    assert not _warte_sichtbar(page), "Die Laufanzeige steht nach der Ausnahme noch da."
    g = _gesperrt(page)
    assert not any(g.values()), f"Nach der Ausnahme sind noch Sperren gesetzt: {g}"
    assert _klick_geht_durch(page, "#bestaetigen"), (
        "Eine Ausnahme in der Auswertung lässt die Oberfläche gesperrt zurück.")


# --- 2, Messergebnis: was der Kontextwechsel wirklich anrichtet ------------------------------

def test_antwort_zu_einer_anderen_frage_wird_gekennzeichnet(seite):
    """Die Antwort ist der einzige Teil, der sich falsch zuordnen KANN — sie entsteht aus dem
    Kontext der Frage, bei der sie abgeschickt wurde.

    Der Wechsel wird hier nicht per Klick ausgelöst (den sperrt der Fix), sondern über
    `refresh()` — genau das tut ein schon vorher laufender Kontoauszug-/Vorjahr-Import, wenn er
    während des KI-Aufrufs fertig wird. Diesen Weg kann die Sperre nicht schließen."""
    page = seite
    feld = _erstes_cent_feld(page)
    _rendere(page, feld)
    _absenden(page, "zählt homeoffice als arbeitstag?")

    page.evaluate("refresh()")     # wie ein nebenher fertig gewordener Import: AKTUELL = fragen[0]
    page.wait_for_function(f"AKTUELL && AKTUELL.feld_id !== {json.dumps(feld)}", timeout=5000)
    page.evaluate(_AUFLOESEN, _antwort(antwort="Homeoffice-Tage zählen nicht als Fahrten."))
    page.wait_for_selector(".chat-antwort", timeout=5000)

    klassen = page.evaluate("[...document.querySelectorAll('#chat-body p')].map(e => e.className)")
    assert "chat-kontext-hinweis" in klassen, (
        "Die Antwort wurde still unter eine andere Frage gehängt — sie liest sich dort als "
        f"Antwort auf diese. Verlauf: {klassen}")
    assert klassen.index("chat-kontext-hinweis") < klassen.index("chat-antwort"), (
        "Der Hinweis steht hinter der Antwort — dann liest der Nutzer erst die falsche Auskunft.")


def test_ohne_kontextwechsel_kein_hinweis(seite):
    """Gegenprobe: bleibt der Nutzer bei seiner Frage, darf keine Einschränkung erscheinen —
    sonst stünde sie über jeder Antwort und sagte nichts mehr."""
    page = seite
    feld = _erstes_cent_feld(page)
    _rendere(page, feld)
    _absenden(page, "zählt homeoffice als arbeitstag?")
    page.evaluate(_AUFLOESEN, _antwort(antwort="Homeoffice-Tage zählen nicht als Fahrten."))
    page.wait_for_selector(".chat-antwort", timeout=5000)

    klassen = page.evaluate("[...document.querySelectorAll('#chat-body p')].map(e => e.className)")
    assert "chat-kontext-hinweis" not in klassen, (
        f"Hinweis erscheint, obwohl die Frage dieselbe geblieben ist: {klassen}")


def test_vorschlag_landet_unter_seiner_eigenen_feld_id(seite):
    """Das Messergebnis zu Punkt 2, festgehalten: der Schreibpfad hängt NICHT an `AKTUELL`.

    Ein Vorschlag für Feld A, bestätigt während die Oberfläche längst bei Feld B steht, wird
    unter A geschrieben — die feld_id reist im Vorschlag selbst mit (Server: v['feld_id'] in
    api.chat(); Oberfläche: verstandenBestaetigen postet v.feld_id). Deshalb ist der
    Kontextwechsel ein Anzeige- und kein Datenfehler. Bricht diese Zusage, wäre er einer."""
    page = seite
    feld_a, feld_b = _erstes_cent_feld(page, 0), _erstes_cent_feld(page, 1)
    gepostet = []
    page.route("**/event", lambda route: (
        gepostet.append(json.loads(route.request.post_data or "{}")),
        route.fulfill(status=400, content_type="application/json",
                      body=json.dumps({"fehler": "im Test abgefangen"}))))

    _rendere(page, feld_a)
    _absenden(page, "620 euro dafür")
    page.evaluate(_AUFLOESEN, _antwort(vorschlaege=[{"feld_id": feld_a, "wert": 62000,
                                                     "beleg": "620 euro"}]))
    page.wait_for_selector("#verstanden:not([hidden])", timeout=5000)
    _rendere(page, feld_b)     # der Nutzer steht jetzt woanders
    page.evaluate("document.querySelector('#verstanden-liste .v-ok').click()")
    page.wait_for_timeout(400)

    assert gepostet, "Es wurde gar kein Event abgeschickt."
    assert gepostet[-1]["feld_id"] == feld_a, (
        f"Der Vorschlag für {feld_a} wurde unter {gepostet[-1]['feld_id']!r} geschrieben "
        f"(AKTUELL stand auf {feld_b}) — aus dem Anzeigefehler wäre ein Datenfehler geworden.")
