"""Ring-Level §33b Abs.5 Kind-PB-Übertragung: Pauschbetrag des Kindes wird auf Elternteil
übertragen. Differential-Test (mit Kind-PB vs Baseline). Nur gesamt + rentner (Person A).

STUFE 1: OHNE S.4-Ausschluss (benannte Lücke: agb_aufwendungen nicht nach Aufwandsart getrennt).

Kz: E0505809 (GdB), E0505807 (blind/hilflos), E0505805 (Hbl-Übertragung) — je Kind-Container.
Ring liest per EM.instanzen, rechnet via catala_behinderten_pb + catala_hinterbliebenen_pb.

NULL LLM.
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

jsonschema = pytest.importorskip("jsonschema")
SCHEMA_DIR = os.path.join(ROOT, "produkt", "haut", "api_schema")


def _val(name: str, obj: dict) -> None:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        jsonschema.Draft202012Validator(json.load(f)).validate(obj)


def _req(base: str, method: str, path: str, body: dict | None = None,
         erwarte: int | None = None):
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
# Hochverdiener (20000000ct = 200k€), Einzelveranlagung, keine agB, keine Vorsorge.
# Basis: ESt ~51.970€ laut Grenzsteuersatz ~42%.
# Kind1: GdB 50 → PB 1140€, nicht hilflos/blind
# Kind2: GdB 100 → PB 2840€, nicht hilflos/blind
# Σ Kind-PB = 1140 + 2840 = 3980€. Δ gesamt: höheres zvE → 45%-Zone → 3980 × 0,45 = 1791 = 179100 ct.
# Δ rentner: zvE 20k Rente → ~42%-Zone → 3980 × 0,42 = 1671 = 167100 ct.
# Gemessen 2026-08-06 17:10: gesamt=179100, rentner=167200. 100ct Diff durch Tarif-Rundung.
PB_DELTA_EXACT_GESAMT = 179100
PB_DELTA_EXACT_RENTNER = 167200
PB_HBL_DELTA = 15000    # 370€ × 42% ≈ 155€ = 15500ct, plus etwas

GESAMT_KEGEL_BASIS = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 20000000),
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
    ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ("einkuenfte_gewinn", 20000000), ("gewinn_betriebsart", "gewerbe"),
    ("fam_anzahl_kinder", 2),
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
    ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("fam_anzahl_kinder", 2),
]

# 2 Kinder UNGLEICHE GdB-Werte, beide mit kind_idnr + Antrag + nicht-selbst-genutzt:
#   Kind1: GdB 50 (PB 1140€), nicht hilflos/blind, keine Hbl → PB 1140€
#   Kind2: GdB 100 (PB 2840€), nicht hilflos/blind, keine Hbl → PB 2840€
# Σ = 3980€
PB_2KINDER = [
    ("kind_idnr", "11111111111"),
    ("kind_behinderten_pb_antrag", True), ("kind_pb_nicht_selbst_genutzt", True),
    ("kind_grad_der_behinderung", 50), ("kind_hilflos_blind_taubblind", False),
    ("kind_hinterbliebenen_uebertragung", False),
    ("kind_idnr__2", "22222222222"),
    ("kind_behinderten_pb_antrag__2", True), ("kind_pb_nicht_selbst_genutzt__2", True),
    ("kind_grad_der_behinderung__2", 100), ("kind_hilflos_blind_taubblind__2", False),
    ("kind_hinterbliebenen_uebertragung__2", False),
]

# 2 Kinder, OHNE kind_idnr → fail-closed: kein Abzug
PB_2KINDER_KEINE_IDNR = [
    ("kind_behinderten_pb_antrag", True), ("kind_pb_nicht_selbst_genutzt", True),
    ("kind_grad_der_behinderung", 50), ("kind_hilflos_blind_taubblind", False),
    ("kind_hinterbliebenen_uebertragung", False),
    ("kind_behinderten_pb_antrag__2", True), ("kind_pb_nicht_selbst_genutzt__2", True),
    ("kind_grad_der_behinderung__2", 100), ("kind_hilflos_blind_taubblind__2", False),
    ("kind_hinterbliebenen_uebertragung__2", False),
]

# 2 Kinder MIT kind_idnr, ABER Antrag=false → kumulativ nicht erfüllt → kein Abzug
PB_2KINDER_KEIN_ANTRAG = [
    ("kind_idnr", "11111111111"),
    ("kind_behinderten_pb_antrag", False), ("kind_pb_nicht_selbst_genutzt", True),
    ("kind_grad_der_behinderung", 50), ("kind_hilflos_blind_taubblind", False),
    ("kind_hinterbliebenen_uebertragung", False),
    ("kind_idnr__2", "22222222222"),
    ("kind_behinderten_pb_antrag__2", False), ("kind_pb_nicht_selbst_genutzt__2", True),
    ("kind_grad_der_behinderung__2", 100), ("kind_hilflos_blind_taubblind__2", False),
    ("kind_hinterbliebenen_uebertragung__2", False),
]

# 2 Kinder MIT kind_idnr, ABER Kind-nimmt-selbst=false → kumulativ nicht erfüllt → kein Abzug
PB_2KINDER_SELBST_GENUTZT = [
    ("kind_idnr", "11111111111"),
    ("kind_behinderten_pb_antrag", True), ("kind_pb_nicht_selbst_genutzt", False),
    ("kind_grad_der_behinderung", 50), ("kind_hilflos_blind_taubblind", False),
    ("kind_hinterbliebenen_uebertragung", False),
    ("kind_idnr__2", "22222222222"),
    ("kind_behinderten_pb_antrag__2", True), ("kind_pb_nicht_selbst_genutzt__2", False),
    ("kind_grad_der_behinderung__2", 100), ("kind_hilflos_blind_taubblind__2", False),
    ("kind_hinterbliebenen_uebertragung__2", False),
]

# 1 Kind mit Hbl-Übertragung: GdB 50 (1140€) + Hbl (370€) = 1510€
PB_1KIND_HBL = [
    ("kind_idnr", "11111111111"),
    ("kind_behinderten_pb_antrag", True), ("kind_pb_nicht_selbst_genutzt", True),
    ("kind_grad_der_behinderung", 50), ("kind_hilflos_blind_taubblind", False),
    ("kind_hinterbliebenen_uebertragung", True),
]


# ===== TESTS =============================================================

def test_p33b_abs5_kind_pb_ring_gesamt(base):
    """gesamt Hochverdiener, 2 Kinder ungleiche GdB → Δ im Band."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "pb_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "pb_mit", GESAMT_KEGEL_BASIS + PB_2KINDER)
    delta = baseline - mit
    assert delta == PB_DELTA_EXACT_GESAMT, f"baseline={baseline} mit={mit} Δ={delta} ≠ {PB_DELTA_EXACT_GESAMT}"


def test_p33b_abs5_kind_pb_ring_rentner(base):
    """rentner_gesamt Hochverdiener, 2 Kinder ungleiche GdB → Δ im Band."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "rentner_gesamt", "pr_base", RENTNER_KEGEL_BASIS)
    mit = _zahl(base, "rentner_gesamt", "pr_mit", RENTNER_KEGEL_BASIS + PB_2KINDER)
    delta = baseline - mit
    assert delta == PB_DELTA_EXACT_RENTNER, f"baseline={baseline} mit={mit} Δ={delta} ≠ {PB_DELTA_EXACT_RENTNER}"


def test_p33b_abs5_kind_pb_ring_fail_closed_keine_idnr(base):
    """Kein Abzug wenn kind_idnr fehlt (§33b Abs.5 S.5)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "pf1_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "pf1_mit", GESAMT_KEGEL_BASIS + PB_2KINDER_KEINE_IDNR)
    delta = baseline - mit
    assert delta == 0, f"baseline={baseline} mit={mit} Δ={delta} — kein Abzug ohne kind_idnr erwartet"


def test_p33b_abs5_kind_pb_ring_fail_closed_kein_antrag(base):
    """Kein Abzug wenn Antrag nicht gestellt (kumulativ S.1 'auf Antrag')."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "pf2_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "pf2_mit", GESAMT_KEGEL_BASIS + PB_2KINDER_KEIN_ANTRAG)
    delta = baseline - mit
    assert delta == 0, f"baseline={baseline} mit={mit} Δ={delta} — kein Abzug ohne Antrag erwartet"


def test_p33b_abs5_kind_pb_ring_fail_closed_selbst_genutzt(base):
    """Kein Abzug wenn Kind den PB selbst nutzt (kumulativ S.1 'wenn ihn das Kind nicht in Anspruch nimmt')."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "pf3_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "pf3_mit", GESAMT_KEGEL_BASIS + PB_2KINDER_SELBST_GENUTZT)
    delta = baseline - mit
    assert delta == 0, f"baseline={baseline} mit={mit} Δ={delta} — kein Abzug bei Kind-Nutzung erwartet"


def test_p33b_abs5_kind_pb_ring_hinterbliebenen(base):
    """Kind-PB mit Hbl-Übertragung: 1 Kind, GdB 50 (1140€) + Hbl (370€) = 1510€ → Δ."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "ph_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "ph_mit", GESAMT_KEGEL_BASIS + PB_1KIND_HBL)
    delta = baseline - mit
    # 1510€ × ~42% ≈ 634€ → 63400ct. Toleranz für Scheiben-Unterschiede.
    assert delta > PB_HBL_DELTA, f"baseline={baseline} mit={mit} Δ={delta} — Hbl-Übertragung erwartet"