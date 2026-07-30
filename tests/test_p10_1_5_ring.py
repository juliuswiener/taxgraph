"""Ring-Level-§10 Abs.1 Nr.5 Kinderbetreuung: der Sonderausgaben-Abzug mindert die festzusetzende
ESt (zahl_cent) am echten /ergebnis-Output. Differential-Test (mit Kinderbetreuung vs Baseline) —
beweist die Ring-Verdrahtung + Erreichbarkeit (POST 201, nicht 400). Deckt Point B (gesamt Z.894)
+ Point C (rentner Z.1360). NULL LLM.

Regression-Lock gegen das TOTE-WIRING-Muster (§35c/§10-Realsplitting: Feld nie in SCHEIBEN.felder
→ 400 → tot → Over-tax). A1 landete worker-seitig (ca23e04) und hatte NUR einen Accessor-Unit-Test
(test_p10_1_5_accessor.py) — kein Ring-Test, der die Erreichbarkeit am echten Output lockt. Die
Felder SIND erreichbar (dieser Test würde am POST 400 scheitern, wenn nicht) — hier fixiert.

Kinderbetreuung = SONDERAUSGABE (mindert zvE, § 10 Abs.1 Nr.5, 80 % cap 4.800 €/Kind) — progressiver
Effekt, kein flaches ×100. Muster §33a/§10-Realsplitting: Richtung + Band (Hochverdiener ~200k € zvE
= 42 %-Zone → Abzug 4.800 € × 0,42 ≈ 2.016 €). Der Accessor (min(aufwand_pro_kind × 80 %, 4.800) ×
anzahl) ist unit-gegolded (test_p10_1_5_accessor.py). Kinderbetreuung hat KEIN conditional Gate →
Baseline (Felder absent → 0) vs mit-1-Kind ist selbst der Erreichbarkeits-Beweis.
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


# ---- Kegel-Bausteine (Hochverdiener ~200k €, damit der Abzug voll in der 42 %-Zone greift) -------

GESAMT_KEGEL_BASIS = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 0),
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv_pv", 0), ("weitere_vorsorgeaufwendungen", 0), ("mit_anspruch_auf_zuschuss", False),
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
    ("basis_kv_pv", 0), ("weitere_vorsorgeaufwendungen", 0), ("mit_anspruch_auf_zuschuss", False),
]


# Kinderbetreuung: 1 Kind, 6.000 € Aufwand → 80 % = 4.800 € = Deckel (exakt). kosten=cent, anzahl=int.
KINDERBETREUUNG_1KIND = [
    ("kinderbetreuungskosten", 600000),        # 6.000 €
    ("kinderbetreuung_anzahl_kinder", 1),      # → min(6000×0,8, 4800) × 1 = 4.800 € Abzug
]

# Erwartetes Δ-Band: Abzug 4.800 € × Grenzsteuersatz 42 % (zvE ~200k) ≈ 2.016 € = 201.600 ct.
DELTA_MIN = 180000   # 1.800 €
DELTA_MAX = 220000   # 2.200 €


# ===== TESTS =============================================================

def test_p10_1_5_ring_gesamt(base):
    """gesamt Hochverdiener: Kinderbetreuung 1 Kind/6.000 € (Abzug 4.800 €) mindert zahl_cent
    progressiv (~2.016 € bei 42 %). Beweist Erreichbarkeit (POST 201) + Sonderausgaben-Durchgriff."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "gkb_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "gkb_mit", GESAMT_KEGEL_BASIS + KINDERBETREUUNG_1KIND)
    delta = baseline - mit
    assert DELTA_MIN <= delta <= DELTA_MAX, f"baseline={baseline} mit={mit} Δ={delta} nicht in [{DELTA_MIN},{DELTA_MAX}]"


def test_p10_1_5_ring_rentner(base):
    """rentner_gesamt Hochverdiener: Kinderbetreuung am Point C (Z.1360) mindert zahl_cent progressiv."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "rentner_gesamt", "rkb_base", RENTNER_KEGEL_BASIS)
    mit = _zahl(base, "rentner_gesamt", "rkb_mit", RENTNER_KEGEL_BASIS + KINDERBETREUUNG_1KIND)
    delta = baseline - mit
    assert DELTA_MIN <= delta <= DELTA_MAX, f"baseline={baseline} mit={mit} Δ={delta} nicht in [{DELTA_MIN},{DELTA_MAX}]"
