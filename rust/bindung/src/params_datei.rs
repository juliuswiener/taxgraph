//! `params/<vz>/*.yaml`: Jahreswerte (Freibetraege, Pauschbetraege, Hoechstbetraege,
//! Tarifkonstanten), getrennt von den Formeln, ein Verzeichnis je Veranlagungszeitraum
//! (`params/schema.md`, Muster nach `OpenFisca`: "Parameter als Daten").
//!
//! Anders als `bindung_*.yaml` gibt es fuer diese Dateiklasse KEIN `schema.json`, nur
//! `params/schema.md` als Prosa-Dokumentation. Eine Stichprobe ueber alle 58 Dateien (19x
//! 2024, 19x 2025, 20x 2026) zeigt: der Kopf (`veranlagungszeitraum`, `authority`,
//! `redistributable`, `gueltig_ab`; `parameter` fehlt in genau einer Datei je Jahrgang,
//! `einkommensteuertarif_p32a.yaml`) ist ausnahmslos vorhanden -- der KOERPER dagegen nicht
//! einheitlich: geschachtelte Staffeln mit Integer-Schluesseln
//! (`behinderten_pauschbetrag_p33b.yaml`: `gdb_staffel: {20: ..., 30: ..., ...}`),
//! `_quelle`-Eintraege mit `datei`/`zitatanker` statt `wert`, nackte nicht-Objekt-Werte
//! (`fahrtkostenpauschale_p33_2a.yaml`: `pauschale_900: 900`) und ein `werte:`-Unterobjekt
//! mit sieben benannten Werten (`riester_p8x.yaml`). Ein einzelnes striktes
//! `deny_unknown_fields`-Schema fuer den Koerper wuerde reale, gueltige Dateien zurueckweisen.
//!
//! Ponytail: nur der Kopf ist typisiert, der Koerper bleibt `#[serde(flatten)]` in eine offene
//! `Value`-Map. Upgrade-Pfad: sobald jede Koerperform (einfacher Parameter, Staffel, Quelle-Verweis,
//! benanntes Werte-Bündel) benannt ist, je Form eine eigene Variante schneiden.
//!
//! `Value` ist hier `serde_yaml_ng::Value`, nicht `serde_json::Value`: die Integer-Schluessel
//! der Staffeln (`gdb_staffel`, `kohorten`) sind in JSON gar nicht darstellbar (JSON-Objekte
//! kennen nur String-Schluessel) -- `serde_json::Value` scheitert an ihnen mit "expected a
//! string key", `serde_yaml_ng::Value` nicht.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_yaml_ng::Value;

/// `params_datei`/`kohorten_datei`-Parse-Fehler.
#[derive(Debug, thiserror::Error)]
pub enum ParamsFehler {
    #[error("konnte {pfad} nicht lesen: {nachricht}")]
    Io { pfad: PathBuf, nachricht: String },
    #[error("YAML-Fehler in {pfad}: {nachricht}")]
    Yaml { pfad: PathBuf, nachricht: String },
}

/// Quellenklasse eines Parameters (`params/schema.md`: "Quellenmodell"). Alle 58 Dateien
/// tragen aktuell `gesetz`; die anderen drei sind dokumentiert, nicht (noch) genutzt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Authority {
    Gesetz,
    Verwaltung,
    Bfh,
    Fg,
}

/// Eine `params/<vz>/*.yaml`-Datei: typisierter Kopf, offener Koerper (siehe Moduldoku).
#[derive(Debug, Clone, Deserialize)]
pub struct ParamsDatei {
    pub parameter: Option<String>,
    pub veranlagungszeitraum: u16,
    pub authority: Authority,
    pub redistributable: bool,
    pub gueltig_ab: String,
    #[serde(flatten)]
    pub werte: BTreeMap<String, Value>,
}

/// Laedt eine `params/<vz>/*.yaml`-Datei.
///
/// # Errors
/// [`ParamsFehler`] bei I/O- oder YAML-Fehlern.
pub fn lade_params(pfad: &Path) -> Result<ParamsDatei, ParamsFehler> {
    let text = std::fs::read_to_string(pfad)
        .map_err(|e| ParamsFehler::Io { pfad: pfad.to_path_buf(), nachricht: e.to_string() })?;
    serde_yaml_ng::from_str(&text)
        .map_err(|e| ParamsFehler::Yaml { pfad: pfad.to_path_buf(), nachricht: e.to_string() })
}
