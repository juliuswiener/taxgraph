//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.

#[path = "catala_orakel_replay.rs"]
mod catala_orakel_replay;
#[path = "eigenschaften.rs"]
mod eigenschaften;
#[path = "ep_gesamt_ueberlauf_hermetisch.rs"]
mod ep_gesamt_ueberlauf_hermetisch;
#[path = "euro_cent_ueberlauf_hermetisch.rs"]
mod euro_cent_ueberlauf_hermetisch;
#[path = "golden_werte.rs"]
mod golden_werte;
#[path = "haushaltsnahe_hermetisch.rs"]
mod haushaltsnahe_hermetisch;
#[path = "p24a_p24b_p31_hermetisch.rs"]
mod p24a_p24b_p31_hermetisch;
#[path = "p35c_und_nachbarn_hermetisch.rs"]
mod p35c_und_nachbarn_hermetisch;
#[path = "params_caps_ueberlauf_hermetisch.rs"]
mod params_caps_ueberlauf_hermetisch;
#[path = "scope_summe_ueberlauf_hermetisch.rs"]
mod scope_summe_ueberlauf_hermetisch;
#[path = "ueberlauf_luecke_hermetisch.rs"]
mod ueberlauf_luecke_hermetisch;
#[path = "wertwache_orakel_werte.rs"]
mod wertwache_orakel_werte;
#[path = "zugriff_ep_afa_wk_hermetisch.rs"]
mod zugriff_ep_afa_wk_hermetisch;
#[path = "zugriff_ueberlauf_hermetisch.rs"]
mod zugriff_ueberlauf_hermetisch;
