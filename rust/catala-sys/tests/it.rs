//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.

#[path = "ausgabe_ueberlauf_hermetisch.rs"]
mod ausgabe_ueberlauf_hermetisch;
#[path = "catala_orakel.rs"]
mod catala_orakel;
#[path = "shim_ueberlauf_waechter.rs"]
mod shim_ueberlauf_waechter;
#[path = "uebergabe_kinderzahl_und_zulage.rs"]
mod uebergabe_kinderzahl_und_zulage;
