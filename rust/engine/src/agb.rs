//! § 33 Abs. 1 `EStG` aussergewoehnliche Belastungen abzueglich zumutbarer Belastung.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `AgbAbzug__AgbAbzug_in`. `zumutbare_belastung` kommt aus
/// [`crate::zumutbare_belastung::berechnen`] -- ein eigener Catala-Scope, kein Teil dieses.
#[derive(Debug, Clone, Copy)]
pub struct AgbAbzugEingabe {
    pub aussergewoehnliche_belastungen: Cent,
    pub zumutbare_belastung: Cent,
}

/// § 33 Abs. 1 `EStG`: aussergewoehnliche Belastungen abzueglich zumutbarer Belastung.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn berechnen(eingabe: AgbAbzugEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::agb_abzug(
        eingabe.aussergewoehnliche_belastungen.get(),
        eingabe.zumutbare_belastung.get(),
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, AgbAbzugEingabe};
    use domain::Cent;

    #[test]
    fn agb_unter_zumutbarer_belastung_ist_null() {
        let ergebnis = berechnen(AgbAbzugEingabe {
            aussergewoehnliche_belastungen: Cent::new(100),
            zumutbare_belastung: Cent::new(500),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(0));
    }
}
