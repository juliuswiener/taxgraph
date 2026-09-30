//! § 10 Abs. 1 Nr. 3 `EStG` Basiskranken- und Pflegeversicherung.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `KrankenPflegeVorsorge__KrankenPflegeVorsorge_in`.
#[derive(Debug, Clone, Copy)]
pub struct VorsorgeaufwendungenEingabe {
    pub basis: Cent,
    pub weitere: Cent,
    pub mit_zuschuss: bool,
}

/// § 10 Abs. 1 Nr. 3 `EStG`: Basisbeitrag, weitere Vorsorgeaufwendungen und Zuschuss-Anspruch auf
/// die abziehbare Kranken-/Pflegevorsorge.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use engine::vorsorgeaufwendungen::{berechnen, VorsorgeaufwendungenEingabe};
/// use domain::Cent;
/// let ergebnis = berechnen(VorsorgeaufwendungenEingabe {
///     basis: Cent::new(50_000),
///     weitere: Cent::new(0),
///     mit_zuschuss: false,
/// })
/// .unwrap();
/// assert_eq!(ergebnis, Cent::new(50_000));
/// ```
pub fn berechnen(eingabe: VorsorgeaufwendungenEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::kranken_pflege_vorsorge(
        eingabe.basis.get(),
        eingabe.weitere.get(),
        eingabe.mit_zuschuss,
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, VorsorgeaufwendungenEingabe};
    use domain::Cent;

    #[test]
    fn basisbeitrag_ohne_zuschuss_ist_voll_abziehbar() {
        let ergebnis = berechnen(VorsorgeaufwendungenEingabe {
            basis: Cent::new(50_000),
            weitere: Cent::new(0),
            mit_zuschuss: false,
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(50_000));
    }
}
