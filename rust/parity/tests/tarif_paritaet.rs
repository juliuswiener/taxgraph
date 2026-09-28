//! Rust (`catala-sys`, C-Backend) gegen Python (`tools/parity/oracle.py`, ueber
//! `produkt/engine/runner.py`) fuer den § 32a-Tarif. `REWRITE_PLAN.md` §5/§7 ("Tarif-Paritaet
//! 24 327 Punkte").
//!
//! Braucht den Catala-Opam-Switch + `python3` mit dem Repo-Umfeld (`produkt/`, `oracle/`) --
//! in CI standardmaessig SKIP, lokal erzwingen:
//!
//!   `PARITY`=1 `cargo` test -p parity --test `tarif_paritaet` -- --nocapture
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use catala_sys::{grundtarif, splittingtarif, Vz};
use parity::{diff_grundtarif, diff_splittingtarif, Oracle};
use proptest::prelude::*;

/// Zonen-Obergrenzen aus `rules/estg/p32a/einkommensteuertarif.catala_en`
/// (Grundfreibetrag, `zone2_obergrenze`, `zone3_obergrenze`, `zone4_obergrenze`), in EURO.
const GRENZEN: [(u16, [i64; 4]); 3] = [
    (2024, [11_784, 17_005, 66_760, 277_825]),
    (2025, [12_096, 17_443, 68_480, 277_825]),
    (2026, [12_348, 17_799, 69_878, 277_825]),
];

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip_ohne_parity_env() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn vz_of(jahr: u16) -> Vz {
    Vz::try_from(jahr).expect("nur 2024/2025/2026 in GRENZEN")
}

#[test]
fn tarif_paritaet_dense_sweep() {
    if skip_ohne_parity_env() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht Catala-Toolchain + Python-Umfeld)");
        return;
    }
    let mut oracle = Oracle::spawn(&repo_root()).expect("oracle.py startet");
    let mut verglichen = 0u64;
    let mut abweichungen = Vec::new();

    for &(jahr, _) in &GRENZEN {
        let vz = vz_of(jahr);
        let mut zve_eur = 0i64;
        while zve_eur <= 300_000 {
            let zve_cent = zve_eur * 100;
            let rust_cent = grundtarif(zve_cent, vz).expect("Grundtarif-Scope laeuft durch");
            if let Some(a) = diff_grundtarif(&mut oracle, zve_cent, jahr, rust_cent)
                .expect("Orakel-Aufruf laeuft durch")
            {
                abweichungen.push(a);
            }
            verglichen += 1;
            zve_eur += 37;
        }
    }

    eprintln!("tarif_paritaet_dense_sweep: {verglichen} Punkte verglichen, {} Abweichungen", abweichungen.len());
    assert_eq!(verglichen, 24_327, "Sweep-Groesse muss REWRITE_PLAN.md §7s Zielzahl treffen");
    assert!(abweichungen.is_empty(), "Abweichungen: {abweichungen:?}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn tarif_paritaet_proptest_grenzfaelle(zve_eur in zve_strategy()) {
        if skip_ohne_parity_env() {
            return Ok(());
        }
        let mut oracle = Oracle::spawn(&repo_root()).expect("oracle.py startet");
        for &(jahr, _) in &GRENZEN {
            let vz = vz_of(jahr);
            let zve_cent = zve_eur * 100;

            let rust_grund = grundtarif(zve_cent, vz).expect("Grundtarif-Scope laeuft durch");
            let a = diff_grundtarif(&mut oracle, zve_cent, jahr, rust_grund)
                .expect("Orakel-Aufruf laeuft durch");
            prop_assert_eq!(a, None);

            let rust_split = splittingtarif(zve_cent, vz).expect("Splittingtarif-Scope laeuft durch");
            let a = diff_splittingtarif(&mut oracle, zve_cent, jahr, rust_split)
                .expect("Orakel-Aufruf laeuft durch");
            prop_assert_eq!(a, None);
        }
    }
}

fn zve_strategy() -> impl Strategy<Value = i64> {
    let grenzwerte: Vec<i64> = GRENZEN
        .iter()
        .flat_map(|(_, g)| g.iter().flat_map(|&x| [x - 1, x, x + 1]))
        .collect();
    prop_oneof![
        3 => -10_000i64..=400_000i64,
        1 => prop::sample::select(grenzwerte),
    ]
}

/// Beweist, dass der Diff-Mechanismus selbst eine Abweichung findet: EIN Rust-Ergebnis wird
/// um 1 Cent verschoben, der Rest bleibt echt. Ohne diesen Test koennte `diff_grundtarif`
/// kaputt sein (z. B. immer `None` liefern) und der Sweep oben liefe trotzdem gruen durch.
#[test]
fn negativkontrolle_erkennt_genau_eine_abweichung() {
    if skip_ohne_parity_env() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht Catala-Toolchain + Python-Umfeld)");
        return;
    }
    let mut oracle = Oracle::spawn(&repo_root()).expect("oracle.py startet");
    let vz = vz_of(2025);
    let mut abweichungen = Vec::new();

    for i in 0..50i64 {
        let zve_cent = i * 500_000; // 0, 5000, 10000, ... EUR
        let mut rust_cent = grundtarif(zve_cent, vz).expect("Grundtarif-Scope laeuft durch");
        if i == 25 {
            rust_cent += 1; // die eine, absichtliche Abweichung
        }
        if let Some(a) = diff_grundtarif(&mut oracle, zve_cent, 2025, rust_cent)
            .expect("Orakel-Aufruf laeuft durch")
        {
            abweichungen.push(a);
        }
    }

    eprintln!("negativkontrolle: {} Abweichungen gefunden (erwartet: 1)", abweichungen.len());
    assert_eq!(abweichungen.len(), 1, "Kontrollprobe muss GENAU eine Abweichung finden");
}
