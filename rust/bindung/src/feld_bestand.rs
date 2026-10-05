//! `FELD_BESTAND.yaml`: die Ratsche gegen stillen Verlust von Nutzerangaben.
//!
//! Verschwindet eine `feld_id` aus der Bindung, verschwinden damit die Angaben, die Nutzer zu
//! diesem Feld gemacht haben -- aus der Oberflaeche, aus der Deklaration, aus dem Ergebnis. Die
//! Events bleiben im Store liegen und wirken nicht mehr. Kein Absturz, keine Meldung.
//!
//! Die Datei sagt nur, welche `feld_id`s es EINMAL gab (`felder`) und welche bewusst entfernt
//! wurden (`entfernt`, mit Begruendung). Sie ist keine zweite Wahrheit ueber die Bindung: neue
//! Felder brauchen keinen Eintrag, die Ratsche kennt nur eine Richtung. Die Pruefung selbst
//! steht in `rust/bindung/tests/feld_bestand.rs`; hier liegen der Lader und die reinen
//! Vergleichsfunktionen, damit die Selbstprobe auf erfundenen Daten laufen kann.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::registry::BINDUNG_VERZEICHNIS;

/// Dateiname im Bindungsverzeichnis. Ohne das Praefix `bindung_`, deshalb laedt
/// `lade_registry` sie nicht als Bindung.
pub const FELD_BESTAND_DATEI: &str = "FELD_BESTAND.yaml";

/// Eine Begruendung unter `entfernt` ist nur eine, wenn sie laenger ist als das: sie muss auch
/// sagen, was aus bereits erfassten Angaben wird, nicht nur, warum das Feld weg ist.
pub const MIN_ZEICHEN_BEGRUENDUNG: usize = 30;

/// Inhalt von `FELD_BESTAND.yaml`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeldBestand {
    /// Alle `feld_id`s, die bei der Erfassung in der Bindung standen.
    #[serde(default)]
    pub felder: Vec<String>,
    /// Bewusst entfernte `feld_id`s mit Begruendung.
    #[serde(default)]
    pub entfernt: BTreeMap<String, String>,
}

/// Fehler beim Lesen von `FELD_BESTAND.yaml`.
#[derive(Debug, thiserror::Error)]
pub enum FeldBestandFehler {
    #[error("konnte {pfad} nicht lesen: {nachricht}")]
    Io { pfad: PathBuf, nachricht: String },
    #[error("YAML-Fehler in {pfad}: {nachricht}")]
    Yaml { pfad: PathBuf, nachricht: String },
}

/// Laedt `FELD_BESTAND.yaml` aus `verzeichnis`. Die Datei ist Pflicht: fehlte sie, liefe die
/// Ratsche gegen eine leere Liste und meldete nichts.
///
/// # Errors
/// [`FeldBestandFehler`], wenn die Datei fehlt oder kein gueltiges YAML dieser Form ist.
pub fn lade_feld_bestand(verzeichnis: &Path) -> Result<FeldBestand, FeldBestandFehler> {
    let pfad = verzeichnis.join(FELD_BESTAND_DATEI);
    let text = std::fs::read_to_string(&pfad).map_err(|e| FeldBestandFehler::Io {
        pfad: pfad.clone(),
        nachricht: e.to_string(),
    })?;
    serde_yaml_ng::from_str(&text).map_err(|e| FeldBestandFehler::Yaml {
        pfad,
        nachricht: e.to_string(),
    })
}

/// Der Bestand, der zur Registry des Dienstes gehoert: `FELD_BESTAND.yaml` in
/// [`BINDUNG_VERZEICHNIS`] unter `wurzel`.
///
/// # Errors
/// [`FeldBestandFehler`], wenn die Datei fehlt oder kein gueltiges YAML dieser Form ist.
pub fn lade_feld_bestand_der_wurzel(wurzel: &Path) -> Result<FeldBestand, FeldBestandFehler> {
    lade_feld_bestand(&wurzel.join(BINDUNG_VERZEICHNIS))
}

/// Was einmal erfasst war, heute fehlt und auch nicht unter `entfernt` steht, sortiert.
/// Das ist der Kern der Ratsche: jedes Element ist ein stiller Verlust.
#[must_use]
pub fn verschwundene(
    erfasst: &BTreeSet<String>,
    heute: &BTreeSet<String>,
    entfernt: &BTreeMap<String, String>,
) -> Vec<String> {
    erfasst
        .iter()
        .filter(|f| !heute.contains(*f) && !entfernt.contains_key(*f))
        .cloned()
        .collect()
}

/// Tote Eintraege: Felder unter `entfernt`, die heute wieder in der Bindung stehen. Ein solcher
/// Eintrag deckte beim naechsten Mal ein echtes Verschwinden mit ab.
#[must_use]
pub fn wiederaufgetauchte(
    heute: &BTreeSet<String>,
    entfernt: &BTreeMap<String, String>,
) -> Vec<String> {
    entfernt
        .keys()
        .filter(|f| heute.contains(*f))
        .cloned()
        .collect()
}

/// Eintraege unter `entfernt`, deren Begruendung kuerzer oder gleich [`MIN_ZEICHEN_BEGRUENDUNG`]
/// Zeichen ist (gezaehlt nach Zeichen, nicht nach Bytes).
#[must_use]
pub fn zu_knappe_begruendungen(entfernt: &BTreeMap<String, String>) -> Vec<String> {
    entfernt
        .iter()
        .filter(|(_, grund)| grund.chars().count() <= MIN_ZEICHEN_BEGRUENDUNG)
        .map(|(feld, _)| feld.clone())
        .collect()
}
