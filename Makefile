# TaxGraph v3 - Phase 0 build and test entrypoints.
#
# Toolchain (see docs/setup.md):
#   - Catala/Clerk via opam switch "taxgraph"
#   - GETTSIM + Catala python runtime in oracle/.venv312 (Python 3.12)
#
# The opam environment is loaded per target so no shell pre-setup is needed.

OPAM_ENV := eval $$(opam env --switch=taxgraph --set-switch)
VENV312  := oracle/.venv312/bin/activate

.PHONY: all s01 s03 tests build-python s02 clean backup restore serve serve-python

all: unit unit-stufe-b tests s02

## Run all Catala/Clerk scope tests (S0.1 tariff, S0.3 Arbeitszimmer/Homeoffice).
tests:
	$(OPAM_ENV); clerk test -W rules/

## Regressionstests der Gate-Semantik (kein Catala, kein Netz, <1s).
## Parallel mit 6 Workern (Julius-Vorgabe 2026-08-19, Maschine hat 12 Kerne). Gemessen am
## selben Checkout: sequenziell 312s, -n 12 68s, beide Läufe 0 Fails. --dist loadfile hält
## jede Testdatei auf EINEM Worker — Pflicht, nicht Kür: test_bescheid_fn_collector und
## test_datenwurzel_ausserhalb_repo schreiben feste /tmp-Pfade, dateiübergreifend gäbe das
## Kollisionen. Prozess-basiert, daher kein Konflikt mit der catala-Thread-Unsafety.
## --continue-on-collection-errors (2026-08-28): ohne das Flag bricht pytest die GANZE Sitzung
## ab, sobald EIN Modul beim Sammeln scheitert — und fast jedes Testmodul hier ruft
## `lade_bindung()` schon auf Modulebene. Eine einzige kaputte bindung-YAML legte damit den
## kompletten Lauf lahm, unter -n 6 sogar mit Haenger bis zum Timeout. Gemessen an genau diesem
## Fall: mit dem Flag laeuft tests/test_bindung_yaml_laedt.py durch und nennt Datei, Zeile,
## Spalte und Ursache; die Folgefehler stehen daneben statt davor.
## FAIL-CLOSED BLEIBT ES, nachgemessen: kaputte Datei -> exit 1, alles heil -> exit 0. Das Flag
## verschluckt nichts, es verwandelt „Abbruch ohne Diagnose" in „roter Test mit Diagnose".
unit:
	python3 -m pytest tests/ -q -n 6 --dist loadfile --continue-on-collection-errors

## Sicherung Fall-Store + Audit-Log + Benutzerkonten (Audit 2026-08-16/17, data-no-backup-restore).
## Die Falldateien sind gitignored — ohne dieses Ziel gibt es KEINE Recovery. audit.jsonl liegt
## BEREITS unter faelle/ (produkt/store/audit.py:AUDIT_DIR) und wird von der faelle/-Sicherung
## automatisch mit erfasst — der alte "audit.jsonl"-Operand traf nie (er lag relativ zu
## FAELLE_ROOT statt zu FAELLE_ROOT/faelle, Review 2026-08-17). users.json (produkt/auth/) MUSS
## mit ins Archiv — ohne sie zeigen restaurierte Faelle nach einem Umzug auf Konten, die es dort
## nicht gibt, und die fail-closed Zugriffspruefung (_fall_owner_check) sperrt sie dauerhaft.
## FAELLE_ROOT/AUTH_USERS/BACKUP_DIR sind per Kommandozeile overridebar (Pflicht fuer den
## Round-Trip-Test in tests/test_backup_restore_roundtrip.py, der NIE die echten Pfade anfasst).
## BACKUP_DIR liegt NEBEN dem Checkout, nicht unter $(HOME) (Nutzer-Vorgabe: nichts im Home anlegen;
## dort liegen die bisherigen Sicherungen ohnehin schon).
##
## FAELLE_ROOT zeigt seit 2026-08-19 nach $XDG_DATA_HOME/taxgraph statt nach produkt/haut: die
## Steuerdaten liegen nicht mehr im Projektverzeichnis (Entscheidung zum Audit-Punkt
## verschluesselung-steuerdaten-im-klartext). Der Wert MUSS mit api_constants._daten_wurzel()
## uebereinstimmen — laufen die beiden auseinander, sichert `make backup` ein leeres Verzeichnis
## und meldet Erfolg. Genau das prueft tests/test_datenwurzel_ausserhalb_repo.py.
##
## Die Reihenfolge ist DREISTUFIG und muss die von _daten_wurzel() sein: $TAXGRAPH_DATEN, dann
## $XDG_DATA_HOME, dann ~/.local/share. Bis 2026-10-01 fehlte die erste Stufe hier: mit
## TAXGRAPH_DATEN=korpus-rt sicherte `make backup` weiter ~/.local/share/taxgraph, waehrend der
## Code nach korpus-rt schrieb — beide Pfade existierten (207 bzw. 192 Dateien), der Fehler war
## also stumm. `?=` bleibt: FAELLE_ROOT per Kommandozeile schlaegt alle drei Stufen.
BACKUP_DIR  ?= $(abspath $(CURDIR)/../taxgraph-backups)
FAELLE_ROOT ?= $(if $(TAXGRAPH_DATEN),$(TAXGRAPH_DATEN),$(if $(XDG_DATA_HOME),$(XDG_DATA_HOME),$(HOME)/.local/share)/taxgraph)
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
## Totalverlust heisst: es kann auch das DatenVERZEICHNIS selbst fehlen (frische Maschine), nicht nur
## faelle/ darunter. Das `mkdir -p` vor dem Entpacken ist dafuer noetig — ohne es scheitert
## `tar -C $(FAELLE_ROOT)` mit `Cannot chdir` (rc 2, gemessen 2026-10-06) und die Sicherung ist auf
## genau dem Weg unbenutzbar, fuer den sie da ist. Waechter:
## rust/api/tests/backup_restore_rundlauf.rs::restore_legt_das_datenverzeichnis_bei_totalverlust_an.
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

## Produktstart (REWRITE_PLAN Schritt 10, Cutover 2026-10-04): der Rust-Dienst ist das Produkt, der
## Python-Dienst bleibt Referenz, Orakel und Rueckfall (`serve-python`). Beide lesen dieselben
## Umgebungsvariablen und dieselben Dateien (Bestand unter $(FAELLE_ROOT)/faelle, Konten in
## produkt/auth/users.json, .env.llm/.env.maps/.env aus der Repo-Wurzel) — der Wechsel in beide
## Richtungen braucht keine Migration.
## SICHERUNG: beide Ziele sichern den Bestand vor dem Start mit `make backup` (rund 30 MB je Start,
## BACKUP_DIR waechst). Scheitert die Sicherung, startet nichts. SICHERN=0 ueberspringt sie.
## AUTH IST AN: `env -u TAXGRAPH_NO_AUTH` raeumt ein aus der Shell geerbtes Einzelnutzer-Opt-out ab.
## Ein Bau mit `--release` ist hier bewusst NICHT vorgesehen: Cargo schaltet dort overflow-checks und
## debug-assertions ab, und kein Vergleichslauf (rust/parity, `make ui-rust`) hat diesen Bau je
## gemessen. Das dev-Profil (opt-level 1, rust/Cargo.toml) ist der gepruefte Bau, nur ohne die
## Test-Uhr (`festzeit`): der Dienst hat die echte Uhr.
## SERVE_TARGET liegt ausserhalb des Checkouts (target/ ist mehrere GB gross). TAXGRAPH_JWT_SECRET
## setzen, damit Anmeldungen einen Neustart ueberleben (ohne: zufaellig je Start, wie in Python).
SERVE_PORT   ?= 8000
SERVE_TARGET ?= $(HOME)/.cache/taxgraph-serve/target
SICHERN      ?= 1
SICHERN_ZIEL  = $(if $(filter 1,$(SICHERN)),backup)

serve: $(SICHERN_ZIEL)
	cd rust && CARGO_TARGET_DIR=$(SERVE_TARGET) cargo build -p api --bin taxgraph-api
	@echo "Rust-Dienst auf http://127.0.0.1:$(SERVE_PORT), Bestand $(FAELLE_ROOT)/faelle"
	exec env -u TAXGRAPH_NO_AUTH TAXGRAPH_ROOT=$(CURDIR) $(SERVE_TARGET)/debug/taxgraph-api $(SERVE_PORT)

## Rueckfall und Referenz: der Python-Dienst, wie er bis zum Cutover lief.
serve-python: $(SICHERN_ZIEL)
	@echo "Python-Dienst auf http://127.0.0.1:$(SERVE_PORT), Bestand $(FAELLE_ROOT)/faelle"
	exec env -u TAXGRAPH_NO_AUTH python3 produkt/haut/server.py $(SERVE_PORT)


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
	. $(VENV312); python -m pytest pipeline/tests/test_gettsim_crosscheck.py -q

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
	. $(VENV312); python golden/golden_lauf.py

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

## Lokaler Freigabenachweis fuer den ERiC-Abgabeweg (Entscheidung
## eric-abgabeweg-bleibt-lokaler-manueller-nachweis, Julius 2026-09-12). Dieses Ziel ist der
## Nachweis, den die EIGENTLICHE Abgabe prueft — `make eric-gate` ist es NICHT: das Gate prueft
## eine Minimal-XML und zaehlt die GESPERRT-Grenze als Bestehen, laeuft also auch dann gruen,
## wenn der Abgabeweg nie gerechnet hat.
##
## Faehrt tests/test_einreichen_durchstich.py durch den ECHTEN HTTP-Endpunkt mit ECHTEM
## checkESt und ECHTEM eric_gate — und macht aus JEDEM Skip ein exit != 0 (tests/skip_ist_rot.py,
## ueber PYTHONPATH geholt). Ohne ERiC oder ohne Herstellerkennung ueberspringen die beiden
## ERiC-Faelle naemlich weiterhin, und pytest meldete dafuer exit 0: "CI gruen" und "Abgabeweg
## geprueft" saehen gleich aus. Genau diese Verwechslung ist der Backlog-Eintrag
## ci-beweist-den-abgabeweg-nicht.
##
## Kein Versand: der Pfad ruft `EricBearbeiteVorgang` mit ERIC_VALIDIERE (ohne ERIC_SENDE),
## cryptoParameter und serverantwortXmlPuffer bleiben NULL. Nachgemessen mit strace — im ganzen
## Lauf kein einziger Netz-Syscall ausserhalb von 127.0.0.1.
##
## Herstellerkennung wie bei eric-gate aus einer gitignorierten .env, bereits gesetztes
## Prozess-Env gewinnt. ERIC_DIR muss auf die ERiC-Auslieferung zeigen; der Default
## ~/02_Software/eric findet die Lib auch in Unterordnern (elster/smoke_test.find_eric_lib).
abgabeweg-freigabe:
	if [ -z "$$ELSTER_HERSTELLER_ID" ] && [ -f .env ]; then set -a; . ./.env; set +a; fi; \
	ERIC_DIR=$${ERIC_DIR:-$$HOME/02_Software/eric} \
	PYTHONPATH=tests$${PYTHONPATH:+:$$PYTHONPATH} \
	python3 -m pytest tests/test_einreichen_durchstich.py -q -rs -p skip_ist_rot

## Rust-Gegenstueck zu abgabeweg-freigabe (V2): rust/api/tests/einreichen_eric_echt.rs startet den
## echten Dienst (taxgraph-api, echter Socket), baut einen Fall ueber die Routen und ruft POST
## /fall/{id}/einreichen mit der ECHTEN ERiC-Bibliothek und der ECHTEN Herstellerkennung. Nur Pruefung
## (ERIC_VALIDIERE), kein Versand. Der Test ist #[ignore]: `cargo test --workspace` und die CI laufen
## ohne ihn, `checkESt` selbst deckt die CI also NICHT. Hier laeuft er mit --ignored, und jede fehlende
## Voraussetzung (Bibliothek, ID) oder ein anderer Lauf als genau ein bestandener Test macht das Ziel rot.
## Herstellerkennung wie bei abgabeweg-freigabe aus der gitignorierten .env; das Protokoll liegt in
## rust/target/ (der Test nennt weder ID noch Antwortrumpf). Die Verdrahtung (Ziel, --ignored, Testname,
## #[ignore]) haelt rust/api/tests/einreichen_eric_echt.rs::die_freigabe_ist_verdrahtet in der CI fest.
abgabeweg-freigabe-rust:
	if [ -z "$$ELSTER_HERSTELLER_ID" ] && [ -f .env ]; then set -a; . ./.env; set +a; fi; \
	mkdir -p rust/target; log=rust/target/abgabeweg-freigabe-rust.log; \
	(cd rust && cargo test -p api --test einreichen_eric_echt -- --ignored --exact einreichen_ueber_den_echten_endpunkt_mit_echtem_checkest) > $$log 2>&1; rc=$$?; \
	cat $$log; \
	[ $$rc -eq 0 ] && grep -q "test result: ok. 1 passed" $$log

## Block-Matrix gegen checkESt in Rust (Option b, Vault: blockmatrix-laeuft-in-rust-gesperrte-zeilen-ueber-den-test-umweg;
## Ersatz fuer tests/test_checkest_blockmatrix.py): rust/bescheid/tests/checkest_blockmatrix.rs baut zwoelf Bloecke und die Zeilen
## Sonstiges der Anlage N (Nr. 48, Nr. 49) und die Anlage AUS (Staat Nr. 51, Abzug Nr. 41) ueber den Abgabeweg und gibt das XML an die
## ECHTE ERiC-Bibliothek (nur Pruefung, ERIC_VALIDIERE, kein Versand). Die drei Tests sind #[ignore]: die CI hat kein ERiC. Fehlende
## Bibliothek, fehlende oder gesperrte (74931) ID ist ein panic; ein anderer Lauf als genau drei bestandene Tests macht das Ziel rot. Herstellerkennung wie bei
## abgabeweg-freigabe-rust aus der gitignorierten .env; das Protokoll liegt in rust/target/ und nennt die ID nie. Die Verdrahtung
## (Ziel, --ignored, --exact, Testnamen, #[ignore]) haelt checkest_blockmatrix.rs::die_blockmatrix_ist_verdrahtet in der CI fest.
blockmatrix-rust:
	if [ -z "$$ELSTER_HERSTELLER_ID" ] && [ -f .env ]; then set -a; . ./.env; set +a; fi; \
	mkdir -p rust/target; log=rust/target/blockmatrix-rust.log; \
	(cd rust && cargo test -p bescheid --test checkest_blockmatrix -- --ignored --nocapture --test-threads=1 --exact zwoelf_bloecke_gegen_echtes_checkest zeilen_nr48_nr49_gegen_echtes_checkest ausland_staat_und_nr41_gegen_echtes_checkest) > $$log 2>&1; rc=$$?; \
	cat $$log; \
	[ $$rc -eq 0 ] && grep -q "test result: ok. 3 passed" $$log

## Rust-Port (REWRITE_PLAN.md). Der generierte Catala-C-Backend liegt committed unter
## rust/catala-sys/generated/ -- catala-c regeneriert ihn (braucht den Opam-Switch).
catala-c:
	$(OPAM_ENV); bash rust/catala-sys/gen.sh

rust-build:
	cd rust && cargo build

## clippy -D warnings VOR den Tests: ein Lint-Fehler soll den Lauf so rot machen wie ein
## Testfehler. PARITY=1 erzwingt die Tarif-Paritaet gegen tools/parity/oracle.py (braucht den
## Opam-Switch + produkt/); ohne PARITY laufen diese Tests als No-op gruen durch (CI hat kein
## Catala/Python-Environment).
rust-test:
	cd rust && cargo build && cargo clippy --all-targets -- -D warnings && cargo test

## Playwright-Tests der Oberflaeche gegen den RUST-Server (REWRITE_PLAN §6/§7, 9c: "Playwright gegen Rust").
## Baut taxgraph-api und startet alle Dateien in tests/ mit `sync_playwright` (heute 22) mit dem Plugin
## tools/ui_rust/ui_rust_plugin.py, das nur `server.make_server` durch den Start von Rust ersetzt. Die
## Tests, die gegen Rust nicht gruen werden koennen, stehen mit Ursache in tools/ui_rust/ausschluss.tsv:
## sie laufen als xfail(strict). Ein gelisteter Test, der gruen wird, und ein Eintrag ohne Test machen
## den Lauf rot; ein neuer Test ausserhalb der Liste, der rot wird, bleibt rot. Braucht Playwright mit
## Chromium (`pip install playwright`, `playwright install chromium`); CI hat beides nicht.
## UI_N = Zahl der xdist-Worker. Das Binary liegt unter $CARGO_TARGET_DIR (sonst rust/target), UI_RUST_BIN ueberschreibt es.
## Die Pruefung der Liste (test_ui_rust_ausschluss.py) nennt `sync_playwright` nur als Text und ist keine UI-Datei.
UI_DATEIEN = $(shell grep -l sync_playwright tests/test_*.py | grep -v test_ui_rust_ausschluss | sort)
UI_N ?= 4

ui-rust:
	cd rust && cargo build -p api --bin taxgraph-api
	PYTHONPATH=tools/ui_rust python3 -m pytest -p ui_rust_plugin $(UI_DATEIEN) -q -n $(UI_N) --dist loadfile -p no:cacheprovider

## Gegenprobe zu ui-rust: Rust startet und endet sofort (UI_RUST_GEGENPROBE=1). Die Tests muessen rot werden;
## bleiben sie gruen, reden sie nicht mit Rust. Das Ziel ist gruen, wenn pytest rot war.
ui-rust-gegenprobe:
	cd rust && cargo build -p api --bin taxgraph-api
	@if UI_RUST_GEGENPROBE=1 PYTHONPATH=tools/ui_rust python3 -m pytest -p ui_rust_plugin tests/test_ui_login.py -q -p no:cacheprovider; \
	then echo "GEGENPROBE FEHLGESCHLAGEN: die Tests blieben gruen, obwohl Rust sofort endet"; exit 1; \
	else echo "Gegenprobe rot, wie gewollt: ohne Rust-Prozess laufen die UI-Tests nicht"; fi

## Stufe-B-Tests (pipeline/, ebilanz/, GETTSIM-Gegenprobe) unter pipeline/tests/. Sie brauchen weder
## produkt/ noch tests/conftest.py noch Catala (gemessen 2026-10-06: gruen in einem Baum ohne produkt/,
## tests/ und rust/) und laufen deshalb auch dann weiter, wenn `tests/` geloescht ist. Ohne venv312
## ueberspringt sich test_gettsim_crosscheck (1 skipped); `make gettsim-crosscheck` faehrt ihn echt.
## Nur 14 s, daher ohne -n. Steht hier und nicht bei `unit`, weil REWRITE_PLAN.md und
## tools/docs_zahlen_pruefen.py Makefile-Zeilen verankern; Zeilen davor verschieben sie.
unit-stufe-b:
	python3 -m pytest pipeline/tests -q

clean:
	$(OPAM_ENV); clerk clean || true
	rm -rf _build _target oracle/gettsim/_catala
