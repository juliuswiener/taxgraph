//! §§ 31, 32 Abs. 6 `EStG` Familienleistungsausgleich (Guenstigerpruefung Kindergeld vs.
//! Kinderfreibetrag).
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `Familienleistungsausgleich__Familienleistungsausgleich_in`.
#[derive(Debug, Clone, Copy)]
pub struct FamilienleistungsausgleichEingabe {
    pub est_ohne_freibetraege: Cent,
    pub est_mit_freibetraegen: Cent,
    pub kindergeld: Cent,
}

/// §§ 31, 32 Abs. 6 `EStG`: `ESt` ohne/mit Freibetraegen und Kindergeld auf die `ESt` nach
/// Familienleistungsausgleich.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn berechnen(eingabe: FamilienleistungsausgleichEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::familienleistungsausgleich(
        eingabe.est_ohne_freibetraege.get(),
        eingabe.est_mit_freibetraegen.get(),
        eingabe.kindergeld.get(),
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, FamilienleistungsausgleichEingabe};
    use domain::Cent;

    #[test]
    fn freibetrag_guenstiger_zieht_kindergeld_hinzu() {
        let ergebnis = berechnen(FamilienleistungsausgleichEingabe {
            est_ohne_freibetraege: Cent::new(1_000_000),
            est_mit_freibetraegen: Cent::new(900_000),
            kindergeld: Cent::new(30_000),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(930_000));
    }
}
