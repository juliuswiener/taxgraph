# TaxGraph v3

Regelbasierte Formalisierung der deutschen Einkommensteuer in
[Catala](https://catala-lang.org), verifiziert gegen unabhaengige Oracles.
Leitprinzip (AINA): LLM schlaegt vor, deterministische Tools verifizieren,
Mensch entscheidet.

Backlog, Entscheidungen, Audits und die Roadmap liegen seit 2026-08-19
**nicht mehr im Repo**, sondern im Obsidian-Vault unter `~/00_projects/vault/`
(`backlog/taxgraph.md`, `decisions/`, `audits/`, `architecture/`). Code-Kommentare der
Form `BACKLOG <id>` verweisen auf einen Eintrag dort — die IDs sind beim Umzug erhalten
geblieben und dienen als Ueberschriften.

Dieser Stand ist **Phase 0 (Spike)**: die komplette Kette einmal durchstechen und
Gate G0 (Go/No-Go fuer Catala) mit Daten beantworten.

## Was in Phase 0 steht

- **§ 32a EStG (Einkommensteuertarif)** fuer die Veranlagungszeitraeume 2024,
  2025 und 2026 als literate Catala: Grundtarif (Abs. 1) und Splitting-Verfahren
  (Abs. 5), mit den gesetzlichen Abrundungsregeln.
  `rules/estg/p32a/`.
- **Parameterschicht** getrennt von den Formeln, je VZ, mit Rechtsquelle und
  Datenquelle pro Wert. `params/<vz>/`.
- **Differentialtest gegen GETTSIM 1.2.1**: Catala nach Python kompiliert und
  ueber tausende geseedete zvE-Werte pro VZ mit GETTSIM verglichen.
  `oracle/gettsim/`, Report in `reports/s02-divergenzen.md`.
- **Default-Logic-Ergonomietest**: § 4 Abs. 5 Nr. 6b/6c (Arbeitszimmer und
  Homeoffice-Tagespauschale) inklusive gegenseitigem Ausschluss.
  `rules/estg/p04_arbeitszimmer_homeoffice/`, Report in `reports/s03-ergonomie.md`.
- **Gate-G0-Report** mit Bewertung und Empfehlung: `reports/gate-g0.md`.

## Schnellstart

Voraussetzungen und exakte Versionen: `docs/setup.md`.

```bash
make tests   # alle Clerk-Scope-Tests (12 gruen)
make s02     # Differentialtest Catala vs GETTSIM, erzeugt reports/s02-divergenzen.md
```

### Dienst starten (seit dem Cutover 2026-10-04: Rust)

```bash
make serve          # Rust-Dienst, das Produkt: http://127.0.0.1:8000 (SERVE_PORT=... aendert den Port)
make serve-python   # Python-Dienst: Rueckfall, Referenz und Orakel der Vergleichstests
```

- Beide Ziele sichern den Bestand zuerst mit `make backup` (rund 30 MB je Start, nach
  `../taxgraph-backups`). Scheitert die Sicherung, startet nichts. `SICHERN=0` ueberspringt sie.
- Beide lesen dieselben Dateien und Umgebungsvariablen: Bestand unter `~/.local/share/taxgraph/faelle`
  (`TAXGRAPH_DATEN` und `XDG_DATA_HOME` wirken wie bisher), Konten in `produkt/auth/users.json`,
  `.env.llm` und `.env.maps` aus der Repo-Wurzel. Der Wechsel in beide Richtungen braucht keine Migration.
- Die Anmeldung ist an. `TAXGRAPH_JWT_SECRET` setzen, damit Anmeldungen einen Neustart ueberleben.
- `make serve` baut im dev-Profil (opt-level 1), nicht mit `--release`: nur dieser Bau wird von den
  Vergleichstests gegen Python geprueft, und `--release` schaltet `overflow-checks` ab.
- Rueckfall: `make serve-python` starten. Hat ein Rust-Lauf eine Akte beschaedigt, den Bestand mit
  `make restore ARCHIV=<tar.gz aus ../taxgraph-backups>` zurueckholen. Den Code des letzten Stands mit
  Python als Standard-Start liefert der lokale Tag `python-standard-letzter` (`904f6215`): `git checkout python-standard-letzter`. Es ist nichts geloescht.
- Betrieb: `BACKUP_DIR` waechst um rund 30 MB je Start, ein Aufraeumen gibt es nicht. Setzt Du
  `TAXGRAPH_USER_STORE`, sichert `make backup` weiter `produkt/auth/users.json`: dann `AUTH_USERS=<Pfad>`
  mitgeben. Der Bau liegt in `~/.cache/taxgraph-serve/target` (`SERVE_TARGET=...` aendert das); der erste
  Bau dauerte gemessen 40 s. Der Dienst bindet nur 127.0.0.1.
- Ein Fall ohne Besitzerfeld ist bei eingeschalteter Anmeldung fuer jedes Konto gesperrt (403). Das war in
  Python schon so.
- Der Versand an ELSTER bleibt Python (`versand.py`, Vorbehalt Julius). `POST /fall/<id>/einreichen`
  prueft in beiden Diensten nur (ERiC `ERIC_VALIDIERE`) und sendet nichts.

### Verifizierte Regeln reproduzieren (frischer Clone)

Die Pipeline-Laeufe unter `pipeline/runs/` sind gitignored — ein frischer Clone
hat sie nicht. Das deterministische Verdikt jeder **verifizierten** Regel liegt
stattdessen als committeter Snapshot unter `pipeline/snapshots/<rule_id>.json`
(Catala A/B, Judge-Verdikt, Gates, `sha256(catala_a)` als Integritaets-Waechter).
Ein frischer Clone regatet daraus ohne Modellkosten:

```bash
make snapshot-verify   # sha256-Integritaet aller Snapshots (schnell, kein clerk)
python pipeline/produktion/run.py --regate   # rekonstruiert das Verdikt aus Snapshots
```

Ein Live-Report in `runs/` schlaegt den Snapshot (in-flight vor Archiv); der
Snapshot ist kanonisch nur, wenn kein Live-Report existiert. Ein manipulierter
Snapshot (catala_a geaendert, Hash nicht) failt hart — nie stiller PASS
(`pipeline/tests/test_snapshot.py`). Snapshots nach einer Abnahme nachziehen:
`python pipeline/snapshot.py write --all`.

## Repo-Layout

```
rules/estg/p32a/                        # § 32a literate Catala + Tests
rules/estg/p04_arbeitszimmer_homeoffice/# § 4 Abs. 5 Nr. 6b/6c literate Catala + Tests
params/2024|2025|2026/                  # Tarifparameter je VZ mit Quellen
params/derive_coefficients.py           # Ableitung/Validierung der Koeffizienten
oracle/gettsim/                         # Differentialtest-Harness + Assembler
golden/                                 # Golden-Test-Korpus (M1.4) + Lauf; Rechenkern: produkt/engine/
docstore/                               # Dokumentstore-Schema, Ingest, docker-compose (M1.5)
sources/                                # Fassungsarchiv: eingefrorene Gesetzestexte + Hash
reports/                                # s02-divergenzen, s03-ergonomie, gate-g0, gettsim-issue-draft
docs/                                   # setup.md (Roadmap/Handover: Vault)
scripts/                                # Installations- und Verifikationsskripte
```

## Phase 1 (in Arbeit)

- **Deliverable Arbeitnehmerfall end-to-end**: Bruttoarbeitslohn rein,
  festzusetzende ESt raus. Kette § 9a-Pauschbetrag -> § 10c-Pauschbetrag ->
  § 32a, Einzel- und Zusammenveranlagung (`rules/estg/p32a/`, Tests in
  `rules/estg/arbeitnehmerfall/`). Differentiell gegen GETTSIM: die zvE-Ableitung
  ist exakt, Divergenzen reduzieren sich auf die dokumentierten § 32a-Effekte
  (`make p1`, Report `reports/p1-arbeitnehmerfall.md`).
- **M1.4 Golden-Korpus** (`golden/`): 57 Faelle (§ 32a-Tarif, Arbeitnehmerfall, Entfernungspauschale, Arbeitszimmer/Homeoffice
  end-to-end), je mit Zitatanker-Gate gegen `sources/` und Wertabgleich gegen
  Catala (`make golden`).
- **M1.2 Parameterschicht** (`params/`): kanonisches Format (`params/schema.md`)
  plus GETTSIM-Import mit Herkunftsvermerk (`make params-import`).
- **M1.5 Dokumentstore** (`docstore/`): Postgres-Schema Dokumente/Segmente/Claims
  mit authority, redistributable, Zitatanker und Fassungs-Hash; Ingest der
  eingefrorenen Fassungen (`make docstore-up docstore-ingest`).

## Quellenmodell (Update 2026-07-09)

Jede Regel-, Definitions- und Parameterstruktur traegt `authority`
(Quellenklasse: `gesetz` / `verwaltung` / `bfh` / `fg` / `literatur`) und
`redistributable` (Exportierbarkeit). In Phase 0 durchgaengig `gesetz` / `true`.
Abgerufene Gesetzestexte werden versioniert (URL, Abrufdatum, SHA256) im
Fassungsarchiv `sources/` eingefroren; `make sources-check` prueft die
Integritaet. Kommentar- oder Literaturinhalte erscheinen in keinem Artefakt.

## Ergebnis Gate G0

Empfehlung: Go fuer Catala. Alle vier Kriterien erfuellt; die verbleibenden
Divergenzen zwischen Catala und GETTSIM sind vollstaendig erklaert und laufen auf
zwei steuerfachliche Klaerungen hinaus (Splitting-Rundung nach § 32a Abs. 5,
literale Koeffizienten 2024/2025). Details in `reports/gate-g0.md`.

## Konventionen

- Code und Kommentare englisch, Gesetzestexte und Reports deutsch.
- Keine Gedankenstriche in Texten.
- Jede fachliche Zahl traegt eine Quellenangabe (Rechtsquelle und Datenquelle).
- Kein LLM-Output ohne deterministisches Gate; Divergenzen werden eskaliert,
  nie stillschweigend aufgeloest.
