"""Ein Weg statt zweier Aufforderungen — der Nutzerfluss um die KI (Julius 2026-08-23).

ANLASS, wörtlich nach einem echten Durchgang: „es ist vom user flow her unklar wenn die ai dinge
verstanden hat und man felder bestätigen soll UND fragen beantworten, was man zuerst machen
sollte. ich denke es sollte ein klarer flow sein. also zb könnte das ki window dann 1 für 1 die
nachfragen stellen, die man dann in einem input feld beantworten kann. dann klick weiter, nächste
nachfrage (evtl auch die möglichkeit ‚später beantworten' …). entweder dann oder nachdem es keine
rückfragen mehr gibt die verstandenen feld zuordnungen bestätigen. auch könnte man direkt am
anfang den nutzer mit zwei buttons auswählen lassen: fragebogen starten oder erst automatisch
auffüllen lassen."

Bis hierher lieferte /chat `vorschlaege` und `rueckfragen` GLEICHZEITIG: die Vorschläge traten als
eigene Seite (#verstanden) vor, die Rückfragen erschienen als Kästen im Chat-Verlauf. Zwei
Aufforderungen nebeneinander, ohne Reihenfolge — genau der Befund.

Geprüft wird hier der NUTZERPFAD, in drei Teilen:

  TEIL 1  Zwischen Fallart und Fluss liegt die Wegwahl: Fragebogen oder erst die KI.
  TEIL 2  Rückfragen kommen EINZELN, mit einem Eingabefeld, das zum Typ des Feldes passt.
          Die Antwort geht NICHT an die KI zurück, sondern über den normalen /event-Pfad direkt
          an ihr Feld — ein Rückweg über das Modell kostete drei Stufen und könnte dieselbe
          Angabe erneut falsch deuten. Zwei Fallen liegen darin, und beide werden gemessen:
          cent (Euro-Eingabe -> Cent, sonst der 100-fache Betrag) und bool (`frage_invertiert`,
          sonst das Gegenteil der Antwort — 2026-08-20 gemessen mit bis zu 3.486 EUR Wirkung).
  TEIL 3  Die Reihenfolge: erst die Rückfragen, dann die Bestätigungen, dann der Fragebogen.
          Nie zwei Aufforderungen gleichzeitig.

„Später beantworten" schreibt bewusst NICHTS und legt keinen Merker an: das Feld bleibt
unbeantwortet und steht dadurch von selbst wieder in der Fragen-Queue. Ein eigener Zustand
verschwände beim Neuladen und machte dem Nutzer eine Zusage, die die Software nicht hält. Der
Test misst deshalb beides — kein Event im Store UND die Frage wieder im Fragebogen.

KEIN LLM: /chat wird per page.route abgefangen. Der Server sieht diesen Aufruf nie, es fließt kein
Key und kein Cent. Im Handler wird nicht geschlafen — genau das (und nur das) würde Playwrights
sync-Schleife blockieren, s. die Warnung in tests/test_ui_chat_wartezustand.py.
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

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None

pytestmark = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")

# Zwei echte Felder aus der Bindung, absichtlich die beiden heiklen Typen:
#   cent  — die Oberfläche nimmt Euro, der Store hält Cent (Faktor 100)
#   bool  — `kein_unterhalt` benennt die ABWESENHEIT, die Frage fragt nach der ANWESENHEIT
CENT_FELD = "bruttoarbeitslohn"
BOOL_FELD_INVERTIERT = "kein_unterhalt"


def _antwort(antwort="", vorschlaege=None, aussagen=None, rueckfragen=None, konflikte=None,
             neue_felder=True):
    """Eine Server-Antwort in der Form, die api.chat() zurückgibt (wie in
    tests/test_ui_chat_aussagen.py — dieselbe Naht, dieselbe Attrappe).

    `rechenweg` wird nur durchgereicht, wenn der Aufrufer ihn angibt — genau das ist der heutige
    Stand: das Feld existiert im LLM-Schema (produkt/haut/api_llm.py), aber api.chat() trägt es
    nicht in die Antwort. Fehlt es, muss die Oberfläche sich verhalten wie bisher.
    """
    def _mach(v, extra):
        d = {"feld_id": v["feld_id"], "wert": v["wert"], "beleg": v.get("beleg", ""),
             "frage": v.get("frage", "Frage zu " + v["feld_id"]),
             "typ": v.get("typ", "cent"), "frage_invertiert": False,
             "einheit": v.get("einheit"), "enum_labels": None,
             "aussage": v.get("aussage", 0)}
        if "rechenweg" in v:
            d["rechenweg"] = v["rechenweg"]
        d.update(extra)
        return d

    vs = [_mach(v, {"event_id": v.get("event_id", "EV-STUB")}) for v in (vorschlaege or [])]
    ks = [_mach(k, {"aktueller_wert": k["aktueller_wert"], "vorschlag_wert": k["wert"],
                    "aktuelles_event_id": k.get("aktuelles_event_id", "EV-ALT"),
                    "gross": k.get("gross", False), "begruendung": ""})
          for k in (konflikte or [])]
    body = {"vorschlaege": vs, "abgelehnt": [], "abgelehnt_gruende": {}, "konflikte": ks,
            "antwort": antwort, "unsicher": False,
            "hinweis": "Vorschläge erfasst — bitte jeden einzeln bestätigen (die KI setzt nichts)."}
    if neue_felder:
        body["aussagen"] = aussagen or []
        body["rueckfragen"] = rueckfragen or []
    return body


def _rf(frage, feld_id, aussage=0):
    return {"frage": frage, "feld_id": feld_id, "aussage": aussage}


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
    """Seite mit gewähltem Fallart-Kachel. `weg` sagt, wie weit es geht:
    None -> stehenbleiben auf der Wegwahl (TEIL 1), sonst der gewählte Knopf.

    Konsolenfehler werden mitgeschnitten; die eine erwartete Meldung (initAuth-Sonde gegen einen
    nicht existierenden Fall) ist namentlich ausgenommen — s. tests/test_ui_chat_aussagen.py.
    """
    gestartet = []
    SONDE = "/fall/auth-sonde-taxgraph/stand"

    def _mach(weg="fragebogen", breite=360, hoehe=780):
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
        page.wait_for_selector("#wegwahl:not([hidden])", timeout=5000)
        if weg is not None:
            page.evaluate(f"document.getElementById('weg-{weg}').click()")
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
    stehen, damit ein Folgeaufruf noch eine Antwort bekommt."""
    rest = list(antworten)

    def _handler(route):
        daten = rest.pop(0) if len(rest) > 1 else rest[0]
        route.fulfill(status=200, content_type="application/json", body=json.dumps(daten))

    page.route("**/chat", _handler)


def _senden(page, text="ich habe bis juni gearbeitet", rueckfragen_erwartet=True):
    """Absenden und warten, bis die Anzeige steht.

    Auf die Rückfragen-Seite MUSS eigens gewartet werden: sie erscheint hinter einem `await`
    (starteRueckfragen holt die Feldtypen aus /fragen). Der freigegebene Absendeknopf allein sagt
    hier also — anders als bei der Aussagen-Anzeige — noch nichts über den Bildschirm.
    """
    page.fill("#chat-text", text)
    page.click("#chat-send")
    page.wait_for_selector("#chat-send:not([disabled])", timeout=5000)
    if rueckfragen_erwartet:
        page.wait_for_selector("#rueckfragen:not([hidden])", timeout=5000)


def _felder(base, fall_id):
    """Der Stand aus dem STORE, über echtes HTTP — nicht der Browser-Zustand. Was die Oberfläche
    anzeigt, ist eine Behauptung; was hier steht, ist das Ergebnis."""
    with urllib.request.urlopen(f"{base}/fall/{fall_id}/stand", timeout=10) as r:
        return json.loads(r.read().decode("utf-8")).get("felder", {})


def _offene_felder(base, fall_id):
    with urllib.request.urlopen(f"{base}/fall/{fall_id}/fragen", timeout=10) as r:
        return [q["feld_id"] for q in json.loads(r.read().decode("utf-8")).get("fragen", [])]


def _sichtbare_rueckfrage(page):
    return page.evaluate("""() => ({
        offen: !document.getElementById('rueckfragen').hidden,
        zaehler: document.getElementById('rf-zaehler').textContent,
        frage: document.getElementById('rf-frage').textContent,
        eingaben: document.querySelectorAll('#rf-eingabe input, #rf-eingabe select').length,
    })""")


# ============================================================ TEIL 1: die Einstiegswahl

def test_die_wegwahl_steht_zwischen_fallart_und_fluss(seite_factory, base):
    """Julius: „direkt am anfang den nutzer mit zwei buttons auswählen lassen". Der Fall ist zu
    diesem Zeitpunkt bereits angelegt (die Fallart steckt in ihm), der Fluss aber noch zu."""
    page = seite_factory(weg=None)
    assert page.is_hidden("#start"), "Der Start-Screen liegt noch vorn."
    assert page.is_hidden("#flow"), (
        "Der Fluss ist schon offen — dann ist die Wahl keine Wahl, sondern eine Zwischenseite.")
    knoepfe = page.evaluate(
        "[...document.querySelectorAll('#wegwahl .kachel')].map(b => b.id)")
    assert knoepfe == ["weg-fragebogen", "weg-ki"], f"Nicht genau zwei Wege: {knoepfe}"
    # Der Hinweis, den Julius ausdrücklich an der Fragebogen-Wahl haben wollte.
    text = page.text_content("#weg-fragebogen")
    assert "KI" in text and "jederzeit" in text, (
        f"Kein Hinweis, dass die KI zwischendurch gefüttert werden kann: {text!r}")
    # Tap-Ziele wie auf dem Start-Screen (dort misst tests/test_ui_responsive.py sie).
    for b in page.query_selector_all("#wegwahl .kachel"):
        box = b.bounding_box()
        assert box and box["height"] >= 44 and box["width"] >= 44, f"Tap-Ziel zu klein: {box}"
    assert page.evaluate("document.documentElement.scrollWidth") <= 360, (
        "Die Wegwahl scrollt bei 360px seitwärts.")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_der_fragebogen_weg_stellt_die_erste_frage(seite_factory):
    """Zwei getrennte Tests statt eines mit zwei Seiten: `sync_playwright().start()` zweimal im
    selben Test läuft in Playwrights eigene asyncio-Schleife („Sync API inside the asyncio loop")."""
    page = seite_factory(weg="fragebogen")
    assert not page.is_hidden("#wegpunkt")
    assert page.evaluate("document.activeElement.id") == "wegpunkt", (
        "Auf dem Fragebogen-Weg liegt der Fokus nicht auf der Frage.")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_der_ki_weg_fuehrt_in_denselben_fluss(seite_factory):
    """Kein zweiter Modus: der KI-Weg öffnet denselben Fragebogen. Der Unterschied ist allein, wo
    der Nutzer zuerst steht — ein echter „KI-Modus" überlebte kein Neuladen und verspräche etwas,
    das die Software nicht hält."""
    page = seite_factory(weg="ki")
    assert not page.is_hidden("#wegpunkt"), (
        "Der KI-Weg versteckt den Fragebogen — dann sind es zwei Modi statt zweier Einstiege.")
    assert page.evaluate("document.activeElement.id") == "chat-text", (
        "Auf dem KI-Weg liegt der Fokus nicht im Eingabefeld der KI — dann ist der Knopf nur ein "
        "anderer Name für denselben Einstieg.")
    assert "Schreib einfach los" in page.text_content("#chat-body"), (
        "Der KI-Weg sagt nicht, was der Nutzer jetzt tun soll.")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


# ============================================================ TEIL 2: eine Frage nach der anderen

def test_rueckfragen_kommen_einzeln(seite):
    """Der Kern des Auftrags: zwei Rückfragen, aber nur EINE auf dem Schirm."""
    page = seite
    _stub(page, _antwort(rueckfragen=[
        _rf("Meinst du 100.000 für das ganze Jahr oder anteilig bis Juni?", CENT_FELD),
        _rf("Unterstützt du jemanden finanziell?", BOOL_FELD_INVERTIERT),
    ]))
    _senden(page)

    z = _sichtbare_rueckfrage(page)
    assert z["offen"], "Die Rückfragen-Seite ist gar nicht erschienen."
    assert "anteilig bis Juni" in z["frage"], f"Falsche oder keine Frage: {z}"
    assert "Unterstützt du jemanden" not in z["frage"], (
        "Beide Fragen stehen zugleich auf dem Schirm — genau das war der Befund.")
    assert "1 von 2" in z["zaehler"], f"Kein Zähler, der die Reihenfolge sichtbar macht: {z}"
    assert z["eingaben"] == 1, f"Nicht genau ein Eingabefeld: {z}"
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_weiter_fuehrt_zur_naechsten_rueckfrage(seite, base):
    page = seite
    fall = page.evaluate("FALL")
    assert CENT_FELD in _offene_felder(base, fall), (
        f"{CENT_FELD} steht nicht in den offenen Fragen — dann prüft der Test den Fallback, "
        "nicht den Weg.")
    _stub(page, _antwort(rueckfragen=[
        _rf("Ganzes Jahr oder anteilig?", CENT_FELD),
        _rf("Unterstützt du jemanden finanziell?", BOOL_FELD_INVERTIERT),
    ]))
    _senden(page)

    page.fill("#rf-input", "62000")
    page.click("#rf-weiter")
    page.wait_for_selector("#rf-zaehler:has-text('2 von 2')", timeout=5000)
    z = _sichtbare_rueckfrage(page)
    assert "Unterstützt du jemanden" in z["frage"], f"Die zweite Frage kam nicht: {z}"
    assert "Ganzes Jahr" not in z["frage"], "Die erste Frage steht noch daneben."


def test_ein_cent_feld_landet_als_cent_im_store(seite, base):
    """Die teuerste Falle dieses Umbaus: die Oberfläche nimmt EURO, der Store hält CENT. Ein
    Eingabefeld, das die Umrechnung nicht macht, schriebe den 100-fachen Betrag — und zwar
    bestätigt, also zählend. Gemessen wird im STORE, nicht im Browser."""
    page = seite
    fall = page.evaluate("FALL")
    _stub(page, _antwort(rueckfragen=[_rf("Wie hoch war dein Bruttoarbeitslohn?", CENT_FELD)]))
    _senden(page)

    page.fill("#rf-input", "62000")
    page.click("#rf-weiter")
    page.wait_for_selector("#rueckfragen", state="hidden", timeout=5000)

    f = _felder(base, fall).get(CENT_FELD)
    assert f is not None, f"Die Antwort ist gar nicht im Store angekommen: {_felder(base, fall)}"
    assert f["wert"] == 6200000, (
        f"62.000 EUR wurden als {f['wert']} gespeichert statt als 6200000 Cent — "
        f"{'der 100-fache' if f['wert'] == 62000 * 100 * 100 else 'ein falscher'} Betrag.")
    assert f["wert"] != 62000, "Euro statt Cent gespeichert — Faktor 100 zu wenig."
    assert f["zustand"] == "bestaetigt", (
        f"Die Antwort liegt nur vorläufig vor ({f['zustand']}) — sie zählt dann in keiner Summe, "
        "obwohl der Nutzer sie selbst eingetippt hat.")
    assert f["herkunft_badge"] == "laie", (
        f"Herkunft {f['herkunft_badge']!r} statt 'laie' — der Nutzer hat es selbst geschrieben, "
        "nicht die KI.")


def test_bool_umkehr_gilt_auch_im_rueckfragen_schritt(seite, base):
    """`kein_unterhalt` benennt die ABWESENHEIT, seine Frage fragt nach der ANWESENHEIT
    (`frage_invertiert: true`). Wer hier „Ja" antwortet, muss `false` im Store erzeugen. Dieselbe
    Umkehr ist 2026-08-20 an zwei Feldern gemessen worden — bis zu 3.486 EUR Wirkung."""
    page = seite
    fall = page.evaluate("FALL")
    assert BOOL_FELD_INVERTIERT in _offene_felder(base, fall), (
        f"{BOOL_FELD_INVERTIERT} steht nicht in den offenen Fragen.")
    _stub(page, _antwort(rueckfragen=[
        _rf("Unterstützt du jemanden finanziell?", BOOL_FELD_INVERTIERT)]))
    _senden(page)

    # „Ja" — der Nutzer unterstützt jemanden.
    page.evaluate("""() => [...document.querySelectorAll('#rf-eingabe .wahl-opt')]
        .find(b => b.textContent.trim() === 'Ja').click()""")
    page.click("#rf-weiter")
    page.wait_for_selector("#rueckfragen", state="hidden", timeout=5000)

    f = _felder(base, fall).get(BOOL_FELD_INVERTIERT)
    assert f is not None, "Die Antwort ist nicht im Store angekommen."
    assert f["wert"] is False, (
        f"„Ja, ich unterstütze jemanden“ wurde als {f['wert']!r} gespeichert. Das Feld heißt "
        "`kein_unterhalt` — gespeichert gehört das Gegenteil der Antwort, sonst entfällt für "
        "diesen Nutzer der ganze Unterhaltsblock (17 Fragen).")


def test_ein_leeres_feld_schreibt_nichts(seite, base):
    """Stille-Null: „Weiter" auf einem leeren Feld darf keine 0 erzeugen. Dieselbe Regel wie im
    Fragebogen (leseWert -> undefined -> kein Event), hier über denselben Leser."""
    page = seite
    fall = page.evaluate("FALL")
    _stub(page, _antwort(rueckfragen=[_rf("Wie hoch war dein Bruttoarbeitslohn?", CENT_FELD)]))
    _senden(page)

    page.click("#rf-weiter")
    page.wait_for_selector("#netz-banner:not([hidden])", timeout=5000)
    assert not page.is_hidden("#rueckfragen"), (
        "Die Seite ist weitergesprungen, obwohl nichts eingegeben wurde.")
    assert CENT_FELD not in _felder(base, fall), (
        "Ein leeres Feld hat einen Wert erzeugt — ununterscheidbar von einer echten 0.")


def test_spaeter_schreibt_nichts_und_die_frage_kommt_im_fragebogen_wieder(seite, base):
    """Julius: „evtl auch die möglichkeit ‚später beantworten' falls der nutzer die unterlagen in
    dem moment nicht da hat". Bewusst OHNE Merker: das Feld bleibt unbeantwortet und steht dadurch
    von selbst wieder in der Queue. Beides wird gemessen — kein Event, und die Frage ist zurück."""
    page = seite
    fall = page.evaluate("FALL")
    _stub(page, _antwort(rueckfragen=[_rf("Wie hoch war dein Bruttoarbeitslohn?", CENT_FELD)]))
    _senden(page)

    page.click("#rf-spaeter")
    page.wait_for_selector("#rueckfragen", state="hidden", timeout=5000)

    assert CENT_FELD not in _felder(base, fall), (
        "„Später“ hat etwas geschrieben — dann ist es kein Aufschub, sondern eine Antwort.")
    assert CENT_FELD in _offene_felder(base, fall), (
        "Das Feld steht nicht mehr in den offenen Fragen — dann ist die Frage weg, obwohl der "
        "Nutzer sie nur aufgeschoben hat.")
    page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)


def test_eine_rueckfrage_ohne_feld_bleibt_beim_chat(seite):
    """Ohne `feld_id` (oder zu einem Feld, das gar nicht mehr gefragt wird) gibt es keinen Typ und
    damit kein Eingabefeld, in das sich etwas schreiben ließe. Dann bleibt der Chat der Weg — wie
    bisher, und der Knopf sagt es."""
    page = seite
    _stub(page, _antwort(rueckfragen=[_rf("Wie war das Jahr für dich insgesamt?", "")]))
    _senden(page)

    z = _sichtbare_rueckfrage(page)
    assert z["offen"] and z["eingaben"] == 0, (
        f"Ohne Feld darf kein Eingabefeld dastehen — es wüsste nicht, wohin: {z}")
    assert "Berater" in page.text_content("#rf-weiter"), (
        f"Der Knopf verspricht etwas anderes: {page.text_content('#rf-weiter')!r}")
    page.click("#rf-weiter")
    assert page.input_value("#chat-text").startswith("Zu deiner Rückfrage"), (
        "Der Knopf führt nicht in den Chat — dann ist die Frage unbeantwortbar.")


# ============================================================ TEIL 3: die Reihenfolge

def test_nie_zwei_aufforderungen_gleichzeitig(seite):
    """Der Befund selbst: solange eine Rückfrage offen ist, darf die Verstanden-Seite NICHT
    danebenstehen. Der Vorschlag gehört zu einem anderen Feld, ist also kein zurückgehaltener
    Wert — er wartet schlicht, bis er dran ist."""
    page = seite
    _stub(page, _antwort(
        rueckfragen=[_rf("Ganzes Jahr oder anteilig?", CENT_FELD)],
        vorschlaege=[{"feld_id": "aussergewoehnliche_belastungen", "wert": 62000,
                      "beleg": "620 euro arztkosten"}]))
    _senden(page)

    assert page.is_hidden("#verstanden"), (
        "Bestätigungen und Rückfrage stehen gleichzeitig auf dem Schirm — genau der Befund.")
    assert page.is_hidden("#wegpunkt"), (
        "Der Fragebogen steht daneben — dann sind es wieder zwei Aufforderungen.")
    assert not page.is_hidden("#rueckfragen")
    # Und der Nutzer erfährt, dass da noch etwas kommt — statt es zu übersehen.
    assert "nach den Rückfragen" in page.text_content("#chat-body"), (
        "Der zurückgestellte Vorschlag wird nirgends erwähnt — für den Nutzer ist er verschwunden.")


def test_nach_der_letzten_rueckfrage_kommen_die_bestaetigungen(seite):
    """Julius: „entweder dann oder nachdem es keine rückfragen mehr gibt die verstandenen feld
    zuordnungen bestätigen". Also: erst wenn die Rückfragen durch sind."""
    page = seite
    _stub(page, _antwort(
        rueckfragen=[_rf("Ganzes Jahr oder anteilig?", CENT_FELD)],
        vorschlaege=[{"feld_id": "aussergewoehnliche_belastungen", "wert": 62000,
                      "beleg": "620 euro arztkosten"}]))
    _senden(page)

    page.fill("#rf-input", "62000")
    page.click("#rf-weiter")
    page.wait_for_selector("#verstanden:not([hidden])", timeout=5000)
    assert page.is_hidden("#rueckfragen"), "Die Rückfragen-Seite steht noch daneben."
    assert page.query_selector("#verstanden-liste .v-ok") is not None, (
        "Die Bestätigung ist nicht anklickbar angekommen.")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_ohne_bestaetigungen_geht_es_zurueck_in_den_fragebogen(seite):
    page = seite
    _stub(page, _antwort(rueckfragen=[_rf("Ganzes Jahr oder anteilig?", CENT_FELD)]))
    _senden(page)
    page.click("#rf-spaeter")
    page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)
    assert page.is_hidden("#verstanden") and page.is_hidden("#rueckfragen")


def test_eine_neue_antwort_ueberholt_die_laufende_runde(seite):
    """Der Nutzer kann jederzeit in den Chat schreiben — auch mitten in einer Runde. Die neue
    Antwort ist zu demselben Gespräch die neuere Auskunft; die alten Fragen können durch den eben
    geschickten Satz längst beantwortet sein. Danach darf kein Rest stehenbleiben."""
    page = seite
    _stub(page,
          _antwort(rueckfragen=[_rf("Ganzes Jahr oder anteilig?", CENT_FELD),
                                _rf("Unterstützt du jemanden?", BOOL_FELD_INVERTIERT)]),
          _antwort(antwort="Danke — dann rechne ich mit dem anteiligen Betrag."))
    _senden(page)

    _senden(page, "anteilig, ich habe nur bis Juni gearbeitet", rueckfragen_erwartet=False)
    page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)
    assert page.is_hidden("#rueckfragen"), (
        "Die alte Rückfrage steht noch da, obwohl der Nutzer inzwischen geantwortet hat.")
    assert "anteiligen Betrag" in page.text_content("#chat-body .chat-antwort")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


# ============================================================ Auflagen

def test_ohne_rueckfragen_bleibt_alles_wie_bisher(seite):
    """Abwärtskompatibel, Auflage 3: fehlt `rueckfragen` in der Antwort (älterer Endpunkt, Ausfall
    einer Stufe), verhält sich alles wie vorher — Vorschläge gehen direkt zur Bestätigung, und
    nichts landet in der Konsole."""
    page = seite
    _stub(page, _antwort(antwort="Das kommt in Zeile 61 der Anlage.",
                         vorschlaege=[{"feld_id": "aussergewoehnliche_belastungen",
                                       "wert": 62000, "beleg": "620 euro"}],
                         neue_felder=False))
    _senden(page, rueckfragen_erwartet=False)

    page.wait_for_selector("#verstanden:not([hidden])", timeout=5000)
    assert page.is_hidden("#rueckfragen"), "Ohne Rückfragen steht trotzdem die neue Seite da."
    assert "Das kommt in Zeile 61" in page.text_content("#chat-body .chat-antwort")
    assert "— oben." in page.text_content("#chat-body"), (
        "Die Meldung verweist nicht mehr auf die Bestätigungen oben.")
    assert not page.fehler, f"Konsolenfehler ohne die neuen Felder: {page.fehler}"


def test_die_chat_sperre_fasst_auch_die_rueckfragen_seite(seite):
    """Auflage 2, gemessen statt angenommen: während eines laufenden KI-Aufrufs darf der Nutzer
    im Rückfragen-Schritt nicht weiterklicken — „Weiter" dort schreibt ein Event und ruft
    refresh(), ändert also `AKTUELL` mitten im Aufruf.

    Die Seite hält den Aufruf offen (window.fetch-Stub wie in
    tests/test_ui_chat_wartezustand.py) — page.route kann ein Zeitfenster nicht offen halten,
    ohne Playwrights eigene Schleife zu blockieren."""
    page = seite
    _stub(page, _antwort(rueckfragen=[_rf("Ganzes Jahr oder anteilig?", CENT_FELD)]))
    _senden(page)

    page.evaluate("""() => {
      window.__CHAT_OFFEN = null;
      window.__ECHT = window.fetch;
      window.fetch = (url, opt) => String(url).endsWith('/chat')
        ? new Promise((res, rej) => { window.__CHAT_OFFEN = { res, rej }; })
        : window.__ECHT(url, opt);
    }""")
    page.fill("#chat-text", "und eine Brille")
    page.click("#chat-send")
    page.wait_for_selector("#chat-send[disabled]", timeout=5000)

    gesperrt = page.evaluate(
        "document.getElementById('rueckfragen').hasAttribute('inert')")
    assert gesperrt, (
        "Die Rückfragen-Seite bleibt während des Aufrufs bedienbar — ein Klick auf „Weiter“ "
        "schriebe dort ein Event und zöge `AKTUELL` unter dem laufenden Aufruf weg.")

    page.evaluate("""(daten) => { const o = window.__CHAT_OFFEN;
        setTimeout(() => o.res(new Response(JSON.stringify(daten),
            { status: 200, headers: { 'Content-Type': 'application/json' } })), 0); }""",
        _antwort(antwort="Verstanden."))
    page.wait_for_selector("#chat-send:not([disabled])", timeout=5000)
    offen = page.evaluate("""() => ['wegpunkt','verstanden','belegt-liste','rueckfragen']
        .filter(id => document.getElementById(id).hasAttribute('inert'))""")
    assert offen == [], f"Nach dem Aufruf sind noch Sperren gesetzt: {offen}"


def test_die_rueckfragen_seite_nimmt_den_fokus(seite):
    """Auflage 6: #verstanden setzt beim Einblenden den Fokus, damit ein Screenreader den
    Bildschirmwechsel ansagt. Für einen Schritt, der die einzige Aufforderung auf dem Schirm ist,
    gilt das erst recht — und zwar bei JEDER Frage, nicht nur bei der ersten."""
    page = seite
    _stub(page, _antwort(rueckfragen=[_rf("Ganzes Jahr oder anteilig?", CENT_FELD),
                                      _rf("Unterstützt du jemanden?", BOOL_FELD_INVERTIERT)]))
    _senden(page)
    assert page.evaluate("document.activeElement.id") == "rueckfragen", (
        "Der Fokus bleibt im Chat — der Screenreader sagt den Wechsel nicht an.")

    page.fill("#rf-input", "62000")
    page.click("#rf-weiter")
    page.wait_for_selector("#rf-zaehler:has-text('2 von 2')", timeout=5000)
    assert page.evaluate("document.activeElement.id") == "rueckfragen", (
        "Bei der zweiten Frage wandert der Fokus nicht mit.")
    # Der Ausweg, der ein Overlay hier zum Käfig machte: „Später" ist auf jeder Frage da.
    assert not page.is_hidden("#rf-spaeter"), (
        "Ohne Ausweg sitzt fest, wer die Frage nicht beantworten kann.")


def test_die_seite_passt_auf_den_schmalen_schirm(seite_factory):
    """Auflage 5: 360px ist die Messlatte (tests/test_ui_responsive.py). Die Frage ist Modelltext
    beliebiger Länge — anders als #frage, das aus der Bindung kommt. Der Prüftext enthält deshalb
    ein unteilbares Wort; genau daran, und nur daran, hängt `overflow-wrap:anywhere`."""
    page = seite_factory(breite=360, hoehe=640)
    lang = ("Meintest du den Betrag auf der Buchung "
            "DE00500105175407324931unterkontoLohnUndGehaltJanuarBisJuni2025Sammelbuchung?")
    _stub(page, _antwort(rueckfragen=[_rf(lang, CENT_FELD)]))
    _senden(page)

    assert page.evaluate("document.documentElement.scrollWidth") <= 360, (
        "Horizontales Scrollen bei 360px durch die Rückfragen-Seite.")
    box = page.query_selector("#rueckfragen").bounding_box()
    assert box["x"] >= 0 and box["x"] + box["width"] <= 360, f"Die Karte ragt aus dem Bild: {box}"
    for sel in ("#rf-weiter", "#rf-spaeter"):
        b = page.query_selector(sel).bounding_box()
        assert b["height"] >= 44, f"Tap-Ziel {sel} unter 44px: {b}"
        assert b["x"] >= 0 and b["x"] + b["width"] <= 360, f"{sel} ragt aus dem Bild: {b}"
    # Keine Überlappung: nebeneinander oder gestapelt, aber nicht übereinander.
    a = page.query_selector("#rf-weiter").bounding_box()
    c = page.query_selector("#rf-spaeter").bounding_box()
    assert (a["x"] + a["width"] <= c["x"] + 1) or (a["y"] + a["height"] <= c["y"] + 1), (
        f"Die beiden Knöpfe überlappen: {a} / {c}")


# ============================================================ Der Rechenweg unter dem Wert

def _rechenweg_lage(page):
    return page.evaluate("""() => {
        const li = document.querySelector('#verstanden-liste .v-zeile');
        if (!li) return null;
        const rw = li.querySelector('.v-rechenweg');
        const wert = li.querySelector('.v-wert') || li.querySelector('.v-paar');
        const beleg = li.querySelector('.v-beleg');
        if (!rw) return {da: false};
        const nach = (a, b) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) > 0;
        return {da: true, text: rw.textContent,
                unter_dem_wert: nach(wert, rw),
                ueber_dem_beleg: beleg ? nach(rw, beleg) : null};
    }""")


def test_der_rechenweg_steht_unter_dem_wert(seite):
    """Julius 2026-08-23 über die ausgerechneten Werte: die Rechnung gehört unter den Wert, „damit
    der Nutzer die Rechnung sieht und prüfen kann". Genau daran hing der teuerste gemessene Fehler
    dieses Kanals — aus „bis Juni 100k p.a." wurde ein volles Jahresbrutto.

    Reihenfolge ist hier Inhalt: Ergebnis, dann wie es entstand, dann worauf es sich stützt."""
    page = seite
    _stub(page, _antwort(vorschlaege=[{
        "feld_id": CENT_FELD, "wert": 2500000, "beleg": "50.000 im jahr, bis juni gearbeitet",
        "rechenweg": {"basis": 5000000, "faktor": 0.5,
                      "erklaerung": "50.000 € pro Jahr ÷ 12 × 6 Monate"}}]))
    _senden(page, rueckfragen_erwartet=False)
    page.wait_for_selector("#verstanden:not([hidden])", timeout=5000)

    lage = _rechenweg_lage(page)
    assert lage and lage["da"], (
        "Der ausgerechnete Wert steht ohne seine Rechnung da — der Nutzer bestätigt eine Zahl, "
        "deren Zustandekommen er nicht sehen kann.")
    assert "50.000 € pro Jahr ÷ 12 × 6 Monate" in lage["text"], f"Rechenweg-Text fehlt: {lage}"
    assert lage["unter_dem_wert"], f"Der Rechenweg steht nicht unter dem Wert: {lage}"
    assert lage["ueber_dem_beleg"], (
        f"Der Rechenweg steht unter dem Zitat statt darüber: {lage} — der Leser geht vom Ergebnis "
        "über die Rechnung zum eigenen Satz zurück, nicht andersherum.")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_ohne_rechenweg_steht_dort_nichts(seite):
    """Abwärtskompatibel und zugleich der Normalfall: wo nichts gerechnet wurde (`rechenweg` null
    oder ganz fehlend — api.chat() liefert das Feld heute noch gar nicht), darf keine leere Zeile
    stehen und nichts in der Konsole landen."""
    page = seite
    _stub(page, _antwort(vorschlaege=[
        {"feld_id": CENT_FELD, "wert": 6200000, "beleg": "62000 brutto", "rechenweg": None},
        {"feld_id": "aussergewoehnliche_belastungen", "wert": 62000, "beleg": "620 euro"},
    ]))
    _senden(page, rueckfragen_erwartet=False)
    page.wait_for_selector("#verstanden:not([hidden])", timeout=5000)

    assert page.query_selector("#verstanden-liste .v-rechenweg") is None, (
        "Ohne Rechnung steht trotzdem eine Rechenweg-Zeile da.")
    assert len(page.query_selector_all("#verstanden-liste .v-zeile")) == 2, (
        "Die Vorschläge selbst kamen nicht mehr durch.")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_ein_rechenweg_ohne_satz_verschwindet_nicht(seite):
    """Kommt die Rechnung ohne ihren Satz (leere `erklaerung`, aber Basis und Faktor da), wird sie
    aus den Zahlen zusammengesetzt statt still weggelassen. `basis` steht laut Schema in derselben
    Einheit wie der Wert — bei Geld also in CENT, und muss als Euro erscheinen."""
    page = seite
    _stub(page, _antwort(vorschlaege=[{
        "feld_id": CENT_FELD, "wert": 2500000, "beleg": "50.000 im jahr",
        "rechenweg": {"basis": 5000000, "faktor": 0.5, "erklaerung": ""}}]))
    _senden(page, rueckfragen_erwartet=False)
    page.wait_for_selector("#verstanden:not([hidden])", timeout=5000)

    lage = _rechenweg_lage(page)
    assert lage and lage["da"], "Ohne Satz ist die Rechnung ganz verschwunden."
    assert "50.000,00" in lage["text"] and "0.5" in lage["text"], (
        f"Die Basis erscheint nicht als Euro-Betrag: {lage['text']!r} — 5000000 sind 50.000 €, "
        "nicht fünf Millionen.")


def test_der_rechenweg_steht_auch_am_widerspruch(seite):
    """Am Konflikt wiegt die Rechnung am schwersten: dort vergleicht der Nutzer die eigene Zahl mit
    einer ausgerechneten und soll abwägen. Ohne den Rechenweg stünden zwei nackte Beträge da."""
    page = seite
    _stub(page, _antwort(konflikte=[{
        "feld_id": CENT_FELD, "aktueller_wert": 5000000, "wert": 2500000,
        "beleg": "bis juni gearbeitet",
        "rechenweg": {"basis": 5000000, "faktor": 0.5,
                      "erklaerung": "50.000 € pro Jahr ÷ 12 × 6 Monate"}}]))
    _senden(page, rueckfragen_erwartet=False)
    page.wait_for_selector("#verstanden:not([hidden])", timeout=5000)

    lage = _rechenweg_lage(page)
    assert lage and lage["da"], "Am Widerspruch fehlt die Rechnung."
    assert "÷ 12 × 6 Monate" in lage["text"]
    assert lage["unter_dem_wert"], (
        f"Der Rechenweg steht nicht unter dem Wertepaar: {lage}")
    assert page.query_selector("#verstanden-liste .v-uebernehmen") is not None, (
        "Der Konflikt ist nicht mehr auflösbar.")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_die_seite_passt_auch_neben_das_ki_panel(seite_factory):
    """Ab 900px ist #flow ein Grid: Inhalt links, KI-Berater rechts. Die neue Seite ist ein Kind
    von #flow und muss dort in Spalte 1 landen — nicht unter oder hinter dem Panel. Genau dieses
    Nebeneinander ist der Grund, eine Seite statt eines Vollbild-Overlays zu bauen: die Rückfrage
    steht neben dem Satz, auf den sie sich bezieht."""
    page = seite_factory(breite=1280, hoehe=800)
    _stub(page, _antwort(rueckfragen=[_rf("Ganzes Jahr oder anteilig?", CENT_FELD)]))
    _senden(page)

    lage = page.evaluate("""() => {
        const r = document.getElementById('rueckfragen').getBoundingClientRect();
        const b = document.getElementById('berater').getBoundingClientRect();
        return {rf_rechts: r.right, berater_links: b.left, rf_breite: r.width,
                berater_sichtbar: b.width > 0 && b.height > 0};
    }""")
    assert lage["berater_sichtbar"], "Das KI-Panel ist neben der Seite verschwunden."
    assert lage["rf_rechts"] <= lage["berater_links"] + 1, (
        f"Die Rückfragen-Seite schiebt sich über das KI-Panel: {lage} — dann verdeckt sie genau "
        "den Zusammenhang, aus dem die Frage stammt.")
    assert page.query_selector("#rf-input") is not None, "Das Eingabefeld fehlt in der Breite."
