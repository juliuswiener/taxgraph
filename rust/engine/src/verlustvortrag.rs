//! § 10d Abs. 2 `EStG` Verlustvortrag-Hoechstbetrag (Mindestbesteuerung).
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `VerlustvortragAbzug__VerlustvortragAbzug_in`.
#[derive(Debug, Clone, Copy)]
pub struct VerlustvortragEingabe {
    pub gesamtbetrag_einkuenfte: Cent,
    pub verlustvortrag_bestand: Cent,
    pub zusammenveranlagung: bool,
}

/// § 10d Abs. 2 `EStG`: Gesamtbetrag der Einkuenfte, Verlustvortragsbestand und
/// Zusammenveranlagungs-Flag auf den Verlustabzug.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use engine::verlustvortrag::{berechnen, VerlustvortragEingabe};
/// use domain::Cent;
/// let ergebnis = berechnen(VerlustvortragEingabe {
///     gesamtbetrag_einkuenfte: Cent::new(5_000_000),
///     verlustvortrag_bestand: Cent::new(6_000_000),
///     zusammenveranlagung: false,
/// })
/// .unwrap();
/// assert_eq!(ergebnis, Cent::new(5_000_000));
/// ```
pub fn berechnen(eingabe: VerlustvortragEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::verlustvortrag_abzug(
        eingabe.gesamtbetrag_einkuenfte.get(),
        eingabe.verlustvortrag_bestand.get(),
        eingabe.zusammenveranlagung,
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, VerlustvortragEingabe};
    use domain::Cent;

    #[test]
    fn bestand_ueber_gde_wird_auf_gde_gekappt() {
        let ergebnis = berechnen(VerlustvortragEingabe {
            gesamtbetrag_einkuenfte: Cent::new(5_000_000),
            verlustvortrag_bestand: Cent::new(6_000_000),
            zusammenveranlagung: false,
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(5_000_000));
    }
}
