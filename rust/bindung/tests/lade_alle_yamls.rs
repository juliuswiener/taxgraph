//! Laedt alle realen Laufzeit-YAMLs (`produkt/bindung/bindung_*.yaml`, `params/<vz>/*.yaml`,
//! `params/kohorten/*.yaml`) und zaehlt sie -- 25 + 58 + 8 = 91, Stand 2026-09-29 (siehe
//! `REWRITE_PLAN.md`). Negative Tests fuer unbekannte Felder (`bindung_*.yaml`,
//! `deny_unknown_fields`) und doppelte `feld_id` (Registry) liegen mit in dieser Datei.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use bindung::{lade_kohorten, lade_params, lade_registry, BindungDatei, RegistryFehler};

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn yaml_dateien(verzeichnis: &std::path::Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(verzeichnis)
        .unwrap_or_else(|e| panic!("{} nicht lesbar: {e}", verzeichnis.display()))
        .map(|eintrag| eintrag.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yaml"))
        .collect()
}

#[test]
fn alle_bindung_yamls_laden() {
    let verzeichnis = repo_root().join("produkt").join("bindung");
    let registry = lade_registry(&verzeichnis)
        .unwrap_or_else(|e| panic!("Registry-Aufbau gescheitert: {e}"));
    assert_eq!(
        registry.dateien.len(),
        25,
        "erwartete 25 bindung_*.yaml-Dateien in {}",
        verzeichnis.display()
    );
}

#[test]
fn alle_params_yamls_laden() {
    let root = repo_root();
    let mut fehler = Vec::new();
    let mut anzahl = 0;
    for vz in ["2024", "2025", "2026"] {
        for pfad in yaml_dateien(&root.join("params").join(vz)) {
            anzahl += 1;
            if let Err(e) = lade_params(&pfad) {
                fehler.push(format!("{}: {e}", pfad.display()));
            }
        }
    }
    assert!(fehler.is_empty(), "params-Ladefehler:\n{}", fehler.join("\n"));
    assert_eq!(anzahl, 58, "erwartete 58 params/<vz>/*.yaml-Dateien");
}

#[test]
fn alle_kohorten_yamls_laden() {
    let verzeichnis = repo_root().join("params").join("kohorten");
    let mut fehler = Vec::new();
    let mut anzahl = 0;
    for pfad in yaml_dateien(&verzeichnis) {
        anzahl += 1;
        if let Err(e) = lade_kohorten(&pfad) {
            fehler.push(format!("{}: {e}", pfad.display()));
        }
    }
    assert!(fehler.is_empty(), "kohorten-Ladefehler:\n{}", fehler.join("\n"));
    assert_eq!(anzahl, 8, "erwartete 8 params/kohorten/*.yaml-Dateien");
}

#[test]
fn unbekanntes_feld_wird_abgewiesen() {
    let yaml = r#"
version: 1
scheibe: test
bindungen:
  - feld_id: testfeld
    quelle:
      regel_id: test_regel
      signatur_slot: slot
    typ: bool
    askable: false
    hilfe_kurz: "Testfeld"
    beispielwert: true
    elster_kz: "E1234567"
    vz_gueltigkeit: [2025]
    anker_ref:
      quelle: "Test-Norm"
      zitatanker: "Testzitat"
    unbekanntes_feld: 1
"#;
    let ergebnis: Result<BindungDatei, _> = serde_yaml_ng::from_str(yaml);
    assert!(ergebnis.is_err(), "unbekanntes Feld haette abgewiesen werden muessen");
}

#[test]
fn doppelte_feld_id_wird_abgewiesen() {
    let verzeichnis = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("duplicate_feld_id");
    let ergebnis = lade_registry(&verzeichnis);
    assert!(
        matches!(ergebnis, Err(RegistryFehler::DoppelteFeldId { .. })),
        "erwartete DoppelteFeldId, bekam {ergebnis:?}"
    );
}
