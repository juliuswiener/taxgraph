//! `elster::testhilfe::schemas_da`: ein fehlendes ERiC-Schema ist rot, ausser `TAXGRAPH_OHNE_XSD` ist genau `1`.
//!
//! Die CI hat kein ERiC-Schema (Lizenz) und setzt das Flag im `cargo test`-Schritt (`ci.yml`, Job `rust`; dass es dort steht,
//! prueft `bindung/tests/ci_konfiguration.rs`). Die Regel selbst, Entscheidung 2026-10-01 (Log #142), steht nur in
//! `schemas_da` und hatte auf der Rust-Seite keinen Test; auf der Python-Seite pruefen sie `tests/test_ci_konfiguration.py`
//! (`test_eric_skip_greift_nur_mit_dem_flag`) und der `conftest.py`, die mit Python fallen. Ein Rueckfall in "jeder Wert
//! ueberspringt" oder "ein fehlendes Schema ist immer gruen" liesse die CI gruen, ohne dass ein Schema je gelesen wurde.
//!
//! Der Test baut zwei ERiC-Ordner im Temp-Verzeichnis (leer, und mit beiden Schema-Dateien) und setzt `ERIC_DIR` und `HOME`
//! darauf, damit weder die Umgebung des Laeufers noch `~/02_Software/eric` mitspricht. Er ist der einzige Test dieses
//! Binaries: niemand liest die Umgebung gleichzeitig.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

const VZ: i64 = 2025;

fn ordner(wurzel: &Path, name: &str, dateien: &[&str]) -> PathBuf {
    let d = wurzel.join(name);
    std::fs::create_dir_all(&d).unwrap();
    for f in dateien {
        std::fs::write(d.join(f), "").unwrap();
    }
    d
}

/// `Ok(antwort)` oder `Err(())`, wenn `schemas_da` mit `flag` (None = nicht gesetzt) in Panik geraet.
fn frage(flag: Option<&str>) -> Result<bool, ()> {
    match flag {
        Some(f) => std::env::set_var("TAXGRAPH_OHNE_XSD", f),
        None => std::env::remove_var("TAXGRAPH_OHNE_XSD"),
    }
    catch_unwind(AssertUnwindSafe(|| elster::testhilfe::schemas_da(VZ))).map_err(|_| ())
}

#[test]
fn nur_genau_eins_erlaubt_ein_fehlendes_schema() {
    let wurzel = std::env::temp_dir().join(format!("schemas-da-flag-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&wurzel);
    let leer = ordner(&wurzel, "leer", &[]);
    let nur_e10 = ordner(&wurzel, "nur_e10", &["E10-2025.xsd"]);
    let nur_extern = ordner(&wurzel, "nur_extern", &["elster11_E10_2025_extern.xsd"]);
    let voll = ordner(
        &wurzel,
        "voll",
        &["E10-2025.xsd", "elster11_E10_2025_extern.xsd"],
    );
    std::env::set_var("HOME", ordner(&wurzel, "heim", &[]));
    // Die erwartete Panik soll das Log nicht fuellen.
    let haken = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let mut ergebnis: Vec<String> = Vec::new();
    let mut erwarte = |was: &str, ist: Result<bool, ()>, soll: Result<bool, ()>| {
        if ist != soll {
            ergebnis.push(format!("{was}: ist {ist:?}, soll {soll:?}"));
        }
    };

    // Ohne Schema: rot, ausser bei genau `1`.
    std::env::set_var("ERIC_DIR", &leer);
    erwarte("kein Schema, Flag fehlt", frage(None), Err(()));
    for flag in [
        "", "0", "true", "yes", "TRUE", " 1", "1 ", "01", "11", "1\n",
    ] {
        erwarte(
            &format!("kein Schema, Flag {flag:?}"),
            frage(Some(flag)),
            Err(()),
        );
    }
    erwarte("kein Schema, Flag 1", frage(Some("1")), Ok(false));

    // Nur eines der beiden Schemas: das gilt als fehlend, auch wenn das andere da ist.
    for (was, ordner) in [("nur E10", &nur_e10), ("nur extern", &nur_extern)] {
        std::env::set_var("ERIC_DIR", ordner);
        erwarte(&format!("{was}, Flag fehlt"), frage(None), Err(()));
        erwarte(&format!("{was}, Flag 1"), frage(Some("1")), Ok(false));
    }

    // Beide da: wahr, das Flag spielt keine Rolle.
    std::env::set_var("ERIC_DIR", &voll);
    for flag in [None, Some("0"), Some("1")] {
        erwarte(
            &format!("beide Schemas, Flag {flag:?}"),
            frage(flag),
            Ok(true),
        );
    }

    std::panic::set_hook(haken);
    let _ = std::fs::remove_dir_all(&wurzel);
    assert!(ergebnis.is_empty(), "{}", ergebnis.join("\n"));
}
