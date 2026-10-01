//! Kz-Format: wie ein Store-Wert in eine ELSTER-Kennzahl geschrieben wird
//! (`est_mapping.py:31-241`, `_cent_nach_kz`, `_kz_wert`, `_schreibe_kz`, `_jahr_aus_kz_wert`).
//!
//! Rundung „zu Ihren Gunsten" (Anleitung ESt 1 A 2025, `anl_est1a_2025.txt:269-274`):
//! Einnahmen/Einkuenfte werden abgerundet, Abzuege/Aufwendungen/Verluste aufgerundet. Kz vom
//! XSD-Typ Dezimal mit zwei Nachkommastellen (`E60…` und die Liste [`KOMMA_OHNE_E60_KZ`]) werden
//! nicht gerundet, sondern exakt als `"N,NN"` geschrieben.

use std::collections::BTreeMap;

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

/// Kz, deren XSD-Typ eine 0 verbietet (`est_mapping.py:145-172`): `GanzzahlPos*` (Basis
/// `xs:positiveInteger`) oder eine Facette ohne `"0"` (`GdB` E0109708/E0505809: Muster
/// `20|25|…|100`; Pflegegrad E0161606: Aufzaehlung 2/3/4). ERiC lehnt eine 0 dort ab, und die
/// ganze Erklaerung ist uneinreichbar. In jeder dieser Kz heisst 0 „nichts anzugeben"
/// (`minOccurs 0`); `schreibe_kz` laesst sie weg (Vault:
/// `decisions/elster-null-in-kz-ohne-null-weglassen`).
///
/// Aus dem XSD abgeleitet ([`crate::xsd::kz_meta`] gegen `E10-2025.xsd`): jede cent-/int-Kz auf
/// einem Schreibweg von `deklariere`, deren Typ die 0 verbietet; Reihenfolge wie in Python (1:1,
/// Instanz und Person B; Spenden; die neun §35c-Kz der Art-Verzweigung). Kz mit Textwert fehlen:
/// `"0,00"` ist gueltig, ein Jahr 0 (`"01.01.0000"`) nicht (`bereich` beim Speichern ungeprueft).
///
/// ponytail: eingefroren am 2026-10-01, jahresunabhaengig, damit `deklariere` ohne XSD laeuft.
/// Grenze: `E10-2024.xsd` verbietet die 0 zusaetzlich in E0106603 (Anzahl weiterer
/// Pflegepersonen); 2025 ist sie dort erlaubt und laut XSD-Label einzutragen. Upgrade: eine Menge
/// je VZ. Drift faengt `null_bleibt_aus_kz_deren_xsd_typ_sie_verbietet` (`tests/eigenschaften.rs`)
/// und fuer die Art-Verzweigung `art_verzweigung_schreibt_keine_verbotene_null`
/// (`deklaration.rs`); beide leiten die Menge live aus dem XSD 2025 ab.
pub const NULL_UNZULAESSIG_KZ: &[&str] = &[
    "E0108202", "E0108002", "E0207611", "E0203503", "E0203504", "E0801705", "E0107207", "E0107208",
    "E0111215", "E0111214", "E0104109", "E0104108", "E0506104", "E0505809", "E0506105", "E0305201",
    "E0240802", "E0242001", "E0240801", "E0241901", "E0109708", "E0161606", "E0205409", "E0205302",
    "E0205201", "E0108105", "E0241001", "E0241101", "E0241201", "E0241301", "E0241302", "E0241401",
    "E0241501", "E0241601", "E0241701",
];

/// Datums-Kz, in die ein Jahres-Wert als `01.01.JJJJ` geschrieben wird (`est_mapping.py:199`).
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

/// Das Format einer Kz (`_cent_nach_kz`-Weiche, `est_mapping.py:179-183`).
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

/// Store-Cent → Kz-Betrag (`_cent_nach_kz`, `est_mapping.py:175-183`).
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

/// Store-Wert → Kz-Wert (`_kz_wert`, `est_mapping.py:216-228`): `cent` gerundet, Jahr in einer
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

/// Die eine Schreibstelle fuer Kz-Werte in `deklariere` (`_schreibe_kz`, `est_mapping.py:231-241`):
/// `ziel[kz] = kz_wert(…)`, ausser der Kz-Wert ist 0 und die Kz verbietet die 0
/// ([`NULL_UNZULAESSIG_KZ`]). Erst umrechnen, dann pruefen: 1-99 Cent werden in einer
/// abgerundeten Kz zur 0 und entfallen ebenso.
///
/// Kein `nicht_deklariert`-Eintrag: eine 0 ist kein verlorener Wert, sondern „nichts anzugeben".
///
/// # Errors
/// Wie [`kz_wert`], dazu `OverflowError`/`ValueError`, wenn der Store-Wert nicht nach JSON geht
/// (NaN/+-inf; im Bestand gemessen 0 von 11294 Events).
pub(crate) fn schreibe_kz(
    ziel: &mut BTreeMap<String, Value>,
    kz: &str,
    wert: &domain::PyWert,
    typ: Option<Feldtyp>,
) -> Result<(), PyFehler> {
    // DIE Grenze Store -> Deklaration: einmal konvertieren, danach laeuft die unveraenderte
    // Value-Kette (`kz_wert`, `gleich_null`) -- die Deklaration IST JSON.
    let wert = wert.zu_json().map_err(PyFehler::from)?;
    let v = kz_wert(&wert, kz, typ)?;
    if NULL_UNZULAESSIG_KZ.contains(&kz) && py::gleich_null(&v) {
        return Ok(());
    }
    ziel.insert(kz.to_owned(), v);
    Ok(())
}

/// Umkehrung zu [`kz_wert`] fuer Datums-Kz (`_jahr_aus_kz_wert`, `est_mapping.py:201-212`):
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
