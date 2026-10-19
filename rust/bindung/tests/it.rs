//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.

#[path = "anker.rs"]
mod anker;
#[path = "bereich_bestand.rs"]
mod bereich_bestand;
#[path = "bereich_pruefung.rs"]
mod bereich_pruefung;
#[path = "ci_catala_orakel.rs"]
mod ci_catala_orakel;
#[path = "ci_konfiguration.rs"]
mod ci_konfiguration;
#[path = "ci_nicht_geprueft.rs"]
mod ci_nicht_geprueft;
#[path = "daten_verzeichnis.rs"]
mod daten_verzeichnis;
#[path = "fehlertexte_hermetisch.rs"]
mod fehlertexte_hermetisch;
#[path = "gitignore_verweise.rs"]
mod gitignore_verweise;
#[path = "keine_python_bindung_im_rust_code.rs"]
mod keine_python_bindung_im_rust_code;
#[path = "lade_alle_yamls.rs"]
mod lade_alle_yamls;
#[path = "muster_pruefung.rs"]
mod muster_pruefung;
#[path = "params_zuordnung_hermetisch.rs"]
mod params_zuordnung_hermetisch;
#[path = "parteispenden_werte_gegen_gesetz.rs"]
mod parteispenden_werte_gegen_gesetz;
#[path = "privatverkauf_werte_gegen_gesetz.rs"]
mod privatverkauf_werte_gegen_gesetz;
#[path = "rav_bav_kz_gegen_vordruck.rs"]
mod rav_bav_kz_gegen_vordruck;
#[path = "registry_quer.rs"]
mod registry_quer;
#[path = "schema_reste.rs"]
mod schema_reste;
#[path = "testmap_vollstaendig.rs"]
mod testmap_vollstaendig;
#[path = "texte_lesbar.rs"]
mod texte_lesbar;
#[path = "ueberlauf_luecke_hermetisch.rs"]
mod ueberlauf_luecke_hermetisch;
#[path = "validierung_und_registry_hermetisch.rs"]
mod validierung_und_registry_hermetisch;
#[path = "vz_bestand.rs"]
mod vz_bestand;
#[path = "werte_gegen_gesetz.rs"]
mod werte_gegen_gesetz;
