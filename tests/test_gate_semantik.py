"""Regressionstests der Gate-Semantik.

Die Kaskade entscheidet ueber Rechtsregeln; ihre Semantik ist inzwischen zu
subtil, um sie nur per Hand zu pruefen. Jeder Test hier steht fuer einen Fehler,
der schon einmal passiert ist oder der still passieren koennte:

  * eine undeklarierte Zusatzannahme wird angerechnet,
  * ein erfundenes Mapping wird belohnt,
  * `scope_gap` blockiert eine korrekt spezifizierte Regel auf ewig,
  * eine Regel mit Bedingungen rutscht auf `verified`,
  * ein doppelter YAML-Schluessel ueberschreibt still ein Test-Gate.

Diese Tests laufen ohne Catala-Toolchain und ohne Netz.

    python -m pytest tests/ -q
"""

from __future__ import annotations

import json
import os
import sys

import pytest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))
sys.path.insert(0, os.path.join(ROOT, "pipeline", "produktion"))

import gates as G                     # noqa: E402
from yamlstrict import load_str, DuplicateKeyError   # noqa: E402


def verdict(annahmen=(), abweichungen=(), gaps=(), faithful=True) -> str:
    return json.dumps({"faithful": faithful,
                       "abweichungen": list(abweichungen),
                       "stille_zusatzannahmen": list(annahmen),
                       "scope_gap": list(gaps)})


BED = {"geltungsbedingungen": [
    {"bedingung": "keine_mahlzeitengestellung", "deckt_ab": "eine Mahlzeit zur Verfügung gestellt",
     "quelle": "§ 9 Abs. 4a S. 8"},
    {"bedingung": "taetigkeit_im_inland", "deckt_ab": "Bei einer Tätigkeit im Ausland",
     "quelle": "§ 9 Abs. 4a S. 5"},
]}


# -- Round-Trip: Anrechnung nur ueber explizites Mapping ----------------------

def test_gemappte_annahme_wird_angerechnet():
    a = [{"annahme": "keine Mahlzeit gestellt", "bedingung_id": "keine_mahlzeitengestellung"}]
    assert G.roundtrip_gate(verdict(a), BED).status == G.PASS


def test_unmapped_annahme_faellt():
    a = [{"annahme": "rundet auf ganze Euro ab", "bedingung_id": None}]
    assert G.roundtrip_gate(verdict(a), BED).status == G.FAIL


def test_erfundene_bedingungs_id_zaehlt_als_undeclared():
    """Ein Judge, der eine ID erfindet, darf nicht belohnt werden."""
    a = [{"annahme": "irgendwas", "bedingung_id": "diese_id_gibt_es_nicht"}]
    r = G.roundtrip_gate(verdict(a), BED)
    assert r.status == G.FAIL
    assert "angerechnet: 0" in r.detail


def test_alter_string_zaehlt_als_undeclared():
    """roundtrip_diff@3 lieferte nackte Strings. Schweigen ist keine Anrechnung."""
    assert G.roundtrip_gate(verdict(["nackte Annahme"]), BED).status == G.FAIL


def test_abweichung_faellt_immer():
    a = [{"annahme": "x", "bedingung_id": "taetigkeit_im_inland"}]
    assert G.roundtrip_gate(verdict(a, ["Cap 1500 statt 1200"]), BED).status == G.FAIL


def test_ohne_deklarierte_bedingungen_ist_jede_annahme_undeclared():
    a = [{"annahme": "x", "bedingung_id": "keine_mahlzeitengestellung"}]
    assert G.roundtrip_gate(verdict(a), {}).status == G.FAIL


def test_abgeschnittenes_verdikt_ist_kein_urteil():
    d = G.roundtrip_parse(verdict())
    d["truncated"] = True
    assert G.roundtrip_gate(d, BED).status == G.FAIL


# -- scope_gap ist informativ, geltungsbereich blockiert ----------------------

WIRKT = [{"norm_teil": "Bei einer Tätigkeit im Ausland treten an die Stelle",
          "klasse": "wirkt_hinein", "begruendung": ""}]


def test_scope_gap_blockiert_nicht_mehr():
    assert G.scope_gap_gate(verdict(gaps=WIRKT)).status == G.PASS


def test_geltungsbereich_faellt_ohne_bedingungen():
    assert G.geltungsbereich_gate(verdict(gaps=WIRKT), {}).status == G.FAIL


def test_geltungsbereich_passt_mit_abdeckender_bedingung():
    assert G.geltungsbereich_gate(verdict(gaps=WIRKT), BED).status == G.PASS


def test_unklassifizierter_gap_eskaliert():
    """Ein Judge, der die Klasse auslaesst, darf nichts durchwinken."""
    gaps = [{"norm_teil": "irgendein Satz", "begruendung": ""}]
    assert G.geltungsbereich_gate(verdict(gaps=gaps), {}).status == G.FAIL


def test_bedingung_ohne_pflichtfeld_faellt():
    bed = {"geltungsbedingungen": [{"bedingung": "x", "deckt_ab": "Bei einer Tätigkeit im Ausland"}]}
    r = G.geltungsbereich_gate(verdict(gaps=WIRKT), bed)
    assert r.status == G.FAIL and "quelle" in r.detail


# -- Test-Gate: Herkunft und Rechenweg ---------------------------------------

SRC = "declaration scope X:\n  output y content money\n\nscope X:\n  definition y equals $1.00\n"
QUELLE = "sources/gesetze-im-internet/estg_p24b_2026-07-09.txt"
ANKER = "beträgt der Entlastungsbetrag im Kalenderjahr 4 260 Euro"


def _cand(seed):
    return {"test_seed": [seed], "input_types": {}, "output_field": "y",
            "output_type": "money", "test_gate_required": True}


@pytest.mark.parametrize("herkunft", ["abgeleitet", "synthetisch"])
def test_nichtamtlicher_seed_verlangt_rechenweg(herkunft):
    seed = {"quelle": QUELLE, "zitatanker": ANKER, "inputs": {}, "expected": 1,
            "herkunft": herkunft}
    r = G.clerk_gate(SRC, "X", _cand(seed), ROOT)
    assert r.status == G.FAIL and "rechenweg" in r.detail


def test_unbekannte_herkunft_faellt():
    seed = {"quelle": QUELLE, "zitatanker": ANKER, "inputs": {}, "expected": 1,
            "herkunft": "geschaetzt", "rechenweg": "x"}
    assert G.clerk_gate(SRC, "X", _cand(seed), ROOT).status == G.FAIL


def test_gefaelschter_zitatanker_faellt_vor_dem_testlauf():
    seed = {"quelle": QUELLE, "zitatanker": "hoechstens 9.999 Euro", "inputs": {},
            "expected": 1}
    r = G.clerk_gate(SRC, "X", _cand(seed), ROOT)
    assert r.status == G.FAIL and "Zitatanker" in r.detail


def test_fehlende_seeds_lassen_das_pflicht_gate_fallen():
    cand = {"test_seed": "none", "test_gate_required": True}
    assert G.clerk_gate(SRC, "X", cand, ROOT).status == G.FAIL


def test_bakeoff_ausnahme_darf_ohne_seeds_durch():
    cand = {"test_seed": "none", "test_gate_required": False}
    assert G.clerk_gate(SRC, "X", cand, ROOT).status == G.SKIP


# -- Rundungs-Lint: Rundung nur mit deklarierter Quelle -----------------------

_ROUND_SRC = ("scope X:\n  definition y equals\n"
              "  (Decimal.truncate of (roh / $1.00)) * $1.00\n")
_PLAIN_SRC = "scope X:\n  definition y equals a + b\n"
_NORM = "... nach § 32a Absatz 1 Satz 6 ist auf den naechsten vollen Euro abzurunden ..."


def test_rundung_ohne_deklaration_faellt_mit_zeile():
    r = G.rundungs_lint_gate(_ROUND_SRC, {"rundung": [], "norm_text": _NORM})
    assert r.status == G.FAIL
    assert "Zeile 3" in r.detail and "truncate" in r.detail


def test_keine_rundung_passt():
    assert G.rundungs_lint_gate(_PLAIN_SRC, {}).status == G.PASS


def test_rundung_mit_gueltiger_deklaration_passt():
    cand = {"norm_text": _NORM, "rundung": [
        {"quelle": "§ 32a Abs. 1 S. 6", "zitatanker": "auf den naechsten vollen Euro abzurunden"}]}
    assert G.rundungs_lint_gate(_ROUND_SRC, cand).status == G.PASS


def test_rundungs_deklaration_ohne_anker_in_norm_faellt():
    """Eine Rundungs-Erlaubnis, deren Zitatanker nicht in der Norm steht, ist eine
    leere Behauptung und deckt nichts."""
    cand = {"norm_text": _NORM, "rundung": [
        {"quelle": "erfunden", "zitatanker": "diesen Satz gibt es nicht"}]}
    assert G.rundungs_lint_gate(_ROUND_SRC, cand).status == G.FAIL


def test_rundung_kommentar_zaehlt_nicht():
    src = "scope X:\n  # hier wuerde man round benutzen\n  definition y equals a\n"
    assert G.rundungs_lint_gate(src, {}).status == G.PASS


# -- Money-Literale: Cent-Betraege muessen ueberleben -------------------------

def test_money_literal_behaelt_cent():
    """1.408,70 EUR ist der amtliche BFH-Erwartungswert. int(value) machte $1408.00."""
    assert G._lit(1408.70, "money") == "$1,408.70"
    assert G._lit(22.40, "money") == "$22.40"


# -- Queue-Status -------------------------------------------------------------

def test_queue_status():
    from run import _queue_status
    gruen = [{"name": "clerk", "status": G.PASS}]
    rot = [{"name": "clerk", "status": G.FAIL}]
    assert _queue_status(gruen, {}) == "verified"
    assert _queue_status(gruen, {"geltungsbedingungen": [{"bedingung": "x"}]}) == "verified_bedingt"
    assert _queue_status(gruen, {"freigabe": "blockiert"}) == "freigabe_blockiert"
    assert _queue_status(rot, {"freigabe": "blockiert"}) == "flagged_for_review"
    # `_first`-Gates sind Diagnose, kein Eskalationsgrund
    assert _queue_status(gruen + [{"name": "typecheck_a_first", "status": G.FAIL}], {}) == "verified"


# -- Pre-Call-Kosten-Cap (budget_abbruch) + Falschgruen-Sperre ----------------

def test_leere_gates_nie_verified_run():
    """Falschgruen-Sperre: ein Report ohne bewertbare Gates (budget_abbruch/Abbruch)
    darf in KEINEM Pfad verified werden - auch nicht mit Geltungsbedingungen."""
    from run import _queue_status
    assert _queue_status([], {}) == "unbewertet"
    assert _queue_status([], {"geltungsbedingungen": [{"bedingung": "x"}]}) == "unbewertet"
    # nur _first-/discovery-Gates zaehlen nicht als bewertbar -> weiter unbewertet
    assert _queue_status([{"name": "discovery", "status": G.SKIP}], {}) == "unbewertet"
    assert _queue_status([{"name": "syntax_a_first", "status": G.PASS}], {}) == "unbewertet"


def test_leere_gates_nie_verified_cascade():
    """Zwei-Funktions-Falle: dieselbe Invariante im cascade-Pfad."""
    import cascade as C
    assert C._queue_status([], {}, []) == "unbewertet"
    assert C._queue_status([], {"geltungsbedingungen": [{"bedingung": "x"}]}, []) == "unbewertet"


def test_estimate_cost_kalibrierung():
    from run import _estimate_cost
    assert _estimate_cost({"quellen": [{}, {}, {}]}) == 0.15  # multi-quellig
    assert _estimate_cost({"quellen": [{}]}) == 0.07          # 1-quellig
    assert _estimate_cost({}) == 0.07                          # ohne Quellen -> 1-quellig-Default


def test_budget_abbruch_report_bleibt_unbewertet():
    """Simulierter Ueberlauf: der Pre-Call-Cap-Report muss durch BEIDE _queue_status-Pfade
    unbewertet bleiben - nie verified (auch mit Geltungsbedingungen)."""
    from run import _budget_abbruch_report, _queue_status
    import cascade as C
    rep = _budget_abbruch_report("regelX", total_cost=0.39, est=0.15, cap=0.40)
    assert rep["queue_status"] == "budget_abbruch"
    assert rep["gates"] == []
    gb = {"geltungsbedingungen": [{"bedingung": "x"}]}
    assert _queue_status(rep["gates"], gb) == "unbewertet"
    assert C._queue_status(rep["gates"], gb, []) == "unbewertet"


def test_pre_call_cap_schwellenlogik():
    """Deterministische Abbruch-Entscheidung: kumuliert + Schaetzung > Cap -> Abbruch."""
    from run import _estimate_cost
    cap, total = 0.40, 0.30
    # multi-quellig (est 0.15): 0.30 + 0.15 = 0.45 > 0.40 -> Abbruch
    assert total + _estimate_cost({"quellen": [{}, {}]}) > cap
    # 1-quellig (est 0.07): 0.30 + 0.07 = 0.37 <= 0.40 -> laeuft weiter
    assert not (total + _estimate_cost({"quellen": [{}]}) > cap)


def test_only_multi_selektion():
    """Multi-only: --only nimmt eine oder mehrere kommagetrennte rule_ids; Reihenfolge =
    Manifest (nicht Argument), Whitespace toleriert, unbekannte ignoriert."""
    from run import _select_rules
    rules = [{"rule_id": "a"}, {"rule_id": "b"}, {"rule_id": "c"}]
    assert [r["rule_id"] for r in _select_rules(rules, "a")] == ["a"]
    assert [r["rule_id"] for r in _select_rules(rules, "a,c")] == ["a", "c"]
    assert [r["rule_id"] for r in _select_rules(rules, "c, a")] == ["a", "c"]  # Manifest-Reihenfolge
    assert [r["rule_id"] for r in _select_rules(rules, "b , c")] == ["b", "c"]  # Whitespace
    assert _select_rules(rules, "x") == []  # unbekannt -> leer (Aufrufer wirft "keine Regel")


def test_budget_abbruch_kein_checkpoint():
    """F1 Checkpoint-Falle: ein budget_abbruch-Report (nie gelaufene Regel) darf beim
    Wiederanlauf NICHT als Checkpoint uebersprungen werden, echte Reports schon."""
    from run import _ist_checkpoint
    assert _ist_checkpoint({"queue_status": "verified_bedingt"}) is True
    assert _ist_checkpoint({"queue_status": "discovery_triage"}) is True
    assert _ist_checkpoint({"queue_status": "flagged_for_review"}) is True
    assert _ist_checkpoint({"queue_status": "budget_abbruch"}) is False  # Neuanlauf, kein Skip


# -- Strikter YAML-Loader -----------------------------------------------------

def test_doppelter_schluessel_ist_ein_fehler():
    with pytest.raises(DuplicateKeyError):
        load_str("regel:\n  test_seed: none\n  test_seed: none\n")


def test_manifeste_laden_strikt():
    from yamlstrict import load_yaml
    for rel in ("pipeline/produktion/rules.yaml", "pipeline/models.yaml",
                "pipeline/bakeoff/tasks.yaml"):
        assert load_yaml(os.path.join(ROOT, rel))


# -- regate darf ein kaputtes Verdikt nicht ueberspringen ---------------------

REGCAND = {"id": "testregel_ohne_registry", "geltungsbedingungen": []}


@pytest.mark.parametrize("kaputt", [{"parse_error": True}, {"truncated": True}])
def test_judge_gates_fallen_bei_kaputtem_verdikt(kaputt):
    """§ 9 Abs. 4a stand zwischenzeitlich gruen, weil `--regate` bei einem
    parse_error-Verdikt die Judge-Gates uebersprang. judge_gates gibt jetzt
    (gates, discoveries) zurueck (Registry-Ratsche)."""
    from run import judge_gates
    g, disc = judge_gates(kaputt, REGCAND)
    assert {"roundtrip", "geltungsbereich", "grenzfall"} <= set(g)
    assert all(x.status == G.FAIL for x in g.values())
    assert disc == []


def test_judge_gates_bei_gutem_verdikt_leere_registry():
    """Ohne registrierte Items sind die deterministischen Gates gruen; neue Funde
    landen in der Discovery-Queue, kippen aber kein Gate."""
    from run import judge_gates
    a = [{"annahme": "keine Mahlzeit", "bedingung_id": "keine_mahlzeitengestellung",
          "betrifft": "x", "kategorie": "interpretation"}]
    g, disc = judge_gates(G.roundtrip_parse(verdict(a, gaps=WIRKT)), REGCAND)
    assert g["roundtrip"].status == G.PASS
    assert g["geltungsbereich"].status == G.PASS
    assert g["discovery"].status == G.SKIP
    assert len(disc) >= 1


def test_lit_negatives_money_ist_catala_parsebar():
    """gates._lit fuer negatives Money: das Minus steht VOR dem Dollar
    (-$3,000.00), nicht dahinter ($-3,000.00 = Catala-Parse-Fehler).

    Regression fuer § 36 (Erstattung/Ueberschuss zugunsten des Steuerpflichtigen)
    und jede Regel mit negativem Output. Vor dem Fix (2026-07-12) erzeugte _lit
    '$-3,000.00' und liess das clerk-Gate still an einem Harness-Defekt scheitern -
    ein roter Test hat nie existiert, der Fix war damit unbewiesen. Dieser Test
    schliesst die Luecke: er waere vor dem Fix rot gewesen."""
    assert G._lit(-3000.0, "money") == "-$3,000.00"
    assert G._lit(-22.40, "money") == "-$22.40"
    # Kein Dollar-vor-Minus mehr (der eigentliche Bug).
    assert not G._lit(-1.0, "money").startswith("$-")
    # Positive und Cent-Betraege bleiben unveraendert korrekt.
    assert G._lit(4499.0, "money") == "$4,499.00"
    assert G._lit(1408.70, "money") == "$1,408.70"
    assert G._lit(0.0, "money") == "$0.00"


def test_rundungs_lint_faengt_richtung():
    """rundungs_lint prueft die RICHTUNG der deklarierten Rundung. Die Norm ordnet
    floor an ('Bruchteile eines Cents bleiben ausser Ansatz'=abschneiden), der Code
    rundet kaufmaennisch (round) -> FAIL. Regression fuer das solzg-Residual
    (2026-07-12): der Lint gibt dem Repair-Loop ein deterministisches Richtungssignal
    statt Interpretationsglueck. Waere vor der Richtungs-Erweiterung gruen gewesen."""
    cand = {"norm_text": "Bruchteile eines Cents bleiben ausser Ansatz",
            "rundung": [{"zitatanker": "Bruchteile eines Cents bleiben ausser Ansatz",
                         "quelle": "§ 4 SolzG", "richtung": "floor"}]}
    src_round = "scope X:\n  definition y equals round of z\n"   # kaufmaennisch
    src_trunc = "scope X:\n  definition y equals truncate of z\n"  # floor
    assert G.rundungs_lint_gate(src_round, cand).status == G.FAIL   # kaufmaennisch != floor
    assert G.rundungs_lint_gate(src_trunc, cand).status == G.PASS   # floor == floor
    # aufrunden (ceil) deklariert, Code schneidet ab (truncate/floor) -> FAIL
    cand_ceil = {"norm_text": "auf volle Euro aufzurunden",
                 "rundung": [{"zitatanker": "auf volle Euro aufzurunden",
                              "quelle": "§ 36 Abs. 3", "richtung": "ceil"}]}
    assert G.rundungs_lint_gate(src_trunc, cand_ceil).status == G.FAIL
    assert G.rundungs_lint_gate("scope X:\n  definition y equals ceiling of z\n",
                                cand_ceil).status == G.PASS
    # Legacy: rundung ohne richtung-Feld -> deklarierte Rundung genuegt (jede Richtung)
    cand_legacy = {"norm_text": "Bruchteile eines Cents bleiben ausser Ansatz",
                   "rundung": [{"zitatanker": "Bruchteile eines Cents bleiben ausser Ansatz",
                                "quelle": "§ 4 SolzG"}]}
    assert G.rundungs_lint_gate(src_round, cand_legacy).status == G.PASS


# --- Praezisions-Lint (Klasse 5): money x/ decimal VOR finalem Cent-Schnitt -------
# Vorregistrierung reports/review/2026-07-12-praezisions-lint-vorregistrierung.md,
# inkl. der drei Adversarial-Faelle des Instructors (Fluss-Sensitivitaet, Division).

def _scope(body: str) -> str:
    return ("declaration scope S:\n  input bmg content money\n"
            "  output out content money\nscope S:\n" + body)


PRAEZ_BUGGY = _scope(
    "  definition out equals\n"
    "    let unterschied equals bmg - $20,350.00 in\n"
    "    let milderung equals unterschied * 0.119 in\n"
    "    (Decimal.truncate of (milderung / $0.01)) * $0.01")

PRAEZ_FIX = _scope(
    "  definition out equals\n"
    "    let unterschied_dec equals (bmg - $20,350.00) / $1.00 in\n"
    "    let milderung_dec equals unterschied_dec * 0.119 in\n"
    "    (Decimal.truncate of (milderung_dec / $0.01)) * $0.01")


def test_praezisions_lint_flaggt_money_dec_vor_cent_schnitt():
    # Stufe 2 (Default seit Julius-Freigabe 2026-07-12): confident-Befund -> FAIL.
    r = G.praezisions_lint_gate(PRAEZ_BUGGY)
    assert r.status == G.FAIL
    assert "Klasse-5" in r.detail


def test_praezisions_lint_fix_form_passt():
    # decimal bis zum Ende, Cent-Schnitt am Schluss -> kein money x decimal -> PASS.
    assert G.praezisions_lint_gate(PRAEZ_FIX).status == G.PASS


def test_praezisions_lint_money_dec_ohne_cut_passt():
    # money x decimal OHNE finalen Cent-Schnitt -> Cent-Rundung gewollt, legitim.
    src = _scope("  definition out equals bmg * 0.05")
    assert G.praezisions_lint_gate(src).status == G.PASS


def test_praezisions_lint_division_flaggt_wie_multiplikation():
    # Adversarial 3: money / decimal rundet genauso cent-mittig -> MUSS flaggen.
    src = _scope(
        "  definition out equals\n"
        "    let anteil equals bmg / 2.0 in\n"
        "    (Decimal.truncate of (anteil / $0.01)) * $0.01")
    assert G.praezisions_lint_gate(src).status == G.FAIL


def test_praezisions_lint_money_dec_NACH_schnitt_flaggt_nicht():
    # Adversarial 1: Nachverarbeitung nach dem Schnitt -> Ordnung zaehlt, nicht Praesenz.
    src = ("declaration scope S:\n  input bmg content money\n"
           "  input darstellung content money\n  output out content money\nscope S:\n"
           "  definition out equals\n"
           "    let geschnitten equals (Decimal.truncate of (bmg / $0.01)) * $0.01 in\n"
           "    let angezeigt equals darstellung * 1.5 in\n"
           "    geschnitten + angezeigt")
    assert G.praezisions_lint_gate(src).status == G.PASS


def test_praezisions_lint_nebenvariable_ohne_fluss_flaggt_nicht():
    # Adversarial 2: korrekte decimal-Form im Fluss zum Schnitt, plus ein money x
    # decimal in einer Nebenvariable, die NICHT in den Schnitt fliesst -> kein Flag.
    src = ("declaration scope S:\n  input bmg content money\n"
           "  input extra content money\n  output out content money\nscope S:\n"
           "  definition out equals\n"
           "    let dec equals (bmg / $1.00) * 0.119 in\n"
           "    let nebensache equals extra * 0.05 in\n"
           "    (Decimal.truncate of (dec / $0.01)) * $0.01")
    assert G.praezisions_lint_gate(src).status == G.PASS


def test_praezisions_lint_inline_in_schnitt_flaggt():
    # money x decimal INLINE in der Schnitt-Expression selbst -> fliesst direkt hinein.
    src = _scope(
        "  definition out equals (Decimal.truncate of ((bmg * 0.119) / $0.01)) * $0.01")
    assert G.praezisions_lint_gate(src).status == G.FAIL


def test_praezisions_lint_ohne_source_skip():
    assert G.praezisions_lint_gate(None).status == G.SKIP


def test_praezisions_lint_stufe1_fallback_informativ(monkeypatch):
    # Der Stufe-1-Modus (informativ) bleibt per Umschalter erreichbar: confident-Befund
    # -> INFO (kippt kein Gate) statt FAIL. Belegt beide Rollout-Stufen des einen Gates.
    monkeypatch.setattr(G, "_PRAEZISION_BLOCKIEREND", False)
    r = G.praezisions_lint_gate(PRAEZ_BUGGY)
    assert r.status == G.INFO
    assert "Stufe 1 informativ" in r.detail
    assert G.praezisions_lint_gate(PRAEZ_FIX).status == G.PASS


def test_praezisions_lint_definition_form_wird_erkannt():
    # Regression (solzg-Repair 2026-07-12): ein Repair schrieb die let-Kette in top-level
    # `definition`s um und behielt money x decimal - das Gate gab faelschlich PASS, weil
    # die Fluss-Erreichbarkeit nur `let` verfolgte. Muss jetzt INFO sein.
    src = ("declaration scope S:\n  input bmg content money\n"
           "  output solz content money\n  internal fg content money\n"
           "  internal normal content money\n  internal milderung content money\n"
           "  internal vor content money\nscope S:\n"
           "  definition fg equals $20,350.00\n"
           "  definition normal equals bmg * 0.055\n"
           "  definition milderung equals (bmg - fg) * 0.119\n"
           "  definition vor equals\n"
           "    if bmg <= fg then $0.00\n"
           "    else (if normal <= milderung then normal else milderung)\n"
           "  definition solz equals (Decimal.truncate of (vor / $0.01)) * $0.01")
    assert G.praezisions_lint_gate(src).status == G.FAIL


def test_praezisions_lint_geklammerte_money_differenz_wird_erkannt():
    # Regression: `(a - b) * 0.119` - der linke Operand ist ein geklammerter money-
    # Ausdruck, kein blosser Name. Die fruehere Operanden-Heuristik uebersah das.
    src = ("declaration scope S:\n  input bmg content money\n  input fg content money\n"
           "  output out content money\n  internal mild content money\nscope S:\n"
           "  definition mild equals (bmg - fg) * 0.119\n"
           "  definition out equals (Decimal.truncate of (mild / $0.01)) * $0.01")
    assert G.praezisions_lint_gate(src).status == G.FAIL


def test_skip_judge_queue_status_nie_verified():
    # Falschgruen-Sperre (Instructor-Leitplanke 2026-07-12): ein skip_judge-Lauf hat kein
    # Judge-Verdikt und darf NIE Richtung verified/verified_bedingt laufen - auch wenn alle
    # deterministischen Gates gruen sind. Er bleibt strukturell/vorlaeufig.
    import cascade as C
    def gr(n, st): return G.GateResult(n, st)
    gruen = [gr("syntax_a", G.PASS), gr("typecheck_a", G.PASS),
             gr("clerk", G.PASS), gr("rundungs_lint", G.PASS)]
    cand_bed = {"geltungsbedingungen": [{"bedingung": "x", "deckt_ab": "y", "quelle": "z"}]}
    # Ohne skip_judge waere das verified_bedingt:
    assert C._queue_status(gruen, cand_bed, [], judge_skipped=False) == "verified_bedingt"
    # MIT skip_judge NIE verified:
    st = C._queue_status(gruen, cand_bed, [], judge_skipped=True)
    assert st == "strukturgeprueft_judge_offen"
    assert "verified" not in st
    # Ein FAIL kippt weiterhin Vorrang (skip_judge macht kein rotes Gate gruen):
    rot = gruen + [gr("typecheck_a", G.FAIL)]
    assert C._queue_status(rot, cand_bed, [], judge_skipped=True) == "flagged_for_review"


def test_gates_fuer_skipped_verdikt_nie_registry_pass():
    # Regate-Falschgruen (Instructor-Verifikations-Befund 2026-07-12): ein bewusst
    # uebersprungenes Judge-Verdikt {skipped:true} darf im Ableitungs-Pfad (gates_fuer,
    # genutzt von --regate) NICHT als vollwertiges Verdikt gelesen werden - sonst faellt der
    # Abgleich auf null Items gegen eine leere Registry und alle Registry-Gates werden
    # faelschlich PASS -> verified_bedingt. Judge-abhaengige Gates muessen SKIP sein.
    import item_registry as IR
    gates, disc = IR.gates_fuer("beliebige_regel", {"geltungsbedingungen": []}, {"skipped": True})
    assert disc == []
    for n in ("roundtrip", "geltungsbereich", "grenzfall", "defekt", "scope_gap", "discovery"):
        assert gates[n].status == G.SKIP, (n, gates[n].status)
        assert "Nachzug" in gates[n].detail
    # kein einziges Registry-Gate PASS - das war die Falschgruen-Quelle:
    assert G.PASS not in [g.status for g in gates.values()]


def test_regate_queue_status_skipped_nie_verified():
    # Der Regate-Pfad nutzt run._queue_status (NICHT cascade._queue_status - zwei getrennte
    # Funktionen, die Sperre muss in BEIDEN sein). skipped-Verdikt + gruene Gates +
    # Geltungsbedingungen -> strukturgeprueft_judge_offen, NIE verified_bedingt. Dieser Test
    # haette das Instructor-Regate-Gruen von 2026-07-12 gefangen.
    import run as R
    def g(n, st): return {"name": n, "status": st, "detail": ""}
    gruen = [g("syntax_a", G.PASS), g("typecheck_a", G.PASS),
             g("clerk", G.PASS), g("equivalence", G.PASS)]
    rule_bed = {"geltungsbedingungen": [{"bedingung": "x"}]}
    # Echtes Verdikt -> verified_bedingt:
    assert R._queue_status(gruen, rule_bed, {"stille_zusatzannahmen": []}, []) == "verified_bedingt"
    # skipped-Verdikt -> nie verified:
    st = R._queue_status(gruen, rule_bed, {"skipped": True}, [])
    assert st == "strukturgeprueft_judge_offen"
    assert "verified" not in st


def test_discover_draft_skipped_bricht_ab(tmp_path, monkeypatch):
    # Falschgruen-Tuer im Triage-Workflow: ein skipped-Verdikt hat null Items; ein
    # stiller Leer-Entwurf saehe aus wie "Queue sauber". discover_draft muss abbrechen,
    # nicht einen leeren Entwurf liefern.
    import item_registry as IR
    monkeypatch.setattr(IR, "ROOT", str(tmp_path))
    rid = "beliebige_regel"
    d = tmp_path / "pipeline" / "runs" / "produktion" / rid
    d.mkdir(parents=True)
    (d / "report.json").write_text(
        json.dumps({"candidate_id": rid, "judge_verdict": {"skipped": True}}),
        encoding="utf-8")
    with pytest.raises(SystemExit, match="uebersprungen"):
        IR.discover_draft(rid)
