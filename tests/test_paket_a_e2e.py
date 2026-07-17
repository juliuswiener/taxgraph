"""Paket-A-Integrationsprobe — die 4 Säulen als SYSTEM (Task #11). Deterministisch, NULL LLM.

Ein durchgehendes EP-Szenario (Entfernungspauschale):
  1. leerer Store  -> Traverser fragt (naechste_fragen, Reihenfolge)
  2. Antworten via store.append_event: laie-bestätigt (entfernung/kfz/oepnv) + llm-VORLÄUFIG (arbeitstage)
  3. Unsicherheits-Intervall spannt auf der noch offenen bounded-Achse (arbeitstage 0..366)
  4. LLM-Wert via ZWEI-SIGNAL bestätigen (append_event ersetzt=llm-event, signal_2)
  5. Intervall SCHRUMPFT (Punkt) — monoton
  6. FAIL-CLOSED: eine festzusetzende Zahl gibt es NUR, wenn der Input-Kegel bestätigt ist
     (store.meet_zustand); vorher None, nachher die echte ESt (bescheid_via_slots -> Catala)
  7. Vorwärts-Trace: trace_ergebnis liefert Justification bis anker_ref

Real-Engine (golden/runner.catala_entfernungspauschale) wo Toolchain da, sonst sauberer Skip.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/store", "produkt/traverser", "produkt/unsicherheit", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import store as ST          # noqa: E402
import traverser as TR      # noqa: E402
import intervall as IV      # noqa: E402

TS = "2026-07-17T14:00:00+00:00"
EP_FELDER = ("ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz")


@pytest.fixture(scope="module")
def ep_bindung():
    b = TR.lade_bindung()
    return {fid: b[fid] for fid in EP_FELDER if fid in b}


def _laie_bestaetigt(s, feld_id, wert):
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand="bestaetigt",
                    herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"ok@{feld_id}"}, ts=TS)


def _llm_vorschlag(s, feld_id, wert):
    return ST.append_event(s, feld_id=feld_id, wert=wert, zustand="vorlaeufig",
                           herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                           schreiber="llm:chat", signal={"signal_1": None, "signal_2": None}, ts=TS)


def _catala_bescheid_fn(ep_bindung):
    """bescheid_via_slots über die EP-Familie -> golden/runner.catala_entfernungspauschale.
    Skip, wenn die Catala-Toolchain fehlt."""
    try:
        import runner  # noqa: F401  (Import triggert catala_runtime)
    except Exception as e:
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")

    def slot_fn(slots):
        s = {"veranlagungszeitraum": 2025,
             "arbeitstage": int(slots.get("arbeitstage", 0)),
             "entfernung_km_roh": int(slots.get("entfernung_km_roh", 0)),
             "oepnv_kosten_jahr": int(slots.get("oepnv_kosten_jahr", 0)),   # cent==euro bei 0
             "eigenes_oder_ueberlassenes_kfz": bool(slots.get("eigenes_oder_ueberlassenes_kfz", False))}
        return runner.catala_entfernungspauschale(s)
    return IV.bescheid_via_slots(ep_bindung, slot_fn)


def _spanne(snapshot, ep_bindung, bescheid_fn):
    r = IV.intervall(snapshot, ep_bindung, bescheid_fn)
    iv = r["intervall"]
    return iv["max_cent"] - iv["min_cent"], iv


def test_paket_a_end_to_end(ep_bindung):
    assert len(ep_bindung) == 4, "EP-Familie unvollständig in der Bindungstabelle"
    bescheid_fn = _catala_bescheid_fn(ep_bindung)   # Skip hier, falls Toolchain fehlt

    # -- 1) leerer Store -> Traverser fragt alle 4 EP-Felder --
    s = ST.leerer_store(2025, fall_id="e2e-ep")
    fragen = TR.naechste_fragen(s, ep_bindung)
    assert set(fragen) == set(EP_FELDER), f"Interview-Queue unvollständig: {fragen}"

    # -- 2) Antworten: 3x laie-bestätigt, arbeitstage llm-VORLÄUFIG --
    _laie_bestaetigt(s, "ep_entfernung_km", 30)
    _laie_bestaetigt(s, "ep_eigenes_kfz", True)
    _laie_bestaetigt(s, "ep_oepnv_kosten", 0)
    llm_ev = _llm_vorschlag(s, "ep_arbeitstage", 220)

    # arbeitstage bleibt in der Queue (vorlaeufig = noch offen), die 3 bestätigten nicht mehr
    fragen2 = TR.naechste_fragen(s, ep_bindung)
    assert fragen2 == ["ep_arbeitstage"], f"nur das offene Feld darf gefragt sein: {fragen2}"

    # -- 3) Intervall spannt auf der bounded-Achse arbeitstage (0..366) --
    snap_a, _ = ST.materialisiere(s)
    spanne_a, iv_a = _spanne(snap_a, ep_bindung, bescheid_fn)
    assert spanne_a > 0, "Intervall müsste bei offenem arbeitstage eine Spanne haben"
    assert not iv_a["min_offen"] and not iv_a["max_offen"], "arbeitstage ist bounded -> keine offene Achse"

    # -- 6a) FAIL-CLOSED vorher: Input-Kegel enthält vorlaeufig -> KEINE feste Zahl --
    zustaende_vorher = [snap_a[f]["zustand"] for f in EP_FELDER]
    assert ST.meet_zustand(zustaende_vorher) == "vorlaeufig"
    assert _feste_zahl(s, ep_bindung, bescheid_fn) is None, "vor Bestätigung darf es keine feste Zahl geben"

    # -- 4) LLM-Wert via ZWEI-SIGNAL bestätigen (ersetzt das llm-Event) --
    ST.append_event(s, feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
                    herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="ui:laie", signal={"signal_1": llm_ev["event_id"], "signal_2": "klick@beleg_arbeitstage"},
                    ersetzt=llm_ev["event_id"], ts=TS)

    # -- 5) Intervall SCHRUMPFT (alles bestätigt -> Punkt) --
    snap_b, sid_b = ST.materialisiere(s)
    spanne_b, iv_b = _spanne(snap_b, ep_bindung, bescheid_fn)
    assert spanne_b < spanne_a, "Intervall schrumpft beim Bestätigen nicht"
    assert spanne_b == 0, "vollständig bestätigt -> Punkt-Intervall"

    # -- 6b) FAIL-CLOSED nachher: Kegel bestätigt -> echte ESt --
    zustaende_nachher = [snap_b[f]["zustand"] for f in EP_FELDER]
    assert ST.meet_zustand(zustaende_nachher) == "bestaetigt"
    zahl = _feste_zahl(s, ep_bindung, bescheid_fn)
    assert zahl is not None and zahl == iv_b["min_cent"], "nach Bestätigung fehlt die feste Zahl"
    # 220 Tage, 30 km (20x0,30 + 10x0,38), eigenes Kfz, VZ 2025 -> 2156 Euro Entfernungspauschale
    assert zahl == 2156, f"EP-Bescheid unerwartet: {zahl}"

    # -- 7) Vorwärts-Trace bis anker_ref --
    tr = TR.trace_ergebnis(s, ep_bindung, snapshot_id=sid_b)
    just = next(j for j in tr["regeln"]["p09_entfernungspauschale"] if j["feld_id"] == "ep_arbeitstage")
    assert just["anker_ref"] == ep_bindung["ep_arbeitstage"]["anker_ref"]
    assert just["zustand"] == "bestaetigt" and just["signal"]["signal_2"]
    assert just["herkunft"]["herkunft"] == "laie"   # nach Bestätigung ist die Herkunft der laie-Beleg


def _feste_zahl(store, ep_bindung, bescheid_fn):
    """Fail-closed: liefert die festzusetzende Zahl NUR, wenn der Input-Kegel vollständig bestätigt
    ist (Meet über die Zustände), sonst None — der Typ als Enforcement, nicht als Bitte."""
    snap, _ = ST.materialisiere(store)
    zustaende = [snap[f]["zustand"] for f in EP_FELDER if f in snap]
    if len(zustaende) < len(EP_FELDER) or ST.meet_zustand(zustaende) != "bestaetigt":
        return None
    return bescheid_fn({f: snap[f]["wert"] for f in EP_FELDER})
