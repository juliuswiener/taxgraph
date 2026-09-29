//! § 15 Abs. 1 S. 1 Nr. 2 `EStG` Einkuenfte aus Gewerbebetrieb als Mitunternehmer.
use catala_sys::CatalaFehler;
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `MitunternehmerEinkuenfte__MitunternehmerEinkuenfte_in`.
#[derive(Debug, Clone, Copy)]
pub struct MitunternehmerEingabe {
    pub gewinnanteil: Cent,
    pub verguetung_taetigkeit: Cent,
    pub verguetung_darlehen: Cent,
    pub verguetung_ueberlassung: Cent,
}

/// § 15 Abs. 1 S. 1 Nr. 2 `EStG`: Gewinnanteil und die drei Sondervergueltungen auf die
/// Mitunternehmereinkuenfte.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn berechnen(eingabe: MitunternehmerEingabe) -> Result<Cent, CatalaFehler> {
    catala_sys::mitunternehmer_einkuenfte(
        eingabe.gewinnanteil.get(),
        eingabe.verguetung_taetigkeit.get(),
        eingabe.verguetung_darlehen.get(),
        eingabe.verguetung_ueberlassung.get(),
    )
    .map(Cent::new)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, MitunternehmerEingabe};
    use domain::Cent;

    #[test]
    fn gewinnanteil_und_sondervergueltungen_addieren_sich() {
        let ergebnis = berechnen(MitunternehmerEingabe {
            gewinnanteil: Cent::new(1_000_000),
            verguetung_taetigkeit: Cent::new(1_200_000),
            verguetung_darlehen: Cent::new(300_000),
            verguetung_ueberlassung: Cent::new(500_000),
        })
        .unwrap();
        assert_eq!(ergebnis, Cent::new(3_000_000));
    }
}
