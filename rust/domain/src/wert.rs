//! Der gespeicherte Wert eines Feldes, typgeprueft gegen den Bindungstyp (Auflage T,
//! `store.py:164-231`, "Stille-Null-Klasse": ein `String "50000"` auf einem `typ=cent`-Feld
//! wurde ohne Fehler bestaetigt, und ueber 15 Lesestellen in `api.py` machten daraus
//! kommentarlos `0`). Ein `Wert` kann in Rust gar nicht erst entstehen, wenn er nicht zum
//! deklarierten [`Feldtyp`] passt — die Pruefung ist damit eine Typ-Eigenschaft, keine
//! Laufzeit-Prüfung mehr, die man vergessen kann.
use serde::{Deserialize, Serialize};

/// Die Bindungs-Typmenge (`store.py:164`: `_TYP_ORD`), in derselben Reihenfolge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Feldtyp {
    Cent,
    Int,
    Bool,
    Enum,
    Datum,
    Text,
}

/// Ein typgeprueft gespeicherter Feldwert.
#[derive(Debug, Clone, PartialEq)]
pub enum Wert {
    /// Geldbetrag in Cent (Ganzzahl).
    Cent(i64),
    /// Zaehl-/Stueckzahl.
    Int(i64),
    Bool(bool),
    /// Einer der `enum_werte` der Bindung.
    Enum(String),
    /// `TT.MM.JJJJ`, NICHT ISO-8601 (amtliches ELSTER-Format, `store.py:180-187`).
    Datum(String),
    Text(String),
}

/// `Wert`-Konstruktion ist an der Typ-Konformitaet gescheitert (Auflage T).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WertFehler {
    #[error("Wert {wert} passt nicht zum Bindungstyp '{typ}'")]
    TypInkonform { wert: String, typ: &'static str },
    #[error("Enum-Wert {0:?} ist keiner der zulaessigen enum_werte")]
    UnbekannterEnumWert(String),
    #[error("Datum {0:?} entspricht nicht dem Format TT.MM.JJJJ")]
    UngueltigesDatum(String),
}

impl Feldtyp {
    #[must_use]
    pub const fn als_str(self) -> &'static str {
        match self {
            Self::Cent => "cent",
            Self::Int => "int",
            Self::Bool => "bool",
            Self::Enum => "enum",
            Self::Datum => "datum",
            Self::Text => "text",
        }
    }
}

/// `TT.MM.JJJJ`: genau 10 ASCII-Zeichen, zwei Ziffern, Punkt, zwei Ziffern, Punkt, vier Ziffern
/// (`store.py:187`: `re.match(r"^\d{2}\.\d{2}\.\d{4}$", wert)`). Prueft nur das Format, nicht die
/// Kalender-Gueltigkeit (der Python-Regex tut das auch nicht). Slice-Pattern statt Indexierung,
/// damit `clippy::indexing_slicing` nicht greift.
fn ist_tt_mm_jjjj(s: &str) -> bool {
    match s.as_bytes() {
        [t0, t1, b'.', m0, m1, b'.', j0, j1, j2, j3] => [t0, t1, m0, m1, j0, j1, j2, j3]
            .iter()
            .all(|b| b.is_ascii_digit()),
        _ => false,
    }
}

impl Wert {
    /// Parst einen JSON-Wert gegen einen Bindungstyp (Auflage T). `enum_werte` ist nur bei
    /// `typ == Enum` relevant.
    ///
    /// # Errors
    /// [`WertFehler`], wenn der JSON-Wert nicht zum deklarierten Typ passt.
    ///
    /// ```
    /// use domain::{Feldtyp, Wert};
    /// let v = Wert::aus_json(&serde_json::json!(1500), Feldtyp::Cent, None).unwrap();
    /// assert_eq!(v, Wert::Cent(1500));
    /// assert!(Wert::aus_json(&serde_json::json!("1500"), Feldtyp::Cent, None).is_err());
    /// assert!(Wert::aus_json(&serde_json::json!(true), Feldtyp::Cent, None).is_err());
    /// ```
    pub fn aus_json(
        wert: &serde_json::Value,
        typ: Feldtyp,
        enum_werte: Option<&[String]>,
    ) -> Result<Self, WertFehler> {
        let inkonform = || WertFehler::TypInkonform {
            wert: wert.to_string(),
            typ: typ.als_str(),
        };
        match typ {
            Feldtyp::Cent | Feldtyp::Int => {
                // `bool` ist in JSON (anders als in Python) ein eigener `Value`-Fall -- kein
                // expliziter Bool-Ausschluss noetig, `as_i64` liefert fuer `Value::Bool` `None`.
                let n = wert.as_i64().ok_or_else(inkonform)?;
                Ok(if matches!(typ, Feldtyp::Cent) {
                    Self::Cent(n)
                } else {
                    Self::Int(n)
                })
            }
            Feldtyp::Bool => wert.as_bool().map(Self::Bool).ok_or_else(inkonform),
            Feldtyp::Enum => {
                let s = wert.as_str().ok_or_else(inkonform)?;
                if enum_werte.is_some_and(|ws| ws.iter().any(|w| w == s)) {
                    Ok(Self::Enum(s.to_owned()))
                } else {
                    Err(WertFehler::UnbekannterEnumWert(s.to_owned()))
                }
            }
            Feldtyp::Datum => {
                let s = wert.as_str().ok_or_else(inkonform)?;
                if ist_tt_mm_jjjj(s) {
                    Ok(Self::Datum(s.to_owned()))
                } else {
                    Err(WertFehler::UngueltigesDatum(s.to_owned()))
                }
            }
            Feldtyp::Text => wert
                .as_str()
                .map(|s| Self::Text(s.to_owned()))
                .ok_or_else(inkonform),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Feldtyp, Wert};
    use serde_json::json;

    #[test]
    fn bool_wird_nicht_als_cent_durchgelassen() {
        // Die Python-Falle (`isinstance(True, int)`) existiert in JSON nicht, aber der Test
        // haelt die Parity trotzdem fest: `true`/`false` sind nie ein gueltiger cent/int-Wert.
        assert!(Wert::aus_json(&json!(true), Feldtyp::Cent, None).is_err());
        assert!(Wert::aus_json(&json!(false), Feldtyp::Int, None).is_err());
    }

    #[test]
    fn enum_akzeptiert_nur_gelistete_werte() {
        let werte = vec!["ja".to_string(), "nein".to_string()];
        assert_eq!(
            Wert::aus_json(&json!("ja"), Feldtyp::Enum, Some(&werte)).unwrap(),
            Wert::Enum("ja".to_string())
        );
        assert!(Wert::aus_json(&json!("vielleicht"), Feldtyp::Enum, Some(&werte)).is_err());
    }

    #[test]
    fn datum_verlangt_tt_mm_jjjj_nicht_iso() {
        assert!(Wert::aus_json(&json!("05.05.1955"), Feldtyp::Datum, None).is_ok());
        assert!(Wert::aus_json(&json!("1955-05-05"), Feldtyp::Datum, None).is_err());
    }
}
