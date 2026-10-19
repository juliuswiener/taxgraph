# TaxGraph v3 - Phase 0 build and test entrypoints.
#
# Toolchain (see docs/setup.md):
#   - Catala/Clerk via opam switch "taxgraph"
#   - GETTSIM in oracle/.venv312 (Python 3.12), fuer params-check, params-import, docstore-*, elster-check
#
# The opam environment is loaded per target so no shell pre-setup is needed.

OPAM_ENV := eval $$(opam env --switch=taxgraph --set-switch)
VENV312  := oracle/.venv312/bin/activate

.PHONY: all s01 s03 tests clean backup restore serve golden

all: unit-stufe-b tests

## Run all Catala/Clerk scope tests (S0.1 tariff, S0.3 Arbeitszimmer/Homeoffice).
tests:
	$(OPAM_ENV); clerk test -W rules/

## Sicherung Fall-Store + Audit-Log + Benutzerkonten (Audit 2026-08-16/17, data-no-backup-restore).
## Die Falldateien sind gitignored — ohne dieses Ziel gibt es KEINE Recovery. audit.jsonl liegt
## BEREITS unter faelle/ (Konfig::audit_pfad, rust/api/src/konfig.rs) und wird von der faelle/-Sicherung
## automatisch mit erfasst — der alte "audit.jsonl"-Operand traf nie (er lag relativ zu
## FAELLE_ROOT statt zu FAELLE_ROOT/faelle, Review 2026-08-17). users.json (neben faelle/) MUSS
## mit ins Archiv — ohne sie zeigen restaurierte Faelle nach einem Umzug auf Konten, die es dort
## nicht gibt, und die fail-closed Zugriffspruefung sperrt sie dauerhaft. AUTH_USERS ist wie Konfig::nutzerdatei (E3 des Loeschplans) $(FAELLE_ROOT)/users.json; fehlt sie, bricht backup ab. Der Rueckgriff auf produkt/auth/users.json fiel mit L3.
## FAELLE_ROOT/AUTH_USERS/BACKUP_DIR sind per Kommandozeile overridebar (Pflicht fuer den
## Round-Trip-Test rust/api/tests/backup_restore_rundlauf.rs, der NIE die echten Pfade anfasst).
## BACKUP_DIR liegt NEBEN dem Checkout, nicht unter $(HOME) (Nutzer-Vorgabe: nichts im Home anlegen;
## dort liegen die bisherigen Sicherungen ohnehin schon).
##
## FAELLE_ROOT zeigt seit 2026-08-19 nach $XDG_DATA_HOME/taxgraph statt nach produkt/haut: die
## Steuerdaten liegen nicht mehr im Projektverzeichnis (Entscheidung zum Audit-Punkt
## verschluesselung-steuerdaten-im-klartext). Der Wert MUSS mit Konfig::aus_env (rust/api/src/konfig.rs)
## uebereinstimmen — laufen die beiden auseinander, sichert `make backup` ein leeres Verzeichnis
## und meldet Erfolg. Das prueft rust/api/tests/datenwurzel.rs::make_backup_sichert_den_ordner_den_der_dienst_benutzt (`make -n backup` gegen Konfig::aus_env, alle drei Stufen).
##
## Die Reihenfolge ist DREISTUFIG und muss die von Konfig::aus_env sein: $TAXGRAPH_DATEN, dann
## $XDG_DATA_HOME, dann ~/.local/share. Bis 2026-10-01 fehlte die erste Stufe hier: mit
## TAXGRAPH_DATEN=korpus-rt sicherte `make backup` weiter ~/.local/share/taxgraph, waehrend der
## Code nach korpus-rt schrieb — beide Pfade existierten (207 bzw. 192 Dateien), der Fehler war
## also stumm. `?=` bleibt: FAELLE_ROOT per Kommandozeile schlaegt alle drei Stufen.
BACKUP_DIR  ?= $(abspath $(CURDIR)/../taxgraph-backups)
FAELLE_ROOT ?= $(if $(TAXGRAPH_DATEN),$(TAXGRAPH_DATEN),$(if $(XDG_DATA_HOME),$(XDG_DATA_HOME),$(HOME)/.local/share)/taxgraph)
AUTH_USERS  ?= $(FAELLE_ROOT)/users.json

## RECHTE (S5a, Julius 2026-10-15): das Archiv traegt die Passwort-Hashes (users.json) und alle Steuerdaten, es
## darf nur der Besitzer lesen (Modus 600). Jede Rezeptzeile ist eine eigene Shell, `umask 077` steht darum in
## jeder, die etwas anlegt; die Maske der aufrufenden Shell (ueblich 022 = 644) gilt hier nicht. Ein SCHON
## vorhandener BACKUP_DIR behaelt seinen Modus (mkdir -p aendert ihn nicht); die Archive darin sind trotzdem 600.
## Waechter: rust/api/tests/backup_restore_rundlauf.rs::ein_archiv_ist_nur_fuer_den_besitzer_lesbar.
##
## AUFBEWAHRUNG (S5b, Julius 2026-10-15): nach einem ERFOLGREICHEN tar bleiben die letzten BACKUP_BEHALTEN (30)
## Archive, die aelteren fallen weg (je Start von `make serve` entsteht eines, ohne Grenze lagen die Steuerdaten in
## beliebig vielen Kopien). Gezaehlt und geloescht werden nur Namen im Schema dieses Rezepts
## (`taxgraph-<Ziffern und Striche>.tar.gz`), nur im Sicherungsordner (der Schritt wechselt hinein und loescht nach
## Namen ohne Pfad), nach Name sortiert (Datum und Uhrzeit stehen vorn, das neueste zuletzt; die Aenderungszeit der
## Dateien zaehlt nicht). Das neue Archiv liegt dabei und ist das neueste. Eine Zahl, die keine ganze Zahl ab 1 ist
## (0 loeschte jedes Archiv), bricht ab, BEVOR etwas angelegt oder geloescht wird. Scheitert tar, bricht make vor dem
## Aufraeumen ab. `restore` raeumt NICHT auf (BACKUP_AUFRAEUMEN=0 an seiner Vorher-Sicherung): das aelteste Archiv
## kann genau das sein, das eingespielt wird. Grenze: ein gescheitertes tar laesst seine angefangene Datei liegen
## (Stand vor S5b); sie zaehlt dann mit. Waechter: backup_restore_rundlauf.rs::nach_einem_backup_bleiben_die_letzten_30_archive.
BACKUP_BEHALTEN   ?= 30
BACKUP_AUFRAEUMEN ?= 1
backup:
	@if [ "$(BACKUP_AUFRAEUMEN)" = "1" ]; then test "$(BACKUP_BEHALTEN)" -ge 1 2>/dev/null || { echo "BACKUP_BEHALTEN muss eine ganze Zahl ab 1 sein, ist aber '$(BACKUP_BEHALTEN)'"; exit 1; }; fi
	umask 077; mkdir -p $(BACKUP_DIR)
	umask 077; tar czf $(abspath $(BACKUP_DIR))/taxgraph-$$(date +%Y%m%d-%H%M%S-%N)-$$$$.tar.gz \
		-C $(FAELLE_ROOT) faelle \
		-C $(dir $(AUTH_USERS)) $(notdir $(AUTH_USERS))
	@if [ "$(BACKUP_AUFRAEUMEN)" = "1" ]; then \
		cd "$(abspath $(BACKUP_DIR))" && ls -1 | grep -E '^taxgraph-[0-9-]+\.tar\.gz$$' | LC_ALL=C sort | head -n -$(BACKUP_BEHALTEN) | \
			while IFS= read -r alt; do echo "Aufbewahrung: entferne $$alt"; rm -f -- "$$alt"; done; \
	fi

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
		$(MAKE) backup FAELLE_ROOT=$(FAELLE_ROOT) AUTH_USERS=$(AUTH_USERS) BACKUP_DIR=$(BACKUP_DIR) BACKUP_AUFRAEUMEN=0 || exit 1; \
	else \
		echo "Hinweis: $(FAELLE_ROOT)/faelle existiert nicht — nichts zu sichern, Vorher-Sicherung uebersprungen."; \
	fi
	rm -rf $(FAELLE_ROOT)/faelle
	mkdir -p $(FAELLE_ROOT) $(dir $(AUTH_USERS))
	tar xzf $(ARCHIV) -C $(FAELLE_ROOT) faelle
	tar xzf $(ARCHIV) -C $(dir $(AUTH_USERS)) $(notdir $(AUTH_USERS)) 2>/dev/null || \
		echo "Hinweis: $(notdir $(AUTH_USERS)) nicht im Archiv (altes Backup?) — Konten NICHT wiederhergestellt."

## Produktstart (REWRITE_PLAN Schritt 10, Cutover 2026-10-04): der Rust-Dienst ist das Produkt. Der
## Python-Dienst (`serve-python`, Rueckfall und Orakel) fiel mit L3; sein letzter Stand ist der Tag
## `python-standard-letzter` (904f6215, auf origin). Der Rust-Dienst liest den Bestand unter
## $(FAELLE_ROOT)/faelle und .env.llm/.env.maps/.env aus der Repo-Wurzel.
## SICHERUNG: das Ziel sichert den Bestand vor dem Start mit `make backup` (rund 30 MB je Start; seit S5b
## bleiben die letzten BACKUP_BEHALTEN = 30 Archive, die aelteren fallen weg). Scheitert die Sicherung, startet
## nichts. SICHERN=0 ueberspringt sie.
## AUTH IST AN: `env -u TAXGRAPH_NO_AUTH` raeumt ein aus der Shell geerbtes Einzelnutzer-Opt-out ab.
## Ein Bau mit `--release` ist hier bewusst NICHT vorgesehen: Cargo schaltet dort overflow-checks und
## debug-assertions ab, und kein Vergleichslauf (`make ui-rust` seit L2, rust/parity seit L1 weg) hat diesen Bau je
## gemessen. Das dev-Profil (opt-level 1, rust/Cargo.toml) ist der gepruefte Bau. Die Test-Uhr
## `festzeit` gibt es seit L1 nicht mehr: der Dienst hat die echte Uhr.
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


## S0.1: §32a tariff tests only.
s01:
	$(OPAM_ENV); clerk test -W rules/estg/p32a/

## S0.3: Arbeitszimmer/Homeoffice tests only.
s03:
	$(OPAM_ENV); clerk test -W rules/estg/p04_arbeitszimmer_homeoffice/

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

## Die 135 Golden-Faelle gegen die Catala-Formalisierung (Wert und Zitatanker), in Rust seit L3 (Vault
## golden-ohne-python-anker-pruefung-nach-rust-2026-10-12): golden_werte rechnet jeden Fall durch die Rust-Engine,
## golden_kopf haelt Zahl, Erwartung und Kennungen, anker prueft jedes Zitat gegen den Quelltext in sources/.
## Die Python-Fassung (golden/golden_lauf.py) bleibt bis L4 liegen, ist aber ohne produkt/ nicht mehr lauffaehig.
## Ein Filter, der nichts trifft, endet in cargo mit "0 passed" und Exit 0: jeder Lauf muss mindestens einen Test bestehen.
golden:
	@cd rust && for t in engine:golden_werte bescheid:golden_kopf bindung:anker; do \
		out=$$(cargo test -p $${t%%:*} --test it $${t##*:}:: 2>&1) || { echo "$$out"; exit 1; }; \
		echo "$$out" | grep "^test result"; \
		echo "$$out" | grep -q "test result: ok. [1-9][0-9]* passed" || { echo "golden: $$t fuehrte keinen Test aus"; exit 1; }; \
	done

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

## Lokaler Freigabenachweis fuer den ERiC-Abgabeweg (V2; das Python-Ziel `abgabeweg-freigabe` fiel mit L2,
## Verlauf: `git show 904f6215:Makefile`): rust/api/tests/einreichen_eric_echt.rs startet den
## echten Dienst (taxgraph-api, echter Socket), baut einen Fall ueber die Routen und ruft POST
## /fall/{id}/einreichen mit der ECHTEN ERiC-Bibliothek und der ECHTEN Herstellerkennung. Nur Pruefung
## (ERIC_VALIDIERE), kein Versand. Der Test ist #[ignore]: `make rust-test` und die CI laufen
## ohne ihn, `checkESt` selbst deckt die CI also NICHT. Hier laeuft er mit --ignored, und jede fehlende
## Voraussetzung (Bibliothek, ID) oder ein anderer Lauf als genau ein bestandener Test macht das Ziel rot.
## Herstellerkennung aus der gitignorierten .env (bereits gesetztes Prozess-Env gewinnt); das Protokoll liegt in
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
	(cd rust && cargo test -p bescheid --test checkest_blockmatrix -- --ignored --nocapture --test-threads=1 --exact zwoelf_bloecke_gegen_echtes_checkest zeilen_nr48_nr49_gegen_echtes_checkest zeilen_nr48_nr49_vz2024_gegen_echtes_checkest ausland_staat_und_nr41_gegen_echtes_checkest privatverkauf_gegen_echtes_checkest rav_bav_gegen_echtes_checkest vorsorge_und_gewinn_gegen_echtes_checkest) > $$log 2>&1; rc=$$?; \
	cat $$log; \
	[ $$rc -eq 0 ] && grep -q "test result: ok. 7 passed" $$log

## Die drei uebrigen Tests gegen das echte checkESt (Vault: drei-echte-checkest-tests-bekommen-ein-make-ziel-2026-10-12): die von
## taxgraph-xml geschriebene Datei (rust/api/tests/xml_export_hermetisch.rs), die Versorgungszeilen 11 bis 13
## (rust/bescheid/tests/versorgung_kz_im_xml.rs) und die Pflicht-Kz der Anlage SO (rust/bescheid/tests/checkest_privatverkauf_spike.rs).
## Je Test ein eigener cargo-Aufruf mit --exact: ein falscher Name liefe als "0 passed" und ist darum rot, denn das Ziel verlangt
## dreimal "ok. 1 passed". Alle drei sind #[ignore] (die CI hat kein ERiC). Nur Pruefung (ERIC_VALIDIERE), kein Versand, kein Netz.
## Fehlende Bibliothek, fehlendes Schema, fehlende oder gesperrte (74931) ID ist ein panic. Herstellerkennung wie bei blockmatrix-rust
## aus der gitignorierten .env; das Protokoll liegt in rust/target/ und nennt die ID nie. Die Verdrahtung (Ziel, -p, --test, --ignored,
## --exact, Testnamen, #[ignore], #[test], Zaehlung) haelt rust/bescheid/tests/checkest_verdrahtung.rs in der CI fest.
checkest-rust:
	if [ -z "$$ELSTER_HERSTELLER_ID" ] && [ -f .env ]; then set -a; . ./.env; set +a; fi; \
	mkdir -p rust/target; log=rust/target/checkest-rust.log; \
	(cd rust && cargo test -p api --test xml_export_hermetisch -- --ignored --nocapture --exact die_geschriebene_datei_besteht_checkest && cargo test -p bescheid --test versorgung_kz_im_xml -- --ignored --nocapture --exact checkest_nimmt_die_zeilen_11_bis_13_an && cargo test -p bescheid --test checkest_privatverkauf_spike -- --ignored --nocapture --exact privatverkauf_kz_pflichtliste_gegen_echtes_checkest) > $$log 2>&1; rc=$$?; \
	cat $$log; \
	[ $$rc -eq 0 ] && [ "$$(grep -c 'test result: ok. 1 passed' $$log)" -eq 3 ]

## Rust-Port (REWRITE_PLAN.md). Der generierte Catala-C-Backend liegt committed unter
## rust/catala-sys/generated/ -- catala-c regeneriert ihn (braucht den Opam-Switch).
catala-c:
	$(OPAM_ENV); bash rust/catala-sys/gen.sh

## Orakel fuer rust/catala-sys/tests/catala_orakel.rs: erzeugte Eingaben, vom Catala-INTERPRETER
## gerechnet, in rust/catala-sys/orakel/orakel.txt. Nach `catala-c` und nach jeder Regelaenderung
## neu erzeugen und mit einchecken (der Test prueft den Regel-Hash im Kopf der Datei).
catala-orakel:
	$(OPAM_ENV); cd rust && cargo run -p catala-sys --example orakel_erzeugen

rust-build:
	cd rust && cargo build

## clippy -D warnings VOR den Tests: ein Lint-Fehler soll den Lauf so rot machen wie ein
## Testfehler. PARITY=1 und die Tarif-Paritaet gegen tools/parity/oracle.py gibt es seit L1 nicht
## mehr: rust/parity ist geloescht (Verlauf: `git show 37269912:rust/parity/`).
##
## cargo-nextest statt `cargo test` (Julius 2026-10-15): ein Prozess je Test, auf 12 Kernen etwa 70 s gegen 157 bis 180 s.
## Installation einmal: `cargo install cargo-nextest --locked` (die CI nimmt 0.9.146 mit Pruefsumme). Konfiguration:
## rust/.config/nextest.toml. nextest fuehrt KEINE Doctests aus; sie laufen im zweiten Schritt (`cargo test --doc`).
## Die Testzahl eines Laufs ist die Summe aus der Zeile `Summary ... N tests run` und den `test result:`-Zeilen der Doctests.
## Die `cargo test -p ... --test ... -- --ignored --exact`-Aufrufe der lokalen ERiC-Ziele oben bleiben `cargo test`: nextest
## braucht fuer #[ignore]-Tests `--run-ignored`, und ihre Pruefung (`test result: ok. N passed`) liest libtest-Zeilen.
rust-test:
	cd rust && cargo build && cargo clippy --all-targets -- -D warnings && cargo nextest run --no-fail-fast && cargo test --doc --no-fail-fast

## Stufe-B-Tests (pipeline/, ebilanz/, GETTSIM-Gegenprobe) unter pipeline/tests/. Sie brauchen weder
## produkt/ noch tests/conftest.py noch Catala (gemessen 2026-10-06: gruen in einem Baum ohne produkt/,
## tests/ und rust/) und laufen deshalb seit L2/L3 ohne diese Verzeichnisse. Ohne venv312
## ueberspringt sich test_gettsim_crosscheck (1 skipped); `make gettsim-crosscheck` faehrt ihn echt.
## Nur 14 s, daher ohne -n. REWRITE_PLAN.md und tools/docs_zahlen_pruefen.py
## verankern Makefile-Zeilen; Zeilen davor verschieben sie.
unit-stufe-b:
	python3 -m pytest pipeline/tests -q

clean:
	$(OPAM_ENV); clerk clean || true
	rm -rf _build _target
