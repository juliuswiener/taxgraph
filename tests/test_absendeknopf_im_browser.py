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
das: RC_IO_KEIN_TICKET (610301200, "nicht geprüft") und RC_PLAUSIBILITAET ("beanstandet")
liefern BEIDE Status 422 — wer nur den HTTP-Status ausliest, kann sie nicht unterscheiden; der
Test verlangt unterschiedlichen Text.

Monkeypatch-Rezept 1:1 aus tests/test_einreichen.py übernommen (dort schon scharf: scheibe=
"gesamt" ohne ein einziges Event durchläuft dort denselben Codepfad bis CE.validate, weil
STAMMDATEN_FELDER Teil ihres Kegels ist und _an_gesamt_sperrgrund bei leerem Store nicht
greift) — hier nur über einen echten Browser-Klick statt eines direkten API.einreichen()-Rufs.
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
for sub in ("produkt/haut", "produkt/import", "golden"):
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


def _patch_bis_checkest(monkeypatch, rc: int, antwort: str = ""):
    """Alles vor CE.validate() umgehen (Deklaration/XML), damit der HTTP-Roundtrip nur noch
    testet, was ab dem checkESt-Ergebnis passiert — Rezept aus test_einreichen.py."""
    import elster_xml as EX
    monkeypatch.setattr(EX, "erzeuge_xml", lambda *a, **k: '<?xml version="1.0"?><Elster/>')
    monkeypatch.setattr(API.EM, "deklariere",
                        lambda *a, **k: {"eingaben_konsistent": True,
                                         "deklaration": {"E0100201": "M"}, "unvollstaendig": []})
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
    """Kernfalle des Auftrags: RC_IO_KEIN_TICKET (610301200, leerer Fehlerpuffer) liefert
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
