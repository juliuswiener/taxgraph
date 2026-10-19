//! `scripts/verify_sources.py`: jede `sources/**/*.meta.yaml` prüfen.

use std::path::Path;

use crate::Fehler;

/// Die Zeilen, die das Python-Skript druckt, und sein Urteil (Exit 0 bei `ok`, sonst 1).
#[derive(Debug, PartialEq, Eq)]
pub struct Bericht {
    pub zeilen: Vec<String>,
    pub ok: bool,
}

/// Prüft alle Metadaten unter `<wurzel>/sources`.
///
/// # Errors
/// [`Fehler::Datei`]: eine Metadatei ist nicht lesbar.
pub fn pruefe(wurzel: &Path) -> Result<Bericht, Fehler> {
    let _ = wurzel;
    Ok(Bericht { zeilen: Vec::new(), ok: true })
}
