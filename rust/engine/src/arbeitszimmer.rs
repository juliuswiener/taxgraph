//! § 4 Abs. 5 Nr. 6b/6c `EStG` haeusliches Arbeitszimmer und Homeoffice-Tagespauschale.
use catala_sys::{CatalaFehler, RaumkostenabzugErgebnis};
use domain::Cent;

/// Eingabe fuer [`berechnen`], 1:1 `Arbeitszimmer_homeoffice.Raumkostenabzug__Raumkostenabzug_in`.
#[derive(Debug, Clone, Copy)]
pub struct ArbeitszimmerEingabe {
    pub arbeitszimmer_vorhanden: bool,
    pub ist_mittelpunkt: bool,
    pub tatsaechliche_aufwendungen: Cent,
    pub jahrespauschale_gewaehlt: bool,
    pub monate_ohne_mittelpunkt: i64,
    pub homeoffice_tage: i64,
    pub jahrespauschale: Cent,
    pub tagespauschale_pro_tag: Cent,
    pub tagespauschale_hoechstbetrag: Cent,
}

/// § 4 Abs. 5 Nr. 6b/6c `EStG`: Arbeitszimmer- und Homeoffice-Eingaben auf die Raumkostenabzuege
/// (getrennt und als Summe).
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use engine::arbeitszimmer::{berechnen, ArbeitszimmerEingabe};
/// use domain::Cent;
/// let ergebnis = berechnen(ArbeitszimmerEingabe {
///     arbeitszimmer_vorhanden: false,
///     ist_mittelpunkt: false,
///     tatsaechliche_aufwendungen: Cent::new(0),
///     jahrespauschale_gewaehlt: false,
///     monate_ohne_mittelpunkt: 0,
///     homeoffice_tage: 120,
///     jahrespauschale: Cent::new(126_000),
///     tagespauschale_pro_tag: Cent::new(600),
///     tagespauschale_hoechstbetrag: Cent::new(126_000),
/// })
/// .unwrap();
/// assert_eq!(ergebnis.abzug_arbeitszimmer_cent, 0);
/// assert_eq!(ergebnis.abzug_homeoffice_cent, 72_000);
/// assert_eq!(ergebnis.abzug_gesamt_cent, 72_000);
/// ```
pub fn berechnen(eingabe: ArbeitszimmerEingabe) -> Result<RaumkostenabzugErgebnis, CatalaFehler> {
    catala_sys::raumkostenabzug(catala_sys::RaumkostenabzugEingabe {
        arbeitszimmer_vorhanden: eingabe.arbeitszimmer_vorhanden,
        ist_mittelpunkt: eingabe.ist_mittelpunkt,
        tatsaechliche_aufwendungen_cent: eingabe.tatsaechliche_aufwendungen.get(),
        jahrespauschale_gewaehlt: eingabe.jahrespauschale_gewaehlt,
        monate_ohne_mittelpunkt: eingabe.monate_ohne_mittelpunkt,
        homeoffice_tage: eingabe.homeoffice_tage,
        jahrespauschale_cent: eingabe.jahrespauschale.get(),
        tagespauschale_pro_tag_cent: eingabe.tagespauschale_pro_tag.get(),
        tagespauschale_hoechstbetrag_cent: eingabe.tagespauschale_hoechstbetrag.get(),
    })
}

#[cfg(test)]
mod tests {
    use super::{berechnen, ArbeitszimmerEingabe};
    use domain::Cent;

    #[test]
    fn nur_homeoffice_ohne_arbeitszimmer() {
        let ergebnis = berechnen(ArbeitszimmerEingabe {
            arbeitszimmer_vorhanden: false,
            ist_mittelpunkt: false,
            tatsaechliche_aufwendungen: Cent::new(0),
            jahrespauschale_gewaehlt: false,
            monate_ohne_mittelpunkt: 0,
            homeoffice_tage: 120,
            jahrespauschale: Cent::new(126_000),
            tagespauschale_pro_tag: Cent::new(600),
            tagespauschale_hoechstbetrag: Cent::new(126_000),
        })
        .unwrap();
        assert_eq!(ergebnis.abzug_arbeitszimmer_cent, 0);
        assert_eq!(ergebnis.abzug_homeoffice_cent, 72_000);
        assert_eq!(ergebnis.abzug_gesamt_cent, 72_000);
    }
}
