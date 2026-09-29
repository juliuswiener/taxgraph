//! Die EINZIGE `unsafe`-Stelle der Crate: `libericapi.so` per `dlopen` (libloading).
//!
//! Signaturen aus `ericapi.h` (ERiC 44.2.4.0, `STDCALL` ist auf Linux die C-Konvention). Die
//! Bibliothek liegt ausserhalb des Repos und wird mit ABSOLUTEM Pfad geladen: `LD_LIBRARY_PATH`
//! zur Laufzeit zu setzen (wie `checkest_gate.py:161`) wirkt auf den laufenden Prozess nicht.
//! `libericapi.so` traegt `RPATH=$ORIGIN`, findet `libericxerces.so` also selbst; die Plugins
//! (`libcheckESt_*.so`) findet ERiC ueber den `pluginPfad` von `EricInitialisiere`.
#![allow(unsafe_code)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::{Path, PathBuf};

use libloading::Library;

use super::EricFehler;

type FnInitialisiere = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type FnEinstellungSetzen = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type FnPufferErzeugen = unsafe extern "C" fn() -> *mut c_void;
type FnPufferInhalt = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type FnPufferFreigeben = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnBearbeiteVorgang = unsafe extern "C" fn(
    *const c_char, // datenpuffer (XML)
    *const c_char, // datenartVersion, z.B. "ESt_2025"
    u32,           // bearbeitungsFlags
    *const c_void, // druckParameter (NULL: kein Druck)
    *const c_void, // cryptoParameter (NULL: kein Versand)
    *mut c_void,   // rueckgabeXmlPuffer
    *mut c_void,   // serverantwortXmlPuffer (NULL: kein Versand)
) -> c_int;

/// Eine geladene und initialisierte ERiC-Instanz (Singlethread-API: genau eine je Prozess).
pub(super) struct Eric {
    // Haelt die Bibliothek geladen, solange die Funktionszeiger unten benutzt werden.
    _lib: Library,
    puffer_erzeugen: FnPufferErzeugen,
    puffer_inhalt: FnPufferInhalt,
    puffer_freigeben: FnPufferFreigeben,
    bearbeite_vorgang: FnBearbeiteVorgang,
    pub(super) log_dir: PathBuf,
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
    /// Laedt `lib_pfad`, ruft `EricInitialisiere(lib_dir, log_dir)` und hebt die Meldungs-Caps an.
    pub(super) fn laden(lib_pfad: &Path, log_dir: PathBuf, meldungen_max: u32) -> Result<Self, EricFehler> {
        // SAFETY: Laden fuehrt die Initialisierer von libericapi.so aus; das ist die amtliche
        // ERiC-Bibliothek aus der Auslieferung unter `$ERIC_DIR`, dieselbe, die das Python-
        // Original per `ctypes.CDLL` laedt. Kein weiterer Code im Prozess teilt ihren Zustand.
        let lib = unsafe { Library::new(lib_pfad) }.map_err(|e| EricFehler::Laden(e.to_string()))?;
        let initialisiere: FnInitialisiere = symbol(&lib, b"EricInitialisiere\0")?;
        let einstellung_setzen: FnEinstellungSetzen = symbol(&lib, b"EricEinstellungSetzen\0")?;
        let eric = Self {
            puffer_erzeugen: symbol(&lib, b"EricRueckgabepufferErzeugen\0")?,
            puffer_inhalt: symbol(&lib, b"EricRueckgabepufferInhalt\0")?,
            puffer_freigeben: symbol(&lib, b"EricRueckgabepufferFreigeben\0")?,
            bearbeite_vorgang: symbol(&lib, b"EricBearbeiteVorgang\0")?,
            _lib: lib,
            log_dir,
        };
        let lib_dir = lib_pfad.parent().unwrap_or_else(|| Path::new("."));
        let plugin_pfad = c_text(&lib_dir.to_string_lossy())?;
        let log_pfad = c_text(&eric.log_dir.to_string_lossy())?;
        // SAFETY: zwei gueltige, NUL-terminierte Pfade, die den Aufruf ueberleben.
        let rc = unsafe { initialisiere(plugin_pfad.as_ptr(), log_pfad.as_ptr()) };
        if rc != 0 {
            return Err(EricFehler::Init(rc));
        }
        let wert = c_text(&meldungen_max.to_string())?;
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

    /// `EricBearbeiteVorgang(xml, datenart, flags, NULL, NULL, puffer, NULL)`; der Puffer wird in
    /// jedem Fall freigegeben, sein Inhalt vorher kopiert.
    pub(super) fn bearbeite(&self, xml: &CStr, datenart: &CStr, flags: u32) -> Result<(i32, String), EricFehler> {
        // SAFETY: ERiC ist initialisiert; der Aufruf hat keine Vorbedingung.
        let puffer = unsafe { (self.puffer_erzeugen)() };
        if puffer.is_null() {
            return Err(EricFehler::Puffer);
        }
        let guard = PufferGuard { eric: self, puffer };
        // SAFETY: gueltige C-Strings; `puffer` stammt aus EricRueckgabepufferErzeugen; Druck-,
        // Crypto- und Serverantwort-Parameter NULL = nur pruefen, nichts senden.
        let rc = unsafe {
            (self.bearbeite_vorgang)(
                xml.as_ptr(),
                datenart.as_ptr(),
                flags,
                std::ptr::null(),
                std::ptr::null(),
                guard.puffer,
                std::ptr::null_mut(),
            )
        };
        // SAFETY: `puffer` ist gueltig bis `guard` faellt; der Inhalt wird VOR der Freigabe kopiert.
        let inhalt = unsafe { (self.puffer_inhalt)(guard.puffer) };
        let antwort = if inhalt.is_null() {
            String::new()
        } else {
            // SAFETY: ERiC liefert einen NUL-terminierten Text, der bis zur Freigabe lebt.
            unsafe { CStr::from_ptr(inhalt) }.to_string_lossy().into_owned()
        };
        drop(guard);
        Ok((rc, antwort))
    }
}

struct PufferGuard<'e> {
    eric: &'e Eric,
    puffer: *mut c_void,
}

impl Drop for PufferGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: `puffer` stammt aus EricRueckgabepufferErzeugen und wird genau einmal freigegeben.
        let _ = unsafe { (self.eric.puffer_freigeben)(self.puffer) };
    }
}
