//! § 10 Abs. 1 Nr. 3 `EStG` Basiskranken- und Pflegeversicherung.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `KrankenPflegeVorsorge__KrankenPflegeVorsorge_in`.
#[derive(Debug, Clone, Copy)]
pub struct VorsorgeaufwendungenEingabe {
    /// Bei Zusammenveranlagung die Summe beider Ehegatten.
    pub basis: Cent,
    /// Bei Zusammenveranlagung die Summe beider Ehegatten.
    pub weitere: Cent,
    /// Anspruch auf Zuschuss von Person A.
    pub mit_zuschuss: bool,
    /// Zusammenveranlagung: es gilt der gemeinsame Hoechstbetrag (§ 10 Abs. 4 S. 3 `EStG`).
    pub zusammenveranlagt: bool,
    /// Anspruch auf Zuschuss von Person B; zaehlt nur bei `zusammenveranlagt`.
    pub mit_zuschuss_ehegatte: bool,
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
///     zusammenveranlagt: false,
///     mit_zuschuss_ehegatte: false,
/// })
/// .unwrap();
/// assert_eq!(ergebnis, Cent::new(50_000));
/// ```
pub fn berechnen(eingabe: VorsorgeaufwendungenEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::kranken_pflege_vorsorge(
        eingabe.basis.get(),
        eingabe.weitere.get(),
        eingabe.mit_zuschuss,
        eingabe.zusammenveranlagt,
        eingabe.mit_zuschuss_ehegatte,
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
            zusammenveranlagt: false,
            mit_zuschuss_ehegatte: false,
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(50_000));
    }
}
