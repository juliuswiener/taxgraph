-- P4.1: SQLite schema for Sachverhalts-Store
-- Event-Log + content-adressierte Snapshots + Audit

CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO meta (key, value) VALUES ('version', '1')
    ON CONFLICT(key) DO NOTHING;

CREATE TABLE IF NOT EXISTS users (
    user_id    TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    role       TEXT NOT NULL DEFAULT 'nutzer',
    created_ts TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE IF NOT EXISTS faelle (
    fall_id                TEXT PRIMARY KEY,
    veranlagungszeitraum   INTEGER NOT NULL,
    user_id                TEXT REFERENCES users(user_id),
    status                 TEXT NOT NULL DEFAULT 'aktiv',
    created_ts             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_ts             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE IF NOT EXISTS events (
    event_id   TEXT PRIMARY KEY,
    ts         TEXT NOT NULL,
    feld_id    TEXT NOT NULL,
    wert       TEXT NOT NULL,       -- JSON-encoded
    zustand    TEXT NOT NULL CHECK (zustand IN ('vorlaeufig', 'bestaetigt')),
    herkunft   TEXT NOT NULL,       -- JSON-encoded {herkunft, pruef_tiefe, haftung}
    schreiber  TEXT NOT NULL,
    signal     TEXT NOT NULL,       -- JSON-encoded {signal_1, signal_2}
    ersetzt    TEXT REFERENCES events(event_id),
    version    INTEGER NOT NULL DEFAULT 1,
    created_ts TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_events_feld_id ON events(feld_id);
CREATE INDEX IF NOT EXISTS idx_events_ersetzt ON events(ersetzt);

CREATE TABLE IF NOT EXISTS snapshots (
    snapshot_id TEXT PRIMARY KEY,
    ts          TEXT NOT NULL,
    bis_event   TEXT NOT NULL REFERENCES events(event_id),
    felder      TEXT NOT NULL,       -- JSON-encoded dict
    eric_befund TEXT,                -- JSON-encoded, nullable
    created_ts  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE IF NOT EXISTS audit_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ts         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    fall_id    TEXT,
    event_id   TEXT,
    action     TEXT NOT NULL,
    detail     TEXT               -- JSON-encoded payload
);
