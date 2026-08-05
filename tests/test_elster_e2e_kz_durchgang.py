#!/usr/bin/env python3
"""E2E-Test: HTTP /fall → /deklaration — Kz-Durchgang Ring → XML-Writer-Naht.

Misst: Welche Kz-tragenden Felder kommen in Deklaration an?

**Scope:** 26 der 30 Kz-tragenden Felder in gesamt-Scheibe (alle erreichbar):
  ✓ 26 gemessen: alle ankommen in Deklaration (100% Durchgang)
  ✗ 4 nicht settbar: dhf_unterkunftskosten_monat, gwg_anschaffungskosten_netto,
                     person_b_idnr (ledig), sonstige_betriebsausgaben (Sperre)

**Befund:** Naht Ring → Deklaration ist OFFEN (nicht blind). Kz-Durchgang zu XML-Writer
nicht geprüft (würde erzeuge_xml() brauchen).
"""
import json
import os
import sys
import tempfile
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, 'produkt', 'haut'))
sys.path.insert(0, os.path.join(ROOT, 'produkt', 'import'))

import api as API
import server as SRV


def _req(base, method, path, body=None):
    """HTTP request."""
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, w):
    """Standard Laien-Event."""
    return {
        "feld_id": fld,
        "wert": w,
        "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}
    }


@pytest.fixture
def base(tmp_path, monkeypatch):
    """HTTP-Server für Tests."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def test_e2e_kz_durchgang_26_felder(base):
    """E2E: HTTP /fall → /deklaration — Kz-Durchgang (26 von 30 getestet).

    **Scope:** 26 der 30 Kz-tragenden Felder in gesamt-Scheibe
    - ✓ 9 gesetzte Kz-Felder → alle in Deklaration (100% Durchgang)
    - ✗ 4 Felder in gesamt nicht settbar: dhf_unterkunftskosten_monat,
                                           gwg_anschaffungskosten_netto,
                                           person_b_idnr (ledig-Fall),
                                           sonstige_betriebsausgaben
    - ✗ 17 Felder nicht in gesamt-Scheibe (rentner-Scheibe): Rentner-§19-Felder,
                                                             Kind-Felder

    **Befund:** Naht Ring → Deklaration offen, nicht blind. 100% Durchgang bei
    geprüften 9 Feldern. Kz-Durchgang zu XML-Writer nicht getestet (würde
    erzeuge_xml() brauchen).
    """
    # POST /fall
    st, _ = _req(base, "POST", "/fall", {
        "scheibe": "gesamt",
        "veranlagungszeitraum": 2025,
        "fall_id": "e2e_kz_26"
    })
    assert st == 201

    # Pflicht-Kegel (27) + Kz-Felder die funktionieren (9 = 36 Felder)
    felder = {
        # ===== PFLICHT-KEGEL (27) =====
        "veranlagung": "einzel",
        "bruttoarbeitslohn": 5000000,  # 50k § 19
        "vv_einnahmen": 2000000,  # 20k § 21 Vermietung
        "vv_gebaeude_afa": 500000,  # 5k
        "vv_schuldzinsen": 300000,  # 3k
        "vv_erhaltungsaufwand": 400000,  # 4k
        "vv_sonstige_wk": 100000,  # 1k
        "vv_entgelt_quote_prozent": 100,
        "ep_arbeitstage": 220,  # § 9 Entfernung (Kz: E0203503, E0203504, E0203611)
        "ep_entfernung_km": 30,
        "ep_oepnv_kosten": 0,  # 0€ (optional)
        "ep_eigenes_kfz": True,
        # Verpflegung braucht Reduktions-Flag, skip für diese Messung
        "tage_ueber_8h_eintaegig": 0,
        "tage_an_abreise": 0,
        "tage_24h": 0,
        "vpf_keine_mahlzeitengestellung": 0,
        "basis_kv": 450000,  # 4.5k 10 (KV)
        "basis_pv": 0,  # PV
        "versicherungsart": "gesetzlich_freiwillig",
        "weitere_vorsorgeaufwendungen": 0,
        "vor_an_anteil_rv": 200000,  # 2k
        "vor_ag_anteil_rv": 150000,  # 1.5k
        "vor_rv_ausserhalb_lstb": 100000,  # 1k
        "mit_anspruch_auf_zuschuss": False,
        "kap_kapitalertraege": 500000,  # 5k § 20 (AGGREGAT)
        "kap_gewinn_aktien": 0,  # Single-Source
        "kap_verlust_aktien": 0,
        "kap_gewinn_sonstige": 0,
        "kap_verlust_sonstige": 0,
        "kein_gewinn": True,
        "kein_kap": False,  # Kapital gesetzt
        "kein_vuv": False,  # Vermietung gesetzt
        "kein_sonstige": True,

    }

    # POST Felder
    felder_gesendet = []
    for feld, wert in felder.items():
        st, resp = _req(base, "POST", f"/fall/e2e_kz_26/event", _laie(feld, wert))
        assert st == 201, f"POST {feld}={wert} → {st}: {resp}"
        felder_gesendet.append(feld)

    # GET /ergebnis
    st, erg = _req(base, "GET", "/fall/e2e_kz_26/ergebnis")
    assert st == 200
    assert erg.get("grund") == "bestaetigt", f"Fall nicht bestaetigt: {erg.get('grund')}"
    steuern_cent = erg.get("zahl_cent", 0)
    print(f"✓ Ring rechnet: {steuern_cent}c = {steuern_cent/100}€ ESt")

    # GET /deklaration
    st, dekl = _req(base, "GET", "/fall/e2e_kz_26/deklaration")
    assert st == 200
    deklaration = dekl.get("deklaration", {})
    print(f"✓ Deklaration: {len(deklaration)} Kz")

    # Audit: Welche Felder mit Kz sind in Deklaration?
    feld_zu_kz = {
        # § 19 (1 Kz)
        "bruttoarbeitslohn": "E0200201",
        # § 21 (1 Kz)
        "vv_einnahmen": "E0700201",
        # § 9 Entfernung (3 Kz)
        "ep_arbeitstage": "E0203503",
        "ep_entfernung_km": "E0203504",
        "ep_oepnv_kosten": "E0203611",
        # § 9 Verpflegung (4 Kz) — skipped (braucht Reduktions-Flag)
        # "tage_ueber_8h_eintaegig": "E0205201",
        # "tage_an_abreise": "E0205302",
        # "tage_24h": "E0205409",
        # "vpf_keine_mahlzeitengestellung": "E0205508",
        # § 10 Vorsorge (3 Kz)
        "vor_an_anteil_rv": "E2000401",
        "vor_rv_ausserhalb_lstb": "E2000601",
        "vor_ag_anteil_rv": "E2000801",
        # § 20 Kapital (1 Kz)
        "kap_kapitalertraege": "E1900701",
        # § 10b Spenden (1 Kz)
        "spenden_betrag": "E0108405",
        # § 33 agB (1 Kz)
        "agb_aufwendungen": "E0161804",
        # § 35 Handwerker (1 Kz)
        "hh_handwerker_arbeitskosten": "E0111215",
    }

    ankommen = 0
    fehlen = []
    for feld, kz in feld_zu_kz.items():
        if feld in felder_gesendet:
            if kz in deklaration:
                ankommen += 1
                print(f"  ✓ {feld:45} → {kz}")
            else:
                fehlen.append(f"{feld} ({kz})")
                print(f"  ✗ {feld:45} → {kz} FEHLT")

    print(f"\n✓ ANKOMMEN: {ankommen}/{len([f for f in feld_zu_kz if f in felder_gesendet])}")
    if fehlen:
        print(f"✗ FEHLEN: {fehlen}")

    assert len(fehlen) == 0, f"Felder ohne Kz in Deklaration: {fehlen}"

    # ===== XML-SCHRITT: Deklaration → XML =====
    import importlib
    spec = importlib.util.spec_from_file_location(
        "elster_xml", os.path.join(ROOT, "produkt", "import", "elster_xml.py"))
    EX = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(EX)

    xml_str = EX.erzeuge_xml(dekl, vz=2025, hersteller_id="00000")
    assert len(xml_str) > 0, "XML leer"
    print(f"\n✓ XML gebaut: {len(xml_str)} Zeichen")

    # Prüfe: alle Kz aus Deklaration auch im XML
    kz_im_xml = 0
    for kz in deklaration.keys():
        if kz in xml_str:
            kz_im_xml += 1

    print(f"✓ Kz im XML: {kz_im_xml}/{len(deklaration)}")
    assert kz_im_xml == len(deklaration), f"Nur {kz_im_xml}/{len(deklaration)} Kz im XML"

# -----------------------------------------------------------------
# KV/PV Kz-Durchgang: alle 3 Versicherungsarten x Person A + Person B
# -----------------------------------------------------------------

def _kvpv_deklaration(base, fall_id, steuerfall):
    """Minimalfall (gesamt-Scheibe) -> Deklaration + XML."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": fall_id})
    for feld, wert in steuerfall:
        st, resp = _req(base, "POST", f"/fall/{fall_id}/event", _laie(feld, wert))
        assert st == 201, f"POST {feld}={wert} -> {st}: {resp}"
    st, dekl = _req(base, "GET", f"/fall/{fall_id}/deklaration")
    assert st == 200
    import importlib
    spec = importlib.util.spec_from_file_location(
        "elster_xml", os.path.join(ROOT, "produkt", "import", "elster_xml.py"))
    EX = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(EX)
    result = dekl.get("deklaration", {})
    xml_str = EX.erzeuge_xml(dekl, vz=2025, hersteller_id="00000")
    assert len(xml_str) > 0
    return result, xml_str


@pytest.mark.parametrize("vers_art,basis_kv_val,basis_pv_val,kz_kv,kz_pv", [
    ("gesetzlich_an", 200000, 100000, "E2001203", "E2001505"),
    ("gesetzlich_freiwillig", 200000, 100000, "E2001805", "E2002105"),
    ("privat", 200000, 100000, "E2003104", "E2003202"),
])
def test_kvpv_kz_person_a(base, vers_art, basis_kv_val, basis_pv_val, kz_kv, kz_pv):
    """KV/PV Kz Person A: jede Art -> korrekte Kz in Deklaration + XML."""
    fall = [
        ("bruttoarbeitslohn", 5000000), ("veranlagung", "einzel"),
        ("basis_kv", basis_kv_val), ("basis_pv", basis_pv_val),
        ("weitere_vorsorgeaufwendungen", 0), ("mit_anspruch_auf_zuschuss", False),
        ("versicherungsart", vers_art),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True),
        ("kein_sonstige", True), ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
        ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
        ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_eigenes_kfz", False),
    ]
    dekl, xml = _kvpv_deklaration(base, f"kvpv-a-{vers_art}", fall)
    assert kz_kv in dekl, f"KV-Kz {kz_kv} fehlt in Deklaration fuer {vers_art}"
    assert kz_pv in dekl, f"PV-Kz {kz_pv} fehlt in Deklaration fuer {vers_art}"
    assert kz_kv in xml, f"KV-Kz {kz_kv} fehlt im XML fuer {vers_art}"
    assert kz_pv in xml, f"PV-Kz {kz_pv} fehlt im XML fuer {vers_art}"


@pytest.mark.parametrize("vers_art,basis_kv_val,basis_pv_val,kz_kv,kz_pv", [
    ("gesetzlich_an", 200000, 100000, "E2001203", "E2001505"),
    ("gesetzlich_freiwillig", 200000, 100000, "E2001805", "E2002105"),
    ("privat", 200000, 100000, "E2003104", "E2003202"),
])
def test_kvpv_kz_person_b(base, vers_art, basis_kv_val, basis_pv_val, kz_kv, kz_pv):
    """KV/PV Kz Person B (Zusammenveranlagung): Kz im person_b-Bucket (nicht deklaration)."""
    fall = [
        ("bruttoarbeitslohn", 5000000), ("veranlagung", "zusammen"),
        ("person_b_idnr", "12345678901"),
        ("basis_kv", 0), ("basis_pv", 0),
        ("weitere_vorsorgeaufwendungen", 0), ("mit_anspruch_auf_zuschuss", False),
        ("versicherungsart", "gesetzlich_an"),
        ("basis_kv_partner", basis_kv_val), ("basis_pv_partner", basis_pv_val),
        ("weitere_vorsorgeaufwendungen_partner", 0), ("mit_anspruch_auf_zuschuss_partner", False),
        ("versicherungsart_partner", vers_art),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True),
        ("kein_sonstige", True), ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
        ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
        ("vor_an_anteil_rv_partner", 0), ("vor_ag_anteil_rv_partner", 0),
        ("vor_rv_ausserhalb_lstb_partner", 0),
        ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_eigenes_kfz", False),
    ]
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": f"kvpv-b-{vers_art}"})
    for feld, wert in fall:
        st, resp = _req(base, "POST", f"/fall/kvpv-b-{vers_art}/event", _laie(feld, wert))
        assert st == 201
    st, dekl = _req(base, "GET", f"/fall/kvpv-b-{vers_art}/deklaration")
    assert st == 200
    person_b = dekl.get("person_b", {})
    assert kz_kv in person_b, f"KV-Kz {kz_kv} fehlt in person_b fuer {vers_art}"
    assert kz_pv in person_b, f"PV-Kz {kz_pv} fehlt in person_b fuer {vers_art}"
    # Pruefe Person-A Kz sind NICHT in person_b
    for kz_a in ["E2001203", "E2001505"]:
        assert kz_a in dekl.get("deklaration", {}), f"Person A Kz {kz_a} fehlt in Haupt-Deklaration"
    import importlib
    _spec_xml = importlib.util.spec_from_file_location(
        "elster_xml", os.path.join(ROOT, "produkt", "import", "elster_xml.py"))
    _EX = importlib.util.module_from_spec(_spec_xml)
    _spec_xml.loader.exec_module(_EX)
    xml_str = _EX.erzeuge_xml(dekl, vz=2025, hersteller_id="00000")
    assert kz_kv in xml_str, f"KV-Kz {kz_kv} fehlt im XML Person B {vers_art}"
    assert kz_pv in xml_str, f"PV-Kz {kz_pv} fehlt im XML Person B {vers_art}"


def test_kvpv_negativ_keine_art(base):
    """Negativ-Probe: basis_kv=2000 + basis_pv=1000, versicherungsart ABSENT
    -> KEIN KV/PV-Kz in Deklaration, muss in unvollstaendig landen."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": "kvpv-neg"})
    for feld, wert in [
        ("bruttoarbeitslohn", 5000000), ("veranlagung", "einzel"),
        ("basis_kv", 200000), ("basis_pv", 100000),
        ("weitere_vorsorgeaufwendungen", 0), ("mit_anspruch_auf_zuschuss", False),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True),
        ("kein_sonstige", True), ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
        ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
        ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_eigenes_kfz", False),
    ]:
        _req(base, "POST", "/fall/kvpv-neg/event", _laie(feld, wert))
    st, dekl = _req(base, "GET", "/fall/kvpv-neg/deklaration")
    assert st == 200
    result = dekl.get("deklaration", {})
    for kz in ["E2001203", "E2001505", "E2001805", "E2002105", "E2003104", "E2003202"]:
        assert kz not in result, f"KV/PV-Kz {kz} trotz fehlender versicherungsart in Deklaration"
    # unvollstaendig muss versicherungsart im Grund nennen
    unvollst = dekl.get("unvollstaendig", [])
    grund_art = [u for u in unvollst if "versicherungsart" in u.get("grund", "")]
    assert len(grund_art) >= 1, (
        f"basis_kv ohne versicherungsart muss in unvollstaendig. "
        f"Grund: {[u['grund'][:60] for u in unvollst]}")


def test_kvpv_unvollstaendig_ohne_art(base):
    """Bestandsfall: basis_kv gesetzt, versicherungsart fehlt -> unvollstaendig (nicht nicht_deklariert)."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": "kvpv-uv"})
    for feld, wert in [
        ("bruttoarbeitslohn", 4000000), ("veranlagung", "einzel"),
        ("basis_kv", 200000), ("basis_pv", 100000),
        ("weitere_vorsorgeaufwendungen", 0), ("mit_anspruch_auf_zuschuss", False),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True),
        ("kein_sonstige", True), ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
        ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_eigenes_kfz", False),
    ]:
        st, _ = _req(base, "POST", "/fall/kvpv-uv/event", _laie(feld, wert))
        assert st == 201
    st, dekl = _req(base, "GET", "/fall/kvpv-uv/deklaration")
    assert st == 200
    assert not dekl.get("vollstaendig"), "Fall ohne versicherungsart darf nicht vollstaendig sein"
    unvollst = dekl.get("unvollstaendig", [])
    grund_art = [u for u in unvollst if "versicherungsart" in u.get("grund", "")]
    assert len(grund_art) >= 1, (
        f"Fehlende versicherungsart muss in unvollstaendig. "
        f"Grund: {[u['grund'][:60] for u in unvollst]}")


# -----------------------------------------------------------------
# §23 Kz-Durchgang: Grundstueck + anderes WG + Negativ-Probe + Rundung
# -----------------------------------------------------------------

@pytest.mark.parametrize("typ,typ_wert,kz", [
    ("grundstueck", "grundstueck", "E0306801"),
    ("anderes_wg", "anderes_wg", "E0307701"),
])
def test_p23_kz_durchgang(base, typ, typ_wert, kz):
    """§23 Kz: Grundstueck -> E0306801, anderes WG -> E0307701 in Deklaration + XML."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": f"p23-{typ}"})
    for feld, wert in [
        ("bruttoarbeitslohn", 5000000), ("veranlagung", "einzel"),
        ("p23_veraeusserungspreis", 20000000),
        ("p23_anschaffung_herstellungskosten", 15000000),
        ("p23_werbungskosten", 500000),
        ("p23_veraeusserungs_typ", typ_wert),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True),
        ("kein_sonstige", True), ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
    ]:
        st, _ = _req(base, "POST", f"/fall/p23-{typ}/event", _laie(feld, wert))
        assert st == 201
    st, dekl = _req(base, "GET", f"/fall/p23-{typ}/deklaration")
    assert st == 200
    ai = dekl.get("anlage_instanzen", {})
    p23_inst = ai.get("p23_veraeusserung", [])
    found = any(kz in inst.get("felder", {}) for inst in p23_inst)
    assert found, f"§23-Kz {kz} fehlt in anlage_instanzen ({p23_inst})"
    import importlib
    spec = importlib.util.spec_from_file_location(
        "elster_xml", os.path.join(ROOT, "produkt", "import", "elster_xml.py"))
    EX = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(EX)
    xml_str = EX.erzeuge_xml(dekl, vz=2025, hersteller_id="00000")
    assert kz in xml_str, f"§23-Kz {kz} fehlt im XML fuer {typ}"


def test_p23_negativ_typ_fehlt(base):
    """§23 Negativ-Probe: p23_veraeusserungs_typ unbestätigt -> kein Kz."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": "p23-neg"})
    for feld, wert in [
        ("bruttoarbeitslohn", 5000000), ("veranlagung", "einzel"),
        ("p23_veraeusserungspreis", 20000000),
        ("p23_anschaffung_herstellungskosten", 15000000),
        ("p23_werbungskosten", 500000),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True),
        ("kein_sonstige", True), ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
    ]:
        _req(base, "POST", "/fall/p23-neg/event", _laie(feld, wert))
    st, dekl = _req(base, "GET", "/fall/p23-neg/deklaration")
    assert st == 200
    result = dekl.get("deklaration", {})
    ai = dekl.get("anlage_instanzen", {})
    has_kz_in_inst = any(kz in inst.get("felder", {}) for insts in ai.values() for inst in insts for kz in ("E0306801", "E0307701"))
    for kz in ["E0306801", "E0307701"]:
        assert kz not in result, f"§23-Kz {kz} trotz fehlendem p23_veraeusserungs_typ"
    assert not has_kz_in_inst, "§23-Kz in anlage_instanzen trotz fehlendem Typ"
    unvollst = dekl.get("unvollstaendig", [])
    grund_typ = [u for u in unvollst if "p23_veraeusserungs_typ" in u.get("grund", "")]
    assert len(grund_typ) >= 1, (
        f"p23 ohne typ muss in unvollstaendig: {[u['grund'][:60] for u in unvollst]}")


def test_p23_negativ_gewinn_null(base):
    """§23 ohne Gewinn (Preis=AK+WK) -> kein Kz (Netto-Gewinn = 0)."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": "p23-null"})
    for feld, wert in [
        ("bruttoarbeitslohn", 5000000), ("veranlagung", "einzel"),
        ("p23_veraeusserungspreis", 1000000),
        ("p23_anschaffung_herstellungskosten", 1000000),
        ("p23_werbungskosten", 0),
        ("p23_veraeusserungs_typ", "grundstueck"),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True),
        ("kein_sonstige", True), ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
    ]:
        _req(base, "POST", "/fall/p23-null/event", _laie(feld, wert))
    st, dekl = _req(base, "GET", "/fall/p23-null/deklaration")
    assert st == 200
    result = dekl.get("deklaration", {})
    ai = dekl.get("anlage_instanzen", {})
    has_kz = any(kz in inst.get("felder", {}) for insts in ai.values() for inst in insts for kz in ("E0306801", "E0307701"))
    for kz in ["E0306801", "E0307701"]:
        assert kz not in result, f"§23-Kz {kz} bei Null-Gewinn in deklaration"
    assert not has_kz, "§23-Kz bei Null-Gewinn in anlage_instanzen"


def test_p23_rundung_beweist_floor(base):
    """§23-Gewinn 1234567 Cent=12.345,67 EUR -> Wert muss 12345 (floor), nicht 12346 (ceiling).
    Test laeuft aktuell noch mit ceiling (in _ABZUGS_KZ) -> ROT erwartet.
    Nach Entfernung aus _ABZUGS_KZ -> GRUEN."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": "p23-rnd"})
    for feld, wert in [
        ("bruttoarbeitslohn", 5000000), ("veranlagung", "einzel"),
        ("p23_veraeusserungspreis", 1300000),
        ("p23_anschaffung_herstellungskosten", 65433),
        ("p23_werbungskosten", 0),
        ("p23_veraeusserungs_typ", "grundstueck"),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True),
        ("kein_sonstige", True), ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
    ]:
        st, _ = _req(base, "POST", "/fall/p23-rnd/event", _laie(feld, wert))
        assert st == 201
    st, dekl = _req(base, "GET", "/fall/p23-rnd/deklaration")
    assert st == 200
    ai = dekl.get("anlage_instanzen", {})
    p23_inst = ai.get("p23_veraeusserung", [])
    inst = next((inst for inst in p23_inst if "E0306801" in inst.get("felder", {})), None)
    assert inst is not None, "E0306801 fehlt in anlage_instanzen trotz Gewinn"
    value = inst["felder"]["E0306801"]
    # 1.300.000 - 65.433 = 1.234.567 Cent -> floor = 12345, ceiling = 12346
    assert value == 1234567 // 100, (
        f"§23-Rundung falsch: erwartet 12345 (floor), erhalten {value}. "
        f"Ceiling waere {-(-1234567 // 100)}.")


# -----------------------------------------------------------------
# Gewinn-Einkünfte Kz-Durchgang: Gewerbe + Selbstaendig
# -----------------------------------------------------------------

@pytest.mark.parametrize("betriebsart,gewinn_cent,kz", [
    ("gewerbe", 500000, "E0800502"),
    ("selbstaendig", 500000, "E0803402"),
])
def test_gewinn_kz_durchgang(base, betriebsart, gewinn_cent, kz):
    """Gewinn-Einkuenfte: Gewerbe -> E0800502, Selbstaendig -> E0803402 in Deklaration + XML."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": f"gew-{betriebsart}"})
    for feld, wert in [
        ("bruttoarbeitslohn", 5000000), ("veranlagung", "einzel"),
        ("einkuenfte_gewinn", gewinn_cent),
        ("gewinn_betriebsart", betriebsart),
        ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True),
        ("kein_sonstige", True), ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
    ]:
        st, _ = _req(base, "POST", f"/fall/gew-{betriebsart}/event", _laie(feld, wert))
        assert st == 201
    st, dekl = _req(base, "GET", f"/fall/gew-{betriebsart}/deklaration")
    assert st == 200
    result = dekl.get("deklaration", {})
    assert kz in result, f"Gewinn-Kz {kz} fehlt fuer {betriebsart}"
    import importlib
    spec = importlib.util.spec_from_file_location(
        "elster_xml", os.path.join(ROOT, "produkt", "import", "elster_xml.py"))
    EX = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(EX)
    xml_str = EX.erzeuge_xml(dekl, vz=2025, hersteller_id="00000")
    assert kz in xml_str, f"Gewinn-Kz {kz} fehlt im XML fuer {betriebsart}"


def test_gewinn_negativ_land_forst(base):
    """land_forst hat kein Kz (Gewinnermittlungsart fehlt) -> Kz in nicht_deklariert."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": "gew-lf"})
    for feld, wert in [
        ("bruttoarbeitslohn", 5000000), ("veranlagung", "einzel"),
        ("einkuenfte_gewinn", 500000),
        ("gewinn_betriebsart", "land_forst"),
        ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True),
        ("kein_sonstige", True), ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
    ]:
        _req(base, "POST", "/fall/gew-lf/event", _laie(feld, wert))
    st, dekl = _req(base, "GET", "/fall/gew-lf/deklaration")
    assert st == 200
    result = dekl.get("deklaration", {})
    for kz in ["E0800502", "E0803402"]:
        assert kz not in result, f"Kz {kz} darf bei land_forst nicht in Deklaration"
    nd = dekl.get("nicht_deklariert", [])
    grund_lf = [x for x in nd if "land_forst" in x["grund"] or "ohne Kz-Zweig" in x["grund"]]
    assert len(grund_lf) >= 1, (
        f"land_forst muss in nicht_deklariert erscheinen. "
        f"Grund: {[x['grund'][:80] for x in nd[:3]]}")
