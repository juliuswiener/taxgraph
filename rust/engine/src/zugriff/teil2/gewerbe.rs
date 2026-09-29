//! `GewStG` §§ 7-11 / § 35 `EStG` und `KStG` Nenner B (`runner.py`, reines Python). Alle
//! Ergebnisse in CENT.
use domain::{Cent, Euro, Vz};

use super::{cent, z, EngineFehler};

/// § 8 Nr. 1 `GewStG` Finanzierungsanteile. PARITÄT: Python setzt jedes fehlende Feld = 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hinzurechnung {
    pub entgelte_schulden: Euro,
    pub renten: Euro,
    pub stille: Euro,
    pub miet_beweglich: Euro,
    pub miet_unbeweglich: Euro,
    pub rechte: Euro,
}

/// § 9 `GewStG` Kuerzungen. PARITÄT: Python setzt jedes fehlende Feld = 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kuerzung {
    /// Nur EZ 2024 (1,2 % des Einheitswerts).
    pub einheitswert: Euro,
    /// Ab EZ 2025 (tatsaechliche Grundsteuer).
    pub grundsteuer: Euro,
    pub gewinnanteile_mitunternehmer: Euro,
    pub schachteldividenden: Euro,
}

/// Was [`gewst`] liefert (`gewst_output`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GewstAusgabe {
    /// Steuermessbetrag (Python: jeder `gewst_output` ausser `"p35_anrechnung"`).
    Messbetrag,
    /// § 35 `EStG`-Anrechnung mit Hebesatz in Prozent.
    P35Anrechnung { hebesatz: i64 },
}

/// Eingabe fuer [`gewst`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GewstEingabe {
    /// Python akzeptiert jedes Jahr und unterscheidet nur `<= 2024`; hier nur 2024-2026.
    pub vz: Vz,
    pub ausgabe: GewstAusgabe,
    /// PARITÄT: Python setzt fehlend = 0.
    pub gewinn_gewerbebetrieb: Euro,
    pub hinzurechnung: Hinzurechnung,
    pub kuerzung: Kuerzung,
    /// § 10a-Fehlbetrag. PARITÄT: Python setzt fehlend = 0.
    pub fehlbetrag_bestand: Euro,
}

fn hinzurechnung_p8(h: &Hinzurechnung) -> i128 {
    let summe = z(h.entgelte_schulden)
        + z(h.renten)
        + z(h.stille)
        + z(h.miet_beweglich).div_euclid(5)
        + z(h.miet_unbeweglich).div_euclid(2)
        + z(h.rechte).div_euclid(4);
    (summe - 200_000).max(0).div_euclid(4)
}

fn kuerzung_p9(k: &Kuerzung, vz: Vz) -> i128 {
    let grund = if vz.jahr() <= 2024 {
        (z(k.einheitswert) * 12).div_euclid(1000)
    } else {
        z(k.grundsteuer)
    };
    grund + z(k.gewinnanteile_mitunternehmer) + z(k.schachteldividenden)
}

/// § 7 -> § 10a -> § 11 `GewStG`: Steuermessbetrag in Cent.
fn messbetrag_cent(e: &GewstEingabe) -> i128 {
    let mut ge = z(e.gewinn_gewerbebetrieb) + hinzurechnung_p8(&e.hinzurechnung)
        - kuerzung_p9(&e.kuerzung, e.vz);
    let fehlbetrag = z(e.fehlbetrag_bestand);
    if fehlbetrag != 0 {
        let kapazitaet = 1_000_000 + ((ge - 1_000_000).max(0) * 60).div_euclid(100);
        ge -= fehlbetrag.min(kapazitaet);
    }
    let abgerundet = if ge > 0 { ge.div_euclid(100) * 100 } else { 0 };
    ((abgerundet - 24_500).max(0) * 35).div_euclid(10)
}

/// Gewerbesteuer-Kette, CENT: Steuermessbetrag (§ 11 `GewStG`, Freibetrag 24.500, 3,5 %) oder
/// § 35 `EStG`-Anrechnung `min(4 x Messbetrag, Messbetrag x Hebesatz // 100)`.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::gewerbe::*;
/// # use domain::{Cent, Euro, Vz};
/// let null = Euro::new(0);
/// let e = GewstEingabe { vz: Vz::Vz2025, ausgabe: GewstAusgabe::Messbetrag,
///     gewinn_gewerbebetrieb: Euro::new(100_000), fehlbetrag_bestand: null,
///     hinzurechnung: Hinzurechnung { entgelte_schulden: null, renten: null, stille: null,
///         miet_beweglich: null, miet_unbeweglich: null, rechte: null },
///     kuerzung: Kuerzung { einheitswert: null, grundsteuer: null,
///         gewinnanteile_mitunternehmer: null, schachteldividenden: null } };
/// assert_eq!(gewst(&e).unwrap(), Cent::new(264_250));
/// ```
pub fn gewst(e: &GewstEingabe) -> Result<Cent, EngineFehler> {
    let mb = messbetrag_cent(e);
    match e.ausgabe {
        GewstAusgabe::Messbetrag => cent(mb),
        GewstAusgabe::P35Anrechnung { hebesatz } => {
            cent((mb * 4).min((mb * i128::from(hebesatz)).div_euclid(100)))
        }
    }
}

/// Eingabe fuer [`kst_nenner_b`]. PARITÄT: Python setzt jedes fehlende Betragsfeld = 0 und
/// jedes fehlende Flag = False; nur `gewst_hebesatz` ist Pflicht.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KstEingabe {
    pub gewinn_estg: Euro,
    pub verdeckte_gewinnausschuettung: Euro,
    pub verdeckte_einlage: Euro,
    pub personensteuern: Euro,
    pub geldstrafen: Euro,
    pub dividende_bezuege: Euro,
    pub beteiligung_prozent: i64,
    pub veraeusserungsgewinn: Euro,
    pub zinsaufwand: Euro,
    pub zinsertrag: Euro,
    pub abschreibungen: Euro,
    pub zins_vortrag_bestand: Euro,
    pub ebitda_vortrag_bestand: Euro,
    pub keine_konzern_oder_nahestehende_b: bool,
    pub eigenkapital_escape_c: bool,
    pub verlustvortrag_bestand: Euro,
    pub schaedlicher_erwerb: bool,
    pub antrag_8d: bool,
    pub fortfuehrungs_voraussetzungen: bool,
    pub umsaetze: Euro,
    pub loehne_gehaelter: Euro,
    pub zuwendungen: Euro,
    pub gewst_hebesatz: i64,
}

/// § 8b `KStG`: 95 % netto steuerfrei (Streubesitz < 10 % voll steuerpflichtig).
fn kst_8b_netto(e: &KstEingabe) -> i128 {
    let div = z(e.dividende_bezuege) * 100;
    let steuerfrei = if e.beteiligung_prozent >= 10 { div } else { 0 };
    let ausser = steuerfrei + z(e.veraeusserungsgewinn) * 100;
    ausser - (ausser * 5).div_euclid(100)
}

/// § 8 Abs. 1/3 `KStG` massgebliches Einkommen (vor § 4h/§ 9/§ 10d).
fn massgebliches_einkommen(e: &KstEingabe) -> i128 {
    let slot = (z(e.gewinn_estg) + z(e.verdeckte_gewinnausschuettung) - z(e.verdeckte_einlage)) * 100;
    let addback = (z(e.personensteuern) + z(e.geldstrafen)) * 100;
    slot - kst_8b_netto(e) + addback
}

/// § 4h/§ 8a Zinsschranke: nicht abziehbarer Zinsaufwand (ungeklemmt, s. `runner.py`).
fn nichtabziehbare_zinsen(e: &KstEingabe) -> i128 {
    let zinsauf = z(e.zinsaufwand) * 100;
    let zinsert = z(e.zinsertrag) * 100;
    let ebitda_basis = massgebliches_einkommen(e) + zinsauf + z(e.abschreibungen) * 100 - zinsert;
    let zinsauf_eff = zinsauf + z(e.zins_vortrag_bestand) * 100;
    let deckel = (ebitda_basis * 30).div_euclid(100) + z(e.ebitda_vortrag_bestand) * 100;
    let abziehbar_kern = zinsert + (zinsauf_eff - zinsert).min(deckel);
    let ausnahme = zinsauf - zinsert < 3_000_000 * 100
        || e.keine_konzern_oder_nahestehende_b
        || e.eigenkapital_escape_c;
    zinsauf - if ausnahme { zinsauf_eff } else { abziehbar_kern }
}

/// § 8c/§ 8d `KStG` auf den Verlustbestand.
fn verlustbestand_nach_8c_8d(e: &KstEingabe) -> i128 {
    let bestand = z(e.verlustvortrag_bestand) * 100;
    let suspension = e.schaedlicher_erwerb && e.antrag_8d && e.fortfuehrungs_voraussetzungen;
    if !e.schaedlicher_erwerb || suspension {
        bestand
    } else {
        0
    }
}

fn einkommen_vor_spenden(e: &KstEingabe) -> i128 {
    massgebliches_einkommen(e) + nichtabziehbare_zinsen(e)
}

/// § 9 Abs. 1 Nr. 2 `KStG` Spendenabzug.
fn spendenabzug(e: &KstEingabe) -> i128 {
    let grenze_a = (einkommen_vor_spenden(e) * 20).div_euclid(100);
    let grenze_b = ((z(e.umsaetze) + z(e.loehne_gehaelter)) * 100 * 4).div_euclid(1000);
    (z(e.zuwendungen) * 100).min(grenze_a.max(grenze_b))
}

/// § 10d (Sockel 1 Mio + 70 %) -> zvE, Boden 0.
fn kst_zve(e: &KstEingabe) -> i128 {
    let gde = einkommen_vor_spenden(e) - spendenabzug(e);
    let sockel = 1_000_000 * 100;
    let hoechst = sockel + ((gde - sockel).max(0) * 70).div_euclid(100);
    (gde - verlustbestand_nach_8c_8d(e).min(hoechst)).max(0)
}

/// `GewSt` der `KapGes`: ohne Freibetrag, Gewerbeertrag = Einkommen vor Spenden.
fn kst_gewst(e: &KstEingabe) -> i128 {
    let ge_euro = einkommen_vor_spenden(e).div_euclid(100);
    let abger = if ge_euro > 0 { ge_euro.div_euclid(100) * 100 } else { 0 };
    let messbetrag = (abger * 35).div_euclid(10);
    (messbetrag * i128::from(e.gewst_hebesatz)).div_euclid(100)
}

/// `KStG` Nenner B (Kapitalgesellschaft), CENT: `KSt` 15 % des zvE + `SolZ` 5,5 % der `KSt` + `GewSt`.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::gewerbe::*;
/// # use domain::{Cent, Euro};
/// let n = Euro::new(0);
/// let e = KstEingabe { gewinn_estg: Euro::new(100_000), verdeckte_gewinnausschuettung: n,
///     verdeckte_einlage: n, personensteuern: n, geldstrafen: n, dividende_bezuege: n,
///     beteiligung_prozent: 0, veraeusserungsgewinn: n, zinsaufwand: n, zinsertrag: n,
///     abschreibungen: n, zins_vortrag_bestand: n, ebitda_vortrag_bestand: n,
///     keine_konzern_oder_nahestehende_b: false, eigenkapital_escape_c: false,
///     verlustvortrag_bestand: n, schaedlicher_erwerb: false, antrag_8d: false,
///     fortfuehrungs_voraussetzungen: false, umsaetze: n, loehne_gehaelter: n, zuwendungen: n,
///     gewst_hebesatz: 400 };
/// assert_eq!(kst_nenner_b(&e).unwrap(), Cent::new(1_500_000 + 82_500 + 1_400_000));
/// ```
pub fn kst_nenner_b(e: &KstEingabe) -> Result<Cent, EngineFehler> {
    let kst = (kst_zve(e) * 15).div_euclid(100);
    let solz = (kst * 55).div_euclid(1000);
    cent(kst + solz + kst_gewst(e))
}
