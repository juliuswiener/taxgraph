//! Jahreswerte aus `params/<vz>`, die runner.py als eigene Accessoren anbietet.
use bindung::Params;
use domain::{Euro, Vz};

use super::fehler::EngineFehler;

/// `catala_grundfreibetrag` -- § 32a Abs. 1 S. 2 Nr. 1 `EStG` Grundfreibetrag des VZ, EURO.
///
/// # Errors
/// [`EngineFehler::Params`], wenn der Wert in `params/<vz>` fehlt.
///
/// ```
/// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let gfb = engine::zugriff::teil1::pauschbetraege::grundfreibetrag(domain::Vz::Vz2026, &p).unwrap();
/// assert_eq!(gfb, domain::Euro::new(12_348));
/// ```
pub fn grundfreibetrag(vz: Vz, p: &Params) -> Result<Euro, EngineFehler> {
    Ok(p.grundfreibetrag(vz)?)
}

/// `catala_arbeitnehmer_pauschbetrag` -- § 9a S. 1 Nr. 1a `EStG` Arbeitnehmer-Pauschbetrag des
/// VZ, EURO.
///
/// # Errors
/// [`EngineFehler::Params`], wenn der Wert in `params/<vz>` fehlt.
///
/// ```
/// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let ap = engine::zugriff::teil1::pauschbetraege::arbeitnehmer_pauschbetrag(domain::Vz::Vz2025, &p).unwrap();
/// assert_eq!(ap, domain::Euro::new(1230));
/// ```
pub fn arbeitnehmer_pauschbetrag(vz: Vz, p: &Params) -> Result<Euro, EngineFehler> {
    Ok(p.arbeitnehmer_pauschbetrag(vz)?)
}
