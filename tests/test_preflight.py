"""P5.5 Preflight-Check — GET /fall/{id}/preflight.

Testet: leeren Fall, Flag-Widerspruch, Partner-Widerspruch, Alleinerziehend-Widerspruch,
Pauschal-Hinweis, Owner-Check.
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
for sub in ("produkt/haut", "produkt/import", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402


def _req(base: str, method: str, path: str, body: dict | None = None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


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


def _init_gesamt(base, fall_id="pf"):
    """Minimal-Gesamtfall mit neutralen Feldern. Nur Kegel-Felder setzen (einmalig, kein Konflikt)."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id})
    # Setze nur Felder, die nicht überschrieben werden müssen:
    # Alle Kegel-Felder auf 0/False/neutral
    for ev in [
        _laie("veranlagung", "einzel"),
        _laie("bruttoarbeitslohn", 0),
        _laie("vv_einnahmen", 0), _laie("vv_gebaeude_afa", 0), _laie("vv_schuldzinsen", 0),
        _laie("vv_erhaltungsaufwand", 0), _laie("vv_sonstige_wk", 0), _laie("vv_entgelt_quote_prozent", 0),
        _laie("ep_entfernung_km", 0), _laie("ep_eigenes_kfz", False), _laie("ep_oepnv_kosten", 0), _laie("ep_arbeitstage", 0),
        _laie("vor_an_anteil_rv", 0), _laie("vor_ag_anteil_rv", 0), _laie("vor_rv_ausserhalb_lstb", 0),
        _laie("basis_kv_pv", 0), _laie("weitere_vorsorgeaufwendungen", 0), _laie("mit_anspruch_auf_zuschuss", False),
        _laie("kap_kapitalertraege", 0), _laie("kap_gewinn_aktien", 0), _laie("kap_verlust_aktien", 0),
        _laie("kap_gewinn_sonstige", 0), _laie("kap_verlust_sonstige", 0),
        _laie("kein_gewinn", True), _laie("kein_kap", True), _laie("kein_vuv", True), _laie("kein_sonstige", True),
    ]:
        _req(base, "POST", f"/fall/{fall_id}/event", ev)
    return fall_id


def _init_an_gesamt(base, fall_id="pf"):
    """Minimaler an_gesamt-Fall."""
    _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id})
    for ev in [
        _laie("bruttoarbeitslohn", 0),
        _laie("veranlagung", "einzel"),
        _laie("ep_entfernung_km", 0), _laie("ep_eigenes_kfz", False), _laie("ep_oepnv_kosten", 0), _laie("ep_arbeitstage", 0),
        _laie("vor_an_anteil_rv", 0), _laie("vor_ag_anteil_rv", 0), _laie("vor_rv_ausserhalb_lstb", 0),
        _laie("basis_kv_pv", 0), _laie("weitere_vorsorgeaufwendungen", 0), _laie("mit_anspruch_auf_zuschuss", False),
        _laie("dhf_unterkunftskosten_monat", 0), _laie("dhf_monate", 0), _laie("dhf_im_inland", True),
        _laie("dhf_beruflich_veranlasst", False), _laie("dhf_eigener_hausstand", False),
        _laie("dhf_finanzielle_beteiligung", False), _laie("dhf_keine_pflicht_dienstwohnung", False),
        _laie("tage_24h", 0), _laie("tage_an_abreise", 0), _laie("tage_ueber_8h_eintaegig", 0),
        _laie("kein_gewinn", True), _laie("kein_kap", True), _laie("kein_vuv", True), _laie("kein_sonstige", True),
        _laie("fam_anzahl_kinder", 0), _laie("verlustvortrag_bestand", 0),
    ]:
        _req(base, "POST", f"/fall/{fall_id}/event", ev)
    return fall_id


class TestPreflightEmpty:
    def test_leerer_fall_gruen_keine_items(self, base):
        fid = _init_an_gesamt(base)
        st, r = _req(base, "GET", f"/fall/{fid}/preflight")
        assert st == 200
        assert r["status"] == "GREEN"
        assert r["items"] == []

    def test_unbekannter_fall_404(self, base):
        st, _ = _req(base, "GET", "/fall/nixda/preflight")
        assert st == 404


class TestPreflightFlag:
    def test_kein_kap_mit_kapitalertraegen(self, base):
        """Gesamt-Fall: kein_kap=True + kap_kapitalertraege>0 = Widerspruch.
        Setze kap_kapitalertraege NICHT in init (dann wäre es 0 und Änderung braucht ersetzt).
        Stattdessen: init setzt kein_kap, dann separat kap_kapitalertraege."""
        _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "pf-flag"})
        _req(base, "POST", "/fall/pf-flag/event", _laie("kein_kap", True))
        _req(base, "POST", "/fall/pf-flag/event", _laie("kap_kapitalertraege", 50000))
        _req(base, "POST", "/fall/pf-flag/event", _laie("veranlagung", "einzel"))
        st, r = _req(base, "GET", "/fall/pf-flag/preflight")
        assert st == 200
        assert r["status"] == "RED"
        flag_items = [i for i in r["items"] if i["bereich"] == "flag"]
        assert len(flag_items) >= 1
        assert "Kapitalerträge" in flag_items[0]["text"]


class TestPreflightPartner:
    def test_partner_feld_ohne_zusammen(self, base):
        """Gesamt: veranlagung=einzel + partner-Feld gesetzt = Widerspruch."""
        _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "pf-part"})
        _req(base, "POST", "/fall/pf-part/event", _laie("veranlagung", "einzel"))
        _req(base, "POST", "/fall/pf-part/event", _laie("rentner_grad_der_behinderung_partner", 50))
        st, r = _req(base, "GET", "/fall/pf-part/preflight")
        assert st == 200
        assert r["status"] == "RED"
        p_items = [i for i in r["items"] if i["bereich"] == "partner"]
        assert len(p_items) >= 1
        assert "Zusammenveranlagung" in p_items[0]["text"]


class TestPreflightAlleinerziehend:
    def test_alleinerziehend_mit_zusammen(self, base):
        """Gesamt: veranlagung=zusammen + fam_alleinstehend=True = Widerspruch."""
        _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "pf-ae"})
        _req(base, "POST", "/fall/pf-ae/event", _laie("veranlagung", "zusammen"))
        _req(base, "POST", "/fall/pf-ae/event", _laie("fam_alleinstehend", True))
        st, r = _req(base, "GET", "/fall/pf-ae/preflight")
        assert st == 200
        assert r["status"] == "RED"
        a_items = [i for i in r["items"] if i["bereich"] == "alleinerziehend"]
        assert len(a_items) >= 1
        assert "Alleinerziehende" in a_items[0]["text"]


class TestPreflightPauschal:
    def test_pauschal_ep_arbeitstage(self, base):
        """an_gesamt: bruttoarbeitslohn > 0, ep_arbeitstage=0 → EP-Hinweis."""
        _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025, "fall_id": "pf-pau"})
        # Setze nur die Felder die den Pauschal-Check triggern
        _req(base, "POST", "/fall/pf-pau/event", _laie("bruttoarbeitslohn", 5000000))
        _req(base, "POST", "/fall/pf-pau/event", _laie("ep_arbeitstage", 0))
        _req(base, "POST", "/fall/pf-pau/event", _laie("veranlagung", "einzel"))
        st, r = _req(base, "GET", "/fall/pf-pau/preflight")
        assert st == 200
        p_items = [i for i in r["items"] if i["bereich"] == "pauschale"]
        assert len(p_items) >= 1
        assert "Entfernungspauschale" in p_items[0]["text"]


def test_preflight_bestaetigt_typ_bereich(base):
    """Jeder item hat typ und bereich — Struktur-Garantie."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "pf-str"})
    _req(base, "POST", "/fall/pf-str/event", _laie("kein_kap", True))
    _req(base, "POST", "/fall/pf-str/event", _laie("kap_kapitalertraege", 50000))
    _req(base, "POST", "/fall/pf-str/event", _laie("veranlagung", "einzel"))
    st, r = _req(base, "GET", "/fall/pf-str/preflight")
    assert st == 200
    for item in r["items"]:
        assert "typ" in item
        assert "bereich" in item
        assert "text" in item
        assert item["typ"] in ("widerspruch", "hinweis")
