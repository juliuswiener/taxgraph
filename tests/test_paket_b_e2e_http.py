"""Paket-B-Durchstich über HTTP — fährt tests/test_paket_a_e2e.py Schritt für Schritt über die
Haut-Endpunkte nach (gleiche EP-Familie, gleiche Asserts, gleiche 2156). Deterministisch, NULL LLM.

Auflagen (Instructor): (A) POST …/chat -> 501, nie 200-Fake; (B) Server bindet AUSSCHLIESSLICH
127.0.0.1 (asserted); (C) api_schema/*.json wird gegen die echten Responses validiert. Der
numerische Teil (Spanne, 2156) hängt an der Catala-Toolchain -> sauberer Skip wie im A-Test.
"""
from __future__ import annotations

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
import store as ST       # noqa: E402

jsonschema = pytest.importorskip("jsonschema")

SCHEMA_DIR = os.path.join(ROOT, "produkt", "haut", "api_schema")
EP_FELDER = {"ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz"}


def _schema(name: str) -> dict:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        return json.load(f)


def _val(name: str, obj: dict) -> None:
    jsonschema.Draft202012Validator(_schema(name)).validate(obj)


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


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


def _llm(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "vorlaeufig",
            "herkunft": {"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "llm:chat", "signal": {"signal_1": None, "signal_2": None}}


@pytest.fixture
def base(tmp_path, monkeypatch):
    # Fall-Daten in ein temporäres Verzeichnis (nie ins Repo, nie in den echten faelle/-Ordner)
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)                      # port=0 -> freier Port
    assert srv.server_address[0] == "127.0.0.1", "Auflage B: Server muss an 127.0.0.1 binden"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=2)


def test_bindet_nur_localhost():
    """Auflage B, hart: die Bind-Adresse ist 127.0.0.1, niemals 0.0.0.0."""
    srv = SRV.make_server(0)
    try:
        assert srv.server_address[0] == "127.0.0.1"
    finally:
        srv.server_close()


def test_chat_501(base):
    """Auflage A: POST /chat liefert 501 mit erklärendem Vertrag, NIE 200."""
    _req(base, "POST", "/fall", {"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": "c1"})
    st, b = _req(base, "POST", "/fall/c1/chat", {"text": "hallo"})
    assert st == 501, f"chat muss 501 sein, war {st}"
    assert "vertrag" in b and "stufe" in b
    assert b.get("fehler") == "not_implemented"


def test_fail_closed_llm_kann_nicht_bestaetigen(base):
    """Der fail-closed-Store weist ein llm:-bestaetigt-Event ab — über HTTP -> 422, nie 201."""
    _req(base, "POST", "/fall", {"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": "f1"})
    boese = {"feld_id": "ep_arbeitstage", "wert": 220, "zustand": "bestaetigt",
             "herkunft": {"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
             "schreiber": "llm:chat", "signal": {"signal_1": None, "signal_2": "gefaelscht"}}
    st, b = _req(base, "POST", "/fall/f1/event", boese)
    assert st == 422, f"llm-bestaetigt muss abgewiesen werden, war {st}"
    assert "fail-closed" in b["fehler"]


def test_schema_gate_negativ():
    """Auflage C, Härtung: ein verfälschtes Objekt MUSS das Schema-Gate rot färben."""
    kaputt = {"fall_id": "x", "snapshot_id": "y", "fragen": [{"feld_id": "a", "typ": "UNBEKANNT"}]}
    with pytest.raises(jsonschema.ValidationError):
        _val("fragen", kaputt)


def test_durchstich_http(base):
    catala = _catala_da()
    fid = "e2e-ep"

    # 0) Fall anlegen
    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201 and b["fall_id"] == fid

    # 1) leerer Fall -> fragen == die 4 EP-Felder
    st, b = _req(base, "GET", f"/fall/{fid}/fragen")
    assert st == 200
    _val("fragen", b)                                          # Auflage C
    assert {q["feld_id"] for q in b["fragen"]} == EP_FELDER
    # Fragetexte kommen aus der Bindung (laienverständlich, kein §)
    assert all("§" not in (q["fragetext_laie"] or "") for q in b["fragen"])

    # 2) 3x laie-bestätigt + 1x llm-VORLÄUFIG
    for fld, w in [("ep_entfernung_km", 30), ("ep_eigenes_kfz", True), ("ep_oepnv_kosten", 0)]:
        st, b = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
        assert st == 201
        _val("event", b)
    st, llm = _req(base, "POST", f"/fall/{fid}/event", _llm("ep_arbeitstage", 220))
    assert st == 201
    llm_ev = llm["event_id"]

    # nur das offene Feld bleibt in der Queue
    st, b = _req(base, "GET", f"/fall/{fid}/fragen")
    assert [q["feld_id"] for q in b["fragen"]] == ["ep_arbeitstage"]

    # 3) stand: arbeitstage schimmernd (KI), Spanne offen (nur mit Engine numerisch)
    st, stand_a = _req(base, "GET", f"/fall/{fid}/stand")
    assert st == 200
    _val("stand", stand_a)
    assert stand_a["felder"]["ep_arbeitstage"]["herkunft_badge"] == "schimmernd"
    assert stand_a["felder"]["ep_entfernung_km"]["herkunft_badge"] == "solide"
    spanne_a = None
    if catala:
        iv = stand_a["intervall"]
        assert iv["max_cent"] - iv["min_cent"] > 0, "offener arbeitstage -> Spanne > 0"
        spanne_a = iv["max_cent"] - iv["min_cent"]

    # 4) FAIL-CLOSED vorher: Input-Kegel enthält vorlaeufig -> keine feste Zahl
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    assert st == 200
    _val("ergebnis", erg)
    assert erg["zahl_cent"] is None
    assert erg["grund"] == "input_kegel_nicht_bestaetigt"

    # 5) LLM-Wert via ZWEI-SIGNAL bestätigen (ersetzt das llm-Event)
    st, b = _req(base, "POST", f"/fall/{fid}/event", {
        "feld_id": "ep_arbeitstage", "wert": 220, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": llm_ev, "signal_2": "klick@beleg_arbeitstage"}, "ersetzt": llm_ev})
    assert st == 201

    # 6) stand: Spanne schrumpft auf Punkt (monoton)
    st, stand_b = _req(base, "GET", f"/fall/{fid}/stand")
    if catala:
        iv2 = stand_b["intervall"]
        spanne_b = iv2["max_cent"] - iv2["min_cent"]
        assert spanne_b < spanne_a and spanne_b == 0

    # 7) FAIL-CLOSED nachher: Kegel bestätigt -> echte Zahl (Naht-Einheit CENT: 215600 = 2156,00 €,
    #    wie test_paket_a_e2e nach der Einheiten-Konvention ad4e22b)
    st, erg2 = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg2)
    if catala:
        assert erg2["zahl_cent"] == 215600
        assert erg2["grund"] == "bestaetigt"
    else:
        assert erg2["zahl_cent"] is None and erg2["grund"] == "engine_unavailable"

    # Vorwärts-Trace/Justification bis anker_ref
    st, w = _req(base, "GET", f"/fall/{fid}/feld/ep_arbeitstage/warum")
    assert st == 200
    _val("warum", w)
    j = w["justification"]
    assert j["herkunft"]["herkunft"] == "laie"      # nach Bestätigung: laie-Beleg
    assert j["zustand"] == "bestaetigt" and j["signal"]["signal_2"]
    assert j["anker_ref"]["zitatanker"] == "jeden Arbeitstag"


# VOR-Summanden (3 Laien-Felder -> EIN signatur_slot gesamtbeitraege_inkl_ag) + GWG-bool.
VOR_FELDER = {"vor_an_anteil_rv": 3500000, "vor_ag_anteil_rv": 3500000, "vor_rv_ausserhalb_lstb": 0}
VOR_KZ = {"E2000401", "E2000801", "E2000601"}


def test_durchstich_n_vor_gwg(base):
    """Option A: Multi-Regel-Scheibe als Interview+Deklaration+Trace. KEIN Gesamt-Bescheid
    (ehrlich), EP behält seinen Teil-Ring, VOR-Summen-Konvention + GWG-bool durch die Haut."""
    catala = _catala_da()
    fid = "e2e-nvg"

    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "n_vor_gwg", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201 and b["scheibe"] == "n_vor_gwg"

    # 1) Interview-Queue enthält alle sechs Regel-Familien (askable Felder), laienverständlich
    st, b = _req(base, "GET", f"/fall/{fid}/fragen")
    assert st == 200
    _val("fragen", b)
    ids = {q["feld_id"] for q in b["fragen"]}
    assert EP_FELDER <= ids                                   # N/EP
    assert set(VOR_FELDER) <= ids                             # VOR (3 Summanden getrennt abgefragt)
    assert {"gwg_netto_ohne_vorsteuer", "gwg_anschaffungskosten_netto"} <= ids   # GWG
    assert any(q["feld_id"].startswith("dhf_") for q in b["fragen"])   # dHf
    assert all("§" not in (q["fragetext_laie"] or "") for q in b["fragen"])

    # 2) Repräsentative Bestätigung: EP (4) + VOR-Split (3) + GWG (bool + cent)
    for fld, w in [("ep_arbeitstage", 220), ("ep_entfernung_km", 30),
                   ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", True)]:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
        assert st == 201
    for fld, w in VOR_FELDER.items():
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
        assert st == 201
    for fld, w in [("gwg_netto_ohne_vorsteuer", True), ("gwg_anschaffungskosten_netto", 60000)]:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
        assert st == 201

    # 3) stand: KEIN Gesamt-Bescheid (K2), EP-Teil-Ring ehrlich getrennt
    st, stand = _req(base, "GET", f"/fall/{fid}/stand")
    assert st == 200
    _val("stand", stand)
    assert stand["intervall"] is None, "Multi-Regel-Scheibe darf keinen Gesamt-Bescheid erfinden"
    if catala:
        assert stand["engine"] == "catala_teilweise"
        assert any(t["familie"] == "ep_werbungskosten" for t in stand["teil_ringe"])
        ep_ring = next(t for t in stand["teil_ringe"] if t["familie"] == "ep_werbungskosten")
        assert ep_ring["intervall"]["min_cent"] == ep_ring["intervall"]["max_cent"] == 215600
    else:
        assert stand["engine"] == "unavailable"
        assert stand["teil_ringe"] == []

    # 4) ergebnis: bewusst KEINE Scheiben-Zahl (kein ehrlicher Gesamt-Accessor)
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    assert st == 200
    _val("ergebnis", erg)
    assert erg["zahl_cent"] is None
    assert erg["grund"] == "kein_scheiben_gesamtbescheid"

    # 5) Deklaration via est_mapping: die drei getrennt erfragten VOR-Felder sind deklariert
    st, dek = _req(base, "GET", f"/fall/{fid}/deklaration")
    assert st == 200
    kz = set(dek.get("deklaration", {}))
    assert VOR_KZ <= kz, f"VOR-Summanden-Kz fehlen in der Deklaration: {kz}"
    assert "E0203503" in kz                                   # ep_arbeitstage
    assert dek["vollstaendig"] is True                        # alle erfassten Felder bestätigt

    # 6) Trace bis anker_ref für ein VOR-Feld
    st, w = _req(base, "GET", f"/fall/{fid}/feld/vor_an_anteil_rv/warum")
    assert st == 200
    _val("warum", w)
    assert w["justification"]["signatur_slot"] == "gesamtbeitraege_inkl_ag"   # Summen-Slot


# ---- Gesamtsteuer-Ring MVP (an_gesamt): erster echter §2-Bescheid, reiner Arbeitnehmerfall ----
AN_GESAMT_VOR = ("vor_an_anteil_rv", "vor_ag_anteil_rv", "vor_rv_ausserhalb_lstb")
AN_GESAMT_DHF = ("dhf_unterkunftskosten_monat", "dhf_monate", "dhf_im_inland",
                 "dhf_beruflich_veranlasst", "dhf_eigener_hausstand",
                 "dhf_finanzielle_beteiligung", "dhf_keine_pflicht_dienstwohnung")
AN_GESAMT_PARTNER = ("bruttoarbeitslohn_partner", "person_b_idnr",
                     "vor_an_anteil_rv_partner", "vor_ag_anteil_rv_partner",
                     "vor_rv_ausserhalb_lstb_partner")
AN_GESAMT_VERPFLEGUNG = ("tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig",
                         "vpf_monate_am_ort", "vpf_keine_mahlzeitengestellung")
AN_GESAMT_KEGEL = [
    ("bruttoarbeitslohn", 4000000),   # 40000 € in Cent (Bindung typ:cent)
    ("veranlagung", "einzel"),
    ("ep_arbeitstage", 220), ("ep_entfernung_km", 30), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", True),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),   # reiner Pendler: keine VOR
    # reiner Pendler: keine dHf (Kosten 0 -> dHf-Abzug 0, Bedingungen egal aber bestätigt)
    ("dhf_unterkunftskosten_monat", 0), ("dhf_monate", 0), ("dhf_im_inland", True),
    ("dhf_beruflich_veranlasst", True), ("dhf_eigener_hausstand", True),
    ("dhf_finanzielle_beteiligung", True), ("dhf_keine_pflicht_dienstwohnung", True),
    # reiner Pendler: keine Verpflegung (alle Tage 0 → Verpflegungs-Abzug 0, Guard irrelevant)
    ("tage_24h", 0), ("tage_an_abreise", 0), ("tage_ueber_8h_eintaegig", 0),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
]


def _an_gesamt_anlegen(base, fid, kegel=AN_GESAMT_KEGEL):
    st, _ = _req(base, "POST", "/fall",
                 {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


def _store_append(fid, feld_id, wert, zustand="bestaetigt"):
    """Setzt ein Feld direkt im Fall-Store (simuliert Erfassung außerhalb des an_gesamt-Interviews) —
    für den Guard-Negativtest. Gleicher Prozess/FAELLE wie der Server."""
    s = API.lade_fall(fid)
    herk = ({"herkunft": "llm_vorschlag"} if zustand == "vorlaeufig" else {"herkunft": "laie"})
    herk.update({"pruef_tiefe": "ungeprueft", "haftung": "nutzer"})
    schreiber = "llm:chat" if zustand == "vorlaeufig" else "ui:laie"
    sig = {"signal_1": None, "signal_2": None if zustand == "vorlaeufig" else "ok"}
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand=zustand, herkunft=herk,
                    schreiber=schreiber, signal=sig)
    API.speichere_fall(fid, s)


def test_an_gesamt_durchstich(base):
    """MVP-Durchstich: voller bestätigter Kegel → echte festzusetzende_est 662900 (=6629 €)."""
    catala = _catala_da()
    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "an_gesamt", "veranlagungszeitraum": 2025, "fall_id": "ag"})
    assert st == 201
    st, fr = _req(base, "GET", "/fall/ag/fragen")
    _val("fragen", fr)
    ids = {q["feld_id"] for q in fr["fragen"]}
    assert ({"bruttoarbeitslohn", "veranlagung", "kein_gewinn", "kein_kap", "kein_vuv",
             "kein_sonstige"} | set(EP_FELDER) | set(AN_GESAMT_VOR) | set(AN_GESAMT_DHF)
            | set(AN_GESAMT_PARTNER) | set(AN_GESAMT_VERPFLEGUNG)) == ids
    for feld, wert in AN_GESAMT_KEGEL:
        st, _ = _req(base, "POST", "/fall/ag/event", _laie(feld, wert))
        assert st == 201
    st, stand = _req(base, "GET", "/fall/ag/stand")
    _val("stand", stand)
    assert stand["ring_gesperrt"] is None
    st, erg = _req(base, "GET", "/fall/ag/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert stand["engine"] == "catala"
        assert stand["intervall"]["min_cent"] == stand["intervall"]["max_cent"] == 662900
        assert erg["zahl_cent"] == 662900 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_an_gesamt_flag_guard(base):
    """kein_kap=false (Nutzer HAT Kapitalerträge) → Ring unzulässig, kein Fake-Bescheid."""
    kegel = [(f, (False if f == "kein_kap" else v)) for f, v in AN_GESAMT_KEGEL]
    _an_gesamt_anlegen(base, "ag_kap", kegel)
    st, erg = _req(base, "GET", "/fall/ag_kap/ergebnis")
    assert erg["zahl_cent"] is None
    assert erg["grund"] == "einkunftsart_nicht_ring_faehig"


def test_an_gesamt_vor_integration(base):
    """Stufe 1a: VOR (§ 10) echt gerechnet. Bruttolohn 40000 + EP 2156 + VOR (AN 3500 / AG 3500,
    Cent) → Sonderausgaben 3500 (nach Cap-vor-Kürzung) → festzusetzende_est 5570 = 557000 Cent
    (Golden B). Der AG-Anteil wird über den Store-Einzelfeld-Zugriff getrennt behandelt."""
    catala = _catala_da()
    kegel = [(f, w) for f, w in AN_GESAMT_KEGEL if not f.startswith("vor_")]
    kegel += [("vor_an_anteil_rv", 350000), ("vor_ag_anteil_rv", 350000), ("vor_rv_ausserhalb_lstb", 0)]
    _an_gesamt_anlegen(base, "ag_vi", kegel)
    st, erg = _req(base, "GET", "/fall/ag_vi/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 557000 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_an_gesamt_am_guard_vorlaeufig(base):
    """Arbeitsmittel (noch kein Modell) bleibt im Guard: ein vorläufiges am-Feld > 0 sperrt (kein Fake)."""
    _an_gesamt_anlegen(base, "ag_am")
    _store_append("ag_am", "am_anschaffungskosten", 60000, zustand="vorlaeufig")
    st, stand = _req(base, "GET", "/fall/ag_am/stand")
    assert stand["engine"] == "gesperrt"
    assert stand["ring_gesperrt"] == "werbungskosten_nicht_ring_faehig"
    st, erg = _req(base, "GET", "/fall/ag_am/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "werbungskosten_nicht_ring_faehig"


def _dhf_kegel(kosten, im_inland=True, weglassen=()):
    """Voller an_gesamt-Kegel mit dHf-Kosten > 0 (Miete 1400 € = 140000 ct, 12 Monate); Bedingungen
    bestätigt-true außer den in `weglassen` genannten (die bleiben offen)."""
    kegel = [(f, w) for f, w in AN_GESAMT_KEGEL if not f.startswith("dhf_")]
    kegel += [("dhf_unterkunftskosten_monat", kosten), ("dhf_monate", 12), ("dhf_im_inland", im_inland)]
    for b in ("dhf_beruflich_veranlasst", "dhf_eigener_hausstand",
              "dhf_finanzielle_beteiligung", "dhf_keine_pflicht_dienstwohnung"):
        if b not in weglassen:
            kegel.append((b, True))
    return kegel


def test_an_gesamt_dhf_ring(base):
    """Stufe 1b: dHf echt gerechnet. EP 2156 + dHf (1400 → gekappt 1000 × 12 = 12000) → WK 14156
    → festzusetzende_est 3143 = 314300 Cent (Golden an_2025_einzel_ep_dhf)."""
    catala = _catala_da()
    _an_gesamt_anlegen(base, "ag_dhf", _dhf_kegel(140000))
    st, erg = _req(base, "GET", "/fall/ag_dhf/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 314300 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_an_gesamt_dhf_tatbestand_offen(base):
    """K2: dHf-Kosten > 0, aber eine Geltungsbedingung nie bestätigt → Ring gesperrt (kein Abzug
    ohne Tatbestand, kein 3143-Fake)."""
    _an_gesamt_anlegen(base, "ag_to", _dhf_kegel(140000, weglassen=("dhf_keine_pflicht_dienstwohnung",)))
    st, erg = _req(base, "GET", "/fall/ag_to/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "dhf_tatbestand_offen"


def test_an_gesamt_dhf_ausland(base):
    """Ausland-dHf ist benannte Lücke MIT Guard: im_inland=false + Kosten > 0 → Ring gesperrt."""
    _an_gesamt_anlegen(base, "ag_ausl", _dhf_kegel(140000, im_inland=False))
    st, erg = _req(base, "GET", "/fall/ag_ausl/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "ausland_dhf_nicht_ring_faehig"


def _verpflegung_kegel(monate=2, keine_mahlzeit=True, tage_24h=10):
    """an_gesamt-Kegel mit Verpflegungs-Reisetagen. Guard-Felder werden nur gesetzt, wenn nicht None
    (None simuliert den UNSET-Fall für den fail-closed-Test)."""
    kegel = [(f, w) for f, w in AN_GESAMT_KEGEL if f != "tage_24h"]
    kegel.append(("tage_24h", tage_24h))
    if monate is not None:
        kegel.append(("vpf_monate_am_ort", monate))
    if keine_mahlzeit is not None:
        kegel.append(("vpf_keine_mahlzeitengestellung", keine_mahlzeit))
    return kegel


def test_an_gesamt_verpflegung_ring(base):
    """Stufe 1b: Verpflegung echt gerechnet. EP 2156 + Verpflegung 280 (10 volle Tage à 28) → WK 2436
    → festzusetzende_est 6542 = 654200 Cent. Reduktion explizit safe (≤3 Monate, keine Mahlzeiten)."""
    catala = _catala_da()
    _an_gesamt_anlegen(base, "vpf", _verpflegung_kegel())
    st, erg = _req(base, "GET", "/fall/vpf/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 654200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_an_gesamt_verpflegung_reduktion_unset(base):
    """fail-closed-on-unset (Instructor-Härtung): Reisetage > 0, aber die Reduktions-Fragen
    (3-Monats-Frist / Mahlzeitenkürzung) UNBEANTWORTET → Ring gesperrt, kein stiller Über-Abzug."""
    _an_gesamt_anlegen(base, "vpu", _verpflegung_kegel(monate=None, keine_mahlzeit=None))
    st, erg = _req(base, "GET", "/fall/vpu/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "verpflegung_reduktion_offen"


def _zusammen_kegel(vor_a=0, ohne=()):
    """Zusammenveranlagung-Kegel: einzel-Basis mit veranlagung=zusammen, KEINE EP (WK_a=0),
    beide 40000 → festzusetzende_est_zusammen 13838. + Partner-Pflichtfelder (außer `ohne`)."""
    k = dict(AN_GESAMT_KEGEL)
    k["veranlagung"] = "zusammen"
    k["ep_arbeitstage"] = 0
    k["ep_entfernung_km"] = 0
    k["ep_eigenes_kfz"] = False
    k["vor_an_anteil_rv"] = vor_a
    kegel = list(k.items())
    for f, w in [("bruttoarbeitslohn_partner", 4000000), ("person_b_idnr", "00000000000")]:
        if f not in ohne:
            kegel.append((f, w))
    return kegel


def test_an_gesamt_zusammen(base):
    """Front 2: Splitting-Ring. Beide Bruttolohn 40000, keine WK/VOR → festzusetzende_est_zusammen
    13838 = 1383800 Cent (Golden an_2025_zusammen_gleich). §9a je Person + Splitting IM Scope."""
    catala = _catala_da()
    _an_gesamt_anlegen(base, "zus", _zusammen_kegel())
    st, erg = _req(base, "GET", "/fall/zus/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1383800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_an_gesamt_zusammen_partner_offen(base):
    """K2: Person-B-Pflichtfeld (person_b_idnr) offen → kein halber Splitting-Bescheid."""
    _an_gesamt_anlegen(base, "zpo", _zusammen_kegel(ohne=("person_b_idnr",)))
    st, erg = _req(base, "GET", "/fall/zpo/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "partner_kegel_offen"


def test_an_gesamt_zusammen_vor_guard(base):
    """MVP-zusammen ohne VOR: ein VOR-Feld (A oder B) > 0 → Ring gesperrt (kein VOR-loser
    Splitting-Bescheid, wenn VOR-Daten vorliegen)."""
    _an_gesamt_anlegen(base, "zvg", _zusammen_kegel(vor_a=350000))
    st, erg = _req(base, "GET", "/fall/zvg/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "partner_vor_offen"


def _gesamt_kegel(einnahmen, afa=0, schuldzinsen=0, kein_vuv=False, bruttolohn=0,
             ep_tage=0, ep_km=0, ep_kfz=False, kein_kap=True, kap_ertraege=0,
             kap_gewinn_aktien=0, kap_verlust_aktien=0, kap_gewinn_sonstige=0, kap_verlust_sonstige=0):
    """gesamt-Kegel (§ 19 + § 21 + § 20): § 21 (Einnahmen/WK) + § 19 (Bruttolohn in Cent + EP) + § 20
    Kapital (E0121709-Aggregat ODER Aktien/sonstige-Töpfe, in Cent) — je 0 = Einkunftsart abwesend
    (bestätigte Null) — + veranlagung + Flags. kein_vuv=false wenn V+V vorhanden, kein_kap=false wenn Kapital."""
    return [("vv_einnahmen", einnahmen), ("vv_gebaeude_afa", afa), ("vv_schuldzinsen", schuldzinsen),
            ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("veranlagung", "einzel"),
            ("bruttoarbeitslohn", bruttolohn),
            ("ep_arbeitstage", ep_tage), ("ep_entfernung_km", ep_km),
            ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", ep_kfz),
            ("kap_kapitalertraege", kap_ertraege), ("kap_gewinn_aktien", kap_gewinn_aktien),
            ("kap_verlust_aktien", kap_verlust_aktien), ("kap_gewinn_sonstige", kap_gewinn_sonstige),
            ("kap_verlust_sonstige", kap_verlust_sonstige), ("kap_zusammenveranlagung", False),
            ("kein_gewinn", True), ("kein_kap", kein_kap), ("kein_vuv", kein_vuv), ("kein_sonstige", True)]


def _gesamt_anlegen(base, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


def test_gesamt_vermieter(base):
    """Front V+V: reiner Vermieter-Fall (Bruttolohn = bestätigte Null). § 21-Einkünfte 30000
    (Einnahmen − WK) → catala_gesamt → festzusetzende_est 4293 = 429300 Cent (§ 10c-Pauschbetrag 36
    abgezogen; vermieter_only-Golden)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "vv", _gesamt_kegel(3000000))   # 30000 € in Cent, kein Job
    st, erg = _req(base, "GET", "/fall/vv/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 429300 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_verlust(base):
    """K2: § 21-Verlust (WK > Einnahmen: 8000 − 6000 − 4000 = −2000) → festzusetzende_est 0,
    NIE negativ (keine Negativsteuer)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "vvl", _gesamt_kegel(800000, afa=600000, schuldzinsen=400000))
    st, erg = _req(base, "GET", "/fall/vvl/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 0 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_gesamt_flag_widerspruch(base):
    """K2: kein_vuv=true (behauptet keine V+V) UND vv_einnahmen > 0 bestätigt → Widerspruch surfacen,
    keine still übergangene Einkunftsart."""
    _gesamt_anlegen(base, "vvw", _gesamt_kegel(3000000, kein_vuv=True))
    st, erg = _req(base, "GET", "/fall/vvw/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "flag_konsistenz_offen"


def test_kombiniert_job_und_vermietung(base):
    """Konvergenz § 19 + § 21: Arbeitnehmer (Bruttolohn 40000, kein Pendel-WK → § 19-Einkünfte 38770)
    MIT Vermietung 18770 → catala_gesamt summiert (§ 2 Abs. 3) → festzusetzende_est 13452 = 1345200 Cent."""
    catala = _catala_da()
    _gesamt_anlegen(base, "kjv", _gesamt_kegel(1877000, bruttolohn=4000000))   # vv 18770 €, Job 40000 € (Cent)
    st, erg = _req(base, "GET", "/fall/kjv/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1345200 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_kombiniert_verlust_mindert_lohn(base):
    """K2-KERN: § 21-Verlust (8000 − 6000 − 7000 = −5000) mindert nach § 2 Abs. 3 den § 19-Lohn
    (Einkünfte 38770) → festzusetzende_est 5388 = 538800 Cent, KLEINER als die reine § 19-ESt (6919).
    Der Verlust wird verrechnet, NICHT verschluckt — und floort nicht auf Negativsteuer."""
    catala = _catala_da()
    _gesamt_anlegen(base, "kvl", _gesamt_kegel(800000, afa=600000, schuldzinsen=700000, bruttolohn=4000000))
    st, erg = _req(base, "GET", "/fall/kvl/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 538800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_kombiniert_mit_pendel_wk(base):
    """Kombiniert mit § 19-Werbungskosten: Job 40000 + Entfernungspauschale (220 Tage, 30 km, Kfz
    = 2156 → § 19-Einkünfte 37844 statt 38770) + Vermietung 18770 → festzusetzende_est 13101 =
    1310100 Cent, KLEINER als ohne Pendel-WK (13452). Belegt, dass die EP-Slots (ep_arbeitstage →
    arbeitstage …) im gesamt-Kegel korrekt in die § 19-WK durchgereicht werden."""
    catala = _catala_da()
    _gesamt_anlegen(base, "kpw", _gesamt_kegel(1877000, bruttolohn=4000000,
                                       ep_tage=220, ep_km=30, ep_kfz=True))
    st, erg = _req(base, "GET", "/fall/kpw/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 1310100 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_kombiniert_job_und_kapital(base):
    """Konvergenz § 19 + § 20: Job 60000 (§ 19-Einkünfte 58770) + Kapitalerträge 10000 → nach Sparer-PB
    9000; Günstigerprüfung § 32d Abs. 6: Grenzsteuer > 25 % → Abgeltung 2250 gewinnt → festzusetzende_est
    est_ohne 13924 + 2250 = 16174 = 1617400 Cent (dev-2s K4-Zielwert, Brutto-Pipeline)."""
    catala = _catala_da()
    _gesamt_anlegen(base, "kjk", _gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True,
                                       kein_kap=False, kap_ertraege=1000000))
    st, erg = _req(base, "GET", "/fall/kjk/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 1617400 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_reines_kapital_guenstiger_null(base):
    """K2/Günstiger: reiner Kapitalfall (kein anderes Einkommen), Erträge 5000 → nach Sparer-PB 4000;
    Grundtarif auf 4000 = 0 < Abgeltung 1000 → Günstiger greift → festzusetzende_est 0."""
    catala = _catala_da()
    _gesamt_anlegen(base, "rkg", _gesamt_kegel(0, kein_vuv=True, kein_kap=False, kap_ertraege=500000))
    st, erg = _req(base, "GET", "/fall/rkg/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 0 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_kapital_semantik_offen_co_okkurrenz(base):
    """K2 fail-closed (Instructor-Q1): E0121709-Aggregat UND Aktien-Topf beide gesetzt → additiv-vs-subset
    ungeklärt → kapital_semantik_offen (kein Rate-Bescheid, kein stilles Verschlucken des Aggregats)."""
    _gesamt_anlegen(base, "kso", _gesamt_kegel(0, kein_vuv=True, kein_kap=False,
                                       kap_ertraege=500000, kap_gewinn_aktien=300000))
    st, erg = _req(base, "GET", "/fall/kso/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "kapital_semantik_offen"


def _rentner_kegel(renten_art="gesetzliche_rente", jahresrente=2000000, beginn=2025, alter=0,
                   gdb=0, hilflos=False, pflegegrad=0, gepflegter_hilflos=False, hinterbliebenen=False,
                   veranlagung="einzel", rentenfreibetrag=None, gdb_partner=0, hilflos_partner=False):
    """rentner_gesamt-Kegel: § 22 (renten_art/jahresrente Cent/beginn/alter) + § 33b-Block + Flags
    (kein_sonstige=False = Rente IST § 22-sonstige; kein_gewinn/kap/vuv=True). rentenfreibetrag (Cent) nur
    aa-Folgejahr; Partner-Behinderung nur zusammen."""
    k = [("rentner_renten_art", renten_art), ("rentner_jahresrente", jahresrente),
         ("rentner_renten_beginn_jahr", beginn), ("rentner_alter_bei_rentenbeginn", alter),
         ("rentner_grad_der_behinderung", gdb), ("rentner_hilflos_blind_taubblind", hilflos),
         ("rentner_pflegegrad", pflegegrad), ("rentner_gepflegter_hilflos", gepflegter_hilflos),
         ("rentner_hinterbliebenenbezuege", hinterbliebenen), ("veranlagung", veranlagung),
         ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False)]
    if rentenfreibetrag is not None:
        k.append(("rentner_rentenfreibetrag", rentenfreibetrag))
    if gdb_partner:
        k.append(("rentner_grad_der_behinderung_partner", gdb_partner))
    if hilflos_partner:
        k.append(("rentner_hilflos_blind_taubblind_partner", hilflos_partner))
    return k


def _rentner_anlegen(base, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


def test_rentner_gesetzl_erstjahr(base):
    """§ 22 gesetzl. Rente Erstjahr 20000 @ 83,5 % Kohorte → einkuenfte_sonstige 16598 →
    festzusetzende_est 811 = 81100 Cent (dev-2 Kreuzprobe S1)."""
    catala = _catala_da()
    _rentner_anlegen(base, "r1", _rentner_kegel(jahresrente=2000000, beginn=2025))
    st, erg = _req(base, "GET", "/fall/r1/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["zahl_cent"] == 81100 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_private_leibrente(base):
    """§ 22 bb private Leibrente 80000 @ Alter 65 (Ertragsanteil 18 %) → es 14298 → 346 = 34600 Cent (S4)."""
    catala = _catala_da()
    _rentner_anlegen(base, "r4", _rentner_kegel(renten_art="private_leibrente", jahresrente=8000000, alter=65))
    st, erg = _req(base, "GET", "/fall/r4/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 34600 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_behinderung(base):
    """§ 22 + § 33b: gesetzl. Rente Erstjahr (es 16598) + GdB 50 (Pauschbetrag 1140 agB) → 568 = 56800 Cent (S5)."""
    catala = _catala_da()
    _rentner_anlegen(base, "r5", _rentner_kegel(jahresrente=2000000, beginn=2025, gdb=50))
    st, erg = _req(base, "GET", "/fall/r5/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 56800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_folgejahr_fixierung_offen(base):
    """K2-KERN (S3): aa-Folgejahr (Rentenbeginn 2015 < VZ) OHNE fixierten Rentenfreibetrag → fail-closed
    rentenfreibetrag_fixierung_offen (kein %×aktuelle-Rente, das würde Rentenerhöhungen unterbesteuern)."""
    _rentner_anlegen(base, "r3", _rentner_kegel(jahresrente=2100000, beginn=2015))   # kein rentenfreibetrag
    st, erg = _req(base, "GET", "/fall/r3/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "rentenfreibetrag_fixierung_offen"


def test_rentner_folgejahr_mit_rentenfreibetrag(base):
    """aa-Folgejahr MIT fixiertem Rentenfreibetrag 6000: (21000 − 6000) − 102 → es 14898 → 458 = 45800 Cent (S2)."""
    catala = _catala_da()
    _rentner_anlegen(base, "r2", _rentner_kegel(jahresrente=2100000, beginn=2015, rentenfreibetrag=600000))
    st, erg = _req(base, "GET", "/fall/r2/ergebnis")
    if catala:
        assert erg["zahl_cent"] == 45800 and erg["grund"] == "bestaetigt"
    else:
        assert erg["zahl_cent"] is None


def test_rentner_partner_behinderung_ohne_zusammen_gesperrt(base):
    """partner_check LIVE (K2): Partner-Behinderung (GdB 60) gesetzt, aber veranlagung=einzel → Widerspruch
    partner_konsistenz_offen (ein Einzelveranlagter hat keinen mitzuveranlagenden Ehe-/Lebenspartner)."""
    _rentner_anlegen(base, "rp", _rentner_kegel(jahresrente=2000000, beginn=2025,
                                                veranlagung="einzel", gdb_partner=60))
    st, erg = _req(base, "GET", "/fall/rp/ergebnis")
    assert erg["zahl_cent"] is None and erg["grund"] == "partner_konsistenz_offen"


def test_graph_uebersicht(base):
    """Read-only Desktop-Graph: Knoten = Regeln der Scheibe mit Status, Kanten = Feld→Regel mit
    Zustand. Ein Traverser-Aufruf, kein Bescheid, kein Schreibpfad."""
    fid = "g1"
    _req(base, "POST", "/fall", {"scheibe": "n_vor_gwg", "veranlagungszeitraum": 2025, "fall_id": fid})
    st, g = _req(base, "GET", f"/fall/{fid}/graph")
    assert st == 200
    _val("graph", g)
    rids = {k["regel_id"] for k in g["knoten"]}
    assert "p09_entfernungspauschale" in rids
    assert len(g["knoten"]) == 6          # EP + dHf + Verpflegung + Arbeitsmittel + VOR + GWG
    # frischer Fall: alle Kanten offen; beide Rollen vertreten
    assert all(k["zustand"] == "offen" for k in g["kanten"])
    assert any(k["rolle"] == "slot" for k in g["kanten"])
    assert any(k["rolle"] == "gate" for k in g["kanten"])
    # nach Bestätigung eines Felds → dessen Kante bestätigt (Store spiegelt sich im Graph)
    _req(base, "POST", f"/fall/{fid}/event", _laie("ep_arbeitstage", 220))
    st, g2 = _req(base, "GET", f"/fall/{fid}/graph")
    kante = next(k for k in g2["kanten"] if k["feld_id"] == "ep_arbeitstage")
    assert kante["zustand"] == "bestaetigt"
