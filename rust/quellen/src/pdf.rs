//! `scripts/freeze_pdf_local.py`: ein LOKAL vorliegendes amtliches PDF (zum Beispiel ein ELSTER-Vordruck) nach `sources/`
//! einfrieren. Anders als `einfrieren` ruft dieses Werkzeug kein Netz: die Datei hat der Nutzer selbst abgelegt (ein Download
//! ist eine Aktion des Nutzers).
//!
//! Ablauf: `pdftotext -layout` → Mindestlänge → Anker-Prüfung (`--erwarte` muss wörtlich im Text stehen) → `sources/<unter>/
//! <name>.txt`, Kopie des PDF als `<name>.pdf`, `<name>.meta.yaml` mit `sha256` von Text und PDF. Alle Prüfungen laufen VOR
//! dem ersten Schreiben. `make sources-check` prüft danach den Text-`sha256`.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::einfrieren::Werkzeuge;
use crate::unterprozess::{lauf, UnterprozessFehler};
use crate::{py, Fehler};

const PDFTOTEXT_LIMIT: Duration = Duration::from_secs(120);

/// Alles, was `freeze_pdf_local.py` aus der Kommandozeile liest.
#[derive(Debug, Clone)]
pub struct PdfAuftrag {
    pub datei: String,
    pub unter: String,
    pub name: String,
    pub titel: String,
    pub url: String,
    pub abrufdatum: String,
    pub erwarte: Vec<String>,
    pub min_laenge: i64,
    pub authority: String,
    pub verwendet_in: String,
}

/// Was nach dem Schreiben bekannt ist.
#[derive(Debug, PartialEq, Eq)]
pub struct PdfGefroren {
    pub txt_pfad: PathBuf,
    pub pdf_pfad: PathBuf,
    pub txt_sha256: String,
    pub pdf_sha256: String,
    /// Die vier Zeilen, die das Python-Skript druckt (`[freeze] …`).
    pub meldungen: Vec<String>,
}

/// `os.path.expanduser`: nur `~` und `~/…` (nicht `~name`), mit `home` als Heimatordner.
///
/// ```
/// use std::path::PathBuf;
/// assert_eq!(quellen::pdf::expandiere_tilde("~/a.pdf", Some("/home/u")), PathBuf::from("/home/u/a.pdf"));
/// assert_eq!(quellen::pdf::expandiere_tilde("~", Some("/home/u")), PathBuf::from("/home/u"));
/// assert_eq!(quellen::pdf::expandiere_tilde("~x/a.pdf", Some("/home/u")), PathBuf::from("~x/a.pdf"));
/// assert_eq!(quellen::pdf::expandiere_tilde("a/~/b", Some("/home/u")), PathBuf::from("a/~/b"));
/// ```
#[must_use]
pub fn expandiere_tilde(pfad: &str, home: Option<&str>) -> PathBuf {
    match (pfad.strip_prefix('~'), home) {
        (Some(rest), Some(h)) if rest.is_empty() || rest.starts_with('/') => PathBuf::from(format!("{}{rest}", h.trim_end_matches('/'))),
        _ => PathBuf::from(pfad),
    }
}

/// Friert das PDF `auftrag.datei` nach `<wurzel>/sources/<unter>/` ein.
///
/// # Errors
/// [`Fehler::PdfFehlt`], [`Fehler::PdftotextFehlt`], [`Fehler::PdftotextFehlgeschlagen`], [`Fehler::KeinUtf8`],
/// [`Fehler::PdfZuKurz`], [`Fehler::AnkerFehlen`], [`Fehler::Datei`], [`Fehler::Unterprozess`].
pub fn pdf_einfrieren(wurzel: &Path, auftrag: &PdfAuftrag, werkzeuge: &Werkzeuge) -> Result<PdfGefroren, Fehler> {
    let pdf = expandiere_tilde(&auftrag.datei, std::env::var("HOME").ok().as_deref());
    if !pdf.is_file() {
        return Err(Fehler::PdfFehlt { pfad: pdf.display().to_string() });
    }
    let text = pdftotext(&pdf, &werkzeuge.pdftotext)?;
    let laenge = py::laenge(&text);
    if i64::try_from(laenge).unwrap_or(i64::MAX) < auftrag.min_laenge {
        return Err(Fehler::PdfZuKurz { laenge, minimum: auftrag.min_laenge });
    }
    let fehlend: Vec<String> = auftrag.erwarte.iter().filter(|e| !text.contains(e.as_str())).cloned().collect();
    if !fehlend.is_empty() {
        return Err(Fehler::AnkerFehlen { fehlend });
    }
    let aus_dir = wurzel.join("sources").join(&auftrag.unter);
    fs::create_dir_all(&aus_dir).map_err(Fehler::datei(&aus_dir))?;
    let txt_pfad = aus_dir.join(format!("{}.txt", auftrag.name));
    let pdf_pfad = aus_dir.join(format!("{}.pdf", auftrag.name));
    let kopf = format!(
        "Quelle: {}\nAbgerufen: {}\nTitel: {}\nHerkunft: amtlicher Vordruck (lokal eingefroren)\n\
         --- Textextraktion (pdftotext -layout) ---\n\n",
        auftrag.url, auftrag.abrufdatum, auftrag.titel
    );
    fs::write(&txt_pfad, format!("{kopf}{text}")).map_err(Fehler::datei(&txt_pfad))?;
    fs::copy(&pdf, &pdf_pfad).map_err(Fehler::datei(&pdf_pfad))?;
    let txt_sha256 = sha_von(&txt_pfad)?;
    let pdf_sha256 = sha_von(&pdf_pfad)?;
    let meta_pfad = aus_dir.join(format!("{}.meta.yaml", auftrag.name));
    fs::write(&meta_pfad, meta(auftrag, &txt_sha256, &pdf_sha256)).map_err(Fehler::datei(&meta_pfad))?;
    let kurz = |s: &str| s.get(..12).unwrap_or(s).to_owned();
    let meldungen = vec![
        format!("[freeze] {}  (Text sha256 {}…)", txt_pfad.display(), kurz(&txt_sha256)),
        format!("[freeze] {}  (PDF  sha256 {}…)", pdf_pfad.display(), kurz(&pdf_sha256)),
        format!("[freeze] Anker ok: {}", py::repr_liste(&auftrag.erwarte)),
        "[freeze] make sources-check sollte jetzt gruen sein.".to_owned(),
    ];
    Ok(PdfGefroren { txt_pfad, pdf_pfad, txt_sha256, pdf_sha256, meldungen })
}

/// `pdftotext -layout <pdf> -`: der Text, mit universellen Zeilenenden (`text=True` in Python macht `\r\n` und `\r` zu `\n`).
fn pdftotext(pdf: &Path, programm: &str) -> Result<String, Fehler> {
    let args = [OsStr::new("-layout"), pdf.as_os_str(), OsStr::new("-")];
    let aus = lauf(programm, &args, None, PDFTOTEXT_LIMIT).map_err(|e| match e {
        UnterprozessFehler::Start { .. } => Fehler::PdftotextFehlt,
        andere => Fehler::Unterprozess(andere),
    })?;
    if !aus.status.success() {
        let stderr = String::from_utf8_lossy(&aus.stderr);
        return Err(Fehler::PdftotextFehlgeschlagen { stderr: stderr.chars().take(300).collect() });
    }
    let text = String::from_utf8(aus.stdout).map_err(|_| Fehler::KeinUtf8)?;
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

fn sha_von(pfad: &Path) -> Result<String, Fehler> {
    let bytes = fs::read(pfad).map_err(Fehler::datei(pfad))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

/// Der Inhalt von `<name>.meta.yaml`, Zeile für Zeile wie im Python-Skript.
fn meta(a: &PdfAuftrag, txt_sha: &str, pdf_sha: &str) -> String {
    format!(
        "# Eingefrorener amtlicher Vordruck (lokal, Nutzer-Download). Provenance-Archiv.\ndokument:\n  titel: \"{}\"\n  quelle_url: \"{}\"\n  \
         abrufdatum: \"{}\"\n  datei: \"{n}.txt\"\n  pdf_datei: \"{n}.pdf\"\n  sha256: \"{txt_sha}\"\n  pdf_sha256: \"{pdf_sha}\"\n  authority: {}\n  \
         redistributable: true\n  erwartete_anker: {}\n  verwendet_in:\n    - \"{}\"\n",
        a.titel,
        a.url,
        a.abrufdatum,
        a.authority,
        py::repr_liste(&a.erwarte),
        a.verwendet_in,
        n = a.name
    )
}
