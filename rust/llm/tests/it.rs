//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.
// Zwei Dateien binden `mod stub;` ein. Jede bekommt ihre eigene Kopie, wie vorher als eigenes Binary: gewollt, kein versehentlich doppelt geladenes Modul.
#![allow(clippy::duplicate_mod)]

#[path = "client_entscheidungen.rs"]
mod client_entscheidungen;
#[path = "client_netz.rs"]
mod client_netz;
#[path = "gates_entscheidungen.rs"]
mod gates_entscheidungen;
#[path = "ors_netz.rs"]
mod ors_netz;
#[path = "parse_entscheidungen.rs"]
mod parse_entscheidungen;
#[path = "pii_entscheidungen.rs"]
mod pii_entscheidungen;
#[path = "prompt_entscheidungen.rs"]
mod prompt_entscheidungen;
#[path = "schema_entscheidungen.rs"]
mod schema_entscheidungen;
