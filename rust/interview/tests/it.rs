//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.

#[path = "bic_nach_iban.rs"]
mod bic_nach_iban;
#[path = "eigenschaften.rs"]
mod eigenschaften;
#[path = "gate_polaritaet.rs"]
mod gate_polaritaet;
#[path = "p24a_rueckfall.rs"]
mod p24a_rueckfall;
#[path = "vorjahr_fragen.rs"]
mod vorjahr_fragen;
