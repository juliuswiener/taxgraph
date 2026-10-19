//! Der Abruf der Lohnsteuerbescheinigung bei ELSTER (`ElsterVaStDaten`), Abweichung Nr. 91.
//!
//! STUMPF fuer den roten Test: noch keine Wirkung.

use std::path::Path;

use elster::EricFehler;

use crate::Zertifikat;

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

/// Stumpf.
///
/// # Errors
/// [`AbrufFehler`].
pub fn dekodiere_datenpaket(
    _lib: Option<&Path>,
    _zertifikat: &Zertifikat,
    _base64: &str,
) -> Result<Vec<u8>, AbrufFehler> {
    Err(AbrufFehler::DatenpaketLeer)
}
