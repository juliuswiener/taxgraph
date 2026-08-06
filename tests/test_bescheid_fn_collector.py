"""Äquivalenz-Collector für _bescheid_fn-Umbau.

Fährt den ECHTEN /ergebnis-Pfad (HTTP + Server + Store + _bescheid_fn) für alle
4 Scheiben auf definierten Seeds und schreibt byte-genauen JSON-Output.

Vor Umbau: ZWEIMAL laufen, diff /tmp/ring_collector/v1/ /tmp/ring_collector/v2/ MUSS leer sein.
Nach Umbau: dritter Lauf, diff mit v1 MUSS leer sein.

Aufruf: python -m pytest tests/test_bescheid_fn_collector.py -x -v -s 2>&1
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
for sub in ("produkt/haut", "golden", "produkt/store", "produkt/traverser",
            "produkt/unsicherheit", "produkt/mapping", "produkt/konsistenz"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API
import server as SRV

OUT_DIR = "/tmp/ring_collector"
VZ = 2025


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


# -- Seeds: repräsentative Sätze je Scheibe -----------------------------------

AN_KEGEL = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 20000000),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 200000), ("basis_pv", 50000), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", True),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("dhf_unterkunftskosten_monat", 0), ("dhf_monate", 0), ("dhf_im_inland", False),
    ("dhf_beruflich_veranlasst", False), ("dhf_eigener_hausstand", False),
    ("dhf_finanzielle_beteiligung", False), ("dhf_keine_pflicht_dienstwohnung", False),
    ("tage_24h", 0), ("tage_an_abreise", 0), ("tage_ueber_8h_eintaegig", 0),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
]

GESAMT_KEGEL = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 20000000),
    ("vv_einnahmen", 600000), ("vv_gebaeude_afa", 120000), ("vv_schuldzinsen", 20000),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 30000), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 200000), ("basis_pv", 50000), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", True),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", False), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    # -- Non-zero Abzüge für die 19 shared catala-Aufrufe --
    ("fam_anzahl_kinder", 2), ("verlustvortrag_bestand", 200000),
    ("spenden_betrag", 100000), ("kist_gezahlt", 100000), ("kist_erstattet", 0),
    ("berufsausbildung_aufwendungen", 100000),
    ("p33a_unterhalt_aufwendungen", 300000), ("p33a_unterhalt_kv_pv", 0),
    ("p33a_andere_einkuenfte_bezuege", 0), ("p33a_ausbildung_anzahl_kinder", 1),
    ("p35c_sanierungsaufwendungen", 500000), ("p35c_energieberater_aufwendungen", 100000),
    ("p35c_ist_uebernaechstes_foerderjahr", True),
    ("p22_nr3_einkuenfte", 50000),
    ("hh_minijob_aufwendungen", 50000),
    ("realsplitting_unterhaltsleistungen", 100000), ("realsplitting_empfaenger_kv_pv", 0),
    ("realsplitting_zustimmung", True),
    ("dba_staat", "polen"), ("dba_methode", "anrechnung"),
    ("dba_einkunftsart", "dividenden"),
    ("dba_gezahlte_auslaendische_steuer", 50000),
    ("dba_auslaendische_einkuenfte", 100000),
    ("dba_abzug_statt_anrechnung", False), ("dba_mehrere_staaten", False),
]

RENTNER_KEGEL = [
    ("veranlagung", "einzel"),
    ("rentner_renten_art", "gesetzliche_rente"),
    ("rentner_jahresrente", 2000000),
    ("rentner_renten_beginn_jahr", 2023),
    ("rentner_alter_bei_rentenbeginn", 65),
    ("rentner_grad_der_behinderung", 0),
    ("rentner_hilflos_blind_taubblind", False),
    ("rentner_pflegegrad", 0),
    ("rentner_gepflegter_hilflos", False),
    ("rentner_hinterbliebenenbezuege", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 200000), ("basis_pv", 50000), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", True),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ("einkuenfte_gewinn", 0), ("rentner_veraeusserungsgewinn", 0),
    ("rentner_veraeusserungs_betriebsart", "gewerbe"),
    ("gewinn_betriebsart", "keine"),
    ("geburtsjahr", 1960),
    ("gewst_hebesatz", 0), ("gewst_messbetrag", 0),
    ("verlustvortrag_bestand", 0),
    ("rentner_rentenfreibetrag", 0),
    # -- Non-zero Abzüge, ESt bleibt > 0 (56.300 cent) --
    ("spenden_betrag", 100000), ("kist_gezahlt", 100000),
    ("p33a_unterhalt_aufwendungen", 300000),
]


def _anlegen(base, fid, scheibe, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": scheibe, "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


def test_collect_an_gesamt(base):
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    results = {}
    _anlegen(base, "an_base", "an_gesamt", AN_KEGEL)
    st, erg = _req(base, "GET", "/fall/an_base/ergebnis")
    results["an_gesamt/zahl_cent"] = erg.get("zahl_cent")
    results["an_gesamt/grund"] = erg.get("grund")
    # EP-Variante
    ep_kegel = list(AN_KEGEL)
    for i, (k, v) in enumerate(ep_kegel):
        if k == "ep_arbeitstage":
            ep_kegel[i] = (k, 220)
        elif k == "ep_entfernung_km":
            ep_kegel[i] = (k, 30)
        elif k == "ep_eigenes_kfz":
            ep_kegel[i] = (k, True)
    _anlegen(base, "an_ep", "an_gesamt", ep_kegel)
    st, erg = _req(base, "GET", "/fall/an_ep/ergebnis")
    results["an_ep_ep/zahl_cent"] = erg.get("zahl_cent")
    results["an_ep_ep/grund"] = erg.get("grund")
    _write_out(results)


def test_collect_gesamt(base):
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    results = {}
    _anlegen(base, "ges_base", "gesamt", GESAMT_KEGEL)
    st, erg = _req(base, "GET", "/fall/ges_base/ergebnis")
    results["gesamt/zahl_cent"] = erg.get("zahl_cent")
    results["gesamt/grund"] = erg.get("grund")
    _write_out(results)


def test_collect_rentner(base):
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    results = {}
    _anlegen(base, "rent_base", "rentner_gesamt", RENTNER_KEGEL)
    st, erg = _req(base, "GET", "/fall/rent_base/ergebnis")
    results["rentner_gesamt/zahl_cent"] = erg.get("zahl_cent")
    results["rentner_gesamt/grund"] = erg.get("grund")
    _write_out(results)


def _write_out(results):
    if not results:
        return
    os.makedirs(OUT_DIR, exist_ok=True)
    path = os.path.join(OUT_DIR, "results.json")
    # merge mit vorhandenem Ergebnis
    if os.path.exists(path):
        existing = json.load(open(path))
        existing.update(results)
        results = existing
    with open(path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, sort_keys=True, ensure_ascii=False)
    print(f"\n[collector] {len(results)} Werte in {path}")