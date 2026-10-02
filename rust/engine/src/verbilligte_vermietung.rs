//! § 21 Abs. 2 `EStG` verbilligte Vermietung.
use catala_sys::CatalaFehler;
use domain::{Cent, Satz};

use crate::dezimal::{self, DezimalUeberlauf};

/// Eingabe fuer [`berechnen`], 1:1 `VerbilligteVermietungWk__VerbilligteVermietungWk_in`.
#[derive(Debug, Clone, Copy)]
pub struct VerbilligteVermietungEingabe {
    pub werbungskosten: Cent,
    pub entgelt_quote_prozent: Satz,
}

/// [`berechnen`] kann an der Decimal->Bruch-Umwandlung ODER am Catala-Scope selbst scheitern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VerbilligteVermietungFehler {
    #[error(transparent)]
    Dezimal(#[from] DezimalUeberlauf),
    #[error(transparent)]
    Catala(#[from] CatalaFehler),
}

/// § 21 Abs. 2 `EStG`: Werbungskosten und Entgeltquote auf die abziehbaren Werbungskosten.
///
/// # Errors
/// Siehe [`VerbilligteVermietungFehler`].
///
/// ```
/// use engine::verbilligte_vermietung::{berechnen, VerbilligteVermietungEingabe};
/// use domain::{Cent, Satz};
/// use rust_decimal::Decimal;
/// let ergebnis = berechnen(VerbilligteVermietungEingabe {
///     werbungskosten: Cent::new(100_000),
///     entgelt_quote_prozent: Satz::new(Decimal::new(100, 0)),
/// })
/// .unwrap();
/// assert_eq!(ergebnis, Cent::new(100_000));
/// ```
pub fn berechnen(
    eingabe: VerbilligteVermietungEingabe,
) -> Result<Cent, VerbilligteVermietungFehler> {
    let (num, den) = dezimal::zu_bruch(eingabe.entgelt_quote_prozent.get())?;
    let cent = catala_sys::verbilligte_vermietung_wk(eingabe.werbungskosten.get(), num, den)?;
    Ok(Cent::new(cent))
}

#[cfg(test)]
mod tests {
    use super::{berechnen, VerbilligteVermietungEingabe};
    use domain::{Cent, Satz};
    use rust_decimal::Decimal;

    #[test]
    fn volle_quote_laesst_werbungskosten_unveraendert() {
        let ergebnis = berechnen(VerbilligteVermietungEingabe {
            werbungskosten: Cent::new(100_000),
            entgelt_quote_prozent: Satz::new(Decimal::new(100, 0)),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(100_000));
    }
}
