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
