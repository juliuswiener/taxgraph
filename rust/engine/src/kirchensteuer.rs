//! § 10 Abs. 1 Nr. 4 `EStG` abziehbare Kirchensteuer.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `Kirchensteuerabzug__Kirchensteuerabzug_in`.
#[derive(Debug, Clone, Copy)]
pub struct KirchensteuerabzugEingabe {
    pub gezahlte_kirchensteuer: Cent,
    pub erstattete_kirchensteuer: Cent,
}

/// § 10 Abs. 1 Nr. 4 `EStG`: gezahlte abzueglich erstatteter Kirchensteuer.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn berechnen(eingabe: KirchensteuerabzugEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::kirchensteuerabzug(
        eingabe.gezahlte_kirchensteuer.get(),
        eingabe.erstattete_kirchensteuer.get(),
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, KirchensteuerabzugEingabe};
    use domain::Cent;

    #[test]
    fn erstattung_mindert_den_abzug() {
        let ergebnis = berechnen(KirchensteuerabzugEingabe {
            gezahlte_kirchensteuer: Cent::new(1_000),
            erstattete_kirchensteuer: Cent::new(300),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(700));
    }
}
