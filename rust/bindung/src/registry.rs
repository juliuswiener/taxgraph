//! Laedt alle `bindung_*.yaml`-Dateien eines Verzeichnisses und prueft `feld_id`-Eindeutigkeit
//! ueber Dateigrenzen hinweg. `store.py` kennt genau einen Zustand je `feld_id`; zwei Bindungen
//! auf dieselbe `feld_id` waeren eine stille Doppelbelegung, die erst beim Schreiben auffiele
//! (oder gar nicht).
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::bindung_datei::{lade_bindung, BindungDatei, BindungFehler};

/// Registry-Aufbaufehler: entweder eine einzelne Datei scheitert, oder zwei Dateien binden
/// dieselbe `feld_id`.
#[derive(Debug, thiserror::Error)]
pub enum RegistryFehler {
    #[error(transparent)]
    Bindung(#[from] BindungFehler),
    #[error("{feld_id}: doppelt gebunden in {erste} und {zweite}")]
    DoppelteFeldId {
        feld_id: String,
        erste: PathBuf,
        zweite: PathBuf,
    },
    #[error("konnte Verzeichnis {pfad} nicht lesen: {nachricht}")]
    Verzeichnis { pfad: PathBuf, nachricht: String },
}

/// Alle geladenen `bindung_*.yaml`-Dateien, nach Pfad sortiert.
#[derive(Debug)]
pub struct Registry {
    pub dateien: Vec<(PathBuf, BindungDatei)>,
}

/// Wo die Bindung liegt, die der Dienst liest: Rust-Besitz (Weg B voll, Entscheidung 2026-10-05).
/// Der Inhalt begann als byteweise Kopie von `produkt/bindung` und waechst ohne Python weiter;
/// Python liest dieses Verzeichnis nicht.
pub const BINDUNG_VERZEICHNIS: &str = "rust/bindung/daten";

/// Die `bindung_*.yaml`-Dateien eines Verzeichnisses (nicht rekursiv, alphabetisch).
/// `FELD_BESTAND.yaml` (und jede andere Datei ohne das Praefix) faellt durch den
/// Dateinamen-Filter, keine Sonderbehandlung noetig.
fn bindung_pfade(verzeichnis: &Path) -> Result<Vec<PathBuf>, RegistryFehler> {
    let mut pfade: Vec<PathBuf> = std::fs::read_dir(verzeichnis)
        .map_err(|e| RegistryFehler::Verzeichnis {
            pfad: verzeichnis.to_path_buf(),
            nachricht: e.to_string(),
        })?
        .filter_map(|eintrag| eintrag.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("bindung_"))
                && p.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml"))
        })
        .collect();
    pfade.sort();
    Ok(pfade)
}

/// Laedt jede `bindung_*.yaml` in `verzeichnis` (nicht rekursiv, alphabetisch) und prueft
/// `feld_id`-Eindeutigkeit ueber alle Dateien hinweg. `FELD_BESTAND.yaml` (und jede andere
/// Datei ohne das Praefix) faellt durch den Dateinamen-Filter, keine Sonderbehandlung noetig.
///
/// Wer wissen will, was der Dienst kennt, nimmt [`lade_registry_der_wurzel`]. Diese Funktion
/// bleibt fuer Verzeichnisse, die der Aufrufer selbst benennt: hermetische Tests mit eigenen YAMLs
/// und die Python-Orakel, deren Antworten ueber `produkt/bindung` berechnet sind.
///
/// # Errors
/// [`RegistryFehler`] beim ersten Lade-, Validierungs- oder Eindeutigkeitsfehler.
pub fn lade_registry(verzeichnis: &Path) -> Result<Registry, RegistryFehler> {
    baue(bindung_pfade(verzeichnis)?)
}

/// Die Registry, die der Dienst benutzt: alle `bindung_*.yaml` in [`BINDUNG_VERZEICHNIS`] unter
/// `wurzel`. Jeder Rust-Code, der die Bindung liest, geht hierher, nie ueber einen eigenen Pfad.
///
/// Das Verzeichnis ist Pflicht: fehlt es, ist das ein Fehler und keine leere Registry, denn ein
/// Dienst ohne Felder liefe weiter und meldete nichts.
///
/// ```
/// let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
/// let reg = bindung::lade_registry_der_wurzel(&wurzel).unwrap();
/// assert!(!reg.dateien.is_empty());
/// ```
///
/// # Errors
/// [`RegistryFehler`] beim ersten Lade-, Validierungs- oder Eindeutigkeitsfehler, auch wenn das
/// Verzeichnis fehlt.
pub fn lade_registry_der_wurzel(wurzel: &Path) -> Result<Registry, RegistryFehler> {
    lade_registry(&wurzel.join(BINDUNG_VERZEICHNIS))
}

fn baue(pfade: Vec<PathBuf>) -> Result<Registry, RegistryFehler> {
    let mut gesehen: HashMap<String, PathBuf> = HashMap::new();
    let mut dateien = Vec::with_capacity(pfade.len());
    for pfad in pfade {
        let datei = lade_bindung(&pfad)?;
        for bindung in &datei.bindungen {
            if let Some(erste) = gesehen.get(&bindung.feld_id) {
                return Err(RegistryFehler::DoppelteFeldId {
                    feld_id: bindung.feld_id.clone(),
                    erste: erste.clone(),
                    zweite: pfad,
                });
            }
            gesehen.insert(bindung.feld_id.clone(), pfad.clone());
        }
        dateien.push((pfad, datei));
    }
    Ok(Registry { dateien })
}
