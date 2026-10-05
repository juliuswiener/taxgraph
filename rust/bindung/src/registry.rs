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
    /// Der Dienst fuehrt die Felder je Dateiname (`bindung_n_vor_gwg.yaml` ist ein Schluessel);
    /// derselbe Name in `produkt/bindung` und `rust/bindung/felder` wuerde einen verdecken.
    #[error("Dateiname {name} liegt in beiden Verzeichnissen: {erste} und {zweite}")]
    DoppelterDateiname {
        name: String,
        erste: PathBuf,
        zweite: PathBuf,
    },
}

/// Alle geladenen `bindung_*.yaml`-Dateien: je Verzeichnis nach Pfad sortiert, die Verzeichnisse
/// in der Reihenfolge, in der sie geladen wurden.
#[derive(Debug)]
pub struct Registry {
    pub dateien: Vec<(PathBuf, BindungDatei)>,
}

/// Wo die gemeinsame Bindung liegt, die Python und Rust lesen (eingefroren).
pub const PYTHON_BINDUNG: &str = "produkt/bindung";

/// Wo die Felder liegen, die nur Rust kennt (Weg B leicht, Entscheidung 2026-10-05). Python liest
/// dieses Verzeichnis nicht.
pub const RUST_FELDER: &str = "rust/bindung/felder";

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
/// Das ist die Registry der **gemeinsamen** Bindung. Wer wissen will, was der Dienst kennt, nimmt
/// [`lade_registry_der_wurzel`]; Tests gegen die Python-Orakel bleiben bei dieser Funktion.
///
/// # Errors
/// [`RegistryFehler`] beim ersten Lade-, Validierungs- oder Eindeutigkeitsfehler.
pub fn lade_registry(verzeichnis: &Path) -> Result<Registry, RegistryFehler> {
    baue(bindung_pfade(verzeichnis)?)
}

/// Die Registry, die der Dienst benutzt: `produkt/bindung` (Pflicht) und `rust/bindung/felder`
/// (die Felder nur fuer Rust). `feld_id` und Dateiname sind ueber beide Verzeichnisse eindeutig.
///
/// Fehlt `rust/bindung/felder`, gilt die gemeinsame Bindung allein: ein Checkout ohne Rust-Felder
/// ist gueltig, nur ein fehlendes `produkt/bindung` ist ein Fehler.
///
/// ```
/// let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
/// let reg = bindung::lade_registry_der_wurzel(&wurzel).unwrap();
/// assert!(!reg.dateien.is_empty());
/// ```
///
/// # Errors
/// [`RegistryFehler`] beim ersten Lade-, Validierungs- oder Eindeutigkeitsfehler;
/// [`RegistryFehler::DoppelterDateiname`], wenn ein Dateiname in beiden Verzeichnissen steht.
pub fn lade_registry_der_wurzel(wurzel: &Path) -> Result<Registry, RegistryFehler> {
    let mut pfade = bindung_pfade(&wurzel.join(PYTHON_BINDUNG))?;
    let rust = wurzel.join(RUST_FELDER);
    if rust.is_dir() {
        for pfad in bindung_pfade(&rust)? {
            let name = pfad.file_name();
            if let Some(erste) = pfade.iter().find(|p| p.file_name() == name) {
                return Err(RegistryFehler::DoppelterDateiname {
                    name: name.map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                    erste: erste.clone(),
                    zweite: pfad,
                });
            }
            pfade.push(pfad);
        }
    }
    baue(pfade)
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
