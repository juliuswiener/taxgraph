//! § 10b `EStG` Spendenabzug.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `SpendenAbzug__SpendenAbzug_in`.
#[derive(Debug, Clone, Copy)]
pub struct SpendenAbzugEingabe {
    pub zuwendungen: Cent,
    pub gesamtbetrag_der_einkuenfte: Cent,
}

/// § 10b `EStG`: Zuwendungen und Gesamtbetrag der Einkuenfte auf den abziehbaren Spendenabzug.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use engine::spenden::{berechnen, SpendenAbzugEingabe};
/// use domain::Cent;
/// let ergebnis = berechnen(SpendenAbzugEingabe {
///     zuwendungen: Cent::new(10_000),
///     gesamtbetrag_der_einkuenfte: Cent::new(1_000_000),
/// })
/// .unwrap();
/// assert_eq!(ergebnis, Cent::new(10_000));
/// ```
pub fn berechnen(eingabe: SpendenAbzugEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::spenden_abzug(
        eingabe.zuwendungen.get(),
        eingabe.gesamtbetrag_der_einkuenfte.get(),
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, SpendenAbzugEingabe};
    use domain::Cent;

    #[test]
    fn spende_unter_deckel_voll_abziehbar() {
        let ergebnis = berechnen(SpendenAbzugEingabe {
            zuwendungen: Cent::new(10_000),
            gesamtbetrag_der_einkuenfte: Cent::new(1_000_000),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(10_000));
    }
}
