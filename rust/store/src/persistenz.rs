//! Persistenz der Store-Datei: [`lade`] liest YAML (`store.py:84-87`), [`speichere`] schreibt
//! JSON atomar mit Modus 0600 (`produkt/haut/api.py:146-162`, `speichere_fall`) — DIESELBE Datei
//! durchlaeuft in Python zwei verschiedene Formate an zwei verschiedenen Stellen. Das
//! funktioniert, weil JSON eine Teilmenge von YAML ist: `yaml.safe_load` parst eine von
//! `speichere_fall` geschriebene JSON-Datei anstandslos. Eine von Hand geschriebene, echte
//! YAML-Datei (Kommentare, Anker, Flow-Referenzen) laedt `lade` ebenfalls — `speichere` selbst
//! erzeugt so etwas nie, weil `api.py` es nie erzeugt.
//!
//! PARITAET: `serde_yaml_ng` liest, `serde_json` schreibt — dieselbe Asymmetrie wie im Original,
//! bewusst nicht geglaettet: sie ist eine reale Betriebseigenschaft der Python-Version, kein
//! Bug, den diese Portierung "beheben" duerfte.
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::store::StoreDatei;

/// Fehler beim Laden/Speichern der Store-Datei.
#[derive(Debug, thiserror::Error)]
pub enum PersistenzFehler {
    #[error("store-datei {0} konnte nicht gelesen werden: {1}")]
    Lesen(PathBuf, std::io::Error),
    #[error("store-datei {0} ist kein gueltiges YAML/JSON: {1}")]
    Format(PathBuf, serde_yaml_ng::Error),
    #[error("store-datei konnte nicht geschrieben werden: {0}")]
    Schreiben(#[from] std::io::Error),
    #[error("store-datei konnte nicht serialisiert werden: {0}")]
    Serialisieren(#[from] serde_json::Error),
}

/// Laedt eine Store-Datei (`store.py:84-87`, `lade`): YAML-Parser, KEINE Schema-Pruefung — eine
/// strukturell falsche Datei liefert hier einen Fehler (Serde braucht die Pflichtfelder), eine
/// inhaltlich falsche (z.B. unbekanntes `feld_id`) laedt anstandslos durch, genau wie in Python.
///
/// # Errors
/// [`PersistenzFehler::Lesen`]/[`PersistenzFehler::Format`].
pub fn lade(pfad: &Path) -> Result<StoreDatei, PersistenzFehler> {
    let text = std::fs::read_to_string(pfad)
        .map_err(|e| PersistenzFehler::Lesen(pfad.to_path_buf(), e))?;
    serde_yaml_ng::from_str(&text).map_err(|e| PersistenzFehler::Format(pfad.to_path_buf(), e))
}

/// Schreibt die Store-Datei atomar (`produkt/haut/api.py:146-162`, `speichere_fall`): Tempfile im
/// selben Verzeichnis, fsync, `rename` — ein Leser sieht nie einen halb geschriebenen Zustand.
/// Modus 0600 ab Neuanlage: hier stehen Steuer-ID, Einkommen und IBAN.
///
/// # Errors
/// [`PersistenzFehler::Schreiben`]/[`PersistenzFehler::Serialisieren`], wenn Tempfile, Schreiben
/// oder `rename` scheitern.
pub fn speichere(pfad: &Path, datei: &StoreDatei) -> Result<(), PersistenzFehler> {
    let verzeichnis = pfad
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(verzeichnis)?;
    let dateiname = pfad.file_name().and_then(|n| n.to_str()).unwrap_or("store");
    let temp_pfad = verzeichnis.join(format!(".{dateiname}.{}.tmp", std::process::id()));
    {
        let mut tmp = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temp_pfad)?;
        serde_json::to_writer(&mut tmp, datei)?;
        tmp.sync_all()?;
    }
    std::fs::rename(&temp_pfad, pfad)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{lade, speichere};
    use crate::store::{StoreDatei, Veranlagungsjahr};

    fn testdatei() -> StoreDatei {
        StoreDatei {
            version: 1,
            veranlagungszeitraum: Veranlagungsjahr(2025),
            fall_id: Some("demo-1".to_string()),
            scheibe: None,
            user_id: None,
            events: Vec::new(),
            snapshots: Vec::new(),
            vorjahr_referenz: None,
        }
    }

    #[test]
    fn speichere_und_lade_roundtrip_mit_modus_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("fall.json");
        let original = testdatei();
        speichere(&pfad, &original).unwrap();
        let modus = std::fs::metadata(&pfad).unwrap().permissions().mode() & 0o777;
        assert_eq!(modus, 0o600);
        let geladen = lade(&pfad).unwrap();
        assert_eq!(geladen.veranlagungszeitraum, Veranlagungsjahr(2025));
        assert_eq!(geladen.fall_id.as_deref(), Some("demo-1"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn lade_liest_json_als_teilmenge_von_yaml() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-json-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("roh.json");
        std::fs::write(
            &pfad,
            r#"{"version":1,"veranlagungszeitraum":2026,"events":[],"snapshots":[]}"#,
        )
        .unwrap();
        let geladen = lade(&pfad).unwrap();
        assert_eq!(geladen.veranlagungszeitraum, Veranlagungsjahr(2026));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// `vorjahr_referenz` ist `PyWert` (K2): der Rueckweg ist verlustfrei, NaN wird ein Fehler,
    /// nie ein stilles `null`.
    #[test]
    fn vorjahr_referenz_roundtrip_und_nan() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-vorjahr-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("fall.json");
        let mut datei = testdatei();
        let referenz = serde_json::json!({"verlustvortrag_bestand": {"wert": 150_000}});
        datei.vorjahr_referenz = Some(referenz.into());
        speichere(&pfad, &datei).unwrap();
        assert_eq!(
            lade(&pfad).unwrap().vorjahr_referenz,
            datei.vorjahr_referenz
        );
        datei.vorjahr_referenz = Some(domain::PyWert::Gleit(f64::NAN));
        assert!(serde_json::to_value(&datei).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// B4: `CPython` schreibt NaN/Infinity nackt in die Fallakte (`api.py:157`, `json.dump` ohne
    /// `allow_nan=False`) und liest sie mit `json.load` als Float zurueck, `1e400` als `inf`.
    /// `lade` liest dieselben Token per YAML als Text. Der Test verlangt das `CPython`-Verhalten.
    #[test]
    #[ignore = "B4: lade liest NaN/Infinity/1e400 als Text, CPython als Float (nicht Teil von K2)"]
    fn b4_nan_und_infinity_laden_als_float_wie_cpython() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-b4-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("b4.json");
        let vorlage = concat!(
            r#"{"version":1,"veranlagungszeitraum":2025,"snapshots":[],"events":[{"event_id":"ID","#,
            r#""ts":"2026-01-01T00:00:00+00:00","feld_id":"ep_arbeitstage","wert":WERT,"#,
            r#""zustand":"bestaetigt","herkunft":{"herkunft":"mensch","pruef_tiefe":"ungeprueft","#,
            r#""haftung":"nutzer"},"schreiber":"julius","signal":{"signal_1":null,"signal_2":"k"},"#,
            r#""ersetzt":null}]}"#
        );
        for (token, erwartet) in [
            ("NaN", f64::NAN),
            ("Infinity", f64::INFINITY),
            ("-Infinity", f64::NEG_INFINITY),
            ("1e400", f64::INFINITY),
        ] {
            let text = vorlage
                .replace("ID", &"0".repeat(64))
                .replace("WERT", token);
            std::fs::write(&pfad, text).unwrap();
            let datei = lade(&pfad).unwrap();
            let wert = &datei.events[0].wert;
            let wie_cpython = match *wert {
                domain::PyWert::Gleit(f) if erwartet.is_nan() => f.is_nan(),
                domain::PyWert::Gleit(f) => f.to_bits() == erwartet.to_bits(),
                _ => false,
            };
            assert!(wie_cpython, "{token}: {wert:?} statt Float wie CPython");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
