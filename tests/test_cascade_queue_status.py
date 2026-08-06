"""Test: cascade._queue_status — Falsch-Gruen-Sperre in der Queue-Entscheidung.

Die Queue-Entscheidung ist eine INHALTLICHE Regel: "ein leerer Fehlerpuffer bei
rc!=0 heisst nicht geprueft, nicht fehlerfrei". Wenn _queue_status einen Lauf mit
FAIL-Gates als "verified" durchlässt, rutscht eine kaputte Regel zu Julius.
"""
import sys
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

import cascade as CA  # noqa: E402
import gates as G  # noqa: E402


# -- _queue_status: alle Zweige -------------------------------------------------

def test_queue_status_unbewertet():
    """Keine bewertbaren Gates (nur _first und discovery) -> unbewertet."""
    r = [G.GateResult("syntax_a_first", G.PASS, ""),
         G.GateResult("syntax_b_first", G.PASS, ""),
         G.GateResult("discovery", G.SKIP, "")]
    assert CA._queue_status(r, {}, []) == "unbewertet"


def test_queue_status_flagged_fail():
    """Ein FAIL-Gate (nicht _first) -> flagged_for_review."""
    r = [G.GateResult("syntax_a", G.FAIL, "boom")]
    assert CA._queue_status(r, {}, []) == "flagged_for_review"


def test_queue_status_discovery_triage():
    """Kein FAIL, aber discoveries -> discovery_triage."""
    r = [G.GateResult("syntax_a", G.PASS, "")]
    assert CA._queue_status(r, {}, [{"id": "new"}]) == "discovery_triage"


def test_queue_status_judge_skipped():
    """Kein FAIL, keine discoveries, aber judge_skipped -> strukturgeprueft_judge_offen."""
    r = [G.GateResult("syntax_a", G.PASS, "")]
    assert CA._queue_status(r, {}, [], judge_skipped=True) == "strukturgeprueft_judge_offen"


def test_queue_status_verified_partial():
    """Kein FAIL, keine discoveries, kein judge_skipped, aber SKIP -> verified_partial."""
    r = [G.GateResult("syntax_a", G.PASS, ""),
         G.GateResult("typecheck_a", G.SKIP, "toolchain fehlt")]
    assert CA._queue_status(r, {}, []) == "verified_partial (toolchain pending)"


def test_queue_status_verified_bedingt():
    """Kein FAIL/SKIP/discovery/judge_skipped, aber geltungsbedingungen -> verified_bedingt."""
    r = [G.GateResult("syntax_a", G.PASS, "")]
    assert CA._queue_status(r, {"geltungsbedingungen": [{"bedingung": "x"}]}, []) == "verified_bedingt"


def test_queue_status_verified():
    """Alles gruen, keine Bedingungen, keine discoveries -> verified."""
    r = [G.GateResult("syntax_a", G.PASS, "")]
    assert CA._queue_status(r, {}, []) == "verified"


def test_queue_status_first_wird_gefiltert():
    """_first-Gates werden aus der Entscheidung rausgefiltert (nur die reparierten zaehlen)."""
    r = [G.GateResult("syntax_a_first", G.FAIL, ""),
         G.GateResult("syntax_a", G.PASS, "")]
    assert CA._queue_status(r, {}, []) == "verified"


def test_queue_status_fail_schlaegt_discovery():
    """FAIL hat Vorrang vor discovery (fail-closed)."""
    r = [G.GateResult("syntax_a", G.FAIL, "")]
    assert CA._queue_status(r, {}, [{"id": "new"}]) == "flagged_for_review"


def test_queue_status_judge_skipped_kein_verified():
    """Falschgruen-Sperre: skip_judge darf NIE zu verified fuehren, auch wenn alles gruen."""
    r = [G.GateResult("syntax_a", G.PASS, "")]
    assert CA._queue_status(r, {}, [], judge_skipped=True) == "strukturgeprueft_judge_offen"
    assert CA._queue_status(r, {}, [], judge_skipped=True) != "verified"
    assert CA._queue_status(r, {"geltungsbedingungen": [{"bedingung": "x"}]}, [],
                            judge_skipped=True) == "strukturgeprueft_judge_offen"
    assert CA._queue_status(r, {"geltungsbedingungen": [{"bedingung": "x"}]}, [],
                            judge_skipped=True) != "verified_bedingt"