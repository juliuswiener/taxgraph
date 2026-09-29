//! §§ 3, 4 `SolzG` 1995 -- Solidaritaetszuschlag natuerlicher Personen (`runner.py`, reines
//! Python; der Catala-Scope `Solzg` wird dort nicht gerufen).
use domain::{Cent, Euro, Vz};

use super::{cent, z, EngineFehler};

/// Freigrenze (einzel, zusammen) je VZ. PARITÄT: Python hartkodiert (`runner.py`
/// `_SOLZ_FREIGRENZE`), nicht aus `params/<vz>`.
const fn freigrenze(vz: Vz) -> (i128, i128) {
    match vz {
        Vz::Vz2024 => (18_130, 36_260),
        Vz::Vz2025 => (19_950, 39_900),
        Vz::Vz2026 => (20_350, 40_700),
    }
}

/// Eingabe fuer [`solz`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolzEingabe {
    pub vz: Vz,
    /// `KiFB`-fiktive `ESt` (§ 3 Abs. 2), EURO.
    pub bemessungsgrundlage: Euro,
    /// § 32d-Abgeltung (§ 3 Abs. 3 S. 2), EURO. PARITÄT: Python setzt fehlend = 0.
    pub kapital_steuer: Euro,
    /// Zusammenveranlagung: doppelte Freigrenze.
    pub splitting: bool,
}

/// §§ 3, 4 `SolzG`: Solidaritaetszuschlag, CENT.
///
/// `basis = max(0, bemessungsgrundlage - kapital_steuer)`; bis zur Freigrenze 0, sonst
/// `min(basis x 5,5 %, (basis - Freigrenze) x 11,9 %)` in Cent abgerundet; dazu 5,5 % der
/// Kapital-Steuer ohne Freigrenze.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::solz::*;
/// # use domain::{Cent, Euro, Vz};
/// let e = SolzEingabe { vz: Vz::Vz2025, bemessungsgrundlage: Euro::new(40000),
///     kapital_steuer: Euro::new(0), splitting: false };
/// assert_eq!(solz(&e).unwrap(), Cent::new(220_000));
/// ```
pub fn solz(e: &SolzEingabe) -> Result<Cent, EngineFehler> {
    let kap = z(e.kapital_steuer);
    let basis = (z(e.bemessungsgrundlage) - kap).max(0);
    let (einzel, zusammen) = freigrenze(e.vz);
    let grenze = if e.splitting { zusammen } else { einzel };
    let haupt = if basis <= grenze {
        0
    } else {
        let regel = (basis * 55).div_euclid(10);
        let milderung = ((basis - grenze) * 119).div_euclid(10);
        regel.min(milderung)
    };
    cent(haupt + (kap * 55).div_euclid(10))
}
