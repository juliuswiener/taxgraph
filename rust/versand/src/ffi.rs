//! Die EINZIGE `unsafe`-Stelle der Crate: `libericapi.so` per `dlopen` (libloading), mit den
//! Zertifikats- und Sendefunktionen, die `elster` bewusst nicht kennt.
//!
//! Signaturen aus `ericapi.h` / `eric_types.h` (ERiC 44.2.4.0, `STDCALL` ist auf Linux die
//! C-Konvention). Die Bibliothek wird mit ABSOLUTEM Pfad geladen; `libericapi.so` traegt
//! `RPATH=$ORIGIN` und findet ihre Abhaengigkeiten selbst, die Plugins ueber den `pluginPfad` von
//! `EricInitialisiere` (derselbe Weg wie `elster::validiere` und `checkest_gate.py`).
#![allow(unsafe_code)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::{Path, PathBuf};

use elster::{EricFehler, VALIDIERE_MELDUNGEN_MAX};
use libloading::Library;

type FnInitialisiere = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type FnEinstellungSetzen = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type FnBeende = unsafe extern "C" fn() -> c_int;
type FnPufferErzeugen = unsafe extern "C" fn() -> *mut c_void;
type FnPufferInhalt = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type FnPufferFreigeben = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnGetHandle = unsafe extern "C" fn(*mut u32, *mut u32, *const c_char) -> c_int;
type FnCloseHandle = unsafe extern "C" fn(u32) -> c_int;
type FnBearbeiteVorgang = unsafe extern "C" fn(
    *const c_char, // datenpuffer (XML)
    *const c_char, // datenartVersion, z.B. "ESt_2025"
    u32,           // bearbeitungsFlags
    *const c_void, // druckParameter (NULL: kein Druck)
    *const c_void, // cryptoParameter (eric_verschluesselungs_parameter_t)
    *mut c_void,   // rueckgabeXmlPuffer
    *mut c_void,   // serverantwortXmlPuffer
) -> c_int;

/// `eric_verschluesselungs_parameter_t` (`eric_types.h`): Version, Handle des Zertifikats, PIN.
#[repr(C)]
struct VerschluesselungsParameter {
    version: u32,
    zertifikat_handle: u32,
    pin: *const c_char,
}

/// Die Version der Struktur ist laut `eric_types.h` derzeit immer 3.
const VERSCHLUESSELUNGS_VERSION: u32 = 3;

/// Eine geladene und initialisierte ERiC-Instanz (Singlethread-API: genau eine je Prozess).
pub(crate) struct Eric {
    puffer_erzeugen: FnPufferErzeugen,
    puffer_inhalt: FnPufferInhalt,
    puffer_freigeben: FnPufferFreigeben,
    bearbeite_vorgang: FnBearbeiteVorgang,
    beende: FnBeende,
    get_handle: FnGetHandle,
    close_handle: FnCloseHandle,
    // Haelt die Bibliothek geladen, solange die Funktionszeiger oben benutzt werden; faellt nach
    // `Drop::drop` (das `EricBeende` ruft).
    _lib: Library,
}

fn c_text(s: &str) -> Result<CString, EricFehler> {
    CString::new(s).map_err(|_| EricFehler::NulByte)
}

/// Kopiert ein Symbol als Funktionszeiger aus der Bibliothek.
fn symbol<T: Copy>(lib: &Library, name: &[u8]) -> Result<T, EricFehler> {
    // SAFETY: `T` ist an jeder Aufrufstelle der in `ericapi.h` deklarierte Funktionstyp des
    // Symbols `name`; der kopierte Zeiger bleibt gueltig, solange `lib` lebt — `Eric` haelt
    // `_lib` fuer seine ganze Lebensdauer.
    unsafe { lib.get::<T>(name).map(|s| *s) }
        .map_err(|e| EricFehler::Laden(format!("{}: {e}", String::from_utf8_lossy(name))))
}

impl Eric {
    /// Laedt `lib_pfad`, ruft `EricInitialisiere(lib_dir, log_dir)` und hebt die Meldungs-Caps an
    /// (wie `checkest_gate._load_and_init`: Abbruch statt stiller Kappung).
    pub(crate) fn laden(lib_pfad: &Path, log_dir: &Path) -> Result<Self, EricFehler> {
        // SAFETY: Laden fuehrt die Initialisierer von libericapi.so aus; das ist die amtliche
        // ERiC-Bibliothek aus der Auslieferung unter `$ERIC_DIR`, dieselbe, die das Python-Original
        // per `ctypes.CDLL` laedt. Kein weiterer Code im Prozess teilt ihren Zustand.
        let lib =
            unsafe { Library::new(lib_pfad) }.map_err(|e| EricFehler::Laden(e.to_string()))?;
        let initialisiere: FnInitialisiere = symbol(&lib, b"EricInitialisiere\0")?;
        let einstellung_setzen: FnEinstellungSetzen = symbol(&lib, b"EricEinstellungSetzen\0")?;
        let eric = Self {
            puffer_erzeugen: symbol(&lib, b"EricRueckgabepufferErzeugen\0")?,
            puffer_inhalt: symbol(&lib, b"EricRueckgabepufferInhalt\0")?,
            puffer_freigeben: symbol(&lib, b"EricRueckgabepufferFreigeben\0")?,
            bearbeite_vorgang: symbol(&lib, b"EricBearbeiteVorgang\0")?,
            beende: symbol(&lib, b"EricBeende\0")?,
            get_handle: symbol(&lib, b"EricGetHandleToCertificate\0")?,
            close_handle: symbol(&lib, b"EricCloseHandleToCertificate\0")?,
            _lib: lib,
        };
        let lib_dir = lib_pfad.parent().unwrap_or_else(|| Path::new("."));
        let plugin_pfad = c_text(&lib_dir.to_string_lossy())?;
        let log_pfad = c_text(&log_dir.to_string_lossy())?;
        // SAFETY: zwei gueltige, NUL-terminierte Pfade, die den Aufruf ueberleben.
        let rc = unsafe { initialisiere(plugin_pfad.as_ptr(), log_pfad.as_ptr()) };
        if rc != 0 {
            return Err(EricFehler::Init(rc));
        }
        let wert = c_text(&VALIDIERE_MELDUNGEN_MAX.to_string())?;
        for name in ["validieren.fehler_max", "validieren.hinweise_max"] {
            let n = c_text(name)?;
            // SAFETY: gueltige C-Strings; ERiC ist initialisiert.
            let r = unsafe { einstellung_setzen(n.as_ptr(), wert.as_ptr()) };
            if r != 0 {
                return Err(EricFehler::Einstellung { name, rc: r });
            }
        }
        Ok(eric)
    }

    /// `EricGetHandleToCertificate`: oeffnet das Zertifikat. Bei rc != 0 gibt es kein Handle und
    /// nichts zu schliessen; sonst schliesst der Waechter es beim Verlassen.
    pub(crate) fn zertifikat_oeffnen(&self, pfad: &CStr) -> Result<ZertifikatWaechter<'_>, i32> {
        let mut handle: u32 = 0;
        let mut info: u32 = 0;
        // SAFETY: zwei gueltige, beschreibbare u32 und ein NUL-terminierter Pfad, die den Aufruf
        // ueberleben.
        let rc = unsafe {
            (self.get_handle)(
                std::ptr::from_mut(&mut handle),
                std::ptr::from_mut(&mut info),
                pfad.as_ptr(),
            )
        };
        if rc == 0 {
            Ok(ZertifikatWaechter { eric: self, handle })
        } else {
            Err(rc)
        }
    }

    /// `EricBearbeiteVorgang(xml, datenart, flags, NULL, &crypto, rueck, server)`; beide Puffer
    /// werden in jedem Fall freigegeben, ihr Inhalt vorher kopiert. Rueckgabe: rc, Rueckgabetext,
    /// Serverantwort.
    pub(crate) fn bearbeite(
        &self,
        xml: &CStr,
        datenart: &CStr,
        flags: u32,
        zertifikat: &ZertifikatWaechter<'_>,
        pin: &CStr,
    ) -> Result<(i32, String, String), EricFehler> {
        let rueck = self.puffer()?;
        let server = self.puffer()?;
        let crypto = VerschluesselungsParameter {
            version: VERSCHLUESSELUNGS_VERSION,
            zertifikat_handle: zertifikat.handle,
            pin: pin.as_ptr(),
        };
        // SAFETY: gueltige C-Strings; `crypto` lebt bis nach dem Aufruf und hat das Layout aus
        // `eric_types.h` (`repr(C)`); beide Puffer stammen aus EricRueckgabepufferErzeugen und
        // leben bis ihre Waechter fallen.
        let rc = unsafe {
            (self.bearbeite_vorgang)(
                xml.as_ptr(),
                datenart.as_ptr(),
                flags,
                std::ptr::null(),
                std::ptr::from_ref(&crypto).cast::<c_void>(),
                rueck.puffer,
                server.puffer,
            )
        };
        Ok((rc, self.inhalt(&rueck), self.inhalt(&server)))
    }

    fn puffer(&self) -> Result<PufferWaechter<'_>, EricFehler> {
        // SAFETY: ERiC ist initialisiert; der Aufruf hat keine Vorbedingung.
        let puffer = unsafe { (self.puffer_erzeugen)() };
        if puffer.is_null() {
            return Err(EricFehler::Puffer);
        }
        Ok(PufferWaechter { eric: self, puffer })
    }

    /// Kopiert den Text des Puffers (NUL-terminiert, lebt bis zur Freigabe).
    fn inhalt(&self, p: &PufferWaechter<'_>) -> String {
        // SAFETY: `p.puffer` ist gueltig, bis `p` faellt.
        let roh = unsafe { (self.puffer_inhalt)(p.puffer) };
        if roh.is_null() {
            return String::new();
        }
        // SAFETY: ERiC liefert einen NUL-terminierten Text, der bis zur Freigabe lebt.
        unsafe { CStr::from_ptr(roh) }
            .to_string_lossy()
            .into_owned()
    }
}

impl Drop for Eric {
    fn drop(&mut self) {
        // SAFETY: ERiC ist initialisiert; `EricBeende` gibt die Instanz frei. Ein Fehlercode hilft
        // beim Beenden nicht mehr.
        let _ = unsafe { (self.beende)() };
    }
}

/// Schliesst das Zertifikats-Handle, wenn es faellt (auch bei einem Abbruch nach dem Oeffnen).
pub(crate) struct ZertifikatWaechter<'e> {
    eric: &'e Eric,
    handle: u32,
}

impl Drop for ZertifikatWaechter<'_> {
    fn drop(&mut self) {
        // SAFETY: `handle` stammt aus EricGetHandleToCertificate und wird genau einmal geschlossen.
        let _ = unsafe { (self.eric.close_handle)(self.handle) };
    }
}

struct PufferWaechter<'e> {
    eric: &'e Eric,
    puffer: *mut c_void,
}

impl Drop for PufferWaechter<'_> {
    fn drop(&mut self) {
        // SAFETY: `puffer` stammt aus EricRueckgabepufferErzeugen und wird genau einmal freigegeben.
        let _ = unsafe { (self.eric.puffer_freigeben)(self.puffer) };
    }
}

/// Ein Log-Verzeichnis nur fuer den Besitzer (0700, wie `tempfile.mkdtemp`: `eric.log` nennt
/// Auszuege der Erklaerung), das nach dem Lauf bleibt. `tempfile` legt es sonst mit 0755 an.
pub(crate) fn neues_log_dir() -> Result<PathBuf, EricFehler> {
    use std::os::unix::fs::PermissionsExt;
    tempfile::Builder::new()
        .prefix("eric_versand_")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .map(tempfile::TempDir::keep)
        .map_err(|e| EricFehler::LogDir(e.to_string()))
}
