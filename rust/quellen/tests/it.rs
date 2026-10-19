//! Alle Testdateien dieses Crates in EINEM Test-Binary (weniger Links). Neue Testdatei: hier eintragen;
//! `rust/bindung/tests/testbinaries_vollstaendig.rs` schlägt sonst an.
//! Aufruf einer Datei: `cargo test -p quellen --test it <datei>::`.

#[path = "einfrieren_golden.rs"]
mod einfrieren_golden;
#[path = "entity_hermetisch.rs"]
mod entity_hermetisch;
#[path = "pruefen_hermetisch.rs"]
mod pruefen_hermetisch;
