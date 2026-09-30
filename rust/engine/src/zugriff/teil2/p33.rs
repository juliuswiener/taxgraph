//! § 33 Abs. 2a, § 33a, § 33b `EStG` -- Pauschbetraege und Unterhalt (`runner.py`, reines
//! Python).
use domain::{Euro, Vz};

use bindung::Params;

use super::{euro, z, EngineFehler};

/// Eingabe fuer [`behinderten_pb`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehindertenPbEingabe {
    pub vz: Vz,
    /// PARITÄT: Python setzt fehlend = False.
    pub ist_hilflos_blind_taubblind: bool,
    /// PARITÄT: Python setzt fehlend = 0 (kein Pauschbetrag).
    pub grad_der_behinderung: i64,
}

/// § 33b Abs. 3 `EStG`: Behinderten-Pauschbetrag, EURO. Blind/hilflos -> Hoechstbetrag;
/// `GdB` < 20 -> 0; sonst Staffel auf `min(100, gdb // 10 * 10)`.
///
/// # Errors
/// [`EngineFehler::TabelleOhneEintrag`], wenn die Staffel die Stufe nicht kennt.
///
/// ```
/// # use engine::zugriff::teil2::p33::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = BehindertenPbEingabe { vz: Vz::Vz2025, ist_hilflos_blind_taubblind: false, grad_der_behinderung: 55 };
/// assert_eq!(behinderten_pb(&e, &p).unwrap(), Euro::new(1140));
/// ```
pub fn behinderten_pb(e: &BehindertenPbEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let v = p.p33b_pauschbetraege(e.vz)?;
    if e.ist_hilflos_blind_taubblind {
        return Ok(v.blind_hilflos_taubblind);
    }
    let gdb = e.grad_der_behinderung;
    if gdb < 20 {
        return Ok(Euro::new(0));
    }
    let stufe = (gdb.div_euclid(10) * 10).min(100);
    // GdB ab 20: die Stufe ist ein Vielfaches von 10 zwischen 20 und 100.
    debug_assert!((20..=100).contains(&stufe) && stufe % 10 == 0);
    v.gdb_staffel
        .get(&stufe)
        .copied()
        .ok_or(EngineFehler::TabelleOhneEintrag {
            tabelle: "gdb_staffel",
            schluessel: stufe,
        })
}

/// Eingabe fuer [`pflege_pb`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PflegePbEingabe {
    pub vz: Vz,
    /// PARITÄT: Python setzt fehlend = False.
    pub ist_hilflos: bool,
    /// PARITÄT: Python setzt fehlend = 0 (kein Pauschbetrag).
    pub pflegegrad: i64,
}

/// § 33b Abs. 6 `EStG`: Pflege-Pauschbetrag, EURO. `ist_hilflos` hat Vorrang; sonst Staffel,
/// unbekannter Pflegegrad -> 0.
///
/// # Errors
/// Parameterfehler (`params/<vz>` unlesbar).
///
/// ```
/// # use engine::zugriff::teil2::p33::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = PflegePbEingabe { vz: Vz::Vz2025, ist_hilflos: false, pflegegrad: 3 };
/// assert_eq!(pflege_pb(&e, &p).unwrap(), Euro::new(1100));
/// ```
pub fn pflege_pb(e: &PflegePbEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let v = p.p33b_pauschbetraege(e.vz)?;
    if e.ist_hilflos {
        return Ok(v.pflege_hilflos);
    }
    Ok(v.pflege_staffel
        .get(&e.pflegegrad)
        .copied()
        .unwrap_or(Euro::new(0)))
}

/// Eingabe fuer [`hinterbliebenen_pb`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HinterbliebenenPbEingabe {
    pub vz: Vz,
    /// PARITÄT: Python setzt fehlend = False.
    pub hat_hinterbliebenenbezuege: bool,
}

/// § 33b Abs. 4 `EStG`: Hinterbliebenen-Pauschbetrag, EURO.
///
/// # Errors
/// Parameterfehler (`params/<vz>` unlesbar).
///
/// ```
/// # use engine::zugriff::teil2::p33::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = HinterbliebenenPbEingabe { vz: Vz::Vz2025, hat_hinterbliebenenbezuege: true };
/// assert_eq!(hinterbliebenen_pb(&e, &p).unwrap(), Euro::new(370));
/// ```
pub fn hinterbliebenen_pb(e: &HinterbliebenenPbEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    Ok(if e.hat_hinterbliebenenbezuege {
        p.p33b_pauschbetraege(e.vz)?.hinterbliebenen
    } else {
        Euro::new(0)
    })
}

/// Eingabe fuer [`p33_2a_fahrtkostenpauschale`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FahrtkostenpauschaleEingabe {
    pub vz: Vz,
    /// Merkzeichen aG/Bl/TBl/H. PARITÄT: Python setzt fehlend = False.
    pub hat_ag_bl_tbl_h: bool,
    /// `GdB` >= 80 oder >= 70 mit G. PARITÄT: Python setzt fehlend = False.
    pub hat_gdb80_oder_70g: bool,
}

/// § 33 Abs. 2a `EStG`: behinderungsbedingte Fahrtkostenpauschale, EURO. 4.500 schliesst 900
/// aus (S. 5).
///
/// # Errors
/// Parameterfehler (`params/<vz>` unlesbar).
///
/// ```
/// # use engine::zugriff::teil2::p33::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = FahrtkostenpauschaleEingabe { vz: Vz::Vz2025, hat_ag_bl_tbl_h: true, hat_gdb80_oder_70g: true };
/// assert_eq!(p33_2a_fahrtkostenpauschale(&e, &p).unwrap(), Euro::new(4500));
/// ```
pub fn p33_2a_fahrtkostenpauschale(
    e: &FahrtkostenpauschaleEingabe,
    p: &Params,
) -> Result<Euro, EngineFehler> {
    let v = p.fahrtkostenpauschale_p33_2a(e.vz)?;
    Ok(if e.hat_ag_bl_tbl_h {
        v.pauschale_4500
    } else if e.hat_gdb80_oder_70g {
        v.pauschale_900
    } else {
        Euro::new(0)
    })
}

/// Eingabe fuer [`p33a_unterhalt`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnterhaltEingabe {
    pub vz: Vz,
    pub aufwendungen: Euro,
    /// PARITÄT: Python setzt fehlend = 0.
    pub kv_pv_beitraege: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (keine Anrechnung).
    pub andere_einkuenfte_bezuege: Euro,
}

/// § 33a Abs. 1 `EStG` Grundfreibetrag je VZ. PARITÄT: Python hartkodiert
/// (`runner.py` `_GFB_33A`), nicht aus `params/<vz>`.
const fn gfb_33a(vz: Vz) -> i128 {
    match vz {
        Vz::Vz2024 => 11_784,
        Vz::Vz2025 => 12_096,
        Vz::Vz2026 => 12_348,
    }
}

/// § 33a Abs. 1 `EStG`: Unterhaltsabzug, EURO.
/// `min(aufw, max(0, GFB + kv_pv - max(0, andere - 624)))`; Schonbetrag 624 hartkodiert.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::p33::*;
/// # use domain::{Euro, Vz};
/// let e = UnterhaltEingabe { vz: Vz::Vz2026, aufwendungen: Euro::new(15000),
///     kv_pv_beitraege: Euro::new(0), andere_einkuenfte_bezuege: Euro::new(0) };
/// assert_eq!(p33a_unterhalt(&e).unwrap(), Euro::new(12348));
/// ```
pub fn p33a_unterhalt(e: &UnterhaltEingabe) -> Result<Euro, EngineFehler> {
    let anrechnung = (z(e.andere_einkuenfte_bezuege) - 624).max(0);
    let hoechst = (gfb_33a(e.vz) + z(e.kv_pv_beitraege) - anrechnung).max(0);
    euro(z(e.aufwendungen).min(hoechst))
}

/// § 33a Abs. 2 `EStG`: Ausbildungsfreibetrag 1.200 EUR je Kind, EURO.
///
/// `anzahl_kinder`: PARITÄT: Python setzt fehlend = 0.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::p33::*;
/// # use domain::Euro;
/// assert_eq!(p33a_ausbildungsfreibetrag(2).unwrap(), Euro::new(2400));
/// ```
pub fn p33a_ausbildungsfreibetrag(anzahl_kinder: i64) -> Result<Euro, EngineFehler> {
    euro(i128::from(anzahl_kinder) * 1200)
}
