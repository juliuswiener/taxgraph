//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.

#[path = "bank_auswahl.rs"]
mod bank_auswahl;
#[path = "bank_regeln.rs"]
mod bank_regeln;
#[path = "kontoauszug_echtdatei.rs"]
mod kontoauszug_echtdatei;
#[path = "kontoauszug_profile.rs"]
mod kontoauszug_profile;
#[path = "unterprozess_zeitlimit.rs"]
mod unterprozess_zeitlimit;
#[path = "vast_beleg_hermetisch.rs"]
mod vast_beleg_hermetisch;
#[path = "vast_ueberlauf_hermetisch.rs"]
mod vast_ueberlauf_hermetisch;
