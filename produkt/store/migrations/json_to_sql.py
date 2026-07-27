"""Migration: dict-basierter Store → SQLStore.

migrate_dict(store_dict, sql_store) kopiert alle Events und Snapshots
idempotent via INSERT OR IGNORE.
"""

from __future__ import annotations

import json
from typing import Any

from produkt.store.sql_backend import SQLStore


def migrate_dict(store_dict: dict, sql_store: SQLStore) -> dict[str, int]:
    """Migriert store_dict nach sql_store. Idempotent (INSERT OR IGNORE).

    Returns: {"events": N, "snapshots": M} — Anzahl migrierter Zeilen.
    """
    # Meta setzen
    for key in ("version", "veranlagungszeitraum", "fall_id"):
        if key in store_dict:
            sql_store[key] = store_dict[key]

    events = store_dict.get("events", [])
    snapshots = store_dict.get("snapshots", [])

    conn = sql_store.conn
    count_events = 0
    for ev in events:
        with conn:
            cur = conn.execute(
                """INSERT OR IGNORE INTO events
                   (event_id, ts, feld_id, wert, zustand, herkunft,
                    schreiber, signal, ersetzt, version)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    ev["event_id"],
                    ev.get("ts", ""),
                    ev["feld_id"],
                    json.dumps(ev["wert"], ensure_ascii=False),
                    ev["zustand"],
                    json.dumps(ev["herkunft"], ensure_ascii=False),
                    ev["schreiber"],
                    json.dumps(
                        ev.get("signal", {"signal_1": None, "signal_2": None}),
                        ensure_ascii=False,
                    ),
                    ev.get("ersetzt"),
                    ev.get("version", 1),
                ),
            )
            if cur.rowcount > 0:
                count_events += 1

    count_snapshots = 0
    for snap in snapshots:
        with conn:
            cur = conn.execute(
                """INSERT OR IGNORE INTO snapshots
                   (snapshot_id, ts, bis_event, felder, eric_befund)
                   VALUES (?, ?, ?, ?, ?)""",
                (
                    snap["snapshot_id"],
                    snap["ts"],
                    snap["bis_event"],
                    json.dumps(snap["felder"], ensure_ascii=False),
                    json.dumps(snap["eric_befund"], ensure_ascii=False)
                    if snap.get("eric_befund") else None,
                ),
            )
            if cur.rowcount > 0:
                count_snapshots += 1

    return {"events": count_events, "snapshots": count_snapshots}
