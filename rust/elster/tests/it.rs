//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.
// Zwei Dateien binden `mod gemeinsam;` ein. Jede bekommt ihre eigene Kopie, wie vorher als eigenes Binary: gewollt, kein versehentlich doppelt geladenes Modul.
#![allow(clippy::duplicate_mod)]

#[path = "bindungs_typ_vs_xsd_typ.rs"]
mod bindungs_typ_vs_xsd_typ;
#[path = "eigenschaften.rs"]
mod eigenschaften;
#[path = "instanzen_hermetisch.rs"]
mod instanzen_hermetisch;
#[path = "kz_pfade_im_schema.rs"]
mod kz_pfade_im_schema;
#[path = "kz_rundung.rs"]
mod kz_rundung;
#[path = "kz_sperre.rs"]
mod kz_sperre;
#[path = "nicht_negativ_gegen_schema.rs"]
mod nicht_negativ_gegen_schema;
#[path = "rentenbeginn_datumsformat.rs"]
mod rentenbeginn_datumsformat;
#[path = "vz_gueltigkeit_schema.rs"]
mod vz_gueltigkeit_schema;
#[path = "xsd_pruefer_hermetisch.rs"]
mod xsd_pruefer_hermetisch;
#[path = "zeichensatz_gegen_schema.rs"]
mod zeichensatz_gegen_schema;
