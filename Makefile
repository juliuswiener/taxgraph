# TaxGraph v3 - Phase 0 build and test entrypoints.
#
# Toolchain (see docs/setup.md):
#   - Catala/Clerk via opam switch "taxgraph"
#   - GETTSIM + Catala python runtime in oracle/.venv312 (Python 3.12)
#
# The opam environment is loaded per target so no shell pre-setup is needed.

OPAM_ENV := eval $$(opam env --switch=taxgraph --set-switch)
VENV312  := oracle/.venv312/bin/activate

.PHONY: all s01 s03 tests build-python s02 clean backup restore

all: unit tests s02

## Run all Catala/Clerk scope tests (S0.1 tariff, S0.3 Arbeitszimmer/Homeoffice).
tests:
	$(OPAM_ENV); clerk test -W rules/

## Regressionstests der Gate-Semantik (kein Catala, kein Netz, <1s).
unit:
	python3 -m pytest tests/ -q

## Sicherung Fall-Store + Audit-Log + Benutzerkonten (Audit 2026-08-16/17, data-no-backup-restore).
## 313 Falldateien sind gitignored — ohne dieses Ziel gibt es KEINE Recovery. audit.jsonl liegt
## BEREITS unter faelle/ (produkt/store/audit.py:AUDIT_DIR) und wird von der faelle/-Sicherung
## automatisch mit erfasst — der alte "audit.jsonl"-Operand traf nie (er lag relativ zu
## FAELLE_ROOT statt zu FAELLE_ROOT/faelle, Review 2026-08-17). users.json (produkt/auth/) MUSS
## mit ins Archiv — ohne sie zeigen restaurierte Faelle nach einem Umzug auf Konten, die es dort
## nicht gibt, und die fail-closed Zugriffspruefung (_fall_owner_check) sperrt sie dauerhaft.
## FAELLE_ROOT/AUTH_USERS/BACKUP_DIR sind per Kommandozeile overridebar (Pflicht fuer den
## Round-Trip-Test in tests/test_backup_restore_roundtrip.py, der NIE die echten Pfade anfasst).
## BACKUP_DIR liegt NEBEN dem Checkout, nicht unter $(HOME) (Nutzer-Vorgabe: nichts im Home anlegen;
## dort liegen die bisherigen Sicherungen ohnehin schon).
BACKUP_DIR  ?= $(abspath $(CURDIR)/../taxgraph-backups)
FAELLE_ROOT ?= produkt/haut
AUTH_USERS  ?= produkt/auth/users.json

backup:
	mkdir -p $(BACKUP_DIR)
	tar czf $(abspath $(BACKUP_DIR))/taxgraph-$$(date +%Y%m%d-%H%M%S-%N)-$$$$.tar.gz \
		-C $(FAELLE_ROOT) faelle \
		-C $(dir $(AUTH_USERS)) $(notdir $(AUTH_USERS))

## Wiederherstellung: make restore ARCHIV=<pfad.tar.gz> [FAELLE_ROOT=...] [AUTH_USERS=...] [CONFIRM=yes]
## ERSETZT (kein Merge, Review-Punkt 3): faelle/ wird vor dem Entpacken komplett geloescht — sonst
## ueberlebt eine Datei, die im Ziel liegt und im Archiv fehlt, den "Restore". Legt VOR jedem
## Ueberschreiben automatisch eine Sicherheits-Sicherung des aktuellen Standes an (Punkt 4, zweite
## Haelfte) und fragt interaktiv nach, bevor sie ausgefuehrt wird (Punkt 4, erste Haelfte) — die
## Rueckfrage zeigt den AUFGELOESTEN Zielpfad, faengt damit auch einen Tippfehler im Variablen-
## namen selbst (z.B. FAELE_ROOT= zeigt still auf den Default, und genau der steht dann sichtbar
## in der Frage). CONFIRM=yes ueberspringt die Rueckfrage fuer Skripte/den Round-Trip-Test.
##
## Die Vorher-Sicherung unterscheidet "nichts da" von "ging schief": existiert faelle/ nicht,
## wird sie mit Hinweis UEBERSPRUNGEN — sonst waere ausgerechnet die Wiederherstellung nach
## Datenverlust blockiert (gemessen 2026-08-17: `backup` bricht mit `tar: faelle: Cannot stat`
## ab und riss `restore` mit, das Ziel blieb leer). Existiert faelle/ und die Sicherung
## SCHEITERT, bricht restore weiterhin ab, bevor irgendetwas geloescht wird — ein pauschales
## `-`/`|| true` haette genau diesen Schutz mit weggenommen.
restore:
	@test -n "$(ARCHIV)" || { echo "Aufruf: make restore ARCHIV=<pfad.tar.gz> [FAELLE_ROOT=...] [AUTH_USERS=...] [CONFIRM=yes]"; exit 1; }
	@test -f "$(ARCHIV)" || { echo "Archiv nicht gefunden: $(ARCHIV)"; exit 1; }
	@if [ "$(CONFIRM)" != "yes" ]; then \
		echo "Restore ERSETZT VOLLSTAENDIG $(FAELLE_ROOT)/faelle und $(AUTH_USERS) mit dem Inhalt von $(ARCHIV)."; \
		printf "Fortfahren? Vorher wird automatisch eine Sicherheits-Sicherung angelegt. [y/N] "; \
		read ans; \
		[ "$$ans" = "y" ] || [ "$$ans" = "Y" ] || { echo "Abgebrochen."; exit 1; }; \
	fi
	@if [ -d "$(FAELLE_ROOT)/faelle" ]; then \
		$(MAKE) backup FAELLE_ROOT=$(FAELLE_ROOT) AUTH_USERS=$(AUTH_USERS) BACKUP_DIR=$(BACKUP_DIR) || exit 1; \
	else \
		echo "Hinweis: $(FAELLE_ROOT)/faelle existiert nicht — nichts zu sichern, Vorher-Sicherung uebersprungen."; \
	fi
	rm -rf $(FAELLE_ROOT)/faelle
	mkdir -p $(FAELLE_ROOT) $(dir $(AUTH_USERS))
	tar xzf $(ARCHIV) -C $(FAELLE_ROOT) faelle
	tar xzf $(ARCHIV) -C $(dir $(AUTH_USERS)) $(notdir $(AUTH_USERS)) 2>/dev/null || \
		echo "Hinweis: $(notdir $(AUTH_USERS)) nicht im Archiv (altes Backup?) — Konten NICHT wiederhergestellt."


## S0.1: §32a tariff tests only.
s01:
	$(OPAM_ENV); clerk test -W rules/estg/p32a/

## S0.3: Arbeitszimmer/Homeoffice tests only.
s03:
	$(OPAM_ENV); clerk test -W rules/estg/p04_arbeitszimmer_homeoffice/

## Compile the §32a Catala module to a self-contained Python package.
build-python:
	$(OPAM_ENV); clerk build p32a-python
	bash oracle/gettsim/assemble_catala.sh

## S0.2: differential test Catala vs GETTSIM. Regenerates reports/s02-divergenzen.md.
s02: build-python
	. $(VENV312); python oracle/gettsim/harness.py

## Golden-Korpus x GETTSIM Cross-Check (Paket 9). Regenerates
## reports/review/2026-07-16-gettsim-crosscheck.md + runs the gate.
gettsim-crosscheck: build-python
	. $(VENV312); python oracle/gettsim/golden_crosscheck.py
	. $(VENV312); python -m pytest tests/test_gettsim_crosscheck.py -q

## Phase-1 deliverable: Arbeitnehmerfall end-to-end (Bruttolohn -> festzusetzende ESt)
## differential vs GETTSIM. Regenerates reports/p1-arbeitnehmerfall.md.
p1: build-python
	. $(VENV312); python oracle/gettsim/harness_e2e.py

## Derive/validate the tariff coefficients from the GETTSIM zone parameters.
params-check:
	. $(VENV312); python params/derive_coefficients.py

## Verify the frozen source archive against the recorded SHA256 hashes.
sources-check:
	python3 scripts/verify_sources.py

## Snapshot the deterministic verdict of every verified rule (runs/-Blocker-Fix).
## Commit pipeline/snapshots/ so a fresh clone can --regate without model costs.
snapshot:
	python3 pipeline/snapshot.py write --all

## Verify sha256(catala_a) of every committed snapshot (fast, no clerk). Nonzero on tamper.
snapshot-verify:
	python3 pipeline/snapshot.py verify --all

## Run the golden corpus against the Catala formalisation (value + citation anchor).
golden: build-python
	. $(VENV312); python golden/runner.py

## Regenerate the golden § 32a cases from the published tariff.
golden-generate:
	python3 golden/generate_cases.py

## Import scalar parameters from GETTSIM into params/<vz>/ with provenance.
params-import:
	. $(VENV312); python params/import_gettsim.py

## Document store (M1.5). Requires a Docker daemon for up/down.
docstore-up:
	docker compose -f docstore/docker-compose.yml up -d

docstore-down:
	docker compose -f docstore/docker-compose.yml down

## Apply the schema (used by the non-Docker path; Docker auto-applies it).
docstore-schema:
	. $(VENV312); python -c "import os,psycopg; psycopg.connect(os.environ.get('DOCSTORE_DSN','host=127.0.0.1 dbname=taxgraph_docstore user=taxgraph password=taxgraph')).cursor().execute(open('docstore/schema.sql').read())" || \
	psql -d taxgraph_docstore -f docstore/schema.sql

## Ingest the frozen sources/ into the document store.
docstore-ingest:
	. $(VENV312); python docstore/ingest.py

## Validate the ELSTER field-mapping stub against its format (no ELSTER access needed).
elster-check:
	. $(VENV312); python elster/validate_mapping.py

## ERiC Offline-CI-Gate (VZ 2025): E10_2025-XSD-Struktur + checkESt (ERIC_VALIDIERE,
## offline, kein Versand, keine Datei-Credentials). Hersteller-ID nur aus $ELSTER_HERSTELLER_ID
## falls exportiert; ohne sie laeuft das Gate ueber die dokumentierte GESPERRT-Grenze durch.
## Laedt eine lokale, gitignored .env (falls vorhanden) VOR dem Python-Aufruf, damit die ID nicht
## per Hand exportiert werden muss; bereits gesetztes Prozess-Env gewinnt (kein Override). In CI
## fehlt .env -> bleibt credential-frei, faellt weiterhin auf die GESPERRT-Grenze zurueck.
eric-gate:
	if [ -z "$$ELSTER_HERSTELLER_ID" ] && [ -f .env ]; then set -a; . ./.env; set +a; fi; \
	ERIC_DIR=$${ERIC_DIR:-$$HOME/02_Software/eric} python3 elster/eric_gate.py

clean:
	$(OPAM_ENV); clerk clean || true
	rm -rf _build _target oracle/gettsim/_catala
