# Fuzz-Targets (Schritt 9b-F)

Parser externer Eingabe. Ziel: kein Panic, kein Overflow, keine Endlosschleife. Eigener Workspace,
kein Mitglied von `rust/Cargo.toml` — `cargo build/clippy/test --workspace` auf stable bleibt unberuehrt.
Braucht `cargo-fuzz` und `rustup toolchain nightly`.

| Target    | Funktionen                                                                                   |
|-----------|----------------------------------------------------------------------------------------------|
| `llm`     | `llm::parse::{chat,rueckfragen,antwort,aussagen,zuordnung}_parse`, `kontoauszug::parse_kategorie`, `pii::filtere` |
| `eingang` | `csv::lies_datensaetze`, `ocr::tsv_zu_zeilen`, `kontoauszug::{parse_csv,parse_pdf_zeilen,eur_cent_signed,py_float,aus_json}`, `beleg::{erkenne_beleg_typ,extrahiere}`, `vast::{cent,aus_lstb,aus_lersl}`, `edaten::uebernehme`, `vorjahr::uebernehme` |
| `elster`  | `xsd_walk`/`kz_meta`/`schema_info` (XSD), Snapshot -> `deklariere` -> `zuruecklesen` -> `erzeuge_xml` (Assert: XML wohlgeformt), `parse_instanz`, `kz_format`, `kz_wert` |
| `store`   | `store::lade` (Assert: lade -> speichere -> lade stabil), `audit::lies`, `fehler_log::lies`  |
| `bindung` | `lade_bindung`, `lade_params`, `lade_kohorten` (Temp-Datei je Ausfuehrung)                    |

Byte 0 der Eingabe waehlt bei `llm`/`eingang`/`elster`/`store` Parameter bzw. Zweig (siehe Kopfkommentar je Target).

## Starten

```
export TMPDIR=$HOME/.cache/taxgraph-tmp CARGO_TARGET_DIR=$HOME/.cache/taxgraph-target-fuzz
cd rust/fuzz
F=$PWD
cargo +nightly fuzz build
cargo +nightly fuzz run llm $F/corpus/llm $F/seeds/llm -- -max_total_time=300 -timeout=10 -rss_limit_mb=2048
```

Ersetze `llm` durch `eingang`, `elster`, `store` oder `bindung`. Standarddauer: 300 s je Target.
`-jobs=N -workers=N` faechert auf, schreibt dann `fuzz-N.log` ins Arbeitsverzeichnis.

## Dateien

- `seeds/<target>/` — kleiner, von Hand erzeugter Seed-Korpus (eingecheckt, keine echten Nutzerdaten).
- `corpus/<target>/` — Laufkorpus, waechst beim Fuzzen, per `.gitignore` ausgeschlossen.
- `artifacts/<target>/crash-*`, `timeout-*`, `oom-*` — Abstuerze, ausgeschlossen. Wiederholen:
  `cargo +nightly fuzz run <target> artifacts/<target>/crash-<hash>`; minimieren: `cargo +nightly fuzz tmin <target> <datei>`.
- `regressions/<target>/` — minimierte Absturzfaelle als Regressions-Seed (eingecheckt). Wiederholen wie oben.
