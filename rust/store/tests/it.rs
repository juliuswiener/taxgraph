//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.

#[path = "akten_ueberlauf_hermetisch.rs"]
mod akten_ueberlauf_hermetisch;
#[path = "append_roh.rs"]
mod append_roh;
#[path = "audit_aufrufer.rs"]
mod audit_aufrufer;
#[path = "audit_parallel.rs"]
mod audit_parallel;
#[path = "dateioperationen.rs"]
mod dateioperationen;
#[path = "eigenschaften.rs"]
mod eigenschaften;
#[path = "entscheidungsstellen.rs"]
mod entscheidungsstellen;
#[path = "zeichensatz.rs"]
mod zeichensatz;
