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

/// Alle geladenen `bindung_*.yaml`-Dateien, sortiert nach Pfad.
#[derive(Debug)]
pub struct Registry {
    pub dateien: Vec<(PathBuf, BindungDatei)>,
}

/// Laedt jede `bindung_*.yaml` in `verzeichnis` (nicht rekursiv, alphabetisch) und prueft
/// `feld_id`-Eindeutigkeit ueber alle Dateien hinweg. `FELD_BESTAND.yaml` (und jede andere
/// Datei ohne das Praefix) faellt durch den Dateinamen-Filter, keine Sonderbehandlung noetig.
///
/// # Errors
/// [`RegistryFehler`] beim ersten Lade-, Validierungs- oder Eindeutigkeitsfehler.
pub fn lade_registry(verzeichnis: &Path) -> Result<Registry, RegistryFehler> {
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
