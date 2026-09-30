//! § 9 Abs. 1 S. 3 Nr. 4/4a, Abs. 2 `EStG` Entfernungspauschale.
use catala_sys::{CatalaFehler, EntfernungspauschaleErgebnis};
use domain::Cent;
use rust_decimal::Decimal;

use crate::dezimal::{self, DezimalUeberlauf};

/// Eingabe fuer [`berechnen`], 1:1 `Entfernungspauschale.Berechnung__Berechnung_in`.
#[derive(Debug, Clone, Copy)]
pub struct EntfernungspauschaleEingabe {
    pub entfernung_km_roh: Decimal,
    pub arbeitstage: i64,
    pub eigenes_oder_ueberlassenes_kfz: bool,
    pub oepnv_kosten_jahr: Cent,
    pub satz_bis_20_km: Cent,
    pub satz_ab_21_km: Cent,
    pub staffelgrenze_km: i64,
    pub hoechstbetrag: Cent,
}

/// [`berechnen`] kann an der Decimal->Bruch-Umwandlung ODER am Catala-Scope selbst scheitern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EntfernungspauschaleFehler {
    #[error(transparent)]
    Dezimal(#[from] DezimalUeberlauf),
    #[error(transparent)]
    Catala(#[from] CatalaFehler),
}

/// § 9 Abs. 1 S. 3 Nr. 4/4a `EStG`: einfache Entfernung, Arbeitstage, Kfz-Flag, OePNV-Kosten,
/// VZ-Saetze und Hoechstbetrag auf die Entfernungspauschale (vor und nach Guenstigerpruefung).
///
/// # Errors
/// Siehe [`EntfernungspauschaleFehler`].
///
/// ```
/// use engine::entfernungspauschale::{berechnen, EntfernungspauschaleEingabe};
/// use domain::Cent;
/// use rust_decimal::Decimal;
/// let ergebnis = berechnen(EntfernungspauschaleEingabe {
///     entfernung_km_roh: Decimal::new(106, 1),
///     arbeitstage: 200,
///     eigenes_oder_ueberlassenes_kfz: false,
///     oepnv_kosten_jahr: Cent::new(0),
///     satz_bis_20_km: Cent::new(30),
///     satz_ab_21_km: Cent::new(38),
///     staffelgrenze_km: 20,
///     hoechstbetrag: Cent::new(450_000),
/// })
/// .unwrap();
/// assert_eq!(ergebnis.entfernungspauschale_cent, 60_000);
/// ```
pub fn berechnen(
    eingabe: EntfernungspauschaleEingabe,
) -> Result<EntfernungspauschaleErgebnis, EntfernungspauschaleFehler> {
    let (num, den) = dezimal::zu_bruch(eingabe.entfernung_km_roh)?;
    let ergebnis = catala_sys::entfernungspauschale(catala_sys::EntfernungspauschaleEingabe {
        entfernung_km_roh_num: num,
        entfernung_km_roh_den: den,
        arbeitstage: eingabe.arbeitstage,
        eigenes_oder_ueberlassenes_kfz: eingabe.eigenes_oder_ueberlassenes_kfz,
        oepnv_kosten_jahr_cent: eingabe.oepnv_kosten_jahr.get(),
        satz_bis_20_km_cent: eingabe.satz_bis_20_km.get(),
        satz_ab_21_km_cent: eingabe.satz_ab_21_km.get(),
        staffelgrenze_km: eingabe.staffelgrenze_km,
        hoechstbetrag_cent: eingabe.hoechstbetrag.get(),
    })?;
    Ok(ergebnis)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, EntfernungspauschaleEingabe};
    use domain::Cent;
    use rust_decimal::Decimal;

    #[test]
    fn angefangener_km_bleibt_unberuecksichtigt() {
        let ergebnis = berechnen(EntfernungspauschaleEingabe {
            entfernung_km_roh: Decimal::new(106, 1),
            arbeitstage: 200,
            eigenes_oder_ueberlassenes_kfz: false,
            oepnv_kosten_jahr: Cent::new(0),
            satz_bis_20_km: Cent::new(30),
            satz_ab_21_km: Cent::new(38),
            staffelgrenze_km: 20,
            hoechstbetrag: Cent::new(450_000),
        })
        .unwrap();
        assert_eq!(ergebnis.entfernungspauschale_cent, 60_000);
    }
}
