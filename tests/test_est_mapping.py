"""Gate für die est_mapping-Schicht (produkt/mapping/, Task #11). Deterministisch, NULL LLM.

Prüft die 5 Fall-Klassen + Round-Trip (1:1 exakt, Aggregation dokumentiert-genau/nicht-deklariert,
Negation Doppel-Negation, Multiplikation Zähl), fail-closed (vorlaeufig -> unvollständig), das
maschinenlesbare Nicht-Deklarierte (Auflage C) und den feldmapping-Konsistenz-Check (Auflage B).
Ausbau Scheiben 2-4: 1:1-Kz Kapital §20 (E0121709 + Aktien-Subset E1900901/E1901301/E1901201), V+V §21
Mieteinnahmen (E0700201), §35a/agB (E0161404/E0161504/E0161804/E0104109); fam null-Kz = GAP,
Rentner §33b 1:1 (Person-A-Kz, Freigabe msg 2719).
Plus Negativtests.
"""
from __future__ import annotations

import copy
import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import est_mapping as EM   # noqa: E402
import store as ST         # noqa: E402
import traverser as TR     # noqa: E402

TS = "2026-07-17T14:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def _b(s, feld_id, wert, zustand="bestaetigt"):
    sig = {"signal_1": None, "signal_2": f"ok@{feld_id}"} if zustand == "bestaetigt" else {"signal_1": None, "signal_2": None}
    herk = H if zustand == "bestaetigt" else {"herkunft": "llm_vorschlag", **{k: v for k, v in H.items() if k != "herkunft"}}
    schr = "ui:laie" if zustand == "bestaetigt" else "llm:chat"
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand=zustand, herkunft=herk, schreiber=schr, signal=sig, ts=TS)


def _voller_store():
    s = ST.leerer_store(2025, fall_id="e2e-map")
    _b(s, "kap_kapitalertraege", 300000)                    # Klasse 1 -> E0121709
    _b(s, "vor_an_anteil_rv", 3500000)                      # Klasse b (1:1) -> E2000401
    _b(s, "vor_ag_anteil_rv", 1000000)                      # -> E2000801
    _b(s, "vor_rv_ausserhalb_lstb", 0)                      # -> E2000601
    _b(s, "vv_gebaeude_afa", 300000)                        # Klasse a -> E0703838 (Summe)
    _b(s, "vv_schuldzinsen", 200000)
    _b(s, "vv_erhaltungsaufwand", 100000)
    _b(s, "vv_sonstige_wk", 50000)
    _b(s, "fam_alleinstehend", True)                        # Klasse d -> E0503701 invertiert
    _b(s, "fam_anzahl_kinder", 2)                           # Klasse e -> 2 Kind-Anlagen
    _b(s, "vv_entgelt_quote_prozent", 100)                  # Klasse c -> nicht deklariert
    return s


# ---- Fall-Klassen ------------------------------------------------------------

def test_klasse_1_und_split_1zu1(bindung):
    snap, sid = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung, snapshot_id=sid)
    assert r["deklaration"]["E0121709"] == 300000                 # 1:1
    assert r["deklaration"]["E2000401"] == 3500000                # VOR-Summand einzeln
    assert r["deklaration"]["E2000801"] == 1000000
    assert r["deklaration"]["E2000601"] == 0
    assert r["basis_snapshot"] == sid


def test_klasse_a_dokumentiert_nicht_deklariert(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung)
    assert "E0703838" not in r["deklaration"]                      # Anlage-V-Ruling: NICHT deklariert
    assert r["dokumentiert"]["E0703838"]["summe"] == 300000 + 200000 + 100000 + 50000  # Summe dokumentiert
    assert set(r["dokumentiert"]["E0703838"]["quell_felder"]) == {"vv_gebaeude_afa", "vv_schuldzinsen",
                                           "vv_erhaltungsaufwand", "vv_sonstige_wk"}  # Auflage A explizit


def test_klasse_d_negation(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E0503701"] is False   # alleinstehend=True -> keine schädliche Haushaltsgem.


def test_klasse_e_multiplikation(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung)
    assert len(r["kind_anlagen"]) == 2


def test_klasse_c_nicht_deklariert_maschinenlesbar(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung)
    ndf = {x["feld_id"] for x in r["nicht_deklariert"]}
    assert "vv_entgelt_quote_prozent" in ndf                       # Auflage C: fehlend ≠ leer
    assert all(x["grund"] for x in r["nicht_deklariert"])          # jeder mit Grund


# ---- fail-closed (K2-Invariante) ---------------------------------------------

def test_fail_closed_vorlaeufig_unvollstaendig(bindung):
    s = _voller_store()
    _b(s, "kap_gewinn_aktien", 99999, zustand="vorlaeufig")        # ein vorläufiges Pflicht-Feld
    snap, _ = ST.materialisiere(s)
    r = EM.deklariere(snap, bindung)
    assert r["vollstaendig"] is False
    uf = {x["feld_id"] for x in r["unvollstaendig"]}
    assert "kap_gewinn_aktien" in uf
    assert "E1900901" not in r["deklaration"]                      # vorläufiger Wert NICHT deklariert


# ---- Round-Trip (Lab N3) -----------------------------------------------------

def test_roundtrip_1zu1_und_negation_exakt(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung)
    rt = EM.zuruecklesen(r, bindung)
    assert rt["felder"]["kap_kapitalertraege"] == 300000          # 1:1 exakt
    assert rt["felder"]["vor_an_anteil_rv"] == 3500000
    assert rt["felder"]["fam_alleinstehend"] is True              # Doppel-Negation == Store


def test_roundtrip_aggregation_nur_summe(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung)
    rt = EM.zuruecklesen(r, bindung)
    # Auflage A: aggregat trägt die dokumentierte Summe (aus dem dokumentiert-Bucket, nicht deklariert);
    # die Details sind NICHT rekonstruierbar
    assert rt["aggregat"]["E0703838"] == 650000
    assert "vv_gebaeude_afa" not in rt["felder"]                  # kein stiller Detail-Verlust vorgetäuscht
    # aggregat-genau: Summe == Σ Store-Details
    assert rt["aggregat"]["E0703838"] == 300000 + 200000 + 100000 + 50000


# ---- Konsistenz (Auflage B) --------------------------------------------------

def test_b_konsistenz_feldmapping(bindung):
    konflikte = EM.konsistenz_feldmapping(bindung)
    assert konflikte == [], f"Kz-Konflikte Bindung ↔ feldmapping: {konflikte}"


# ---- Negativtests ------------------------------------------------------------

def test_neg_verfaelschte_summe_bricht_roundtrip(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung)
    r2 = copy.deepcopy(r)
    r2["dokumentiert"]["E0703838"]["summe"] += 1                  # dokumentierte Summe manipuliert
    rt = EM.zuruecklesen(r2, bindung)
    assert rt["aggregat"]["E0703838"] != 300000 + 200000 + 100000 + 50000


def test_neg_determinismus(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    assert EM.deklariere(snap, bindung) == EM.deklariere(snap, bindung)


# ---- Nachauflage D: Eingabe-Guard gegen Falsch-Grün --------------------------

def test_d_guard_snapshot_objekt_statt_felder(bindung):
    """Snapshot-OBJEKT (mit felder/snapshot_id) statt felder-Ebene -> ValueError, nicht stilles Leer-Grün."""
    s = _voller_store()
    felder, sid = ST.materialisiere(s)
    snapshot_objekt = {"snapshot_id": sid, "ts": TS, "bis_event": "x" * 64, "felder": felder}
    with pytest.raises(ValueError):
        EM.deklariere(snapshot_objekt, bindung)


def test_d_guard_kein_treffer(bindung):
    """Nicht-leere Eingabe, aber KEIN Feld in der Bindungstabelle -> ValueError (falsche Struktur)."""
    fremd = {"voellig_fremdes_feld": {"wert": 1, "zustand": "bestaetigt", "herkunft": {}}}
    with pytest.raises(ValueError):
        EM.deklariere(fremd, bindung)


# ---- Ausbau Scheiben 2-4: 1:1-Kz + GAP (Instructor-Order 2026-07-17) ----------

def _store_mit(felder: dict, vz=2025):
    """Fresh Store, alle Felder bestätigt (isoliert von _voller_store, wahrt One-Active-Event/Feld)."""
    s = ST.leerer_store(vz, fall_id="ausbau")
    for fid, wert in felder.items():
        _b(s, fid, wert)
    return s


def test_scheibe3_kapital_und_vv_1zu1_roundtrip(bindung):
    """Kapital §20 (4 Kz) + V+V §21 Mieteinnahmen: je 1:1 + exakter Round-Trip."""
    felder = {"kap_kapitalertraege": 1000000, "kap_gewinn_aktien": 300000,
              "kap_verlust_aktien": 20000, "kap_verlust_sonstige": 15000, "vv_einnahmen": 1200000}
    snap, sid = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung, snapshot_id=sid)
    assert r["deklaration"]["E0121709"] == 1000000
    assert r["deklaration"]["E1900901"] == 300000
    assert r["deklaration"]["E1901301"] == 20000
    assert r["deklaration"]["E1901201"] == 15000
    assert r["deklaration"]["E0700201"] == 1200000
    rt = EM.zuruecklesen(r, bindung)
    for fid, wert in felder.items():
        assert rt["felder"][fid] == wert                       # 1:1 invertierbar (exakt)


def test_aktien_subset_semantik_beide_deklariert(bindung):
    """E1900901 (Aktiengewinn) ist Teilmenge von E0121709 (Kapitalerträge) — beide werden EINZELN
    deklariert (Vordruck-Memo für die Verlustverrechnung); est_mapping mappt jedes 1:1, die
    Subset-Beziehung ist Validierungs- (nicht Transform-)Sache."""
    snap, _ = ST.materialisiere(_store_mit({"kap_kapitalertraege": 1000000, "kap_gewinn_aktien": 300000}))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E0121709"] == 1000000 and r["deklaration"]["E1900901"] == 300000
    assert r["deklaration"]["E1900901"] <= r["deklaration"]["E0121709"]   # Subset (Testdaten-konsistent)


def test_scheibe2_sonder_35a_agb_1zu1_roundtrip(bindung):
    """§35a-Ermäßigung (Minijob/Dienstleistung/Handwerker) + agB: STRONG-Kz je 1:1 + Round-Trip."""
    felder = {"agb_aufwendungen": 500000, "hh_minijob_aufwendungen": 250000,
              "hh_dienstleistungen": 400000, "hh_handwerker_arbeitskosten": 120000}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E0104109"] == 500000
    assert r["deklaration"]["E0161404"] == 250000
    assert r["deklaration"]["E0161504"] == 400000
    assert r["deklaration"]["E0161804"] == 120000
    rt = EM.zuruecklesen(r, bindung)
    for fid, wert in felder.items():
        assert rt["felder"][fid] == wert


def test_scheibe4_rentner_p33b_1zu1_und_klasse_f(bindung):
    """Rentner-Scheibe4: die 5 §33b-Behinderungs-/Pflege-Felder sind nach Instructor-Freigabe (msg 2719,
    Sektions-Pfad-Beleg Person-A/E01097-Block) 1:1 auf ihre Kz gebunden; der p33b-Pauschbetrag selbst
    (Regel-Output) hat kein Feld und erscheint nicht; die Renten-Verzweigung ohne bestätigte Art bleibt
    fail-closed unvollständig (Klasse f)."""
    snap, _ = ST.materialisiere(_store_mit({
        "rentner_grad_der_behinderung": 50, "rentner_hilflos_blind_taubblind": True,
        "rentner_hinterbliebenenbezuege": True, "rentner_pflegegrad": 3,
        "rentner_gepflegter_hilflos": True, "rentner_jahresrente": 1800000}))
    r = EM.deklariere(snap, bindung)
    d = r["deklaration"]
    assert d["E0109708"] == 50 and d["E0109706"] is True        # GdB + hilflos (Person A)
    assert d["E0109704"] is True                                # Hinterbliebenen (eigen, nicht Kind-Transfer)
    assert d["E0161606"] == 3 and d["E0161808"] is True         # Pflegegrad + Merkzeichen H
    uf = {x["feld_id"] for x in r["unvollstaendig"]}
    assert "rentner_jahresrente" in uf                          # Klasse f ohne Art -> unvollständig
    rt = EM.zuruecklesen(r, bindung)                            # 1:1 exakt invertierbar
    assert rt["felder"]["rentner_grad_der_behinderung"] == 50
    assert rt["felder"]["rentner_gepflegter_hilflos"] is True


def test_ehegatte_behinderung_partner_1zu1(bindung):
    """Ehegatte-Behinderung §33b Person B (Freigabe msg 2725): die _partner-Felder mappen 1:1 auf EIGENE
    Person-B-Kz (E0505809/E0505807, E05058-Block) — Klasse 1, NICHT g (Person B hat eigene Kz, kein
    Person-A-Reuse). Exakt invertierbar."""
    snap, _ = ST.materialisiere(_store_mit({
        "rentner_grad_der_behinderung_partner": 60,
        "rentner_hilflos_blind_taubblind_partner": True}))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E0505809"] == 60                   # GdB Partner (Person-B-eigener Kz)
    assert r["deklaration"]["E0505807"] is True                 # hilflos/blind Partner
    rt = EM.zuruecklesen(r, bindung)
    assert rt["felder"]["rentner_grad_der_behinderung_partner"] == 60
    assert rt["felder"]["rentner_hilflos_blind_taubblind_partner"] is True


def test_neg_scheibe3_verfaelschtes_1zu1_bricht_roundtrip(bindung):
    """Manipuliertes 1:1-Kapital-Kz -> Round-Trip weicht ab (kein stiller Durchlauf)."""
    snap, _ = ST.materialisiere(_store_mit({"kap_kapitalertraege": 1000000}))
    r = EM.deklariere(snap, bindung)
    r2 = copy.deepcopy(r)
    r2["deklaration"]["E0121709"] += 1
    rt = EM.zuruecklesen(r2, bindung)
    assert rt["felder"]["kap_kapitalertraege"] != 1000000


# ---- Klasse f: Renten-Art-Verzweigung (Nachtrag A, 1 Wert-Slot -> N-Kz) -------

def test_klasse_f_verzweigung_aa_basisversorgung(bindung):
    """gesetzliche Rente (aa) -> Leibr_gesetzl-Kz E1800301 (Betrag) + E1800501 (Beginn)."""
    snap, _ = ST.materialisiere(_store_mit({"rentner_renten_art": "gesetzliche_rente",
                                            "rentner_jahresrente": 1800000, "rentner_renten_beginn_jahr": 2015}))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E1800301"] == 1800000
    assert r["deklaration"]["E1800501"] == 2015                # Jahr-Granularität (Datum = Submission-Layer)


def test_klasse_f_verzweigung_private_und_sonstige(bindung):
    """private Leibrente (bb) -> Leibr_priv E1801601/E1801701; sonstige -> Leibr_sonst E1803102/E1803202."""
    r_priv = EM.deklariere(ST.materialisiere(_store_mit({"rentner_renten_art": "private_leibrente",
        "rentner_jahresrente": 900000, "rentner_renten_beginn_jahr": 2018}))[0], bindung)
    assert r_priv["deklaration"]["E1801601"] == 900000 and r_priv["deklaration"]["E1801701"] == 2018
    r_sonst = EM.deklariere(ST.materialisiere(_store_mit({"rentner_renten_art": "sonstige_leibrente",
        "rentner_jahresrente": 120000, "rentner_renten_beginn_jahr": 2020}))[0], bindung)
    assert r_sonst["deklaration"]["E1803102"] == 120000 and r_sonst["deklaration"]["E1803202"] == 2020


def test_klasse_f_fail_closed_ohne_bestaetigte_art(bindung):
    """Wert bestätigt, Art VORLÄUFIG -> Kz-Zweig offen -> unvollständig, NICHT deklariert (fail-closed)."""
    s = ST.leerer_store(2025, fall_id="verzw-failclosed")
    _b(s, "rentner_jahresrente", 1800000)                       # bestätigt
    _b(s, "rentner_renten_art", "gesetzliche_rente", zustand="vorlaeufig")   # Art nur vorläufig
    snap, _ = ST.materialisiere(s)
    r = EM.deklariere(snap, bindung)
    assert not any(k.startswith("E1800") for k in r["deklaration"])   # kein Renten-Kz gesetzt
    assert "rentner_jahresrente" in {x["feld_id"] for x in r["unvollstaendig"]}


def test_klasse_f_roundtrip_value(bindung):
    """Round-Trip: der Betrag ist über das Art-Zweig-Kz invertierbar (die exakte Art ist gruppen-genau)."""
    snap, _ = ST.materialisiere(_store_mit({"rentner_renten_art": "private_leibrente",
                                            "rentner_jahresrente": 900000, "rentner_renten_beginn_jahr": 2018}))
    r = EM.deklariere(snap, bindung)
    rt = EM.zuruecklesen(r, bindung)
    assert rt["felder"]["rentner_jahresrente"] == 900000
    assert rt["felder"]["rentner_renten_beginn_jahr"] == 2018


def test_neg_klasse_f_unbekannte_art_kein_kz(bindung):
    """Eine nicht gemappte Art -> KEIN Kz (nicht_deklariert), kein Default-Zweig."""
    s = ST.leerer_store(2025, fall_id="verzw-unbekannt")
    _b(s, "rentner_jahresrente", 1800000)
    _b(s, "rentner_renten_art", "voellig_unbekannte_art")       # nicht in enum_werte/kz-map
    snap, _ = ST.materialisiere(s)
    r = EM.deklariere(snap, bindung)
    assert not any(k.startswith("E1800") or k.startswith("E1801") for k in r["deklaration"])
    assert "rentner_jahresrente" in {x["feld_id"] for x in r["nicht_deklariert"]}


def test_klasse_f_veraeusserung_betriebsart(bindung):
    """§ 16 Abs. 4 Betriebsveräußerungsgewinn: Klasse-f-Verzweigung je Betriebsart — gewerbe→E0801301
    (Anlage G), selbstaendig→E0901201 (Anlage S); land_forst = benannte GAP (kein § 16-FB-Kz im Schema)
    → fail-closed nicht_deklariert, kein Default-Zweig."""
    def dekl(art):
        snap, _ = ST.materialisiere(_store_mit({"rentner_veraeusserungsgewinn": 15000000,
                                                "rentner_veraeusserungs_betriebsart": art}))
        return EM.deklariere(snap, bindung)
    assert dekl("gewerbe")["deklaration"]["E0801301"] == 15000000
    assert dekl("selbstaendig")["deklaration"]["E0901201"] == 15000000
    r_lf = dekl("land_forst")
    assert "E0801301" not in r_lf["deklaration"] and "E0901201" not in r_lf["deklaration"]  # GAP-Zweig
    assert "rentner_veraeusserungsgewinn" in {x["feld_id"] for x in r_lf["nicht_deklariert"]}


# ---- Klasse g: Person-Multiplikation (Zusammenveranlagung, Front 2) -----------

def test_klasse_g_person_b_instanz(bindung):
    """_partner-Einkommensfelder -> person_b-Bucket (Kz wie Person A, Instanz B); IdNr B = distinktes
    Mantelbogen-Kz in der Haupt-Deklaration."""
    felder = {"bruttoarbeitslohn_partner": 3800000, "vor_an_anteil_rv_partner": 350000,
              "vor_ag_anteil_rv_partner": 350000, "vor_rv_ausserhalb_lstb_partner": 0,
              "person_b_idnr": "00000000000"}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["person_b"]["E0200201"] == 3800000                 # Bruttolohn Person B, Instanz B
    assert r["person_b"]["E2000401"] == 350000 and r["person_b"]["E2000801"] == 350000
    assert r["deklaration"]["E0100082"] == "00000000000"        # IdNr B = 1:1 in der Haupt-Deklaration
    assert "E0200201" not in r["deklaration"]                   # Person-B-Lohn NICHT in Person-A-Deklaration


def test_klasse_g_kapital_person_b(bindung):
    """§20 Kapital Person-B: kap_*_partner -> person_b-Bucket mit DENSELBEN Person-A-Kz (Anlage-KAP-
    Instanz B, kein distinktes Ehegatte-Kz — Schema-Recon 2026-07-18). Exakt invertierbar."""
    felder = {"kap_kapitalertraege_partner": 500000, "kap_gewinn_aktien_partner": 200000,
              "kap_verlust_aktien_partner": 50000, "kap_verlust_sonstige_partner": 30000}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["person_b"]["E0121709"] == 500000 and r["person_b"]["E1900901"] == 200000
    assert r["person_b"]["E1901301"] == 50000 and r["person_b"]["E1901201"] == 30000
    assert "E0121709" not in r["deklaration"]                   # Person-B-Kapital NICHT in Person-A-Deklaration
    rt = EM.zuruecklesen(r, bindung)
    assert rt["felder"]["kap_kapitalertraege_partner"] == 500000


def test_klasse_gf_renten_verzweigung_person_b(bindung):
    """§22 Rente Person-B: Klasse g×f — jahresrente_partner/beginn_partner verzweigen je
    rentner_renten_art_partner in den person_b-Bucket (aa gesetzl → E1800301/E1800501, bb privat →
    E1801601/E1801701), dieselben Person-A-Kz; ohne bestätigte Partner-Art fail-closed."""
    # aa gesetzliche Rente Person B
    snap, _ = ST.materialisiere(_store_mit({"rentner_renten_art_partner": "gesetzliche_rente",
        "rentner_jahresrente_partner": 1800000, "rentner_renten_beginn_jahr_partner": 2015}))
    r = EM.deklariere(snap, bindung)
    assert r["person_b"]["E1800301"] == 1800000 and r["person_b"]["E1800501"] == 2015
    assert "E1800301" not in r["deklaration"]                   # nicht in Person-A-Deklaration
    # bb private Leibrente Person B
    r2 = EM.deklariere(ST.materialisiere(_store_mit({"rentner_renten_art_partner": "private_leibrente",
        "rentner_jahresrente_partner": 900000, "rentner_renten_beginn_jahr_partner": 2018}))[0], bindung)
    assert r2["person_b"]["E1801601"] == 900000 and r2["person_b"]["E1801701"] == 2018
    # ohne Partner-Art -> fail-closed unvollständig
    r3 = EM.deklariere(ST.materialisiere(_store_mit({"rentner_jahresrente_partner": 1800000}))[0], bindung)
    assert "rentner_jahresrente_partner" in {x["feld_id"] for x in r3["unvollstaendig"]}
    assert not r3["person_b"]


def test_klasse_g_roundtrip(bindung):
    felder = {"bruttoarbeitslohn_partner": 3800000, "vor_an_anteil_rv_partner": 350000}
    snap, _ = ST.materialisiere(_store_mit(felder))
    rt = EM.zuruecklesen(EM.deklariere(snap, bindung), bindung)
    assert rt["felder"]["bruttoarbeitslohn_partner"] == 3800000
    assert rt["felder"]["vor_an_anteil_rv_partner"] == 350000


def test_klasse_g_fail_closed_partner_vorlaeufig(bindung):
    """§26b-Splitting fail-closed: ein vorläufiges Person-B-Feld -> nicht in Instanz B, unvollständig."""
    s = ST.leerer_store(2025, fall_id="partner-fc")
    _b(s, "bruttoarbeitslohn_partner", 3800000, zustand="vorlaeufig")
    snap, _ = ST.materialisiere(s)
    r = EM.deklariere(snap, bindung)
    assert "E0200201" not in r["person_b"]                      # vorläufig -> nicht deklariert
    assert "bruttoarbeitslohn_partner" in {x["feld_id"] for x in r["unvollstaendig"]}
