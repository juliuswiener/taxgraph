//! Der Dienst liest `rust/bindung/felder` neben `produkt/bindung` (Weg B leicht, 2026-10-05).
//!
//! Der Lader selbst ist in `rust/bindung/tests/rust_felder_verzeichnis.rs` geprueft. Hier steht nur,
//! dass der Dienst ihn benutzt: ein Feld, das nur in `rust/bindung/felder` steht, ist im Katalog des
//! Dienstes (`Zustand::katalog`), und eine `feld_id` in beiden Verzeichnissen endet in einer
//! unerwarteten Ausnahme (500) statt in einer stillen Doppelbelegung.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;

use api::konfig::Konfig;
use api::{ApiFehler, Zustand};
use auth::Auth;

fn bindung(feld_id: &str, askable: bool) -> String {
    let frage = if askable {
        "    fragetext_laie: \"Wie hoch war der Betrag?\"\n"
    } else {
        ""
    };
    format!(
        "version: 1\nscheibe: test\nbindungen:\n  - feld_id: {feld_id}\n    quelle: {{regel_id: r, signatur_slot: s}}\n    typ: bool\n    \
         askable: {askable}\n{frage}    hilfe_kurz: T\n    beispielwert: true\n    elster_kz: \"E0123456\"\n    vz_gueltigkeit: [2025]\n    \
         anker_ref: {{quelle: Q, zitatanker: Z}}\n"
    )
}

fn schreibe(wurzel: &Path, verzeichnis: &str, datei: &str, inhalt: &str) {
    let d = wurzel.join(verzeichnis);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(datei), inhalt).unwrap();
}

fn zustand(wurzel: &Path, tmp: &Path) -> Zustand {
    let konfig = Konfig {
        wurzel: wurzel.to_path_buf(),
        faelle: tmp.join("faelle"),
        audit_dir: tmp.join("faelle"),
    };
    let auth = Auth::neu(
        "testgeheimnis".into(),
        tmp.join("users.json"),
        Some(konfig.audit_pfad()),
    );
    Zustand::neu(konfig, auth)
}

#[test]
fn feld_nur_in_rust_felder_ist_im_katalog_des_dienstes() {
    let wurzel = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    schreibe(
        wurzel.path(),
        "produkt/bindung",
        "bindung_a.yaml",
        &bindung("feld_python", false),
    );
    schreibe(
        wurzel.path(),
        "rust/bindung/felder",
        "bindung_r.yaml",
        &bindung("feld_nur_rust", true),
    );
    let katalog = zustand(wurzel.path(), tmp.path()).katalog().unwrap();
    // askable => der Sprachmodell-Schreiber darf es vorschlagen (store.py:139-162).
    assert!(katalog.erlaubt("llm", "feld_nur_rust"));
    // Kontrolle: die gemeinsame Bindung ist weiter da, und ihr nicht fragbares Feld ist es nicht.
    assert!(!katalog.erlaubt("llm", "feld_python"));
}

#[test]
fn feld_id_in_beiden_verzeichnissen_ist_eine_unerwartete_ausnahme() {
    let wurzel = tempfile::tempdir().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    schreibe(
        wurzel.path(),
        "produkt/bindung",
        "bindung_a.yaml",
        &bindung("feld_x", false),
    );
    schreibe(
        wurzel.path(),
        "rust/bindung/felder",
        "bindung_r.yaml",
        &bindung("feld_x", false),
    );
    match zustand(wurzel.path(), tmp.path()).katalog() {
        Err(ApiFehler::Unerwartet { typ, meldung }) => {
            assert_eq!(typ, "OSError");
            assert!(meldung.contains("feld_x: doppelt gebunden"), "{meldung}");
        }
        anders => panic!("{anders:?}"),
    }
}
