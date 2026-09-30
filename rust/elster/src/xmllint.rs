//! Offline-XSD-Validierung per `xmllint` (`elster/submission/validate_xsd.py`): Struktur-Gate
//! gegen `elster11_E10_<vz>_extern.xsd`, ohne Netz, ohne Hersteller-ID.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::xsd::finde_datei;

/// Rahmen-Schema `elster11_E10_<vz>_extern.xsd` unter den ERiC-Wurzeln.
///
/// ```
/// assert!(elster::finde_xsd_schema("1999").is_none());
/// ```
#[must_use]
pub fn finde_xsd_schema(vz: &str) -> Option<PathBuf> {
    finde_datei(&format!("elster11_E10_{vz}_extern.xsd"))
}

fn xmllint() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join("xmllint"))
        .find(|p| p.is_file())
}

enum Quelle<'a> {
    Datei(&'a Path),
    Text(&'a [u8]),
}

fn validiere(quelle: &Quelle<'_>, vz: &str) -> (bool, String) {
    let Some(bin) = xmllint() else {
        return (
            false,
            "xmllint nicht gefunden (libxml2-utils installieren).".to_owned(),
        );
    };
    let Some(schema) = finde_xsd_schema(vz) else {
        return (
            false,
            format!("Schema elster11_E10_{vz}_extern.xsd nicht gefunden — ERIC_DIR setzen / ERiC-Doku entpacken."),
        );
    };
    let mut cmd = Command::new(bin);
    cmd.arg("--noout")
        .arg("--schema")
        .arg(&schema)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let ausgabe = match quelle {
        Quelle::Datei(p) => cmd.arg(p).stdin(Stdio::null()).output(),
        Quelle::Text(xml) => cmd
            .arg("-")
            .stdin(Stdio::piped())
            .spawn()
            .and_then(|mut kind| {
                if let Some(mut stdin) = kind.stdin.take() {
                    stdin.write_all(xml)?;
                }
                kind.wait_with_output()
            }),
    };
    match ausgabe {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
            (
                out.status.success(),
                if stderr.is_empty() {
                    "validates".to_owned()
                } else {
                    stderr
                },
            )
        }
        Err(e) => (false, format!("xmllint: {e}")),
    }
}

/// `(ok, meldung)`; `ok` genau dann, wenn `xmllint` die Datei gegen das E10-`vz`-Schema annimmt.
///
/// ```
/// let (ok, _meldung) = elster::validiere_xsd(std::path::Path::new("/gibt/es/nicht.xml"), "2025");
/// assert!(!ok);
/// ```
#[must_use]
pub fn validiere_xsd(xml_pfad: &Path, vz: &str) -> (bool, String) {
    validiere(&Quelle::Datei(xml_pfad), vz)
}

/// Wie [`validiere_xsd`], das XML kommt ueber stdin (keine Datei auf der Platte).
///
/// ```
/// let (ok, _meldung) = elster::validiere_xsd_text(b"<kaputt", "2025");
/// assert!(!ok);
/// ```
#[must_use]
pub fn validiere_xsd_text(xml: &[u8], vz: &str) -> (bool, String) {
    validiere(&Quelle::Text(xml), vz)
}
