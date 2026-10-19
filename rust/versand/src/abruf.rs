//! Der Abruf der Lohnsteuerbescheinigung bei ELSTER (`ElsterVaStDaten`), Abweichung Nr. 91.
//!
//! Ablauf nach dem ERiC-Handbuch 44.2.4.0, Kap. 9.2.4.2 und 9.9.6: Zertifikat oeffnen, Anfrage,
//! Wahl der `<Id>`, Abholung, `EricDekodiereDaten`. Jeder Schritt vor dem ersten ERiC-Aufruf bricht
//! ab, ohne ERiC zu laden. Steuer-ID, Datenpaket und PIN stehen nie in einem Fehlertext.

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use elster::EricFehler;

use crate::{ffi, zertifikat_abgelehnt, Zertifikat};

/// Warum der Abruf abbricht.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AbrufFehler {
    /// Das Datenpaket ist leer.
    #[error("Das Datenpaket ist leer.")]
    DatenpaketLeer,
    /// Das Datenpaket enthaelt ein NUL-Byte.
    #[error("Das Datenpaket enthaelt ein NUL-Byte.")]
    DatenpaketNulByte,
    /// Zertifikatspfad, -datei oder PIN fehlt, oder ERiC lehnt das Zertifikat ab.
    #[error("{0}")]
    Zertifikat(String),
    /// ERiC kann das Datenpaket nicht entschluesseln.
    #[error("ERiC konnte das Datenpaket nicht entschluesseln (EricDekodiereDaten rc={rc}).")]
    Dekodieren { rc: i32 },
    /// ERiC liess sich nicht finden, laden oder initialisieren, oder ein Puffer fehlte.
    #[error(transparent)]
    Eric(#[from] EricFehler),
}

/// Zertifikatspfad und PIN als C-Strings; bricht VOR jedem ERiC-Aufruf ab, wenn eins fehlt.
fn zertifikat_als_c(zertifikat: &Zertifikat) -> Result<(CString, CString), AbrufFehler> {
    zertifikat
        .pruefe()
        .map_err(|e| AbrufFehler::Zertifikat(e.to_string()))?;
    let nul = || AbrufFehler::Zertifikat("Zertifikatspfad oder PIN enthaelt ein NUL-Byte".to_owned());
    let pfad = CString::new(zertifikat.pfad.as_os_str().as_bytes()).map_err(|_| nul())?;
    let pin = CString::new(zertifikat.pin.as_bytes()).map_err(|_| nul())?;
    Ok((pfad, pin))
}

/// `EricDekodiereDaten` auf einer geladenen Instanz mit offenem Zertifikat.
fn dekodiere_paket(
    eric: &ffi::Eric,
    handle: &ffi::ZertifikatWaechter<'_>,
    pin: &CStr,
    base64: &CStr,
) -> Result<Vec<u8>, AbrufFehler> {
    let (rc, klartext) = eric.dekodiere(handle, pin, base64)?;
    if rc != 0 {
        return Err(AbrufFehler::Dekodieren { rc });
    }
    Ok(klartext)
}

/// Entschluesselt ein abgeholtes Datenpaket (Base64, wie es in `<Datenpaket>` steht) mit dem
/// Zertifikat: `EricDekodiereDaten`. Ruft das Netz nie. Ein Fehler von ERiC ist ein benannter
/// Fehler, nie ein Panic; der Puffer von ERiC wird dann nicht gelesen.
///
/// # Errors
/// [`AbrufFehler`]; jeder ausser [`AbrufFehler::Eric`] und [`AbrufFehler::Dekodieren`] heisst: ERiC
/// wurde nicht geladen.
pub fn dekodiere_datenpaket(
    lib: Option<&Path>,
    zertifikat: &Zertifikat,
    base64: &str,
) -> Result<Vec<u8>, AbrufFehler> {
    if base64.is_empty() {
        return Err(AbrufFehler::DatenpaketLeer);
    }
    let base64_c = CString::new(base64).map_err(|_| AbrufFehler::DatenpaketNulByte)?;
    let (pfad_c, pin_c) = zertifikat_als_c(zertifikat)?;
    let lib = lib.ok_or(EricFehler::NichtGefunden)?;
    let log_dir = ffi::neues_log_dir()?;
    let eric = ffi::Eric::laden(lib, &log_dir)?;
    let handle = eric
        .zertifikat_oeffnen(&pfad_c)
        .map_err(|rc| AbrufFehler::Zertifikat(zertifikat_abgelehnt(rc)))?;
    dekodiere_paket(&eric, &handle, &pin_c, &base64_c)
}
