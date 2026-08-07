"""Ring-Level-Mobilitätsprämie (/ergebnis mobilitaetspraemie_cent) — § 101 EStG, Point-A.

Fährt den ECHTEN /ergebnis-Pfad (Scheibe an_gesamt = reiner-AN einzel) über HTTP und assertet
mobilitaetspraemie_cent gegen hand-verifizierte Golden-Werte. Deterministisch, NULL LLM.

Stufe-1-Scope (Point A): nur reiner-AN einzel. zusammen/gesamt/rentner emittieren None
(Stufe-2 = per-Ehegatte-S.3 + doppelter GFB, bewusst ungebunden — verpasste Prämie ist
fiskalisch sicher, nie eine falsche Auszahlung). § 101 S.4 = 14 % der Bemessungsgrundlage;
S.2 begrenzt auf GFB-Unterschreitung des zvE; S.3 auf EP+WK über AN-Pauschbetrag.
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


def _schema(name: str) -> dict:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        return json.load(f)


def _val(name: str, obj: dict) -> None:
    jsonschema.Draft202012Validator(_schema(name)).validate(obj)


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


def _an_anlegen(base, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


# ---- Kegel-Basis (an_gesamt, alle Pflicht-Kegel bestätigt) --------------

AN_KEGEL_BASIS = [
    ("bruttoarbeitslohn", 0),
    ("veranlagung", "einzel"),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("dhf_unterkunftskosten_monat", 0), ("dhf_monate", 0), ("dhf_im_inland", True),
    ("dhf_beruflich_veranlasst", True), ("dhf_eigener_hausstand", True),
    ("dhf_finanzielle_beteiligung", True), ("dhf_keine_pflicht_dienstwohnung", True),
    ("tage_24h", 0), ("tage_an_abreise", 0), ("tage_ueber_8h_eintaegig", 0),
    ("uebernachtung_kosten_monat", 0), ("uebernachtung_monate", 0), ("uebernachtung_monate_bisher", 0),
    ("uebernachtung_im_inland", True), ("uebernachtung_auswaerts", True),
    ("uebernachtung_alleinnutzung", True), ("uebernachtung_keine_lange_unterbrechung", True),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
]


def _mit(kegel, **overrides):
    return [(f, overrides.get(f, w)) for f, w in kegel]


# Pendler-Kegel: 220 Arbeitstage, 40 km (einfach), eigenes Kfz-egal, kein ÖPNV.
#   EP gesamt = 20 km × 0,30 × 220 + 20 km × 0,38 × 220 = 1320 + 1672 = 2992 €
#   ep_ab_21  = 1672 € ; wk_gesamt = 2992 € ; AN-Pauschbetrag = 1230 €
#   S.3-Deckel = min(1672, 2992 − 1230=1762) = 1672
PENDLER = dict(ep_arbeitstage=220, ep_entfernung_km=40)


# ===== TESTS =============================================================

def test_p101_ring_geringverdiener_pendler(base):
    """10000€ Brutto + Pendler, zvE 6972€ < GFB 12096€:
    Bemessungsgrundlage = min(ep_ab_21=1672, GFB−zvE=5124) = 1672 → 14% = 234,08€ = 23408 Cent."""
    catala = _catala_da()
    kegel = _mit(AN_KEGEL_BASIS, bruttoarbeitslohn=1000000, **PENDLER)
    _an_anlegen(base, "p101a", kegel)
    st, erg = _req(base, "GET", "/fall/p101a/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["grund"] == "bestaetigt"
        assert erg["mobilitaetspraemie_cent"] == 23408, \
            f"praemie_ring={erg['mobilitaetspraemie_cent']} expected=23408"
    else:
        assert erg["mobilitaetspraemie_cent"] is None


def test_p101_ring_hochverdiener_keine_praemie(base):
    """40000€ Brutto + Pendler, zvE 36972€ > GFB 12096€:
    § 101 S.2 Unterschreitung = 0 → keine Prämie (0), nicht None (Pendlerstrecke vorhanden)."""
    catala = _catala_da()
    kegel = _mit(AN_KEGEL_BASIS, bruttoarbeitslohn=4000000, **PENDLER)
    _an_anlegen(base, "p101b", kegel)
    st, erg = _req(base, "GET", "/fall/p101b/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["grund"] == "bestaetigt"
        assert erg["mobilitaetspraemie_cent"] == 0, \
            f"praemie_ring={erg['mobilitaetspraemie_cent']} expected=0"
    else:
        assert erg["mobilitaetspraemie_cent"] is None


def test_p101_ring_keine_pendlerstrecke_none(base):
    """Keine Entfernung (ep_entfernung_km=0) → Point-A-Gate greift nicht → None (kein Feld emittiert)."""
    catala = _catala_da()
    kegel = _mit(AN_KEGEL_BASIS, bruttoarbeitslohn=1000000)  # kein Pendler
    _an_anlegen(base, "p101c", kegel)
    st, erg = _req(base, "GET", "/fall/p101c/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["grund"] == "bestaetigt"
    assert erg["mobilitaetspraemie_cent"] is None


def test_p101_ring_zusammen_none_stufe2(base):
    """veranlagung=zusammen → Point-A-Gate (not zusammen) greift nicht → None.
    Stufe-2 (per-Ehegatte-S.3, doppelter GFB) bewusst ungebunden: verpasste Prämie fiskalisch sicher."""
    kegel = _mit(AN_KEGEL_BASIS, bruttoarbeitslohn=1000000, veranlagung="zusammen", **PENDLER)
    _an_anlegen(base, "p101d", kegel)
    st, erg = _req(base, "GET", "/fall/p101d/ergebnis")
    _val("ergebnis", erg)
    # Unabhängig vom grund (zusammen kann Partner-Kegel-offen sein): NIE eine Prämie.
    assert erg["mobilitaetspraemie_cent"] is None
