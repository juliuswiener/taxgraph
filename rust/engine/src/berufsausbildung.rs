//! § 10 Abs. 1 Nr. 7 `EStG` Berufsausbildungsaufwendungen.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `Berufsausbildung__Berufsausbildung_in`.
#[derive(Debug, Clone, Copy)]
pub struct BerufsausbildungEingabe {
    pub aufwendungen: Cent,
}

/// § 10 Abs. 1 Nr. 7 `EStG`: Aufwendungen auf die abziehbaren Sonderausgaben.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn berechnen(eingabe: BerufsausbildungEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::berufsausbildung(eingabe.aufwendungen.get()).map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, BerufsausbildungEingabe};
    use domain::Cent;

    #[test]
    fn unterhalb_hoechstbetrag_voll_abziehbar() {
        let ergebnis = berechnen(BerufsausbildungEingabe {
            aufwendungen: Cent::new(10_000),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(10_000));
    }
}
