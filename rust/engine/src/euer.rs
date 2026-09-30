//! § 4 Abs. 3 `EStG` Einnahmenueberschussrechnung.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `EuerGewinn__EuerGewinn_in`.
#[derive(Debug, Clone, Copy)]
pub struct EuerEingabe {
    pub betriebseinnahmen: Cent,
    pub betriebsausgaben: Cent,
}

/// § 4 Abs. 3 `EStG`: Betriebseinnahmen abzueglich Betriebsausgaben auf den Gewinn. Kann negativ
/// sein (Verlustjahr).
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use engine::euer::{berechnen, EuerEingabe};
/// use domain::Cent;
/// let ergebnis = berechnen(EuerEingabe {
///     betriebseinnahmen: Cent::new(3_000_000),
///     betriebsausgaben: Cent::new(5_000_000),
/// })
/// .unwrap();
/// assert_eq!(ergebnis, Cent::new(-2_000_000));
/// ```
pub fn berechnen(eingabe: EuerEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::euer_gewinn(
        eingabe.betriebseinnahmen.get(),
        eingabe.betriebsausgaben.get(),
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, EuerEingabe};
    use domain::Cent;

    #[test]
    fn ausgaben_ueber_einnahmen_ergeben_verlust() {
        let ergebnis = berechnen(EuerEingabe {
            betriebseinnahmen: Cent::new(3_000_000),
            betriebsausgaben: Cent::new(5_000_000),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(-2_000_000));
    }
}
