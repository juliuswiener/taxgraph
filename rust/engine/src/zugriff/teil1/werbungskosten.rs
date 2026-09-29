//! Werbungskosten-Bausteine aus runner.py: Arbeitszimmer/Homeoffice (`catala_raumkosten`),
//! Entfernungspauschale (`catala_entfernungspauschale`, `catala_ep_ab_21km`). Alle EURO.
use bindung::Params;
use domain::{Cent, Euro, Vz};
use rust_decimal::Decimal;

use super::afa::{p6_2_gwg, P62GwgEingabe};
use super::fehler::{in_cent, ok, EngineFehler};
use super::reisekosten::{
    dhf_abzug, uebernachtung_abzug, verpflegung_abzug, DhfEingabe, UebernachtungEingabe, VerpflegungEingabe,
};
use crate::arbeitszimmer::{self, ArbeitszimmerEingabe};
use crate::entfernungspauschale::{self as ep_scope, EntfernungspauschaleEingabe as EpScopeEingabe};

/// Eingabe fuer [`raumkosten`] (Sachverhalt-Schluessel wie in runner.py).
#[derive(Debug, Clone, Copy)]
pub struct RaumkostenEingabe {
    /// `s["veranlagungszeitraum"]` (Pflicht).
    pub veranlagungszeitraum: Vz,
    /// PARITÄT: Python setzt fehlend = False (fail-open)
    pub arbeitszimmer_vorhanden: bool,
    /// PARITÄT: Python setzt fehlend = False (fail-open)
    pub ist_mittelpunkt: bool,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub tatsaechliche_aufwendungen: Euro,
    /// PARITÄT: Python setzt fehlend = False (fail-open)
    pub jahrespauschale_gewaehlt: bool,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub monate_ohne_mittelpunkt: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub homeoffice_tage: i64,
}

/// `catala_raumkosten` -- § 4 Abs. 5 Nr. 6b/6c `EStG`: Arbeitszimmer- plus Homeoffice-Abzug,
/// EURO (abgerundet wie Pythons `// 100`).
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler, fehlenden Parametern oder Ueberlauf.
///
/// ```
/// use engine::zugriff::teil1::werbungskosten::{raumkosten, RaumkostenEingabe};
/// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = RaumkostenEingabe {
///     veranlagungszeitraum: domain::Vz::Vz2025, arbeitszimmer_vorhanden: false, ist_mittelpunkt: false,
///     tatsaechliche_aufwendungen: domain::Euro::new(0), jahrespauschale_gewaehlt: false,
///     monate_ohne_mittelpunkt: 0, homeoffice_tage: 100,
/// };
/// assert_eq!(raumkosten(&e, &p).unwrap(), domain::Euro::new(600));
/// ```
pub fn raumkosten(e: &RaumkostenEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let r = p.arbeitszimmer(e.veranlagungszeitraum)?;
    let out = arbeitszimmer::berechnen(ArbeitszimmerEingabe {
        arbeitszimmer_vorhanden: e.arbeitszimmer_vorhanden,
        ist_mittelpunkt: e.ist_mittelpunkt,
        tatsaechliche_aufwendungen: in_cent(e.tatsaechliche_aufwendungen)?,
        jahrespauschale_gewaehlt: e.jahrespauschale_gewaehlt,
        monate_ohne_mittelpunkt: e.monate_ohne_mittelpunkt,
        homeoffice_tage: e.homeoffice_tage,
        jahrespauschale: in_cent(r.jahrespauschale)?,
        tagespauschale_pro_tag: in_cent(r.tagespauschale_pro_tag)?,
        tagespauschale_hoechstbetrag: in_cent(r.tagespauschale_hoechstbetrag)?,
    })?;
    Ok(Cent::new(out.abzug_gesamt_cent).floor_euro())
}

/// Eingabe fuer [`entfernungspauschale`] und [`ep_ab_21km`].
#[derive(Debug, Clone, Copy)]
pub struct EntfernungspauschaleEingabe {
    /// `s["veranlagungszeitraum"]` (Pflicht).
    pub veranlagungszeitraum: Vz,
    /// `Decimal(str(s["entfernung_km_roh"]))` (Pflicht), exakt.
    pub entfernung_km_roh: Decimal,
    /// `int(s["arbeitstage"])` (Pflicht).
    pub arbeitstage: i64,
    /// PARITÄT: Python setzt fehlend = False (fail-open)
    pub eigenes_oder_ueberlassenes_kfz: bool,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub oepnv_kosten_jahr: Euro,
}

/// Euro-Satz je km (`0.30`) in Cent. Python: `Money(f"{satz:.2f}")`.
fn satz_cent(satz: Decimal) -> Result<Cent, EngineFehler> {
    let c = satz * Decimal::ONE_HUNDRED;
    if !c.fract().is_zero() {
        return Err(EngineFehler::NichtCentGenau(satz));
    }
    i64::try_from(c).map(Cent::new).map_err(|_| EngineFehler::Ueberlauf("Satz->Cent"))
}

/// `catala_entfernungspauschale` -- § 9 Abs. 1 S. 3 Nr. 4/4a `EStG`: abziehbarer Betrag nach
/// Hoechstbetrag, EURO (abgerundet).
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler, fehlenden Parametern oder Ueberlauf.
///
/// ```
/// use engine::zugriff::teil1::werbungskosten::{entfernungspauschale, EntfernungspauschaleEingabe};
/// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = EntfernungspauschaleEingabe {
///     veranlagungszeitraum: domain::Vz::Vz2025, entfernung_km_roh: rust_decimal::Decimal::from(100),
///     arbeitstage: 220, eigenes_oder_ueberlassenes_kfz: true, oepnv_kosten_jahr: domain::Euro::new(0),
/// };
/// assert_eq!(entfernungspauschale(&e, &p).unwrap(), domain::Euro::new(8008));
/// ```
pub fn entfernungspauschale(e: &EntfernungspauschaleEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let r = p.entfernungspauschale(e.veranlagungszeitraum)?;
    let out = ep_scope::berechnen(EpScopeEingabe {
        entfernung_km_roh: e.entfernung_km_roh,
        arbeitstage: e.arbeitstage,
        eigenes_oder_ueberlassenes_kfz: e.eigenes_oder_ueberlassenes_kfz,
        oepnv_kosten_jahr: in_cent(e.oepnv_kosten_jahr)?,
        satz_bis_20_km: satz_cent(r.satz_bis_20_km)?,
        satz_ab_21_km: satz_cent(r.satz_ab_21_km)?,
        staffelgrenze_km: r.staffelgrenze_km,
        hoechstbetrag: in_cent(r.hoechstbetrag_ohne_kfz)?,
    })?;
    Ok(Cent::new(out.abziehbarer_betrag_cent).floor_euro())
}

/// Euro-Satz in ganzen Cent, abgeschnitten. Python: `int(Decimal(str(satz)) * 100)`.
fn satz_cent_abgeschnitten(satz: Decimal) -> Result<i64, EngineFehler> {
    i64::try_from((satz * Decimal::ONE_HUNDRED).trunc()).map_err(|_| EngineFehler::Ueberlauf("Satz->Cent"))
}

/// `catala_ep_ab_21km` -- § 9 Abs. 1 S. 3 Nr. 4 S. 2 `EStG`: der ab dem 21. vollen km erhoehte
/// Teil der Entfernungspauschale (Bemessungsbasis der § 101-Mobilitaetspraemie), EURO.
///
/// Aus denselben Saetzen wie [`entfernungspauschale`]. Konservativ abgerundet und am tatsaechlich
/// abziehbaren Rest gekappt (abziehbarer Betrag minus Anteil bis 20 km): nie mehr ab-21-km als
/// real beruecksichtigt. Fuer eine Praemie (Auszahlung) ist Unter-Ansatz die sichere Richtung.
///
/// # Errors
/// Wie [`entfernungspauschale`].
///
/// ```
/// use engine::zugriff::teil1::werbungskosten::{ep_ab_21km, EntfernungspauschaleEingabe};
/// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = EntfernungspauschaleEingabe {
///     veranlagungszeitraum: domain::Vz::Vz2025, entfernung_km_roh: rust_decimal::Decimal::from(30),
///     arbeitstage: 100, eigenes_oder_ueberlassenes_kfz: false, oepnv_kosten_jahr: domain::Euro::new(0),
/// };
/// assert_eq!(ep_ab_21km(&e, &p).unwrap(), domain::Euro::new(380));
/// ```
pub fn ep_ab_21km(e: &EntfernungspauschaleEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let r = p.entfernungspauschale(e.veranlagungszeitraum)?;
    // § 9 Abs. 1 S. 3 Nr. 4 S. 4: nur volle Entfernungs-km (int() schneidet Richtung 0 ab).
    let km_voll = i64::try_from(e.entfernung_km_roh.trunc()).map_err(|_| EngineFehler::Ueberlauf("km"))?;
    let grenze = r.staffelgrenze_km;
    let satz_ab21_ct = satz_cent_abgeschnitten(r.satz_ab_21_km)?;
    let satz_bis20_ct = satz_cent_abgeschnitten(r.satz_bis_20_km)?;
    let ueber = ok(km_voll.checked_sub(grenze), "km - grenze")?.max(0);
    let ab21_roh = ok(
        e.arbeitstage
            .checked_mul(ueber)
            .and_then(|x| x.checked_mul(satz_ab21_ct))
            .and_then(|x| x.checked_div_euclid(100)),
        "ab21_roh",
    )?;
    let ep_bis20 = ok(
        e.arbeitstage
            .checked_mul(km_voll.min(grenze))
            .and_then(|x| x.checked_mul(satz_bis20_ct))
            .and_then(|x| x.checked_div_euclid(100)),
        "ep_bis20",
    )?;
    let gesamt = entfernungspauschale(e, p)?.get();
    let rest = ok(gesamt.checked_sub(ep_bis20), "gesamt - ep_bis20")?;
    Ok(Euro::new(ab21_roh.min(rest).max(0)))
}

/// Eingabe fuer [`werbungskosten_n`]. Jeder Zweig laeuft nur, wenn sein Schluessel im
/// Sachverhalt steht (`"k" in s`) -- `None` heisst "Schluessel fehlt", nicht "0".
#[derive(Debug, Clone, Copy)]
pub struct WerbungskostenNEingabe {
    /// `"entfernung_km_roh" in s`.
    pub entfernung: Option<EntfernungspauschaleEingabe>,
    /// `"unterkunftskosten_monat" in s`.
    pub doppelte_haushaltsfuehrung: Option<DhfEingabe>,
    /// `tage_24h`, `tage_an_abreise` oder `tage_ueber_8h_eintaegig` in `s`.
    pub verpflegung: Option<VerpflegungEingabe>,
    /// `"uebernachtung_kosten_monat" in s`.
    pub uebernachtung: Option<UebernachtungEingabe>,
    /// `"am_anschaffungskosten" in s` -- Arbeitsmittel, Level-1-GWG-Sofortabzug.
    pub am_anschaffungskosten: Option<Euro>,
}

/// `catala_werbungskosten_n` -- § 9 `EStG` Werbungskosten Anlage N, ROH-Summe in EURO, OHNE
/// § 9a-Arbeitnehmer-Pauschbetrag (den wendet der Tarif `festzusetzende_est_einzel` intern an;
/// hier waere er ein doppelter Abzug).
///
/// Summe aus Entfernungspauschale, doppelter Haushaltsfuehrung, Verpflegung (§ 9 Abs. 4a),
/// Uebernachtung (§ 9 Abs. 1 Nr. 5a) und Arbeitsmitteln (§ 9 Abs. 1 Nr. 7 i.V.m. § 6 Abs. 2,
/// Sofortabzug bis 800 EUR; der § 7-AfA-Zweig ueber 800 ist ungebunden, die Haut sperrt ihn).
///
/// # Errors
/// [`EngineFehler`] aus den Zweigen.
///
/// ```
/// use engine::zugriff::teil1::werbungskosten::{werbungskosten_n, WerbungskostenNEingabe};
/// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = WerbungskostenNEingabe {
///     entfernung: None, doppelte_haushaltsfuehrung: None, verpflegung: None, uebernachtung: None,
///     am_anschaffungskosten: Some(domain::Euro::new(500)),
/// };
/// assert_eq!(werbungskosten_n(&e, &p).unwrap(), domain::Euro::new(500));
/// ```
pub fn werbungskosten_n(e: &WerbungskostenNEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let mut wk: i64 = 0;
    if let Some(ep) = &e.entfernung {
        wk = ok(wk.checked_add(entfernungspauschale(ep, p)?.get()), "wk ep")?;
    }
    if let Some(dhf) = &e.doppelte_haushaltsfuehrung {
        wk = ok(wk.checked_add(dhf_abzug(dhf, p)?), "wk dhf")?;
    }
    if let Some(vpf) = &e.verpflegung {
        wk = ok(wk.checked_add(verpflegung_abzug(vpf, p)?), "wk vpf")?;
    }
    if let Some(uen) = &e.uebernachtung {
        wk = ok(wk.checked_add(uebernachtung_abzug(uen, p)?), "wk uen")?;
    }
    if let Some(ak) = e.am_anschaffungskosten {
        let gwg = p6_2_gwg(&P62GwgEingabe { gwg_anschaffungskosten_netto: ak })?;
        wk = ok(wk.checked_add(gwg.get()), "wk am")?;
    }
    Ok(Euro::new(wk))
}
