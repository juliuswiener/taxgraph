//! ELSTER-Kennzahl: `E` und sieben ASCII-Ziffern.
//!
//! Die Regel steht nur in [`Kz::ist_gueltig`]. Die Pruefstellen `bindung_datei.rs` (Bindungs-Schema
//! `schema.json:262`, `^E[0-9]{7}$`) und `xsd.rs` (`xsd_verify.py:30`, `^E\d{7}$`) rufen sie. Python
//! ist an zwei Raendern weiter, gemessen 3.12.9 und 3.14.7: `$` laesst ein abschliessendes `\n` zu,
//! und `\d` nimmt auch Nicht-ASCII-Ziffern (`E` + sieben arabisch-indische Ziffern). `Kz` bleibt
//! bei der engeren Rust-Regel.
use std::fmt;

use serde::de::{Deserialize, Deserializer, Error};

/// Eine geprueft gueltige Kennzahl, z. B. `E0100401`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Kz(String);

/// Der Text ist keine Kennzahl.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ungueltige Kz {0:?} (erwartet E und sieben Ziffern 0-9)")]
pub struct UngueltigeKz(pub String);

impl Kz {
    /// Validiert und baut eine `Kz`.
    ///
    /// # Errors
    /// [`UngueltigeKz`], wenn `s` nicht `^E[0-9]{7}$` ist.
    ///
    /// ```
    /// use domain::Kz;
    /// assert_eq!(Kz::new("E0100401").unwrap().as_str(), "E0100401");
    /// assert!(Kz::new("E010040").is_err());
    /// assert!(Kz::new("E0100401\n").is_err());
    /// ```
    pub fn new(s: impl Into<String>) -> Result<Self, UngueltigeKz> {
        let s = s.into();
        if Self::ist_gueltig(&s) {
            Ok(Self(s))
        } else {
            Err(UngueltigeKz(s))
        }
    }

    /// Die Kz-Regel `^E[0-9]{7}$`, ohne Allokation.
    ///
    /// ```
    /// use domain::Kz;
    /// assert!(Kz::ist_gueltig("E0100401"));
    /// assert!(!Kz::ist_gueltig("E0100401\n"));
    /// ```
    #[must_use]
    pub fn ist_gueltig(s: &str) -> bool {
        match s.as_bytes() {
            [b'E', ziffern @ ..] => ziffern.len() == 7 && ziffern.iter().all(u8::is_ascii_digit),
            _ => false,
        }
    }

    /// Die Kz beginnt mit `E60`, dem Praefix der EUeR-Kz (`est_mapping.py:236`).
    ///
    /// ```
    /// use domain::Kz;
    /// assert!(Kz::new("E6004901").unwrap().hat_e60_praefix());
    /// assert!(!Kz::new("E0100401").unwrap().hat_e60_praefix());
    /// ```
    #[must_use]
    pub fn hat_e60_praefix(&self) -> bool {
        self.0.starts_with("E60")
    }

    /// Die Kennzahl als `&str`.
    ///
    /// ```
    /// assert_eq!(domain::Kz::new("E6004901").unwrap().as_str(), "E6004901");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Kz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Liest eine Kennzahl aus einem Text und prueft sie mit [`Kz::new`]. Eine ungueltige Kennzahl
/// scheitert beim Laden und nennt den Text.
///
/// ```
/// use domain::Kz;
/// assert_eq!(serde_json::from_str::<Kz>("\"E0100401\"").unwrap().as_str(), "E0100401");
/// assert!(serde_json::from_str::<Kz>("\"E010040\"").is_err());
/// ```
impl<'de> Deserialize<'de> for Kz {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::Kz;

    #[test]
    fn regel_der_rust_pruefstellen() {
        for s in ["E0100401", "E6004901", "E0000000", "E9999999"] {
            assert_eq!(Kz::new(s).unwrap().to_string(), s);
        }
        for s in [
            "",
            "E",
            "E010040",
            "E01004011",
            "e0100401",
            " E0100401",
            "E0100401 ",
            "E0100401\n",
            "E010040x",
            "E\u{660}\u{661}\u{660}\u{660}\u{664}\u{660}\u{661}",
            "\u{ff25}0100401",
        ] {
            assert!(Kz::new(s).is_err(), "{s:?}");
        }
    }

    #[test]
    fn deserialize_prueft_dieselbe_regel() {
        for s in ["E0100401", "E6004901"] {
            let kz: Kz = serde_json::from_value(serde_json::json!(s)).unwrap();
            assert_eq!(kz.as_str(), s);
        }
        for s in ["", "E010040", "E0100401\n", "e0100401"] {
            let fehler = serde_json::from_value::<Kz>(serde_json::json!(s)).unwrap_err();
            assert!(fehler.to_string().contains("ungueltige Kz"), "{s:?}: {fehler}");
        }
        assert!(serde_json::from_value::<Kz>(serde_json::json!(7_654_321)).is_err());
    }
}
