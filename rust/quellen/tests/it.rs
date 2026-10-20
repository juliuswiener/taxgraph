//! Alle Testdateien dieses Crates in EINEM Test-Binary (weniger Links). Neue Testdatei: hier eintragen;
//! `rust/bindung/tests/testbinaries_vollstaendig.rs` schlägt sonst an.
//! Aufruf einer Datei: `cargo test -p quellen --test it <datei>::`.

#[path = "cli_hermetisch.rs"]
mod cli_hermetisch;
#[path = "einfrieren_golden.rs"]
mod einfrieren_golden;
#[path = "entity_hermetisch.rs"]
mod entity_hermetisch;
#[path = "pdf_lokal_hermetisch.rs"]
mod pdf_lokal_hermetisch;
#[path = "pruefen_hermetisch.rs"]
mod pruefen_hermetisch;
#[path = "py_paritaet.rs"]
mod py_paritaet;
#[path = "unterprozess_hermetisch.rs"]
mod unterprozess_hermetisch;
