# TaxGraph v3 - Phase 0 build and test entrypoints.
#
# Toolchain (see docs/setup.md):
#   - Catala/Clerk via opam switch "taxgraph"
#   - GETTSIM + Catala python runtime in oracle/.venv312 (Python 3.12)
#
# The opam environment is loaded per target so no shell pre-setup is needed.

OPAM_ENV := eval $$(opam env --switch=taxgraph --set-switch)
VENV312  := oracle/.venv312/bin/activate

.PHONY: all s01 s03 tests build-python s02 clean

all: tests s02

## Run all Catala/Clerk scope tests (S0.1 tariff, S0.3 Arbeitszimmer/Homeoffice).
tests:
	$(OPAM_ENV); clerk test -W rules/

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

## Derive/validate the tariff coefficients from the GETTSIM zone parameters.
params-check:
	. $(VENV312); python params/derive_coefficients.py

## Verify the frozen source archive against the recorded SHA256 hashes.
sources-check:
	python3 scripts/verify_sources.py

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

clean:
	$(OPAM_ENV); clerk clean || true
	rm -rf _build _target oracle/gettsim/_catala
