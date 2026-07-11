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
