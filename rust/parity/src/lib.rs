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
