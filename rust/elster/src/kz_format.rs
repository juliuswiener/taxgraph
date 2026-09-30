//! Kz-Format: wie ein Store-Wert in eine ELSTER-Kennzahl geschrieben wird
//! (`est_mapping.py:31-211`, `_cent_nach_kz`, `_kz_wert`, `_jahr_aus_kz_wert`).
//!
//! Rundung „zu Ihren Gunsten" (Anleitung ESt 1 A 2025, `anl_est1a_2025.txt:269-274`):
//! Einnahmen/Einkuenfte werden abgerundet, Abzuege/Aufwendungen/Verluste aufgerundet. Kz vom
//! XSD-Typ Dezimal mit zwei Nachkommastellen (`E60…` und die Liste [`KOMMA_OHNE_E60_KZ`]) werden
//! nicht gerundet, sondern exakt als `"N,NN"` geschrieben.

use domain::{Cent, Euro, Feldtyp};
use serde_json::Value;

use crate::py::{self, PyFehler};

/// Kz, die Abzuege/Aufwendungen/Verluste deklarieren — aufrunden (`est_mapping.py:35-88`).
///
/// Sieben Eintraege tragen kein Bindungsfeld mehr (E0703838, E2001203, E2001505, E2001805,
/// E2002105, E2003104, E2003202); sie bleiben, weil sie sachlich richtig klassifiziert sind.
/// Nicht aufgenommen: Kz, die einen Abzug MINDERN (E0107602, E0506604/05, E0205508, E0700501)
/// und Anrechnungsbetraege auf die Steuer (E0601901, E1905101, E0200301, E19047xx).
pub const ABZUGS_KZ: &[&str] = &[
    "E0703838", "E2000401", "E2000801", "E2000601", "E1901301", "E1901201", "E0161804", "E0104109",
    "E0107208", "E0111215", "E2001203", "E2001505", "E2001805", "E2002105", "E2003104", "E2003202",
    "E2001403", "E2001503", "E2001803", "E2001903", "E2002003", "E0505607", "E0503110", "E0503310",
    "E0107601", "E0108202", "E0108105", "E0304601", "E0506105", "E0120103", "E0124401", "E0203611",
    "E0207611", "E0705701", "E0305201", "E0241901", "E0242001",
];

/// Kz vom XSD-Typ `DezimalzahlNichtNegOhneFuehrNull_MaxL15_MaxVK12_MinNK2_MaxNK2_CType_RABE`
/// (bzw. ohne `NichtNeg` fuer E1905101) ohne E60-Praefix (`est_mapping.py:128-142`). Ohne diese
/// Liste stuende der rohe Euro-Betrag im XML und checkESt lehnte ab („Geldbetraege muessen vom
/// Format '0,00' sein").
pub const KOMMA_OHNE_E60_KZ: &[&str] = &[
    "E0200301", "E0200501", "E1904701", "E1904901", "E1904801", "E1905101",
];

/// Kz vom XSD-Typ `GanzzahlPos`, an denen ERiC eine 0 ablehnt (`est_mapping.py:152-154`).
///
/// ponytail: nur die Spenden-Kz; 14 weitere cent-Kz der Bindung tragen denselben Typ (gezaehlt
/// 2026-09-26). Upgrade: Menge aus [`crate::xsd::kz_meta`] ableiten.
pub const NULL_UNZULAESSIG_KZ: &[&str] = &["E0108105"];

/// Datums-Kz, in die ein Jahres-Wert als `01.01.JJJJ` geschrieben wird (`est_mapping.py:181`).
/// Aus dem XSD abgeleitet (Datums-Typ ∩ Kz-Literale des Moduls).
pub const DATUMS_KZ: &[&str] = &["E1800501", "E1801701", "E1803202"];

/// Wie ein Cent-Betrag in eine Kz geschrieben wird. Ein Kz hat genau ein Format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KzFormat {
    /// Einnahmen/Einkuenfte: auf ganze Euro abrunden (Richtung −∞, Python `//`).
    EuroAbgerundet,
    /// Abzuege/Aufwendungen/Verluste: auf ganze Euro aufrunden.
    EuroAufgerundet,
    /// Dezimal-Kz: Cent exakt als `"N,NN"`.
    KommaCent,
}

/// Das Format einer Kz (`_cent_nach_kz`-Weiche, `est_mapping.py:161-165`).
///
/// ```
/// use elster::{kz_format, KzFormat};
/// assert_eq!(kz_format("E6004901"), KzFormat::KommaCent);
/// assert_eq!(kz_format("E0200301"), KzFormat::KommaCent);
/// assert_eq!(kz_format("E0705701"), KzFormat::EuroAufgerundet);
/// assert_eq!(kz_format("E0200201"), KzFormat::EuroAbgerundet);
/// ```
#[must_use]
pub fn kz_format(kz: &str) -> KzFormat {
    if kz.starts_with("E60") || KOMMA_OHNE_E60_KZ.contains(&kz) {
        KzFormat::KommaCent
    } else if ABZUGS_KZ.contains(&kz) {
        KzFormat::EuroAufgerundet
    } else {
        KzFormat::EuroAbgerundet
    }
}

/// Ein in eine Kz geschriebener Geldbetrag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KzBetrag {
    /// Ganze Euro (XSD-Ganzzahl-Typen).
    Euro(Euro),
    /// Cent-genau, als `"N,NN"` geschrieben.
    Komma(Cent),
}

impl KzBetrag {
    /// Der Wert, wie er in der Deklaration steht (JSON-Zahl bzw. -Text).
    ///
    /// ```
    /// use domain::{Cent, Euro};
    /// use elster::KzBetrag;
    /// assert_eq!(KzBetrag::Euro(Euro::new(3)).als_json(), serde_json::json!(3));
    /// assert_eq!(KzBetrag::Komma(Cent::new(250)).als_json(), serde_json::json!("2,50"));
    /// ```
    #[must_use]
    pub fn als_json(&self) -> Value {
        match self {
            Self::Euro(e) => Value::from(e.get()),
            Self::Komma(c) => Value::String(komma_text(*c)),
        }
    }
}

/// `f"{wert // 100},{wert % 100:02d}"`.
///
/// PARITÄT: P1 — fuer negative Betraege liefert das `"-2,50"` fuer −150 Cent (Python `//` rundet
/// gegen −∞, `%` ist nicht-negativ), obwohl −1,50 € gemeint sind. Korrektur erst nach Paritaet in
/// eigenem Commit (`REWRITE_PLAN.md` §4, F1).
fn komma_text(c: Cent) -> String {
    let euro = c.floor_euro().get();
    let rest = c.get().rem_euclid(100);
    format!("{euro},{rest:02}")
}

/// Store-Cent → Kz-Betrag (`_cent_nach_kz`, `est_mapping.py:157-165`).
///
/// ```
/// use domain::{Cent, Euro};
/// use elster::{cent_nach_kz, KzBetrag};
/// // Einnahme: abrunden
/// assert_eq!(cent_nach_kz(Cent::new(199), "E0200201"), KzBetrag::Euro(Euro::new(1)));
/// // Abzug: aufrunden
/// assert_eq!(cent_nach_kz(Cent::new(101), "E0705701"), KzBetrag::Euro(Euro::new(2)));
/// // Dezimal-Kz: exakt; P1 bei negativen Betraegen
/// assert_eq!(cent_nach_kz(Cent::new(-150), "E6004901").als_json(), serde_json::json!("-2,50"));
/// ```
#[must_use]
pub fn cent_nach_kz(cent: Cent, kz: &str) -> KzBetrag {
    match kz_format(kz) {
        KzFormat::KommaCent => KzBetrag::Komma(cent),
        KzFormat::EuroAufgerundet => KzBetrag::Euro(cent.ceil_euro()),
        KzFormat::EuroAbgerundet => KzBetrag::Euro(cent.floor_euro()),
    }
}

/// Store-Wert → Kz-Wert (`_kz_wert`, `est_mapping.py:198-210`): `cent` gerundet, Jahr in einer
/// Datums-Kz als `01.01.JJJJ`, alles andere unveraendert.
///
/// Der 01.01. ist eine Annahme, keine Kenntnis: der Nutzer nennt das Jahr des Rentenbeginns.
///
/// # Errors
/// [`PyFehler`] `TypeError`, wenn ein `cent`-Wert keine ganze Zahl ist. PARITÄT: Python rechnet
/// eine Gleitkommazahl dort still weiter (Ergebnis `float`); Auflage T des Stores laesst sie nicht
/// zu, der Port weist sie ab.
///
/// ```
/// use domain::Feldtyp;
/// use elster::kz_wert;
/// use serde_json::json;
/// assert_eq!(kz_wert(&json!(2015), "E1800501", Some(Feldtyp::Int)).unwrap(), json!("01.01.2015"));
/// assert_eq!(kz_wert(&json!(12345), "E0200201", Some(Feldtyp::Cent)).unwrap(), json!(123));
/// assert_eq!(kz_wert(&json!("x"), "E0100201", Some(Feldtyp::Text)).unwrap(), json!("x"));
/// ```
pub fn kz_wert(wert: &Value, kz: &str, typ: Option<Feldtyp>) -> Result<Value, PyFehler> {
    if typ == Some(Feldtyp::Cent) {
        let cent = match wert {
            Value::Bool(b) => i64::from(*b),
            Value::Number(n) => n
                .as_i64()
                .ok_or_else(|| PyFehler::typ("cent-Wert ist keine ganze Zahl"))?,
            _ => return Err(PyFehler::typ("unsupported operand type(s) for //")),
        };
        return Ok(cent_nach_kz(Cent::new(cent), kz).als_json());
    }
    if DATUMS_KZ.contains(&kz) {
        if let Value::Number(n) = wert {
            if let Some(jahr) = n.as_i64() {
                return Ok(Value::String(format!("01.01.{jahr:04}")));
            }
        }
    }
    Ok(wert.clone())
}

/// Umkehrung zu [`kz_wert`] fuer Datums-Kz (`_jahr_aus_kz_wert`, `est_mapping.py:183-194`):
/// `"TT.MM.JJJJ"` → Jahr, sonst unveraendert.
///
/// ```
/// use elster::jahr_aus_kz_wert;
/// use serde_json::json;
/// assert_eq!(jahr_aus_kz_wert(&json!(" 01.01.2015 "), "E1800501"), json!(2015));
/// assert_eq!(jahr_aus_kz_wert(&json!("2015"), "E1800501"), json!("2015"));
/// assert_eq!(jahr_aus_kz_wert(&json!("01.01.2015"), "E0200201"), json!("01.01.2015"));
/// ```
#[must_use]
pub fn jahr_aus_kz_wert(wert: &Value, kz: &str) -> Value {
    if !DATUMS_KZ.contains(&kz) {
        return wert.clone();
    }
    let Value::String(s) = wert else {
        return wert.clone();
    };
    // ponytail: `\d` ist in Python jede Unicode-Ziffer; hier nur ASCII (Store-Werte sind ASCII).
    match py::strip(s).as_bytes() {
        [t0, t1, b'.', m0, m1, b'.', j @ ..]
            if j.len() == 4
                && [t0, t1, m0, m1].iter().all(|b| b.is_ascii_digit())
                && j.iter().all(u8::is_ascii_digit) =>
        {
            let jahr = j
                .iter()
                .fold(0_i64, |acc, b| acc * 10 + i64::from(b - b'0'));
            Value::from(jahr)
        }
        _ => wert.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{cent_nach_kz, kz_format, KzBetrag, KzFormat, ABZUGS_KZ};
    use domain::{Cent, Euro};
    use proptest::prelude::*;

    proptest! {
        /// Rundung „zu Ihren Gunsten": eine Einnahme wird nie hoeher, ein Abzug nie niedriger
        /// deklariert als der Cent-Betrag; beide weichen um weniger als einen Euro ab.
        #[test]
        fn rundung_zugunsten_der_steuerpflichtigen(c in -10_000_000_000_i64..10_000_000_000) {
            let KzBetrag::Euro(ein) = cent_nach_kz(Cent::new(c), "E0200201") else { panic!() };
            prop_assert!(ein.get() * 100 <= c && c - ein.get() * 100 < 100);
            let KzBetrag::Euro(ab) = cent_nach_kz(Cent::new(c), "E0705701") else { panic!() };
            prop_assert!(ab.get() * 100 >= c && ab.get() * 100 - c < 100);
        }
    }

    #[test]
    fn jedes_abzugs_kz_rundet_auf() {
        for kz in ABZUGS_KZ {
            assert_eq!(kz_format(kz), KzFormat::EuroAufgerundet, "{kz}");
            assert_eq!(cent_nach_kz(Cent::new(1), kz), KzBetrag::Euro(Euro::new(1)));
        }
    }
}
