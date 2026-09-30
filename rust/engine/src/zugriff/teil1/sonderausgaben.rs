//! Sonderausgaben-Accessoren aus runner.py: § 10b, § 10 Abs. 1 Nr. 4 (+ Abs. 4b S. 3),
//! § 10 Abs. 1 Nr. 3/3a, § 10 Abs. 1 Nr. 7. Alle EURO rein, EURO raus.
use domain::Euro;

use super::fehler::{in_cent, ok, EngineFehler};
use crate::berufsausbildung::{self, BerufsausbildungEingabe};
use crate::kirchensteuer::{self, KirchensteuerabzugEingabe};
use crate::spenden::{self, SpendenAbzugEingabe};
use crate::vorsorgeaufwendungen::{self, VorsorgeaufwendungenEingabe};

/// Eingabe fuer [`p10b_spenden`].
#[derive(Debug, Clone, Copy)]
pub struct P10bSpendenEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub zuwendungen: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: Hoechstbetrag 0)
    pub gesamtbetrag_der_einkuenfte: Euro,
}

/// `catala_p10b_spenden` -- § 10b Abs. 1 S. 1 `EStG`: min(Zuwendungen; 20 % des `GdE`), EURO.
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::sonderausgaben::{p10b_spenden, P10bSpendenEingabe};
/// let e = P10bSpendenEingabe { zuwendungen: Euro::new(5000), gesamtbetrag_der_einkuenfte: Euro::new(10_000) };
/// assert_eq!(p10b_spenden(&e).unwrap(), Euro::new(2000));
/// ```
pub fn p10b_spenden(e: &P10bSpendenEingabe) -> Result<Euro, EngineFehler> {
    let c = spenden::berechnen(SpendenAbzugEingabe {
        zuwendungen: in_cent(e.zuwendungen)?,
        gesamtbetrag_der_einkuenfte: in_cent(e.gesamtbetrag_der_einkuenfte)?,
    })?;
    Ok(c.floor_euro())
}

/// Eingabe fuer [`p10_kist`] und [`p10_4b_erstattungsueberhang`].
#[derive(Debug, Clone, Copy)]
pub struct P10KistEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub gezahlte_kirchensteuer: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: keine Erstattung gegengerechnet)
    pub erstattete_kirchensteuer: Euro,
}

/// `catala_p10_kist` -- § 10 Abs. 1 Nr. 4 `EStG`: gezahlte minus erstattete Kirchensteuer, nie
/// negativ, EURO. Ein Ueberhang gehoert nicht hierher, sondern nach
/// [`p10_4b_erstattungsueberhang`].
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::sonderausgaben::{p10_kist, P10KistEingabe};
/// let e = P10KistEingabe { gezahlte_kirchensteuer: Euro::new(300), erstattete_kirchensteuer: Euro::new(500) };
/// assert_eq!(p10_kist(&e).unwrap(), Euro::new(0));
/// ```
pub fn p10_kist(e: &P10KistEingabe) -> Result<Euro, EngineFehler> {
    let c = kirchensteuer::berechnen(KirchensteuerabzugEingabe {
        gezahlte_kirchensteuer: in_cent(e.gezahlte_kirchensteuer)?,
        erstattete_kirchensteuer: in_cent(e.erstattete_kirchensteuer)?,
    })?;
    Ok(c.floor_euro())
}

/// `catala_p10_4b_erstattungsueberhang` -- § 10 Abs. 4b S. 3 `EStG`: Erstattungsueberhang der
/// Kirchensteuer (Nr. 4), der dem `GdE` hinzuzurechnen ist, EURO: max(0, erstattet - gezahlt).
/// Nicht umgesetzt: Verrechnung nach S. 2 und der Ueberhang bei Nr. 3 (KV/PV).
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] bei `i64`-Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::sonderausgaben::{p10_4b_erstattungsueberhang, P10KistEingabe};
/// let e = P10KistEingabe { gezahlte_kirchensteuer: Euro::new(300), erstattete_kirchensteuer: Euro::new(500) };
/// assert_eq!(p10_4b_erstattungsueberhang(&e).unwrap(), Euro::new(200));
/// ```
pub fn p10_4b_erstattungsueberhang(e: &P10KistEingabe) -> Result<Euro, EngineFehler> {
    let d = ok(
        e.erstattete_kirchensteuer
            .get()
            .checked_sub(e.gezahlte_kirchensteuer.get()),
        "p10_4b",
    )?;
    Ok(Euro::new(d.max(0)))
}

/// Eingabe fuer [`p10_kv_pv`].
#[derive(Debug, Clone, Copy)]
pub struct P10KvPvEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub basis_kv_pv: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub weitere_vorsorgeaufwendungen: Euro,
    /// PARITÄT: Python setzt fehlend = False (fail-open: Hoechstbetrag 2.800 statt 1.900)
    pub mit_anspruch_auf_zuschuss: bool,
}

/// `catala_p10_kv_pv` -- § 10 Abs. 1 Nr. 3/3a `EStG`: abziehbare KV/PV-Vorsorge, EURO.
/// max(Basis; min(Basis + weitere; Hoechstbetrag)) -- die Basisabsicherung ist immer voll
/// abziehbar (§ 10 Abs. 4 S. 4), nur die weiteren werden gedeckelt.
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::sonderausgaben::{p10_kv_pv, P10KvPvEingabe};
/// let e = P10KvPvEingabe {
///     basis_kv_pv: Euro::new(3000), weitere_vorsorgeaufwendungen: Euro::new(500), mit_anspruch_auf_zuschuss: false,
/// };
/// assert_eq!(p10_kv_pv(&e).unwrap(), Euro::new(3000));
/// ```
pub fn p10_kv_pv(e: &P10KvPvEingabe) -> Result<Euro, EngineFehler> {
    let c = vorsorgeaufwendungen::berechnen(VorsorgeaufwendungenEingabe {
        basis: in_cent(e.basis_kv_pv)?,
        weitere: in_cent(e.weitere_vorsorgeaufwendungen)?,
        mit_zuschuss: e.mit_anspruch_auf_zuschuss,
    })?;
    Ok(c.floor_euro())
}

/// Eingabe fuer [`p10_1_7_berufsausbildung`].
#[derive(Debug, Clone, Copy)]
pub struct P1017BerufsausbildungEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub berufsausbildung_aufwendungen: Euro,
}

/// `catala_p10_1_7_berufsausbildung` -- § 10 Abs. 1 Nr. 7 S. 1 `EStG`: min(Aufwendungen; 6.000),
/// EURO.
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::sonderausgaben::{p10_1_7_berufsausbildung, P1017BerufsausbildungEingabe};
/// let e = P1017BerufsausbildungEingabe { berufsausbildung_aufwendungen: Euro::new(7000) };
/// assert_eq!(p10_1_7_berufsausbildung(&e).unwrap(), Euro::new(6000));
/// ```
pub fn p10_1_7_berufsausbildung(e: &P1017BerufsausbildungEingabe) -> Result<Euro, EngineFehler> {
    let c = berufsausbildung::berechnen(BerufsausbildungEingabe {
        aufwendungen: in_cent(e.berufsausbildung_aufwendungen)?,
    })?;
    Ok(c.floor_euro())
}
