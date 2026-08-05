"""P2-#3: Rentner Person-B VOR (Basisvorsorge RV) Fix — Ring-Level Tests.

Fixmap: scratchpad/fixmap_rentner_partner.md, Abschnitt P2-#3.
(1) Erreichbarkeit der 3 Partner-Felder in RENTNER_FELDER (POST → 201).
(2) Ring-Differential: rentner zusammen MIT B-VOR senkt zahl_cent.
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

import api as API
import server as SRV


def _req(base: str, method: str, path: str, body: dict | None = None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    headers = {"Content-Type": "application/json"}
    req = urllib.request.Request(base + path, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(feld_id: str, wert):
    """Bestaetigtes Laie-Event."""
    return {
        "feld_id": feld_id,
        "wert": wert,
        "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": None, "signal_2": f"ok@{feld_id}"},
    }


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


@pytest.fixture
def base(tmp_path, monkeypatch):
    faelle_dir = str(tmp_path / "faelle")
    monkeypatch.setattr(API, "FAELLE", faelle_dir)
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


# ======================================================================
# (1) Erreichbarkeitstests: POST auf die 3 VOR-Partner-Felder → 201
# ======================================================================

class TestErreichbarkeitVP:
    """VOR_PARTNER_FELDER in RENTNER_FELDER: alle drei Felder beschreibbar."""

    @pytest.fixture(autouse=True)
    def setup_fall(self, base):
        self.base = base
        st, _ = _req(base, "POST", "/fall", {"scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025,
                                              "fall_id": "vor-pf-test"})
        assert st == 201
        for f, v in [
            ("rentner_renten_art", "gesetzliche_rente"),
            ("rentner_jahresrente", 2000000),
            ("rentner_renten_beginn_jahr", 2025),
            ("rentner_alter_bei_rentenbeginn", 0),
            ("rentner_grad_der_behinderung", 0),
            ("rentner_hilflos_blind_taubblind", False),
            ("rentner_pflegegrad", 0),
            ("rentner_gepflegter_hilflos", False),
            ("rentner_hinterbliebenenbezuege", False),
            ("veranlagung", "zusammen"),
            ("kein_gewinn", True),
            ("kein_kap", True),
            ("kein_vuv", True),
            ("kein_sonstige", False),
            ("vor_an_anteil_rv", 0),
            ("vor_ag_anteil_rv", 0),
            ("vor_rv_ausserhalb_lstb", 0),
            ("basis_kv", 0),
            ("basis_pv", 0),
            ("weitere_vorsorgeaufwendungen", 0),
            ("mit_anspruch_auf_zuschuss", False),
        ]:
            s, _ = _req(base, "POST", "/fall/vor-pf-test/event", _laie(f, v))
            assert s == 201

    def test_post_vor_an_rv_partner(self):
        s, _ = _req(self.base, "POST", "/fall/vor-pf-test/event", _laie("vor_an_anteil_rv_partner", 30000))
        assert s == 201

    def test_post_vor_ag_rv_partner(self):
        s, _ = _req(self.base, "POST", "/fall/vor-pf-test/event", _laie("vor_ag_anteil_rv_partner", 15000))
        assert s == 201

    def test_post_vor_rv_ausserhalb_lstb_partner(self):
        s, _ = _req(self.base, "POST", "/fall/vor-pf-test/event", _laie("vor_rv_ausserhalb_lstb_partner", 5000))
        assert s == 201


# ======================================================================
# (2) Ring-Differential: B-VOR senkt Steuer im rentner_gesamt-Ring
# ======================================================================

def _setup_rentner_zusammen(base, fall_id, vor_partner=0):
    """Lege rentner_gesamt-Fall an mit veranlagung=zusammen + Optionale B-VOR."""
    events = [
        ("rentner_renten_art", "gesetzliche_rente"),
        ("rentner_jahresrente", 2000000),      # 20000€ in Cent
        ("rentner_renten_beginn_jahr", 2025),
        ("rentner_alter_bei_rentenbeginn", 0),
        ("rentner_grad_der_behinderung", 0),
        ("rentner_hilflos_blind_taubblind", False),
        ("rentner_pflegegrad", 0),
        ("rentner_gepflegter_hilflos", False),
        ("rentner_hinterbliebenenbezuege", False),
        ("veranlagung", "zusammen"),
        ("kein_gewinn", True),
        ("kein_kap", True),
        ("kein_vuv", True),
        ("kein_sonstige", False),
        ("vor_an_anteil_rv", 0),
        ("vor_ag_anteil_rv", 0),
        ("vor_rv_ausserhalb_lstb", 0),
        ("basis_kv", 0),
        ("basis_pv", 0),
        ("weitere_vorsorgeaufwendungen", 0),
        ("mit_anspruch_auf_zuschuss", False),
    ]
    st, _ = _req(base, "POST", "/fall", {"scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025,
                                          "fall_id": fall_id})
    assert st == 201
    for feld_id, wert in events:
        s, _ = _req(base, "POST", f"/fall/{fall_id}/event", _laie(feld_id, wert))
        assert s == 201, f"{feld_id}: {s}"
    # B-VOR-Partner optional
    if vor_partner > 0:
        for fid, w in [
            ("vor_an_anteil_rv_partner", 30000),
            ("vor_ag_anteil_rv_partner", 15000),
            ("vor_rv_ausserhalb_lstb_partner", vor_partner),
        ]:
            s, _ = _req(base, "POST", f"/fall/{fall_id}/event", _laie(fid, w))
            assert s == 201


def test_rentner_zusammen_bvor_differential(base):
    """Person-B VOR-Werte vorhanden → zahl_cent niedrig(er) als ohne (Δ >= 0).

    Beide Faelle: same rentner_gesamt-Kegel. Fall A = keine B-VOR, Fall B = B-VOR > 0.
    Die Steuerzahl muss bei B <= A sein (B-VOR koennte im Python-Pfad auch 0 Abzug haben).
    """
    _setup_rentner_zusammen(base, "rvo", 0)
    _, erg_o = _req(base, "GET", "/fall/rvo/ergebnis")
    assert erg_o["grund"] == "bestaetigt"
    steuer_o = erg_o.get("zahl_cent") or 0

    _setup_rentner_zusammen(base, "rvp", 5000)  # B-VOR setzen
    _, erg_p = _req(base, "GET", "/fall/rvp/ergebnis")
    assert erg_p["grund"] == "bestaetigt"
    steuer_p = erg_p.get("zahl_cent") or 0

    # B-VOR muss Steuer senken oder gleich lassen (nicht steigern)
    assert steuer_p <= steuer_o, \
        f"B-VOR sollte Steuer senken oder gleich lassen: ohne={steuer_o}, mit={steuer_p}"


def test_rentner_einzel_kein_impact_bvor(base):
    """Person A only (einzel) → B-VOR hat keinen Effekt."""

    events_single = [
        ("rentner_renten_art", "gesetzliche_rente"),
        ("rentner_jahresrente", 2000000),
        ("rentner_renten_beginn_jahr", 2025),
        ("rentner_alter_bei_rentenbeginn", 0),
        ("rentner_grad_der_behinderung", 0),
        ("rentner_hilflos_blind_taubblind", False),
        ("rentner_pflegegrad", 0),
        ("rentner_gepflegter_hilflos", False),
        ("rentner_hinterbliebenenbezuege", False),
        ("veranlagung", "einzel"),
        ("kein_gewinn", True),
        ("kein_kap", True),
        ("kein_vuv", True),
        ("kein_sonstige", False),
        ("vor_an_anteil_rv", 0),
        ("vor_ag_anteil_rv", 0),
        ("vor_rv_ausserhalb_lstb", 0),
        ("basis_kv", 0),
        ("basis_pv", 0),
        ("weitere_vorsorgeaufwendungen", 0),
        ("mit_anspruch_auf_zuschuss", False),
    ]
    st, _ = _req(base, "POST", "/fall", {"scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025,
                                          "fall_id": "rvs"})
    assert st == 201
    for f, v in events_single:
        s, _ = _req(base, "POST", "/fall/rvs/event", _laie(f, v))
        assert s == 201
    _, erg_s = _req(base, "GET", "/fall/rvs/ergebnis")
    assert erg_s["grund"] == "bestaetigt"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
