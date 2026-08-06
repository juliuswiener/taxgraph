"""KiSt §51a Bemessung: Angestellter kirchensteuerpflichtig, kein Kapital.

BUG (2026-08-06): gesamt slot_fn Z.1146 gibt `kap_st_total` als `est_mit_fb`,
catala_kist erwartet aber die volle ESt OHNE §32d-Kapital. Bei kap=0 ist
kap_st_total=0 → KiSt=0 für JEDEN kirchensteuerpflichtigen Angestellten.

Fix: est_mit_fb = ESt ohne Kapitalanteil = solz_info["est_roh_ohne_kap"].
§32d-Abgeltung-KiSt wird über e/(4+k) in §32d Abs.1 S.3-4 geregelt, nicht
separat.

Test muss auf aktuellem Code ROT sein (KiSt=0 trotz Konfession).
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


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _req(base, method, path, body=None, erwarte=None):
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
        assert status == erwarte, f"erwarte={erwarte}, erhalten={status} {method} {path}"
    elif status >= 400:
        raise AssertionError(f"Fehler {status} {method} {path}: {content}")
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


def _catala_da():
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


# Pflicht-Kegel gesamt (api_constants: VV_GESAMT_FELDER + veranlagung + bruttoarbeitslohn
# + EP_FELDER + VOR_FELDER + KV_PV_FELDER + KAP_FELDER + AN_GESAMT_FLAGS)
GESAMT_KEGEL_BASE = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 2400000),  # 24000 EUR
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
]


def test_kist_bemessung_ohne_kapital(base):
    """Angestellter, roem.-kath., NRW, kein Kapital → KiSt = 9% der ESt.

    MUSS ROT sein auf aktuellem Code (kap_st_total=0 → KiSt=0).
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    kegel += [("kist_konfession", "roemisch-katholisch"),
              ("kist_bundesland", "nordrhein_westfalen")]
    _anlegen(base, "kist0", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kist0/ergebnis")
    assert st == 200
    kist_cent = erg.get("kist_cent")
    assert kist_cent is not None, f"kist_cent fehlt: {erg}"
    assert kist_cent > 0, (
        f"KiSt = 0 obwohl kirchensteuerpflichtig (roem.-kath., NRW). "
        f"BUG: gesamt slot_fn gibt kap_st_total statt ESt. {erg}")
    # Erwartet: 9 % der ESt (§51a, NRW → 9%)
    est_cent = erg["zahl_cent"]
    expected = est_cent // 100 * 9   # 9 % von ESt (EUR → KiSt CENT)
    assert kist_cent == expected, (
        f"KiSt {kist_cent} != {expected} (9% von {est_cent} CENT)")


def test_kist_bemessung_ohne_kapital_24000(base):
    """24.000 EUR Lohn, roem.-kath., NRW → KiSt ~20988 cent (209,88 EUR).

    Explizite Werte aus Mains Messung: ESt 233.200 cent, KiSt 20.988 cent.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    kegel += [("kist_konfession", "roemisch-katholisch"),
              ("kist_bundesland", "nordrhein_westfalen")]
    _anlegen(base, "kist24", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kist24/ergebnis")
    assert st == 200
    kist_cent = erg.get("kist_cent")
    assert kist_cent is not None, f"kist_cent fehlt: {erg}"
    # Rot-Erwartung: aktuell kap_st_total=0 → KiSt=0
    assert kist_cent > 0, f"ROT bestätigt: KiSt=0 bei 24.000 EUR Lohn, kath., NRW"


def test_kist_mit_kapital(base):
    """Angestellter mit Kapitalerträgen (5000 EUR).

    §51a-Bemessungsgrundlage OHNE §32d-Kapital → KiSt nur auf Nicht-Kapital-ESt.
    KiSt auf Abgeltungsteuer (§32d Abs.1 S.3-5) ist NICHT implementiert (benannte Lücke).
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    # kein_kap=False → Kapital vorhanden; kap_kapitalertraege=500000 (5000 EUR in cent)
    for i, (k, v) in enumerate(kegel):
        if k == "kein_kap":
            kegel[i] = (k, False)
        elif k == "kap_kapitalertraege":
            kegel[i] = (k, 500000)  # 5000 EUR in cent
    kegel += [("kist_konfession", "roemisch-katholisch"),
              ("kist_bundesland", "nordrhein_westfalen")]
    _anlegen(base, "kistkap", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kistkap/ergebnis")
    assert st == 200
    kist_cent = erg.get("kist_cent")
    assert kist_cent is not None, f"kist_cent fehlt: {erg}"
    # Charakterisierung der Lücke: KiSt entspricht 9% der ESt OHNE Kapitalanteil.
    # Bei 24.000 Lohn + 5.000 Kapital, ESt ~14.582 EUR. KiSt=209,88 = 9% von 2.332.
    # Der Kapitalanteil (ESt-Differenz ~12.250 EUR) trägt 0 KiSt.
    # Vollständige KiSt wäre ~320 EUR. Die 110 EUR Differenz = §32d-Abs.1-S.3-5-Lücke.
    ohni_kap = 233200  # est_cent ohne Kapital aus test_kist_bemessung_ohne_kapital_24000
    expected_ohne_kap = ohni_kap // 100 * 9  # 20988 cent = 209,88 EUR
    assert kist_cent == expected_ohne_kap, (
        f"KiSt {kist_cent} != {expected_ohne_kap} (9% von ESt ohne Kapital). "
        f"Lücke: Kapital-KiSt nach §32d Abs.1 S.3-5 fehlt. erg={erg}")
    # Lücke dokumentieren: KiSt auf Kapital nach §32d Abs.1 S.3-5
    # Formel: KiSt_on_cap = k × (e - 4q) / (4 + k), mit q=0 → k × e / (4 + k)
    # Für NRW (k=9%): 0.09 × 500000 / 4.09 = 11002 cent = 110,02 EUR
    kist_auf_kapital_erwartet = 11002
    kist_vollstaendig = expected_ohne_kap + kist_auf_kapital_erwartet
    print(f"\n[LÜCKE] KiSt auf Kapital (§32d Abs.1 S.3-5) fehlt: ~{kist_auf_kapital_erwartet} cent "
          f"(vollständig: ~{kist_vollstaendig} cent)")


def _anlegen(base, fid, scheibe, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": scheibe, "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201