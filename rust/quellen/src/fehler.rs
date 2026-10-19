//! Die Fehler der drei Werkzeuge. Jeder trägt einen Namen und einen Text; kein Fehler schreibt eine halbe Quelle
//! (`einfrieren` und `pdf_einfrieren` prüfen alles vor dem ersten Schreiben).

use crate::unterprozess::UnterprozessFehler;

/// Alles, was `einfrieren`, `pdf_einfrieren` und `pruefen` ablehnen kann.
#[derive(Debug, thiserror::Error)]
pub enum Fehler {
    /// Die Start-Passage steht nicht im entstrippten Text (Python: `Start-Passage nicht gefunden`).
    #[error("Start-Passage nicht gefunden: {start:?}")]
    StartFehlt { start: String },
    /// Der Text ist kürzer als `--min-laenge`: meist hat das Start-Muster falsch getroffen.
    #[error("Text zu kurz ({laenge} < {minimum} Zeichen). Nichts geschrieben - vermutlich hat das Start-Muster nicht getroffen.")]
    ZuKurz { laenge: usize, minimum: i64 },
    /// Eine mit `--erwarte` genannte Passage steht nicht wörtlich im Text.
    #[error("Erwartete Passage(n) fehlen, nichts geschrieben:\n  {}", .fehlend.iter().map(|f| format!("{f:?}")).collect::<Vec<_>>().join("\n  "))]
    ErwarteFehlt { fehlend: Vec<String> },
    /// Eine benannte Entity (`&name;`) steht nicht in der Tabelle. Nie ein stilles Weglassen.
    #[error("Unbekannte HTML-Entity &{name};: nicht in der Tabelle von quellen::entity (nichts geschrieben)")]
    UnbekannteEntity { name: String },
    /// `&name` ohne Semikolon. Python deutet manche Namen auch so; hier ist es ein Fehler.
    #[error("HTML-Entity &{name} ohne Semikolon: nicht eindeutig (nichts geschrieben)")]
    EntityOhneSemikolon { name: String },
    /// `&#…;` mit einer Zahl, die kein erlaubtes Zeichen ist (0, Steuerzeichen, Surrogat, über U+10FFFF, Nichtzeichen).
    #[error("Unzulässige Zeichenreferenz {text} (nichts geschrieben)")]
    UnzulaessigeZeichenreferenz { text: String },
    /// Der Abruf der URL ist gescheitert.
    #[error("Abruf von {url} gescheitert: {grund}")]
    Abruf { url: String, grund: String },
    /// Das lokale PDF fehlt (Python: `PDF nicht gefunden`).
    #[error("PDF nicht gefunden: {pfad} — Julius muss es zuerst lokal ablegen.")]
    PdfFehlt { pfad: String },
    /// `pdftotext` ist gelaufen und hat einen Fehlercode gemeldet.
    #[error("pdftotext fehlgeschlagen: {stderr}")]
    PdftotextFehlgeschlagen { stderr: String },
    /// Die Ausgabe von `pdftotext` ist kein UTF-8 (Python: `UnicodeDecodeError`).
    #[error("Die Ausgabe von pdftotext ist kein UTF-8")]
    KeinUtf8,
    /// Das PDF-Text ist kürzer als `--min-laenge`.
    #[error("Text zu kurz ({laenge} < {minimum}) — Extraktion pruefen.")]
    PdfZuKurz { laenge: usize, minimum: i64 },
    /// Anker fehlen wörtlich im PDF-Text.
    #[error("Anker fehlen woertlich im Text (Freeze abgebrochen): {}", .fehlend.join("; "))]
    AnkerFehlen { fehlend: Vec<String> },
    /// Ein festes reguläres Muster des Werkzeugs lässt sich nicht übersetzen (Programmfehler, jeder Lauf zeigt ihn sofort).
    #[error("Muster des Werkzeugs nicht übersetzbar: {0}")]
    Muster(String),
    /// `curl`, `gzip` oder `pdftotext` fehlt, hängt oder ist nicht lesbar.
    #[error(transparent)]
    Unterprozess(#[from] UnterprozessFehler),
    /// Lesen oder Schreiben einer Datei.
    #[error("{pfad}: {quelle}")]
    Datei {
        pfad: String,
        #[source]
        quelle: std::io::Error,
    },
}

impl Fehler {
    /// Kurzform für `map_err`: `std::fs::read(p).map_err(Fehler::datei(p))`.
    pub(crate) fn datei(pfad: &std::path::Path) -> impl Fn(std::io::Error) -> Self + '_ {
        move |quelle| Self::Datei { pfad: pfad.display().to_string(), quelle }
    }
}
