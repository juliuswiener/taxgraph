"""Ring-Level-§10 Abs.1 Nr.5 Kinderbetreuung: der Sonderausgaben-Abzug mindert die festzusetzende
ESt (zahl_cent) am echten /ergebnis-Output. Differential-Test (mit Kinderbetreuung vs Baseline) —
beweist die Ring-Verdrahtung + Erreichbarkeit (POST 201, nicht 400). Deckt Point B (gesamt Z.894)
+ Point C (rentner Z.1360). NULL LLM.

PER-KIND (2026-08-06 Fix): Kz im Kind-Container, Ring liest per EM.instanzen. 2 Kinder
UNGLEICHE Beträge (8000+2000) → pro Kind: 4800+1600 = 6400€ Abzug. KEINE Gleichverteilung.
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


# 2 Kinder, UNGLEICHE Beträge: Kind1=8000€ (gedeckelt 4800), Kind2=2000€ (1600) → Σ=6400€.
# Vor Fix (Gleichverteilung): 10000/2=5000 → 2×4000=8000€ (falsch).
KINDERBETREUUNG_2KINDER = [
    ("kinderbetreuungskosten", 800000),          # Kind 1: 8.000 € → 80%=6400, Deckel 4800 → 4800
    ("kinderbetreuungskosten__2", 200000),       # Kind 2: 2.000 € → 80%=1600 (< Deckel) → 1600
]

# Erwartetes Δ-Band: Abzug 6.400 € × Grenzsteuersatz 42 % (zvE ~200k) ≈ 2.688 € = 268.800 ct.
DELTA_MIN = 240000   # 2.400 €
DELTA_MAX = 290000   # 2.900 €


# ===== TESTS =============================================================

def test_p10_1_5_ring_gesamt(base):
    """gesamt Hochverdiener: 2 Kinder ungleiche Beträge (8000+2000) → Σ=6400€ Abzug.
    KEINE Gleichverteilung mehr. Beweist Erreichbarkeit + per-Kind-Instanz-Mechanik."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "gkb_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "gkb_mit", GESAMT_KEGEL_BASIS + KINDERBETREUUNG_2KINDER)
    delta = baseline - mit
    assert DELTA_MIN <= delta <= DELTA_MAX, f"baseline={baseline} mit={mit} Δ={delta} nicht in [{DELTA_MIN},{DELTA_MAX}]"


def test_p10_1_5_ring_rentner(base):
    """rentner_gesamt Hochverdiener: Kinderbetreuung am Point C (Z.1360) mindert zahl_cent.
    2 Kinder ungleiche Beträge."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "rentner_gesamt", "rkb_base", RENTNER_KEGEL_BASIS)
    mit = _zahl(base, "rentner_gesamt", "rkb_mit", RENTNER_KEGEL_BASIS + KINDERBETREUUNG_2KINDER)
    delta = baseline - mit
    assert DELTA_MIN <= delta <= DELTA_MAX, f"baseline={baseline} mit={mit} Δ={delta} nicht in [{DELTA_MIN},{DELTA_MAX}]"