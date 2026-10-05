//! `lade_registry_der_wurzel`: der Dienst liest `produkt/bindung` (gemeinsam mit Python, eingefroren)
//! und `rust/bindung/felder` (nur Rust). Weg B leicht, Entscheidung 2026-10-05.
//!
//! Was hier schiefgehen kann und wen es trifft:
//!
//! - Ein Rust-Feld mit der `feld_id` eines Python-Felds ueberschriebe still den Zustand in `store`.
//! - Derselbe Dateiname in beiden Verzeichnissen verdeckte eine Datei im Dienst, der die Felder je
//!   Dateiname fuehrt (`bindung_n_vor_gwg.yaml`).
//! - Fehlte `rust/bindung/felder` im Repo, liefe der Dienst ohne Rust-Felder und meldete nichts. Der
//!   Lader erlaubt das (ein Checkout ohne Rust-Felder ist gueltig), also haelt der letzte Test fest,
//!   dass das Verzeichnis im Repo steht.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use bindung::{
    lade_registry, lade_registry_der_wurzel, RegistryFehler, PYTHON_BINDUNG, RUST_FELDER,
};

fn wurzel(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("bindung-wurzel-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn schreibe(wurzel: &Path, verzeichnis: &str, datei: &str, inhalt: &str) {
    let d = wurzel.join(verzeichnis);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(datei), inhalt).unwrap();
}

fn gueltig(feld_id: &str) -> String {
    format!(
        "version: 1\nscheibe: test\nbindungen:\n  - feld_id: {feld_id}\n    quelle: {{regel_id: r, signatur_slot: s}}\n    typ: bool\n    \
         askable: false\n    hilfe_kurz: T\n    beispielwert: true\n    elster_kz: \"E0123456\"\n    vz_gueltigkeit: [2025]\n    \
         anker_ref: {{quelle: Q, zitatanker: Z}}\n"
    )
}

fn namen(r: &bindung::Registry) -> Vec<String> {
    r.dateien
        .iter()
        .map(|(p, _)| p.file_name().unwrap().to_str().unwrap().to_owned())
        .collect()
}

#[test]
fn beide_verzeichnisse_gemeinsame_bindung_zuerst() {
    let w = wurzel("beide");
    schreibe(&w, PYTHON_BINDUNG, "bindung_z.yaml", &gueltig("feld_py"));
    schreibe(&w, RUST_FELDER, "bindung_a.yaml", &gueltig("feld_rust"));
    let r = lade_registry_der_wurzel(&w).unwrap();
    // Reihenfolge: erst `produkt/bindung`, dann `rust/bindung/felder`, nicht eine gemeinsame Sortierung.
    assert_eq!(namen(&r), ["bindung_z.yaml", "bindung_a.yaml"]);
    let ids: Vec<&str> = r
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter().map(|b| b.feld_id.as_str()))
        .collect();
    assert_eq!(ids, ["feld_py", "feld_rust"]);
    std::fs::remove_dir_all(&w).unwrap();
}

#[test]
fn ohne_rust_verzeichnis_gilt_die_gemeinsame_bindung_allein() {
    let w = wurzel("ohne-rust");
    schreibe(&w, PYTHON_BINDUNG, "bindung_a.yaml", &gueltig("feld_py"));
    let r = lade_registry_der_wurzel(&w).unwrap();
    assert_eq!(namen(&r), ["bindung_a.yaml"]);
    std::fs::remove_dir_all(&w).unwrap();
}

#[test]
fn ohne_gemeinsame_bindung_ist_es_ein_fehler_auch_mit_rust_verzeichnis() {
    let w = wurzel("ohne-python");
    schreibe(&w, RUST_FELDER, "bindung_a.yaml", &gueltig("feld_rust"));
    match lade_registry_der_wurzel(&w).unwrap_err() {
        RegistryFehler::Verzeichnis { pfad, .. } => assert!(pfad.ends_with(PYTHON_BINDUNG)),
        anderer => panic!("falscher Fehler: {anderer}"),
    }
    std::fs::remove_dir_all(&w).unwrap();
}

#[test]
fn feld_id_in_beiden_verzeichnissen_wird_abgewiesen() {
    let w = wurzel("doppelte-id");
    schreibe(&w, PYTHON_BINDUNG, "bindung_a.yaml", &gueltig("feld_x"));
    schreibe(&w, RUST_FELDER, "bindung_b.yaml", &gueltig("feld_x"));
    match lade_registry_der_wurzel(&w).unwrap_err() {
        RegistryFehler::DoppelteFeldId {
            feld_id,
            erste,
            zweite,
        } => {
            assert_eq!(feld_id, "feld_x");
            assert!(erste.ends_with("produkt/bindung/bindung_a.yaml"));
            assert!(zweite.ends_with("rust/bindung/felder/bindung_b.yaml"));
        }
        anderer => panic!("falscher Fehler: {anderer}"),
    }
    std::fs::remove_dir_all(&w).unwrap();
}

#[test]
fn dateiname_in_beiden_verzeichnissen_wird_abgewiesen() {
    let w = wurzel("doppelter-name");
    schreibe(&w, PYTHON_BINDUNG, "bindung_a.yaml", &gueltig("feld_py"));
    schreibe(&w, RUST_FELDER, "bindung_a.yaml", &gueltig("feld_rust"));
    match lade_registry_der_wurzel(&w).unwrap_err() {
        RegistryFehler::DoppelterDateiname {
            name,
            erste,
            zweite,
        } => {
            assert_eq!(name, "bindung_a.yaml");
            assert!(erste.ends_with("produkt/bindung/bindung_a.yaml"));
            assert!(zweite.ends_with("rust/bindung/felder/bindung_a.yaml"));
        }
        anderer => panic!("falscher Fehler: {anderer}"),
    }
    std::fs::remove_dir_all(&w).unwrap();
}

/// `deny_unknown_fields` gilt fuer die Rust-Dateien wie fuer die gemeinsamen: ein Tippfehler im
/// Schluessel ist ein Fehler mit dem Pfad der Datei, kein ignoriertes Feld.
#[test]
fn rust_datei_mit_unbekanntem_schluessel_nennt_ihren_pfad() {
    let w = wurzel("unbekannt");
    schreibe(&w, PYTHON_BINDUNG, "bindung_a.yaml", &gueltig("feld_py"));
    let kaputt = format!("{}    askabel: true\n", gueltig("feld_rust"));
    schreibe(&w, RUST_FELDER, "bindung_r.yaml", &kaputt);
    let fehler = lade_registry_der_wurzel(&w).unwrap_err().to_string();
    assert!(fehler.contains("rust/bindung/felder/bindung_r.yaml"), "{fehler}");
    assert!(fehler.contains("askabel"), "{fehler}");
    std::fs::remove_dir_all(&w).unwrap();
}

/// Dateien ohne Praefix `bindung_` (die README des Verzeichnisses) lädt der Lader nicht.
#[test]
fn readme_im_rust_verzeichnis_wird_nicht_als_bindung_geladen() {
    let w = wurzel("readme");
    schreibe(&w, PYTHON_BINDUNG, "bindung_a.yaml", &gueltig("feld_py"));
    schreibe(&w, RUST_FELDER, "README.md", "kein YAML: [\n");
    let r = lade_registry_der_wurzel(&w).unwrap();
    assert_eq!(namen(&r), ["bindung_a.yaml"]);
    std::fs::remove_dir_all(&w).unwrap();
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Das echte Repo: `rust/bindung/felder` steht im Baum, die Registry ist die gemeinsame Bindung
/// plus die Rust-Dateien, und `feld_id` ist ueber beide eindeutig (der Lader prueft das beim Bauen).
#[test]
fn echtes_repo_traegt_das_rust_verzeichnis_und_laedt_beides() {
    let w = repo_root();
    assert!(
        w.join(RUST_FELDER).is_dir(),
        "{RUST_FELDER} fehlt im Repo: der Dienst liefe ohne Rust-Felder und meldete nichts"
    );
    let gemeinsam = lade_registry(&w.join(PYTHON_BINDUNG)).unwrap();
    let rust = lade_registry(&w.join(RUST_FELDER)).unwrap();
    let alles = lade_registry_der_wurzel(&w).unwrap();
    assert_eq!(
        alles.dateien.len(),
        gemeinsam.dateien.len() + rust.dateien.len()
    );
    for ((a, _), (b, _)) in gemeinsam.dateien.iter().zip(&alles.dateien) {
        assert_eq!(a, b, "die gemeinsame Bindung steht vorn, in gleicher Reihenfolge");
    }
}
