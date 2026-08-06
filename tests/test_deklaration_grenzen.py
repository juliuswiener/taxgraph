"""Deklarations-Grenzfälle: Was darf NICHT drin sein?

Komplementär zu Durchgangs-Tests (Test: alles kommt an).
Diese: Test: Falsches kommt nicht an, Sperrtes bleibt aus, Ersetzungen wirken.

Drei Szenarien:
1. Unbestätigte Werte dürfen NICHT in Deklaration
2. Gesperrte Fälle → /einreichen lehnt ab MIT unserem Sperrgrund (nicht ERiCs falschem)
3. Ersatzereignisse → neuer Wert in Deklaration, nicht alter
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
for sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402

jsonschema = pytest.importorskip("jsonschema")


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _vorl(fld, w):
    """Vorläufig: nicht bestätigt."""
    return {"feld_id": fld, "wert": w, "zustand": "vorlaeufig",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": None}}


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


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
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


VZ = 2025


def test_unbestaettigte_werte_nicht_in_deklaration(base):
    """Vorläufige (unbestätigte) Werte dürfen NICHT in /deklaration erscheinen.

    Fall: Ein Feld als vorlaeufig setzen (nicht bestaetigt).
    Ergebnis: /deklaration muss das Feld auslassen (nur bestätigte Werte).
    """
    st, _ = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": VZ, "fall_id": "ub1"})
    assert st == 201, f"Fall-Anlage fehlgeschlagen: {st}"

    # Pflicht-Felder (Kegel) minimal bestätigt
    pflicht = [
        ("veranlagung", "einzel"),
        ("bruttoarbeitslohn", 0),
        ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
        ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
        ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
        ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
        ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
        ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
        ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
        ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
        ("einkuenfte_gewinn", 100000),
        ("gewinn_betriebsart", "gewerbe"),
    ]
    for fld, wert in pflicht:
        st, _ = _req(base, "POST", f"/fall/ub1/event", _laie(fld, wert))
        assert st == 201, f"Pflicht-Feld {fld} fehlgeschlagen: {st}"

    # Test-Feld VORLAEUFIG setzen (nicht bestätigt)
    st, _ = _req(base, "POST", f"/fall/ub1/event", _vorl("spenden_betrag", 50000))
    assert st == 201, f"Vorlaeufig-Feld fehlgeschlagen: {st}"

    # /deklaration holen
    st, dekl = _req(base, "GET", "/fall/ub1/deklaration")
    assert st == 200, f"Deklarations-Abruf fehlgeschlagen: {st}"

    # spenden_betrag darf NICHT in Deklaration sein (nur vorlaeufig, nicht bestaetigt)
    spenden_im_xml = "E0260010" in str(dekl.get("deklaration", {}))  # Spenden-Kz (beispiel)
    assert not spenden_im_xml, (
        "Vorläufiges Feld (vorlaeufig, nicht bestaetigt) erscheint in /deklaration. "
        "Nur bestätigte Werte dürfen deklariert werden."
    )
    assert dekl.get("vollstaendig") is False, (
        "Unbestätigte Felder sollten vollstaendig=False erzeugen"
    )


def test_gesperrter_fall_einreichen_mit_sperrgrund(base):
    """Sperrgrund → /einreichen lehnt ab mit unserem Sperrgrund, nicht ERiCs falschem.

    Fall: Handwerkerkosten ohne Antwort auf Förderungsfrage → handwerker_foerderung_offen.
    Ergebnis: /einreichen liefert 409 mit grund=handwerker_foerderung_offen.
    (Nicht 422 plausibilitaet_verletzt, die ERiC liefert würde.)
    """
    st, _ = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": VZ, "fall_id": "sp1"})
    assert st == 201

    # Minimal Kegel
    minimal = [
        ("veranlagung", "einzel"),
        ("bruttoarbeitslohn", 1000000),
        ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
        ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
        ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
        ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
        ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
        ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
        ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ]
    for fld, wert in minimal:
        st, _ = _req(base, "POST", f"/fall/sp1/event", _laie(fld, wert))
        assert st == 201, f"Feld {fld} fehlgeschlagen: {st}"

    # Sperrgrund auslösen: Handwerker OHNE Förder-Antwort
    st, _ = _req(base, "POST", f"/fall/sp1/event", _laie("hh_handwerker_arbeitskosten", 300000))
    assert st == 201
    st, _ = _req(base, "POST", f"/fall/sp1/event", _laie("hh_in_eu_ewr", True))
    assert st == 201
    st, _ = _req(base, "POST", f"/fall/sp1/event", _laie("hh_rechnung_unbar", True))
    assert st == 201
    # hh_handwerker_keine_foerderung NICHT setzen → Sperrung

    # /einreichen aufrufen
    st, resp = _req(base, "POST", f"/fall/sp1/einreichen", {"empfaenger_land": "BY"}, erwarte=409)

    # MUSS 409 sein mit unserem Sperrgrund (handwerker_foerderung_offen),
    # NICHT 422 mit ERiCs plausibilitaet_verletzt
    assert resp.get("grund") == "handwerker_foerderung_offen", (
        f"Grund sollte handwerker_foerderung_offen, got {resp.get('grund')}. "
        "Nutzer bekommt falschen Grund (plausibilitaet_verletzt) statt echten (Förderung vergessen)."
    )


def test_ersatzereignis_neuer_wert_in_deklaration(base):
    """Ersatzereignis (ersetzt=event_id) → neuer Wert in /deklaration, nicht alter.

    Fall: Feld setzen, dann mit Ersatzereignis überschreiben.
    Ergebnis: Nur der neue Wert in Deklaration.
    """
    st, _ = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": VZ, "fall_id": "ers1"})
    assert st == 201

    # Minimal Kegel
    minimal = [
        ("veranlagung", "einzel"),
        ("bruttoarbeitslohn", 1000000),
        ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
        ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
        ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
        ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
        ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
        ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
        ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ]
    for fld, wert in minimal:
        st, _ = _req(base, "POST", f"/fall/ers1/event", _laie(fld, wert))
        assert st == 201

    # Spenden mit Wert 1 setzen
    st, ev1 = _req(base, "POST", f"/fall/ers1/event", _laie("spenden_betrag", 100000))
    assert st == 201
    ev1_id = ev1.get("event_id")
    assert ev1_id, f"Event-ID fehlt im Response: {ev1}"

    # Spenden mit Wert 2 (als Ersatz für Event 1)
    st, ev2 = _req(base, "POST", f"/fall/ers1/event", {
        "feld_id": "spenden_betrag",
        "wert": 200000,
        "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": None, "signal_2": "ok@spenden_betrag"},
        "ersetzt": ev1_id,
    })
    assert st == 201

    # /deklaration holen
    st, dekl = _req(base, "GET", "/fall/ers1/deklaration")
    assert st == 200

    # Deklaration muss den neuen Wert enthalten, nicht den alten
    dekl_str = json.dumps(dekl.get("deklaration", {}))

    # Neuer Wert: 200000 Cent = 2000 EUR
    # Alter Wert: 100000 Cent = 1000 EUR
    # Deklaration darf nur EINEN enthalten (den neuen)

    has_new = "200000" in dekl_str or "2000" in dekl_str
    has_old = "100000" in dekl_str and "1000" in dekl_str

    assert has_new, (
        "Neuer Spenden-Wert (200000) nicht in Deklaration. "
        "Ersatzereignis wirkt nicht."
    )
    assert not has_old, (
        "Alter Spenden-Wert (100000) noch in Deklaration. "
        "Ersatzereignis hat alte Wert nicht überschrieben."
    )
