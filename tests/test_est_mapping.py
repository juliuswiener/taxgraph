"""Gate für die est_mapping-Schicht (produkt/mapping/, Task #11). Deterministisch, NULL LLM.

Prüft die 5 Fall-Klassen + Round-Trip (1:1 exakt, Aggregation aggregat-genau/lossy explizit, Negation
Doppel-Negation, Multiplikation Zähl), fail-closed (vorlaeufig -> unvollständig), das maschinenlesbare
Nicht-Deklarierte (Auflage C) und den feldmapping-Konsistenz-Check (Auflage B). Plus Negativtests.
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


def test_klasse_a_aggregation_lossy_explizit(bindung):
    snap, _ = ST.materialisiere(_voller_store())
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"]["E0703838"] == 300000 + 200000 + 100000 + 50000   # Summe
    assert set(r["lossy"]["E0703838"]) == {"vv_gebaeude_afa", "vv_schuldzinsen",
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
    # Auflage A: aggregat trägt die Summe, die Details sind NICHT rekonstruierbar
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
    r2["deklaration"]["E0703838"] += 1                            # Summe manipuliert
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
