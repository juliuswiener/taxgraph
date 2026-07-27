"""SQL-backed store — dict-like API, parametrisierte Queries, versionierter Optimistic Lock."""

from __future__ import annotations

import json
import os
import sqlite3
from collections.abc import MutableMapping
from datetime import datetime, timezone
from typing import Any


class ConcurrencyError(Exception):
    """Optimistic-Lock-Konflikt: Event wurde parallel geändert."""


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


# ------------------------------------------------------------------ SQLite-Adapter (dict↔JSON)

def _json(val: Any) -> str:
    return json.dumps(val, ensure_ascii=False)


def _from_json(val: str) -> Any:
    return json.loads(val) if val else None


def _row_event(row: sqlite3.Row) -> dict:
    return {
        "event_id": row["event_id"],
        "ts": row["ts"],
        "feld_id": row["feld_id"],
        "wert": _from_json(row["wert"]),
        "zustand": row["zustand"],
        "herkunft": _from_json(row["herkunft"]),
        "schreiber": row["schreiber"],
        "signal": _from_json(row["signal"]),
        "ersetzt": row["ersetzt"],
    }


def _row_snapshot(row: sqlite3.Row) -> dict:
    snap = {
        "snapshot_id": row["snapshot_id"],
        "ts": row["ts"],
        "bis_event": row["bis_event"],
        "felder": _from_json(row["felder"]),
    }
    if row["eric_befund"]:
        snap["eric_befund"] = _from_json(row["eric_befund"])
    return snap


# ------------------------------------------------------------------ Der SQL-Store

class SQLStore(MutableMapping):
    """Dict-artiges Store-Interface backed by SQLite/PostgreSQL.

    Schlüssel (lesbar/schreibbar):
      version, veranlagungszeitraum, fall_id, events, snapshots

    Store-Methoden (append_event, materialisiere, erzeuge_snapshot)
    delegieren an store.py-Funktionen oder arbeiten direkt auf SQL.
    """

    def __init__(self, db_path: str, fall_id: str | None = None,
                 veranlagungszeitraum: int | None = None):
        self._db_path = db_path
        self._conn: sqlite3.Connection | None = None
        self._fall_id = None
        self._vz = None
        if veranlagungszeitraum is not None:
            self["veranlagungszeitraum"] = veranlagungszeitraum
        if fall_id is not None:
            self["fall_id"] = fall_id
        # In-Memory-Cache für events/snapshots (reduziert DB-Zugriffe)
        self._events_cache: list[dict] | None = None
        self._snapshots_cache: list[dict] | None = None

    # ---- Verbindung / Schema ------------------------------------------------

    @property
    def conn(self) -> sqlite3.Connection:
        if self._conn is None:
            self._conn = sqlite3.connect(self._db_path)
            self._conn.row_factory = sqlite3.Row
            self._conn.execute("PRAGMA journal_mode=WAL")
            self._conn.execute("PRAGMA foreign_keys=ON")
            self._init_schema()
        return self._conn

    def _init_schema(self) -> None:
        schema = os.path.join(
            os.path.dirname(__file__), "migrations", "001_create_schema.sqlite.sql"
        )
        with open(schema) as f:
            self.conn.executescript(f.read())

    def close(self) -> None:
        if self._conn is not None:
            self._conn.close()
            self._conn = None
        self._events_cache = None
        self._snapshots_cache = None

    # ---- Meta-Schlüssel (version, veranlagungszeitraum, fall_id) ------------

    def _get_meta(self, key: str) -> Any:
        cur = self.conn.execute("SELECT value FROM meta WHERE key = ?", (key,))
        row = cur.fetchone()
        if row is None:
            raise KeyError(key)
        return json.loads(row["value"])

    def _set_meta(self, key: str, value: Any) -> None:
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?) "
            "ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            (key, json.dumps(value)),
        )

    # ---- Events -------------------------------------------------------------

    def _load_events(self) -> list[dict]:
        cur = self.conn.execute(
            "SELECT * FROM events ORDER BY rowid"
        )
        return [_row_event(r) for r in cur.fetchall()]

    def _load_snapshots(self) -> list[dict]:
        cur = self.conn.execute("SELECT * FROM snapshots ORDER BY rowid")
        return [_row_snapshot(r) for r in cur.fetchall()]

    # ---- MutableMapping (dict-like API) -------------------------------------

    _DICT_KEYS = frozenset({"version", "veranlagungszeitraum", "fall_id",
                            "events", "snapshots"})

    def __getitem__(self, key: str) -> Any:
        if key == "version":
            return int(self._get_meta("version"))
        if key == "veranlagungszeitraum":
            return self._vz or int(self._get_meta("veranlagungszeitraum"))
        if key == "fall_id":
            return self._fall_id or self._get_meta("fall_id")
        if key == "events":
            if self._events_cache is None:
                self._events_cache = self._load_events()
            return self._events_cache
        if key == "snapshots":
            if self._snapshots_cache is None:
                self._snapshots_cache = self._load_snapshots()
            return self._snapshots_cache
        raise KeyError(key)

    def __setitem__(self, key: str, value: Any) -> None:
        if key == "version":
            self._set_meta("version", value)
        elif key == "veranlagungszeitraum":
            self._vz = int(value)
            self._set_meta("veranlagungszeitraum", int(value))
        elif key == "fall_id":
            self._fall_id = str(value)
            self._set_meta("fall_id", str(value))
        elif key == "events":
            self._events_cache = list(value)
            self._replace_events(value)
        elif key == "snapshots":
            self._snapshots_cache = list(value)
            self._replace_snapshots(value)
        else:
            raise KeyError(key)

    def __delitem__(self, key: str) -> None:
        raise NotImplementedError("SQLStore unterstützt kein Löschen von Schlüsseln.")

    def __iter__(self):
        return iter(self._DICT_KEYS)

    def __len__(self) -> int:
        return len(self._DICT_KEYS)

    def __contains__(self, key: object) -> bool:
        return key in self._DICT_KEYS

    # ---- Bulk-Replace für Events/Snapshots (wird von store.py-Funktionen verwendet) ---

    def _replace_events(self, events: list[dict]) -> None:
        self.conn.execute("DELETE FROM events")
        for ev in events:
            self._insert_event_row(ev)
        self.conn.commit()

    def _replace_snapshots(self, snapshots: list[dict]) -> None:
        self.conn.execute("DELETE FROM snapshots")
        for snap in snapshots:
            self._insert_snapshot_row(snap)
        self.conn.commit()

    def _insert_event_row(self, ev: dict) -> None:
        self.conn.execute(
            """INSERT INTO events (event_id, ts, feld_id, wert, zustand,
                                   herkunft, schreiber, signal, ersetzt, version)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
            (ev["event_id"], ev.get("ts", _now()), ev["feld_id"],
             _json(ev["wert"]), ev["zustand"],
             _json(ev["herkunft"]), ev["schreiber"],
             _json(ev.get("signal", {"signal_1": None, "signal_2": None})),
             ev.get("ersetzt"), ev.get("version", 1)),
        )

    def _insert_snapshot_row(self, snap: dict) -> None:
        self.conn.execute(
            """INSERT INTO snapshots (snapshot_id, ts, bis_event, felder, eric_befund)
               VALUES (?, ?, ?, ?, ?)""",
            (snap["snapshot_id"], snap["ts"], snap["bis_event"],
             _json(snap["felder"]),
             _json(snap.get("eric_befund")) if snap.get("eric_befund") else None),
        )

    # ---- P4.4: Optimistic Locking + Append-Event ----------------------------

    def append_event(self, *, feld_id: str, wert, zustand: str, herkunft: dict,
                     schreiber: str, signal: dict | None = None,
                     ersetzt: str | None = None,
                     ts: str | None = None,
                     katalog: dict | None = None,
                     expected_version: int | None = None) -> dict:
        """Event an SQL-Store anhängen (mit optimistic-lock).

        expected_version: Erwartete Version des betroffenen Events (bei ersetzt).
        None = kein Lock-Check (Neuanlage).
        """
        # Validierung via store.py delegieren
        import produkt.store.store as ST
        # Wir brauchen ein dict-Proxy, damit store.py-Funktionen darauf schreiben können
        event = ST.append_event(
            self, feld_id=feld_id, wert=wert, zustand=zustand,
            herkunft=herkunft, schreiber=schreiber, signal=signal,
            ersetzt=ersetzt, ts=ts, katalog=katalog,
        )

        # Optimistic Lock: Version prüfen beim Überschreiben
        if ersetzt and expected_version is not None:
            cur = self.conn.execute(
                "SELECT version FROM events WHERE event_id = ?", (ersetzt,)
            )
            row = cur.fetchone()
            if row is None or row["version"] != expected_version:
                raise ConcurrencyError(
                    f"Optimistic-Lock-Konflikt: Event {ersetzt} hat Version "
                    f"{row['version'] if row else '—'}, erwartet {expected_version}."
                )

        # Persistieren
        with self.conn:
            self._insert_event_row(event)
            self._events_cache = None  # invalidieren
            # Version hochzählen beim ersetzt-Ziel
            if ersetzt:
                self.conn.execute(
                    "UPDATE events SET version = version + 1 WHERE event_id = ?",
                    (ersetzt,),
                )

        return event

    def materialisiere(self, bis_event: str | None = None) -> tuple[dict, str]:
        import produkt.store.store as ST
        return ST.materialisiere(self, bis_event)

    def erzeuge_snapshot(self, *, bis_event: str | None = None,
                         ts: str | None = None,
                         eric_befund: dict | None = None) -> dict:
        import produkt.store.store as ST
        snap = ST.erzeuge_snapshot(self, bis_event=bis_event, ts=ts,
                                   eric_befund=eric_befund)
        with self.conn:
            self._insert_snapshot_row(snap)
            self._snapshots_cache = None
        return snap

    def lade_katalog(self, bindung: dict) -> dict:
        import produkt.store.store as ST
        return ST.lade_katalog(bindung)
