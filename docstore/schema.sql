-- TaxGraph Dokumentstore (Roadmap M1.5) - Grundgeruest.
-- Kanonisches Dokumentmodell: Dokumente -> Segmente -> Claims.
-- Trennt Bindungswirkung (authority) und Exportierbarkeit (redistributable)
-- und verankert jeden Claim ueber ein woertliches Kurzzitat (Zitatanker) an
-- einem konkreten Segment. Fassungs-Hash (sha256) sichert die eingefrorene
-- Fassung. Idempotent: laesst sich mehrfach anwenden.

BEGIN;

DROP TABLE IF EXISTS claim CASCADE;
DROP TABLE IF EXISTS segment CASCADE;
DROP TABLE IF EXISTS dokument CASCADE;
DROP TYPE IF EXISTS authority_class;
DROP TYPE IF EXISTS dokument_typ;
DROP TYPE IF EXISTS segment_typ;
DROP TYPE IF EXISTS claim_typ;
DROP TYPE IF EXISTS claim_status;

-- Quellenklasse mit unterschiedlicher Bindungswirkung (Quellenmodell).
CREATE TYPE authority_class AS ENUM ('gesetz', 'verwaltung', 'bfh', 'fg', 'literatur');

CREATE TYPE dokument_typ AS ENUM
  ('gesetz', 'verordnung', 'urteil', 'bmf_schreiben', 'richtlinie', 'kommentar');

-- Typspezifische Segmentierung.
CREATE TYPE segment_typ AS ENUM
  ('absatz', 'satz', 'nummer', 'randziffer', 'tenor', 'leitsatz', 'gruende');

CREATE TYPE claim_typ AS ENUM
  ('parameter', 'ausnahme', 'definition', 'testfall', 'streitstand');

CREATE TYPE claim_status AS ENUM
  ('extracted', 'formalized', 'verified', 'approved', 'rejected');

-- Ein Dokument in einer konkreten Fassung.
CREATE TABLE dokument (
  id               bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  -- Kanonische ID: Norm-URI (Gesetz), ECLI (Urteil), BMF-Geschaeftszeichen (Schreiben).
  kanonische_id    text            NOT NULL,
  typ              dokument_typ    NOT NULL,
  authority        authority_class NOT NULL,
  redistributable  boolean         NOT NULL,
  titel            text,
  fassung          text,                        -- Fassungsbezeichnung / Stand
  quelle_url       text,
  abrufdatum       date,
  sha256           text            NOT NULL,     -- Fassungs-Hash des Originaltexts
  objektpfad       text,                         -- Pfad zur immutablen Originaldatei
  bstbl2           boolean,                      -- nur Urteile: im BStBl II veroeffentlicht
  erstellt_am      timestamptz     NOT NULL DEFAULT now(),
  UNIQUE (kanonische_id, fassung)
);

-- Segmente eines Dokuments (Satz, Randziffer, ...), reihenfolgestabil.
CREATE TABLE segment (
  id           bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  dokument_id  bigint      NOT NULL REFERENCES dokument(id) ON DELETE CASCADE,
  typ          segment_typ NOT NULL,
  label        text        NOT NULL,   -- z.B. "Abs. 1 Nr. 6c", "Abs. 1 Satz 6", "Rz. 12"
  position     int         NOT NULL,   -- Reihenfolge im Dokument
  text         text        NOT NULL,   -- Wortlaut des Segments (Normalisierungsbasis)
  UNIQUE (dokument_id, position)
);

-- Aus einem Segment extrahierter Claim (Parameter, Ausnahme, Definition, Testfall,
-- Streitstand). authority/redistributable werden vom Dokument geerbt und explizit
-- gehalten. Der Zitatanker ist ein woertliches Kurzzitat, das deterministisch gegen
-- segment.text verifiziert wird.
CREATE TABLE claim (
  id                 bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  segment_id         bigint          NOT NULL REFERENCES segment(id) ON DELETE RESTRICT,
  typ                claim_typ       NOT NULL,
  authority          authority_class NOT NULL,
  redistributable    boolean         NOT NULL,
  zitatanker         text            NOT NULL,
  anker_verifiziert  boolean         NOT NULL DEFAULT false,
  payload            jsonb           NOT NULL,   -- typspezifische Nutzlast
  status             claim_status    NOT NULL DEFAULT 'extracted',
  veranlagungszeitraum int,
  gueltig_ab         date,
  erstellt_am        timestamptz     NOT NULL DEFAULT now()
);

CREATE INDEX idx_segment_dokument ON segment (dokument_id);
CREATE INDEX idx_claim_segment    ON claim (segment_id);
CREATE INDEX idx_dokument_auth    ON dokument (authority);
CREATE INDEX idx_claim_status     ON claim (status);

-- Harte Regel des Quellenmodells: ein Claim darf nur dann exportierbar sein,
-- wenn sein Zitatanker verifiziert ist.
ALTER TABLE claim ADD CONSTRAINT claim_anker_vor_export
  CHECK (NOT redistributable OR anker_verifiziert);

COMMIT;
