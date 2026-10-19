//! `scripts/verify_sources.py`: jede `sources/**/*.meta.yaml` prüfen.
//!
//! Dieselben Urteile und dieselben Zeilen wie das Python-Skript:
//!
//! | Zeile | Bedeutung |
//! |---|---|
//! | `OK       <quelle>` | `sha256` stimmt, Wortlaut mindestens 60 Zeichen |
//! | `MISMATCH <quelle>` | `sha256` der Datei weicht von der Meta ab |
//! | `LEER     <quelle> (Wortlaut nur N Zeichen)` | `sha256` stimmt, aber unter 60 Zeichen Wortlaut |
//! | `MISSING FILE:  <datei> (referenced by <meta>)` | die Meta nennt eine Datei, die nicht da ist |
//! | `MISSING FIELDS: <meta>` | `datei:` oder `sha256:` fehlt |
//! | `no source metadata found` | kein `*.meta.yaml` unter `sources/` |
//!
//! Gemessen wird der Wortlaut nach der Marke `--- Wortlaut`, nicht die Datei: der Metadaten-Kopf allein bringt rund 300
//! Bytes mit, und § 9 Abs. 1 S. 3 Nr. 6 ist ein einziger Satz und damit legitim kurz.

use std::fs;
use std::path::{Component, Path, PathBuf};

use regex::Regex;
use sha2::{Digest, Sha256};

use crate::py;
use crate::Fehler;

const MARKE: &str = "--- Wortlaut";
const MIN_WORTLAUT: usize = 60;

/// Die Zeilen, die das Python-Skript druckt, und sein Urteil (Exit 0 bei `ok`, sonst 1).
#[derive(Debug, PartialEq, Eq)]
pub struct Bericht {
    pub zeilen: Vec<String>,
    pub ok: bool,
}

/// Prüft alle Metadaten unter `<wurzel>/sources` und sammelt die Zeilen.
///
/// ```
/// let leer = std::env::temp_dir().join("quellen-doctest-leer");
/// let bericht = quellen::pruefen::pruefe(&leer).unwrap();
/// assert_eq!(bericht.zeilen, vec!["no source metadata found"]);
/// assert!(!bericht.ok);
/// ```
///
/// # Errors
/// [`Fehler::Datei`]: eine Metadatei oder Quelle ist nicht lesbar, oder die Metadatei ist kein UTF-8.
pub fn pruefe(wurzel: &Path) -> Result<Bericht, Fehler> {
    let mut zeilen = Vec::new();
    let ok = pruefe_zeilenweise(wurzel, &mut |z| zeilen.push(z.to_owned()))?;
    Ok(Bericht { zeilen, ok })
}

/// Wie [`pruefe`], gibt aber jede Zeile sofort an `ausgabe` (das Python-Skript druckt auch Zeile für Zeile; bricht ein
/// Lesefehler ab, stehen die Zeilen davor schon da).
///
/// # Errors
/// Wie [`pruefe`].
pub fn pruefe_zeilenweise(wurzel: &Path, ausgabe: &mut dyn FnMut(&str)) -> Result<bool, Fehler> {
    let wurzel = normalisiere(wurzel);
    let mut metas = Vec::new();
    sammle(&wurzel.join("sources"), &mut metas)?;
    // sorted(glob(...)): Pythons Reihenfolge sind die Pfadtexte, nicht die Pfadteile ('.' vor '/', 'B' vor 'a').
    metas.sort_by_key(|p| p.to_string_lossy().into_owned());
    if metas.is_empty() {
        ausgabe("no source metadata found");
        return Ok(false);
    }
    let datei_feld = feld_muster("datei")?;
    let sha_feld = feld_muster("sha256")?;
    let mut ok = true;
    for meta in &metas {
        let text = lies_text(meta, true)?;
        let datei = feld(&datei_feld, &text);
        let erwartet = feld(&sha_feld, &text);
        let (Some(datei), Some(erwartet)) = (datei, erwartet) else {
            ausgabe(&format!("MISSING FIELDS: {}", relativ(meta, &wurzel)));
            ok = false;
            continue;
        };
        let ziel = meta.parent().unwrap_or(Path::new("")).join(datei);
        if !ziel.exists() {
            ausgabe(&format!("MISSING FILE:  {datei} (referenced by {})", relativ(meta, &wurzel)));
            ok = false;
            continue;
        }
        let rel = relativ(&ziel, &wurzel);
        let bytes = fs::read(&ziel).map_err(Fehler::datei(&ziel))?;
        if format!("{:x}", Sha256::digest(&bytes)) != erwartet {
            ausgabe(&format!("{:<8} {rel}", "MISMATCH"));
            ok = false;
            continue;
        }
        // Eine leere Quelle passt zu ihrem sha256: der Integritätscheck allein wüsste nichts von zwei versehentlich leer
        // eingefrorenen Paragraphen.
        let laenge = wortlaut_laenge(&ziel)?;
        if laenge < MIN_WORTLAUT {
            ausgabe(&format!("{:<8} {rel} (Wortlaut nur {laenge} Zeichen)", "LEER"));
            ok = false;
            continue;
        }
        ausgabe(&format!("{:<8} {rel}", "OK"));
    }
    Ok(ok)
}

/// `^\s*<name>:\s*"?([^"\n]+)"?\s*$` mit `re.MULTILINE`, wie `field()` im Python-Skript.
fn feld_muster(name: &str) -> Result<Regex, Fehler> {
    let muster = format!(r#"(?m)^\s*{name}:\s*"?([^"\n]+)"?\s*$"#);
    Regex::new(&muster).map_err(|e| Fehler::Muster(e.to_string()))
}

/// Der erste Treffer, gestrippt; leer zählt wie "fehlt" (Python: `not datei`).
fn feld<'t>(muster: &Regex, text: &'t str) -> Option<&'t str> {
    let wert = py::strip(muster.captures(text)?.get(1)?.as_str());
    (!wert.is_empty()).then_some(wert)
}

/// Länge des Wortlauts in Zeichen: alles nach der ersten Marke, ohne Leerraum am Rand; ohne Marke die ganze Datei.
fn wortlaut_laenge(pfad: &Path) -> Result<usize, Fehler> {
    let text = lies_text(pfad, false)?;
    let rest = text.find(MARKE).and_then(|i| text.get(i + MARKE.len()..)).unwrap_or(&text);
    Ok(py::laenge(py::strip(rest)))
}

/// `open(pfad, encoding="utf-8"[, errors="replace"]).read()`: `\r\n` und `\r` werden zu `\n` (universelle Zeilenenden).
/// `streng`: ungültiges UTF-8 ist ein Fehler (Python: `UnicodeDecodeError`); sonst Ersatzzeichen.
fn lies_text(pfad: &Path, streng: bool) -> Result<String, Fehler> {
    let bytes = fs::read(pfad).map_err(Fehler::datei(pfad))?;
    let text = if streng {
        String::from_utf8(bytes).map_err(|e| Fehler::Datei {
            pfad: pfad.display().to_string(),
            quelle: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    };
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// `glob(<dir>/**/*.meta.yaml, recursive=True)`: rekursiv, Namen mit Punkt am Anfang zählen nicht (Datei und Ordner).
fn sammle(dir: &Path, aus: &mut Vec<PathBuf>) -> Result<(), Fehler> {
    let eintraege = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Fehler::Datei { pfad: dir.display().to_string(), quelle: e }),
    };
    for eintrag in eintraege {
        let eintrag = eintrag.map_err(Fehler::datei(dir))?;
        if eintrag.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let pfad = eintrag.path();
        if pfad.is_dir() {
            sammle(&pfad, aus)?;
        } else if pfad.to_string_lossy().ends_with(".meta.yaml") {
            aus.push(pfad);
        }
    }
    Ok(())
}

/// Entfernt `.` und löst `..` lexikalisch auf (wie `os.path.normpath`, ohne das Dateisystem zu befragen).
fn normalisiere(pfad: &Path) -> PathBuf {
    let mut aus = PathBuf::new();
    for teil in pfad.components() {
        match teil {
            Component::CurDir => {}
            Component::ParentDir if aus.file_name().is_some() => {
                aus.pop();
            }
            anderes => aus.push(anderes.as_os_str()),
        }
    }
    aus
}

// ponytail: nur Pfade unter der Wurzel; ein `datei:` mit `..` aus der Wurzel hinaus würde in Python als `../…` erscheinen,
// hier als voller Pfad. Kein Fall im Baum (207 Metadaten, keine hat einen Schrägstrich in `datei:`).
fn relativ(pfad: &Path, wurzel: &Path) -> String {
    let p = normalisiere(pfad);
    p.strip_prefix(wurzel).unwrap_or(&p).display().to_string()
}
