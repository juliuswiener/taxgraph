//! Veranlagungszeitraum. Verschoben aus `catala-sys` (Schritt 1) nach `domain` (Schritt 2,
//! `REWRITE_PLAN.md` §8 F-Frage "wer haengt von wem ab"): `catala-sys` bleibt Blatt und haengt
//! jetzt von `domain` ab, nicht umgekehrt — jede hoehere Schicht (bindung, store, engine, ...)
//! braucht `Vz` ohnehin, aber keine von ihnen braucht die C-FFI aus `catala-sys`.
use std::fmt;

/// Veranlagungszeitraum (assessment period). Ein Wert ausserhalb des unterstuetzten Bereichs
/// wuerde im generierten C (`switch` ohne `default`-Zweig, Audit C Teil 2.3 Schritt F)
/// `abort()`en — dieser Typ macht das Verhalten unrepresentierbar statt es nur an der
/// FFI-Grenze zu pruefen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Vz {
    Vz2024 = 0,
    Vz2025 = 1,
    Vz2026 = 2,
}

/// `jahr` liegt ausserhalb des unterstuetzten Bereichs 2024..=2026.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("kein unterstuetzter Veranlagungszeitraum: {0}")]
pub struct UngueltigeVz(pub u16);

impl Vz {
    /// Das Kalenderjahr als `u16`.
    ///
    /// ```
    /// use domain::Vz;
    /// assert_eq!(Vz::Vz2025.jahr(), 2025);
    /// ```
    #[must_use]
    pub const fn jahr(self) -> u16 {
        match self {
            Self::Vz2024 => 2024,
            Self::Vz2025 => 2025,
            Self::Vz2026 => 2026,
        }
    }
}

impl TryFrom<u16> for Vz {
    type Error = UngueltigeVz;

    /// ```
    /// use domain::Vz;
    /// assert_eq!(Vz::try_from(2025u16), Ok(Vz::Vz2025));
    /// assert!(Vz::try_from(2023u16).is_err());
    /// ```
    fn try_from(jahr: u16) -> Result<Self, Self::Error> {
        match jahr {
            2024 => Ok(Self::Vz2024),
            2025 => Ok(Self::Vz2025),
            2026 => Ok(Self::Vz2026),
            other => Err(UngueltigeVz(other)),
        }
    }
}

impl fmt::Display for Vz {
    /// ```
    /// use domain::Vz;
    /// assert_eq!(Vz::Vz2026.to_string(), "2026");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.jahr())
    }
}

#[cfg(test)]
mod tests {
    use super::Vz;

    #[test]
    fn jede_unterstuetzte_vz_rundtrippt_ueber_das_jahr() {
        for vz in [Vz::Vz2024, Vz::Vz2025, Vz::Vz2026] {
            assert_eq!(Vz::try_from(vz.jahr()), Ok(vz));
        }
    }
}
