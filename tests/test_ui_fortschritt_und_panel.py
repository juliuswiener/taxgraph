"""Fortschritts-Nenner und KI-Panel — zwei Befunde aus einem echten Nutzerlauf (Julius 2026-08-21).

(1) „wenn bei bescheid 2/2 -> 3/3 -> 4/4 steht hat das keine aussage: es sollte 1/(gesamte
    fragenanzahl die noch beantwortet werden muss (regelmäßig geupdated)) da stehen."

    Der Nenner war `Object.values(stand.felder).length` — die Zahl der schon ANGEFASSTEN Felder.
    Der wuchs mit jeder Antwort um genau so viel wie der Zähler; Ring und Leiste standen deshalb
    dauerhaft fast voll. Jetzt: bestätigte + noch offene Fragen, letztere aus /fragen.

    Die Gesamtzahl ist dabei keine Konstante — eine Antwort kann Blöcke abschalten (sie sinkt) oder
    aufmachen (sie steigt). Beide Richtungen stehen hier als eigener Test, weil beide den Balken
    springen lassen und die Anzeige das benennen muss statt es zu verschweigen.

(2) „es macht mehr sinn den KI agent als eigenes panel rechts darzustellen als in die column der
    felder." Breit: daneben. Schmal: darunter, nicht abgeschnitten.

WELCHE Felder hier geantwortet werden, wird GEMESSEN und nicht geraten: die Bindung ändert sich
(gerade jetzt entstehen parallel Screening-Flags), und ein fest verdrahteter Feldname wäre ein Test,
der beim nächsten YAML-Commit aus dem falschen Grund rot oder — schlimmer — aus dem falschen Grund
grün wird. Die Sonden unten suchen sich ihr Feld über die HTTP-API selbst.

KEIN LLM: es wird kein /chat aufgerufen.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import audit             # noqa: E402
import server as SRV     # noqa: E402

try:
    from playwright.sync_api import sync_playwright  # noqa: E402
except ImportError:
    sync_playwright = None

pytestmark = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
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


def _req(base, method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _event(fid, wert, ersetzt=None):
    ev = {"feld_id": fid, "wert": wert, "zustand": "bestaetigt",
          "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
          "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fid}"}}
    if ersetzt:
        ev["ersetzt"] = ersetzt
        ev["signal"]["signal_1"] = ersetzt
    return ev


# --- Sonden: welches Feld tut was? -----------------------------------------------------------
# Beide legen eigene Wegwerf-Fälle an und messen die Wirkung EINER Antwort auf die Länge der
# Traverser-Queue. Der eigentliche Test läuft danach auf einem frischen Fall.

_SONDE = [0]


def _neuer_fall(base, scheibe="gesamt"):
    _SONDE[0] += 1
    fid = f"sonde-{_SONDE[0]}"
    status, body = _req(base, "POST", "/fall", {"scheibe": scheibe,
                                                "veranlagungszeitraum": 2025, "fall_id": fid})
    assert status == 201, (status, body)
    return body["fall_id"]


def _offene(base, fall_id):
    status, body = _req(base, f"GET", f"/fall/{fall_id}/fragen")
    assert status == 200, (status, body)
    return body["fragen"]


def _kandidatenwerte(q):
    """Antwortwerte, die für dieses Feld überhaupt in Frage kommen — FELDWERTE, wie die API sie
    erwartet (keine UI-Umkehr über frage_invertiert; die passiert erst in leseWert())."""
    if q["typ"] == "bool":
        return [False, True]
    if q["typ"] == "enum":
        return list(q.get("enum_werte") or [])
    return [q["beispielwert"]] if q.get("beispielwert") is not None else []


def _sonde(base, kandidaten, treffer):
    """Erstes (feld_id, wert), für das `treffer(vorher, nachher)` gilt. Jede Probe auf eigenem Fall
    — sonst hingen die Messungen aneinander."""
    for q in kandidaten:
        for wert in _kandidatenwerte(q):
            fall = _neuer_fall(base)
            vorher = len(_offene(base, fall))
            status, body = _req(base, "POST", f"/fall/{fall}/event", _event(q["feld_id"], wert))
            if status not in (200, 201):
                continue          # Feld nimmt diesen Wert nicht an — nächster Kandidat
            nachher = len(_offene(base, fall))
            if treffer(vorher, nachher):
                return q["feld_id"], wert, vorher, nachher
    return None


def _sonde_korrektur(base, feld, wert, gegenwerte):
    """Erster Gegenwert, der die abgeschalteten Fragen wieder in die Queue holt, plus wie viele.
    Wieder über HTTP gemessen, damit die Skip-Entscheidung nicht an der geprüften Anzeige hängt."""
    for gegen in gegenwerte:
        fall = _neuer_fall(base)
        _req(base, "POST", f"/fall/{fall}/event", _event(feld, wert))
        eng = len(_offene(base, fall))
        st, body = _req(base, "GET", f"/fall/{fall}/feld/{feld}/warum")
        if st != 200:
            continue
        st, _ = _req(base, "POST", f"/fall/{fall}/event",
                     _event(feld, gegen, ersetzt=body["justification"]["event_id"]))
        if st not in (200, 201):
            continue
        weit = len(_offene(base, fall))
        if weit > eng:
            return gegen, weit - eng
    return None, 0


def _vorlaeufiges_feld(base, ziel_fall):
    """Ein VORLÄUFIGER Wert im Zielfall, über den echten Produktweg: ein bestätigtes Feld aus einem
    Vorjahres-Fall übernehmen (POST /vorjahr, herkunft=vorjahr, zustand=vorlaeufig — genau das, was
    der Knopf „↻ Vorjahr-Daten übernehmen" auslöst). Kein LLM nötig, um einen Vorschlag zu haben.
    Rückgabe: Zahl der übernommenen Felder."""
    quelle = _neuer_fall(base)
    kandidaten = [q for q in _offene(base, quelle)
                  if q["typ"] == "cent" and q.get("beispielwert") is not None]
    assert kandidaten, "Kein cent-Feld mit Beispielwert für die Vorjahr-Übernahme"
    for q in kandidaten[:5]:
        _req(base, "POST", f"/fall/{quelle}/event", _event(q["feld_id"], q["beispielwert"]))
    status, body = _req(base, "POST", f"/fall/{ziel_fall}/vorjahr", {"vorjahr_fall_id": quelle})
    assert status == 200, (status, body)
    return body.get("uebernommen", 0)


@pytest.fixture
def fall_fragen(base):
    """Die Queue eines frischen Falls — Grundlage beider Sonden."""
    return _offene(base, _neuer_fall(base))


# --- Browser ---------------------------------------------------------------------------------

@pytest.fixture
def seite_factory(base):
    gestartet = []

    def _mach(breite=360, hoehe=780):
        p = sync_playwright().start()
        gestartet.append(p)
        browser = p.chromium.launch(args=["--no-sandbox", "--disable-setuid-sandbox"])
        ctx = browser.new_context(viewport={"width": breite, "height": hoehe})
        page = ctx.new_page()
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        # P0b (2026-08-23): zwischen Fallart und Fluss liegt jetzt die Wegwahl (Fragebogen / erst KI).
        page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
        page.wait_for_selector("#wegpunkt:not([hidden])", timeout=8000)
        return page

    try:
        yield _mach
    finally:
        for p in gestartet:
            p.stop()


def _anzeige(page):
    """Was auf dem Bildschirm steht — Zähler, Nenner, Leiste, Notiz."""
    roh = page.evaluate("""() => ({
        mitte: document.getElementById('ring-mitte').textContent,
        text: document.getElementById('fortschritt-text').textContent,
        max: document.getElementById('fortschritt').max,
        wert: document.getElementById('fortschritt').value,
        anteil: document.getElementById('ring').style.getPropertyValue('--anteil'),
    })""")
    zaehler, _, nenner = roh["mitte"].partition("/")
    roh["fest"] = int(zaehler) if zaehler else None
    roh["gesamt"] = int(nenner) if nenner else None
    return roh


def _antworte(page, base, feld_id, wert, ersetzt=None):
    """Antwort über den Server schreiben, dann die Seite aktualisieren lassen. `refresh()` gibt ein
    Promise zurück, das Playwright abwartet — kein Pollen, kein Schlafen."""
    fall = page.evaluate("FALL")
    status, body = _req(base, "POST", f"/fall/{fall}/event", _event(feld_id, wert, ersetzt))
    assert status in (200, 201), (status, body)
    page.evaluate("() => refresh()")


def _event_id(base, page, feld_id):
    fall = page.evaluate("FALL")
    status, body = _req(base, "GET", f"/fall/{fall}/feld/{feld_id}/warum")
    assert status == 200, (status, body)
    return body["justification"]["event_id"]


# --- (1) Der Nenner --------------------------------------------------------------------------

def test_nenner_ist_die_zahl_der_noch_offenen_fragen(seite_factory, base, fall_fragen):
    """Der Ausgangspunkt: noch nichts beantwortet, und trotzdem steht dort ein voller Nenner.

    Vorher war `stand.felder` an dieser Stelle LEER — die Mitte blieb leer und die Leiste stand auf
    max=1/value=0. Es gab also nie eine Aussage über die Gesamtmenge, nicht bloß eine ungenaue."""
    page = seite_factory()
    a = _anzeige(page)
    offen = len(fall_fragen)
    assert offen > 1, "Diese Scheibe stellt nur eine Frage — dann misst der Test nichts."
    assert a["fest"] == 0, f"Zähler steht nicht auf 0: {a['mitte']!r}"
    assert a["gesamt"] == offen, (
        f"Nenner {a['gesamt']} ≠ offene Fragen {offen}. Der Nenner muss die Fragen zählen, die noch "
        f"zu beantworten sind — nicht die Felder, die schon ein Event haben.")
    assert a["max"] == offen, f"Die Leiste läuft gegen max={a['max']}, nicht gegen {offen}."
    assert a["wert"] == 0
    assert f"0 von {offen} Fragen beantwortet" in a["text"], a["text"]


def test_nenner_waechst_nicht_mit_dem_zaehler_mit(seite_factory, base, fall_fragen):
    """Julius' eigentlicher Punkt. Eine Antwort, die weder einen Block auf- noch zumacht: der
    Zähler steigt um 1, der Nenner bleibt STEHEN. Vorher stiegen beide, und genau daraus entstand
    2/2 → 3/3 → 4/4."""
    treffer = _sonde(base, list(reversed(fall_fragen)), lambda v, n: n == v - 1)
    if treffer is None:
        pytest.skip("Keine Frage gefunden, deren Antwort genau sich selbst aus der Queue nimmt.")
    feld, wert, _, _ = treffer

    page = seite_factory()
    vor = _anzeige(page)
    _antworte(page, base, feld, wert)
    nach = _anzeige(page)

    assert nach["fest"] == vor["fest"] + 1, (
        f"Der Zähler ist nicht gestiegen: {vor['mitte']!r} → {nach['mitte']!r}")
    assert nach["gesamt"] == vor["gesamt"], (
        f"Der Nenner ist mitgewachsen: {vor['mitte']!r} → {nach['mitte']!r}. Genau das war der "
        f"Befund — ein Nenner, der dem Zähler folgt, sagt nichts.")
    assert nach["gesamt"] > nach["fest"], (
        f"{nach['mitte']} liest sich als „fertig“, obwohl noch Fragen offen sind.")
    assert float(nach["anteil"]) < 1, (
        f"Der Ring steht auf --anteil={nach['anteil']}, also voll, nach einer einzigen Antwort.")
    assert "entfallen" not in nach["text"] and "dazugekommen" not in nach["text"], (
        f"Die Gesamtzahl hat sich nicht geändert, die Anzeige behauptet aber eine Änderung: "
        f"{nach['text']!r}")


def test_eine_antwort_die_bloecke_abschaltet_senkt_die_gesamtzahl_und_sagt_es(
        seite_factory, base, fall_fragen):
    """Die Gesamtzahl ist keine Konstante: ein Screening-Gate mit „nein" streicht ganze Regeln, und
    mit ihnen deren Fragen. Der Balken springt dabei VOR — nicht weil mehr geschafft wurde, sondern
    weil weniger zu tun ist. Unerklärt ist das ein Rätsel, deshalb muss die Anzeige es benennen."""
    treffer = _sonde(base, fall_fragen, lambda v, n: n < v - 1)
    if treffer is None:
        pytest.skip("Kein Feld gefunden, dessen Antwort mehr als sich selbst aus der Queue nimmt.")
    feld, wert, sonde_vor, sonde_nach = treffer

    page = seite_factory()
    vor = _anzeige(page)
    _antworte(page, base, feld, wert)
    nach = _anzeige(page)

    assert nach["gesamt"] < vor["gesamt"], (
        f"{feld}={wert!r} nimmt laut Sonde {sonde_vor - sonde_nach} Fragen aus der Queue, die "
        f"Anzeige bleibt aber bei {vor['mitte']!r} → {nach['mitte']!r}. Dann steht dort eine "
        f"Gesamtzahl, die beim Laden eingefroren wurde.")
    weg = vor["gesamt"] - nach["gesamt"]
    assert f"{weg} Fragen entfallen" in nach["text"], (
        f"Die Gesamtzahl fiel um {weg}, die Anzeige sagt aber nichts dazu: {nach['text']!r}. Ein "
        f"Balken, der ohne Erklärung vorspringt, ist genau das Irritierende.")
    assert nach["max"] == nach["gesamt"], (
        f"Die Leiste läuft weiter gegen den alten Nenner (max={nach['max']}, "
        f"angezeigt {nach['gesamt']}).")


def test_eine_korrektur_die_bloecke_aufmacht_hebt_die_gesamtzahl_und_sagt_es(
        seite_factory, base, fall_fragen):
    """Die Gegenrichtung, und der unangenehmere Fall: der Nutzer nimmt sein „nein" zurück, die
    Fragen kommen zurück, der Balken springt ZURÜCK. Ein Fortschritt, der nur vorwärts gehen darf,
    müsste hier lügen — er würde Arbeit als erledigt zeigen, die gerade erst entstanden ist.

    Ob dieser Fall überhaupt zu haben ist, entscheidet die SONDE über HTTP — nie die Anzeige, die
    hier geprüft wird. Sonst verwandelte ein kaputter Nenner den Test in ein `skip` statt in ein
    Rot: gemessen bei der Mutationsprobe, die Nenner-Mutation M1 tat genau das."""
    treffer = _sonde(base, fall_fragen, lambda v, n: n < v - 1)
    if treffer is None:
        pytest.skip("Kein Feld gefunden, dessen Antwort mehr als sich selbst aus der Queue nimmt.")
    feld, wert, _, _ = treffer
    gegenwerte = [w for w in _kandidatenwerte(
        next(q for q in fall_fragen if q["feld_id"] == feld)) if w != wert]
    if not gegenwerte:
        pytest.skip(f"{feld} hat keinen zweiten Wert, mit dem sich die Antwort zurücknehmen ließe.")
    gegenwert, erwartet_dazu = _sonde_korrektur(base, feld, wert, gegenwerte)
    if gegenwert is None:
        pytest.skip(f"Keine Korrektur von {feld} bringt Fragen zurück in die Queue.")

    page = seite_factory()
    _antworte(page, base, feld, wert)
    eng = _anzeige(page)
    # Die Korrektur ersetzt das bestehende Event — derselbe Weg, den korrigiereBestaetigt() geht.
    _antworte(page, base, feld, gegenwert, ersetzt=_event_id(base, page, feld))
    weit = _anzeige(page)

    assert weit["gesamt"] > eng["gesamt"], (
        f"Die Korrektur bringt laut Sonde {erwartet_dazu} Fragen zurück, die Anzeige bleibt aber "
        f"bei {eng['mitte']!r} → {weit['mitte']!r}.")
    dazu = weit["gesamt"] - eng["gesamt"]
    assert f"{dazu} Fragen dazugekommen" in weit["text"], (
        f"Die Gesamtzahl stieg um {dazu}, die Anzeige sagt aber nichts dazu: {weit['text']!r}")
    assert weit["max"] == weit["gesamt"] and weit["wert"] == weit["fest"], (
        f"Leiste und Zahl gehen auseinander: max={weit['max']}, wert={weit['wert']}, "
        f"angezeigt {weit['mitte']}")
    assert float(weit["anteil"]) < float(eng["anteil"]), (
        "Der Ring ist nicht zurückgegangen, obwohl Fragen dazugekommen sind — dann behauptet er "
        "einen Fortschritt, den es nicht mehr gibt.")


def test_ein_vorlaeufiger_wert_zaehlt_genau_einmal(seite_factory, base):
    """Der Nenner ist `fest + offen`, NICHT `felder.length + offen`. Ein vorläufiger Wert steht in
    beiden Quellen: in `stand.felder` (er hat ein Event) und in der Frage-Queue (der Traverser hält
    `vorlaeufig` für unbeantwortet, _unbeantwortet()). Aus `felder.length + offen` würde er zweimal
    gezählt und die Gesamtzahl stiege, obwohl der Nutzer nichts beantwortet hat — ein Vorschlag ist
    keine zusätzliche Frage, er ist eine Antwort, die noch aussteht.

    Der Vorschlag kommt hier aus der Vorjahr-Übernahme, dem Knopf in der Import-Box. Kein LLM."""
    page = seite_factory()
    vor = _anzeige(page)
    n = _vorlaeufiges_feld(base, page.evaluate("FALL"))
    assert n > 0, "Die Vorjahr-Übernahme hat kein Feld übernommen — dann prüft der Test nichts."
    page.evaluate("() => refresh()")
    nach = _anzeige(page)

    zustaende = page.evaluate(
        "() => Object.values(STAND.felder).map(f => f.zustand)")
    assert zustaende.count("vorlaeufig") == n, (
        f"Erwartet {n} vorläufige Felder im Stand, gefunden {zustaende}")
    assert nach["fest"] == vor["fest"], (
        f"Ein Vorschlag ist keine Bestätigung — der Zähler ist trotzdem gestiegen: "
        f"{vor['mitte']!r} → {nach['mitte']!r}")
    assert nach["gesamt"] == vor["gesamt"], (
        f"Die Gesamtzahl stieg um {nach['gesamt'] - vor['gesamt']}, obwohl {n} Vorschläge nur "
        f"bestehende Fragen vorbelegen. Sie werden doppelt gezählt: einmal aus stand.felder, "
        f"einmal aus der Frage-Queue. {vor['mitte']!r} → {nach['mitte']!r}")
    assert "dazugekommen" not in nach["text"], (
        f"Die Anzeige meldet neue Fragen, obwohl nur vorbelegt wurde: {nach['text']!r}")


def test_ein_netzfehler_meldet_keinen_fortschritt(seite_factory, base):
    """Die Gegenrichtung zum Nenner: kommt /fragen nicht durch, ist die Zahl der offenen Fragen
    UNBEKANNT — nicht null. `fest + 0` läse sich als „alles beantwortet"; ein Netzfehler würde den
    Balken vollaufen lassen. Also die zuletzt bekannte Zahl behalten.

    Gemessen, nicht angenommen: die Mutationsprobe zeigte, dass genau diese Zeile ohne diesen Test
    unbemerkt zurückgedreht werden kann."""
    page = seite_factory()
    vor = _anzeige(page)
    assert vor["gesamt"] and vor["gesamt"] > 1

    page.route("**/fragen", lambda route: route.abort())
    page.evaluate("() => refresh()")
    nach = _anzeige(page)

    assert nach["gesamt"] == vor["gesamt"], (
        f"Nach dem Netzfehler steht dort {nach['mitte']!r} statt {vor['mitte']!r} — eine "
        f"gescheiterte Abfrage darf keine Gesamtzahl setzen.")
    assert float(nach["anteil"]) < 1, (
        f"Der Ring ist durch einen Netzfehler vollgelaufen (--anteil={nach['anteil']}).")
    assert nach["max"] == vor["max"], f"Die Leiste läuft jetzt gegen max={nach['max']}"
    assert not page.evaluate("() => document.getElementById('netz-banner').hidden"), (
        "Der Nutzer erfährt nichts vom Netzfehler.")


# --- (2) Das Panel ---------------------------------------------------------------------------

def _kasten(page, sel):
    b = page.query_selector(sel).bounding_box()
    assert b is not None, f"{sel} hat keine Bounding-Box (nicht sichtbar)"
    return b


def test_panel_liegt_auf_breitem_schirm_rechts_neben_dem_inhalt(seite_factory):
    """Julius: „als eigenes panel rechts … als in die column der felder". Also: rechts NEBEN der
    Frage-Karte, nicht unter ihr — und ohne sie zu überlappen."""
    page = seite_factory(breite=1280, hoehe=900)
    frage = _kasten(page, "#wegpunkt")
    berater = _kasten(page, "#berater")

    assert berater["x"] >= frage["x"] + frage["width"] - 1, (
        f"Der Berater beginnt bei x={berater['x']}, die Frage-Karte endet bei "
        f"{frage['x'] + frage['width']} — er steht nicht daneben, sondern überlappt sie.")
    assert berater["y"] < frage["y"] + frage["height"], (
        f"Der Berater beginnt bei y={berater['y']}, unterhalb der Frage-Karte "
        f"(die endet bei {frage['y'] + frage['height']}) — er hängt also weiter darunter.")
    assert berater["width"] >= 280, f"Panel zu schmal zum Lesen: {berater['width']}"
    assert page.evaluate("document.documentElement.scrollWidth") <= 1280, "Seitwärts-Scrollen"
    # Die Belegt-Liste ist ebenfalls Inhalt und gehört in die linke Spalte, nicht unter das Panel.
    belegt = _kasten(page, "#belegt-liste")
    assert belegt["x"] < berater["x"], (
        f"Die Belegt-Liste ({belegt['x']}) ist in die Berater-Spalte ({berater['x']}) gerutscht.")


def test_panel_bleibt_beim_scrollen_stehen(seite_factory, base, fall_fragen):
    """„Die KI sollte immer offen sein" (Julius 2026-08-14) — in einer eigenen Spalte ist das erst
    wahr, wenn das Panel beim Scrollen sichtbar bleibt. Sonst hat der Umbau nach rechts den Berater
    aus dem Bild geschoben, sobald der Nutzer zur Belegt-Liste hinunterscrollt.

    Das ist zugleich die Probe auf `grid-row: 1 / span 20` im CSS: ein sticky Element bewegt sich
    nur innerhalb seiner Grid-Area. Mit `1 / -1` — der naheliegenden Schreibweise — bezieht sich
    -1 auf das Ende des EXPLIZITEN Rasters, das hier keine Zeilen hat; die Area bliebe eine Zeile
    hoch und das Panel scrollte weg."""
    page = seite_factory(breite=1280, hoehe=480)
    # Erst Inhalt schaffen: eine frische Seite ist kürzer als der Bildschirm, dann gibt es nichts
    # zu scrollen und der Test bestätigte nur sich selbst. Zwölf beantwortete Felder füllen die
    # Belegt-Liste in der LINKEN Spalte — genau die Situation, in der das Panel wegrutschen würde.
    fall = page.evaluate("FALL")
    for q in [q for q in fall_fragen if q.get("beispielwert") is not None][:12]:
        _req(base, "POST", f"/fall/{fall}/event", _event(q["feld_id"], q["beispielwert"]))
    page.evaluate("() => refresh()")
    hoehe = page.evaluate("document.documentElement.scrollHeight")
    assert hoehe > 780, (
        f"Die Seite ist mit {hoehe}px gar nicht scrollbar — dann misst dieser Test nichts.")
    oben = _kasten(page, "#berater")["y"]
    page.evaluate("window.scrollTo(0, 300)")
    page.wait_for_timeout(150)
    unten = _kasten(page, "#berater")["y"]

    assert unten > oben - 300 + 1, (
        f"Der Berater ist beim Scrollen um {oben - unten:.0f}px mitgewandert (also voll "
        f"mitgescrollt) — er klebt nicht. Das Panel ist damit nicht „immer da“.")
    assert unten >= 0, f"Der Berater ist aus dem Bild gescrollt (y={unten})."


def test_panel_rutscht_auf_schmalem_schirm_unter_den_inhalt(seite_factory):
    """Auflage: auf schmalen Schirmen zurück UNTER den Inhalt, nicht abgeschnitten. 360px ist die
    Breite, die tests/test_ui_responsive.py durchgängig misst."""
    page = seite_factory(breite=360, hoehe=640)
    frage = _kasten(page, "#wegpunkt")
    berater = _kasten(page, "#berater")

    assert berater["y"] >= frage["y"] + frage["height"] - 1, (
        f"Der Berater (y={berater['y']}) liegt nicht unter der Frage-Karte "
        f"(endet bei {frage['y'] + frage['height']}) — bei 360px gibt es keine zweite Spalte.")
    assert berater["x"] >= 0 and berater["x"] + berater["width"] <= 360, (
        f"Der Berater ragt aus dem Bild: x={berater['x']}, Breite={berater['width']}")
    assert page.evaluate("document.documentElement.scrollWidth") <= 360, (
        "Horizontales Scrollen bei 360px")
    assert page.query_selector("#chat-text").bounding_box()["height"] >= 44, "Tap-Ziel Eingabefeld"


def test_die_chat_sperre_greift_auch_im_zweispaltigen_bild(seite_factory):
    """Die Auflage, die man nur durch Nachsehen einlöst: chatSperren() setzt `inert` über feste IDs
    (#wegpunkt, #verstanden, #belegt-liste, .vorjahr-box). Läge einer dieser Bereiche nach dem
    Umbau woanders — oder umschlösse einer von ihnen den Berater —, sperrte ein laufender KI-Aufruf
    sein eigenes Eingabefeld mit, und der Nutzer käme ohne Neuladen nicht mehr heraus.

    Deshalb ist der Umbau reines CSS. Hier wird das nachgemessen statt angenommen."""
    page = seite_factory(breite=1280, hoehe=900)
    page.evaluate("() => chatSperren(true)")
    gesperrt = page.evaluate("""() => ({
        wegpunkt: document.getElementById('wegpunkt').hasAttribute('inert'),
        verstanden: document.getElementById('verstanden').hasAttribute('inert'),
        belegt: document.getElementById('belegt-liste').hasAttribute('inert'),
        importbox: document.querySelector('.vorjahr-box').hasAttribute('inert'),
        chattext: document.getElementById('chat-text').disabled,
    })""")
    assert all(gesperrt.values()), f"Die Sperre greift ins Leere: {gesperrt}"
    # Der Berater darf NICHT über einen inerten Vorfahren mitgesperrt sein: dann wäre sein
    # Eingabefeld während des Aufrufs unerreichbar, obwohl chatSperren() es gar nicht sperrt.
    assert page.evaluate(
        "() => !document.getElementById('berater').closest('[inert]')"), (
        "Der Berater hängt unter einem gesperrten Bereich — der laufende KI-Aufruf würde sein "
        "eigenes Panel mit sperren.")
    page.evaluate("() => chatSperren(false)")
    assert page.evaluate(
        "() => !document.querySelector('#wegpunkt[inert], #verstanden[inert]')"), "Nicht freigegeben"
