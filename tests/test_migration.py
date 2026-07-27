"""Tests für json_to_sql migration — Idempotenz, Vollständigkeit."""

from __future__ import annotations

import json
import os
import tempfile

import pytest

from produkt.store.sql_backend import SQLStore
from produkt.store.migrations.json_to_sql import migrate_dict

TS = "2026-07-24T12:00:00+00:00"


def _sample_store_dict() -> dict:
    """Erzeugt einen vollständigen Store-Dict (wie aus store.py)."""
    import hashlib

    def _sha(text: str) -> str:
        return hashlib.sha256(text.encode("utf-8")).hexdigest()

    def _canonical_json(obj) -> str:
        return json.dumps(obj, sort_keys=True, ensure_ascii=False, separators=(",", ":"))

    e1 = {
        "event_id": "",
        "ts": TS,
        "feld_id": "ep_arbeitstage",
        "wert": 220,
        "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": None, "signal_2": "ok"},
        "ersetzt": None,
    }
    e1["event_id"] = _sha(_canonical_json({k: v for k, v in e1.items() if k != "event_id"}))

    e2 = {
        "event_id": "",
        "ts": TS,
        "feld_id": "agb_aufwendungen",
        "wert": 120000,
        "zustand": "vorlaeufig",
        "herkunft": {"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "llm:chat",
        "signal": {"signal_1": None, "signal_2": None},
        "ersetzt": None,
    }
    e2["event_id"] = _sha(_canonical_json({k: v for k, v in e2.items() if k != "event_id"}))

    felder = {
        "ep_arbeitstage": {"wert": 220, "zustand": "bestaetigt",
                           "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}},
    }
    sid = _sha(_canonical_json(felder))
    snap = {
        "snapshot_id": sid,
        "ts": TS,
        "bis_event": e1["event_id"],
        "felder": felder,
        "eric_befund": {"rc": 0, "klasse": "plausibel", "gekappt_verdacht": False, "fehler_anzahl": 0},
    }

    return {
        "version": 1,
        "veranlagungszeitraum": 2025,
        "fall_id": "fall-test",
        "events": [e1, e2],
        "snapshots": [snap],
    }


@pytest.fixture()
def sql_store():
    db = tempfile.NamedTemporaryFile(suffix=".sqlite", delete=False)
    db.close()
    s = SQLStore(db.name, fall_id="mig-test", veranlagungszeitraum=2025)
    yield s
    s.close()
    if os.path.exists(db.name):
        os.unlink(db.name)


# ---- Test: Vollständige Migration ------------------------------------------

def test_migrate_vollstaendig(sql_store):
    sd = _sample_store_dict()
    result = migrate_dict(sd, sql_store)
    assert result["events"] == 2
    assert result["snapshots"] == 1

    events = sql_store["events"]
    assert len(events) == 2
    event_ids = {e["feld_id"] for e in events}
    assert "ep_arbeitstage" in event_ids
    assert "agb_aufwendungen" in event_ids

    snapshots = sql_store["snapshots"]
    assert len(snapshots) == 1
    assert snapshots[0]["bis_event"] == sd["events"][0]["event_id"]


def test_migrate_meta(sql_store):
    sd = _sample_store_dict()
    migrate_dict(sd, sql_store)
    assert sql_store["version"] == 1
    assert sql_store["veranlagungszeitraum"] == 2025


def test_migrate_fall_id(sql_store):
    sd = _sample_store_dict()
    migrate_dict(sd, sql_store)
    assert sql_store["fall_id"] == "fall-test"


# ---- Test: Idempotenz ------------------------------------------------------

def test_migrate_idempotent(sql_store):
    sd = _sample_store_dict()
    r1 = migrate_dict(sd, sql_store)
    assert r1["events"] == 2

    r2 = migrate_dict(sd, sql_store)
    assert r2["events"] == 0  # alle bereits existent → INSERT OR IGNORE

    assert len(sql_store["events"]) == 2  # keine Duplikate


# ---- Test: Leerer Store ----------------------------------------------------

def test_migrate_leer(sql_store):
    result = migrate_dict({"version": 1, "veranlagungszeitraum": 2025,
                           "events": [], "snapshots": []}, sql_store)
    assert result["events"] == 0
    assert result["snapshots"] == 0
    assert len(sql_store["events"]) == 0


# ---- Test: Materialisierung nach Migration ---------------------------------

def test_migrate_dann_materialisieren(sql_store):
    sd = _sample_store_dict()
    migrate_dict(sd, sql_store)

    felder, sid = sql_store.materialisiere()
    assert "ep_arbeitstage" in felder
    assert felder["ep_arbeitstage"]["wert"] == 220

    # snapshot_id in der DB muss der Materialisierung bis zu diesem Event entsprechen
    snap = sql_store["snapshots"][0]
    felder_punkt, sid_punkt = sql_store.materialisiere(bis_event=snap["bis_event"])
    assert sid_punkt == snap["snapshot_id"]

    # Voll-Log-Materialisierung enthält mehr Felder (agb_aufwendungen)
    felder_full, _ = sql_store.materialisiere()
    assert len(felder_full) >= len(felder_punkt)
