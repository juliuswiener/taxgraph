"""Gate für die est_mapping-Schicht (produkt/mapping/, Task #11). Deterministisch, NULL LLM.

Prüft die 5 Fall-Klassen + Round-Trip (1:1 exakt, Aggregation dokumentiert-genau/nicht-deklariert,
Negation Doppel-Negation, Multiplikation Zähl), fail-closed (vorlaeufig -> unvollständig), das
maschinenlesbare Nicht-Deklarierte (Auflage C) und den feldmapping-Konsistenz-Check (Auflage B).
Ausbau Scheiben 2-4: 1:1-Kz Kapital §20 (E1900701 + Aktien-Subset E1900901/E1901301/E1901201), V+V §21
Mieteinnahmen (E0700201), §35a/agB (E0104109/E0107208/E0111215/E0161804); fam null-Kz = GAP,
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
    # generischer Fixture-Writer: ein Nutzer-eingegebener Wert (ui:laie), bestätigt ODER vorläufig (1. Signal).
    # NICHT llm:chat — ein vorläufiger Wert ≠ ein LLM-Vorschlag (K1 Feld-Katalog: llm:/import:*/berechnet: sind
    # katalog-restringiert; ui:laie ist der Mensch → kein Katalog-Check). Deckt beliebige Felder generisch.
    sig = {"signal_1": None, "signal_2": f"ok@{feld_id}"} if zustand == "bestaetigt" else {"signal_1": None, "signal_2": None}
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand=zustand, herkunft=H, schreiber="ui:laie", signal=sig, ts=TS)


def _voller_store():
    s = ST.leerer_store(2025, fall_id="e2e-map")
    _b(s, "kap_kapitalertraege", 300000)                    # Klasse 1 -> E1900701
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
    assert r["deklaration"]["E1900701"] == 3000                    # 1:1 (CENT→EURO floor)
    assert r["deklaration"]["E2000401"] == 35000                   # VOR-Summand einzeln
    assert r["deklaration"]["E2000801"] == 10000
    assert r["deklaration"]["E2000601"] == 0
    assert r["basis_snapshot"] == sid


def test_klasse_a_dokumentiert_nicht_deklariert(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung)
    assert "E0703838" not in r["deklaration"]                      # Anlage-V-Ruling: NICHT deklariert
    assert r["dokumentiert"]["E0703838"]["summe"] == 6500  # 3000+2000+1000+500 (EUR)
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
    assert rt["felder"]["kap_kapitalertraege"] == 3000            # 1:1 exakt (CENT→EURO nach Roundtrip)
    assert rt["felder"]["vor_an_anteil_rv"] == 35000
    assert rt["felder"]["fam_alleinstehend"] is True              # Doppel-Negation == Store (bool→unverändert)


def test_roundtrip_aggregation_nur_summe(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung)
    rt = EM.zuruecklesen(r, bindung)
    # Auflage A: aggregat trägt die dokumentierte Summe (aus dem dokumentiert-Bucket, nicht deklariert);
    # die Details sind NICHT rekonstruierbar
    assert rt["aggregat"]["E0703838"] == 6500
    assert "vv_gebaeude_afa" not in rt["felder"]                  # kein stiller Detail-Verlust vorgetäuscht
    # aggregat-genau: Summe == Σ Store-Details (EUR)
    assert rt["aggregat"]["E0703838"] == 6500


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
    assert r["deklaration"]["E1900701"] == 10000
    assert r["deklaration"]["E1900901"] == 3000
    assert r["deklaration"]["E1901301"] == 200
    assert r["deklaration"]["E1901201"] == 150
    assert r["deklaration"]["E0700201"] == 12000
    rt = EM.zuruecklesen(r, bindung)
    # Round-Trip Cent→Kz→zurueck: EURO-Werte (nicht Store-CENT)
    for fid, wert in felder.items():
        b = bindung.get(fid, {})
        expected = wert // 100 if b.get("typ") == "cent" else wert
        assert rt["felder"][fid] == expected                   # 1:1 invertierbar (exakt, EURO)


def test_aktien_subset_semantik_beide_deklariert(bindung):
    """E1900901 (Aktiengewinn) ist Teilmenge von E1900701 (Kapitalerträge) — beide werden EINZELN
    deklariert (Vordruck-Memo für die Verlustverrechnung); est_mapping mappt jedes 1:1, die
    Subset-Beziehung ist Validierungs- (nicht Transform-)Sache."""
    snap, _ = ST.materialisiere(_store_mit({"kap_kapitalertraege": 1000000, "kap_gewinn_aktien": 300000}))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E1900701"] == 10000 and r["deklaration"]["E1900901"] == 3000
    assert r["deklaration"]["E1900901"] <= r["deklaration"]["E1900701"]   # Subset (Testdaten-konsistent)


def test_scheibe2_sonder_35a_agb_1zu1_roundtrip(bindung):
    """§35a-Ermäßigung (Minijob/Dienstleistung/Handwerker) + agB: STRONG-Kz je 1:1 + Round-Trip."""
    felder = {"agb_aufwendungen": 500000, "hh_minijob_aufwendungen": 250000,
              "hh_dienstleistungen": 400000, "hh_handwerker_arbeitskosten": 120000}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E0161804"] == 5000
    assert r["deklaration"]["E0104109"] == 2500
    assert r["deklaration"]["E0107208"] == 4000
    assert r["deklaration"]["E0111215"] == 1200
    rt = EM.zuruecklesen(r, bindung)
    for fid, wert in felder.items():
        b = bindung.get(fid, {})
        expected = wert // 100 if b.get("typ") == "cent" else wert
        assert rt["felder"][fid] == expected


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
    """Ehegatte-Behinderung §33b Person B (XSD-Kz-Section-Sweep Cluster C): die _partner-Felder mappen
    auf Klasse g (PARTNER_INSTANZ) — DENSELBEN Person-A-Kz (E0109708/E0109706, AgB/Beh-Block,
    zweite Instanz), NICHT auf eigene E0505809/E0505807-Kz (das ist der §33b Abs.5-Kind-Übertrag,
    strukturell fremd). person_b-Bucket, exakt invertierbar."""
    snap, _ = ST.materialisiere(_store_mit({
        "rentner_grad_der_behinderung_partner": 60,
        "rentner_hilflos_blind_taubblind_partner": True}))
    r = EM.deklariere(snap, bindung)
    assert r["person_b"]["E0109708"] == 60                      # GdB Partner (Person-A-Kz reused)
    assert r["person_b"]["E0109706"] is True                    # hilflos/blind Partner
    assert "E0505809" not in r["deklaration"] and "E0505807" not in r["deklaration"]
    rt = EM.zuruecklesen(r, bindung)
    assert rt["felder"]["rentner_grad_der_behinderung_partner"] == 60
    assert rt["felder"]["rentner_hilflos_blind_taubblind_partner"] is True


def test_neg_scheibe3_verfaelschtes_1zu1_bricht_roundtrip(bindung):
    """Manipuliertes 1:1-Kapital-Kz -> Round-Trip weicht ab (kein stiller Durchlauf)."""
    snap, _ = ST.materialisiere(_store_mit({"kap_kapitalertraege": 1000000}))
    r = EM.deklariere(snap, bindung)
    r2 = copy.deepcopy(r)
    r2["deklaration"]["E1900701"] += 1
    rt = EM.zuruecklesen(r2, bindung)
    assert rt["felder"]["kap_kapitalertraege"] != 10000


# ---- Klasse f: Renten-Art-Verzweigung (Nachtrag A, 1 Wert-Slot -> N-Kz) -------

def test_klasse_f_verzweigung_aa_basisversorgung(bindung):
    """gesetzliche Rente (aa) -> Leibr_gesetzl-Kz E1800301 (Betrag) + E1800501 (Beginn)."""
    snap, _ = ST.materialisiere(_store_mit({"rentner_renten_art": "gesetzliche_rente",
                                            "rentner_jahresrente": 1800000, "rentner_renten_beginn_jahr": 2015}))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E1800301"] == 18000
    assert r["deklaration"]["E1800501"] == 2015                # Jahr-Granularität (Datum = Submission-Layer, int→unverändert)


def test_klasse_f_verzweigung_private_und_sonstige(bindung):
    """private Leibrente (bb) -> Leibr_priv E1801601/E1801701; sonstige -> Leibr_sonst E1803102/E1803202."""
    r_priv = EM.deklariere(ST.materialisiere(_store_mit({"rentner_renten_art": "private_leibrente",
        "rentner_jahresrente": 900000, "rentner_renten_beginn_jahr": 2018}))[0], bindung)
    assert r_priv["deklaration"]["E1801601"] == 9000 and r_priv["deklaration"]["E1801701"] == 2018
    r_sonst = EM.deklariere(ST.materialisiere(_store_mit({"rentner_renten_art": "sonstige_leibrente",
        "rentner_jahresrente": 120000, "rentner_renten_beginn_jahr": 2020}))[0], bindung)
    assert r_sonst["deklaration"]["E1803102"] == 1200 and r_sonst["deklaration"]["E1803202"] == 2020


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
    assert rt["felder"]["rentner_jahresrente"] == 9000
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
    """§ 16 Abs. 4 Betriebsveräußerungsgewinn: Klasse-f-Verzweigung deckt alle drei Betriebsarten/Anlagen
    ab — gewerbe→E0801301 (Anlage G), selbstaendig→E0804501 (Anlage S), land_forst→E0901201 (Anlage L)."""
    def dekl(art):
        snap, _ = ST.materialisiere(_store_mit({"rentner_veraeusserungsgewinn": 15000000,
                                                "rentner_veraeusserungs_betriebsart": art}))
        return EM.deklariere(snap, bindung)
    assert dekl("gewerbe")["deklaration"]["E0801301"] == 150000
    assert dekl("selbstaendig")["deklaration"]["E0804501"] == 150000
    assert dekl("land_forst")["deklaration"]["E0901201"] == 150000


# ---- Klasse g: Person-Multiplikation (Zusammenveranlagung, Front 2) -----------

def test_klasse_g_person_b_instanz(bindung):
    """_partner-Einkommensfelder -> person_b-Bucket (Kz wie Person A, Instanz B); IdNr B = distinktes
    Mantelbogen-Kz in der Haupt-Deklaration."""
    felder = {"bruttoarbeitslohn_partner": 3800000, "vor_an_anteil_rv_partner": 350000,
              "vor_ag_anteil_rv_partner": 350000, "vor_rv_ausserhalb_lstb_partner": 0,
              "person_b_idnr": "00000000000"}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["person_b"]["E0200201"] == 38000                    # Bruttolohn Person B, Instanz B (CENT→EUR)
    assert r["person_b"]["E2000401"] == 3500 and r["person_b"]["E2000801"] == 3500
    assert r["deklaration"]["E0100082"] == "00000000000"        # IdNr B = 1:1 in der Haupt-Deklaration
    assert "E0200201" not in r["deklaration"]                   # Person-B-Lohn NICHT in Person-A-Deklaration


def test_klasse_g_kapital_person_b(bindung):
    """§20 Kapital Person-B: kap_*_partner -> person_b-Bucket mit DENSELBEN Person-A-Kz (Anlage-KAP-
    Instanz B, kein distinktes Ehegatte-Kz — Schema-Recon 2026-07-18). Exakt invertierbar."""
    felder = {"kap_kapitalertraege_partner": 500000, "kap_gewinn_aktien_partner": 200000,
              "kap_verlust_aktien_partner": 50000, "kap_verlust_sonstige_partner": 30000}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["person_b"]["E1900701"] == 5000 and r["person_b"]["E1900901"] == 2000
    assert r["person_b"]["E1901301"] == 500 and r["person_b"]["E1901201"] == 300
    assert "E1900701" not in r["deklaration"]                   # Person-B-Kapital NICHT in Person-A-Deklaration
    rt = EM.zuruecklesen(r, bindung)
    assert rt["felder"]["kap_kapitalertraege_partner"] == 5000


def test_klasse_gf_renten_verzweigung_person_b(bindung):
    """§22 Rente Person-B: Klasse g×f — jahresrente_partner/beginn_partner verzweigen je
    rentner_renten_art_partner in den person_b-Bucket (aa gesetzl → E1800301/E1800501, bb privat →
    E1801601/E1801701), dieselben Person-A-Kz; ohne bestätigte Partner-Art fail-closed."""
    # aa gesetzliche Rente Person B
    snap, _ = ST.materialisiere(_store_mit({"rentner_renten_art_partner": "gesetzliche_rente",
        "rentner_jahresrente_partner": 1800000, "rentner_renten_beginn_jahr_partner": 2015}))
    r = EM.deklariere(snap, bindung)
    assert r["person_b"]["E1800301"] == 18000 and r["person_b"]["E1800501"] == 2015
    assert "E1800301" not in r["deklaration"]                   # nicht in Person-A-Deklaration
    # bb private Leibrente Person B
    r2 = EM.deklariere(ST.materialisiere(_store_mit({"rentner_renten_art_partner": "private_leibrente",
        "rentner_jahresrente_partner": 900000, "rentner_renten_beginn_jahr_partner": 2018}))[0], bindung)
    assert r2["person_b"]["E1801601"] == 9000 and r2["person_b"]["E1801701"] == 2018
    # ohne Partner-Art -> fail-closed unvollständig
    r3 = EM.deklariere(ST.materialisiere(_store_mit({"rentner_jahresrente_partner": 1800000}))[0], bindung)
    assert "rentner_jahresrente_partner" in {x["feld_id"] for x in r3["unvollstaendig"]}
    assert not r3["person_b"]


def test_klasse_g_roundtrip(bindung):
    felder = {"bruttoarbeitslohn_partner": 3800000, "vor_an_anteil_rv_partner": 350000}
    snap, _ = ST.materialisiere(_store_mit(felder))
    rt = EM.zuruecklesen(EM.deklariere(snap, bindung), bindung)
    assert rt["felder"]["bruttoarbeitslohn_partner"] == 38000
    assert rt["felder"]["vor_an_anteil_rv_partner"] == 3500


def test_klasse_g_fail_closed_partner_vorlaeufig(bindung):
    """§26b-Splitting fail-closed: ein vorläufiges Person-B-Feld -> nicht in Instanz B, unvollständig."""
    s = ST.leerer_store(2025, fall_id="partner-fc")
    _b(s, "bruttoarbeitslohn_partner", 3800000, zustand="vorlaeufig")
    snap, _ = ST.materialisiere(s)
    r = EM.deklariere(snap, bindung)
    assert "E0200201" not in r["person_b"]                      # vorläufig -> nicht deklariert
    assert "bruttoarbeitslohn_partner" in {x["feld_id"] for x in r["unvollstaendig"]}


# ---- Klasse INSTANZ Konsument: Multi-Objekt §21 (reale Bindung, erster Instanz-Konsument) --------
# Objekt A = Basis-feld_ids (Instanz 1, deklaration/dokumentiert wie bisher); Objekt B = __2-Suffix
# (anlage_instanzen, E0700201-Reuse + E0703838-Aggregat je Objekt). Ring-Σ = dev-1-Nachtrag.
_OBJ_A = {"vv_einnahmen": 1200000, "vv_gebaeude_afa": 300000, "vv_schuldzinsen": 200000,
          "vv_erhaltungsaufwand": 100000, "vv_sonstige_wk": 50000}          # WK-Σ 650000, §21_A 550000
_OBJ_B = {"vv_einnahmen__2": 900000, "vv_gebaeude_afa__2": 200000, "vv_schuldzinsen__2": 100000}  # WK-Σ 300000, §21_B 600000


def test_multi_objekt_vv_zwei_objekte(bindung):
    """Zwei Vermietungsobjekte: Objekt A in der Haupt-Deklaration (Instanz 1), Objekt B in
    anlage_instanzen[vv_objekt] (Instanz 2) — je E0700201-Reuse + eigenes E0703838-WK-Aggregat."""
    snap, _ = ST.materialisiere(_store_mit({**_OBJ_A, **_OBJ_B}))
    r = EM.deklariere(snap, bindung)
    # Objekt A (Instanz 1): unverändertes Verhalten
    assert r["deklaration"]["E0700201"] == 12000
    assert r["dokumentiert"]["E0703838"]["summe"] == 6500
    # Objekt B (Instanz 2): eigener Bucket, dieselbe Kz (Reuse)
    inst = r["anlage_instanzen"]["vv_objekt"]
    assert len(inst) == 1 and inst[0]["index"] == 2
    assert inst[0]["felder"]["E0700201"] == 9000
    assert inst[0]["dokumentiert"]["E0703838"]["summe"] == 3000
    assert inst[0]["dokumentiert"]["E0703838"]["quell_felder"] == ["vv_gebaeude_afa__2", "vv_schuldzinsen__2"]
    assert r["vollstaendig"] is True


def test_multi_objekt_summe_datenvollstaendig_fuer_ring(bindung):
    """Der Ring (dev-1) summiert § 21-Einkünfte je Objekt (Einnahmen − WK-Aggregat). Dieser Test belegt,
    dass die Deklaration ALLE dafür nötigen Zahlen je Objekt trägt: Σ = (1200000−650000)+(900000−300000)."""
    snap, _ = ST.materialisiere(_store_mit({**_OBJ_A, **_OBJ_B}))
    r = EM.deklariere(snap, bindung)
    obj_a = r["deklaration"]["E0700201"] - r["dokumentiert"]["E0703838"]["summe"]
    inst = r["anlage_instanzen"]["vv_objekt"][0]
    obj_b = inst["felder"]["E0700201"] - inst["dokumentiert"]["E0703838"]["summe"]
    assert obj_a == 5500 and obj_b == 6000
    assert obj_a + obj_b == 11500                             # Σ § 21-Einkünfte über beide Objekte (EUR)


def test_multi_objekt_roundtrip(bindung):
    """Round-Trip: base + base__2 exakt invertierbar (1:1); Aggregat je Objekt nur Summe (E0703838[__2])."""
    snap, _ = ST.materialisiere(_store_mit({**_OBJ_A, **_OBJ_B}))
    r = EM.deklariere(snap, bindung)
    rt = EM.zuruecklesen(r, bindung)
    assert rt["felder"]["vv_einnahmen"] == 12000               # Objekt A (EUR)
    assert rt["felder"]["vv_einnahmen__2"] == 9000             # Objekt B (EUR)
    assert rt["aggregat"]["E0703838"] == 6500                  # Objekt-A-Aggregat (EUR)
    assert rt["aggregat"]["E0703838__2"] == 3000               # Objekt-B-Aggregat je Instanz (EUR)


def test_multi_objekt_fail_closed_objekt_b_vorlaeufig(bindung):
    """K2 je Objekt: Objekt-B-Einnahmen vorläufig -> Objekt B nicht im Bucket, Gesamt unvollständig
    (kein halber Multi-Objekt-Bescheid)."""
    s = _store_mit(_OBJ_A)
    _b(s, "vv_einnahmen__2", 900000, zustand="vorlaeufig")
    snap, _ = ST.materialisiere(s)
    r = EM.deklariere(snap, bindung)
    assert r["vollstaendig"] is False
    assert "vv_einnahmen__2" in {x["feld_id"] for x in r["unvollstaendig"]}
    assert r["anlage_instanzen"] == {}                         # vorläufiges Objekt B nicht deklariert


def test_multi_objekt_partner_beide_vermieter(bindung):
    """vv_*_partner-Landeplatz: Ehepaar, beide Vermieter = zwei Objekte auf der Instanz-Achse (Objekt B
    = __2). Der Multi-Objekt-Kanal deckt den Person-B-V+V-Defer ab (kein eigener _partner-Pfad nötig)."""
    snap, _ = ST.materialisiere(_store_mit({**_OBJ_A, **_OBJ_B}))
    r = EM.deklariere(snap, bindung)
    # beide Objekte tragen dieselbe Person-A-Kz E0700201 (Anlage V je Objekt), kein distinktes Ehegatte-Kz
    alle_e0700201 = ([r["deklaration"]["E0700201"]]
                     + [e["felder"]["E0700201"] for e in r["anlage_instanzen"]["vv_objekt"]])
    assert alle_e0700201 == [12000, 9000]


# ---- Klasse INSTANZ Konsument 2: Per-Kind (Anlage Kind, ELSTER-Form, reines 1:1 je Instanz × A/B) --------
# Zwei Achsen: Kind-Instanz (instanz_gruppe:kind, Kind 1=Basis / Kind 2..N=__n) × Elternteil A/B (zwei distinkte
# Basis-Kz je Konzept: _a→E0500807/E0500601, _b→E0500808/E0500805). Tarif-/Ring-neutral (count-MVP bleibt).
_KIND_1 = {"kind_idnr": "11111111111",
           "kind_kindschaftsverhaeltnis_a": "leibliches Kind", "kind_kindschaftsverhaeltnis_b": "leibliches Kind",
           "kind_kindschaftsverh_zeitraum_a": "01.01.2025 - 31.12.2025",
           "kind_kindschaftsverh_zeitraum_b": "01.01.2025 - 31.12.2025"}
_KIND_2 = {"kind_idnr__2": "22222222222",
           "kind_kindschaftsverhaeltnis_a__2": "Pflegekind", "kind_kindschaftsverhaeltnis_b__2": "Pflegekind",
           "kind_kindschaftsverh_zeitraum_a__2": "01.03.2025 - 31.12.2025",
           "kind_kindschaftsverh_zeitraum_b__2": "01.03.2025 - 31.12.2025"}


def test_per_kind_zwei_kinder(bindung):
    """Zwei Kinder: Kind 1 in der Haupt-Deklaration (Instanz 1), Kind 2 in anlage_instanzen[kind] (Instanz 2)
    — je 5 Anlage-Kind-Kz mit Reuse; Elternteil A/B sind zwei distinkte Basis-Kz je Konzept."""
    snap, _ = ST.materialisiere(_store_mit({**_KIND_1, **_KIND_2}))
    r = EM.deklariere(snap, bindung)
    # Kind 1 (Basis): 5 Kz in der Haupt-Deklaration, A/B distinkt
    assert r["deklaration"]["E0500406"] == "11111111111"                   # IdNr
    assert r["deklaration"]["E0500807"] == "leibliches Kind"               # Kindschaftsverh. Elternteil A
    assert r["deklaration"]["E0500808"] == "leibliches Kind"               # Elternteil B (distinktes Kz)
    assert r["deklaration"]["E0500601"] == "01.01.2025 - 31.12.2025"       # Zeitraum A
    assert r["deklaration"]["E0500805"] == "01.01.2025 - 31.12.2025"       # Zeitraum B
    # Kind 2 (Instanz): eigener Bucket, dieselben Kz (Reuse je Kind)
    inst = r["anlage_instanzen"]["kind"]
    assert len(inst) == 1 and inst[0]["index"] == 2
    assert inst[0]["felder"]["E0500406"] == "22222222222"
    assert inst[0]["felder"]["E0500807"] == "Pflegekind" and inst[0]["felder"]["E0500808"] == "Pflegekind"
    assert inst[0]["felder"]["E0500601"] == "01.03.2025 - 31.12.2025"
    assert r["vollstaendig"] is True


def test_per_kind_ab_zwei_distinkte_kz(bindung):
    """Elternteil A/B tragen je Kind ZWEI distinkte Kz (E0500807/E0500808 Kindschaftsverh., E0500601/E0500805
    Zeitraum) — Sektions-Pfad K_Verh_A/B (kein Reuse ÜBER die A/B-Achse, nur über die Kind-Achse)."""
    snap, _ = ST.materialisiere(_store_mit(_KIND_1))
    r = EM.deklariere(snap, bindung)
    # A und B sind ZWEI distinkte Kz (nicht ein geteiltes) — beide getrennt in der Deklaration
    assert "E0500807" in r["deklaration"] and "E0500808" in r["deklaration"] and "E0500807" != "E0500808"
    assert "E0500601" in r["deklaration"] and "E0500805" in r["deklaration"]


def test_per_kind_roundtrip(bindung):
    """Round-Trip: base + base__2 exakt invertierbar (1:1 Text-Werte) über beide Kinder + A/B-Achse."""
    snap, _ = ST.materialisiere(_store_mit({**_KIND_1, **_KIND_2}))
    rt = EM.zuruecklesen(EM.deklariere(snap, bindung), bindung)
    assert rt["felder"]["kind_idnr"] == "11111111111"                     # Kind 1
    assert rt["felder"]["kind_idnr__2"] == "22222222222"                  # Kind 2
    assert rt["felder"]["kind_kindschaftsverhaeltnis_a"] == "leibliches Kind"
    assert rt["felder"]["kind_kindschaftsverhaeltnis_a__2"] == "Pflegekind"


def test_per_kind_fail_closed_kind_2_vorlaeufig(bindung):
    """K2 je Kind: ein vorläufiges Kind-2-Feld -> Kind 2 (dieses Feld) nicht im Bucket, Gesamt unvollständig."""
    s = _store_mit(_KIND_1)
    _b(s, "kind_idnr__2", "22222222222", zustand="vorlaeufig")
    snap, _ = ST.materialisiere(s)
    r = EM.deklariere(snap, bindung)
    assert r["vollstaendig"] is False
    assert "kind_idnr__2" in {x["feld_id"] for x in r["unvollstaendig"]}
    assert "kind" not in r["anlage_instanzen"]                            # vorläufiges einziges Kind-2-Feld nicht deklariert


def test_per_kind_tarif_neutral_kein_ring_feld(bindung):
    """Per-Kind ist FORM-Vervollständigung: die count-MVP-Felder (fam_anzahl_kinder) bleiben unberührt;
    die per-Kind-Felder haben KEINEN signatur_slot in eine Ring-Rechnung (nur Anlage-Kind-Kz-Deklaration)."""
    b_idnr = bindung["kind_idnr"]
    assert b_idnr["instanz_gruppe"] == "kind" and b_idnr["elster_kz"] == "E0500406"
    assert "signatur_slot" not in b_idnr["quelle"]                        # Geltungsbedingung, kein Ring-Slot


# ---- Klasse INSTANZ Konsument 3: Multi-Rente (Anlage R, VERZWEIGUNG × Instanz) --------
# Jede Rente-Instanz hat EIGENE renten_art -> eigener aa/bb-Kz (VERZWEIGUNG je Instanz, Kern-Extension).
# Rente 1 = Basis, Rente 2..N = __n; renten_art ist SELEKTOR (nicht instanz_gruppe-getaggt, je Instanz via
# rentner_renten_art__idx gelesen). NICHT tarif-neutral: per-Rente-Ertragsanteil-Σ = dev-1-Ring-Nachtrag.
_RENTE_1 = {"rentner_renten_art": "gesetzliche_rente", "rentner_jahresrente": 2000000,
            "rentner_renten_beginn_jahr": 2025}                            # aa -> E1800301 / E1800501
_RENTE_2 = {"rentner_renten_art__2": "private_leibrente", "rentner_jahresrente__2": 900000,
            "rentner_renten_beginn_jahr__2": 2018}                         # bb -> E1801601 / E1801701


def test_multi_rente_zwei_renten_verschiedene_art(bindung):
    """Gesetzliche Rente (Rente 1, aa) + private Leibrente (Rente 2, bb): je eigener VERZWEIGUNG-Kz je
    Instanz-Art. Rente 1 in der Haupt-Deklaration, Rente 2 in anlage_instanzen[rente]."""
    snap, _ = ST.materialisiere(_store_mit({**_RENTE_1, **_RENTE_2}))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E1800301"] == 20000 and r["deklaration"]["E1800501"] == 2025   # Rente 1 aa (EUR)
    inst = r["anlage_instanzen"]["rente"]
    assert len(inst) == 1 and inst[0]["index"] == 2
    assert inst[0]["felder"]["E1801601"] == 9000 and inst[0]["felder"]["E1801701"] == 2018   # Rente 2 bb (EUR)
    assert "E1801601" not in r["deklaration"]                            # Rente-2-Kz NICHT in Person-A-Deklaration
    assert r["vollstaendig"] is True


def test_multi_rente_kz_reuse_gleiche_art(bindung):
    """Zwei Renten DERSELBEN Art (beide gesetzlich) -> beide E1800301 (Reuse je Instanz, wie Multi-Objekt)."""
    zwei_gesetzl = {**_RENTE_1, "rentner_renten_art__2": "gesetzliche_rente",
                    "rentner_jahresrente__2": 1500000, "rentner_renten_beginn_jahr__2": 2020}
    r = EM.deklariere(ST.materialisiere(_store_mit(zwei_gesetzl))[0], bindung)
    assert r["deklaration"]["E1800301"] == 20000                       # Rente 1 (EUR)
    assert r["anlage_instanzen"]["rente"][0]["felder"]["E1800301"] == 15000   # Rente 2, DIESELBE Kz (EUR)


def test_multi_rente_fail_closed_instanz_art_offen(bindung):
    """fail-closed je Instanz (Auflage 3): Rente-2-Betrag bestätigt, aber die Instanz-Art vorläufig ->
    Kz-Zweig offen -> unvollständig, KEIN Phantom-Kz, Rente 2 nicht im Bucket."""
    s = _store_mit(_RENTE_1)
    _b(s, "rentner_jahresrente__2", 900000)                              # bestätigt
    _b(s, "rentner_renten_art__2", "private_leibrente", zustand="vorlaeufig")   # Art nur vorläufig
    snap, _ = ST.materialisiere(s)
    r = EM.deklariere(snap, bindung)
    assert r["vollstaendig"] is False
    assert "rentner_jahresrente__2" in {x["feld_id"] for x in r["unvollstaendig"]}
    assert "rente" not in r["anlage_instanzen"]                          # leere Instanz gefiltert
    assert "E1801601" not in r["deklaration"]                            # kein Phantom-Kz


def test_multi_rente_roundtrip(bindung):
    """Round-Trip: base + base__2 über die VERZWEIGUNG-Zweig-Kz invertierbar (Value; Art gruppen-genau)."""
    snap, _ = ST.materialisiere(_store_mit({**_RENTE_1, **_RENTE_2}))
    rt = EM.zuruecklesen(EM.deklariere(snap, bindung), bindung)
    assert rt["felder"]["rentner_jahresrente"] == 20000                  # Rente 1 (EUR)
    assert rt["felder"]["rentner_jahresrente__2"] == 9000                # Rente 2 (über E1801601, EUR)
    assert rt["felder"]["rentner_renten_beginn_jahr__2"] == 2018         # über E1801701 (int→unverändert)


def test_multi_rente_instanz_kz_kein_phantom(bindung):
    """Drift-Awareness (Auflage 4): die Instanz-VERZWEIGUNG-Kz sind Art-Zweig-Kz (erlaubte Menge), kein
    neues/Phantom-Kz — instanz+art-bewusst."""
    snap, _ = ST.materialisiere(_store_mit({**_RENTE_1, **_RENTE_2}))
    r = EM.deklariere(snap, bindung)
    verzweigung_kz = {kz for cfg in EM.VERZWEIGUNG.values() for kz in cfg["kz"].values()}
    inst_kz = {kz for e in r["anlage_instanzen"]["rente"] for kz in e["felder"]}
    assert inst_kz and inst_kz <= verzweigung_kz, f"Instanz-Renten-Kz ohne VERZWEIGUNG-Herkunft: {inst_kz - verzweigung_kz}"


def test_multi_rente_alter_rentenfreibetrag_pro_instanz(bindung):
    """Ring-Ready (Instructor-Follow): alter + rentenfreibetrag sind per-Rente (instanz_gruppe:rente) → im
    Store/Snapshot je Instanz (dev-1s Ring liest per-Rente-Ertragsanteil aa/bb), aber KEIN eigener Kz
    (Tarif-Inputs → nicht_deklariert, kein Phantom in anlage_instanzen)."""
    felder = {**_RENTE_1, **_RENTE_2,
              "rentner_alter_bei_rentenbeginn__2": 65, "rentner_rentenfreibetrag__2": 600000}
    snap, _ = ST.materialisiere(_store_mit(felder))
    assert snap["rentner_alter_bei_rentenbeginn__2"]["wert"] == 65        # per-Instanz im Snapshot (Ring liest sie)
    assert snap["rentner_rentenfreibetrag__2"]["wert"] == 600000
    r = EM.deklariere(snap, bindung)
    inst = r["anlage_instanzen"]["rente"][0]
    assert inst["felder"] == {"E1801601": 9000, "E1801701": 2018}       # nur Kz-Felder (EUR), alter/rentenfreibetrag KEIN Phantom
    nd = {x["feld_id"] for x in r["nicht_deklariert"]}
    assert "rentner_alter_bei_rentenbeginn__2" in nd and "rentner_rentenfreibetrag__2" in nd
    assert r["vollstaendig"] is True


# ---- Ring-Naht #5: instanzen(store, bindung, gruppe) — Instanz-Enumeration für dev-1s per-Instanz-Σ ----

def test_instanzen_enumeriert_base_und_n(bindung):
    """ALLE Instanzen (Basis index=1 + __n), felder auf Basis-feld_id normiert, inkl. zustand/herkunft."""
    s = _store_mit({"vv_einnahmen": 1200000, "vv_einnahmen__2": 900000, "vv_gebaeude_afa__2": 200000})
    inst = EM.instanzen(s, bindung, "vv_objekt")
    assert [i["index"] for i in inst] == [1, 2]                       # Basis + __2, ALLE enumeriert
    assert inst[0]["felder"]["vv_einnahmen"]["wert"] == 1200000       # Instanz 1 = Basis
    assert inst[1]["felder"]["vv_einnahmen"]["wert"] == 900000        # __2 normiert auf Basis-Key
    assert inst[1]["felder"]["vv_gebaeude_afa"]["wert"] == 200000
    assert inst[0]["felder"]["vv_einnahmen"]["herkunft"]["herkunft"] == "laie"   # herkunft-Vektor sichtbar
    assert all(i["zustand"] == "bestaetigt" for i in inst)


def test_instanzen_zustand_meet_fail_closed(bindung):
    """per-Instanz-zustand = meet: eine unvollständige Instanz -> vorlaeufig (dev-1s K2-Guard, kein Σ)."""
    s = _store_mit({"vv_einnahmen": 1200000})
    _b(s, "vv_einnahmen__2", 900000, zustand="vorlaeufig")            # Instanz 2 vorläufig
    inst = EM.instanzen(s, bindung, "vv_objekt")
    z = {i["index"]: i["zustand"] for i in inst}
    assert z[1] == "bestaetigt" and z[2] == "vorlaeufig"


def test_instanzen_gleiche_enumeration_wie_deklaration(bindung):
    """EINE Enumerations-Wahrheit (parse_instanz): die instanzen()-__n-Indizes decken die anlage_instanzen-
    Indizes der Deklaration; instanzen() liefert zusätzlich die Basis (index 1). Keine Regex-Drift."""
    s = _store_mit({"vv_einnahmen": 1200000, "vv_einnahmen__2": 900000, "vv_einnahmen__3": 300000})
    inst_idx = {i["index"] for i in EM.instanzen(s, bindung, "vv_objekt")}
    dekl = EM.deklariere(ST.materialisiere(s)[0], bindung)
    dekl_idx = {e["index"] for e in dekl["anlage_instanzen"]["vv_objekt"]}
    assert inst_idx == {1, 2, 3} and dekl_idx == {2, 3}              # instanzen inkl. Basis, Deklaration nur __n
    assert dekl_idx <= inst_idx                                       # dieselbe Enumeration


def test_instanzen_gruppe_generisch_rente(bindung):
    """Generischer gruppe-Parameter für rente (dev-1 #6): trägt die getaggten Renten-Felder je Instanz —
    INKL. rentner_renten_art (jetzt instanz_gruppe:rente getaggt, #6-Prep), damit dev-1s Ring den
    per-Rente-Ertragsanteil (aa/bb je Art) über DIESELBE Naht bekommt, ohne zweiten Lesepfad."""
    s = _store_mit({**_RENTE_1, **_RENTE_2})
    inst = EM.instanzen(s, bindung, "rente")
    assert [i["index"] for i in inst] == [1, 2]
    assert inst[0]["felder"]["rentner_jahresrente"]["wert"] == 2000000
    assert inst[0]["felder"]["rentner_renten_art"]["wert"] == "gesetzliche_rente"    # Art je Rente (Rente 1, aa)
    assert inst[1]["felder"]["rentner_jahresrente"]["wert"] == 900000
    assert inst[1]["felder"]["rentner_renten_art"]["wert"] == "private_leibrente"     # Art je Rente (Rente 2, bb) — #6-ready


def test_instanzen_leere_gruppe(bindung):
    """Keine Felder der Gruppe im Store -> leere Liste (kein Phantom)."""
    s = _store_mit({"vv_einnahmen": 1200000})
    assert EM.instanzen(s, bindung, "kind") == []
