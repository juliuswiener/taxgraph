"""Die Sperre bietet die Angabe an, die sie ausloest (backlog
partnerangaben-nach-umstellung-auf-einzel-sackgasse, 2026-09-26).

WER IN EINZELVERANLAGUNG UMSTELLT, KOMMT SO NICHT HERAUS. Ein Paar fuellt gemeinsam aus,
traegt Kapitalertraege des Partners ein, stellt danach auf getrennte Erklaerung um. Der Ring
sperrt (partner_konsistenz_offen, § 26b: ein Einzelveranlagter hat keinen mitzuveranlagenden
Partner), die Anzeige nennt den Grund — und keinen Weg zurueck.

DIE URSACHE IST NICHT DER FEHLENDE LINK ALLEIN. Gemessen 2026-09-26 im Browser (Chromium,
echter Server): die Belegt-Liste „Schon beantwortet" ist im Sperrzustand SICHTBAR (464x13066 px,
187 Zeilen, pointer-events auto) — und der Klick auf die Zeile des ausloesenden Feldes ist TOT.
Beide Serveraufrufe antworten dabei 200 (/warum und /feld/<fid>/frage); der Klick endet vorher in
app.js. Dieselbe Zeile bei grund='bestaetigt' funktioniert. Der Weg ist also nicht da und kaputt,
er ist im Sperrzustand abgeschnitten.

WARUM ABGESCHNITTEN: `veranlagung=einzel` setzt `relevanz['p2_festzusetzung_zusammen']` auf
`ausgeschlossen` (vorher `unentschieden`) — und an genau dieser regel_id haengen ALLE ACHT
Partnerfelder (gemessen, s. PARTNER_FELDER unten). `korrigiereBestaetigt` sperrt bei diesem
Status und kehrt zurueck, BEVOR es `KORREKTUR_FID` setzt. Die Sperre schaltet damit die Frage ab,
deren Korrektur sie aufheben wuerde.

DIESER TEST PRUEFT DEN WEG, DEN DIE ENTSCHEIDUNG VERLANGT (Weg A,
decisions/nach-wechsel-auf-einzel-bleibt-die-sperre-mit-rueckweg.md): der Sperrgrund traegt die
ausloesende feld_id bis in die Anzeige, und die Sperranzeige oeffnet die Frage zu DIESEM Feld
ueber den bestehenden Einzelfeld-Weg. `/fragen` bleibt unveraendert — dort gehoert das Feld
nicht hin, es ist beantwortet. Und `korrigiereBestaetigt` bleibt unveraendert: dass die Sperre
dort weiter greift, ist die Gegenprobe unten, keine Behauptung.

NULL LLM. Der Browserteil braucht Playwright und ueberspringt ohne ihn."""
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
for sub in ("produkt/haut", "produkt/store", "produkt/konsistenz", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API              # noqa: E402
import audit                   # noqa: E402
import partner_check as PC     # noqa: E402
import server as SRV           # noqa: E402

try:
    from playwright.sync_api import sync_playwright  # noqa: E402
except ImportError:
    sync_playwright = None

# Menschenplausible Werte je Feld, typkonform gegen die Bindung (store.py:_pruefe_typ_konformitaet).
# rentner_grad_der_behinderung_partner ist int-GdB, das Merkzeichen ein bool, der Rest cent.
WERT = {
    "rentner_grad_der_behinderung_partner": 60,
    "rentner_hilflos_blind_taubblind_partner": True,
    "kap_kapitalertraege_partner": 500000,
    "kap_gewinn_aktien_partner": 100000,
    "kap_gewinn_sonstige_partner": 100000,
    "kap_verlust_aktien_partner": 100000,
    "kap_verlust_sonstige_partner": 100000,
    "rentner_jahresrente_partner": 2000000,
}
# rentner_jahresrente_partner liegt NUR in rentner_gesamt (gemessen: gesamt fuehrt 7 von 8).
FELD_SCHEIBE = [(f, "rentner_gesamt" if f == "rentner_jahresrente_partner" else "gesamt")
                for f in PC.PARTNER_FELDER]


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


def _req(base_url, method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base_url + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fid, wert):
    return {"feld_id": fid, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fid}"}}


def _sperrfall(base_url, fall_id, feld, wert, scheibe="gesamt"):
    """Der Weg des Nutzers: gemeinsam veranlagen, Partnerangabe bestaetigen, DANN auf einzel
    umstellen (nur `veranlagung` per ersetzt=, wie im Ticket). Gibt (ergebnis, feld_event_id)."""
    st, _ = _req(base_url, "POST", "/fall",
                 {"scheibe": scheibe, "veranlagungszeitraum": 2025, "fall_id": fall_id})
    assert st == 201, (scheibe, st)
    st, r = _req(base_url, "POST", f"/fall/{fall_id}/event", _laie("veranlagung", "zusammen"))
    assert st == 201, (st, r)
    veranlagung_ev = r["event_id"]
    st, r = _req(base_url, "POST", f"/fall/{fall_id}/event", _laie(feld, wert))
    assert st == 201, (feld, st, r)
    feld_ev = r["event_id"]
    st, r = _req(base_url, "POST", f"/fall/{fall_id}/event",
                 dict(_laie("veranlagung", "einzel"), ersetzt=veranlagung_ev))
    assert st == 201, ("Umstellung abgelehnt", st, r)
    st, erg = _req(base_url, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200, (st, erg)
    return erg, feld_ev


# ---- AK: der Sperrgrund traegt die ausloesende Angabe --------------------------------------

def test_sperrgrund_traegt_die_ausloesende_feld_id(base):
    """ROT an HEAD: die Sperre liefert grund und Satz, aber kein Feld — der Nutzer erfaehrt,
    DASS etwas nicht passt, nicht WORAN es haengt.

    `partner_check.partner_ohne_zusammen()` liefert feld_id, wert und einen fertigen Satz je
    Widerspruch (partner_check.py:66); `_an_gesamt_sperrgrund` behielt davon nur den Namen."""
    erg, _ = _sperrfall(base, "sf1", "kap_kapitalertraege_partner", 500000)
    assert erg["grund"] == "partner_konsistenz_offen", erg
    felder = erg.get("sperr_felder")
    # ALLE VIER Schluessel, die `partner_check.partner_ohne_zusammen()` liefert (partner_check.py:66)
    # — nicht nur die feld_id. Der Satz gehoert dazu: er ist der Text, den die Anzeige am Feld
    # vorliest, und er nennt den Betrag nicht (die Anzeige zeigt ihn, nicht der Satz).
    assert felder == [{
        "feld_id": "kap_kapitalertraege_partner",
        "wert": 500000,
        "veranlagung": "einzel",
        "grund": "Du hast etwas bei „Kapitaleinkünfte des Partners“ eingetragen, aber keine "
                 "Zusammenveranlagung gewählt. Partnerbezogene Angaben sind nur bei gemeinsamer "
                 "Veranlagung möglich. Bitte prüfe deine Angaben.",
    }], (
        f"Der Sperrgrund nennt die ausloesende Angabe nicht. Gemessen: {felder!r} — "
        f"erwartet wird genau kap_kapitalertraege_partner.")


@pytest.mark.parametrize("feld,scheibe", FELD_SCHEIBE, ids=[f for f, _ in FELD_SCHEIBE])
def test_jedes_partnerfeld_wird_genauso_gemeldet(base, feld, scheibe):
    """AK4 der Triage: alle acht Felder aus PARTNER_FELDER verhalten sich gleich.

    Gemessen 2026-09-26: 15 von 15 Faellen (acht Felder, rentner_jahresrente_partner nur in
    rentner_gesamt) sperren mit partner_konsistenz_offen, und alle acht tragen dieselbe
    regel_id p2_festzusetzung_zusammen. Ein Fix, der nur das gemessene Beispiel kennt, laesst
    sieben Felder in der Sackgasse."""
    erg, _ = _sperrfall(base, "sf_" + feld[:18], feld, WERT[feld], scheibe)
    assert erg["grund"] == "partner_konsistenz_offen", (feld, scheibe, erg)
    assert [w["feld_id"] for w in erg.get("sperr_felder") or []] == [feld], erg


def test_sperrfeld_ist_ueber_den_einzelfeld_weg_erreichbar(base):
    """Die zweite Haelfte: die gemeldete Kennung muss auch eine Frage ergeben.

    Der Weg existiert seit e7f9f2a (`GET /fall/<id>/feld/<fid>/frage`, api.py:363) und
    antwortet im Sperrzustand 200 — gemessen mit dem echten Server, waehrend der Klick in der
    Oberflaeche tot war. Die Anzeige fuehrt auf DIESEN Weg; hier steht, dass er traegt."""
    erg, _ = _sperrfall(base, "sf2", "kap_kapitalertraege_partner", 500000)
    fid = erg["sperr_felder"][0]["feld_id"]
    st, r = _req(base, "GET", f"/fall/sf2/feld/{fid}/frage")
    assert st == 200, (st, r)
    assert r["frage"]["feld_id"] == fid, r["frage"]
    assert r["frage"]["typ"] == "cent", r["frage"]


def test_korrektur_hebt_die_sperre_auf(base):
    """Der Weg muss auch ankommen: die Korrektur der gemeldeten Angabe loest die Sperre.

    Ohne diese Haelfte waere „die Sperre bietet ihr Feld an" eine Anzeige-Zusage ohne Wirkung.
    Mit `ersetzt` (Auflage B, store.py:375) — ohne sie antwortet der Store 422, gemessen."""
    erg, feld_ev = _sperrfall(base, "sf3", "kap_kapitalertraege_partner", 500000)
    st, r = _req(base, "POST", "/fall/sf3/event",
                 dict(_laie("kap_kapitalertraege_partner", 0), ersetzt=feld_ev))
    assert st == 201, ("Korrektur abgelehnt", st, r)
    st, erg2 = _req(base, "GET", "/fall/sf3/ergebnis")
    assert st == 200 and erg2["grund"] == "input_kegel_nicht_bestaetigt", (
        f"Die Sperre steht nach der Korrektur immer noch — dann waere der angebotene Weg "
        f"keiner. grund={erg2['grund']!r}")


# ---- Gegenprobe: ohne Sperre aendert sich nichts ------------------------------------------

def test_ohne_sperre_keine_sperr_felder(base):
    """Die Kontrolle. Bei Zusammenveranlagung ist die Partnerangabe legitim — dann gibt es
    keinen Widerspruch, und die Antwort darf keine ausloesenden Felder behaupten."""
    st, _ = _req(base, "POST", "/fall",
                 {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "sf4"})
    assert st == 201
    for fid, w in (("veranlagung", "zusammen"), ("kap_kapitalertraege_partner", 500000)):
        st, _ = _req(base, "POST", "/fall/sf4/event", _laie(fid, w))
        assert st == 201, fid
    st, erg = _req(base, "GET", "/fall/sf4/ergebnis")
    assert st == 200
    # Gepinnt auf den GEMESSENEN Grund, nicht auf „irgendetwas Sperrendes": dieser Fall ist nur
    # zwei Antworten weit, der Ring steht aus einem anderen Grund still (flag_konsistenz_offen).
    assert erg["grund"] == "flag_konsistenz_offen", erg
    # Positivkontrolle fuer die Zeile darunter: der Fall hat wirklich eine Partnerangabe
    # bestaetigt stehen. Ohne sie bewiese ein leeres `sperr_felder` nur, dass nichts da ist.
    assert erg["grund"] != "partner_konsistenz_offen", erg
    # GEPINNT auf [] — und das ist keine Selbstverstaendlichkeit: `flag_konsistenz_offen` kommt
    # ebenfalls aus `_an_gesamt_sperrgrund`, der Zweig laeuft also, und `sperrgrund_felder` gibt
    # fuer JEDEN Grund ausser dem Widerspruch eine leere Liste zurueck. Genau das ist der Vertrag:
    # der Schluessel steht an jeder Guard-Antwort, benennt aber nur beim Widerspruch Felder.
    assert erg["sperr_felder"] == [], (
        f"Ohne Widerspruch darf kein ausloesendes Feld gemeldet werden: {erg.get('sperr_felder')!r}")


# ---- Der Weg im Browser, und die Sperre, die bleiben muss ----------------------------------

@pytest.fixture
def seite(base):
    if sync_playwright is None:
        pytest.skip("playwright fehlt")
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True,
                                    args=["--no-sandbox", "--disable-setuid-sandbox"])
        page = browser.new_page(viewport={"width": 1100, "height": 1400})
        page.fehler = []
        page.on("pageerror", lambda e: page.fehler.append(str(e)))
        page.goto(base)
        page.wait_for_load_state("networkidle")
        yield page, base
        browser.close()


def test_sperranzeige_oeffnet_die_frage_zu_ihrem_feld(seite):
    """ROT an HEAD: der Klick auf die Sperranzeige oeffnet die Frage zu genau der Angabe, die
    die Sperre ausloest — und zwar auf dem Weg, der im Sperrzustand traegt.

    GEMESSEN 2026-09-26: die Belegt-Liste ist im Sperrzustand sichtbar, ihr Klick aber tot —
    korrigiereBestaetigt kehrt beim Status `ausgeschlossen` zurueck, bevor es KORREKTUR_FID
    setzt. Dieser Weg geht daran vorbei, und das ist Absicht: die Sperre bleibt fuer alles
    andere scharf (s. die Gegenprobe darunter)."""
    page, base = seite
    erg, _ = _sperrfall(base, "sf5", "kap_kapitalertraege_partner", 500000)
    assert erg["grund"] == "partner_konsistenz_offen"
    page.evaluate("FALL = 'sf5';")
    page.evaluate("document.getElementById('login').hidden = true;")
    page.evaluate("document.getElementById('start').hidden = true;")
    page.evaluate("document.getElementById('flow').hidden = false;")
    page.evaluate("(async () => { await refresh(); })()")
    page.wait_for_timeout(500)
    # `refresh()` bringt den Sperr-Screen NUR bei leerer Queue vorn (`if (fragen.length === 0)`),
    # und in dieser Szene ist sie NICHT leer: der Fall ist zwei Antworten weit (gemessen 277 offene
    # Fragen). Ihn auf dem Navigationsweg zu erreichen hiesse, 277 Fragen zu beantworten. Gerufen
    # wird deshalb die Anzeige selbst — `refresh()` ist nur fuer STAND noetig, den `zeigeFrage`
    # braucht. Dass es WIRKLICH diese Anzeige ist, die `#fertig` vorn bringt, steht darunter.
    page.evaluate("(async () => { await zeigeErgebnis(); })()")
    page.wait_for_timeout(300)

    assert not page.evaluate("document.getElementById('fertig').hidden"), (
        "Der Sperr-Screen ist nicht vorn — der Test misst sonst einen anderen Zustand.")

    # Die Sperranzeige muss die Angabe als Weg anbieten: genau EIN Bedienelement mit der
    # Feld-Kennung, das die Frage oeffnet.
    knoepfe = page.evaluate("""() => {
      return [...document.querySelectorAll('#ergebnis [data-sperr-feld]')]
        .map(e => e.getAttribute('data-sperr-feld'));
    }""")
    assert knoepfe == ["kap_kapitalertraege_partner"], (
        f"Die Sperranzeige bietet die ausloesende Angabe nicht an (gefunden: {knoepfe!r}).")

    page.click("#ergebnis [data-sperr-feld]")
    page.wait_for_timeout(800)
    nach = page.evaluate("""({
      wegpunkt_hidden: document.getElementById('wegpunkt').hidden,
      frage: document.getElementById('frage').textContent,
      aktuell: (typeof AKTUELL !== 'undefined' && AKTUELL) ? AKTUELL.feld_id : null,
      korrektur_fid: (typeof KORREKTUR_FID !== 'undefined') ? KORREKTUR_FID : null,
      banner: (document.getElementById('netz-banner') || {}).textContent || ''})""")
    assert nach["wegpunkt_hidden"] is False, nach
    assert nach["aktuell"] == "kap_kapitalertraege_partner", nach
    assert nach["korrektur_fid"] == "kap_kapitalertraege_partner", (
        f"Ohne KORREKTUR_FID schickt `bestaetigen()` kein `ersetzt` — der Store antwortet "
        f"dann 422 (Auflage B). Gemessen: {nach!r}")
    assert nach["banner"].strip() == "", nach
    assert page.fehler == [], page.fehler


def test_normale_korrektur_bleibt_im_sperrzustand_gesperrt(seite):
    """GEGENPROBE, und der Grund, warum dieser Weg nicht durch `korrigiereBestaetigt` laeuft.

    `korrigiereBestaetigt` sperrt bei `relevanz.status == "ausgeschlossen"` (app.js, seit
    e7f9f2a „NUR ausgeschlossen SPERRT"). Diese Sperre bleibt unveraendert: derselbe Klick
    ueber die Belegt-Liste muss im Sperrzustand weiterhin abweisen. Waere sie aufgeweicht,
    waere „ich habe die Sperre nicht angefasst" eine Behauptung ohne Beleg."""
    page, base = seite
    erg, _ = _sperrfall(base, "sf6", "kap_kapitalertraege_partner", 500000)
    assert erg["grund"] == "partner_konsistenz_offen"
    page.evaluate("FALL = 'sf6';")
    page.evaluate("document.getElementById('login').hidden = true;")
    page.evaluate("document.getElementById('start').hidden = true;")
    page.evaluate("document.getElementById('flow').hidden = false;")
    page.evaluate("(async () => { await refresh(); })()")
    page.wait_for_timeout(500)

    # Der Relevanz-Status, an dem die Sperre haengt — er ist der Grund, nicht ein Nebenumstand.
    rel = page.evaluate(
        "JSON.stringify((STAND.relevanz || {})['p2_festzusetzung_zusammen'])")
    assert json.loads(rel)["status"] == "ausgeschlossen", rel

    geklickt = page.evaluate("""() => {
      const li = [...document.querySelectorAll('#belegt-liste li')]
        .find(x => x.querySelector('.z-name')
                   && x.querySelector('.z-name').title === 'kap_kapitalertraege_partner');
      if (!li) return false;
      li.click();
      return true;
    }""")
    assert geklickt, "Die Belegt-Zeile des Feldes fehlt — der Test misst sonst nichts."
    page.wait_for_timeout(800)
    nach = page.evaluate("""({
      wegpunkt_hidden: document.getElementById('wegpunkt').hidden,
      aktuell: (typeof AKTUELL !== 'undefined' && AKTUELL) ? AKTUELL.feld_id : null,
      korrektur_fid: (typeof KORREKTUR_FID !== 'undefined') ? KORREKTUR_FID : null,
      banner: (document.getElementById('netz-banner') || {}).textContent || ''})""")
    # Gemessen wird AKTUELL, nicht `#wegpunkt.hidden`: in dieser Szene ist die Queue NICHT leer
    # (277 offene Fragen), also steht ohnehin eine Frage im Bild. „Sichtbar" hiesse hier nichts.
    # Was zaehlt: die Korrektur des Partnerfeldes ist NICHT durchgelassen worden.
    assert nach["aktuell"] != "kap_kapitalertraege_partner", (
        f"korrigiereBestaetigt ist im Sperrzustand durchgelassen worden — die Sperre wurde "
        f"aufgeweicht. Gemessen: {nach!r}")
    assert nach["korrektur_fid"] is None, nach
    assert nach["banner"].strip() == (
        "Diese Frage ist durch eine andere Antwort entfallen und lässt sich nicht mehr ändern. "
        "Willst du sie zurückholen, ändere die Antwort, die sie abgeschaltet hat."), (
        f"Die Abweisung klingt anders als gemessen: {nach['banner']!r}")
