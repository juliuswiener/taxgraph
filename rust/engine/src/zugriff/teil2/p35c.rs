//! § 35c `EStG` -- energetische Sanierung (`runner.py`, reines Python). Saetze und
//! Hoechstbetraege hartkodiert wie in Python (7 %/6 %, 14.000/12.000, 50 %).
use domain::{Cent, Euro};

use super::{cent, euro, int_mal_float, z, EngineFehler};

/// Eingabe fuer [`p35c_sanierung`] und [`p35c_ermaessigung_cent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SanierungEingabe {
    /// PARITÄT: Python setzt fehlend = 0.
    pub sanierungsaufwendungen: Euro,
    /// PARITÄT: Python setzt fehlend = False (7 %/14.000 statt 6 %/12.000).
    pub ist_uebernaechstes_foerderjahr: bool,
}

/// § 35c Abs. 1 `EStG`: Sanierungsermaessigung, EURO. `min(int(aufw x 0.07), 14.000)`, im
/// uebernaechsten Foerderjahr `min(int(aufw x 0.06), 12.000)`.
///
/// PARITÄT: Float-Satz wie Python.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::p35c::*;
/// # use domain::Euro;
/// let e = SanierungEingabe { sanierungsaufwendungen: Euro::new(20000), ist_uebernaechstes_foerderjahr: true };
/// assert_eq!(p35c_sanierung(&e).unwrap(), Euro::new(1200));
/// ```
pub fn p35c_sanierung(e: &SanierungEingabe) -> Result<Euro, EngineFehler> {
    let (satz, hoechst) = if e.ist_uebernaechstes_foerderjahr { (0.06, 12_000) } else { (0.07, 14_000) };
    euro(int_mal_float(e.sanierungsaufwendungen, satz)?.min(hoechst))
}

/// § 35c Abs. 1 `EStG` als CENT-Zweig von `catala_est` (`runner.py`
/// `_p35c_ermaessigung_cent`): exakte Ganzzahl `min(aufw x 100 x satz // 100, hoechst x 100)`.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::p35c::*;
/// # use domain::{Cent, Euro};
/// let e = SanierungEingabe { sanierungsaufwendungen: Euro::new(20000), ist_uebernaechstes_foerderjahr: false };
/// assert_eq!(p35c_ermaessigung_cent(&e).unwrap(), Cent::new(140_000));
/// ```
pub fn p35c_ermaessigung_cent(e: &SanierungEingabe) -> Result<Cent, EngineFehler> {
    let (satz, hoechst) = if e.ist_uebernaechstes_foerderjahr { (6, 12_000) } else { (7, 14_000) };
    cent((z(e.sanierungsaufwendungen) * 100 * satz).div_euclid(100).min(hoechst * 100))
}

/// § 35c Abs. 1 S. 4 `EStG`: Energieberater 50 %, EURO. `int(aufw x 0.50)`.
///
/// `energieberater_aufwendungen`: PARITÄT: Python setzt fehlend = 0.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::p35c::*;
/// # use domain::Euro;
/// assert_eq!(p35c_energieberater(Euro::new(1001)).unwrap(), Euro::new(500));
/// ```
pub fn p35c_energieberater(energieberater_aufwendungen: Euro) -> Result<Euro, EngineFehler> {
    euro(int_mal_float(energieberater_aufwendungen, 0.50)?)
}

/// Eingabe fuer [`p35c_jahresdeckel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JahresdeckelEingabe {
    /// PARITÄT: Python setzt fehlend = 0.
    pub sanierung_ermaessigung: Euro,
    /// PARITÄT: Python setzt fehlend = 0.
    pub energieberater_ermaessigung: Euro,
    /// PARITÄT: Python setzt fehlend = False (Deckel 14.000 statt 12.000).
    pub ist_uebernaechstes_foerderjahr: bool,
}

/// § 35c Jahresdeckel fuer Sanierung + Energieberater, EURO: `min(s + e, HB)`.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::p35c::*;
/// # use domain::Euro;
/// let e = JahresdeckelEingabe { sanierung_ermaessigung: Euro::new(13000),
///     energieberater_ermaessigung: Euro::new(2000), ist_uebernaechstes_foerderjahr: false };
/// assert_eq!(p35c_jahresdeckel(&e).unwrap(), Euro::new(14000));
/// ```
pub fn p35c_jahresdeckel(e: &JahresdeckelEingabe) -> Result<Euro, EngineFehler> {
    let hb = if e.ist_uebernaechstes_foerderjahr { 12_000 } else { 14_000 };
    let summe = z(e.sanierung_ermaessigung) + z(e.energieberater_ermaessigung);
    euro(if summe > hb { hb } else { summe })
}
