//! `params/kohorten/*.yaml`: Alters-/Jahrgangs-Staffeltabellen, die ueber mehrere
//! Veranlagungszeitraeume hinweg gelten (anders als `params/<vz>/*.yaml`, das genau einen VZ
//! bindet). Alle 8 Dateien teilen denselben Kopf (`parameter`, `authority`, `redistributable`,
//! `rechtsquelle`, `datenquelle`, `einheit`) -- Stichprobe ueber alle 8 bestaetigt das
//! ausnahmslos. `rechtsquelle` und `einheit` sind hier selbst Objekte (nicht wie im
//! Parameter-Koerper einzelne Strings), deshalb `Value` statt `String`.
//!
//! Der Koerper traegt einen von drei Namen -- `kohorten` (6x, teils Integer- teils
//! String-Schluessel), `fenster` (`degressive_afa_fenster_p7.yaml`) oder `staffel`
//! (`ekfz_sonderafa_staffel_p7.yaml`) -- deshalb wie bei `ParamsDatei` `#[serde(flatten)]` in
//! eine offene Map statt eines benannten Felds.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_yaml_ng::Value;

use crate::params_datei::Authority;

/// `KohortenDatei`-Parse-Fehler. Eigener Typ statt Wiederverwendung von `ParamsFehler`, obwohl
/// beide Varianten identisch aussehen -- Kohorten- und Jahresparameter sind fachlich getrennte
/// Bereiche (eigene Datei-Konvention, eigene Gueltigkeitsregel), nur die Fehlerform ist zufaellig
/// gleich.
#[derive(Debug, thiserror::Error)]
pub enum KohortenFehler {
    #[error("konnte {pfad} nicht lesen: {nachricht}")]
    Io { pfad: PathBuf, nachricht: String },
    #[error("YAML-Fehler in {pfad}: {nachricht}")]
    Yaml { pfad: PathBuf, nachricht: String },
}

/// Eine `params/kohorten/*.yaml`-Datei: typisierter Kopf, offener Koerper (siehe Moduldoku).
#[derive(Debug, Clone, Deserialize)]
pub struct KohortenDatei {
    pub parameter: String,
    pub authority: Authority,
    pub redistributable: bool,
    pub rechtsquelle: Value,
    pub datenquelle: String,
    pub einheit: Value,
    #[serde(flatten)]
    pub koerper: BTreeMap<String, Value>,
}

/// Laedt eine `params/kohorten/*.yaml`-Datei.
///
/// # Errors
/// [`KohortenFehler`] bei I/O- oder YAML-Fehlern.
pub fn lade_kohorten(pfad: &Path) -> Result<KohortenDatei, KohortenFehler> {
    let text = std::fs::read_to_string(pfad).map_err(|e| KohortenFehler::Io {
        pfad: pfad.to_path_buf(),
        nachricht: e.to_string(),
    })?;
    serde_yaml_ng::from_str(&text).map_err(|e| KohortenFehler::Yaml {
        pfad: pfad.to_path_buf(),
        nachricht: e.to_string(),
    })
}
