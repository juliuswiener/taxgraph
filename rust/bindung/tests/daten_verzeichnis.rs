//! `lade_registry_der_wurzel`: der Dienst liest `rust/bindung/daten` und nichts sonst. Weg B voll,
//! Entscheidung 2026-10-05; das Verzeichnis begann als Kopie von `produkt/bindung` und gehoert Rust.
//!
//! Was hier schiefgehen kann und wen es trifft:
//!
//! - Liest der Lader wieder `produkt/bindung`, sieht der Dienst kein Feld, das nur Rust kennt, und
//!   meldet nichts. Ein hermetischer Test legt ein Feld nur in eines der beiden Verzeichnisse
//!   (`nur_das_rust_verzeichnis_zaehlt`); der letzte Test haelt fest, dass die echte Registry aus
//!   `rust/bindung/daten` stammt.
//! - Dieselbe `feld_id` in zwei Dateien ueberschriebe still den Zustand in `store`.
//! - Fehlte das Verzeichnis, liefe der Dienst ohne Felder und meldete nichts: es ist Pflicht.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use bindung::{lade_registry_der_wurzel, RegistryFehler, BINDUNG_VERZEICHNIS};

/// Das Verzeichnis, das Python liest. Hier nur als Attrappe: der Lader darf es nicht ansehen.
const PYTHON_ATTRAPPE: &str = "produkt/bindung";

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
         askable: false\n    hilfe_kurz: Tipp\n    beispielwert: true\n    elster_kz: \"E0123456\"\n    vz_gueltigkeit: [2025]\n    \
         anker_ref: {{quelle: Q, zitatanker: Zit}}\n"
    )
}

fn feld_ids(r: &bindung::Registry) -> Vec<&str> {
    r.dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter().map(|b| b.feld_id.as_str()))
        .collect()
}

#[test]
fn nur_das_rust_verzeichnis_zaehlt() {
    let w = wurzel("nur-rust");
    schreibe(
        &w,
        BINDUNG_VERZEICHNIS,
        "bindung_b.yaml",
        &gueltig("feld_rust_b"),
    );
    schreibe(
        &w,
        BINDUNG_VERZEICHNIS,
        "bindung_a.yaml",
        &gueltig("feld_rust_a"),
    );
    // Attrappe: ein gueltiges Feld im Verzeichnis, das nur noch Python liest.
    schreibe(
        &w,
        PYTHON_ATTRAPPE,
        "bindung_p.yaml",
        &gueltig("feld_nur_python"),
    );
    let r = lade_registry_der_wurzel(&w).unwrap();
    // Nach Dateiname sortiert, und das Python-Feld fehlt.
    assert_eq!(feld_ids(&r), ["feld_rust_a", "feld_rust_b"]);
    std::fs::remove_dir_all(&w).unwrap();
}

#[test]
fn ohne_das_verzeichnis_ist_es_ein_fehler_auch_wenn_python_eines_hat() {
    let w = wurzel("ohne-daten");
    schreibe(
        &w,
        PYTHON_ATTRAPPE,
        "bindung_p.yaml",
        &gueltig("feld_nur_python"),
    );
    match lade_registry_der_wurzel(&w).unwrap_err() {
        RegistryFehler::Verzeichnis { pfad, .. } => {
            assert!(pfad.ends_with(BINDUNG_VERZEICHNIS), "{}", pfad.display());
        }
        anderer => panic!("falscher Fehler: {anderer}"),
    }
    std::fs::remove_dir_all(&w).unwrap();
}

#[test]
fn feld_id_in_zwei_dateien_wird_abgewiesen() {
    let w = wurzel("doppelte-id");
    schreibe(
        &w,
        BINDUNG_VERZEICHNIS,
        "bindung_a.yaml",
        &gueltig("feld_x"),
    );
    schreibe(
        &w,
        BINDUNG_VERZEICHNIS,
        "bindung_b.yaml",
        &gueltig("feld_x"),
    );
    match lade_registry_der_wurzel(&w).unwrap_err() {
        RegistryFehler::DoppelteFeldId {
            feld_id,
            erste,
            zweite,
        } => {
            assert_eq!(feld_id, "feld_x");
            assert!(erste.ends_with("rust/bindung/daten/bindung_a.yaml"));
            assert!(zweite.ends_with("rust/bindung/daten/bindung_b.yaml"));
        }
        anderer => panic!("falscher Fehler: {anderer}"),
    }
    std::fs::remove_dir_all(&w).unwrap();
}

/// `deny_unknown_fields`: ein Tippfehler im Schluessel ist ein Fehler mit dem Pfad der Datei,
/// kein ignoriertes Feld.
#[test]
fn datei_mit_unbekanntem_schluessel_nennt_ihren_pfad() {
    let w = wurzel("unbekannt");
    schreibe(
        &w,
        BINDUNG_VERZEICHNIS,
        "bindung_a.yaml",
        &gueltig("feld_a"),
    );
    let kaputt = format!("{}    askabel: true\n", gueltig("feld_r"));
    schreibe(&w, BINDUNG_VERZEICHNIS, "bindung_r.yaml", &kaputt);
    let fehler = lade_registry_der_wurzel(&w).unwrap_err().to_string();
    assert!(
        fehler.contains("rust/bindung/daten/bindung_r.yaml"),
        "{fehler}"
    );
    assert!(fehler.contains("askabel"), "{fehler}");
    std::fs::remove_dir_all(&w).unwrap();
}

/// Dateien ohne Praefix `bindung_` (`README.md`, `FELD_BESTAND.yaml`, `schema.json`) laedt der
/// Lader nicht, auch wenn sie keine Bindung sind.
#[test]
fn dateien_ohne_praefix_werden_nicht_als_bindung_geladen() {
    let w = wurzel("readme");
    schreibe(
        &w,
        BINDUNG_VERZEICHNIS,
        "bindung_a.yaml",
        &gueltig("feld_a"),
    );
    schreibe(&w, BINDUNG_VERZEICHNIS, "README.md", "kein YAML: [\n");
    schreibe(
        &w,
        BINDUNG_VERZEICHNIS,
        "FELD_BESTAND.yaml",
        "kein: [Bindung\n",
    );
    let r = lade_registry_der_wurzel(&w).unwrap();
    assert_eq!(feld_ids(&r), ["feld_a"]);
    std::fs::remove_dir_all(&w).unwrap();
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Das echte Repo: jede geladene Datei liegt unter `rust/bindung/daten`, keine unter
/// `produkt/bindung`. Liest der Lader wieder das Python-Verzeichnis, schlaegt genau das an.
#[test]
fn echte_registry_stammt_aus_rust_bindung_daten() {
    let w = repo_root();
    let daten = w.join(BINDUNG_VERZEICHNIS).canonicalize().unwrap();
    let r = lade_registry_der_wurzel(&w).unwrap();
    assert!(
        r.dateien.len() >= 25,
        "nur {} Dateien geladen",
        r.dateien.len()
    );
    for (pfad, _) in &r.dateien {
        let pfad = pfad.canonicalize().unwrap();
        assert!(
            pfad.starts_with(&daten),
            "{} liegt nicht unter {}",
            pfad.display(),
            daten.display()
        );
    }
}
