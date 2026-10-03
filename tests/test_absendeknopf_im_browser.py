"""P3.2c — der Absendeknopf im Browser: bislang kann die Software prüfen, aber niemand im
Browser kommt an POST /fall/{id}/einreichen heran (Julius: "absendeknopf ist luecke").

Der Knopf löst NUR die lokale checkESt-Prüfung aus, sendet nichts ans Finanzamt (das bleibt
CLI-only, elster/versand.py). test_einreichen.py und test_einreichen_durchstich.py rufen den
Endpunkt direkt, nie über den Browser — kein bestehender Test würde rot, wenn der Knopf fehlt
oder falsch verdrahtet ist. Dieser hier ruft ihn per echtem Playwright-Klick.

Kern des Auftrags (api.py:einreichen(), s. dortige Kommentare): drei verschiedene Fälle landen
auf 422 — kein_pruefmodul_fuer_vz, plausibilitaet_verletzt, rc_kein_plausibilitaetsverdikt.
Nur der mittlere ist "geprüft und beanstandet"; die beiden anderen (und alle 409/503-Fälle)
sind "NICHT geprüft" — der Browser darf diese drei nicht über einen Kamm scheren, sonst sieht
ein Nutzer "nicht geprüft" für "in Ordnung" an.
test_nicht_geprueft_unterscheidet_sich_von_beanstandet_trotz_gleichem_http_status prüft genau
das: RC_IO_SCHEMA_VALIDIERUNGSFEHLER (610301200, "nicht geprüft") und RC_PLAUSIBILITAET ("beanstandet")
liefern BEIDE Status 422 — wer nur den HTTP-Status ausliest, kann sie nicht unterscheiden; der
Test verlangt unterschiedlichen Text.

Monkeypatch-Rezept 1:1 aus tests/test_einreichen.py übernommen (dort schon scharf: scheibe=
"gesamt" ohne ein einziges Event durchläuft dort denselben Codepfad bis CE.validate, weil
_an_gesamt_sperrgrund (bescheid_deklaration.py) dHf, Verpflegung, Übernachtung, Arbeitsmittel,
Partner-/Alleinerziehend-Konsistenz, §34-Abs.3-Excess und die an_gesamt-Gaps (Kinder/
Verlustvortrag/Progression/DBA) prüft — Stammdaten (u.a. Geburtsdatum) kommt darin an keiner
Stelle vor, unabhängig vom Kegel; ein leerer Store lässt jede dieser Prüfungen unbeantwortet,
keine schlägt an) — hier nur über einen echten Browser-Klick statt eines direkten
API.einreichen()-Rufs.
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
for sub in ("produkt/haut", "produkt/eingang", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402
import audit              # noqa: E402

try:
    from playwright.sync_api import sync_playwright  # noqa: E402
except ImportError:
    sync_playwright = None


@pytest.fixture
def base(tmp_path, monkeypatch):
    """HTTP-Server auf Port 0, daemon thread — identisch zum Muster in test_ui_rechenweg.py."""
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


def _req(base: str, method: str, path: str, body: dict | None = None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


@pytest.fixture
def playwright_context():
    if sync_playwright is None:
        pytest.skip("Playwright nicht installiert")
    try:
        with sync_playwright() as p:
            try:
                browser = p.chromium.launch(
                    headless=True, timeout=15000,
                    args=["--no-sandbox", "--disable-setuid-sandbox"])
            except Exception as e:
                pytest.skip(f"Chromium Start fehlgeschlagen: {e}")
            try:
                # bypass_csp: page.wait_for_function() wertet einen String als JS im
                # Seitenkontext aus — ohne 'unsafe-eval' verbietet die scharfe Produktions-CSP
                # das (s. dieselbe Begründung in tests/test_ui_chat_wartezustand.py). Betrifft
                # nur dieses Testwerkzeug, nicht die echte CSP (die prüft test_idor_und_csp.py).
                yield browser.new_context(viewport={"width": 360, "height": 640}, bypass_csp=True)
            finally:
                browser.close()
    except Exception as e:
        pytest.skip(f"Playwright-Setup fehlgeschlagen: {e}")


def _patch_bis_checkest(monkeypatch, rc: int, antwort: str = "", nicht_deklariert: list | None = None):
    """Alles vor CE.validate() umgehen (Deklaration/XML), damit der HTTP-Roundtrip nur noch
    testet, was ab dem checkESt-Ergebnis passiert — Rezept aus test_einreichen.py."""
    import elster_xml as EX
    monkeypatch.setattr(EX, "erzeuge_xml", lambda *a, **k: '<?xml version="1.0"?><Elster/>')
    monkeypatch.setattr(API.EM, "deklariere",
                        lambda *a, **k: {"eingaben_konsistent": True,
                                         "deklaration": {"E0100201": "M"}, "unvollstaendig": [],
                                         "nicht_deklariert": nicht_deklariert or []})
    import checkest_gate as CE
    monkeypatch.setattr(CE, "validate", lambda *a, **k: (rc, antwort))
    return CE


def _fall_und_fertig_screen(base: str, page, fall_id: str) -> None:
    """Fall anlegen (scheibe=gesamt, KEIN Event nötig — s. Moduldoc), dann direkt auf den
    #fertig-Screen springen. Kein voller Fragebogen-Durchlauf nötig, weil der Knopf selbst
    geprüft wird, nicht der Weg dorthin (den deckt der Rest der UI-Testsuite ab)."""
    s, r = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                        "fall_id": fall_id})
    assert s == 201, r
    page.goto(base)
    page.wait_for_load_state("networkidle")
    page.evaluate(f"FALL = '{fall_id}';")
    page.evaluate("document.getElementById('start').hidden = true;")
    page.evaluate("document.getElementById('flow').hidden = false;")
    page.evaluate("document.getElementById('fertig').hidden = false;")


def test_knopf_ist_im_fertig_screen_nach_preflight(base, playwright_context):
    """Struktur: der Knopf existiert, steckt im #fertig-Screen, und zwar NACH #preflight
    (die Preflight-Hinweise sind die letzte Information vor der Abgabe)."""
    page = playwright_context.new_page()
    try:
        _fall_und_fertig_screen(base, page, "knopf-struktur")
        btn = page.query_selector("#einreichen-btn")
        assert btn is not None, "kein #einreichen-btn im DOM — der Absendeknopf fehlt"
        assert btn.get_attribute("type") == "button", "Knopf ohne type=button riskiert Form-Submit"

        reihenfolge = page.evaluate("""
            () => {
              const fertig = document.getElementById('fertig');
              const kinder = Array.from(fertig.querySelectorAll('#preflight, #einreichen-btn'));
              return kinder.map(k => k.id);
            }
        """)
        assert reihenfolge == ["preflight", "einreichen-btn"], (
            f"Reihenfolge im #fertig-Screen falsch: {reihenfolge} — der Knopf muss NACH "
            "#preflight stehen")
    finally:
        page.close()


def test_vor_dem_klick_steht_nur_lokal_nichts_ans_finanzamt(base, playwright_context):
    """Kriterium 2 (Vault: keine-abgabe-aus-dem-browser): BEVOR der Knopf zum ersten Mal geklickt
    wird, steht sichtbar davor, dass nur lokal geprüft und nichts ans Finanzamt gesendet wird.
    aria-describedby, damit auch ein Screenreader den Satz am Knopf vorliest."""
    page = playwright_context.new_page()
    try:
        _fall_und_fertig_screen(base, page, "knopf-vorab-satz")
        satz = page.query_selector("#einreichen-lokal")
        assert satz is not None and satz.is_visible(), "kein sichtbarer Satz vor dem Absendeknopf"
        text = satz.text_content()
        assert "lokal" in text and "Finanzamt" in text, f"Satz sagt nicht lokal/Finanzamt: {text!r}"
        davor = page.evaluate("""() => !!(document.getElementById('einreichen-lokal')
            .compareDocumentPosition(document.getElementById('einreichen-btn'))
            & Node.DOCUMENT_POSITION_FOLLOWING)""")
        assert davor, "der Satz muss VOR dem Knopf stehen"
        assert page.get_attribute("#einreichen-btn", "aria-describedby") == "einreichen-lokal"
    finally:
        page.close()


def test_klick_ruft_wirklich_den_einreichen_endpunkt(base, playwright_context, monkeypatch):
    """Wirkung, nicht nur Struktur: ein echter Klick muss einen echten POST an
    /fall/{id}/einreichen auslösen — sonst ist der Knopf Dekoration."""
    _patch_bis_checkest(monkeypatch, rc=0, antwort="")
    page = playwright_context.new_page()
    try:
        fid = "knopf-ruft-endpunkt"
        _fall_und_fertig_screen(base, page, fid)
        with page.expect_response(
                lambda r: r.url.endswith(f"/fall/{fid}/einreichen") and r.request.method == "POST",
                timeout=5000) as resp_info:
            page.click("#einreichen-btn")
        assert resp_info.value.status == 200
    finally:
        page.close()


def _klick_und_text(base, playwright_context, monkeypatch, fall_id: str, rc: int, antwort: str = ""):
    """Ein Fall, ein Klick, der resultierende Text in #einreichen-status — für die drei
    Zustands-Tests unten wiederverwendet."""
    _patch_bis_checkest(monkeypatch, rc=rc, antwort=antwort)
    page = playwright_context.new_page()
    try:
        _fall_und_fertig_screen(base, page, fall_id)
        page.click("#einreichen-btn")
        page.wait_for_function(
            "document.getElementById('einreichen-status').textContent.trim().length > 0",
            timeout=5000)
        return page.evaluate("document.getElementById('einreichen-status').textContent")
    finally:
        page.close()


def test_erfolg_zeigt_in_ordnung_ohne_beanstandung_oder_unsicherheit(base, playwright_context, monkeypatch):
    """rc==0 → Haupttext sagt "in Ordnung" (oder gleichwertig), NICHT "beanstandet" und NICHT
    "nicht geprüft"."""
    text = _klick_und_text(base, playwright_context, monkeypatch, "zustand-ok", rc=0)
    tl = text.lower()
    assert "beanstandet" not in tl, text
    assert "nicht geprüft" not in tl, text
    assert "ordnung" in tl, f"kein Erfolgs-Wortlaut in: {text!r}"


def test_beanstandet_zeigt_anderen_text_als_erfolg(base, playwright_context, monkeypatch):
    """rc=RC_PLAUSIBILITAET (610001002) → geprüft UND beanstandet. Darf nicht wie der
    Erfolgsfall klingen."""
    text = _klick_und_text(base, playwright_context, monkeypatch, "zustand-beanstandet",
                           rc=610001002, antwort="<FehlerRegelpruefung>E0100201</FehlerRegelpruefung>")
    tl = text.lower()
    assert "beanstandet" in tl, f"kein Beanstandungs-Wortlaut in: {text!r}"
    assert "nicht geprüft" not in tl, (
        f"beanstandet darf nicht wie 'nicht geprüft' klingen: {text!r}")


def test_nicht_geprueft_unterscheidet_sich_von_beanstandet_trotz_gleichem_http_status(
        base, playwright_context, monkeypatch):
    """Kernfalle des Auftrags: RC_IO_SCHEMA_VALIDIERUNGSFEHLER (610301200, leerer Fehlerpuffer) liefert
    GENAU WIE der Plausibilitätsfehler HTTP 422 — wer nur den Status ausliest, verwechselt
    "nicht geprüft" mit "beanstandet". Der Browser-Text muss trotzdem unterscheidbar sein und
    dem Nutzer klarmachen, dass hier gar kein Urteil vorliegt."""
    text_nicht_geprueft = _klick_und_text(base, playwright_context, monkeypatch,
                                          "zustand-nicht-geprueft", rc=610301200, antwort="")
    tl = text_nicht_geprueft.lower()
    assert "beanstandet" not in tl, (
        f"'nicht geprüft' (leerer Puffer, rc=610301200) darf nicht wie 'beanstandet' klingen: "
        f"{text_nicht_geprueft!r}")
    assert "ordnung" not in tl, (
        f"'nicht geprüft' darf erst recht nicht wie eine Freigabe klingen: {text_nicht_geprueft!r}")

    text_beanstandet = _klick_und_text(base, playwright_context, monkeypatch,
                                       "zustand-beanstandet-vergleich", rc=610001002,
                                       antwort="<FehlerRegelpruefung>E0100201</FehlerRegelpruefung>")
    assert text_nicht_geprueft != text_beanstandet, (
        "beide Fälle liefern HTTP 422 und denselben Browser-Text — 'nicht geprüft' und "
        "'beanstandet' sind dann für den Nutzer ununterscheidbar:\n"
        f"  nicht_geprueft={text_nicht_geprueft!r}\n  beanstandet={text_beanstandet!r}")


# Zwei Einträge in der Form, die est_mapping.deklariere() baut: einer mit Handlungsanweisung
# (`hinweis`, Klasse i), einer nur mit Grund (Klasse c).
_NICHT_DEKLARIERT = [
    {"feld_id": "kist_konfession", "grund": "Wert 'andere' ohne XSD-Code-Zuordnung (E0100402)",
     "hinweis": "Bitte in Mein ELSTER nachtragen."},
    {"feld_id": "sonder_feld", "grund": "kein elster_kz"},
]


def test_einreichen_200_traegt_nicht_deklariert(base, monkeypatch):
    """Ohne `nicht_deklariert` im 200-Body kann die Oberfläche nicht wissen, dass gerechnete
    Werte nicht in der Erklärung stehen — sie zeigte dann „in Ordnung"
    (Vault: hinweis-erreicht-den-nutzer-nie)."""
    _patch_bis_checkest(monkeypatch, rc=0, nicht_deklariert=_NICHT_DEKLARIERT)
    s, r = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                        "fall_id": "nd-body"})
    assert s == 201, r
    s, r = _req(base, "POST", "/fall/nd-body/einreichen", {})
    assert s == 200, r
    assert r["nicht_deklariert"] == _NICHT_DEKLARIERT, r


def test_nicht_deklariert_zeigt_jeden_eintrag_statt_in_ordnung(base, playwright_context, monkeypatch):
    """Entscheidung nicht-deklarierte-werte-zeigt-die-pruefung-vollstaendig: nach dem Klick steht
    JEDER Eintrag mit Grund und `hinweis` im Status, „in Ordnung" erscheint nicht, der Knopf
    bleibt klickbar. Die leere Liste („in Ordnung") prüft
    test_erfolg_zeigt_in_ordnung_ohne_beanstandung_oder_unsicherheit."""
    _patch_bis_checkest(monkeypatch, rc=0, nicht_deklariert=_NICHT_DEKLARIERT)
    page = playwright_context.new_page()
    try:
        _fall_und_fertig_screen(base, page, "nd-anzeige")
        page.click("#einreichen-btn")
        page.wait_for_function(
            "document.getElementById('einreichen-status').textContent.trim().length > 0",
            timeout=5000)
        text = page.evaluate("document.getElementById('einreichen-status').textContent")
        klickbar = not page.evaluate("document.getElementById('einreichen-btn').disabled")
    finally:
        page.close()
    assert "in ordnung" not in text.lower(), f"'in Ordnung' trotz nicht übertragener Werte: {text!r}"
    for e in _NICHT_DEKLARIERT:
        assert e["feld_id"] in text and e["grund"] in text, f"Eintrag fehlt: {e} in {text!r}"
    assert _NICHT_DEKLARIERT[0]["hinweis"] in text, f"hinweis fehlt in {text!r}"
    assert klickbar, "Knopf muss nach der Antwort klickbar bleiben — keine neue Sperre"


# fetch-Stub, der /einreichen offen haelt, statt auf reale Netz-Latenz zu vertrauen — sonst
# koennte "disabled waehrend der Anfrage" schon vorbei sein, bevor Python nachsieht (dieselbe
# Race, die tests/test_ui_chat_wartezustand.py fuer /chat dokumentiert und dort umgeht).
_EINREICHEN_STUB = """() => {
  window.__EINREICHEN_OFFEN = null;
  window.__FETCH_ECHT = window.fetch;
  window.fetch = (url, opt) => {
    if (String(url).includes('/einreichen')) {
      return new Promise((res, rej) => { window.__EINREICHEN_OFFEN = { res, rej }; });
    }
    return window.__FETCH_ECHT(url, opt);
  };
}"""

_EINREICHEN_AUFLOESEN = """(daten) => {
  const offen = window.__EINREICHEN_OFFEN;
  setTimeout(() => offen.res(new Response(JSON.stringify(daten),
      { status: 200, headers: { 'Content-Type': 'application/json' } })), 0);
}"""


def test_doppel_klick_schuetzt_gegen_doppel_submit(base, playwright_context):
    """Muster aus vorjahrUebernehmen() (app.js): btn.disabled waehrend der Anfrage. Ohne den
    Schutz koennte ein ungeduldiger Doppelklick zwei ERiC-Laeufe gleichzeitig anstossen."""
    page = playwright_context.new_page()
    try:
        _fall_und_fertig_screen(base, page, "zustand-doppelklick")
        page.evaluate(_EINREICHEN_STUB)
        page.click("#einreichen-btn")
        page.wait_for_function("window.__EINREICHEN_OFFEN !== null", timeout=5000)

        disabled_waehrend = page.evaluate("document.getElementById('einreichen-btn').disabled")
        assert disabled_waehrend, "Knopf muss waehrend der laufenden Pruefung disabled sein"

        page.evaluate(_EINREICHEN_AUFLOESEN, {
            "fall_id": "zustand-doppelklick", "eingereicht": False,
            "basis_snapshot": "x", "befund_gebunden": False, "vz": 2025, "rc": 0,
            "klasse": "plausibel", "xml_bytes": 10, "plausibel": True,
            "hinweis": "checkESt bestanden.",
        })
        page.wait_for_function(
            "document.getElementById('einreichen-status').textContent.trim().length > 0",
            timeout=5000)
        disabled_danach = page.evaluate("document.getElementById('einreichen-btn').disabled")
        assert not disabled_danach, "Knopf muss nach der Antwort wieder klickbar sein"
    finally:
        page.close()


# ===================================================================================================
# Je Ursache ein fester Text (Vault die-einreichen-anzeige-nennt-je-ursache-einen-festen-text)
#
# Vorher las die Oberflaeche vom Server nur `grund` und `unvollstaendig`: acht verschiedene Ursachen
# von "nicht geprueft" ergaben EINEN Anzeigetext (Probe 2026-10-03: 1 von 8). Der Server schickte
# `rc`, `klasse`, `detail` und `ericantwort` laengst mit. Die Ueberschrift bleibt "Nicht geprüft.";
# der Satz darunter kommt aus einer Tabelle in app.js. Dieser Wortlaut ist von Julius am 2026-10-03
# unveraendert uebernommen worden und steht hier mit Absicht als Literal: eine stille Aenderung
# der Tabelle faellt damit auf.
# ===================================================================================================

STANDARD_SATZ = "Aus der Prüfung liegt kein Ergebnis vor. Der Fall gilt als offen."
SATZ = {
    "scheibe_nicht_abgabefaehig": (
        "Dieser Fall ist eine Teilrechnung und kann keine Erklärung tragen. Lege ihn mit der "
        "vollständigen Berechnung an, wenn du eine Erklärung brauchst."),
    "xml_nicht_baubar": (
        "Aus deinen Angaben lässt sich noch keine Erklärung erzeugen. Der Fall gilt als offen."),
    "eric_nicht_verfuegbar": (
        "Das Prüfprogramm der Finanzverwaltung ist auf diesem Rechner nicht verfügbar."),
    "kein_pruefmodul_fuer_vz": "Für das Veranlagungsjahr 2025 gibt es im Prüfprogramm kein Prüfmodul.",
    "hersteller_id_gesperrt": (
        "Die Hersteller-Kennung dieses Programms ist beim Prüfprogramm gesperrt. Das liegt nicht "
        "an deinen Angaben."),
    "io_reader_unerwartete_elemente": (
        "Das Prüfprogramm hat die erzeugte Erklärung nicht gelesen: Sie enthält Elemente, die es "
        "nicht erwartet."),
}
MARKE = "GEHEIM-PFAD-eric.log"


def _absaetze(page) -> list[str]:
    """Klick, warten bis das Ergebnis steht, dann der Text je Absatz von #einreichen-status
    (textContent verklebt die Absaetze ohne Trenner, das taugt nicht zum Vergleichen)."""
    page.click("#einreichen-btn")
    page.wait_for_function(
        "document.getElementById('einreichen-status').textContent.trim().length > 0", timeout=5000)
    return page.evaluate(
        "Array.from(document.getElementById('einreichen-status').children).map(e => e.textContent)")


def _gespielte_antwort(page, fall_id: str, status: int, body: dict) -> list[str]:
    """Der Server antwortet (gespielt) mit `body`: prueft die Oberflaeche allein, unabhaengig davon,
    ob der echte Server diese Form heute erzeugt. Mehrfach auf derselben Seite aufrufbar."""
    muster = f"**/fall/{fall_id}/einreichen"
    page.route(muster, lambda route: route.fulfill(
        status=status, content_type="application/json", body=json.dumps(body)))
    try:
        return _absaetze(page)
    finally:
        page.unroute(muster)


def _echte_antwort(base, playwright_context, monkeypatch, fall_id: str, *, rc: int | None = None,
                   antwort: str = "", ursache: str | None = None) -> list[str]:
    """Echter Server, echtes Chromium; nur die Pruefung (ERiC) und der XML-Bau werden ersetzt.
    `ursache`: "eric_fehlt" (503), "xml" (422) oder None (dann zaehlt `rc`)."""
    import elster_xml as EX
    CE = _patch_bis_checkest(monkeypatch, rc=rc if rc is not None else 0, antwort=antwort)
    if ursache == "eric_fehlt":
        def _boom(*a, **k):
            raise RuntimeError("libericapi.so nicht gefunden")
        monkeypatch.setattr(CE, "validate", _boom)
    elif ursache == "xml":
        def _kaputt(*a, **k):
            raise EX.XmlFehler("Pflichtfeld fehlt")
        monkeypatch.setattr(EX, "erzeuge_xml", _kaputt)
    page = playwright_context.new_page()
    try:
        _fall_und_fertig_screen(base, page, fall_id)
        return _absaetze(page)
    finally:
        page.close()


def _teilscheibe_antwort(base, playwright_context) -> list[str]:
    """409 scheibe_nicht_abgabefaehig: Fall auf der Scheibe an_gesamt. Die Probe erzwang den
    Schirm #fertig; ob ein Nutzer ihn im normalen Ablauf mit so einem Fall erreicht, ist offen."""
    s, r = _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025,
                                        "fall_id": "teilscheibe"})
    assert s == 201, r
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("FALL = 'teilscheibe';")
        for i, h in (("start", "true"), ("flow", "false"), ("fertig", "false")):
            page.evaluate(f"document.getElementById('{i}').hidden = {h};")
        return _absaetze(page)
    finally:
        page.close()


def _pruefe_nicht_geprueft(name: str, absaetze: list[str], satz: str, pruefcode: int | None) -> None:
    assert absaetze[:2] == ["Nicht geprüft.", satz], f"{name}: {absaetze}"
    erwartet = [f"Prüfcode: {pruefcode}"] if pruefcode is not None else []
    assert absaetze[2:] == erwartet, f"{name}: Prüfcode-Zeile falsch: {absaetze}"
    kl = " ".join(absaetze).lower()
    # Die zwei Gegenregeln des Moduls: "nicht geprüft" klingt weder nach Urteil noch nach Freigabe.
    assert "beanstandet" not in kl and "ordnung" not in kl, f"{name}: {absaetze}"


def test_je_ursache_ein_fester_text_und_alle_verschieden(base, playwright_context, monkeypatch):
    """AK1: sieben Ursachen von "nicht geprüft", ein Klick je Ursache, echter Server. Der Satz ist
    der Tabellentext, und kein Fall zeigt dasselbe wie ein anderer. Wuerde die Tabelle fehlen,
    zeigte jeder Fall den Rueckfallsatz (Probe vorher: 1 von 8)."""
    faelle = {
        "hersteller_id_gesperrt": (dict(rc=610301202), SATZ["hersteller_id_gesperrt"], 610301202),
        "kein_pruefmodul_fuer_vz": (dict(rc=610001042), SATZ["kein_pruefmodul_fuer_vz"], 610001042),
        "io_reader_unerwartete_elemente": (dict(rc=610301106),
                                           SATZ["io_reader_unerwartete_elemente"], 610301106),
        "io_gate_nicht_geprueft": (dict(rc=610301200), STANDARD_SATZ, 610301200),
        "sonstig": (dict(rc=610301006), STANDARD_SATZ, 610301006),
        "xml_nicht_baubar": (dict(ursache="xml"), SATZ["xml_nicht_baubar"], None),
        "eric_nicht_verfuegbar": (dict(ursache="eric_fehlt"), SATZ["eric_nicht_verfuegbar"], None),
    }
    gesehen = {}
    for i, (name, (args, satz, code)) in enumerate(faelle.items()):
        absaetze = _echte_antwort(base, playwright_context, monkeypatch, f"ursache-{i}", **args)
        _pruefe_nicht_geprueft(name, absaetze, satz, code)
        gesehen[name] = tuple(absaetze)
    teilscheibe = _teilscheibe_antwort(base, playwright_context)
    _pruefe_nicht_geprueft("scheibe_nicht_abgabefaehig", teilscheibe,
                           SATZ["scheibe_nicht_abgabefaehig"], None)
    gesehen["scheibe_nicht_abgabefaehig"] = tuple(teilscheibe)
    assert len(set(gesehen.values())) == len(gesehen), (
        f"verschiedene Ursachen, gleiche Anzeige: {gesehen}")


def test_unbekannter_schluessel_zeigt_den_rueckfallsatz(base, playwright_context):
    """AK1: ein `grund` oder eine `klasse`, die die Tabelle nicht kennt, zeigt den heutigen Satz und
    bleibt "Nicht geprüft." (nie "in Ordnung"). Auch Namen, die ein JS-Objekt von Haus aus hat
    ("constructor", "__proto__"), duerfen nicht als Tabellentreffer durchgehen."""
    page = playwright_context.new_page()
    try:
        _fall_und_fertig_screen(base, page, "unbekannt")
        basis = {"fall_id": "unbekannt", "eingereicht": False}
        for grund in ("gibt_es_nicht", "constructor", "__proto__", "toString"):
            a = _gespielte_antwort(page, "unbekannt", 422, {**basis, "grund": grund})
            assert a == ["Nicht geprüft.", STANDARD_SATZ], f"grund={grund!r}: {a}"
        for klasse in ("gibt_es_nicht", "constructor", "__proto__"):
            a = _gespielte_antwort(page, "unbekannt", 422, {
                **basis, "grund": "rc_kein_plausibilitaetsverdikt", "klasse": klasse, "rc": 777777})
            assert a == ["Nicht geprüft.", STANDARD_SATZ, "Prüfcode: 777777"], (
                f"klasse={klasse!r}: {a}")
        # Kein `grund` und kein 200: auch das ist nicht "in Ordnung".
        a = _gespielte_antwort(page, "unbekannt", 500, {"fehler": "irgendwas"})
        assert a == ["Nicht geprüft.", STANDARD_SATZ], a
    finally:
        page.close()


@pytest.mark.parametrize("feld", ["detail", "ericantwort", "hinweis"])
def test_roh_felder_des_servers_stehen_nie_auf_dem_schirm(base, playwright_context, feld):
    """AK2: `detail` kann den Pfad von eric.log samt Log-Auszug tragen (checkest_gate), bei
    xml_nicht_baubar ist es `str(e)` einer Ausnahme, die `hinweis`-Saetze sind in "Sie"-Form. Der
    Test legt in EIN Feld eine Marke und verlangt, dass sie in keiner Antwortform erscheint.
    Zugleich muss die Antwort wirklich angezeigt worden sein (sonst waere der Test leer)."""
    formen = {
        "rc_kein_plausibilitaetsverdikt": {"klasse": "sonstig", "rc": 610301006},
        "kein_pruefmodul_fuer_vz": {"klasse": "datenartversion_unbekannt", "rc": 610001042, "vz": 2025},
        "xml_nicht_baubar": {},
        "eric_nicht_verfuegbar": {},
        "scheibe_nicht_abgabefaehig": {"scheibe": "an_gesamt"},
        "plausibilitaet_verletzt": {"klasse": "plausibilitaet_fehler", "rc": 610001002},
    }
    page = playwright_context.new_page()
    try:
        _fall_und_fertig_screen(base, page, "roh")
        for grund, rest in formen.items():
            a = _gespielte_antwort(page, "roh", 422, {
                "fall_id": "roh", "eingereicht": False, "grund": grund, **rest, feld: MARKE})
            text = " ".join(a)
            assert a and a[0] in ("Nicht geprüft.", "Geprüft und beanstandet."), (
                f"{grund}: keine Anzeige des Ergebnisses: {a}")
            assert MARKE not in text and "eric.log" not in text, (
                f"{grund}: Feld {feld!r} steht roh auf dem Schirm: {a}")
    finally:
        page.close()


def test_pruefcode_steht_in_einer_eigenen_zeile(base, playwright_context, monkeypatch):
    """AK3: bei jedem Fall mit `rc` steht "Prüfcode: <rc>" in einem eigenen Absatz unter dem Satz.
    Zu 610301200 (Sammelcode, mindestens zwei unverwandte Ursachen) und zu jedem Code der Klasse
    `sonstig` (hier 610301006) steht KEIN Ursachentext, nur der Rueckfallsatz plus Prüfcode — die
    Regel vom 2026-08-30 (Kommentar ueber einreichenPruefen). Zu 610301106 (Klasse
    io_reader_unerwartete_elemente) gibt es einen Text. Ohne rc (503) gibt es keine Zeile."""
    for i, (rc, satz) in enumerate([(610301200, STANDARD_SATZ), (610301106,
                                    SATZ["io_reader_unerwartete_elemente"]),
                                    (610301006, STANDARD_SATZ)]):
        a = _echte_antwort(base, playwright_context, monkeypatch, f"code-{i}", rc=rc)
        assert a == ["Nicht geprüft.", satz, f"Prüfcode: {rc}"], f"rc={rc}: {a}"
    a = _echte_antwort(base, playwright_context, monkeypatch, "code-503", ursache="eric_fehlt")
    assert a == ["Nicht geprüft.", SATZ["eric_nicht_verfuegbar"]], a


def test_sperrgrund_zeigt_den_klartext_des_servers(base, playwright_context):
    """AK4: das 409 mit Sperrgrund bringt `klartext` mit, die Oberflaeche zeigt ihn statt des
    Sammelsatzes "... weil eine erforderliche Angabe fehlt". Echter Fall, echter Server, kein
    Ersatz der Pruefung (sie wird nie erreicht): Scheibe gesamt, VZ 2025, ein bestaetigtes Event
    `dhf_unterkunftskosten_monat` ergibt `dhf_tatbestand_offen`."""
    s, r = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                        "fall_id": "sperre"})
    assert s == 201, r
    s, r = _req(base, "POST", "/fall/sperre/event", {
        "feld_id": "dhf_unterkunftskosten_monat", "wert": 140000, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": {"typ": "laie_eingabe"}, "signal_2": "laie_bestaetigt"}})
    assert s == 201, r
    page = playwright_context.new_page()
    try:
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("FALL = 'sperre';")
        for i, h in (("start", "true"), ("flow", "false"), ("fertig", "false")):
            page.evaluate(f"document.getElementById('{i}').hidden = {h};")
        a = _absaetze(page)
    finally:
        page.close()
    klartext = API.sperrgrund_klartext("dhf_tatbestand_offen")
    _pruefe_nicht_geprueft("dhf_tatbestand_offen", a, klartext, None)
    assert "erforderliche Angabe fehlt" not in " ".join(a), f"Sammelsatz steht noch da: {a}"
