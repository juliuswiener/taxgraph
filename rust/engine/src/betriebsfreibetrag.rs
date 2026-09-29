//! § 16 Abs. 4 `EStG` Freibetrag fuer Betriebsveraeusserung/-aufgabe.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `BetriebsFreibetrag__BetriebsFreibetrag_in`.
#[derive(Debug, Clone, Copy)]
pub struct BetriebsFreibetragEingabe {
    pub veraeusserungsgewinn: Cent,
}

/// § 16 Abs. 4 `EStG`: Veraeusserungsgewinn auf den (abgeschmolzenen) Freibetrag.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn berechnen(eingabe: BetriebsFreibetragEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::betriebs_freibetrag(eingabe.veraeusserungsgewinn.get()).map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, BetriebsFreibetragEingabe};
    use domain::Cent;

    #[test]
    fn teilabschmelzung_ueber_136000() {
        let ergebnis = berechnen(BetriebsFreibetragEingabe {
            veraeusserungsgewinn: Cent::new(16_000_000),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(2_100_000));
    }
}
