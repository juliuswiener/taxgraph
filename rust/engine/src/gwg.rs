//! § 6 Abs. 2 `EStG` geringwertige Wirtschaftsgueter (GWG-Sofortabzug).
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `GwgSofortabzug__GwgSofortabzug_in`.
#[derive(Debug, Clone, Copy)]
pub struct GwgEingabe {
    pub anschaffungskosten_netto: Cent,
}

/// § 6 Abs. 2 `EStG`: Anschaffungskosten netto (je Wirtschaftsgut) auf den Sofortabzug.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn berechnen(eingabe: GwgEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::gwg_sofortabzug(eingabe.anschaffungskosten_netto.get()).map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, GwgEingabe};
    use domain::Cent;

    #[test]
    fn ueber_800_kein_sofortabzug() {
        let ergebnis = berechnen(GwgEingabe {
            anschaffungskosten_netto: Cent::new(80_100),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(0));
    }
}
