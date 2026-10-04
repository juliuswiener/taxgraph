//! § 2 `EStG` verallgemeinerte Veranlagung (`runner.py` `_gesamt_out` und seine Leser), dazu
//! die zwei direkten Catala-Accessoren § 10d Abs. 2 und § 34 Abs. 3.
use catala_sys::FestzusetzendeEstErgebnis;
use domain::{Cent, Euro, Vz};

use super::rente::{einkuenfte_versorgung, EinkuenfteVersorgungEingabe};
use bindung::Params;

use super::{euro, z, EngineFehler};
use crate::fuenftelregelung::{self, FuenftelregelungEingabe};
use crate::tarif::{festzusetzende_est_gesamt, festzusetzende_est_gesamt_zusammen, GesamtEingabe};
use crate::verlustvortrag::{self, VerlustvortragEingabe};

/// Eingabe fuer [`gesamt`] und die anderen Leser desselben Scope-Laufs.
///
/// PARITÄT: Python setzt jedes fehlende Betragsfeld = 0 (`m(k)` in `_gesamt_out`), fehlendes
/// `tarif_modifiziert` = False, fehlendes `kinder_ganzjaehrig` = 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GesamtfallEingabe {
    pub vz: Vz,
    /// PARITÄT: Python waehlt den Zusammen-Scope nur bei `veranlagung == "zusammen"`; jeder
    /// andere Wert, auch ein fehlender oder unbekannter, laeuft als Einzelveranlagung.
    pub zusammenveranlagung: bool,
    pub einkuenfte_nichtselbststaendig: Euro,
    pub einkuenfte_kapitalvermoegen: Euro,
    pub einkuenfte_vermietung: Euro,
    pub einkuenfte_sonstige: Euro,
    pub einkuenfte_gewinn: Euro,
    pub altersentlastungsbetrag: Euro,
    pub entlastungsbetrag_alleinerziehende: Euro,
    /// Tatsaechliche Sonderausgaben vor Vorsorge-Zuschlag und § 10c-Floor.
    pub sonderausgaben: Euro,
    /// § 10 Abs. 1 Nr. 2 Gesamtbeitraege inkl. AG-Anteil; 0 = kein Vorsorgeabzug.
    pub vorsorge_gesamtbeitraege_inkl_ag: Euro,
    pub vorsorge_ag_anteil_steuerfrei: Euro,
    pub aussergewoehnliche_belastungen: Euro,
    pub freibetraege_kinder: Euro,
    pub sonstige_abzuege_vom_einkommen: Euro,
    pub anzurechnende_auslaendische_steuern: Euro,
    pub steuerermaessigungen: Euro,
    pub steuer_kapital_gesondert: Euro,
    /// Direkter Wert; wird ersetzt, wenn `kinder_ganzjaehrig != 0`.
    pub hinzurechnung_kindergeld: Euro,
    /// § 31 S. 4: Kinder mit ganzjaehrigem Kindergeld; != 0 -> `Kindergeld x 12 x Kinder`.
    pub kinder_ganzjaehrig: i64,
    pub hinzurechnung_zulage: Euro,
    pub tarif_modifiziert: bool,
    pub tarifliche_est_modifiziert: Euro,
    /// § 19 Abs. 2 Versorgungsbezuege, addiert auf die nichtselbststaendigen Einkuenfte.
    pub versorgung: EinkuenfteVersorgungEingabe,
}

/// § 10 Abs. 1 Nr. 2, Abs. 3: abziehbare Altersvorsorge (`runner.py` `_vorsorge_abzug`).
fn vorsorge_abzug(e: &GesamtfallEingabe, p: &Params) -> Result<i128, EngineFehler> {
    let beitraege = z(e.vorsorge_gesamtbeitraege_inkl_ag);
    if beitraege == 0 {
        return Ok(0);
    }
    Ok(
        (beitraege.min(z(p.vorsorge_hoechstbeitrag(e.vz)?)) - z(e.vorsorge_ag_anteil_steuerfrei))
            .max(0),
    )
}

/// § 10c Guenstigervergleich (`runner.py` `_sonderausgaben_final`).
fn sonderausgaben_final(e: &GesamtfallEingabe, p: &Params) -> Result<i128, EngineFehler> {
    let pausch =
        z(p.sonderausgaben_pauschbetrag(e.vz)?) * if e.zusammenveranlagung { 2 } else { 1 };
    Ok((z(e.sonderausgaben) + vorsorge_abzug(e, p)?).max(pausch))
}

fn in_cent(v: i128) -> Result<Cent, EngineFehler> {
    Ok(euro(v)?.to_cent()?)
}

fn c(e: Euro) -> Result<Cent, EngineFehler> {
    Ok(e.to_cent()?)
}

/// Der eine Catala-Lauf (`runner.py` `_gesamt_out`): Einzel- oder Zusammen-Scope.
///
/// # Errors
/// [`EngineFehler::Catala`] aus dem Scope; [`EngineFehler::Ueberlauf`].
fn gesamt_out(
    e: &GesamtfallEingabe,
    p: &Params,
) -> Result<FestzusetzendeEstErgebnis, EngineFehler> {
    let hinzu_kg = if e.kinder_ganzjaehrig == 0 {
        z(e.hinzurechnung_kindergeld)
    } else {
        z(p.kindergeld_monatlich_je_kind(e.vz)?) * 12 * i128::from(e.kinder_ganzjaehrig)
    };
    let an = z(e.einkuenfte_nichtselbststaendig) + z(einkuenfte_versorgung(&e.versorgung, p)?);
    let eingabe = GesamtEingabe {
        einkuenfte_nichtselbststaendig: in_cent(an)?,
        einkuenfte_kapitalvermoegen: c(e.einkuenfte_kapitalvermoegen)?,
        einkuenfte_vermietung: c(e.einkuenfte_vermietung)?,
        einkuenfte_sonstige: c(e.einkuenfte_sonstige)?,
        einkuenfte_gewinn: c(e.einkuenfte_gewinn)?,
        altersentlastungsbetrag: c(e.altersentlastungsbetrag)?,
        entlastungsbetrag_alleinerziehende: c(e.entlastungsbetrag_alleinerziehende)?,
        sonderausgaben: in_cent(sonderausgaben_final(e, p)?)?,
        aussergewoehnliche_belastungen: c(e.aussergewoehnliche_belastungen)?,
        freibetraege_kinder: c(e.freibetraege_kinder)?,
        sonstige_abzuege_vom_einkommen: c(e.sonstige_abzuege_vom_einkommen)?,
        anzurechnende_auslaendische_steuern: c(e.anzurechnende_auslaendische_steuern)?,
        steuerermaessigungen: c(e.steuerermaessigungen)?,
        steuer_kapital_gesondert: c(e.steuer_kapital_gesondert)?,
        hinzurechnung_kindergeld: in_cent(hinzu_kg)?,
        hinzurechnung_zulage: c(e.hinzurechnung_zulage)?,
        tarif_modifiziert: e.tarif_modifiziert,
        tarifliche_est_modifiziert: c(e.tarifliche_est_modifiziert)?,
    };
    Ok(if e.zusammenveranlagung {
        festzusetzende_est_gesamt_zusammen(eingabe, e.vz)
    } else {
        festzusetzende_est_gesamt(eingabe, e.vz)
    }?)
}

fn floor(cent: i64) -> Euro {
    let euro = Cent::new(cent).floor_euro();
    // Abrunden gegen −∞: der Euro-Wert liegt hoechstens 99 Cent unter dem Cent-Wert.
    debug_assert!({
        let rest = i128::from(cent) - i128::from(euro.get()) * 100;
        (0..100).contains(&rest)
    });
    euro
}

/// § 2 `EStG` festzusetzende Einkommensteuer, EURO (Cent abgerundet).
///
/// # Errors
/// [`EngineFehler::Catala`]; [`EngineFehler::Ueberlauf`].
///
/// ```
/// # use engine::zugriff::teil2::gesamt::*;
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = beispiel_gesamtfall();
/// assert!(gesamt(&e, &p).unwrap().get() > 0);
/// ```
pub fn gesamt(e: &GesamtfallEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    Ok(floor(gesamt_out(e, p)?.festzusetzende_est_cent()?))
}

/// § 2 Abs. 3 `EStG` Gesamtbetrag der Einkuenfte desselben Laufs, EURO.
///
/// # Errors
/// Wie [`gesamt`].
///
/// ```
/// # use engine::zugriff::teil2::gesamt::*;
/// # use domain::Euro;
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// assert_eq!(gesamt_gde(&beispiel_gesamtfall(), &p).unwrap(), Euro::new(50_000));
/// ```
pub fn gesamt_gde(e: &GesamtfallEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    Ok(floor(gesamt_out(e, p)?.gesamtbetrag_der_einkuenfte_cent()?))
}

/// Tarifliche `ESt` (§ 32a auf das zvE) desselben Laufs, EURO.
///
/// # Errors
/// Wie [`gesamt`].
///
/// ```
/// # use engine::zugriff::teil2::gesamt::*;
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = beispiel_gesamtfall();
/// assert_eq!(gesamt_tarifliche(&e, &p).unwrap(), gesamt(&e, &p).unwrap());
/// ```
pub fn gesamt_tarifliche(e: &GesamtfallEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    Ok(floor(gesamt_out(e, p)?.tarifliche_est_cent()?))
}

/// § 2 Abs. 5 zu versteuerndes Einkommen desselben Laufs, EURO.
///
/// # Errors
/// Wie [`gesamt`].
///
/// ```
/// # use engine::zugriff::teil2::gesamt::*;
/// # use domain::Euro;
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// // 50.000 - § 10c-Pauschbetrag 36
/// assert_eq!(gesamt_zve(&beispiel_gesamtfall(), &p).unwrap(), Euro::new(49_964));
/// ```
pub fn gesamt_zve(e: &GesamtfallEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    Ok(floor(gesamt_out(e, p)?.zu_versteuerndes_einkommen_cent()?))
}

/// Rechenweg-Kette fuer die Erklaer-UI, alle Werte EURO (`runner.py` `catala_gesamt_kette`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GesamtKette {
    pub gesamtbetrag_der_einkuenfte: Euro,
    pub zu_versteuerndes_einkommen: Euro,
    pub tarifliche_est: Euro,
    pub festzusetzende_est: Euro,
}

/// `GdE` -> zvE -> tarifliche `ESt` -> festzusetzende `ESt` aus EINEM Scope-Lauf, EURO.
///
/// # Errors
/// Wie [`gesamt`].
///
/// ```
/// # use engine::zugriff::teil2::gesamt::*;
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = beispiel_gesamtfall();
/// let k = gesamt_kette(&e, &p).unwrap();
/// assert_eq!(k.festzusetzende_est, gesamt(&e, &p).unwrap());
/// ```
pub fn gesamt_kette(e: &GesamtfallEingabe, p: &Params) -> Result<GesamtKette, EngineFehler> {
    let o = gesamt_out(e, p)?;
    Ok(GesamtKette {
        gesamtbetrag_der_einkuenfte: floor(o.gesamtbetrag_der_einkuenfte_cent()?),
        zu_versteuerndes_einkommen: floor(o.zu_versteuerndes_einkommen_cent()?),
        tarifliche_est: floor(o.tarifliche_est_cent()?),
        festzusetzende_est: floor(o.festzusetzende_est_cent()?),
    })
}

/// Einzelveranlagung VZ 2025 mit 50.000 EUR nichtselbststaendigen Einkuenften, sonst alles 0
/// -- Beispielwert fuer die Doku.
///
/// ```
/// use engine::zugriff::teil2::gesamt::beispiel_gesamtfall;
/// let e = beispiel_gesamtfall();
/// assert_eq!(e.einkuenfte_nichtselbststaendig.get(), 50_000);
/// assert!(!e.zusammenveranlagung);
/// ```
#[doc(hidden)]
#[must_use]
pub const fn beispiel_gesamtfall() -> GesamtfallEingabe {
    let n = Euro::new(0);
    GesamtfallEingabe {
        vz: Vz::Vz2025,
        zusammenveranlagung: false,
        einkuenfte_nichtselbststaendig: Euro::new(50_000),
        einkuenfte_kapitalvermoegen: n,
        einkuenfte_vermietung: n,
        einkuenfte_sonstige: n,
        einkuenfte_gewinn: n,
        altersentlastungsbetrag: n,
        entlastungsbetrag_alleinerziehende: n,
        sonderausgaben: n,
        vorsorge_gesamtbeitraege_inkl_ag: n,
        vorsorge_ag_anteil_steuerfrei: n,
        aussergewoehnliche_belastungen: n,
        freibetraege_kinder: n,
        sonstige_abzuege_vom_einkommen: n,
        anzurechnende_auslaendische_steuern: n,
        steuerermaessigungen: n,
        steuer_kapital_gesondert: n,
        hinzurechnung_kindergeld: n,
        kinder_ganzjaehrig: 0,
        hinzurechnung_zulage: n,
        tarif_modifiziert: false,
        tarifliche_est_modifiziert: n,
        versorgung: EinkuenfteVersorgungEingabe {
            versorgung_jahresrente: n,
            freibetrag: super::rente::VersorgungsfreibetragEingabe {
                bemessungsgrundlage: n,
                beginn_jahr: 0,
            },
        },
    }
}

/// Eingabe fuer [`p10d_2`]. PARITÄT: Python setzt jedes fehlende Feld = 0 bzw. False.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerlustabzugEingabe {
    pub gesamtbetrag_einkuenfte: Euro,
    pub verlustvortrag_bestand: Euro,
    pub zusammenveranlagung: bool,
}

/// § 10d Abs. 2 `EStG`: Verlustvortrag-Abzug (Catala `Verlustvortrag`), EURO.
///
/// # Errors
/// [`EngineFehler::Catala`]; [`EngineFehler::Ueberlauf`].
///
/// ```
/// # use engine::zugriff::teil2::gesamt::*;
/// # use domain::Euro;
/// let e = VerlustabzugEingabe { gesamtbetrag_einkuenfte: Euro::new(50_000),
///     verlustvortrag_bestand: Euro::new(60_000), zusammenveranlagung: false };
/// assert_eq!(p10d_2(&e).unwrap(), Euro::new(50_000));
/// ```
pub fn p10d_2(e: &VerlustabzugEingabe) -> Result<Euro, EngineFehler> {
    let abzug = verlustvortrag::berechnen(VerlustvortragEingabe {
        gesamtbetrag_einkuenfte: c(e.gesamtbetrag_einkuenfte)?,
        verlustvortrag_bestand: c(e.verlustvortrag_bestand)?,
        zusammenveranlagung: e.zusammenveranlagung,
    })?;
    Ok(abzug.floor_euro())
}

/// Eingabe fuer [`ermaessigter_durchschnittssatz`]. PARITÄT: Python setzt jedes fehlende
/// Feld = 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurchschnittssatzEingabe {
    pub ao_einkuenfte: Euro,
    pub est_gesamt_zzgl_progression: Euro,
    pub bemessungsgrundlage_durchschnitt: Euro,
}

/// § 34 Abs. 3 `EStG`: `ESt` auf den Veraeusserungsgewinn bis 5 Mio. (Catala
/// `ErmaessigterDurchschnittssatz`), EURO.
///
/// # Errors
/// [`EngineFehler::Catala`] (z. B. Bemessungsgrundlage 0); [`EngineFehler::Ueberlauf`].
///
/// ```
/// # use engine::zugriff::teil2::gesamt::*;
/// # use domain::Euro;
/// let e = DurchschnittssatzEingabe { ao_einkuenfte: Euro::new(100_000),
///     est_gesamt_zzgl_progression: Euro::new(200_000), bemessungsgrundlage_durchschnitt: Euro::new(1_000_000) };
/// assert_eq!(ermaessigter_durchschnittssatz(&e).unwrap(), Euro::new(14_000));
/// ```
pub fn ermaessigter_durchschnittssatz(e: &DurchschnittssatzEingabe) -> Result<Euro, EngineFehler> {
    let est_ao = fuenftelregelung::berechnen(FuenftelregelungEingabe {
        ao_einkuenfte: c(e.ao_einkuenfte)?,
        est_gesamt_zzgl_progression: c(e.est_gesamt_zzgl_progression)?,
        bemessungsgrundlage_durchschnitt: c(e.bemessungsgrundlage_durchschnitt)?,
    })?;
    Ok(est_ao.floor_euro())
}
