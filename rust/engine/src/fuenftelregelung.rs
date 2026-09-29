//! § 34 Abs. 1/3 `EStG` ausserordentliche Einkuenfte: ermaessigter Durchschnittssteuersatz.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1
/// `ErmaessigterDurchschnittssatz__ErmaessigterDurchschnittssatz_in`.
#[derive(Debug, Clone, Copy)]
pub struct FuenftelregelungEingabe {
    pub ao_einkuenfte: Cent,
    pub est_gesamt_zzgl_progression: Cent,
    pub bemessungsgrundlage_durchschnitt: Cent,
}

/// § 34 Abs. 3 `EStG`: ausserordentliche Einkuenfte (gedeckelt auf 5 Mio.), `ESt` gesamt zzgl.
/// Progression und Bemessungsgrundlage Durchschnitt auf die `ESt` auf die ausserordentlichen
/// Einkuenfte.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn berechnen(eingabe: FuenftelregelungEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::ermaessigter_durchschnittssatz(
        eingabe.ao_einkuenfte.get(),
        eingabe.est_gesamt_zzgl_progression.get(),
        eingabe.bemessungsgrundlage_durchschnitt.get(),
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, FuenftelregelungEingabe};
    use domain::Cent;

    #[test]
    fn unter_14_prozent_greift_der_floor() {
        let ergebnis = berechnen(FuenftelregelungEingabe {
            ao_einkuenfte: Cent::new(10_000_000),
            est_gesamt_zzgl_progression: Cent::new(20_000_000),
            bemessungsgrundlage_durchschnitt: Cent::new(100_000_000),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(1_400_000));
    }
}
