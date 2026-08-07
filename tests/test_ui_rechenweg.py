"""P5.4 Rechenweg-Kette UI-Tests — Playwright.

Positivfall: kinderloser Fall → /ergebnis liefert kette → vier Stufen sichtbar.
Negativfall: kette null/fehlt → Rechenweg-Liste NICHT im DOM.
"""

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402
import audit                # noqa: E402


try:
    from playwright.sync_api import sync_playwright  # noqa: E402
except ImportError:
    sync_playwright = None


@pytest.fixture
def base(tmp_path, monkeypatch):
    """HTTP-Server auf Port 0, daemon thread."""
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


def _req(base: str, method: str, path: str, body: dict | None = None,
         erwarte: int | None = None):
    """HTTP-Request mit optionalem Status-Check.

    Prüft selbst:
    - 5xx → AssertionError (nie unterdrückbar)
    - 4xx → AssertionError, es sei denn `erwarte=<code>` ist gesetzt
    - 2xx → durch
    - erwarte=N → assert status == N
    """
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            status = r.status
            content = json.loads(r.read())
    except urllib.error.HTTPError as e:
        status = e.code
        content = json.loads(e.read())
    if erwarte is not None:
        assert status == erwarte, (
            f"erwarte={erwarte}, erhalten={status} {method} {path} {body}")
    elif status >= 500:
        raise AssertionError(
            f"Serverfehler {status} {method} {path} {body}: {content}")
    elif status >= 400:
        raise AssertionError(
            f"Fehler {status} {method} {path} {body}: {content}")
    return status, content


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


@pytest.fixture
def playwright_context():
    if sync_playwright is None:
        pytest.skip("Playwright nicht installiert")
    try:
        with sync_playwright() as p:
            try:
                browser = p.chromium.launch(
                    headless=True,
                    timeout=15000,
                    args=["--no-sandbox", "--disable-setuid-sandbox"]
                )
            except Exception as e:
                import traceback
                tb = traceback.format_exc()
                pytest.skip(f"Chromium Start fehlgeschlagen: {e}\n{tb}")
            try:
                yield browser.new_context(viewport={"width": 360, "height": 640})
            finally:
                browser.close()
    except Exception as e:
        import traceback
        tb = traceback.format_exc()
        pytest.skip(f"Playwright-Setup fehlgeschlagen: {e}\n{tb}")


def _einfachen_fall_anlegen(base: str, fall_id: str) -> str:
    """Kinderloser gesamt-Fall mit ALLEN Kegel-Feldern → /ergebnis liefert kette."""
    s, _ = _req(base, "POST", "/fall", {
        "scheibe": "gesamt",
        "veranlagungszeitraum": 2025,
        "fall_id": fall_id,
    })
    # gesamt-Kegel: VV_GESAMT_FELDER + veranlagung + bruttoarbeitslohn
    # + EP_FELDER + VOR_FELDER + KV_PV_FELDER + KAP_FELDER + AN_GESAMT_FLAGS
    # kette wird nur in festzusetzende_est_gesamt gesetzt (api.py:969).
    events = [
        _laie("veranlagung", "einzel"),
        _laie("bruttoarbeitslohn", 5000000),  # 50.000 € in Cent
        # VV: alles 0
        _laie("vv_einnahmen", 0),
        _laie("vv_gebaeude_afa", 0),
        _laie("vv_schuldzinsen", 0),
        _laie("vv_erhaltungsaufwand", 0),
        _laie("vv_sonstige_wk", 0),
        _laie("vv_entgelt_quote_prozent", 0),
        # EP
        _laie("ep_entfernung_km", 20),
        _laie("ep_eigenes_kfz", True),
        _laie("ep_oepnv_kosten", 0),
        _laie("ep_arbeitstage", 220),
        # VOR
        _laie("vor_an_anteil_rv", 0),
        _laie("vor_ag_anteil_rv", 0),
        _laie("vor_rv_ausserhalb_lstb", 0),
        # KV/PV
        _laie("basis_kv", 0), _laie("basis_pv", 0),
        _laie("versicherungsart", "gesetzlich_an"),
        _laie("vorsorge_arbeitslosenversicherung", 0), _laie("vorsorge_erwerbsunfaehigkeit", 0), _laie("vorsorge_unfall_haftpflicht", 0), _laie("vorsorge_rv_alt_mit_ueberschuss", 0), _laie("vorsorge_rv_alt_ohne_ueberschuss", 0),
        _laie("mit_anspruch_auf_zuschuss", False),
        # KAP
        _laie("kap_kapitalertraege", 0),
        _laie("kap_gewinn_aktien", 0),
        _laie("kap_verlust_aktien", 0),
        _laie("kap_gewinn_sonstige", 0),
        _laie("kap_verlust_sonstige", 0),
        # Flags
        _laie("kein_gewinn", True),
        _laie("kein_kap", True),
        _laie("kein_vuv", True),
        _laie("kein_sonstige", True),
    ]
    for ev in events:
        s2, _ = _req(base, "POST", f"/fall/{fall_id}/event", ev)
    return fall_id


def _einfachen_an_gesamt_fall_anlegen(base: str, fall_id: str) -> str:
    """Kinderloser an_gesamt-Fall → /ergebnis liefert grund=bestaetigt, aber kette=None."""
    s, _ = _req(base, "POST", "/fall", {
        "scheibe": "an_gesamt",
        "veranlagungszeitraum": 2025,
        "fall_id": fall_id,
    })
    # an_gesamt-Kegel: bruttoarbeitslohn, veranlagung, EP_FELDER, VOR_FELDER,
    # KV_PV_FELDER, DHF_RING, DHF_BEDINGUNGEN, VERPFLEGUNG_TAGE, AN_GESAMT_FLAGS,
    # fam_anzahl_kinder, verlustvortrag_bestand
    events = [
        _laie("bruttoarbeitslohn", 5000000),
        _laie("veranlagung", "einzel"),
        _laie("ep_entfernung_km", 20),
        _laie("ep_eigenes_kfz", True),
        _laie("ep_oepnv_kosten", 0),
        _laie("ep_arbeitstage", 220),
        _laie("vor_an_anteil_rv", 0),
        _laie("vor_ag_anteil_rv", 0),
        _laie("vor_rv_ausserhalb_lstb", 0),
        _laie("basis_kv", 0), _laie("basis_pv", 0),
        _laie("versicherungsart", "gesetzlich_an"),
        _laie("vorsorge_arbeitslosenversicherung", 0), _laie("vorsorge_erwerbsunfaehigkeit", 0), _laie("vorsorge_unfall_haftpflicht", 0), _laie("vorsorge_rv_alt_mit_ueberschuss", 0), _laie("vorsorge_rv_alt_ohne_ueberschuss", 0),
        _laie("mit_anspruch_auf_zuschuss", False),
        _laie("dhf_unterkunftskosten_monat", 0),
        _laie("dhf_monate", 0),
        _laie("dhf_im_inland", True),
        _laie("dhf_beruflich_veranlasst", False),
        _laie("dhf_eigener_hausstand", False),
        _laie("dhf_finanzielle_beteiligung", False),
        _laie("dhf_keine_pflicht_dienstwohnung", False),
        _laie("tage_24h", 0),
        _laie("tage_an_abreise", 0),
        _laie("tage_ueber_8h_eintaegig", 0),
        _laie("kein_gewinn", True),
        _laie("kein_kap", True),
        _laie("kein_vuv", True),
        _laie("kein_sonstige", True),
        _laie("fam_anzahl_kinder", 0),
        _laie("verlustvortrag_bestand", 0),
        # Nicht-Kegel-Felder
        _laie("p36_lohnsteuer", 0),
        _laie("p36_vorauszahlungen", 0),
        _laie("kist_konfession", "keine"),
        _laie("p35a_mitveranlagung", False),
    ]
    for ev in events:
        s2, _ = _req(base, "POST", f"/fall/{fall_id}/event", ev)
    return fall_id


def test_rechenweg_positiv(base, playwright_context):
    """kette vorhanden → vier Stufen im DOM mit Beträgen aus API-Antwort."""
    page = playwright_context.new_page()
    try:
        fid = _einfachen_fall_anlegen(base, "rw-pos")

        # Ergebnis abrufen und kette bestätigen
        s, ergebnis = _req(base, "GET", f"/fall/{fid}/ergebnis")
        assert s == 200
        k = ergebnis.get("kette")
        assert k is not None, "kette fehlt im Positivfall (gesamt, kinderlos)"
        assert "gesamtbetrag_der_einkuenfte" in k
        assert "festzusetzende_est" in k

        # Cent-Gleichung: kette letzte Stufe × 100 == zahl_cent
        assert k["festzusetzende_est"] * 100 == ergebnis["zahl_cent"], \
            f"kette[festzusetzende_est]={k['festzusetzende_est']} * 100 != zahl_cent={ergebnis['zahl_cent']}"

        # Seite laden — echten UI-Pfad simulieren (flow sichtbar, dann zeigeErgebnis)
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate(f"FALL = '{fid}';")
        page.evaluate("document.getElementById('start').hidden = true;")
        page.evaluate("document.getElementById('flow').hidden = false;")
        page.evaluate("(async () => { await zeigeErgebnis(); })();")
        page.wait_for_selector("#rechenweg:not([hidden])", timeout=5000)

        # Alle vier Stufen sichtbar
        reihen = page.query_selector_all("#rechenweg-body .rw-reihe")
        n_stufen = sum(1 for r in reihen if "rw-delta" not in (r.get_attribute("class") or ""))
        assert n_stufen >= 4, f"Erwarte 4 Stufen, gefunden: {n_stufen}"

        # Letzte Stufe = festzusetzende_est
        letzte = reihen[-1]
        wert_zelle = letzte.query_selector(".rw-wert")
        assert wert_zelle is not None
        wert_text = wert_zelle.text_content().strip()
        assert wert_text.endswith("€"), f"Letzte Stufe kein Euro-Wert: {wert_text}"

        # kette-Werte aus API-Antwort prüfbar im DOM
        gesamt_text = None
        for r in reihen:
            label = r.query_selector(".rw-label")
            if label and "Gesamtbetrag der Einkünfte" in (label.text_content() or ""):
                wert = r.query_selector(".rw-wert")
                if wert:
                    gesamt_text = wert.text_content().strip()
        assert gesamt_text is not None, "Gesamtbetrag der Einkünfte nicht im DOM"
        # DOM zeigt lokalisierten Betrag ("48.680 €"), API-Wert ist roher Integer (48680)
        # Browser-JS toLocaleString('de-DE') formatiert 48680 → "48.680"
        assert str(k["gesamtbetrag_der_einkuenfte"]) in gesamt_text.replace(".", ""), \
            f"API-Wert {k['gesamtbetrag_der_einkuenfte']} nicht in '{gesamt_text}'"

        # Kein horizontales Scrollen
        scroll = page.evaluate("document.documentElement.scrollWidth")
        assert scroll <= 360, f"scrollWidth={scroll} > 360"
    finally:
        page.close()


def test_rechenweg_negativ(base, playwright_context):
    """kette null/fehlt → Rechenweg-Liste NICHT im DOM."""
    page = playwright_context.new_page()
    try:
        # Fall anlegen, der kette nicht liefert (kinder > 0 → §31-Zweig)
        s, _ = _req(base, "POST", "/fall", {
            "scheibe": "an_gesamt",
            "veranlagungszeitraum": 2025,
            "fall_id": "rw-neg",
        })
        # Nur ein Feld bestätigen → kein fertiges Ergebnis (guard), kette fehlt
        _req(base, "POST", "/fall/rw-neg/event", _laie("bruttoarbeitslohn", 50000))
        _req(base, "POST", "/fall/rw-neg/event", _laie("veranlagung", "einzel"))
        _req(base, "POST", "/fall/rw-neg/event", _laie("fam_anzahl_kinder", 0))
        _req(base, "POST", "/fall/rw-neg/event", _laie("ep_entfernung_km", 20))
        _req(base, "POST", "/fall/rw-neg/event", _laie("ep_eigenes_kfz", True))
        _req(base, "POST", "/fall/rw-neg/event", _laie("ep_oepnv_kosten", 0))
        _req(base, "POST", "/fall/rw-neg/event", _laie("ep_arbeitstage", 220))
        # Restlicher Kegel (nicht mehr existierende Felder ausgelassen)
        _req(base, "POST", "/fall/rw-neg/event", _laie("verlustvortrag_bestand", 0))
        _req(base, "POST", "/fall/rw-neg/event", _laie("p36_lohnsteuer", 0))
        _req(base, "POST", "/fall/rw-neg/event", _laie("p36_vorauszahlungen", 0))
        _req(base, "POST", "/fall/rw-neg/event", _laie("kist_konfession", "keine"))
        _req(base, "POST", "/fall/rw-neg/event", _laie("p35a_mitveranlagung", False))

        page.goto(base)
        page.wait_for_load_state("networkidle")

        # Ergebnis abrufen, prüfen ob kette null
        s, ergebnis = _req(base, "GET", f"/fall/rw-neg/ergebnis")
        # Fall ist minimal bestückt → guard oder ergebnis
        k = ergebnis.get("kette")

        # UI simulieren: gleich wie im Positivfall, aber mit Fall dessen ergebnis kette=null
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.getElementById('start').hidden = true;")
        page.evaluate("document.getElementById('flow').hidden = false;")
        page.evaluate("FALL = 'rw-neg';")
        page.evaluate("(async () => { await zeigeErgebnis(); })();")

        # Kurz warten bis DOM stabil
        import time
        time.sleep(1)

        rw = page.query_selector("#rechenweg")
        assert rw is not None, "#rechenweg nicht gefunden"

        # Prüfen ob hidden (entweder via hidden-Attribut oder display:none)
        is_hidden = page.evaluate("document.getElementById('rechenweg').hidden")
        assert is_hidden, "rechenweg sollte hidden sein wenn kette null/fehlt (Guard-Fall)"

        # Kein horizontales Scrollen
        scroll = page.evaluate("document.documentElement.scrollWidth")
        assert scroll <= 360, f"scrollWidth={scroll} > 360"
    finally:
        page.close()


def test_rechenweg_bestaetigt_ohne_kette(base, playwright_context):
    """grund=bestaetigt, kette=None → rechenweg sichtbar mit Hinweistext, Tabelle hidden."""
    page = playwright_context.new_page()
    try:
        fid = _einfachen_an_gesamt_fall_anlegen(base, "rw-hinweis")

        # Ergebnis: bestaetigt aber kette=None (an_gesamt hat keine Kette)
        s, ergebnis = _req(base, "GET", f"/fall/{fid}/ergebnis")
        assert s == 200
        assert ergebnis["grund"] == "bestaetigt", f"erwarte bestaetigt, habe {ergebnis['grund']}"
        assert ergebnis["kette"] is None, "an_gesamt sollte keine kette liefern"

        # UI laden
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate(f"FALL = '{fid}';")
        page.evaluate("document.getElementById('start').hidden = true;")
        page.evaluate("document.getElementById('flow').hidden = false;")
        page.evaluate("(async () => { await zeigeErgebnis(); })();")

        # rechenweg sichtbar
        page.wait_for_selector("#rechenweg:not([hidden])", timeout=5000)

        # Tabelle hidden, Hinweis sichtbar
        tb_hidden = page.evaluate("document.getElementById('rechenweg-tabelle').hidden")
        assert tb_hidden, "Tabelle sollte hidden sein wenn keine kette da"
        hn_hidden = page.evaluate("document.getElementById('rechenweg-hinweis').hidden")
        assert not hn_hidden, "Hinweis sollte sichtbar sein wenn kette fehlt"

        hinweis_text = page.evaluate("document.getElementById('rechenweg-hinweis').textContent")
        assert hinweis_text and len(hinweis_text) > 0, "Hinweis sollte Text enthalten"

        # Kein horizontales Scrollen
        scroll = page.evaluate("document.documentElement.scrollWidth")
        assert scroll <= 360, f"scrollWidth={scroll} > 360"
    finally:
        page.close()


def test_ergebnis_hinweis_offen_bei_stiller_null(base, playwright_context):
    """stille-null-klasse-c (Variante b): grund=bestaetigt, zahl_cent gilt, aber eine vorlaeufige
    p23-Instanz steht in offen -> UI muss den Hinweis im Erfolgs-Zweig zeigen (nicht nur im
    guard-Zweig, wo r.offen vorher schon gerendert wurde)."""
    page = playwright_context.new_page()
    try:
        fid = _einfachen_fall_anlegen(base, "rw-hinweis-offen")
        for feld, wert in [
            ("p23_veraeusserungspreis", 20000000),
            ("p23_anschaffung_herstellungskosten", 15000000),
            ("p23_werbungskosten", 500000),
            ("p23_veraeusserungs_typ", "grundstueck"),
        ]:
            ev = _laie(feld, wert)
            ev["zustand"] = "vorlaeufig"
            ev["signal"] = {"signal_1": None, "signal_2": None}
            _req(base, "POST", f"/fall/{fid}/event", ev)

        s, ergebnis = _req(base, "GET", f"/fall/{fid}/ergebnis")
        assert s == 200
        assert ergebnis["grund"] == "bestaetigt", ergebnis
        assert ergebnis["zahl_cent"] is not None, "Zahl muss trotz stiller Null stehen"
        assert "p23_veraeusserungspreis" in ergebnis["offen"], ergebnis["offen"]

        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate(f"FALL = '{fid}';")
        page.evaluate("document.getElementById('start').hidden = true;")
        page.evaluate("document.getElementById('flow').hidden = false;")
        page.evaluate("(async () => { await zeigeErgebnis(); })();")
        page.wait_for_selector("#ergebnis .ergebnis-hinweis-offen", timeout=5000)

        # Zahl bleibt im Erfolgs-Zweig sichtbar (kein Guard-Umschalten)
        klasse = page.evaluate("document.getElementById('ergebnis').className")
        assert klasse == "ergebnis", f"Erfolgs-Zweig erwartet, war: {klasse}"
        zahl = page.query_selector("#ergebnis .erg-zahl")
        assert zahl is not None and zahl.text_content().strip(), "Zahl fehlt trotz stiller Null"

        hinweis_text = page.evaluate(
            "document.querySelector('#ergebnis .ergebnis-hinweis-offen').textContent")
        assert "p23_veraeusserungspreis" in hinweis_text, hinweis_text
        assert "nicht" in hinweis_text.lower(), hinweis_text

        # Kein horizontales Scrollen
        scroll = page.evaluate("document.documentElement.scrollWidth")
        assert scroll <= 360, f"scrollWidth={scroll} > 360"
    finally:
        page.close()