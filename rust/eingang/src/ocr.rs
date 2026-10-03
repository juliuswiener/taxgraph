//! PDF-Text und OCR als Unterprozesse (`pdftotext -layout`, `pdftoppm -r 200`,
//! `tesseract -l deu tsv`), mit den Zeitlimits und dem Seitendeckel aus
//! `kontoauszug_writer.py:42-56, 326-443` und `beleg_writer.py:43-53, 215-309`.
//!
//! Die Grenzen schuetzen einen Dienst vor einer hochgeladenen Datei: ein Unterprozess ohne
//! Zeitlimit haelt alles auf, und 500 Seiten × 60 s waeren acht Stunden. Ueber dem Deckel gibt
//! es BEWUSST kein Teilergebnis (ein halber Auszug sieht aus wie ein ganzer).
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

use crate::csv;

/// `PDFTOTEXT_ZEITLIMIT_S`.
pub const PDFTOTEXT_ZEITLIMIT: Duration = Duration::from_secs(30);
/// `PDFTOPPM_ZEITLIMIT_S`.
pub const PDFTOPPM_ZEITLIMIT: Duration = Duration::from_secs(60);
/// `TESSERACT_ZEITLIMIT_S`.
pub const TESSERACT_ZEITLIMIT: Duration = Duration::from_secs(60);
/// `OCR_SEITEN_HOECHSTZAHL`.
pub const OCR_SEITEN_HOECHSTZAHL: usize = 40;

/// Text je Zeilenindex → Confidence 0..1 (leer = Textlayer, implizit 1.0).
pub type ConfMap = BTreeMap<usize, f64>;

/// Fehler beim Lesen eines PDFs.
#[derive(Debug, thiserror::Error)]
pub enum OcrFehler {
    /// Pythons `subprocess.TimeoutExpired` — die API antwortet 422.
    #[error("Command '{befehl}' timed out after {sekunden} seconds")]
    Zeitlimit { befehl: String, sekunden: u64 },
    /// `OcrZuAufwendig` — 422, Meldung wortgleich.
    #[error("{0}")]
    ZuAufwendig(String),
    /// `PdfNichtLesbar` — `pdftotext` kann die Datei nicht oeffnen (Exit weder 0 noch 3); 422.
    #[error("Die Datei lässt sich nicht als PDF öffnen (kein PDF, beschädigt oder mit Passwort geschützt).")]
    NichtLesbar,
    /// Ein Hilfsprogramm liegt nicht auf dem `PATH` (Python: `FileNotFoundError`). Ein
    /// Betriebsproblem, keine Eigenschaft der Datei: die API antwortet 503 mit diesem Text. Jeder
    /// andere Startfehler (etwa fehlende Ausfuehrungsrechte, Pythons `PermissionError`) ist [`Self::Io`].
    #[error("Das Programm '{programm}' fehlt auf diesem Rechner.")]
    Start { programm: String },
    /// Ausgabe ist kein UTF-8 (Python: `UnicodeDecodeError`, 500).
    #[error("{0}: Ausgabe ist kein UTF-8")]
    KeinUtf8(String),
    /// `pdftoppm` erzeugte kein Bild (Python: `IndexError`, 500).
    #[error("pdftoppm lieferte keine Seite")]
    KeinBild,
    #[error("tesseract-TSV: {0}")]
    Tsv(#[from] csv::CsvFehler),
    #[error("Ein-/Ausgabe: {0}")]
    Io(#[from] std::io::Error),
}

/// `subprocess.run(befehl, capture_output=True, text=True, timeout=…, env=…)` → `(stdout, Exit)` —
/// universelle Zeilenenden wie Pythons Textmodus (`\r\n`/`\r` → `\n`).
fn lauf(
    befehl: &[&str],
    zeitlimit: Duration,
    ein_faden: bool,
) -> Result<(String, ExitStatus), OcrFehler> {
    let anzeige = format!(
        "[{}]",
        befehl
            .iter()
            .map(|b| format!("'{b}'"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let (prog, args) = befehl
        .split_first()
        .ok_or_else(|| std::io::Error::other("leerer Befehl"))?;
    let mut cmd = Command::new(prog);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if ein_faden {
        // OMP_THREAD_LIMIT=1: mehrere OpenMP-Faeden bremsen unter Ueberbuchung bis ans Zeitlimit.
        cmd.env("OMP_THREAD_LIMIT", "1");
    }
    let mut kind = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            OcrFehler::Start {
                programm: (*prog).to_owned(),
            }
        } else {
            OcrFehler::Io(e)
        }
    })?;
    let mut stdout = kind
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("stdout"))?;
    // Lesen im eigenen Faden, sonst blockiert ein volles Pipe-Puffer das Kind bis zum Zeitlimit.
    let leser = std::thread::spawn(move || {
        let mut puffer = Vec::new();
        stdout.read_to_end(&mut puffer).map(|_| puffer)
    });
    let Some(status) = kind.wait_timeout(zeitlimit)? else {
        let _ = kind.kill();
        let _ = kind.wait();
        let _ = leser.join();
        return Err(OcrFehler::Zeitlimit {
            befehl: anzeige,
            sekunden: zeitlimit.as_secs(),
        });
    };
    let bytes = leser
        .join()
        .map_err(|_| OcrFehler::KeinUtf8(anzeige.clone()))??;
    let text = String::from_utf8(bytes).map_err(|_| OcrFehler::KeinUtf8(anzeige))?;
    Ok((text.replace("\r\n", "\n").replace('\r', "\n"), status))
}

/// `_textlayer_ist_plausibel`: mindestens 20 Zeichen nach `strip()`.
fn plausibel(seite: &str) -> bool {
    llm::py::laenge(llm::py::strip(seite)) >= 20
}

/// `_tsv_zu_zeilen`: tesseract-TSV → `(zeilentext, min-Confidence 0..1)` je (block, par, line).
///
/// ```
/// let tsv = "level\tblock_num\tpar_num\tline_num\tconf\ttext\n5\t1\t1\t1\t96\tHallo\n5\t1\t1\t1\t80\tWelt\n";
/// assert_eq!(eingang::ocr::tsv_zu_zeilen(tsv).unwrap(), vec![("Hallo Welt".to_string(), 0.8)]);
/// ```
///
/// # Errors
/// [`csv::CsvFehler`].
pub fn tsv_zu_zeilen(tsv: &str) -> Result<Vec<(String, f64)>, csv::CsvFehler> {
    let (_, zeilen, fehler) = csv::dict_reader(tsv, '\t');
    if let Some(e) = fehler {
        // Python bricht beim Iterieren ab; kein Teilergebnis (vorherige Zeilen gehen verloren).
        return Err(e);
    }
    let mut reihenfolge: Vec<(String, String, String)> = Vec::new();
    let mut gruppen: BTreeMap<(String, String, String), Vec<(String, f64)>> = BTreeMap::new();
    let feld = |z: &csv::Zeile, k: &str| z.get(k).cloned().flatten().unwrap_or_default();
    for z in &zeilen {
        if z.get("level").cloned().flatten().as_deref() != Some("5") {
            continue;
        }
        let key = (
            feld(z, "block_num"),
            feld(z, "par_num"),
            feld(z, "line_num"),
        );
        let conf = z
            .get("conf")
            .cloned()
            .flatten()
            .and_then(|c| crate::kontoauszug::py_float(&c))
            .unwrap_or(-1.0);
        if !gruppen.contains_key(&key) {
            reihenfolge.push(key.clone());
        }
        gruppen
            .entry(key)
            .or_default()
            .push((feld(z, "text"), conf));
    }
    Ok(reihenfolge
        .into_iter()
        .map(|k| {
            let woerter = gruppen.remove(&k).unwrap_or_default();
            let text = woerter
                .iter()
                .filter(|(w, _)| !llm::py::strip(w).is_empty())
                .map(|(w, _)| w.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let confs: Vec<f64> = woerter
                .iter()
                .filter(|(_, c)| *c >= 0.0)
                .map(|(_, c)| c / 100.0)
                .collect();
            (text, confs.into_iter().reduce(f64::min).unwrap_or(0.0))
        })
        .collect())
}

fn text_und_conf(zeilen: Vec<(String, f64)>) -> (String, ConfMap) {
    let conf = zeilen
        .iter()
        .enumerate()
        .map(|(i, (_, c))| (i, *c))
        .collect();
    (
        zeilen
            .into_iter()
            .map(|(t, _)| t)
            .collect::<Vec<_>>()
            .join("\n"),
        conf,
    )
}

fn tesseract_tsv(bild: &Path) -> Result<Vec<(String, f64)>, OcrFehler> {
    let b = bild.to_string_lossy();
    let (tsv, _) = lauf(
        &["tesseract", &b, "stdout", "-l", "deu", "tsv"],
        TESSERACT_ZEITLIMIT,
        true,
    )?;
    Ok(tsv_zu_zeilen(&tsv)?)
}

fn pngs(dir: &Path) -> Result<Vec<std::path::PathBuf>, OcrFehler> {
    let mut v: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    v.sort();
    Ok(v)
}

/// `_ocr_tesseract_seite(pfad, seiten_nr)` (1-indiziert).
fn ocr_seite(pfad: &str, seite: usize) -> Result<(String, ConfMap), OcrFehler> {
    let td = tempfile::tempdir()?;
    let praefix = td.path().join("seite");
    let n = seite.to_string();
    // Python prueft weder Rueckgabecode noch Ausgabe von pdftoppm, nur das Zeitlimit.
    lauf(
        &[
            "pdftoppm",
            "-png",
            "-r",
            "200",
            "-f",
            &n,
            "-l",
            &n,
            pfad,
            &praefix.to_string_lossy(),
        ],
        PDFTOPPM_ZEITLIMIT,
        false,
    )?;
    let bild = pngs(td.path())?
        .into_iter()
        .next()
        .ok_or(OcrFehler::KeinBild)?;
    Ok(text_und_conf(tesseract_tsv(&bild)?))
}

/// `_ocr_tesseract_zeilen(pfad)`: Voll-Scan, jede Seite ein OCR-Lauf; Deckel VOR dem ersten.
fn ocr_alle(pfad: &str, zu_viel: &dyn Fn(usize) -> String) -> Result<(String, ConfMap), OcrFehler> {
    let td = tempfile::tempdir()?;
    let praefix = td.path().join("seite");
    lauf(
        &[
            "pdftoppm",
            "-png",
            "-r",
            "200",
            pfad,
            &praefix.to_string_lossy(),
        ],
        PDFTOPPM_ZEITLIMIT,
        false,
    )?;
    let seiten: Vec<_> = pngs(td.path())?
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .collect();
    if seiten.len() > OCR_SEITEN_HOECHSTZAHL {
        return Err(OcrFehler::ZuAufwendig(zu_viel(seiten.len())));
    }
    let mut zeilen = Vec::new();
    for s in &seiten {
        zeilen.extend(tesseract_tsv(s)?);
    }
    Ok(text_und_conf(zeilen))
}

/// Seiten mit Textlayer behalten, implausible einzeln nach-OCRen; Zeilenindex ueber die
/// Seitengrenzen fortgezaehlt (`rstrip("\n")` VOR dem Zaehlen, sonst verschiebt sich `conf_map`).
fn gemischt(
    pfad: &str,
    text: &str,
    zu_viel: &dyn Fn(usize, usize) -> String,
) -> Result<(String, ConfMap), OcrFehler> {
    let mut seiten: Vec<&str> = text.split('\x0c').collect();
    seiten.pop();
    if seiten.iter().all(|s| plausibel(s)) {
        return Ok((text.to_owned(), ConfMap::new()));
    }
    let nach_ocr = seiten.iter().filter(|s| !plausibel(s)).count();
    if nach_ocr > OCR_SEITEN_HOECHSTZAHL {
        return Err(OcrFehler::ZuAufwendig(zu_viel(nach_ocr, seiten.len())));
    }
    let mut teile = Vec::new();
    let mut conf = ConfMap::new();
    let mut versatz = 0;
    for (i, seite) in seiten.iter().enumerate() {
        let teil = if plausibel(seite) {
            (*seite).to_owned()
        } else {
            let (t, c) = ocr_seite(pfad, i + 1)?;
            conf.extend(c.into_iter().map(|(k, v)| (versatz + k, v)));
            t
        };
        let teil = teil.trim_end_matches('\n').to_owned();
        versatz += teil.matches('\n').count() + 1;
        teile.push(teil);
    }
    Ok((teile.join("\n"), conf))
}

fn pdftotext(pfad: &str) -> Result<String, OcrFehler> {
    let (text, status) = lauf(
        &["pdftotext", "-layout", pfad, "-"],
        PDFTOTEXT_ZEITLIMIT,
        false,
    )?;
    // 3 = Rechte-Fehler (Kopierschutz): kein Text, die Seiten lassen sich aber rastern, also weiter
    // in die OCR. Jeder andere Code, auch ein Signal, heisst „nicht lesbar" (1: kein oder kaputtes
    // PDF, falsches Passwort; 2: Ausgabe; 99: sonstiges).
    match status.code() {
        Some(0 | 3) => Ok(text),
        _ => Err(OcrFehler::NichtLesbar),
    }
}

/// `lies_kontoauszug_pdf(pfad)`: Textlayer zuerst (BEL→Leerzeichen), sonst Voll-Scan.
///
/// # Errors
/// [`OcrFehler`].
///
/// ```
/// use eingang::ocr::{lies_kontoauszug_pdf, OcrFehler};
/// // Eine Datei, die pdftotext nicht oeffnen kann, ist ein Fehler und kein leerer Auszug.
/// assert!(matches!(lies_kontoauszug_pdf("/gibt/es/nicht.pdf"), Err(OcrFehler::NichtLesbar)));
/// ```
pub fn lies_kontoauszug_pdf(pfad: &str) -> Result<(String, ConfMap), OcrFehler> {
    let text = pdftotext(pfad)?.replace('\x07', " ");
    if llm::py::strip(&text).is_empty() {
        return ocr_alle(pfad, &|n| {
            format!(
                "Der Auszug hat {n} Seiten ohne jeden Textlayer und müsste komplett per Bilderkennung gelesen werden (Höchstzahl: {OCR_SEITEN_HOECHSTZAHL}). Bitte auf den benötigten Zeitraum kürzen oder als CSV exportieren."
            )
        });
    }
    gemischt(pfad, &text, &|n, von| {
        format!(
            "{n} von {von} Seiten haben keinen lesbaren Textlayer und müssten einzeln per Bilderkennung gelesen werden (Höchstzahl: {OCR_SEITEN_HOECHSTZAHL}). Bitte den Auszug auf den benötigten Zeitraum kürzen oder als CSV exportieren."
        )
    })
}

/// `lies_beleg_text(pfad)`: `.txt` direkt; ohne Textlayer EIN tesseract-Lauf ueber die ganze
/// Datei (Zeitlimit 60 s × 40); sonst wie beim Kontoauszug.
///
/// # Errors
/// [`OcrFehler`].
///
/// ```
/// use std::io::Write;
/// use eingang::ocr::lies_beleg_text;
/// let mut datei = tempfile::Builder::new().suffix(".txt").tempfile().unwrap();
/// write!(datei, "Lohnsteuerbescheinigung\r\nNr. 3 45.000,00").unwrap();
/// let (text, konfidenz) = lies_beleg_text(datei.path().to_str().unwrap()).unwrap();
/// assert_eq!(text, "Lohnsteuerbescheinigung\nNr. 3 45.000,00"); // Zeilenenden normalisiert
/// assert!(konfidenz.is_empty()); // Klartext hat keine OCR-Konfidenz
/// ```
pub fn lies_beleg_text(pfad: &str) -> Result<(String, ConfMap), OcrFehler> {
    if pfad.to_lowercase().ends_with(".txt") {
        let text = std::fs::read_to_string(pfad)?;
        return Ok((
            text.replace("\r\n", "\n").replace('\r', "\n"),
            ConfMap::new(),
        ));
    }
    let text = pdftotext(pfad)?;
    if llm::py::strip(&text).is_empty() {
        let grenze = TESSERACT_ZEITLIMIT * u32::try_from(OCR_SEITEN_HOECHSTZAHL).unwrap_or(40);
        return Ok((
            lauf(&["tesseract", pfad, "-", "-l", "deu"], grenze, true)?.0,
            ConfMap::new(),
        ));
    }
    gemischt(pfad, &text, &|n, von| {
        format!(
            "{n} von {von} Seiten haben keinen lesbaren Textlayer und müssten einzeln per Bilderkennung gelesen werden (Höchstzahl: {OCR_SEITEN_HOECHSTZAHL}). Bitte den Beleg auf die benötigten Seiten kürzen."
        )
    })
}
