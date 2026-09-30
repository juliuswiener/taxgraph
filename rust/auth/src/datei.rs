//! Nutzerdatei `users.json` (`auth.py:37-66`): lesen, und atomar nur fuer den Eigentuemer
//! lesbar schreiben.
//!
//! Format wie `json.dump(store, f, ensure_ascii=False, sort_keys=True)`: Schluessel sortiert
//! (`serde_json::Map` ist ohne `preserve_order` eine `BTreeMap`, also Codepunkt-Ordnung wie
//! Pythons `sort_keys`), Trenner `", "` und `": "` (Pythons Voreinstellung) statt `serde_json`s
//! kompaktem `","`/`":"` — damit Python und Rust dieselbe Datei byte-gleich schreiben.
use std::io::Write;
use std::path::Path;

use serde_json::Value;

use crate::AuthFehler;

/// `_lade_users`: fehlt die Datei, ein leerer Bestand `{"users": {}}`.
pub(crate) fn lade(pfad: &Path) -> Result<Value, AuthFehler> {
    match std::fs::read_to_string(pfad) {
        Ok(text) => {
            let wert: Value = serde_json::from_str(&text).map_err(AuthFehler::NutzerdateiKaputt)?;
            if wert.get("users").is_some_and(Value::is_object) {
                Ok(wert)
            } else {
                // PARITAET: Python wirft `KeyError`/`TypeError` beim ersten Zugriff (500).
                Err(AuthFehler::NutzerdateiOhneUsers)
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({"users": {}})),
        Err(e) => Err(AuthFehler::Speicher(e)),
    }
}

/// `_speichere_users`: `<pfad>.tmp` mit `O_EXCL` und Modus 0600 neu anlegen (eine liegen
/// gebliebene tmp-Datei vorher loeschen — ihre Rechte sind unbekannt), `fsync`, dann `rename`.
pub(crate) fn speichere(pfad: &Path, bestand: &Value) -> Result<(), AuthFehler> {
    use std::os::unix::fs::OpenOptionsExt;
    if let Some(dir) = pfad.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp_name = pfad.as_os_str().to_owned();
    tmp_name.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp_name);
    match std::fs::remove_file(&tmp) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(AuthFehler::Speicher(e)),
        _ => {}
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)?;
    f.write_all(py_json(bestand).as_bytes())?;
    f.flush()?;
    f.sync_all()?;
    std::fs::rename(&tmp, pfad)?;
    Ok(())
}

/// `json.dumps(wert, ensure_ascii=False, sort_keys=True)` mit Pythons Standard-Trennern.
///
/// ```
/// let w = serde_json::json!({"b": [1, 2], "a": "ä"});
/// assert_eq!(auth::py_json(&w), r#"{"a": "ä", "b": [1, 2]}"#);
/// ```
#[must_use]
pub fn py_json(wert: &Value) -> String {
    let mut puffer = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut puffer, PyFormatter);
    // Serialisierung in einen `Vec` scheitert nur an nicht-endlichen Floats, die ein aus JSON
    // gelesener `Value` nicht tragen kann.
    if serde::Serialize::serialize(wert, &mut ser).is_err() {
        return String::new();
    }
    String::from_utf8(puffer).unwrap_or_default()
}

struct PyFormatter;

impl serde_json::ser::Formatter for PyFormatter {
    fn begin_array_value<W: ?Sized + Write>(
        &mut self,
        w: &mut W,
        erstes: bool,
    ) -> std::io::Result<()> {
        if erstes {
            Ok(())
        } else {
            w.write_all(b", ")
        }
    }

    fn begin_object_key<W: ?Sized + Write>(
        &mut self,
        w: &mut W,
        erstes: bool,
    ) -> std::io::Result<()> {
        if erstes {
            Ok(())
        } else {
            w.write_all(b", ")
        }
    }

    fn begin_object_value<W: ?Sized + Write>(&mut self, w: &mut W) -> std::io::Result<()> {
        w.write_all(b": ")
    }
}
