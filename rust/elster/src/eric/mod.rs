//! ERiC offline: checkESt-Plausibilitaetspruefung (`elster/checkest_gate.py`) und das Finden der
//! Bibliothek (`elster/smoke_test.py::find_eric_lib`).
//!
//! Nur `ERIC_VALIDIERE` — die Pruefung laeuft lokal im checkESt-Plugin, KEIN Netz, KEIN Versand.
//! Ein Versand-Flag gibt es in dieser Crate nicht (der Echtversand liegt in der Crate `versand`,
//! `REWRITE_PLAN.md` F4).
//!
//! Falsch-Gruen-Sperre: nur `rc == 0` ist [`EricKlasse::Plausibel`]. Vier Klassen heissen
//! „nicht geprueft" ([`nicht_geprueft`]) — sie liefern einen leeren Fehlerpuffer, der wie
//! „keine Beanstandung" aussieht.

mod ffi;

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, SyncSender};
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
    /// Der Text des Betriebssystems (`dlerror`), wie ihn Python als `OSError` nennt.
    #[error("{0}")]
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
    #[error("ERiC-Thread: {0}")]
    Thread(String),
}

/// Ein Auftrag an den ERiC-Thread: Eingabe und der Weg zurueck.
struct Auftrag {
    xml: std::ffi::CString,
    datenart: std::ffi::CString,
    antwort: SyncSender<Result<(i32, String), EricFehler>>,
}

/// Der Weg zum ERiC-Thread; `None`, bis der erste Aufruf ihn startet (oder nachdem er endete).
static ARBEITER: Mutex<Option<Sender<Auftrag>>> = Mutex::new(None);
/// Das Log-Verzeichnis des geladenen ERiC; `None`, solange kein Laden gelang
/// (`checkest_gate._STATE["log_dir"]`).
static LOG_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

fn gesperrt<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Wie `tempfile.mkdtemp(prefix="eric_checkest_")`: `eric_checkest_` und 8 Zeichen aus
/// `a-z0-9_`, neu angelegt, nur fuer den Besitzer (`eric.log` nennt Auszuege der Erklaerung). Die
/// Zeichen kommen nicht aus einer Zufallsquelle, sondern aus Zeit, Prozess und Zaehler; ein
/// vorhandener Name wird uebersprungen.
fn neues_log_dir() -> Result<PathBuf, EricFehler> {
    use std::os::unix::fs::DirBuilderExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789_";
    static ZAEHLER: AtomicU64 = AtomicU64::new(0);
    for _ in 0..100 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let mut zustand = u64::try_from(nanos & u128::from(u64::MAX)).unwrap_or(1)
            ^ (u64::from(std::process::id()) << 32)
            ^ ZAEHLER.fetch_add(1, Ordering::Relaxed).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut name = String::from("eric_checkest_");
        for _ in 0..8 {
            // xorshift64*
            zustand ^= zustand >> 12;
            zustand ^= zustand << 25;
            zustand ^= zustand >> 27;
            let z = zustand.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33;
            let ix = usize::try_from(z).unwrap_or(0) % ALPHABET.len();
            name.push(char::from(ALPHABET.get(ix).copied().unwrap_or(b'_')));
        }
        let dir = std::env::temp_dir().join(name);
        match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(EricFehler::LogDir(e.to_string())),
        }
    }
    Err(EricFehler::LogDir(
        "kein freier Name fuer das Log-Verzeichnis".to_owned(),
    ))
}

/// Der ERiC-Thread: laedt ERiC beim ersten Auftrag (und bei jedem weiteren, solange das Laden
/// scheitert, wie `_load_and_init`) und fuehrt ALLE Aufrufe aus. Pythons Server ruft ERiC immer aus
/// seinem einen Hauptthread; die Singlethread-API von ERiC kennt keine Instanz je Thread.
fn arbeite(auftraege: &Receiver<Auftrag>) {
    let mut eric: Option<ffi::Eric> = None;
    while let Ok(a) = auftraege.recv() {
        let ergebnis = (|| {
            if eric.is_none() {
                let lib = find_eric_lib().ok_or(EricFehler::NichtGefunden)?;
                let log = neues_log_dir()?;
                let geladen = ffi::Eric::laden(&lib, log.clone(), VALIDIERE_MELDUNGEN_MAX)?;
                *gesperrt(&LOG_DIR) = Some(log);
                eric = Some(geladen);
            }
            let e = eric.as_ref().ok_or(EricFehler::NichtGefunden)?;
            e.bearbeite(&a.xml, &a.datenart, ERIC_VALIDIERE)
        })();
        let _ = a.antwort.send(ergebnis);
    }
}

/// Offline-Plausibilitaetspruefung (`checkest_gate.validate`): `(rc, ericantwort_xml)`.
/// ERiC wird beim ersten Aufruf geladen und initialisiert; alle Aufrufe laufen auf EINEM
/// eigenen Thread, der sie nacheinander abarbeitet (Singlethread-API).
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
    let (antwort, zurueck) = std::sync::mpsc::sync_channel(1);
    sende(Auftrag {
        xml,
        datenart,
        antwort,
    })?;
    zurueck
        .recv()
        .map_err(|_| EricFehler::Thread("ERiC-Thread endete ohne Antwort".to_owned()))?
}

/// Gibt den Auftrag an den ERiC-Thread und startet ihn dafuer, wenn es ihn nicht gibt. Ein
/// beendeter Thread (Panik) nimmt keinen Auftrag mehr an: der naechste Versuch startet einen neuen,
/// statt fuer immer zu scheitern.
fn sende(mut auftrag: Auftrag) -> Result<(), EricFehler> {
    let mut arbeiter = gesperrt(&ARBEITER);
    for _ in 0..2 {
        if arbeiter.is_none() {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("eric".to_owned())
                .spawn(move || arbeite(&rx))
                .map_err(|e| EricFehler::Thread(e.to_string()))?;
            *arbeiter = Some(tx);
        }
        let Some(tx) = arbeiter.as_ref() else { break };
        match tx.send(auftrag) {
            Ok(()) => return Ok(()),
            Err(std::sync::mpsc::SendError(zurueck)) => {
                auftrag = zurueck;
                *arbeiter = None;
            }
        }
    }
    Err(EricFehler::Thread("kein ERiC-Thread erreichbar".to_owned()))
}

/// Pfad zu `eric.log` des geladenen ERiC, falls vorhanden (`checkest_gate.eric_log_pfad`).
///
/// ```
/// let _ = elster::eric_log_pfad();
/// ```
#[must_use]
pub fn eric_log_pfad() -> Option<PathBuf> {
    let pfad = gesperrt(&LOG_DIR).as_ref()?.join("eric.log");
    pfad.exists().then_some(pfad)
}

/// Der Name einer Klasse, wie ihn Python schreibt und `schema.json` kennt.
///
/// ```
/// use elster::{klasse_name, EricKlasse};
/// assert_eq!(klasse_name(EricKlasse::IoGateNichtGeprueft), "io_gate_nicht_geprueft");
/// ```
#[must_use]
pub const fn klasse_name(k: EricKlasse) -> &'static str {
    match k {
        EricKlasse::Plausibel => "plausibel",
        EricKlasse::PlausibilitaetFehler => "plausibilitaet_fehler",
        EricKlasse::IoGateNichtGeprueft => "io_gate_nicht_geprueft",
        EricKlasse::HerstellerIdGesperrt => "hersteller_id_gesperrt",
        EricKlasse::DatenartversionUnbekannt => "datenartversion_unbekannt",
        EricKlasse::IoReaderUnerwarteteElemente => "io_reader_unerwartete_elemente",
        EricKlasse::Sonstig => "sonstig",
    }
}

/// Die Zeilen eines Textes, wie `open(...).readlines()` im Textmodus sie liefert: `\n`, `\r\n` und
/// ein einzelnes `\r` beenden eine Zeile; ohne Zeilenende bleibt die letzte Zeile.
fn zeilen_wie_python(text: &str) -> Vec<String> {
    let einheitlich = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut zeilen: Vec<String> = einheitlich.split('\n').map(str::to_owned).collect();
    if zeilen.last().is_some_and(String::is_empty) {
        zeilen.pop();
    }
    zeilen
}

/// Die letzten `n` Zeilen von `eric.log` mit `ERROR` oder dem Anfang `EC(` (`_log_auszug`). Wirft
/// nie: ein Lesefehler ist ein leerer Auszug — das Logfile ist ein Zusatz, kein Pruefpfad.
pub(crate) fn log_auszug(pfad: &std::path::Path, n: usize) -> String {
    let Ok(roh) = std::fs::read(pfad) else {
        return String::new();
    };
    let text = String::from_utf8_lossy(&roh);
    let treffer: Vec<String> = zeilen_wie_python(&text)
        .into_iter()
        .filter(|z| z.contains("ERROR") || z.starts_with("EC("))
        .collect();
    let ab = treffer.len().saturating_sub(n);
    treffer
        .iter()
        .skip(ab)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n")
}

/// [`unerwarteter_rc_hinweis`] mit dem Pfad von `eric.log` als Eingabe.
fn hinweis_mit_log(rc: i64, antwort: &str, log: Option<&std::path::Path>) -> String {
    let kern = format!(
        "rc={rc} [{}] — kein Plausibilitaetsverdikt, die Erklaerung wurde nicht inhaltlich geprueft.",
        klasse_name(klassifiziere_rc(rc))
    );
    if !antwort.is_empty() && antwort.contains("<Text>") {
        // Puffer nicht leer: der Grund steht dann bereits in der Ericantwort selbst.
        return kern;
    }
    let Some(pfad) = log else {
        return kern + " Rueckgabepuffer leer -> Details ggf. in eric.log (Pfad nicht ermittelbar).";
    };
    let auszug = log_auszug(pfad, 10);
    let ende = if auszug.is_empty() {
        ".".to_owned()
    } else {
        format!(":\n{auszug}")
    };
    format!(
        "{kern} Rueckgabepuffer leer -> Details in eric.log ({}){ende}",
        pfad.display()
    )
}

/// Klartext-Befund fuer jeden rc, der weder `RC_OK` noch `RC_PLAUSIBILITAET` ist
/// (`checkest_gate.unerwarteter_rc_hinweis`), Wort fuer Wort: nennt den rohen rc und die Klasse;
/// ist der Fehlerpuffer leer, zusaetzlich den Weg zu `eric.log` samt dessen letzten Fehlerzeilen.
///
/// ```
/// let t = elster::unerwarteter_rc_hinweis(610_301_202, "<Text>x</Text>");
/// assert_eq!(
///     t,
///     "rc=610301202 [hersteller_id_gesperrt] — kein Plausibilitaetsverdikt, die Erklaerung \
///      wurde nicht inhaltlich geprueft."
/// );
/// ```
#[must_use]
pub fn unerwarteter_rc_hinweis(rc: i64, antwort: &str) -> String {
    hinweis_mit_log(rc, antwort, eric_log_pfad().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eric_test_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn klassennamen_sind_die_serde_namen() {
        for k in [
            EricKlasse::Plausibel,
            EricKlasse::PlausibilitaetFehler,
            EricKlasse::IoGateNichtGeprueft,
            EricKlasse::HerstellerIdGesperrt,
            EricKlasse::DatenartversionUnbekannt,
            EricKlasse::IoReaderUnerwarteteElemente,
            EricKlasse::Sonstig,
        ] {
            assert_eq!(serde_json::to_value(k).unwrap(), klasse_name(k));
        }
    }

    #[test]
    fn zeilen_wie_readlines() {
        assert_eq!(zeilen_wie_python("a\r\nb\rc\nd"), ["a", "b", "c", "d"]);
        assert_eq!(zeilen_wie_python("a\n\n"), ["a", ""]);
        assert_eq!(zeilen_wie_python("").len(), 0);
    }

    #[test]
    fn auszug_nimmt_die_letzten_zehn_fehlerzeilen() {
        let d = tmp("auszug");
        let log = d.join("eric.log");
        let mut text = String::new();
        for i in 0..12 {
            let _ = write!(text, "EC(abc) {i}\nINFO nichts {i}\nx ERROR {i}\r\n");
        }
        std::fs::write(&log, &text).unwrap();
        let a = log_auszug(&log, 10);
        let zeilen: Vec<&str> = a.split('\n').collect();
        assert_eq!(zeilen.len(), 10);
        assert_eq!(zeilen[0], "EC(abc) 7");
        assert_eq!(zeilen[9], "x ERROR 11");
        assert_eq!(log_auszug(&d.join("fehlt.log"), 10), "");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn hinweis_in_den_drei_faellen() {
        let kern = "rc=610301200 [io_gate_nicht_geprueft] — kein Plausibilitaetsverdikt, \
                    die Erklaerung wurde nicht inhaltlich geprueft.";
        assert_eq!(hinweis_mit_log(610_301_200, "x<Text>y", None), kern);
        assert_eq!(
            hinweis_mit_log(610_301_200, "", None),
            format!("{kern} Rueckgabepuffer leer -> Details ggf. in eric.log (Pfad nicht ermittelbar).")
        );
        let d = tmp("hinweis");
        let log = d.join("eric.log");
        std::fs::write(&log, "kein Treffer\n").unwrap();
        assert_eq!(
            hinweis_mit_log(610_301_200, "", Some(&log)),
            format!("{kern} Rueckgabepuffer leer -> Details in eric.log ({}).", log.display())
        );
        std::fs::write(&log, "EC(1) kaputt\n").unwrap();
        assert_eq!(
            hinweis_mit_log(610_301_200, "<Foo/>", Some(&log)),
            format!(
                "{kern} Rueckgabepuffer leer -> Details in eric.log ({}):\nEC(1) kaputt",
                log.display()
            )
        );
        let _ = std::fs::remove_dir_all(d);
    }
}
