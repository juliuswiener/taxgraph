//! § 33 Abs. 3 `EStG` zumutbare Belastung.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `ZumutbareBelastung__ZumutbareBelastung_in`.
#[derive(Debug, Clone, Copy)]
pub struct ZumutbareBelastungEingabe {
    pub gesamtbetrag_der_einkuenfte: Cent,
    pub anzahl_kinder: i64,
    pub splitting: bool,
}

/// § 33 Abs. 3 `EStG`: Gesamtbetrag der Einkuenfte, Kinderzahl und Splitting-Flag auf die
/// zumutbare Belastung.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn berechnen(eingabe: ZumutbareBelastungEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::zumutbare_belastung(
        eingabe.gesamtbetrag_der_einkuenfte.get(),
        eingabe.anzahl_kinder,
        eingabe.splitting,
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, ZumutbareBelastungEingabe};
    use domain::Cent;

    #[test]
    fn hoeheres_einkommen_erhoeht_die_zumutbare_belastung() {
        let niedrig = berechnen(ZumutbareBelastungEingabe {
            gesamtbetrag_der_einkuenfte: Cent::new(2_000_000),
            anzahl_kinder: 0,
            splitting: false,
        })
        .unwrap();
        let hoch = berechnen(ZumutbareBelastungEingabe {
            gesamtbetrag_der_einkuenfte: Cent::new(6_000_000),
            anzahl_kinder: 0,
            splitting: false,
        })
        .unwrap();
        assert!(hoch > niedrig);
    }
}
