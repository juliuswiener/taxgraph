"""Ring-Level §10 Abs.1 Nr.3 S.2 KV/PV-Beiträge des Kindes: in DENSELBEN Abs.4-Deckel
des Elternteils, nicht separat. Differential-Test (mit Kind-Beiträgen vs Baseline).
Nur gesamt + rentner (Person A). 2 Kinder ungleiche Beträge. fail-closed: kind_idnr fehlt → kein Δ.
NULL LLM.

Kz: E0503110 (Kind-KV), E0503310 (Kind-PV) — je Kind, ceiling, _ABZUGS_KZ.
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


def _req(base: str, method: str, path: str, body: dict | None = None,
         erwarte: int | None = None):
    """HTTP-Request mit optionalem Status-Check (1:1 aus test_p10_1_9_ring.py)."""
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


# ---- Kegel-Bausteine ----
# Basis-KV = 80000ct (800€), Basis-PV = 20000ct (200€) → sum=1000€
# HB bei ohne_Zuschuss = 2800€ → Spielraum 1800€
# Kind1: kind_kv=100000ct (1000€), kind_pv=50000ct (500€) → 1500€ Kind-Summe
# Kind2: kind_kv=50000ct (500€), kind_pv=30000ct (300€) → 800€ Kind-Summe
# Σ Kind = 2300€ → basis_kv_pv gesamt = 1000+2300 = 3300€ → capped auf 2800
# → Δ Person-A = (2800-1000) = 1800€ mehr Abzug
# Bei Grenzsteuersatz ~42%: Δ ~756€ = 75600ct
# Mit Rundungstoleranz: 70k-80k ct

GESAMT_KEGEL_BASIS = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 20000000),
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 80000), ("basis_pv", 20000),  # 1000€ Basis KV/PV
    ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
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
    ("basis_kv", 80000), ("basis_pv", 20000),  # 1000€ Basis KV/PV
    ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
]

# 2 Kinder UNGLEICHE Beträge, je mit kind_idnr:
#   Kind1: kind_kv=1000€ (100000ct), kind_pv=500€ (50000ct) → 1500€
#   Kind2: kind_kv=500€ (50000ct), kind_pv=300€ (30000ct) → 800€
# Σ Kind = 2300€ → basis_kv_pv = 1000+2300=3300 → capped 2800 → Δ=1800€
KV_PV_2KINDER = [
    ("kind_idnr", "11111111111"), ("kind_kv", 100000), ("kind_pv", 50000),
    ("kind_idnr__2", "22222222222"), ("kind_kv__2", 50000), ("kind_pv__2", 30000),
]

# Dieselben Beträge OHNE kind_idnr → fail-closed: kein Abzug
KV_PV_2KINDER_KEINE_IDNR = [
    ("kind_kv", 100000), ("kind_pv", 50000),
    ("kind_kv__2", 50000), ("kind_pv__2", 30000),
]

# Basis-Durchbruch: kind_summe (2300€) voll abziehbar (kein Deckel durch Abs.4).
# 2300€ × 42% (Grenzsteuersatz) ≈ 966€ = 96600ct.
# Mit Toleranz für Scheiben-Unterschiede.
DELTA_MIN = 80000   # 800 €
DELTA_MAX = 120000  # 1200 €


# ===== TESTS =============================================================

def test_p10_1_3_kind_kv_pv_ring_gesamt(base):
    """gesamt Hochverdiener, Einzelveranlagung: 2 Kinder ungleiche KV/PV → Δ im Band."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "kb_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "kb_mit", GESAMT_KEGEL_BASIS + KV_PV_2KINDER)
    delta = baseline - mit
    assert DELTA_MIN <= delta <= DELTA_MAX, f"baseline={baseline} mit={mit} Δ={delta}"


def test_p10_1_3_kind_kv_pv_ring_rentner(base):
    """rentner_gesamt Hochverdiener, Einzelveranlagung: 2 Kinder → Δ im Band."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "rentner_gesamt", "kr_base", RENTNER_KEGEL_BASIS)
    mit = _zahl(base, "rentner_gesamt", "kr_mit", RENTNER_KEGEL_BASIS + KV_PV_2KINDER)
    delta = baseline - mit
    assert DELTA_MIN <= delta <= DELTA_MAX, f"baseline={baseline} mit={mit} Δ={delta}"


def test_p10_1_3_kind_kv_pv_ring_fail_closed(base):
    """Kein Abzug wenn kind_idnr fehlt (S.2 'Angabe der erteilten Identifikationsnummer')."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "kf_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "kf_mit", GESAMT_KEGEL_BASIS + KV_PV_2KINDER_KEINE_IDNR)
    delta = baseline - mit
    assert delta == 0, f"baseline={baseline} mit={mit} Δ={delta} — kein Abzug ohne kind_idnr erwartet"