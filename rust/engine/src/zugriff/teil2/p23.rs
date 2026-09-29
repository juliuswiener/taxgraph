//! § 23 Abs. 3 `EStG` -- private Veraeusserungsgeschaefte (`runner.py`, reines Python).
use domain::Euro;

use super::{euro, z, EngineFehler};

/// Eingabe fuer [`p23_veraeusserungsgewinn`]. Alle Felder Pflicht (Python `s[k]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VeraeusserungsgewinnEingabe {
    pub veraeusserungspreis: Euro,
    pub anschaffungs_herstellungskosten: Euro,
    pub werbungskosten: Euro,
}

/// § 23 Abs. 3 S. 1 `EStG`: Gewinn eines Geschaefts, EURO; darf negativ sein.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::p23::*;
/// # use domain::Euro;
/// let e = VeraeusserungsgewinnEingabe { veraeusserungspreis: Euro::new(1000),
///     anschaffungs_herstellungskosten: Euro::new(1200), werbungskosten: Euro::new(50) };
/// assert_eq!(p23_veraeusserungsgewinn(&e).unwrap(), Euro::new(-250));
/// ```
pub fn p23_veraeusserungsgewinn(e: &VeraeusserungsgewinnEingabe) -> Result<Euro, EngineFehler> {
    euro(z(e.veraeusserungspreis) - z(e.anschaffungs_herstellungskosten) - z(e.werbungskosten))
}

/// § 23 Abs. 3 S. 5 `EStG`: Freigrenze 1.000 EUR (keine Freibetrag), EURO.
/// `gesamtgewinn >= 1000 -> gesamtgewinn`, sonst 0.
///
/// ```
/// # use engine::zugriff::teil2::p23::*;
/// # use domain::Euro;
/// assert_eq!(p23_freigrenze(Euro::new(999)), Euro::new(0));
/// assert_eq!(p23_freigrenze(Euro::new(1000)), Euro::new(1000));
/// ```
#[must_use]
pub fn p23_freigrenze(gesamtgewinn: Euro) -> Euro {
    if gesamtgewinn.get() >= 1000 {
        gesamtgewinn
    } else {
        Euro::new(0)
    }
}

/// Eingabe fuer [`p23_verlusttopf`]. Alle Felder Pflicht (Python `s[k]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerlusttopfEingabe {
    pub gewinn_pvg: Euro,
    pub verlust_pvg: Euro,
}

/// § 23 Abs. 3 S. 7 `EStG`: Verlusttopf im selben Jahr, `max(0, gewinn - verlust)`, EURO.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::p23::*;
/// # use domain::Euro;
/// let e = VerlusttopfEingabe { gewinn_pvg: Euro::new(500), verlust_pvg: Euro::new(800) };
/// assert_eq!(p23_verlusttopf(&e).unwrap(), Euro::new(0));
/// ```
pub fn p23_verlusttopf(e: &VerlusttopfEingabe) -> Result<Euro, EngineFehler> {
    euro((z(e.gewinn_pvg) - z(e.verlust_pvg)).max(0))
}
