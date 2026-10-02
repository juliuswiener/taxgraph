//! § 10 Abs. 1 Nr. 5/9, § 10 Abs. 1a Nr. 1 `EStG` -- Sonderausgaben je Kind bzw. Ex-Ehegatte
//! (`runner.py`, reines Python).
use domain::{Euro, Vz};

use super::{euro, int_mal_satz, z, EngineFehler};
use bindung::{Params, SatzHoechstbetrag};

/// Gemeinsamer Rechenweg von Kinderbetreuung und Schulgeld: `aufw <= 0 -> 0`, sonst
/// `min(int(aufw * satz), hb)`.
fn satz_mit_deckel(aufw: Euro, s: SatzHoechstbetrag, hb: i128) -> Result<Euro, EngineFehler> {
    if aufw.get() <= 0 {
        return Ok(Euro::new(0));
    }
    euro(int_mal_satz(aufw, s.abzugssatz)?.min(hb))
}

/// Eingabe fuer [`p10_1_5_kinderbetreuung`] (EIN Kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KinderbetreuungEingabe {
    /// PARITÄT: Python setzt fehlend = 2025. Bei `aufwendungen <= 0` liest Python den VZ nicht.
    pub vz: Vz,
    /// PARITÄT: Python setzt fehlend = 0.
    pub aufwendungen: Euro,
}

/// § 10 Abs. 1 Nr. 5 `EStG`: Kinderbetreuungskosten je Kind, EURO.
/// `min(int(aufw x abzugssatz), hoechstbetrag_je_kind)`.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::sonderausgaben::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = KinderbetreuungEingabe { vz: Vz::Vz2025, aufwendungen: Euro::new(3000) };
/// assert_eq!(p10_1_5_kinderbetreuung(&e, &p).unwrap(), Euro::new(2400));
/// ```
pub fn p10_1_5_kinderbetreuung(
    e: &KinderbetreuungEingabe,
    p: &Params,
) -> Result<Euro, EngineFehler> {
    let s = p.kinderbetreuung(e.vz)?;
    satz_mit_deckel(e.aufwendungen, s, z(s.hoechstbetrag_je_kind))
}

/// Eingabe fuer [`p10_1_9_schulgeld`] (EIN Kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchulgeldEingabe {
    /// PARITÄT: Python setzt fehlend = 2025. Bei `aufwendungen <= 0` liest Python den VZ nicht.
    pub vz: Vz,
    /// PARITÄT: Python setzt fehlend = 0.
    pub aufwendungen: Euro,
    /// Zusammenveranlagung verdoppelt den Hoechstbetrag. PARITÄT: Python setzt fehlend = False.
    pub splitting: bool,
}

/// § 10 Abs. 1 Nr. 9 `EStG`: Schulgeld je Kind, EURO.
/// `min(int(aufw x abzugssatz), hb)`, `hb` bei Splitting verdoppelt.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::sonderausgaben::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = SchulgeldEingabe { vz: Vz::Vz2025, aufwendungen: Euro::new(20000), splitting: true };
/// assert_eq!(p10_1_9_schulgeld(&e, &p).unwrap(), Euro::new(5000));
/// ```
pub fn p10_1_9_schulgeld(e: &SchulgeldEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let s = p.schulgeld(e.vz)?;
    let hb = z(s.hoechstbetrag_je_kind) * if e.splitting { 2 } else { 1 };
    satz_mit_deckel(e.aufwendungen, s, hb)
}

/// Eingabe fuer [`p10_1a_realsplitting`]. PARITÄT: Python setzt jedes fehlende Feld = 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealsplittingEingabe {
    pub unterhaltsleistungen: Euro,
    pub kv_pv_beitraege: Euro,
    /// Teilmenge von `kv_pv_beitraege` mit Krankengeldanspruch.
    pub kv_krankengeld: Euro,
}

/// § 10 Abs. 1a Nr. 1 `EStG`: Realsplitting, EURO.
/// `min(unterhalt, 13.805 + kv_pv - ceil(kv_krankengeld x 4 %))`; 13.805 hartkodiert wie in
/// Python.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::sonderausgaben::*;
/// # use domain::Euro;
/// let e = RealsplittingEingabe { unterhaltsleistungen: Euro::new(16000),
///     kv_pv_beitraege: Euro::new(2000), kv_krankengeld: Euro::new(2000) };
/// assert_eq!(p10_1a_realsplitting(&e).unwrap(), Euro::new(15725));
/// ```
pub fn p10_1a_realsplitting(e: &RealsplittingEingabe) -> Result<Euro, EngineFehler> {
    let kuerzung = -(-z(e.kv_krankengeld) * 4).div_euclid(100);
    let deckel = 13_805 + z(e.kv_pv_beitraege) - kuerzung;
    euro(z(e.unterhaltsleistungen).min(deckel))
}
