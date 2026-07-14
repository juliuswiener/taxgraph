"""Tests der Verified-Snapshot-Mechanik (runs/-Blocker-Fix, 2026-07-14).

Der Snapshot committet die deterministisch pruefbaren Teile eines verifizierten
Reports, sodass ein frischer Checkout ohne runs/ das Verdikt regaten kann. Diese
Tests halten die drei Instructor-Praezisierungen fest:
  1. Snapshot traegt sha256(catala_a) als Waechter.
  2. Live-Report schlaegt Snapshot (mit Warnung); Snapshot kanonisch nur ohne Live.
  3. Korrumpierter Snapshot FAILt hart - nie stiller PASS (Negativtest).

Kein Netz, kein Modell, kein clerk (die End-to-end-Rekonstruktion mit Toolchain
prueft `make regate-fresh`). Rein die Mechanik.
"""

from __future__ import annotations

import json
import os
import sys

import pytest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

import snapshot as SNAP   # noqa: E402


def _report(rid="t", status="verified_bedingt", catala_a="```catala\nx\n```",
            catala_b="```catala\ny\n```"):
    return {
        "candidate_id": rid,
        "queue_status": status,
        "module_name": "Mod",
        "catala_a": catala_a,
        "catala_b": catala_b,
        "judge_verdict": {"abweichungen": [], "stille_zusatzannahmen": []},
        "gates": [{"name": "clerk", "status": "PASS", "detail": "ok"}],
        "bedingungen": ["b1", "b2", "b3"],
        # Felder, die NICHT in den Snapshot gehoeren (Kosten/Provenance):
        "total_cost_usd": 0.44, "provenance": [{"x": 1}],
    }


@pytest.fixture()
def dirs(tmp_path, monkeypatch):
    snaps = tmp_path / "snaps"
    runs = tmp_path / "runs"
    monkeypatch.setattr(SNAP, "SNAP_DIR", str(snaps))
    monkeypatch.setattr(SNAP, "OUT_ROOT", str(runs))
    return snaps, runs


def _live(runs, rep):
    d = os.path.join(str(runs), rep["candidate_id"])
    os.makedirs(d, exist_ok=True)
    json.dump(rep, open(os.path.join(d, "report.json"), "w", encoding="utf-8"))


# -- 1. Round-trip + Feld-Treue ----------------------------------------------

def test_build_snapshot_nur_deterministische_felder(dirs):
    snap = SNAP.build_snapshot(_report())
    assert set(snap) == set(SNAP.SNAP_FIELDS) | {
        "schema_version", "catala_a_sha256", "catala_b_sha256"}
    # Kosten/Provenance sind bewusst NICHT drin (reproduzierbar, kein Auditlog).
    assert "total_cost_usd" not in snap and "provenance" not in snap
    assert len(snap["catala_a_sha256"]) == 64
    assert len(snap["catala_b_sha256"]) == 64


def test_write_und_load_round_trip(dirs):
    rep = _report(rid="rt")
    SNAP.write_snapshot("rt", rep)
    got = SNAP.load_snapshot("rt")
    assert got["catala_a"] == rep["catala_a"]
    assert got["catala_b"] == rep["catala_b"]
    assert got["judge_verdict"] == rep["judge_verdict"]
    assert got["queue_status"] == "verified_bedingt"
    assert got["aus_snapshot"] is True


# -- 2. Nur verified* wird gesnapshottet -------------------------------------

def test_flagged_wird_nicht_gesnapshottet(dirs):
    snaps, _ = dirs
    _, st = SNAP.write_snapshot("f", _report(rid="f", status="flagged_for_review"))
    assert st.startswith("uebersprungen")
    assert not os.path.exists(SNAP.snapshot_path("f"))


def test_kein_catala_a_nicht_snapshotwuerdig(dirs):
    assert not SNAP.is_snapshotwuerdig(_report(catala_a=""))
    assert SNAP.is_snapshotwuerdig(_report())


# -- 3. Integritaet: korrumpierter Snapshot FAILt hart (Negativtest) ---------

def test_korrupter_catala_a_failt_hart(dirs):
    SNAP.write_snapshot("k", _report(rid="k"))
    sp = SNAP.snapshot_path("k")
    snap = json.load(open(sp, encoding="utf-8"))
    # catala_a heimlich aendern, Hash NICHT nachziehen -> Manipulation.
    snap["catala_a"] = "```catala\nBOESE\n```"
    json.dump(snap, open(sp, "w", encoding="utf-8"))
    with pytest.raises(SNAP.SnapshotIntegrityError):
        SNAP.load_snapshot("k")


def test_korrupter_catala_b_failt_hart(dirs):
    """catala_b ist ebenfalls gehasht (pfad-unabhaengige Integritaet, nicht nur
    transitiv ueber equivalence). b-Tamper -> hart FAIL."""
    SNAP.write_snapshot("kb", _report(rid="kb"))
    sp = SNAP.snapshot_path("kb")
    snap = json.load(open(sp, encoding="utf-8"))
    snap["catala_b"] = "```catala\nBOESE_B\n```"
    json.dump(snap, open(sp, "w", encoding="utf-8"))
    with pytest.raises(SNAP.SnapshotIntegrityError):
        SNAP.load_snapshot("kb")


def test_korrupter_hash_failt_hart(dirs):
    SNAP.write_snapshot("h", _report(rid="h"))
    sp = SNAP.snapshot_path("h")
    snap = json.load(open(sp, encoding="utf-8"))
    snap["catala_a_sha256"] = "0" * 64
    json.dump(snap, open(sp, "w", encoding="utf-8"))
    with pytest.raises(SNAP.SnapshotIntegrityError):
        SNAP.load_snapshot("h")


def test_resolve_propagiert_integritaetsfehler(dirs):
    """resolve_report faellt NICHT still auf None zurueck, wenn der Snapshot
    korrupt ist - es propagiert den harten Fehler (kein stiller PASS)."""
    SNAP.write_snapshot("p", _report(rid="p"))
    sp = SNAP.snapshot_path("p")
    snap = json.load(open(sp, encoding="utf-8"))
    snap["catala_a"] = "geaendert"
    json.dump(snap, open(sp, "w", encoding="utf-8"))
    with pytest.raises(SNAP.SnapshotIntegrityError):
        SNAP.resolve_report("p")


# -- 4. Vorrangregel: live schlaegt Snapshot (mit Warnung) -------------------

def test_live_schlaegt_snapshot_mit_warnung(dirs):
    snaps, runs = dirs
    SNAP.write_snapshot("v", _report(rid="v"))
    _live(runs, _report(rid="v", catala_a="```catala\nLIVE\n```"))
    rep, quelle, _, warn = SNAP.resolve_report("v")
    assert quelle == "live"
    assert rep["catala_a"] == "```catala\nLIVE\n```"
    assert warn  # Warnung, weil beide existieren


def test_nur_snapshot_ist_kanonisch(dirs):
    SNAP.write_snapshot("s", _report(rid="s"))
    rep, quelle, _, warn = SNAP.resolve_report("s")
    assert quelle == "snapshot"
    assert warn == ""
    assert rep["aus_snapshot"] is True


def test_weder_live_noch_snapshot(dirs):
    rep, quelle, _, _ = SNAP.resolve_report("nix")
    assert rep is None and quelle == "none"


def test_nur_live_ohne_snapshot_keine_warnung(dirs):
    snaps, runs = dirs
    _live(runs, _report(rid="L"))
    rep, quelle, _, warn = SNAP.resolve_report("L")
    assert quelle == "live" and warn == ""
