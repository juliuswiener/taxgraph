//! `parity` — vergleicht `catala-sys` (Rust/C) gegen `tools/parity/oracle.py` (Python), dem
//! Referenz-Pfad ueber `produkt/engine/runner.py`. `REWRITE_PLAN.md` §5.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

/// Ein laufender `tools/parity/oracle.py`-Prozess mit JSON-Zeilen auf stdin/stdout.
pub struct Oracle {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// Das Orakel liess sich nicht starten oder antwortete nicht wie erwartet.
#[derive(Debug, thiserror::Error)]
pub enum OrakelFehler {
    #[error("Orakel-Prozess: {0}")]
    Io(#[from] std::io::Error),
    #[error("Orakel-Antwort nicht als JSON lesbar: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Orakel hat die Verbindung geschlossen (stdout leer)")]
    Geschlossen,
    #[error("Orakel meldet einen Fehler: {0}")]
    Python(String),
}

#[derive(serde::Serialize)]
struct Anfrage {
    #[serde(rename = "fn")]
    funktion: &'static str,
    zve_cent: i64,
    vz: u16,
}

#[derive(serde::Deserialize)]
struct Antwort {
    ok: bool,
    cent: Option<i64>,
    error: Option<String>,
}

#[derive(serde::Serialize)]
struct EventIdAnfrage<'a> {
    #[serde(rename = "fn")]
    funktion: &'static str,
    event: &'a serde_json::Value,
}

#[derive(serde::Deserialize)]
struct EventIdAntwort {
    ok: bool,
    result: Option<String>,
    error: Option<String>,
}

impl Oracle {
    /// Startet `python3 tools/parity/oracle.py` mit `repo_root` als Arbeitsverzeichnis.
    ///
    /// # Errors
    /// Wenn `python3` nicht startet oder das Skript nicht existiert.
    pub fn spawn(repo_root: &std::path::Path) -> Result<Self, OrakelFehler> {
        let mut child = Command::new("python3")
            .arg("tools/parity/oracle.py")
            .current_dir(repo_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        // ponytail: `.take()` liefert `None` nur, wenn `Stdio::piped()` oben fehlte -- das
        // waere ein Programmierfehler in dieser Funktion selbst, kein Laufzeitfall; ein
        // `io::Error` statt eines `unwrap`/`expect` haelt trotzdem den `unwrap_used`-Lint ein.
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("oracle.py: stdin nicht pipe-bar"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("oracle.py: stdout nicht pipe-bar"))?;
        Ok(Self { child, stdin, stdout: BufReader::new(stdout) })
    }

    fn call(&mut self, funktion: &'static str, zve_cent: i64, vz: u16) -> Result<i64, OrakelFehler> {
        let anfrage = Anfrage { funktion, zve_cent, vz };
        let mut zeile = serde_json::to_string(&anfrage)?;
        zeile.push('\n');
        self.stdin.write_all(zeile.as_bytes())?;
        self.stdin.flush()?;

        let mut antwort_zeile = String::new();
        let n = self.stdout.read_line(&mut antwort_zeile)?;
        if n == 0 {
            return Err(OrakelFehler::Geschlossen);
        }
        let antwort: Antwort = serde_json::from_str(antwort_zeile.trim())?;
        if antwort.ok {
            antwort.cent.ok_or_else(|| {
                OrakelFehler::Python("ok=true ohne cent-Feld".to_string())
            })
        } else {
            Err(OrakelFehler::Python(antwort.error.unwrap_or_default()))
        }
    }

    /// § 32a Abs. 1 `EStG` Grundtarif ueber `produkt/engine/runner.py`, in Cent.
    ///
    /// # Errors
    /// Siehe [`OrakelFehler`].
    pub fn grundtarif(&mut self, zve_cent: i64, vz: u16) -> Result<i64, OrakelFehler> {
        self.call("grundtarif", zve_cent, vz)
    }

    /// § 32a Abs. 5 `EStG` Splittingtarif ueber `produkt/engine/runner.py`, in Cent.
    ///
    /// # Errors
    /// Siehe [`OrakelFehler`].
    pub fn splittingtarif(&mut self, zve_cent: i64, vz: u16) -> Result<i64, OrakelFehler> {
        self.call("splittingtarif", zve_cent, vz)
    }

    /// `produkt/store/store.py::event_id` ueber ein beliebiges JSON-Objekt (Hex-String), fuer
    /// die `store`-Crate-Paritaet (`tests/store_paritaet.rs`). Eigene Anfrage-/Antwortform
    /// (`result` statt `cent`), da das Ergebnis ein String ist, kein Cent-Betrag.
    ///
    /// # Errors
    /// Siehe [`OrakelFehler`].
    pub fn event_id(&mut self, event: &serde_json::Value) -> Result<String, OrakelFehler> {
        let anfrage = EventIdAnfrage { funktion: "store.event_id", event };
        let mut zeile = serde_json::to_string(&anfrage)?;
        zeile.push('\n');
        self.stdin.write_all(zeile.as_bytes())?;
        self.stdin.flush()?;

        let mut antwort_zeile = String::new();
        let n = self.stdout.read_line(&mut antwort_zeile)?;
        if n == 0 {
            return Err(OrakelFehler::Geschlossen);
        }
        let antwort: EventIdAntwort = serde_json::from_str(antwort_zeile.trim())?;
        if antwort.ok {
            antwort
                .result
                .ok_or_else(|| OrakelFehler::Python("ok=true ohne result-Feld".to_string()))
        } else {
            Err(OrakelFehler::Python(antwort.error.unwrap_or_default()))
        }
    }

    /// Generischer Aufruf fuer die `engine`-Funktionen aus Schritt 4a: `funktion` ist der
    /// `DISPATCH`-Schluessel und `args` das `args`-Objekt, wie `tools/parity/oracle.py`s
    /// JSONL-Schema sie fuer diese Funktionen erwartet (Cent/Bruch/Bool/Integer, siehe dort).
    ///
    /// # Errors
    /// Siehe [`OrakelFehler`].
    pub fn call_engine(&mut self, funktion: &str, args: serde_json::Value) -> Result<i64, OrakelFehler> {
        #[derive(serde::Serialize)]
        struct EngineAnfrage<'a> {
            #[serde(rename = "fn")]
            funktion: &'a str,
            args: serde_json::Value,
        }
        let anfrage = EngineAnfrage { funktion, args };
        let mut zeile = serde_json::to_string(&anfrage)?;
        zeile.push('\n');
        self.stdin.write_all(zeile.as_bytes())?;
        self.stdin.flush()?;

        let mut antwort_zeile = String::new();
        let n = self.stdout.read_line(&mut antwort_zeile)?;
        if n == 0 {
            return Err(OrakelFehler::Geschlossen);
        }
        let antwort: Antwort = serde_json::from_str(antwort_zeile.trim())?;
        if antwort.ok {
            antwort.cent.ok_or_else(|| {
                OrakelFehler::Python("ok=true ohne cent-Feld".to_string())
            })
        } else {
            Err(OrakelFehler::Python(antwort.error.unwrap_or_default()))
        }
    }
}

impl Drop for Oracle {
    fn drop(&mut self) {
        // Ein liegen gebliebener Python-Prozess ist kein Datenverlust, nur eine Leiche;
        // Fehler beim Aufraeumen sind bewusst nicht fatal.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Ein einzelner Vergleichsfall, der abwich.
#[derive(Debug, PartialEq, Eq)]
pub struct Abweichung {
    pub zve_cent: i64,
    pub vz: u16,
    pub rust_cent: i64,
    pub python_cent: i64,
}

/// Vergleicht `rust_cent` gegen `oracle`s Antwort fuer dieselbe Eingabe; `None`, wenn beide
/// uebereinstimmen.
///
/// # Errors
/// Reicht [`OrakelFehler`] durch, wenn der Python-Aufruf selbst scheitert (kein Vergleich
/// moeglich -- anders als eine Abweichung im Ergebnis).
pub fn diff_grundtarif(
    oracle: &mut Oracle,
    zve_cent: i64,
    vz: u16,
    rust_cent: i64,
) -> Result<Option<Abweichung>, OrakelFehler> {
    let python_cent = oracle.grundtarif(zve_cent, vz)?;
    Ok(if python_cent == rust_cent {
        None
    } else {
        Some(Abweichung { zve_cent, vz, rust_cent, python_cent })
    })
}

/// Vergleicht `rust_cent` gegen `oracle`s Antwort fuer denselben Splittingtarif-Fall.
///
/// # Errors
/// Siehe [`diff_grundtarif`].
pub fn diff_splittingtarif(
    oracle: &mut Oracle,
    zve_cent: i64,
    vz: u16,
    rust_cent: i64,
) -> Result<Option<Abweichung>, OrakelFehler> {
    let python_cent = oracle.splittingtarif(zve_cent, vz)?;
    Ok(if python_cent == rust_cent {
        None
    } else {
        Some(Abweichung { zve_cent, vz, rust_cent, python_cent })
    })
}

/// Vergleicht `rust_hex` (`store::EventId::von_json(...).to_string()`) gegen `oracle`s
/// `produkt/store/store.py::event_id`-Antwort fuer dasselbe JSON-Event; `None`, wenn beide
/// uebereinstimmen.
///
/// # Errors
/// Siehe [`diff_grundtarif`].
pub fn diff_event_id(
    oracle: &mut Oracle,
    event: &serde_json::Value,
    rust_hex: &str,
) -> Result<Option<String>, OrakelFehler> {
    let python_hex = oracle.event_id(event)?;
    Ok(if python_hex == rust_hex { None } else { Some(python_hex) })
}

/// Ein Vergleichsfall fuer eine `engine`-Funktion aus Schritt 4a (deliverable 3/4), der abwich.
/// Traegt `funktion`/`args` statt der `tarif`-spezifischen `zve_cent`/`vz`, weil die 16
/// Funktionen unterschiedliche Eingabeformen haben (siehe `tools/parity/oracle.py::DISPATCH`).
#[derive(Debug, PartialEq)]
pub struct EngineAbweichung {
    pub funktion: String,
    pub args: serde_json::Value,
    pub rust_cent: i64,
    pub python_cent: i64,
}

/// Vergleicht `rust_cent` gegen `oracle`s Antwort fuer eine beliebige `engine`-Funktion.
///
/// # Errors
/// Siehe [`diff_grundtarif`].
pub fn diff_engine(
    oracle: &mut Oracle,
    funktion: &str,
    args: serde_json::Value,
    rust_cent: i64,
) -> Result<Option<EngineAbweichung>, OrakelFehler> {
    let python_cent = oracle.call_engine(funktion, args.clone())?;
    Ok(if python_cent == rust_cent {
        None
    } else {
        Some(EngineAbweichung { funktion: funktion.to_string(), args, rust_cent, python_cent })
    })
}

#[derive(serde::Serialize)]
struct AppendSequenceAnfrage<'a> {
    #[serde(rename = "fn")]
    funktion: &'static str,
    store: &'a serde_json::Value,
    calls: &'a [serde_json::Value],
}

#[derive(serde::Deserialize)]
struct AppendSequenceHuelle {
    ok: bool,
    result: Option<AppendSequenceErgebnis>,
    error: Option<String>,
}

/// Antwort auf `store.append_sequence`: pro Aufruf entweder der neue `event_id` oder eine
/// Fehlerklasse (`tools/parity/oracle.py::_fehlerklasse`), plus die `feld_id -> event_id`-Menge
/// der am Ende noch aktiven Events (`store.py::_aktives`, sortiert).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct AppendSequenceErgebnis {
    pub results: Vec<AppendCallErgebnis>,
    pub aktive_event_ids: Vec<String>,
}

/// Ergebnis EINES Aufrufs innerhalb einer `append_sequence`-Anfrage.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(untagged)]
pub enum AppendCallErgebnis {
    Erfolg { event_id: String },
    Fehler { err: String },
}

impl Oracle {
    /// `produkt/store/store.py::append_event` als Aufruf-Sequenz gegen EINEN Store
    /// (`tests/store_append_paritaet.rs`, Nachtrag zu Deliverable #2): `store` ist eine
    /// Store-Datei (leer oder eine reale Fall-Datei) als JSON, `calls` eine Liste roher
    /// `append_event`-kwargs (s. `tools/parity/oracle.py::_append_sequence`).
    ///
    /// # Errors
    /// Siehe [`OrakelFehler`].
    pub fn append_sequence(
        &mut self,
        store: &serde_json::Value,
        calls: &[serde_json::Value],
    ) -> Result<AppendSequenceErgebnis, OrakelFehler> {
        let anfrage = AppendSequenceAnfrage { funktion: "store.append_sequence", store, calls };
        let mut zeile = serde_json::to_string(&anfrage)?;
        zeile.push('\n');
        self.stdin.write_all(zeile.as_bytes())?;
        self.stdin.flush()?;

        let mut antwort_zeile = String::new();
        let n = self.stdout.read_line(&mut antwort_zeile)?;
        if n == 0 {
            return Err(OrakelFehler::Geschlossen);
        }
        let antwort: AppendSequenceHuelle = serde_json::from_str(antwort_zeile.trim())?;
        if antwort.ok {
            antwort
                .result
                .ok_or_else(|| OrakelFehler::Python("ok=true ohne result-Feld".to_string()))
        } else {
            Err(OrakelFehler::Python(antwort.error.unwrap_or_default()))
        }
    }
}

/// Serialisiert Zugriffe auf das eine geteilte `Oracle` in den Integrationstests
/// (`tests/tarif_paritaet.rs`) -- Tests laufen sonst parallel und teilen sich einen einzigen
/// Python-Prozess nicht sauber.
pub static ORACLE_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use super::{diff_grundtarif, Oracle};

    fn repo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn oracle_antwortet() {
        let _guard = super::ORACLE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut o = Oracle::spawn(&repo_root()).unwrap();
        let cent = o.grundtarif(12_096 * 100, 2025).unwrap();
        assert_eq!(cent, 0);
    }

    #[test]
    fn diff_erkennt_uebereinstimmung() {
        let _guard = super::ORACLE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut o = Oracle::spawn(&repo_root()).unwrap();
        let python_cent = o.grundtarif(5_000_000, 2025).unwrap();
        let abweichung = diff_grundtarif(&mut o, 5_000_000, 2025, python_cent).unwrap();
        assert_eq!(abweichung, None);
    }

    #[test]
    fn diff_erkennt_abweichung() {
        let _guard = super::ORACLE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut o = Oracle::spawn(&repo_root()).unwrap();
        let python_cent = o.grundtarif(5_000_000, 2025).unwrap();
        let abweichung = diff_grundtarif(&mut o, 5_000_000, 2025, python_cent + 1).unwrap();
        assert!(abweichung.is_some());
    }
}
