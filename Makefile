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

clean:
	$(OPAM_ENV); clerk clean || true
	rm -rf _build _target oracle/gettsim/_catala
