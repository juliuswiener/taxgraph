//! § 20 Abs. 6/9, § 32d Abs. 1 `EStG` -- Kapitaleinkuenfte (`runner.py` Weg A, reines Python).
use domain::{Euro, Vz};

use bindung::Params;

use super::{euro, z, EngineFehler};

/// Eingabe fuer [`sparer_pb`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparerPbEingabe {
    pub vz: Vz,
    /// PARITÄT: Python setzt fehlend = 0.
    pub kapitalertraege: Euro,
    /// PARITÄT: Python setzt fehlend = False (Einzel-Pauschbetrag).
    pub zusammenveranlagung: bool,
}

/// § 20 Abs. 9 `EStG`: Kapitalertraege nach Sparer-Pauschbetrag, EURO.
/// `max(0, kapitalertraege - pausch)`; `pausch` bei Zusammenveranlagung verdoppelt (S. 3).
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::kapital::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = SparerPbEingabe { vz: Vz::Vz2025, kapitalertraege: Euro::new(2500), zusammenveranlagung: true };
/// assert_eq!(sparer_pb(&e, &p).unwrap(), Euro::new(500));
/// ```
pub fn sparer_pb(e: &SparerPbEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let pausch = z(p.sparer_pauschbetrag(e.vz)?) * if e.zusammenveranlagung { 2 } else { 1 };
    euro((z(e.kapitalertraege) - pausch).max(0))
}

/// Eingabe fuer [`kapital_verrechnung`]. PARITÄT: Python setzt jedes fehlende Feld = 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KapitalVerrechnungEingabe {
    pub gewinn_aktien: Euro,
    pub verlust_aktien: Euro,
    pub gewinn_sonstige: Euro,
    pub verlust_sonstige: Euro,
}

/// § 20 Abs. 6 `EStG`: zwei getrennte Verlusttoepfe, je Boden 0, EURO. Keine
/// topfuebergreifende Verrechnung.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::kapital::*;
/// # use domain::Euro;
/// let e = KapitalVerrechnungEingabe {
///     gewinn_aktien: Euro::new(100), verlust_aktien: Euro::new(300),
///     gewinn_sonstige: Euro::new(500), verlust_sonstige: Euro::new(200),
/// };
/// assert_eq!(kapital_verrechnung(&e).unwrap(), Euro::new(300));
/// ```
pub fn kapital_verrechnung(e: &KapitalVerrechnungEingabe) -> Result<Euro, EngineFehler> {
    let aktien = (z(e.gewinn_aktien) - z(e.verlust_aktien)).max(0);
    let sonstige = (z(e.gewinn_sonstige) - z(e.verlust_sonstige)).max(0);
    euro(aktien + sonstige)
}

/// Eingabe fuer [`kapital_steuer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KapitalSteuerEingabe {
    pub vz: Vz,
    /// PARITÄT: Python setzt fehlend = 0.
    pub kapitaleinkuenfte: Euro,
    /// PARITÄT: Python setzt fehlend = 0.
    pub est_regulaer_mit_kap: Euro,
    /// PARITÄT: Python setzt fehlend = 0.
    pub est_regulaer_ohne_kap: Euro,
}

/// § 32d Abs. 1/6 `EStG`: `min(kap * satz // 100, est_mit - est_ohne)`, EURO. Die beiden
/// `ESt`-Groessen rechnet der Aufrufer.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::kapital::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = KapitalSteuerEingabe { vz: Vz::Vz2025, kapitaleinkuenfte: Euro::new(1000),
///     est_regulaer_mit_kap: Euro::new(5300), est_regulaer_ohne_kap: Euro::new(5000) };
/// assert_eq!(kapital_steuer(&e, &p).unwrap(), Euro::new(250));
/// ```
pub fn kapital_steuer(e: &KapitalSteuerEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let satz = i128::from(p.abgeltungssatz_prozent(e.vz)?);
    let abgeltung = (z(e.kapitaleinkuenfte) * satz).div_euclid(100);
    let delta = z(e.est_regulaer_mit_kap) - z(e.est_regulaer_ohne_kap);
    euro(abgeltung.min(delta))
}
