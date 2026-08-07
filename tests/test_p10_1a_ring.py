"""Ring-Level-§10 Abs.1a Nr.1 Realsplitting: der Sonderausgaben-Abzug (Unterhalt Ex-Ehegatte)
mindert die festzusetzende ESt (zahl_cent) am echten /ergebnis-Output. Differential-Test
(mit Realsplitting vs Baseline) — beweist die Ring-Verdrahtung + das Zustimmungs-Gate. Deckt
Point B (gesamt Z.906) + Point C (rentner Z.1365). NULL LLM.

Realsplitting ist eine SONDERAUSGABE (mindert das zvE, § 10 Abs.1a) — KEINE Steuerermäßigung wie
§35c. Der Steuer-Effekt ist daher der PROGRESSIVE Grenzsteuersatz auf den Abzug, NICHT flach ×100.
Wie test_ring_regression_kampagne (§33a) wird deshalb Richtung + Band geprüft (Hochverdiener ~200k €
zvE = 42 %-Zone → Abzug 13.805 € × 0,42 ≈ 5.798 €), nicht ein exaktes Δ. Der Accessor selbst
(min(unterhalt, 13.805 + kv_pv)) ist unit-gegolded (test_p10_1a_accessor.py). Kritisch: OHNE
Zustimmung (Anlage U) KEIN Abzug → Δ == 0 (over-tax-safe, K2).
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


# Realsplitting-Felder (typ cent für Beträge, bool für Zustimmung). Unterhalt 20.000 € → auf den
# Höchstbetrag 13.805 € gedeckelt (kv_pv=0). Zustimmung=True = Anlage-U-Voraussetzung erfüllt.
REALSPLITTING_MIT = [
    ("realsplitting_unterhaltsleistungen", 2000000),   # 20.000 € → min(20.000, 13.805) = 13.805 €
    ("realsplitting_empfaenger_kv_pv", 0),
    ("realsplitting_zustimmung", True),
]
REALSPLITTING_OHNE_ZUSTIMMUNG = [
    ("realsplitting_unterhaltsleistungen", 2000000),
    ("realsplitting_empfaenger_kv_pv", 0),
    ("realsplitting_zustimmung", False),               # Gate zu → KEIN Abzug (over-tax-safe)
]

# Erwartetes Δ exakt: 13.805 € (Maximalabzug §10 Abs.1a) × Grenzsteuersatz im Seed ~41,9 %
# (zvE 200k Lohn → Progressionszone 68.481–277.825 €). 20000000 cent Lohn → zvE ca. 164.000 €
# brutto (nach Vorsorge/Werbungsk), 13.805 € runter auf ~150.195 € → 42 %-Zone.
# Gemessen 2026-08-06 17:10: gesamt=578300, rentner=578300 (beide selber Grenzsatz).
DELTA_EXACT = 578300


# ===== TESTS =============================================================

def test_p10_1a_ring_gesamt_mit_zustimmung(base):
    """gesamt Hochverdiener: Realsplitting 13.805 € (gedeckelt) mit Zustimmung mindert zahl_cent
    progressiv (~5.798 € bei 42 % Grenzsteuersatz)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "grs_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "grs_mit", GESAMT_KEGEL_BASIS + REALSPLITTING_MIT)
    delta = baseline - mit
    assert delta == DELTA_EXACT, f"baseline={baseline} mit={mit} Δ={delta} ≠ {DELTA_EXACT}"


def test_p10_1a_ring_gesamt_ohne_zustimmung_kein_abzug(base):
    """gesamt: Realsplitting-Felder gesetzt, aber realsplitting_zustimmung=False → KEIN Abzug,
    zahl_cent unverändert (Gate greift, over-tax-safe). Ohne Anlage-U-Zustimmung kein § 10 Abs.1a."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "grs_base2", GESAMT_KEGEL_BASIS)
    ohne = _zahl(base, "gesamt", "grs_ohne", GESAMT_KEGEL_BASIS + REALSPLITTING_OHNE_ZUSTIMMUNG)
    assert baseline - ohne == 0, f"ohne Zustimmung darf nicht mindern: baseline={baseline} ohne={ohne}"


def test_p10_1a_ring_rentner_mit_zustimmung(base):
    """rentner_gesamt Hochverdiener: Realsplitting am Point C (Z.1365) mindert zahl_cent progressiv."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "rentner_gesamt", "rrs_base", RENTNER_KEGEL_BASIS)
    mit = _zahl(base, "rentner_gesamt", "rrs_mit", RENTNER_KEGEL_BASIS + REALSPLITTING_MIT)
    delta = baseline - mit
    assert delta == DELTA_EXACT, f"baseline={baseline} mit={mit} Δ={delta} ≠ {DELTA_EXACT}"
