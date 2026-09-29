//! § 33 `EStG` aussergewoehnliche Belastungen (`catala_p33_zumutbar`, `catala_p33_agb`).
use domain::{Cent, Euro};

use super::fehler::{in_cent, EngineFehler};
use crate::agb::{self, AgbAbzugEingabe};
use crate::zumutbare_belastung::{self, ZumutbareBelastungEingabe};

/// Eingabe fuer [`p33_zumutbar`].
#[derive(Debug, Clone, Copy)]
pub struct P33ZumutbarEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: zumutbare Belastung 0)
    pub gesamtbetrag_der_einkuenfte: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: hoechster Satz)
    pub anzahl_kinder: i64,
    /// PARITÄT: Python setzt fehlend = False (fail-closed: Einzelsatz)
    pub splitting: bool,
}

/// § 33 Abs. 3 zumutbare Belastung, cent-genau (Catala-`Money`), fuer die Verkettung.
fn zumutbar_cent(e: &P33ZumutbarEingabe) -> Result<Cent, EngineFehler> {
    Ok(zumutbare_belastung::berechnen(ZumutbareBelastungEingabe {
        gesamtbetrag_der_einkuenfte: in_cent(e.gesamtbetrag_der_einkuenfte)?,
        anzahl_kinder: e.anzahl_kinder,
        splitting: e.splitting,
    })?)
}

/// `catala_p33_zumutbar` -- § 33 Abs. 3 `EStG` zumutbare Belastung (Tranchen-Methode, 1-7 %
/// nach Kinderzahl/Splitting), EURO (abgerundet).
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::belastungen::{p33_zumutbar, P33ZumutbarEingabe};
/// let e = P33ZumutbarEingabe { gesamtbetrag_der_einkuenfte: Euro::new(0), anzahl_kinder: 0, splitting: false };
/// assert_eq!(p33_zumutbar(&e).unwrap(), Euro::new(0));
/// ```
pub fn p33_zumutbar(e: &P33ZumutbarEingabe) -> Result<Euro, EngineFehler> {
    Ok(zumutbar_cent(e)?.floor_euro())
}

/// Eingabe fuer [`p33_agb`].
#[derive(Debug, Clone, Copy)]
pub struct P33AgbEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub aussergewoehnliche_belastungen: Euro,
    /// Staffel-Eingaben der zumutbaren Belastung.
    pub zumutbar: P33ZumutbarEingabe,
}

/// `catala_p33_agb` -- § 33 Abs. 1 `EStG`: agB minus zumutbare Belastung, mindestens 0, EURO.
/// Die zumutbare Belastung bleibt cent-genau bis zum Endergebnis (EIN Rundungsschritt).
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::belastungen::{p33_agb, P33AgbEingabe, P33ZumutbarEingabe};
/// let e = P33AgbEingabe {
///     aussergewoehnliche_belastungen: Euro::new(1000),
///     zumutbar: P33ZumutbarEingabe { gesamtbetrag_der_einkuenfte: Euro::new(0), anzahl_kinder: 0, splitting: false },
/// };
/// assert_eq!(p33_agb(&e).unwrap(), Euro::new(1000));
/// ```
pub fn p33_agb(e: &P33AgbEingabe) -> Result<Euro, EngineFehler> {
    let agb_cent = in_cent(e.aussergewoehnliche_belastungen)?;
    let c = agb::berechnen(AgbAbzugEingabe {
        aussergewoehnliche_belastungen: agb_cent,
        zumutbare_belastung: zumutbar_cent(&e.zumutbar)?,
    })?;
    Ok(c.floor_euro())
}
