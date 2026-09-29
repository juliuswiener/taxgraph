//! § 32a `EStG` Einkommensteuertarif (Grundtarif/Splittingtarif) und die End-to-end-Scopes
//! `FestzusetzendeEst{Einzel,Zusammen,Gesamt,GesamtZusammen}`, plus die § 34 Abs. 1
//! Fuenftelregelung (orchestriert den Tarif zweimal, KEIN eigener Catala-Scope).
use catala_sys::{CatalaFehler, FestzusetzendeEstErgebnis};
pub use catala_sys::Vz;
use domain::Cent;

/// Entscheidet, welcher der beiden Tarif-Scopes (Grundtarif/Splittingtarif) bzw. welcher der
/// beiden End-to-end-Arbeitnehmerfaelle (Einzel/Zusammen) laeuft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Veranlagung {
    Einzel,
    Zusammen,
}

/// § 32a Abs. 1 `EStG` Grundtarif: zu versteuerndes Einkommen auf die tarifliche `ESt`.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn grundtarif(zve: Cent, vz: Vz) -> Result<Cent, CatalaFehler> {
    catala_sys::grundtarif(zve.get(), vz).map(Cent::new)
}

/// § 32a Abs. 5 `EStG` Splittingtarif: gemeinsames zvE auf die tarifliche `ESt`.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn splittingtarif(zve_gemeinsam: Cent, vz: Vz) -> Result<Cent, CatalaFehler> {
    catala_sys::splittingtarif(zve_gemeinsam.get(), vz).map(Cent::new)
}

/// Eingabe fuer [`festzusetzende_est_einzel`], 1:1 `FestzusetzendeEstEinzel__FestzusetzendeEstEinzel_in`.
#[derive(Debug, Clone, Copy)]
pub struct FestzusetzendeEstEinzelEingabe {
    pub bruttoarbeitslohn: Cent,
    pub werbungskosten: Cent,
    pub sonderausgaben: Cent,
}

/// End-to-end Arbeitnehmerfall: Bruttoarbeitslohn, Werbungskosten und Sonderausgaben auf die
/// festzusetzende Einkommensteuer.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn festzusetzende_est_einzel(
    eingabe: FestzusetzendeEstEinzelEingabe,
    vz: Vz,
) -> Result<Cent, CatalaFehler> {
    catala_sys::festzusetzende_est_einzel(
        eingabe.bruttoarbeitslohn.get(),
        eingabe.werbungskosten.get(),
        eingabe.sonderausgaben.get(),
        vz,
    )
    .map(Cent::new)
}

/// Wie [`festzusetzende_est_einzel`], gibt aber alle sechs Zwischenergebnisse zurueck (u. a.
/// `zu_versteuerndes_einkommen`, Bemessungsgroesse der § 101-Grundfreibetrags-Unterschreitung).
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn festzusetzende_est_einzel_voll(
    eingabe: FestzusetzendeEstEinzelEingabe,
    vz: Vz,
) -> Result<FestzusetzendeEstErgebnis, CatalaFehler> {
    catala_sys::festzusetzende_est_einzel_voll(
        eingabe.bruttoarbeitslohn.get(),
        eingabe.werbungskosten.get(),
        eingabe.sonderausgaben.get(),
        vz,
    )
}

/// Eingabe fuer [`festzusetzende_est_zusammen`], 1:1
/// `FestzusetzendeEstZusammen__FestzusetzendeEstZusammen_in`.
#[derive(Debug, Clone, Copy)]
pub struct FestzusetzendeEstZusammenEingabe {
    pub bruttoarbeitslohn_a: Cent,
    pub werbungskosten_a: Cent,
    pub bruttoarbeitslohn_b: Cent,
    pub werbungskosten_b: Cent,
    pub sonderausgaben_gemeinsam: Cent,
}

/// § 26b `EStG` Zusammenveranlagung (Splitting): Bruttoarbeitslohn und Werbungskosten je Partner
/// sowie gemeinsame Sonderausgaben auf die festzusetzende Einkommensteuer.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn festzusetzende_est_zusammen(
    eingabe: FestzusetzendeEstZusammenEingabe,
    vz: Vz,
) -> Result<Cent, CatalaFehler> {
    catala_sys::festzusetzende_est_zusammen(
        eingabe.bruttoarbeitslohn_a.get(),
        eingabe.werbungskosten_a.get(),
        eingabe.bruttoarbeitslohn_b.get(),
        eingabe.werbungskosten_b.get(),
        eingabe.sonderausgaben_gemeinsam.get(),
        vz,
    )
    .map(Cent::new)
}

/// Eingabe fuer [`festzusetzende_est_gesamt`]/[`festzusetzende_est_gesamt_zusammen`]: die
/// Money-/Bool-Felder, die `FestzusetzendeEstGesamt_in` und `FestzusetzendeEstGesamtZusammen_in`
/// TEILEN -- welcher der beiden Catala-Scopes laeuft, entscheidet einzig, welche der beiden
/// Funktionen aufgerufen wird.
#[derive(Debug, Clone, Copy)]
pub struct GesamtEingabe {
    pub einkuenfte_nichtselbststaendig: Cent,
    pub einkuenfte_kapitalvermoegen: Cent,
    pub einkuenfte_vermietung: Cent,
    pub einkuenfte_sonstige: Cent,
    pub einkuenfte_gewinn: Cent,
    pub altersentlastungsbetrag: Cent,
    pub entlastungsbetrag_alleinerziehende: Cent,
    pub sonderausgaben: Cent,
    pub aussergewoehnliche_belastungen: Cent,
    pub freibetraege_kinder: Cent,
    pub sonstige_abzuege_vom_einkommen: Cent,
    pub anzurechnende_auslaendische_steuern: Cent,
    pub steuerermaessigungen: Cent,
    pub steuer_kapital_gesondert: Cent,
    pub hinzurechnung_kindergeld: Cent,
    pub hinzurechnung_zulage: Cent,
    pub tarif_modifiziert: bool,
    pub tarifliche_est_modifiziert: Cent,
}

impl GesamtEingabe {
    fn zu_ffi(self) -> catala_sys::GesamtEingabe {
        catala_sys::GesamtEingabe {
            einkuenfte_nichtselbststaendig_cent: self.einkuenfte_nichtselbststaendig.get(),
            einkuenfte_kapitalvermoegen_cent: self.einkuenfte_kapitalvermoegen.get(),
            einkuenfte_vermietung_cent: self.einkuenfte_vermietung.get(),
            einkuenfte_sonstige_cent: self.einkuenfte_sonstige.get(),
            einkuenfte_gewinn_cent: self.einkuenfte_gewinn.get(),
            altersentlastungsbetrag_cent: self.altersentlastungsbetrag.get(),
            entlastungsbetrag_alleinerziehende_cent: self.entlastungsbetrag_alleinerziehende.get(),
            sonderausgaben_cent: self.sonderausgaben.get(),
            aussergewoehnliche_belastungen_cent: self.aussergewoehnliche_belastungen.get(),
            freibetraege_kinder_cent: self.freibetraege_kinder.get(),
            sonstige_abzuege_vom_einkommen_cent: self.sonstige_abzuege_vom_einkommen.get(),
            anzurechnende_auslaendische_steuern_cent: self.anzurechnende_auslaendische_steuern.get(),
            steuerermaessigungen_cent: self.steuerermaessigungen.get(),
            steuer_kapital_gesondert_cent: self.steuer_kapital_gesondert.get(),
            hinzurechnung_kindergeld_cent: self.hinzurechnung_kindergeld.get(),
            hinzurechnung_zulage_cent: self.hinzurechnung_zulage.get(),
            tarif_modifiziert: self.tarif_modifiziert,
            tarifliche_est_modifiziert_cent: self.tarifliche_est_modifiziert.get(),
        }
    }
}

/// Veranlagter Gesamtfall, Einzelveranlagung: nimmt bereits final berechnete
/// Einkuenfte/Abzuege entgegen -- die Vorstufen (Sonderausgaben-Endbetrag, Vorsorgeabzug,
/// Kindergeld-Vergleich) sind Teil-B-Python-Glue und nicht Teil dieser Funktion.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn festzusetzende_est_gesamt(
    eingabe: GesamtEingabe,
    vz: Vz,
) -> Result<FestzusetzendeEstErgebnis, CatalaFehler> {
    catala_sys::festzusetzende_est_gesamt(eingabe.zu_ffi(), vz)
}

/// Wie [`festzusetzende_est_gesamt`], fuer die Zusammenveranlagung -- identische Eingabefelder.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn festzusetzende_est_gesamt_zusammen(
    eingabe: GesamtEingabe,
    vz: Vz,
) -> Result<FestzusetzendeEstErgebnis, CatalaFehler> {
    catala_sys::festzusetzende_est_gesamt_zusammen(eingabe.zu_ffi(), vz)
}

/// Eingabe fuer [`fuenftel`] (§ 34 Abs. 1 `EStG`).
#[derive(Debug, Clone, Copy)]
pub struct FuenftelEingabe {
    pub zu_versteuerndes_einkommen: Cent,
    pub ausserordentliche_einkuenfte: Cent,
    pub veranlagung: Veranlagung,
    pub vz: Vz,
}

/// [`fuenftel`] kann an der § 34 Abs. 1 S. 3-Vorbedingung ODER an einem Tarif-Aufruf scheitern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FuenftelFehler {
    #[error("§ 34 Abs. 1 S. 3 `EStG` setzt ein positives zu versteuerndes Einkommen voraus")]
    ZveNichtPositiv,
    #[error(transparent)]
    Catala(#[from] CatalaFehler),
}

/// § 34 Abs. 1 `EStG` Fuenftelregelung fuer aussererordentliche Einkuenfte (Kernfall
/// verbleibendes zvE >= 0 UND Negativfall S. 3). Orchestriert den bestehenden Tarif-Scope
/// zweimal -- KEIN eigener Catala-Scope, der Faktor 5 ist die Struktur-Konstante von
/// `p34_fuenftel_ao_est`:
///
/// ```text
/// verbleibendes_zve = zvE - ao
/// est1 = Tarif(verbleibendes_zve)
/// est2 = Tarif(verbleibendes_zve + ao/5)
/// est_ao = 5 * (est2 - est1)
/// tarifliche_est = est1 + est_ao
/// ```
///
/// # Errors
/// Siehe [`FuenftelFehler`].
pub fn fuenftel(eingabe: FuenftelEingabe) -> Result<Cent, FuenftelFehler> {
    let zve = eingabe.zu_versteuerndes_einkommen.get();
    let ao = eingabe.ausserordentliche_einkuenfte.get();
    let verbleibendes_zve = zve - ao;

    let tarif = |x: i64| -> Result<i64, CatalaFehler> {
        match eingabe.veranlagung {
            Veranlagung::Einzel => catala_sys::grundtarif(x, eingabe.vz),
            Veranlagung::Zusammen => catala_sys::splittingtarif(x, eingabe.vz),
        }
    };

    if verbleibendes_zve < 0 {
        // § 34 Abs. 1 S. 3: Grundbetrag 0, tarifliche `ESt` = 5 * Tarif(zvE/5).
        if zve <= 0 {
            return Err(FuenftelFehler::ZveNichtPositiv);
        }
        let est = tarif(zve.div_euclid(5))?;
        return Ok(Cent::new(5 * est));
    }

    let est1 = tarif(verbleibendes_zve)?;
    let est2 = tarif(verbleibendes_zve + ao.div_euclid(5))?;
    Ok(Cent::new(est1 + 5 * (est2 - est1)))
}

#[cfg(test)]
mod tests {
    use super::{
        festzusetzende_est_einzel, fuenftel, grundtarif, FestzusetzendeEstEinzelEingabe,
        FuenftelEingabe, Veranlagung, Vz,
    };
    use domain::Cent;

    #[test]
    fn grundfreibetrag_ist_steuerfrei() {
        let ergebnis = grundtarif(Cent::new(0), Vz::Vz2024).unwrap();
        assert_eq!(ergebnis, Cent::new(0));
    }

    #[test]
    fn bruttolohn_ohne_abzuege_laeuft_durch() {
        let ergebnis = festzusetzende_est_einzel(
            FestzusetzendeEstEinzelEingabe {
                bruttoarbeitslohn: Cent::new(0),
                werbungskosten: Cent::new(0),
                sonderausgaben: Cent::new(0),
            },
            Vz::Vz2024,
        )
        .unwrap();
        assert_eq!(ergebnis, Cent::new(0));
    }

    #[test]
    fn fuenftelregelung_ohne_ausserordentliche_einkuenfte_ist_neutral() {
        // ao = 0 -> verbleibendes_zve = zve, est1 = est2 -> tarifliche_est = est1 = Tarif(zve).
        let zve = Cent::new(5_000_000);
        let referenz = grundtarif(zve, Vz::Vz2024).unwrap();
        let ergebnis = fuenftel(FuenftelEingabe {
            zu_versteuerndes_einkommen: zve,
            ausserordentliche_einkuenfte: Cent::new(0),
            veranlagung: Veranlagung::Einzel,
            vz: Vz::Vz2024,
        })
        .unwrap();
        assert_eq!(ergebnis, referenz);
    }
}
