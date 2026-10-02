//! § 24a `EStG` Altersentlastungsbetrag.
use catala_sys::CatalaFehler;
use domain::{Cent, Satz};

use crate::dezimal::{self, DezimalUeberlauf};

/// Eingabe fuer [`berechnen`], 1:1 `Altersentlastungsbetrag__Altersentlastungsbetrag_in`.
#[derive(Debug, Clone, Copy)]
pub struct AltersentlastungsbetragEingabe {
    pub arbeitslohn: Cent,
    pub positive_andere_einkuenfte: Cent,
    pub prozentsatz: Satz,
    pub hoechstbetrag: Cent,
}

/// [`berechnen`] kann an der Decimal->Bruch-Umwandlung ODER am Catala-Scope selbst scheitern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AltersentlastungsbetragFehler {
    #[error(transparent)]
    Dezimal(#[from] DezimalUeberlauf),
    #[error(transparent)]
    Catala(#[from] CatalaFehler),
}

/// § 24a `EStG`: Arbeitslohn, positive andere Einkuenfte, Prozentsatz und Hoechstbetrag auf den
/// Altersentlastungsbetrag.
///
/// # Errors
/// Siehe [`AltersentlastungsbetragFehler`].
///
/// ```
/// use engine::altersentlastungsbetrag::{berechnen, AltersentlastungsbetragEingabe};
/// use domain::{Cent, Satz};
/// use rust_decimal::Decimal;
/// // Grenzfall-Seed TestUnterHoechstbetrag: Bemessung 2000+1000=3000, Prozentsatz 20 %
/// // (als Prozentzahl, das Modul teilt intern /100) -> 600 < Hoechstbetrag 760 -> 600.
/// let ergebnis = berechnen(AltersentlastungsbetragEingabe {
///     arbeitslohn: Cent::new(200_000),
///     positive_andere_einkuenfte: Cent::new(100_000),
///     prozentsatz: Satz::new(Decimal::new(200, 1)),
///     hoechstbetrag: Cent::new(76_000),
/// })
/// .unwrap();
/// assert_eq!(ergebnis, Cent::new(60_000));
/// ```
pub fn berechnen(
    eingabe: AltersentlastungsbetragEingabe,
) -> Result<Cent, AltersentlastungsbetragFehler> {
    let (num, den) = dezimal::zu_bruch(eingabe.prozentsatz.get())?;
    let cent = catala_sys::altersentlastungsbetrag(
        eingabe.arbeitslohn.get(),
        eingabe.positive_andere_einkuenfte.get(),
        num,
        den,
        eingabe.hoechstbetrag.get(),
    )?;
    Ok(Cent::new(cent))
}

#[cfg(test)]
mod tests {
    use super::{berechnen, AltersentlastungsbetragEingabe};
    use domain::{Cent, Satz};
    use rust_decimal::Decimal;

    #[test]
    fn unter_hoechstbetrag_greift_der_prozentsatz() {
        // Grenzfall-Seed TestUnterHoechstbetrag: Bemessung 2000+1000=3000, Prozentsatz 20 %
        // (als Prozentzahl, das Modul teilt intern /100) -> 600 < Hoechstbetrag 760 -> 600.
        let ergebnis = berechnen(AltersentlastungsbetragEingabe {
            arbeitslohn: Cent::new(200_000),
            positive_andere_einkuenfte: Cent::new(100_000),
            prozentsatz: Satz::new(Decimal::new(200, 1)),
            hoechstbetrag: Cent::new(76_000),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(60_000));
    }
}
