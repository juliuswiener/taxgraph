//! `runner.catala_est` und seine Einzelzweige. Python dispatcht ueber die Anwesenheit von
//! Schluesseln im Sachverhalt-dict und liefert je Zweig EURO oder CENT; hier ist jeder Zweig
//! eine Variante von [`Sachverhalt`], und die Einheit steht im Rueckgabetyp ([`EstBetrag`]).
use domain::{Cent, Euro, Vz};

use super::gesamt::{gesamt, GesamtfallEingabe};
use super::gewerbe::{gewst, kst_nenner_b, GewstEingabe, KstEingabe};
use super::p35c::{p35c_ermaessigung_cent, SanierungEingabe};
use super::sonstige::{kfz_nutzungswert_monat_cent, KfzNutzungswertEingabe};
use bindung::Params;

use super::EngineFehler;
use crate::zugriff::teil1::werbungskosten::{
    entfernungspauschale, raumkosten, EntfernungspauschaleEingabe, RaumkostenEingabe,
};
use crate::tarif::{
    self, FestzusetzendeEstEinzelEingabe, FestzusetzendeEstZusammenEingabe, Veranlagung,
};

fn c(e: Euro) -> Result<Cent, EngineFehler> {
    Ok(e.to_cent()?)
}

/// Eingabe fuer [`est_einzel`] und [`est_einzel_zve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EstEinzelEingabe {
    pub vz: Vz,
    pub bruttoarbeitslohn: Euro,
    /// PARITÄT: Python setzt fehlend = 0.
    pub werbungskosten: Euro,
    /// PARITÄT: Python setzt fehlend = 0.
    pub sonderausgaben: Euro,
}

impl EstEinzelEingabe {
    fn scope(&self) -> Result<FestzusetzendeEstEinzelEingabe, EngineFehler> {
        Ok(FestzusetzendeEstEinzelEingabe {
            bruttoarbeitslohn: c(self.bruttoarbeitslohn)?,
            werbungskosten: c(self.werbungskosten)?,
            sonderausgaben: c(self.sonderausgaben)?,
        })
    }
}

/// End-to-end Arbeitnehmerfall (Scope `festzusetzende_est_einzel`), EURO.
///
/// # Errors
/// [`EngineFehler::Catala`]; [`EngineFehler::Ueberlauf`].
///
/// ```
/// # use engine::zugriff::teil2::est::*;
/// # use domain::{Euro, Vz};
/// let e = EstEinzelEingabe { vz: Vz::Vz2025, bruttoarbeitslohn: Euro::new(0),
///     werbungskosten: Euro::new(0), sonderausgaben: Euro::new(0) };
/// assert_eq!(est_einzel(&e).unwrap(), Euro::new(0));
/// ```
pub fn est_einzel(e: &EstEinzelEingabe) -> Result<Euro, EngineFehler> {
    Ok(tarif::festzusetzende_est_einzel(e.scope()?, e.vz)?.floor_euro())
}

/// § 2 Abs. 5 zvE des Arbeitnehmerfalls (gleiche Eingabe wie [`est_einzel`]), EURO.
///
/// # Errors
/// [`EngineFehler::Catala`]; [`EngineFehler::Ueberlauf`].
///
/// ```
/// # use engine::zugriff::teil2::est::*;
/// # use domain::{Euro, Vz};
/// let e = EstEinzelEingabe { vz: Vz::Vz2025, bruttoarbeitslohn: Euro::new(50_000),
///     werbungskosten: Euro::new(0), sonderausgaben: Euro::new(0) };
/// // 50.000 - AN-Pauschbetrag 1.230 - SA-Pauschbetrag 36
/// assert_eq!(est_einzel_zve(&e).unwrap(), Euro::new(48_734));
/// ```
pub fn est_einzel_zve(e: &EstEinzelEingabe) -> Result<Euro, EngineFehler> {
    let o = tarif::festzusetzende_est_einzel_voll(e.scope()?, e.vz)?;
    Ok(Cent::new(o.zu_versteuerndes_einkommen_cent).floor_euro())
}

/// Eingabe fuer [`est_zusammen`]. PARITÄT: Python setzt jedes fehlende Betragsfeld = 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EstZusammenEingabe {
    pub vz: Vz,
    pub bruttoarbeitslohn_a: Euro,
    pub bruttoarbeitslohn_b: Euro,
    pub werbungskosten_a: Euro,
    pub werbungskosten_b: Euro,
    pub sonderausgaben_gemeinsam: Euro,
}

/// § 26b `EStG` Zusammenveranlagung (Scope `festzusetzende_est_zusammen`), EURO.
///
/// # Errors
/// [`EngineFehler::Catala`]; [`EngineFehler::Ueberlauf`].
///
/// ```
/// # use engine::zugriff::teil2::est::*;
/// # use domain::{Euro, Vz};
/// let n = Euro::new(0);
/// let e = EstZusammenEingabe { vz: Vz::Vz2025, bruttoarbeitslohn_a: n, bruttoarbeitslohn_b: n,
///     werbungskosten_a: n, werbungskosten_b: n, sonderausgaben_gemeinsam: n };
/// assert_eq!(est_zusammen(&e).unwrap(), Euro::new(0));
/// ```
pub fn est_zusammen(e: &EstZusammenEingabe) -> Result<Euro, EngineFehler> {
    let eingabe = FestzusetzendeEstZusammenEingabe {
        bruttoarbeitslohn_a: c(e.bruttoarbeitslohn_a)?,
        werbungskosten_a: c(e.werbungskosten_a)?,
        bruttoarbeitslohn_b: c(e.bruttoarbeitslohn_b)?,
        werbungskosten_b: c(e.werbungskosten_b)?,
        sonderausgaben_gemeinsam: c(e.sonderausgaben_gemeinsam)?,
    };
    Ok(tarif::festzusetzende_est_zusammen(eingabe, e.vz)?.floor_euro())
}

/// Eingabe fuer [`tarif_est`] (Tarif-Zweig von `catala_est`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TarifEingabe {
    pub vz: Vz,
    pub veranlagung: Veranlagung,
    pub zu_versteuerndes_einkommen: Euro,
}

/// Tarif in CENT fuer ein zvE in ganzen EURO (`Money(f"{x}.00")`).
fn tarif_cent(vz: Vz, v: Veranlagung, zve_euro: i64) -> Result<i64, EngineFehler> {
    let zve = c(Euro::new(zve_euro))?;
    Ok(match v {
        Veranlagung::Einzel => tarif::grundtarif(zve, vz),
        Veranlagung::Zusammen => tarif::splittingtarif(zve, vz),
    }?
    .get())
}

/// § 32a `EStG` tarifliche `ESt` auf ein zvE (Grund- oder Splittingtarif), EURO.
///
/// # Errors
/// [`EngineFehler::Catala`]; [`EngineFehler::Ueberlauf`].
///
/// ```
/// # use engine::zugriff::teil2::est::*;
/// # use engine::tarif::Veranlagung;
/// # use domain::{Euro, Vz};
/// let e = TarifEingabe { vz: Vz::Vz2025, veranlagung: Veranlagung::Einzel,
///     zu_versteuerndes_einkommen: Euro::new(12_096) };
/// assert_eq!(tarif_est(&e).unwrap(), Euro::new(0));
/// ```
pub fn tarif_est(e: &TarifEingabe) -> Result<Euro, EngineFehler> {
    Ok(Cent::new(tarif_cent(e.vz, e.veranlagung, e.zu_versteuerndes_einkommen.get())?).floor_euro())
}

/// Eingabe fuer [`fuenftel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuenftelEingabe {
    pub vz: Vz,
    pub veranlagung: Veranlagung,
    pub zu_versteuerndes_einkommen: Euro,
    pub ausserordentliche_einkuenfte: Euro,
}

/// § 34 Abs. 1 `EStG` Fuenftelregelung, EURO (`runner.py` `catala_fuenftel`).
///
/// Anders als [`crate::tarif::fuenftel`] (rechnet in Cent) teilt Python hier in ganzen EURO:
/// `Tarif(zvE // 5)` mit `zvE` in Euro, nicht in Cent -- darum ein eigener Rechenweg.
///
/// - `verbleibend = zvE - ao >= 0`: `est1 + 5 x (Tarif(verbleibend + ao // 5) - est1)`
/// - `verbleibend < 0`, `zvE > 0` (S. 3): `5 x Tarif(zvE // 5)`
///
/// # Errors
/// [`EngineFehler::FuenftelZveNichtPositiv`] bei `verbleibend < 0` und `zvE <= 0`;
/// [`EngineFehler::Catala`]; [`EngineFehler::Ueberlauf`].
///
/// ```
/// # use engine::zugriff::teil2::est::*;
/// # use engine::tarif::Veranlagung;
/// # use domain::{Euro, Vz};
/// let e = FuenftelEingabe { vz: Vz::Vz2025, veranlagung: Veranlagung::Einzel,
///     zu_versteuerndes_einkommen: Euro::new(10_000), ausserordentliche_einkuenfte: Euro::new(0) };
/// assert_eq!(fuenftel(&e).unwrap(), Euro::new(0));
/// ```
pub fn fuenftel(e: &FuenftelEingabe) -> Result<Euro, EngineFehler> {
    let zve = i128::from(e.zu_versteuerndes_einkommen.get());
    let ao = i128::from(e.ausserordentliche_einkuenfte.get());
    let verbleibend = zve - ao;
    let tarif = |x: i128| -> Result<i128, EngineFehler> {
        let x = i64::try_from(x).map_err(|_| super::UEBERLAUF)?;
        Ok(i128::from(tarif_cent(e.vz, e.veranlagung, x)?))
    };
    let cent = if verbleibend < 0 {
        if zve <= 0 {
            return Err(EngineFehler::FuenftelZveNichtPositiv);
        }
        5 * tarif(zve.div_euclid(5))?
    } else {
        let est1 = tarif(verbleibend)?;
        let est2 = tarif(verbleibend + ao.div_euclid(5))?;
        est1 + 5 * (est2 - est1)
    };
    super::euro(cent.div_euclid(100))
}

/// Ein Zweig von `runner.catala_est`. Reihenfolge der Python-Pruefung (erste passende gewinnt):
/// `sanierungsaufwendungen` in s, `bruttolistenpreis` in s, `gesamtfall` truthy,
/// `gewerbesteuer` truthy, `koerperschaft` truthy, `entfernung_km_roh` in s,
/// `arbeitszimmer_vorhanden` in s, `bruttoarbeitslohn_a` in s, `bruttoarbeitslohn` in s,
/// `ausserordentliche_einkuenfte` in s, sonst Tarif.
#[derive(Debug, Clone, Copy)]
pub enum Sachverhalt {
    Sanierung(SanierungEingabe),
    KfzNutzungswert(KfzNutzungswertEingabe),
    Gesamtfall(GesamtfallEingabe),
    Gewerbesteuer(GewstEingabe),
    Koerperschaft(KstEingabe),
    /// Teil-1-Accessor `entfernungspauschale`, EURO.
    Entfernungspauschale(EntfernungspauschaleEingabe),
    /// Teil-1-Accessor `raumkosten`, EURO.
    Arbeitszimmer(RaumkostenEingabe),
    BruttoarbeitslohnZusammen(EstZusammenEingabe),
    Bruttoarbeitslohn(EstEinzelEingabe),
    Fuenftel(FuenftelEingabe),
    Tarif(TarifEingabe),
}

/// Ergebnis von [`est`]: die Einheit haengt am Zweig.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstBetrag {
    Euro(Euro),
    Cent(Cent),
}

/// `runner.catala_est`: rechnet den Zweig und liefert dessen native Einheit -- CENT fuer
/// Sanierung, Kfz, `GewSt`, `KSt`; EURO fuer alle anderen.
///
/// # Errors
/// Die Fehler des jeweiligen Zweigs.
///
/// ```
/// # use engine::zugriff::teil2::{est::*, p35c::SanierungEingabe};
/// # use domain::{Cent, Euro};
/// # let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
/// # let p = bindung::Params::lade(&root).unwrap();
/// let s = Sachverhalt::Sanierung(SanierungEingabe { sanierungsaufwendungen: Euro::new(1000),
///     ist_uebernaechstes_foerderjahr: false });
/// assert_eq!(est(&s, &p).unwrap(), EstBetrag::Cent(Cent::new(7000)));
/// ```
pub fn est(s: &Sachverhalt, p: &Params) -> Result<EstBetrag, EngineFehler> {
    Ok(match s {
        Sachverhalt::Sanierung(e) => EstBetrag::Cent(p35c_ermaessigung_cent(e)?),
        Sachverhalt::KfzNutzungswert(e) => EstBetrag::Cent(kfz_nutzungswert_monat_cent(e)?),
        Sachverhalt::Gesamtfall(e) => EstBetrag::Euro(gesamt(e, p)?),
        Sachverhalt::Gewerbesteuer(e) => EstBetrag::Cent(gewst(e)?),
        Sachverhalt::Koerperschaft(e) => EstBetrag::Cent(kst_nenner_b(e)?),
        Sachverhalt::Entfernungspauschale(e) => EstBetrag::Euro(entfernungspauschale(e, p)?),
        Sachverhalt::Arbeitszimmer(e) => EstBetrag::Euro(raumkosten(e, p)?),
        Sachverhalt::BruttoarbeitslohnZusammen(e) => EstBetrag::Euro(est_zusammen(e)?),
        Sachverhalt::Bruttoarbeitslohn(e) => EstBetrag::Euro(est_einzel(e)?),
        Sachverhalt::Fuenftel(e) => EstBetrag::Euro(fuenftel(e)?),
        Sachverhalt::Tarif(e) => EstBetrag::Euro(tarif_est(e)?),
    })
}
