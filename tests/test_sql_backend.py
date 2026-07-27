"""Tests for SQLStore + FileStore — parametrisiert über beide Backends."""

from __future__ import annotations

import json
import os
import tempfile

import pytest

from produkt.store import create_store, ConcurrencyError
from produkt.store.sql_backend import SQLStore
from produkt.store.file_backend import FileStore

TS = "2026-07-24T12:00:00+00:00"


# ---- Fixtures ---------------------------------------------------------------

@pytest.fixture(params=["sqlite", "file"])
def store(request):
    """Parametrisiert: testet beide Backends."""
    if request.param == "sqlite":
        db = tempfile.NamedTemporaryFile(suffix=".sqlite", delete=False)
        db.close()
        s = SQLStore(db.name, fall_id="test-fall", veranlagungszeitraum=2025)
        # Init-Schema läuft automatisch
        yield s
        s.close()
        os.unlink(db.name)
    else:
        path = tempfile.NamedTemporaryFile(suffix=".yaml", delete=False)
        path.close()
        init = {
            "version": 1,
            "veranlagungszeitraum": 2025,
            "fall_id": "test-fall",
            "events": [],
            "snapshots": [],
        }
        s = FileStore(path.name, store_dict=init)
        yield s
        s.save()
        os.unlink(path.name)


@pytest.fixture()
def store_with_event(store):
    """Store mit einem Event + Snapshot."""
    ev = store.append_event(
        feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"},
        ts=TS,
    )
    return store, ev


# ---- (1) Dict-API ----------------------------------------------------------

def test_dict_getitem_version(store):
    assert store["version"] == 1


def test_dict_getitem_veranlagungszeitraum(store):
    assert store["veranlagungszeitraum"] == 2025


def test_dict_getitem_fall_id(store):
    assert store["fall_id"] == "test-fall"


def test_dict_getitem_events(store):
    assert store["events"] == []


def test_dict_getitem_snapshots(store):
    assert store["snapshots"] == []


def test_dict_key_error(store):
    with pytest.raises(KeyError):
        _ = store["nonexistent"]


def test_dict_contains(store):
    assert "version" in store
    assert "events" in store
    assert "snapshots" in store
    assert "nonexistent" not in store


def test_dict_setitem(store):
    store["version"] = 2
    assert store["version"] == 2


def test_dict_setitem_events(store_with_event):
    store, ev = store_with_event
    assert len(store["events"]) == 1
    assert store["events"][0]["feld_id"] == "ep_arbeitstage"


def test_dict_setitem_key_error(store):
    with pytest.raises(KeyError):
        store["invalid_key"] = "x"


def test_dict_len(store):
    assert len(store) >= 4  # mindestens die 4 Kernschlüssel


# ---- (2) append_event ------------------------------------------------------

def test_append_event_basic(store):
    ev = store.append_event(
        feld_id="ep_entfernung_km", wert=30, zustand="vorlaeufig",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": None},
        ts=TS,
    )
    assert ev["feld_id"] == "ep_entfernung_km"
    assert ev["zustand"] == "vorlaeufig"
    assert len(ev["event_id"]) == 64  # SHA256


def test_append_event_bestaetigt_braucht_signal2(store):
    with pytest.raises(ValueError):
        store.append_event(
            feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
            herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            schreiber="ui:laie", signal={"signal_1": None, "signal_2": None},
            ts=TS,
        )


def test_append_event_ersetzt(store):
    e1 = store.append_event(
        feld_id="ep_arbeitstage", wert=200, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "first"},
        ts=TS,
    )
    e2 = store.append_event(
        feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "second"},
        ersetzt=e1["event_id"], ts=TS,
    )
    assert e2["ersetzt"] == e1["event_id"]


# ---- (3) materialisiere ----------------------------------------------------

def test_materialisiere_empty(store):
    felder, sid = store.materialisiere()
    assert felder == {}
    assert len(sid) == 64


def test_materialisiere_with_events(store_with_event):
    store, ev = store_with_event
    felder, sid = store.materialisiere()
    assert "ep_arbeitstage" in felder
    assert felder["ep_arbeitstage"]["wert"] == 220


def test_materialisiere_ersetzt_aufloesung(store):
    e1 = store.append_event(
        feld_id="ep_arbeitstage", wert=200, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "x"},
        ts=TS,
    )
    store.append_event(
        feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "y"},
        ersetzt=e1["event_id"], ts=TS,
    )
    felder, _ = store.materialisiere()
    assert felder["ep_arbeitstage"]["wert"] == 220


# ---- (4) erzeuge_snapshot --------------------------------------------------

def test_erzeuge_snapshot(store_with_event):
    store, ev = store_with_event
    snap = store.erzeuge_snapshot(
        ts=TS, eric_befund={"rc": 0, "klasse": "plausibel",
                            "gekappt_verdacht": False, "fehler_anzahl": 0},
    )
    assert snap["snapshot_id"] == snap.get("snapshot_id")
    assert len(snap["snapshot_id"]) == 64


def test_erzeuge_snapshot_materialisierung(store_with_event):
    store, ev = store_with_event
    snap = store.erzeuge_snapshot(ts=TS)
    felder, _ = store.materialisiere()
    assert snap["felder"] == felder


# ---- (5) Concurrent-Safety (Optimistic Lock) -------------------------------

def test_no_concurrency_error_on_new_event(store):
    """Neuanlage ohne expected_version löst keinen ConcurrencyError aus."""
    ev = store.append_event(
        feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"},
        ts=TS,
    )
    assert ev["event_id"] is not None


def test_concurrency_error_on_version_mismatch(store):
    """expected_version != DB-Version → ConcurrencyError."""
    if not isinstance(store, SQLStore):
        pytest.skip("Nur für SQL-Backend")
    e1 = store.append_event(
        feld_id="ep_arbeitstage", wert=200, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "x"},
        ts=TS, expected_version=1,
    )
    with pytest.raises(ConcurrencyError):
        store.append_event(
            feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
            herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            schreiber="ui:laie", signal={"signal_1": None, "signal_2": "y"},
            ersetzt=e1["event_id"], ts=TS,
            expected_version=999,  # falsche Version
        )


# ---- (6) Transaktionen (Event-Schreiben in Transaktion) --------------------

def test_event_persistenz_zwischen_sessions(store):
    """Event bleibt nach close/reopen erhalten (nur SQL)."""
    if not isinstance(store, SQLStore):
        pytest.skip("Nur für SQL-Backend")
    db_path = store._db_path
    store.append_event(
        feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"},
        ts=TS,
    )
    store.close()

    store2 = SQLStore(db_path, fall_id="test-fall", veranlagungszeitraum=2025)
    assert len(store2["events"]) == 1
    assert store2["events"][0]["feld_id"] == "ep_arbeitstage"
    store2.close()


# ---- FileStore-spezifisch --------------------------------------------------

def test_filestore_save_reload():
    """FileStore: save + open ergibt gleichen Inhalt."""
    path = tempfile.NamedTemporaryFile(suffix=".yaml", delete=False)
    path.close()
    try:
        init = {"version": 1, "veranlagungszeitraum": 2025, "fall_id": "f",
                "events": [], "snapshots": []}
        fs = FileStore(path.name, store_dict=init)
        fs.append_event(
            feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
            herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"},
            ts=TS,
        )
        fs.save()

        fs2 = FileStore.open(path.name)
        assert len(fs2["events"]) == 1
        assert fs2["events"][0]["feld_id"] == "ep_arbeitstage"
    finally:
        if os.path.exists(path.name):
            os.unlink(path.name)
