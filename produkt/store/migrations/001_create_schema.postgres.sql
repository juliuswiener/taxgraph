-- P4.1: PostgreSQL schema for Sachverhalts-Store
-- Event-Log + content-adressierte Snapshots + Audit
-- JSONB for flexible payloads, SERIAL for auto-increment

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
    created_ts TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS faelle (
    fall_id                TEXT PRIMARY KEY,
    veranlagungszeitraum   INTEGER NOT NULL,
    user_id                TEXT REFERENCES users(user_id),
    status                 TEXT NOT NULL DEFAULT 'aktiv',
    created_ts             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_ts             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS events (
    event_id   TEXT PRIMARY KEY,
    ts         TIMESTAMPTZ NOT NULL,
    feld_id    TEXT NOT NULL,
    wert       JSONB NOT NULL,
    zustand    TEXT NOT NULL CHECK (zustand IN ('vorlaeufig', 'bestaetigt')),
    herkunft   JSONB NOT NULL,       -- {herkunft, pruef_tiefe, haftung}
    schreiber  TEXT NOT NULL,
    signal     JSONB NOT NULL,       -- {signal_1, signal_2}
    ersetzt    TEXT REFERENCES events(event_id),
    version    INTEGER NOT NULL DEFAULT 1,
    created_ts TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_events_feld_id ON events(feld_id);
CREATE INDEX IF NOT EXISTS idx_events_ersetzt ON events(ersetzt);

CREATE TABLE IF NOT EXISTS snapshots (
    snapshot_id TEXT PRIMARY KEY,
    ts          TIMESTAMPTZ NOT NULL,
    bis_event   TEXT NOT NULL REFERENCES events(event_id),
    felder      JSONB NOT NULL,
    eric_befund JSONB,
    created_ts  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS audit_log (
    id         SERIAL PRIMARY KEY,
    ts         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    fall_id    TEXT,
    event_id   TEXT,
    action     TEXT NOT NULL,
    detail     JSONB
);
