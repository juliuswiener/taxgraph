"""Tests der Registry-Ratsche (Stufe 4).

Die zentrale Eigenschaft: die Gates haengen NUR an der Registry + am Manifest, nicht
am frischen Lauf. Identischer Registry-Stand -> identisches Verdikt, per
Konstruktion. Neue Funde kippen kein Gate. Kein Netz, kein Modell.
"""

from __future__ import annotations

import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

import item_registry as IR   # noqa: E402
import gates as G            # noqa: E402


def reg(items):
    return {"rule_id": "t", "version": 1, "items": items}


NT = lambda ref, triage, bed=None: {  # noqa: E731
    "art": "norm_teil", "anker": ["ref", ref], "klasse": "wirkt_hinein",
    "triage": triage, **({"bedingung": bed} if bed else {})}
AN = lambda b, k, triage, bed=None: {  # noqa: E731
    "art": "annahme", "anker": ["betrifft_kat", b, k], "triage": triage,
    **({"bedingung": bed} if bed else {})}


def rule(*bed):
    return {"geltungsbedingungen": [{"bedingung": b} for b in bed]}


# -- Determinismus: Gate haengt nur an Registry + Manifest --------------------

def test_geltungsbereich_deterministisch_unabhaengig_vom_lauf():
    r = reg([NT("§ 9 abs. 2", "bedingung_neu", "nur_inland")])
    # deklariert -> PASS
    assert IR.geltungsbereich_gate(r, rule("nur_inland")).status == G.PASS
    # nicht deklariert -> FAIL
    assert IR.geltungsbereich_gate(r, rule()).status == G.FAIL
    # Das Ergebnis ist bei identischem (Registry, Manifest) reproduzierbar.
    assert (IR.geltungsbereich_gate(r, rule("nur_inland")).status
            == IR.geltungsbereich_gate(r, rule("nur_inland")).status)


def test_grenzfall_item_faellt_grenzfall_gate():
    r = reg([NT("§ 9 abs. 3", "grenzfall")])
    assert IR.grenzfall_gate(r).status == G.FAIL
    # ein grenzfall-Item ist KEINE geltungsbereich-Luecke
    assert IR.geltungsbereich_gate(r, rule()).status == G.PASS


def test_nicht_material_und_nicht_echt_blockieren_nie():
    r = reg([AN("x", "einheit", "nicht_material"),
             AN("y", "sonstige", "nicht_echt")])
    assert IR.roundtrip_gate(r, rule()).status == G.PASS
    assert IR.geltungsbereich_gate(r, rule()).status == G.PASS


def test_annahme_bedingung_neu_ohne_deklaration_faellt_roundtrip():
    r = reg([AN("alleinstehend", "interpretation", "bedingung_neu", "def_abs3")])
    assert IR.roundtrip_gate(r, rule()).status == G.FAIL
    assert IR.roundtrip_gate(r, rule("def_abs3")).status == G.PASS


# -- Judge als Detektor: neue Funde -> Discovery, kein Gate-Kippen -------------

def test_abgleich_trennt_bekannt_von_neu():
    r = reg([AN("x", "einheit", "nicht_material")])
    verdict = {"abweichungen": [], "scope_gap": [],
               "stille_zusatzannahmen": [
                   {"annahme": "x ist netto", "betrifft": "x", "kategorie": "einheit",
                    "bedingung_id": None},
                   {"annahme": "neuer Fund", "betrifft": "z", "kategorie": "zeitbezug",
                    "bedingung_id": None}]}
    a = IR.abgleich(r, verdict)
    assert len(a["bekannt"]) == 1
    assert len(a["neu"]) == 1
    assert a["neu"][0]["anker"] == ["betrifft_kat", "z", "zeitbezug"]


def test_neue_funde_kippen_kein_gate():
    """Registry leer, Verdikt voller wirkt_hinein-Items -> geltungsbereich bleibt
    PASS. Erst Triage in die Registry aendert etwas."""
    r = reg([])
    verdict = {"abweichungen": [], "stille_zusatzannahmen": [],
               "scope_gap": [{"norm_teil": "t", "referenz": "§ 1",
                              "klasse": "wirkt_hinein", "abgedeckt_von": "none"}]}
    gates, neu = IR.gates_fuer("t", rule(), verdict)
    assert gates["geltungsbereich"].status == G.PASS
    assert gates["discovery"].status == G.SKIP
    assert len(neu) == 1


def test_kaputtes_verdikt_faellt_gates():
    gates, neu = IR.gates_fuer("t", rule(), {"parse_error": True})
    assert gates["geltungsbereich"].status == G.FAIL
    assert gates["roundtrip"].status == G.FAIL
    assert neu == []


# -- neue Triage-Status (Stufe 4, Erstbefuellung) -----------------------------

def test_defekt_formalisierer_blockiert_freigabe_nicht_als_bedingung():
    r = reg([{"art": "annahme", "anker": ["betrifft_kat", "ergebnis", "rundung"],
              "triage": "defekt_formalisierer"}])
    assert IR.defekt_gate(r).status == G.FAIL
    # kein bedingung_neu -> roundtrip/geltungsbereich unberuehrt
    assert IR.roundtrip_gate(r, rule()).status == G.PASS
    assert IR.geltungsbereich_gate(r, rule()).status == G.PASS


def test_aufnehmen_verweigert_bedingung_auf_defekt(tmp_path, monkeypatch):
    monkeypatch.setattr(IR, "REG_DIR", str(tmp_path))
    import pytest
    with pytest.raises(SystemExit):
        IR.aufnehmen({"rule_id": "r", "items": [
            {"art": "annahme", "anker": ["betrifft_kat", "e", "rundung"],
             "triage": "defekt_formalisierer", "bedingung": "boese"}]})


def test_offen_bis_neuschnitt_blockiert_nicht():
    r = reg([{"art": "norm_teil", "anker": ["ref", "§ 9 abs. 1 s. 3 nr. 5a s. 1"],
              "klasse": "wirkt_hinein", "triage": "offen_bis_neuschnitt"}])
    # kein bedingung_neu -> keine geltungsbereich-Luecke
    assert IR.geltungsbereich_gate(r, rule()).status == G.PASS
    assert IR.defekt_gate(r).status == G.PASS


def test_gates_fuer_enthaelt_defekt():
    gates, _ = IR.gates_fuer("t", rule(), {"abweichungen": [], "scope_gap": [],
                                           "stille_zusatzannahmen": []})
    assert "defekt" in gates


# -- Daempfung 2a: unabhaengige Norm-Teile -> Backlog, kein Gate --------------

def test_nicht_material_backlog_blockiert_nie():
    r = reg([{"art": "norm_teil", "anker": ["ref", "§ 1"], "klasse": "unabhaengig",
              "triage": "nicht_material_backlog"}])
    assert IR.geltungsbereich_gate(r, rule()).status == G.PASS
    assert IR.roundtrip_gate(r, rule()).status == G.PASS
    assert IR.defekt_gate(r).status == G.PASS
    assert IR.grenzfall_gate(r).status == G.PASS


def test_discover_draft_daempft_unabhaengig(tmp_path, monkeypatch):
    """Ein unabhaengiger Norm-Teil geht mechanisch in den Backlog; ein
    wirkt_hinein mit Deckung wird Detektor-Vorschlag; ohne Deckung -> offen."""
    import json
    run = tmp_path / "pipeline" / "runs" / "produktion" / "r"
    run.mkdir(parents=True)
    (run / "report.json").write_text(json.dumps({"judge_verdict": {
        "abweichungen": [], "stille_zusatzannahmen": [],
        "scope_gap": [
            {"norm_teil": "unabh Satz", "referenz": "§ 1", "klasse": "unabhaengig",
             "abgedeckt_von": "none"},
            {"norm_teil": "deckt Satz", "referenz": "§ 2", "klasse": "wirkt_hinein",
             "abgedeckt_von": "eine_bedingung"},
            {"norm_teil": "offen Satz", "referenz": "§ 3", "klasse": "wirkt_hinein",
             "abgedeckt_von": "none"}]}}))
    monkeypatch.setattr(IR, "ROOT", str(tmp_path))
    monkeypatch.setattr(IR, "REG_DIR", str(tmp_path / "reg"))
    d = IR.discover_draft("r")
    tri = {e["anker"][1]: e["triage"] for e in d["items"]}
    assert tri["§ 1"] == "nicht_material_backlog"
    assert tri["§ 2"] == "bedingung_neu"
    assert tri["§ 3"] == "offen"


# -- Anker-Fix Abweichungen (Schritt 5): nicht mehr degeneriert ---------------

def test_abweichung_verschiedene_texte_verschiedene_schluessel():
    """Verschiedene Abweichungen einer Regel bekommen verschiedene Schluessel -
    kein Sammel-Eintrag ["betrifft","?"] mehr, der jede weitere echte Abweichung
    als "bekannt" schluckte (stilles Gruen)."""
    v = {"abweichungen": ["A rundet das Ergebnis ab, B nicht.",
                          "A verlangt Uebernachtung am selben Tag."],
         "stille_zusatzannahmen": [], "scope_gap": []}
    neu = IR.abgleich(reg([]), v)["neu"]
    keys = {n["schluessel"] for n in neu}
    assert len(neu) == 2 and len(keys) == 2
    assert all(n["anker"] != ["betrifft", "?"] for n in neu)


def test_abweichung_gleicher_wortlaut_stabiler_schluessel():
    """Dieselbe Abweichung mit IDENTISCHEM Wortlaut -> derselbe Schluessel ueber
    Laeufe. Achtung: gilt nur bei identischem Wortlaut. Driftet der Judge-Wortlaut,
    entsteht ein neuer Schluessel -> Re-Discovery + erneute Triage (sichere
    Fehlrichtung, kein stilles Schlucken). Stabil wird es erst mit dem
    verpflichtenden `betrifft`-Feld (Backlog, siehe _anker)."""
    a = "A rundet das Ergebnis ab, B nicht."
    mk = lambda: IR.abgleich(  # noqa: E731
        reg([]), {"abweichungen": [a], "stille_zusatzannahmen": [],
                  "scope_gap": []})["neu"][0]["schluessel"]
    assert mk() == mk()


def test_abweichung_betrifft_wird_bevorzugt():
    """Traegt die Abweichung ein `betrifft`, ankert sie darauf (stabil) statt am
    driftenden Text - der Dict-Pfad des Judge-Schemas."""
    an = IR._anker("abweichung", {"betrifft": "ergebnis",
                                  "aussage": "irgendein driftender Wortlaut"})
    assert an == ["betrifft", "ergebnis"]


# -- Detector-dedup-per-key ---------------------------------------------------

def test_dedup_items_widerspruch_wird_offen():
    """Doppel-Vorbelegung desselben Items (gleicher _key) mit widerspruechlicher Triage
    -> ein Item, konservativ 'offen', Vorbelegungs-Felder entfernt, Texte zusammengefuehrt."""
    items = [
        {"art": "annahme", "anker": ["x"], "text": "a", "triage": "nicht_material", "konvention": "konv:y"},
        {"art": "annahme", "anker": ["x"], "text": "b", "triage": "bedingung_neu", "bedingung": "z"},
    ]
    out = IR._dedup_items(items)
    assert len(out) == 1
    assert out[0]["triage"] == "offen"
    assert "konvention" not in out[0] and "bedingung" not in out[0]
    assert "a" in out[0]["text"] and "b" in out[0]["text"]


def test_dedup_items_verschiedene_keys_bleiben():
    items = [{"art": "annahme", "anker": ["x"], "text": "a", "triage": "offen"},
             {"art": "annahme", "anker": ["y"], "text": "b", "triage": "offen"}]
    assert len(IR._dedup_items(items)) == 2


def test_dedup_items_gleiche_triage_merged_text():
    items = [{"art": "norm_teil", "anker": ["p"], "text": "t1", "triage": "nicht_material_backlog"},
             {"art": "norm_teil", "anker": ["p"], "text": "t2", "triage": "nicht_material_backlog"}]
    out = IR._dedup_items(items)
    assert len(out) == 1 and "t1" in out[0]["text"] and "t2" in out[0]["text"]
    assert out[0]["triage"] == "nicht_material_backlog"  # kein Konflikt -> bleibt
