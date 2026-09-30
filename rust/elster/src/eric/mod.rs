//! ERiC offline: checkESt-Plausibilitaetspruefung (`elster/checkest_gate.py`) und das Finden der
//! Bibliothek (`elster/smoke_test.py::find_eric_lib`).
//!
//! Nur `ERIC_VALIDIERE` — die Pruefung laeuft lokal im checkESt-Plugin, KEIN Netz, KEIN Versand.
//! Ein Versand-Flag gibt es in dieser Crate nicht (Echtversand ist Julius vorbehalten,
//! `REWRITE_PLAN.md` F4).
//!
//! Falsch-Gruen-Sperre: nur `rc == 0` ist [`EricKlasse::Plausibel`]. Vier Klassen heissen
//! „nicht geprueft" ([`nicht_geprueft`]) — sie liefern einen leeren Fehlerpuffer, der wie
//! „keine Beanstandung" aussieht.

mod ffi;

use std::path::PathBuf;
use std::sync::Mutex;

pub use store::EricKlasse;

use crate::xsd::eric_wurzeln;

/// Nur pruefen, nicht senden (`ericapi` bearbeitungsFlags).
pub const ERIC_VALIDIERE: u32 = 1 << 1;
/// Fehler-/Hinweis-Cap: ERiC kappt per Default bei 20 Meldungen, erlaubt sind 1..=1000.
pub const VALIDIERE_MELDUNGEN_MAX: u32 = 1000;

pub const RC_OK: i64 = 0;
/// Plausibilitaetsfehler; die Liste steht im Rueckgabepuffer.
pub const RC_PLAUSIBILITAET: i64 = 610_001_002;
/// `ERIC_IO_READER_SCHEMA_VALIDIERUNGSFEHLER`: Sammelcode VOR der Plausibilitaet, Ursache nur in eric.log.
pub const RC_IO_SCHEMA_VALIDIERUNGSFEHLER: i64 = 610_301_200;
/// Test-Hersteller-ID gesperrt.
pub const RC_HERSTELLER_GESPERRT: i64 = 610_301_202;
/// `ERIC_GLOBAL_DATENARTVERSION_UNBEKANNT`: kein Pruefmodul fuer diesen VZ (z.B. `ESt_2026`).
pub const RC_DATENARTVERSION_UNBEKANNT: i64 = 610_001_042;
/// `ERIC_IO_READER_UNERWARTETE_ELEMENTE`.
pub const RC_IO_UNERWARTETE_ELEMENTE: i64 = 610_301_106;

/// rc → Klasse (`klassifiziere_rc`). Ein nicht gelisteter rc ist [`EricKlasse::Sonstig`], nie
/// „plausibel".
///
/// ```
/// use elster::{klassifiziere_rc, EricKlasse};
/// assert_eq!(klassifiziere_rc(0), EricKlasse::Plausibel);
/// assert_eq!(klassifiziere_rc(610_301_200), EricKlasse::IoGateNichtGeprueft);
/// assert_eq!(klassifiziere_rc(-1), EricKlasse::Sonstig);
/// ```
#[must_use]
pub fn klassifiziere_rc(rc: i64) -> EricKlasse {
    match rc {
        RC_OK => EricKlasse::Plausibel,
        RC_PLAUSIBILITAET => EricKlasse::PlausibilitaetFehler,
        RC_IO_SCHEMA_VALIDIERUNGSFEHLER => EricKlasse::IoGateNichtGeprueft,
        RC_HERSTELLER_GESPERRT => EricKlasse::HerstellerIdGesperrt,
        RC_DATENARTVERSION_UNBEKANNT => EricKlasse::DatenartversionUnbekannt,
        RC_IO_UNERWARTETE_ELEMENTE => EricKlasse::IoReaderUnerwarteteElemente,
        _ => EricKlasse::Sonstig,
    }
}

/// Die Klassen, bei denen die Erklaerung gar nicht inhaltlich geprueft wurde
/// (`NICHT_GEPRUEFT_KLASSEN`).
///
/// ```
/// use elster::{nicht_geprueft, EricKlasse};
/// assert!(nicht_geprueft(EricKlasse::DatenartversionUnbekannt));
/// assert!(!nicht_geprueft(EricKlasse::PlausibilitaetFehler));
/// ```
#[must_use]
pub fn nicht_geprueft(k: EricKlasse) -> bool {
    matches!(
        k,
        EricKlasse::IoGateNichtGeprueft
            | EricKlasse::HerstellerIdGesperrt
            | EricKlasse::DatenartversionUnbekannt
            | EricKlasse::IoReaderUnerwarteteElemente
    )
}

/// Erreicht die Fehlerzahl den Cap, kann die Liste gekappt sein — nie als vollstaendig behandeln.
///
/// ```
/// assert!(!elster::gekappt_verdacht("<FehlerRegelpruefung>"));
/// ```
#[must_use]
pub fn gekappt_verdacht(antwort: &str) -> bool {
    antwort.matches("<FehlerRegelpruefung>").count() >= VALIDIERE_MELDUNGEN_MAX as usize
}

/// ERiC-Bibliothek: erste `libericapi.so` (sortiert) unter `$ERIC_DIR`, dann `~/02_Software/eric`.
///
/// ponytail: das Legacy-Fallback des Originals (ein `elster/`-Verzeichnis neben dem Skript)
/// fehlt — die Crate kennt ihren Repo-Ort zur Laufzeit nicht. Upgrade: `$ERIC_DIR` setzen.
///
/// ```
/// let _ = elster::find_eric_lib(); // None ohne ERiC-Auslieferung
/// ```
#[must_use]
pub fn find_eric_lib() -> Option<PathBuf> {
    crate::xsd::finde_datei_in(&eric_wurzeln(), "libericapi.so")
}

/// ERiC liess sich nicht laden oder initialisieren.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EricFehler {
    #[error("libericapi.so nicht gefunden — ERIC_DIR setzen (s. elster/README).")]
    NichtGefunden,
    #[error("ERiC laden: {0}")]
    Laden(String),
    #[error("EricInitialisiere fehlgeschlagen (rc={0}).")]
    Init(i32),
    #[error(
        "EricEinstellungSetzen({name}={max}) fehlgeschlagen (rc={rc}) — Fehler-Cap NICHT angehoben, \
         Falsch-Gruen-Risiko. Abbruch statt stiller Kappung.",
        max = VALIDIERE_MELDUNGEN_MAX
    )]
    Einstellung { name: &'static str, rc: i32 },
    #[error("EricRueckgabepufferErzeugen lieferte NULL")]
    Puffer,
    #[error("XML oder Datenart enthaelt ein NUL-Byte")]
    NulByte,
    #[error("Log-Verzeichnis: {0}")]
    LogDir(String),
}

static ERIC: Mutex<Option<ffi::Eric>> = Mutex::new(None);

fn neues_log_dir() -> Result<PathBuf, EricFehler> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!("eric_checkest_{}_{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| EricFehler::LogDir(e.to_string()))?;
    Ok(dir)
}

/// Offline-Plausibilitaetspruefung (`checkest_gate.validate`): `(rc, ericantwort_xml)`.
/// ERiC wird beim ersten Aufruf einmal je Prozess geladen und initialisiert; Aufrufe sind
/// serialisiert (Singlethread-API).
///
/// # Errors
/// [`EricFehler`], wenn ERiC nicht gefunden, geladen oder initialisiert werden kann.
///
/// ```no_run
/// let (rc, _antwort) = elster::validiere(b"<Elster/>", "ESt_2025").unwrap();
/// println!("{:?}", elster::klassifiziere_rc(i64::from(rc)));
/// ```
pub fn validiere(xml: &[u8], datenart_version: &str) -> Result<(i32, String), EricFehler> {
    let xml = std::ffi::CString::new(xml).map_err(|_| EricFehler::NulByte)?;
    let datenart = std::ffi::CString::new(datenart_version).map_err(|_| EricFehler::NulByte)?;
    let mut guard = ERIC
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.is_none() {
        let lib = find_eric_lib().ok_or(EricFehler::NichtGefunden)?;
        *guard = Some(ffi::Eric::laden(
            &lib,
            neues_log_dir()?,
            VALIDIERE_MELDUNGEN_MAX,
        )?);
    }
    let eric = guard.as_ref().ok_or(EricFehler::NichtGefunden)?;
    eric.bearbeite(&xml, &datenart, ERIC_VALIDIERE)
}

/// Pfad zu `eric.log` des initialisierten ERiC, falls vorhanden.
///
/// ```
/// let _ = elster::eric_log_pfad();
/// ```
#[must_use]
pub fn eric_log_pfad() -> Option<PathBuf> {
    let guard = ERIC
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let pfad = guard.as_ref()?.log_dir.join("eric.log");
    pfad.exists().then_some(pfad)
}
