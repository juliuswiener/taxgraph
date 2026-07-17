"""Gate für das Unsicherheits-Derivat (produkt/unsicherheit/, Task #11, Julius #6).

Deterministisch, NULL LLM. Kern-Tests mit einer reinen Modell-bescheid_fn (kein Toolchain nötig);
ein Real-Engine-Test über golden/runner.catala_* mit sauberem Toolchain-Skip (gettsim-Muster,
nie stilles Grün). Plus Negativtests: gedeckelt-Flag, offene Achse, nicht-fixierbar.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "unsicherheit"))
import intervall as IV  # noqa: E402


# -- kleine Helfer zum Bauen von Bindung/Snapshot ------------------------------

def _b(typ, **kw):
    e = {"typ": typ, "askable": True, "quelle": {"regel_id": "r", "signatur_slot": kw.get("slot", "s")}}
    if "bereich" in kw:
        e["bereich"] = kw["bereich"]
    if "enum_werte" in kw:
        e["enum_werte"] = kw["enum_werte"]
    if kw.get("summand"):
        e["slot_beitrag"] = "summand"
    return e


def _snap(**felder):
    """felder: feld_id -> (wert, zustand). Fehlt ein feld_id -> offen (kein Event)."""
    return {fid: {"wert": w, "zustand": z, "herkunft": {}} for fid, (w, z) in felder.items()}


# Modell-Steuer: linear + monoton, rein deterministisch (kein Toolchain).
def _modell(werte):
    return (werte.get("f_tage", 0) * 100
            + werte.get("f_km", 0) * 50
            + (200 if werte.get("f_flag") else 0))


BINDUNG = {
    "f_tage": _b("int", slot="tage", bereich={"min": 0, "max": 100}),
    "f_km": _b("int", slot="km", bereich={"min": 0, "max": 10}),
    "f_flag": _b("bool", slot="flag"),
}


# -- Kern -----------------------------------------------------------------------

def test_alle_bestaetigt_punkt_intervall():
    snap = _snap(f_tage=(20, "bestaetigt"), f_km=(5, "bestaetigt"), f_flag=(True, "bestaetigt"))
    r = IV.intervall(snap, BINDUNG, _modell)
    iv = r["intervall"]
    assert iv["min_cent"] == iv["max_cent"] == 20 * 100 + 5 * 50 + 200
    assert not iv["gedeckelt"] and not iv["min_offen"] and not iv["max_offen"]
    assert r["beitraege"] == []


def test_eine_unsichere_achse_spannt():
    # f_tage vorlaeufig -> variiert 0..100; andere bestätigt
    snap = _snap(f_tage=(20, "vorlaeufig"), f_km=(5, "bestaetigt"), f_flag=(False, "bestaetigt"))
    r = IV.intervall(snap, BINDUNG, _modell)
    iv = r["intervall"]
    assert iv["min_cent"] == 0 * 100 + 5 * 50
    assert iv["max_cent"] == 100 * 100 + 5 * 50
    assert not iv["gedeckelt"] and not iv["min_offen"]
    assert r["beitraege"][0]["feld_id"] == "f_tage"
    assert r["beitraege"][0]["spanne_cent"] == 100 * 100


def test_bestaetigen_verengt_spanne():
    offen = _snap(f_km=(5, "bestaetigt"), f_flag=(False, "bestaetigt"))  # f_tage offen
    r1 = IV.intervall(offen, BINDUNG, _modell)
    fixiert = _snap(f_tage=(20, "bestaetigt"), f_km=(5, "bestaetigt"), f_flag=(False, "bestaetigt"))
    r2 = IV.intervall(fixiert, BINDUNG, _modell)
    spanne1 = r1["intervall"]["max_cent"] - r1["intervall"]["min_cent"]
    spanne2 = r2["intervall"]["max_cent"] - r2["intervall"]["min_cent"]
    assert spanne2 < spanne1, "Bestätigen verengt die Spanne nicht"


def test_gedeckelt_flag_bei_kleinem_cap():
    # zwei bool-Achsen + cap=2 -> nur eine Achse passt exakt, andere = Rest
    b = {"a": _b("bool", slot="a"), "bb": _b("bool", slot="b")}
    fn = lambda w: (10 if w.get("a") else 0) + (3 if w.get("bb") else 0)
    snap = _snap(a=(False, "vorlaeufig"), bb=(False, "vorlaeufig"))
    r = IV.intervall(snap, b, fn, cap=2)
    iv = r["intervall"]
    assert iv["gedeckelt"] is True
    assert iv["exakt_bzgl_top_k"] == 1
    assert len(iv["rest_felder"]) == 1


def test_offene_achse_unbounded_mit_vorschlag():
    # cent ohne bereich, vorlaeufig-Vorschlag vorhanden -> fixierbar, aber Intervall offen
    b = {"geld": _b("cent", slot="g")}
    snap = _snap(geld=(50000, "vorlaeufig"))
    r = IV.intervall(snap, b, lambda w: w.get("geld", 0))
    iv = r["intervall"]
    assert "geld" in iv["offene_achsen"]
    assert iv["min_offen"] and iv["max_offen"]


def test_nicht_fixierbar_kein_ersatzwert():
    # cent ohne bereich, offen (kein Event) -> nicht fixierbar -> kein numerischer Bescheid
    b = {"geld": _b("cent", slot="g")}
    r = IV.intervall({}, b, lambda w: 999)
    iv = r["intervall"]
    assert "geld" in iv["nicht_fixierbar"]
    assert iv["min_cent"] is None and iv["max_cent"] is None
    assert iv["min_offen"] and iv["max_offen"]
    assert r["beitraege"] == []


def test_enum_alle_werte():
    b = {"e": _b("enum", slot="e", enum_werte=["a", "b", "c"])}
    fn = lambda w: {"a": 1, "b": 5, "c": 3}.get(w.get("e"), 0)
    snap = _snap(e=("a", "vorlaeufig"))
    r = IV.intervall(snap, b, fn)
    assert r["intervall"]["min_cent"] == 1 and r["intervall"]["max_cent"] == 5


def test_deterministisch():
    snap = _snap(f_tage=(20, "vorlaeufig"), f_km=(5, "vorlaeufig"), f_flag=(False, "bestaetigt"))
    assert IV.intervall(snap, BINDUNG, _modell) == IV.intervall(snap, BINDUNG, _modell)


# -- Summen-Konvention-Adapter --------------------------------------------------

def test_bescheid_via_slots_summanden():
    b = {"an": _b("cent", slot="gesamt", summand=True),
         "ag": _b("cent", slot="gesamt", summand=True)}
    # cent-Quantität (nenner_b_cent) -> nach_cent = identity: hier unit-neutral, testet nur die
    # Summanden-Aggregation (100+40).
    fn = IV.bescheid_via_slots(b, lambda slots: slots.get("gesamt", 0), quantitaet="nenner_b_cent")
    assert fn({"an": 100, "ag": 40}) == 140      # Summanden addiert


# -- Real-Engine-Integration (golden/runner.catala_*), Toolchain-Skip -----------

def test_real_engine_entfernungspauschale():
    try:
        sys.path.insert(0, os.path.join(ROOT, "golden"))
        import runner  # noqa: F401  (Import triggert catala_runtime; fehlt Toolchain -> Skip)
    except Exception as e:
        pytest.skip(f"Catala-Toolchain nicht verfügbar (opam-Env/_catala): {type(e).__name__}: {e}")

    def bescheid(feld_werte):
        s = {"veranlagungszeitraum": 2025,
             "arbeitstage": feld_werte.get("ep_arbeitstage", 0),
             "entfernung_km_roh": feld_werte.get("ep_entfernung_km", 0),
             "eigenes_oder_ueberlassenes_kfz": True,
             "oepnv_kosten_jahr": 0}
        return runner.catala_entfernungspauschale(s)

    bindung = {"ep_arbeitstage": _b("int", slot="arbeitstage", bereich={"min": 0, "max": 366}),
               "ep_entfernung_km": _b("int", slot="entfernung_km_roh", bereich={"min": 0, "max": 100})}
    snap = _snap(ep_arbeitstage=(220, "vorlaeufig"), ep_entfernung_km=(30, "bestaetigt"))
    r = IV.intervall(snap, bindung, bescheid, cap=8)
    iv = r["intervall"]
    # 0 Arbeitstage -> 0 Pauschale (min); mehr Tage -> mehr (monoton in Tagen)
    assert iv["min_cent"] == 0
    assert iv["max_cent"] > iv["min_cent"]
    assert not iv["min_offen"]
