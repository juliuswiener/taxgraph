//! § 32b, § 34c, § 22 Nr. 3 `EStG` und § 6 Abs. 1 Nr. 4 `EStG` (Kfz-Nutzungswert) -- kleine
//! Accessoren ohne Parameterdatei (`runner.py`, reines Python).
use domain::{Cent, Euro};

use super::{cent, euro, floor_div, z, EngineFehler};

/// Eingabe fuer [`p32b_1`]. Alle Felder Pflicht (Python `s[k]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressionsvorbehaltEingabe {
    pub zu_versteuerndes_einkommen: Euro,
    pub progressionseinkuenfte: Euro,
    pub est_auf_erhoehte_bemessung: Euro,
}

/// § 32b Abs. 1 `EStG`: Progressionsvorbehalt, EURO.
/// `erhoehte = zvE + pe`; `erhoehte <= 0 -> 0`; sonst `est_erhoeht x zvE // erhoehte`.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::sonstige::*;
/// # use domain::Euro;
/// let e = ProgressionsvorbehaltEingabe { zu_versteuerndes_einkommen: Euro::new(30000),
///     progressionseinkuenfte: Euro::new(10000), est_auf_erhoehte_bemessung: Euro::new(7209) };
/// assert_eq!(p32b_1(&e).unwrap(), Euro::new(5406));
/// ```
pub fn p32b_1(e: &ProgressionsvorbehaltEingabe) -> Result<Euro, EngineFehler> {
    let zve = z(e.zu_versteuerndes_einkommen);
    let erhoehte = zve + z(e.progressionseinkuenfte);
    if erhoehte <= 0 {
        return Ok(Euro::new(0));
    }
    euro(floor_div(z(e.est_auf_erhoehte_bemessung) * zve, erhoehte)?)
}

/// Eingabe fuer [`p34c_1`]. Alle Felder Pflicht (Python `s[k]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuslaendischeSteuerEingabe {
    pub gezahlte_auslaendische_steuer: Euro,
    pub deutsche_est_inkl_ausl: Euro,
    pub zu_versteuerndes_einkommen: Euro,
    pub auslaendische_einkuenfte_staat: Euro,
}

/// § 34c Abs. 1 `EStG`: Anrechnung auslaendischer Steuer (ein Staat), EURO.
/// `ausl <= 0` oder `zvE <= 0` -> 0; sonst `min(gezahlt, est x ausl // zvE)`.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::sonstige::*;
/// # use domain::Euro;
/// let e = AuslaendischeSteuerEingabe { gezahlte_auslaendische_steuer: Euro::new(10000),
///     deutsche_est_inkl_ausl: Euro::new(16000), zu_versteuerndes_einkommen: Euro::new(60000),
///     auslaendische_einkuenfte_staat: Euro::new(30000) };
/// assert_eq!(p34c_1(&e).unwrap(), Euro::new(8000));
/// ```
pub fn p34c_1(e: &AuslaendischeSteuerEingabe) -> Result<Euro, EngineFehler> {
    let zve = z(e.zu_versteuerndes_einkommen);
    let ausl = z(e.auslaendische_einkuenfte_staat);
    if ausl <= 0 || zve <= 0 {
        return Ok(Euro::new(0));
    }
    let hoechst = floor_div(z(e.deutsche_est_inkl_ausl) * ausl, zve)?;
    euro(z(e.gezahlte_auslaendische_steuer).min(hoechst))
}

/// § 22 Nr. 3 S. 2 `EStG`: Freigrenze 256 EUR fuer sonstige Leistungen, CENT.
/// Unter 25.600 Cent -> 0, sonst der volle Betrag.
///
/// ```
/// # use engine::zugriff::teil2::sonstige::*;
/// # use domain::Cent;
/// assert_eq!(p22_nr3_einkuenfte(Cent::new(25_599)), Cent::new(0));
/// assert_eq!(p22_nr3_einkuenfte(Cent::new(25_600)), Cent::new(25_600));
/// ```
#[must_use]
pub fn p22_nr3_einkuenfte(betrag: Cent) -> Cent {
    if betrag.get() < 25_600 {
        Cent::new(0)
    } else {
        betrag
    }
}

/// Eingabe fuer [`kfz_nutzungswert_monat_cent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KfzNutzungswertEingabe {
    pub bruttolistenpreis: Euro,
    /// 1 voll, 2 halb, 4 viertel. PARITÄT: Python setzt fehlend = 1.
    pub bruchteils_teiler: i64,
}

/// § 6 Abs. 1 Nr. 4 S. 2 `EStG` 1 %-Regel: Monatswert `BLP x 100 // 100 // teiler`, CENT
/// (`runner.py` `_kfz_nutzungswert_monat_cent`, Zweig von `catala_est`).
///
/// # Errors
/// [`EngineFehler::DivisionDurchNull`] bei Teiler 0 (Python `ZeroDivisionError`).
///
/// ```
/// # use engine::zugriff::teil2::sonstige::*;
/// # use domain::{Cent, Euro};
/// let e = KfzNutzungswertEingabe { bruttolistenpreis: Euro::new(50000), bruchteils_teiler: 4 };
/// assert_eq!(kfz_nutzungswert_monat_cent(&e).unwrap(), Cent::new(12500));
/// ```
pub fn kfz_nutzungswert_monat_cent(e: &KfzNutzungswertEingabe) -> Result<Cent, EngineFehler> {
    let blp_cent = z(e.bruttolistenpreis) * 100;
    cent(floor_div(blp_cent.div_euclid(100), i128::from(e.bruchteils_teiler))?)
}
