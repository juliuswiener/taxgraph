# TaxGraph v3

Regelbasierte Formalisierung der deutschen Einkommensteuer in
[Catala](https://catala-lang.org), verifiziert gegen unabhaengige Oracles.
Leitprinzip (AINA): LLM schlaegt vor, deterministische Tools verifizieren,
Mensch entscheidet. Roadmap in `docs/taxgraph-v3-roadmap.md`.

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

## Repo-Layout

```
rules/estg/p32a/                        # § 32a literate Catala + Tests
rules/estg/p04_arbeitszimmer_homeoffice/# § 4 Abs. 5 Nr. 6b/6c literate Catala + Tests
params/2024|2025|2026/                  # Tarifparameter je VZ mit Quellen
params/derive_coefficients.py           # Ableitung/Validierung der Koeffizienten
oracle/gettsim/                         # Differentialtest-Harness + Assembler
sources/                                # Fassungsarchiv: eingefrorene Gesetzestexte + Hash
reports/                                # s02-divergenzen, s03-ergonomie, gate-g0
docs/                                   # setup.md, roadmap, handover
scripts/                                # Installations- und Verifikationsskripte
```

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
