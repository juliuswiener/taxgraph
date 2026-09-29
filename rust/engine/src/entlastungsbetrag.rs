//! § 24b `EStG` Entlastungsbetrag fuer Alleinerziehende.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `Entlastungsbetrag__Entlastungsbetrag_in`.
#[derive(Debug, Clone, Copy)]
pub struct EntlastungsbetragEingabe {
    pub alleinstehend: bool,
    pub anzahl_kinder: i64,
    pub monate_ohne_voraussetzung: i64,
}

/// § 24b `EStG`: Alleinstehend-Flag, Kinderzahl und Monate ohne Voraussetzung auf den
/// Entlastungsbetrag.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn berechnen(eingabe: EntlastungsbetragEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::entlastungsbetrag(
        eingabe.alleinstehend,
        eingabe.anzahl_kinder,
        eingabe.monate_ohne_voraussetzung,
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, EntlastungsbetragEingabe};

    #[test]
    fn nicht_alleinstehend_hat_keinen_anspruch() {
        let ergebnis = berechnen(EntlastungsbetragEingabe {
            alleinstehend: false,
            anzahl_kinder: 2,
            monate_ohne_voraussetzung: 0,
        })
        .unwrap();
        assert_eq!(ergebnis.get(), 0);
    }
}
