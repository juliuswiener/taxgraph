//! ELSTER-Kennzahl: `E` und sieben ASCII-Ziffern.
//!
//! Die Regel steht nur in [`Kz::ist_gueltig`]. Die Pruefstellen `bindung_datei.rs` (Bindungs-Schema
//! `schema.json:262`, `^E[0-9]{7}$`) und `xsd.rs` (`xsd_verify.py:30`, `^E\d{7}$`) rufen sie. Python
//! ist an zwei Raendern weiter, gemessen 3.12.9 und 3.14.7: `$` laesst ein abschliessendes `\n` zu,
//! und `\d` nimmt auch Nicht-ASCII-Ziffern (`E` + sieben arabisch-indische Ziffern). `Kz` bleibt
//! bei der engeren Rust-Regel.
use std::fmt;

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

    /// Text beginnt mit `E60`, dem Praefix der EUeR-Kz (`est_mapping.py:236`). Prueft nur den
    /// Anfang, nicht die Kz-Regel.
    ///
    /// ```
    /// use domain::Kz;
    /// assert!(Kz::hat_e60_praefix("E6004901"));
    /// assert!(!Kz::hat_e60_praefix("E0100401"));
    /// assert!(Kz::hat_e60_praefix("E60"));
    /// ```
    #[must_use]
    pub fn hat_e60_praefix(s: &str) -> bool {
        s.starts_with("E60")
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
}
