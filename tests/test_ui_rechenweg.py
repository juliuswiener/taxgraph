"""P5.4 Rechenweg-Kette UI-Tests — Playwright.

Positivfall: kinderloser Fall → /ergebnis liefert kette → vier Stufen sichtbar.
Negativfall: kette null/fehlt → Rechenweg-Liste NICHT im DOM.
Gleichheit: kette erscheint nur, wenn ihre letzte Stufe exakt zahl_cent ist (ohne Browser).
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
from _kegel import kegel_fuer  # noqa: E402 — ein Bauer fuer alle Scheiben (tests/_kegel.py)


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
    # gesamt-Kegel GEBAUT, nicht kopiert (tests/_kegel.py). Von Hand standen hier 33 Felder;
    # 29 trugen genau den Abwesenheitswert (gemessen 2026-09-26). Nur die vier unten sind echte
    # Werte dieses Falls. Die Handliste kannte `agb_zwangslaeufig`/`agb_notwendig_angemessen`
    # nicht -- seit die im Kegel stehen, sperrte der Fall, und `kette` fehlte.
    # kette wird nur in festzusetzende_est_gesamt gesetzt (api.py:969).
    events = [_laie(f, w) for f, w in kegel_fuer("gesamt", {
        "bruttoarbeitslohn": 5000000,   # 50.000 € in Cent
        "ep_entfernung_km": 20,
        "ep_eigenes_kfz": True,
        "ep_arbeitstage": 220,
    })]
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
    # an_gesamt-Kegel GEBAUT, nicht kopiert (tests/_kegel.py). Von Hand standen hier 34 Kegel-Felder;
    # 29 trugen genau den Abwesenheitswert (gemessen 2026-09-26). Nur die fuenf unten sind echte Werte.
    # Die vier Nicht-Kegel-Felder bleiben einzeln -- der Bauer haengt sie sonst mit dem
    # Abwesenheitswert an, und `kist_konfession` ist ein enum ohne Abwesenheitswert.
    events = [_laie(f, w) for f, w in kegel_fuer("an_gesamt", {
        "bruttoarbeitslohn": 5000000,
        "ep_entfernung_km": 20,
        "ep_eigenes_kfz": True,
        "ep_arbeitstage": 220,
        "dhf_im_inland": True,
    })] + [
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
        # Fall anlegen, der kette nicht liefert. Grund ist die SCHEIBE an_gesamt, nicht die
        # Kinderzahl: an_gesamt hat gar keinen Rechenweg. Der Kommentar sagte frueher
        # "kinder > 0 → §31-Zweig" — das war nie der tragende Grund und ist seit AK-K
        # (Kette auch mit Kindern) endgueltig falsch.
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

# Entscheidung rechenweg-wird-nur-korrekt-angezeigt (2026-09-12): die Kette erscheint nur, wenn
# ihre letzte Stufe exakt die ausgegebene Steuer ist. Die Kette rechnet auf dem Rohstand VOR den
# Korrekturen in _festzusetzende(_r) (§ 34, § 35, § 32b, § 32d). Gemessen an 5f5cbfd, VZ 2025,
# kinderlos, dieselbe /ergebnis-Antwort:
#   gesamt, zusammen, 120.000 EUR Veräußerungsgewinn bei A:  zahl 30.358 EUR, Kette 34.338 EUR
#   gesamt, zusammen, je 60.000 EUR bei A und B:             zahl 17.332 EUR, Kette 17.486 EUR
#   rentner_gesamt, 20.000 EUR Rente + 120.000 EUR bei A:    zahl 20.511 EUR, Kette 27.544 EUR
#   gesamt, einzel, 60.000 EUR Lohn + 30.000 EUR Kapital (§ 32d):     zahl 21.174, Kette 13.924 EUR
#   dito + 50.000 EUR Gewerbe, 1.500 EUR GewSt-Messbetrag (§ 35):     zahl 28.756, Kette 34.756 EUR
#   dito + 20.000 EUR Lohnersatz (§ 32b):                             zahl 16.527, Kette 13.924 EUR
#   dito + 120.000 EUR Veräußerungsgewinn (§ 34):                     zahl 44.584, Kette 45.256 EUR
# Die Gegenproben ohne Sonderregel (Lohn zusammen: 8.238 = 8.238, Lohn einzel: 13.924 = 13.924,
# Rente: 811 = 811) müssen die Kette WEITER zeigen — sonst wäre „nie eine Kette" ebenfalls grün.
_VORSORGE_NULL = [
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
]
_LOHN_60K_ZUSAMMEN = [
    ("veranlagung", "zusammen"), ("bruttoarbeitslohn", 6_000_000),
    ("ep_arbeitstage", 0), ("ep_eigenes_kfz", False), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0),
    ("kein_vuv", True), ("kein_sonstige", True), ("kein_kap", True),
    # Person-B-Kegel, sonst sperrt partner_kegel_offen vor jeder Zahl
    ("bruttoarbeitslohn_partner", 0), ("kap_kapitalertraege_partner", 0),
    ("kap_gewinn_aktien_partner", 0), ("kap_gewinn_sonstige_partner", 0),
    ("kap_verlust_aktien_partner", 0), ("kap_verlust_sonstige_partner", 0),
] + _VORSORGE_NULL
_LOHN_60K_EINZEL = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 6_000_000),
    ("ep_arbeitstage", 0), ("ep_eigenes_kfz", False), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0),
    ("kein_vuv", True), ("kein_sonstige", True),
] + _VORSORGE_NULL
_RENTE_20K = [
    ("veranlagung", "einzel"),
    ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 2_000_000),
    ("rentner_renten_beginn_jahr", 2025), ("rentner_alter_bei_rentenbeginn", 65),
    ("rentner_rentenfreibetrag", 0), ("rentner_grad_der_behinderung", 0),
    ("rentner_hilflos_blind_taubblind", False), ("rentner_hinterbliebenenbezuege", False),
    ("rentner_pflegegrad", 0), ("rentner_gepflegter_hilflos", False),
    ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
] + _VORSORGE_NULL
# § 16 Abs. 4-Gates bestätigt-true, Veräußerungsgewinn bei A (und B)
_VG_A = [("kein_gewinn", False), ("rentner_alter_55_oder_berufsunfaehig", True),
         ("rentner_freibetrag_erstmalig", True)]
_VG_B = [("rentner_alter_55_oder_berufsunfaehig_partner", True),
         ("rentner_freibetrag_erstmalig_partner", True)]
_A120_ALLEIN = _LOHN_60K_ZUSAMMEN + _VG_A + [("rentner_veraeusserungsgewinn", 12_000_000)]


def _fall_mit(base: str, fid: str, scheibe: str, felder: list) -> dict:
    """Legt den Fall an, bestätigt alle Felder und liefert die /ergebnis-Antwort."""
    _req(base, "POST", "/fall", {"scheibe": scheibe, "veranlagungszeitraum": 2025, "fall_id": fid})
    felder = kegel_fuer(scheibe, dict(felder))
    for feld, wert in felder:
        _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
    _, ergebnis = _req(base, "GET", f"/fall/{fid}/ergebnis")
    assert ergebnis["grund"] == "bestaetigt", ergebnis
    return ergebnis


@pytest.mark.parametrize("scheibe,felder,kette_pflicht", [
    ("gesamt", _LOHN_60K_ZUSAMMEN + [("kein_gewinn", True)], True),
    ("gesamt", _A120_ALLEIN, False),
    ("gesamt", _LOHN_60K_ZUSAMMEN + _VG_A + _VG_B + [
        ("rentner_veraeusserungsgewinn", 6_000_000),
        ("rentner_veraeusserungsgewinn_partner", 6_000_000)], False),
    ("rentner_gesamt", _RENTE_20K + [("kein_gewinn", True)], True),
    ("rentner_gesamt", _RENTE_20K + _VG_A + [("rentner_veraeusserungsgewinn", 12_000_000)], False),
    ("gesamt", _LOHN_60K_EINZEL + [("kein_kap", True), ("kein_gewinn", True)], True),
    ("gesamt", _LOHN_60K_EINZEL + [
        ("kein_gewinn", True), ("kein_kap", False), ("kap_kapitalertraege", 3_000_000),
        ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
        ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0)], False),
    ("gesamt", _LOHN_60K_EINZEL + [
        ("kein_kap", True), ("kein_gewinn", False), ("einkuenfte_gewinn", 5_000_000),
        ("gewinn_betriebsart", "gewerbe"), ("gewst_messbetrag", 150_000), ("gewst_hebesatz", 400)], False),
    ("gesamt", _LOHN_60K_EINZEL + [
        ("kein_kap", True), ("kein_gewinn", True), ("p32b_progressionseinkuenfte", 2_000_000)], False),
    ("gesamt", _LOHN_60K_EINZEL + [("kein_kap", True)] + _VG_A + [
        ("rentner_veraeusserungsgewinn", 12_000_000)], False),
], ids=["lohn60k-gegenprobe", "vg-a120-allein", "vg-a60-b60", "rente20k-gegenprobe", "rente20k-vg120",
        "lohn60k-einzel-gegenprobe", "p32d-kap30k", "p35-gewst1500", "p32b-pe20k", "vg-einzel120"])
def test_kette_endet_bei_der_zahl(base, request, scheibe, felder, kette_pflicht):
    """Zwei Beträge mit demselben Label in derselben Antwort: steht eine Kette da, endet sie bei
    zahl_cent. Beide Setzstellen (gesamt- und Rentner-Zweig) sind abgedeckt."""
    ergebnis = _fall_mit(base, request.node.callspec.id, scheibe, felder)
    k = ergebnis["kette"]
    if kette_pflicht:
        assert k is not None, f"Gegenprobe ohne Sonderregel verliert die Kette: {ergebnis}"
    if k is not None:
        assert k["festzusetzende_est"] * 100 == ergebnis["zahl_cent"], (
            f"Rechenweg endet bei {k['festzusetzende_est']} EUR, die Zahl darüber ist "
            f"{ergebnis['zahl_cent']} ct — zwei Steuern unter demselben Label")


# --- AK-K: die Kette gibt es auch mit Kindern -----------------------------------------------
# Bis hierher wurde sie nur bei kinder==0 gebaut (§ 31-Zweig-Ambiguität vermeiden). Ein Kinderfall
# hat aber nicht EINE Kette: § 31 S. 1 EStG bewirkt die Freistellung ENTWEDER über die Freibeträge
# nach § 32 Abs. 6 ODER über das Kindergeld. Die Kette kommt deshalb aus dem Lauf, der die Steuer
# bestimmt hat, und nennt die Entscheidung (`kette["p31"]`).
#
# Beide Zweige sind bei VZ 2025 erreichbar und hier gepinnt — mit Zahlen, nicht mit Spannen:
#   Kindergeld gewinnt (Normalfall):  4.800 EUR Freibetrag einzel gegen 3.060 EUR Kindergeld/Jahr
#   Freibetrag gewinnt:               9.600 EUR bei Zusammenveranlagung (hoher Grenzsatz)
_LOHN_300K_ZUSAMMEN = [(f, 30_000_000 if f == "bruttoarbeitslohn" else w)
                       for f, w in _LOHN_60K_ZUSAMMEN]
# 20.000 EUR einzel, 1 Kind: hier gewinnt das Kindergeld (gemessen 2026-09-26: der Ohne-
# Freibetrag-Lauf ergibt 1.327 EUR, der Mit-Freibetrag-Lauf plus 3.060 EUR Kindergeld liegt
# darueber). Die Kette kommt deshalb aus dem Ohne-Lauf — dieselben vier Stufen wie ohne Kind.
_LOHN_20K_EINZEL = [(f, 2_000_000 if f == "bruttoarbeitslohn" else w)
                    for f, w in _LOHN_60K_EINZEL] + [("kein_kap", True)]
# Die vier Stufen des kinderlosigen 20.000-EUR-Falls (gemessen, s. probe_kette_zahlen.py).
_KETTE_20K = {"gesamtbetrag_der_einkuenfte": 18_770, "zu_versteuerndes_einkommen": 18_734,
              "tarifliche_est": 1_327, "festzusetzende_est": 1_327}
# Dasselbe fuer RENTNER_KEGEL (200.000 EUR Rente): die vier Stufen, kinderlos gemessen.
_KETTE_RENTNER = {"gesamtbetrag_der_einkuenfte": 166_898, "zu_versteuerndes_einkommen": 166_862,
                  "tarifliche_est": 59_170, "festzusetzende_est": 59_170}


def test_p31_stufe_nennt_die_guenstigerpruefung(base):
    """Der Freibetrag gewinnt (Zusammenveranlagung, 300.000 EUR, 1 Kind): die Kette zeigt den
    Freibetrag-Lauf und rechnet das Kindergeld hinzu — est_mit + Kindergeld ist die Spitze
    (§ 31 S. 4 EStG), nicht est_mit. Ohne die Hinzurechnung endete die Kette bei 99.596 EUR,
    waehrend die Zahl darueber 102.656 EUR ist."""
    e = _fall_mit(base, "p31-freibetrag-gewinnt", "gesamt",
                  _LOHN_300K_ZUSAMMEN + [("kein_gewinn", True), ("fam_anzahl_kinder", 1)])
    assert e["zahl_cent"] == 10_265_600, e
    k = e["kette"]
    assert k is not None, f"Kette fehlt bei 1 Kind trotz bestaetigtem Fall: {e}"
    assert k["festzusetzende_est"] == 102_656, k
    assert k["festzusetzende_est"] * 100 == e["zahl_cent"], k
    assert k["tarifliche_est"] == 99_596, k
    assert k["zu_versteuerndes_einkommen"] == 289_098, k
    assert k["p31"] == {"guenstiger": "freibetraege", "kindergeld": 3_060,
                        "text": k["p31"]["text"]}, k["p31"]
    assert "hinzugerechnet" in k["p31"]["text"] and "3.060" in k["p31"]["text"], k["p31"]


def test_p31_kindergeld_gewinnt_kette_bleibt_vollstaendig(base):
    """Gegenprobe: hier gewinnt das Kindergeld (kleines Einkommen). Die Kette darf nicht davon
    abhaengen, welcher Zweig gewinnt — sie muss auch hier vollstaendig sein und die Entscheidung
    nennen. Der Betrag ist mit Kind derselbe wie ohne (13.924 EUR): das Kindergeld bleibt."""
    e = _fall_mit(base, "p31-kindergeld-gewinnt", "gesamt",
                  _LOHN_20K_EINZEL + [("kein_gewinn", True), ("fam_anzahl_kinder", 1)])
    assert e["zahl_cent"] == 132_700, e
    k = e["kette"]
    assert k is not None, f"Kette fehlt bei 1 Kind trotz bestaetigtem Fall: {e}"
    assert k["festzusetzende_est"] == 1_327, k
    assert k["festzusetzende_est"] * 100 == e["zahl_cent"], k
    assert k["tarifliche_est"] == 1_327, k
    assert k["p31"]["guenstiger"] == "kindergeld", k["p31"]
    assert k["p31"]["kindergeld"] == 3_060, k["p31"]


def test_p31_kontrolle_kinderlos_hat_dieselbe_kette_wie_vorher(base):
    """Kontrolle: derselbe Fall ohne Kind liefert die Kette unveraendert.

    Gepinnt sind alle VIER Stufen, nicht nur die letzte — eine Abwesenheitsbehauptung
    ("kein p31") waere auch dann gruen, wenn die Kette ganz verschwaende. Dieselben Felder
    wie test_p31_kindergeld_gewinnt_kette_bleibt_vollstaendig, nur fam_anzahl_kinder=0:
    der Unterschied zwischen beiden Laeufen ist damit genau die AK-K-Aenderung."""
    e = _fall_mit(base, "p31-kontrolle", "gesamt",
                  _LOHN_20K_EINZEL + [("kein_gewinn", True), ("fam_anzahl_kinder", 0)])
    assert e["zahl_cent"] == 132_700, e
    k = e["kette"]
    assert k is not None, f"Kette fehlt im kinderlosigen Fall: {e}"
    assert {s: k[s] for s in _KETTE_20K} == _KETTE_20K, k
    assert "p31" not in k, f"Ohne Kinder gibt es keine § 31-Entscheidung zu nennen: {k}"


def test_rentner_kette_mit_kind_nennt_p31(base):
    """Dieselbe Naht im Rentner-Zweig: 200.000 EUR Rente, 1 Kind, Kindergeld gewinnt.

    Der Kegel kommt aus test_rentner_kette_nie_gebaut.RENTNER_KEGEL (200.000 EUR, kein_gewinn
    bestaetigt) -- der 20.000-EUR-Kegel dieses Moduls traegt kein kein_gewinn und sperrt deshalb
    schon vor der Zahl. Dieselben Felder wie der dortige kinderlosse Lauf, nur mit einem Kind:
    so ist der Unterschied zwischen beiden Ketten genau die AK-K-Aenderung."""
    from test_rentner_kette_nie_gebaut import RENTNER_KEGEL
    kinderlos = _fall_mit(base, "p31-rentner-ohne", "rentner_gesamt", RENTNER_KEGEL)
    e = _fall_mit(base, "p31-rentner", "rentner_gesamt",
                  RENTNER_KEGEL + [("fam_anzahl_kinder", 1)])
    assert e["zahl_cent"] == kinderlos["zahl_cent"], (
        f"Das Kindergeld gewinnt hier, also bleibt die Zahl wie ohne Kind: "
        f"{e['zahl_cent']} gegen {kinderlos['zahl_cent']}")
    k = e["kette"]
    assert k is not None, f"Kette fehlt bei rentner_gesamt mit Kind: {e}"
    assert k["festzusetzende_est"] * 100 == e["zahl_cent"], k
    assert k["p31"]["guenstiger"] == "kindergeld", k["p31"]
    assert k["p31"]["kindergeld"] == 3_060, k["p31"]
    # Kontrolle: der kinderlose Lauf derselben Felder traegt dieselben vier Stufen, ohne
    # p31-Stufe — gepinnt mit Werten, nicht mit einer Abwesenheitsbehauptung. Der Rentner-Fall
    # steht bei 59.170 EUR (RENTNER_KEGEL, 200.000 EUR Rente, kein_gewinn bestaetigt).
    assert {s: k[s] for s in _KETTE_RENTNER} == _KETTE_RENTNER, k
    assert kinderlos["kette"] == {
        "gesamtbetrag_der_einkuenfte": 166_898, "zu_versteuerndes_einkommen": 166_862,
        "tarifliche_est": 59_170, "festzusetzende_est": 59_170}, kinderlos["kette"]


def test_p31_stufe_ist_im_dom_sichtbar(base, playwright_context):
    """Was der Nutzer sieht: der § 31-Satz steht als Unterzeile an der letzten Rechenweg-Zeile.

    Ohne diesen Test waere die UI-Naht (app.js, k.p31.text) nur behauptet — die uebrigen
    AK-K-Tests lesen die API-Antwort, nicht das DOM. Geprueft wird der Freibetrag-Fall:
    dort liegt die festzusetzende Steuer (102.656 EUR) ueber der tariflichen (99.596 EUR),
    und genau diesen Sprung muss der Satz erklaeren."""
    page = playwright_context.new_page()
    try:
        _fall_mit(base, "p31-dom", "gesamt",
                  _LOHN_300K_ZUSAMMEN + [("kein_gewinn", True), ("fam_anzahl_kinder", 1)])
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("FALL = 'p31-dom';")
        page.evaluate("document.getElementById('start').hidden = true;")
        page.evaluate("document.getElementById('flow').hidden = false;")
        page.evaluate("(async () => { await zeigeErgebnis(); })();")
        page.wait_for_selector("#rechenweg:not([hidden])", timeout=5000)

        reihen = page.query_selector_all("#rechenweg-body .rw-reihe")
        letzte = reihen[-1]
        label = letzte.query_selector(".rw-label")
        assert "festzusetzende Einkommensteuer" in label.text_content(), label.text_content()
        unter = label.query_selector(".rw-sub")
        assert unter is not None, (
            "Keine Unterzeile an der letzten Stufe — der § 31-Satz erreicht die Oberflaeche "
            f"nicht. Zeilen: {[r.text_content() for r in reihen]}")
        text = unter.text_content()
        assert "Kindergeld" in text and "3.060" in text, text
        assert "hinzugerechnet" in text, f"Der Sprung ueber die tarifliche Steuer fehlt: {text}"
        # Die Zahl darueber ist die festgesetzte, nicht die tarifliche.
        wert = letzte.query_selector(".rw-wert").text_content().strip()
        assert wert == "102.656 €", f"Letzte Stufe zeigt {wert!r} statt 102.656 €"
    finally:
        page.close()


# § 32d, 30.000 EUR Kapitalerträge — dieselbe Liste wie test_kette_endet_bei_der_zahl[p32d-kap30k].
# Diese Konstellation ist der Kern des p24a-Befunds: vor dem Endstand-Fix endete die Kette hier bei
# 13.924 EUR (der § 32d-Zuschlag von 7.250 EUR fehlte), der Wächter verwarf sie still, und die
# Oberfläche zeigte den Hinweis statt einer Tabelle, die um 7.250 EUR zu niedrig gerechnet hatte.
_FELDER_P32D = _LOHN_60K_EINZEL + [
    ("kein_gewinn", True), ("kein_kap", False), ("kap_kapitalertraege", 3_000_000),
    ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0)]


def test_endstandskette_zeigt_tabelle_mit_der_gezahlten_steuer(base, playwright_context):
    """Was der Nutzer sieht, nachdem die Kette aus dem Endstand kommt: die TABELLE, und in ihrer
    letzten Zeile dieselbe Zahl, die darüber steht — hier 21.174 EUR (§ 32d-Fall).

    Der Name hielt, was der Wächter vor p24a tat (Hinweis statt Tabelle, weil die Kette um 7.250 EUR
    zu niedrig endete). Der § 32d-Fall bleibt als DOM-Pin, jetzt mit der Zahl, die er zeigen MUSS:
    wäre die Tabelle wieder hidden, hätte eine Seite ihre Endkorrektur verloren (Paritaets-Risiko).
    Das Netz selbst — Hinweis statt Tabelle bei wirklicher Abweichung — ist in
    tests/test_rechenweg_endstand.py als API-Fall gepinnt."""
    page = playwright_context.new_page()
    try:
        fid = "rw-p32d"
        ergebnis = _fall_mit(base, fid, "gesamt", _FELDER_P32D)
        assert ergebnis["kette"] is not None, (
            "Der § 32d-Fall verliert die Kette, obwohl beide Seiten aus dem Endstand speisen "
            f"sollten: zahl_cent={ergebnis['zahl_cent']}")
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate(f"FALL = '{fid}';")
        page.evaluate("document.getElementById('start').hidden = true;")
        page.evaluate("document.getElementById('flow').hidden = false;")
        page.evaluate("(async () => { await zeigeErgebnis(); })();")
        page.wait_for_selector("#rechenweg:not([hidden])", timeout=5000)
        assert not page.evaluate("document.getElementById('rechenweg-tabelle').hidden"), (
            "Tabelle hidden, obwohl eine Kette da ist")
        assert page.evaluate("document.getElementById('rechenweg-hinweis').hidden")
        letzte = page.query_selector_all("#rechenweg-body .rw-reihe")[-1]
        wert = letzte.query_selector(".rw-wert").text_content().strip()
        assert wert == "21.174 €", (
            f"Letzte Stufe zeigt {wert!r} statt 21.174 € — dann sind Zahl und Kette wieder "
            "zwei verschiedene Steuern")
    finally:
        page.close()
