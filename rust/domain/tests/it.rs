//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.

#[path = "bereiche_hermetisch.rs"]
mod bereiche_hermetisch;
#[path = "fehlerpfade_hermetisch.rs"]
mod fehlerpfade_hermetisch;
#[path = "fehlertexte_hermetisch.rs"]
mod fehlertexte_hermetisch;
#[path = "grenzen_hermetisch.rs"]
mod grenzen_hermetisch;
#[path = "lader_abweichungen_hermetisch.rs"]
mod lader_abweichungen_hermetisch;
#[path = "ueberlauf_luecke_hermetisch.rs"]
mod ueberlauf_luecke_hermetisch;
#[path = "zeichensatz_hermetisch.rs"]
mod zeichensatz_hermetisch;
#[path = "zuordnung_hermetisch.rs"]
mod zuordnung_hermetisch;
