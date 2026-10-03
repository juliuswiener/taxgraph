//! `parity::fallzahl` liest `PARITY_N` wirklich aus der Umgebung (die Unit-Tests im Crate pruefen nur
//! die Auswertung eines gegebenen Werts). Kein `PARITY=1` noetig: reiner Test, ein Prozess, EIN
//! `#[test]` -- die Umgebung des Prozesses gehoert diesem Test allein.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use parity::fallzahl::{holen, holen_u32, wache_gilt, ENV};

fn meldung_der_panik(wert: &str) -> String {
    std::env::set_var(ENV, wert);
    let r = std::panic::catch_unwind(|| holen("test stelle", 1200));
    let p = r.expect_err(&format!("{ENV}={wert:?} haette abbrechen muessen"));
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_default()
}

#[test]
fn die_umgebung_bestimmt_die_zahl_und_muell_bricht_ab() {
    std::env::remove_var(ENV);
    assert_eq!(
        holen("test stelle", 1200),
        1200,
        "ohne PARITY_N: der Standard"
    );
    assert_eq!(holen_u32("test stelle", 1500), 1500);

    std::env::set_var(ENV, "7");
    assert_eq!(holen("test stelle", 1200), 7, "PARITY_N=7 wird gelesen");
    assert_eq!(holen_u32("test stelle", 1500), 7);
    assert!(!wache_gilt("test wache", 7, 1200));

    std::env::set_var(ENV, "10000");
    assert_eq!(holen("test stelle", 1200), 10_000);

    std::env::set_var(ENV, "1200");
    assert!(wache_gilt("test wache", holen("test stelle", 1200), 1200));

    for muell in ["", "0", "abc", "-5", "10 000", "1e4", "7.5"] {
        let m = meldung_der_panik(muell);
        assert!(
            m.contains("test stelle") && m.contains("PARITY_N") && m.contains("keine Fallzahl"),
            "{muell:?}: Panik ohne Klartext: {m:?}"
        );
    }

    std::env::set_var(ENV, "5000000000");
    if usize::BITS > 32 {
        let r = std::panic::catch_unwind(|| holen_u32("test stelle", 1200));
        assert!(
            r.is_err(),
            "mehr als u32::MAX Faelle passen nicht in proptest"
        );
    }
    std::env::remove_var(ENV);
}
