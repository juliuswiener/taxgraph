# docstore/ - Dokumentstore-Grundgeruest (M1.5)

Kanonisches Dokumentmodell fuer Rechtsquellen: Dokumente -> Segmente -> Claims,
in Postgres. Vorstufe des vollen Ingestion-Systems (Phasen 2+). Schlank gehalten:
Schema plus Ingest der eingefrorenen § 32a-/§ 4-Fassungen aus `sources/`.

## Modell

- **dokument**: eine Fassung einer Rechtsquelle. Kanonische ID (Norm-URI /
  ECLI / BMF-Geschaeftszeichen), `authority` (Quellenklasse), `redistributable`,
  `sha256` (Fassungs-Hash), Objektpfad, bei Urteilen `bstbl2`.
- **segment**: reihenfolgestabile Segmente (Satz, Nummer, Randziffer, ...).
- **claim**: aus einem Segment extrahierter Claim (parameter/ausnahme/definition/
  testfall/streitstand) mit `zitatanker` und `anker_verifiziert`. Eine
  CHECK-Constraint erzwingt: `redistributable` nur bei verifiziertem Zitatanker.

Schema: `schema.sql`.

## Start (Docker, Muster fuer nordserver, lokal lauffaehig)

```bash
make docstore-up        # startet Postgres via docker compose (Schema auto-init)
make docstore-ingest    # ingestet sources/ (Dokumente, Segmente, § 32a-Claims)
make docstore-down      # stoppt den Container
```

Zugangsdaten (docker-compose.yml): DB/User/Passwort je `taxgraph`, Port 5432.
Ueberschreibbar per `DOCSTORE_DSN`.

## Start (ohne Docker, lokaler Postgres-Cluster)

Wo kein Docker-Daemon laeuft, gegen einen lokalen Cluster:

```bash
createdb taxgraph_docstore
psql -d taxgraph_docstore -f docstore/schema.sql
DOCSTORE_DSN="host=127.0.0.1 dbname=taxgraph_docstore user=<u> password=<p>" \
  python docstore/ingest.py
```

## Ingest

`ingest.py` prueft je Dokument zuerst die Freeze-Integritaet (Datei-SHA256 gegen
den in `sources/**/*.meta.yaml` hinterlegten Hash), segmentiert den Text
(§ 32a: Absatz/Nummer/Satz; § 4: Nummern 6b/6c) und legt fuer die § 32a-2026-
Fassung fuenf Parameter-Claims an, deren Zitatanker deterministisch gegen den
Segmenttext verifiziert wird. Der Lauf ist idempotent.

Erwartetes Ergebnis v1: 2 Dokumente, 10 Segmente, 5 Claims (alle mit
verifiziertem Zitatanker).

## Abhaengigkeiten

Python: `psycopg[binary]`, `PyYAML` (siehe `docstore/requirements.txt`).

## Naechste Schritte

- Segmentierung auf echte Satz-Ebene verfeinern; Randziffern fuer BMF/Urteile.
- Ingest weiterer Fassungen (VZ 2024/2025) sobald deren Volltext eingefroren ist.
- Anbindung an die Formalisierungspipeline (Claims -> Review-Queue), Phase 2.
