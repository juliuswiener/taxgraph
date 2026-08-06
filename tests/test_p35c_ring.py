"""Ring-Level-§35c: die Steuerermäßigung mindert die festzusetzende ESt (zahl_cent) am echten
/ergebnis-Output. Differential-Test (mit §35c vs Baseline) — beweist die Ring-Verdrahtung, ohne
den Accessor zu re-derivieren. Deckt Point B (gesamt Z.872) + Point C (rentner Z.1333). NULL LLM.

Der Accessor selbst ist unit-gegolded (test_p35c_accessor.py, BMF v.2025-08-21). Hier zählt nur:
zahl_cent(mit §35c) == zahl_cent(Baseline) − Jahresdeckel×100 (voller Durchgriff bei genug ESt).
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

jsonschema = pytest.importorskip("jsonschema")
SCHEMA_DIR = os.path.join(ROOT, "produkt", "haut", "api_schema")


def _val(name: str, obj: dict) -> None:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        jsonschema.Draft202012Validator(json.load(f)).validate(obj)


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


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


VZ = 2025


def _anlegen(base, scheibe, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": scheibe, "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201, f"{feld}={wert} abgelehnt: {st}"


def _zahl(base, scheibe, fid, kegel):
    _anlegen(base, scheibe, fid, kegel)
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt", f"grund={erg['grund']} offen={erg.get('offen')}"
    return erg["zahl_cent"]


# ---- Kegel-Bausteine (Hochverdiener, damit §35c voll durchgreift) -------

GESAMT_KEGEL_BASIS = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 0),
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ("einkuenfte_gewinn", 20000000), ("gewinn_betriebsart", "gewerbe"),
]

RENTNER_KEGEL_BASIS = [
    ("veranlagung", "einzel"),
    ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 20000000),
    ("rentner_renten_beginn_jahr", 2025), ("rentner_alter_bei_rentenbeginn", 65),
    ("rentner_rentenfreibetrag", 0), ("rentner_grad_der_behinderung", 0),
    ("rentner_hilflos_blind_taubblind", False), ("rentner_hinterbliebenenbezuege", False),
    ("rentner_pflegegrad", 0), ("rentner_gepflegter_hilflos", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
]


# §35c-Felder sind cent-typisiert (typ: cent). Sanierung 20.000 € = 2.000.000 Cent.
P35C_SANIERUNG_J1 = [                       # 20.000 € × 7 % = 1.400 € Deckel → 140.000 ct Δ
    ("p35c_sanierungsaufwendungen", 2000000), ("p35c_ist_uebernaechstes_foerderjahr", False),
]
P35C_KOMBI_DECKEL = [                       # 215.000 € San + 3.000 € EB → Deckel 14.000 € → 1.400.000 ct Δ
    ("p35c_sanierungsaufwendungen", 21500000), ("p35c_ist_uebernaechstes_foerderjahr", False),
    ("p35c_energieberater_aufwendungen", 300000),
]


# ===== TESTS =============================================================

def test_p35c_ring_gesamt_sanierung(base):
    """gesamt Hochverdiener: §35c Sanierung 20.000 € J1 mindert zahl_cent um exakt 140.000 (1.400 €)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "g35c_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "g35c_san", GESAMT_KEGEL_BASIS + P35C_SANIERUNG_J1)
    assert baseline - mit == 140000, f"baseline={baseline} mit§35c={mit} Δ={baseline - mit}"


def test_p35c_ring_gesamt_kombi_jahresdeckel(base):
    """gesamt: San 215.000 € + Energieberater 3.000 € → Jahresdeckel 14.000 € → Δ = 1.400.000 ct.
    Beweist, dass der Deckel greift (roh wären 15.050+1.500 = 16.550 €)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "g35c_base2", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "g35c_kombi", GESAMT_KEGEL_BASIS + P35C_KOMBI_DECKEL)
    assert baseline - mit == 1400000, f"baseline={baseline} mit§35c={mit} Δ={baseline - mit}"


def test_p35c_ring_rentner_sanierung(base):
    """rentner_gesamt Hochverdiener: §35c am Point C (Z.1333) mindert zahl_cent um 140.000."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "rentner_gesamt", "r35c_base", RENTNER_KEGEL_BASIS)
    mit = _zahl(base, "rentner_gesamt", "r35c_san", RENTNER_KEGEL_BASIS + P35C_SANIERUNG_J1)
    assert baseline - mit == 140000, f"baseline={baseline} mit§35c={mit} Δ={baseline - mit}"
