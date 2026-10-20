//! Die drei Werkzeuge für die eingefrorenen Rechtstexte unter `sources/`, in Rust statt Python:
//!
//! | Unterbefehl | ersetzt | Aufgabe |
//! |---|---|---|
//! | `pruefe` | `scripts/verify_sources.py` | jede `*.meta.yaml`: `sha256` stimmt, Wortlaut mindestens 60 Zeichen (`make sources-check`) |
//! | `einfrieren` | `scripts/freeze_source.py` | URL abrufen, HTML entstrippen, Passage ausschneiden, prüfen, `<name>.txt` und `<name>.meta.yaml` schreiben |
//! | `pdf-einfrieren` | `scripts/freeze_pdf_local.py` | lokales PDF mit `pdftotext -layout` einfrieren |
//!
//! Ziel ist dieselbe Ausgabe Byte für Byte; der Vergleich mit den Python-Skripten steht im Bericht des Auftrags.

pub mod cli;
pub mod einfrieren;
pub mod entity;
pub mod entity_tabelle;
pub mod fehler;
pub mod pdf;
pub mod pruefen;
pub mod py;
pub mod unterprozess;

pub use fehler::Fehler;
