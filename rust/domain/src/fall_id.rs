//! Fall-Kennung: `[A-Za-z0-9_-]{1,64}` als Ganzes (`api_constants.py:48`, `_FALL_RE`, geprueft
//! mit `fullmatch` in `api.py:133`, `api.py:292`, `api.py:937` und `fehler_log.py:123`).
//!
//! Dieselbe Regel wie die heutigen Rust-Pruefstellen `eigener_fall.rs:27` (`ist_fall_id`) und
//! `fehler_log.rs:94-98`. Namensgleich, aber ein anderer Begriff: `store::fehler_log::FallId` haelt
//! auch Sperr-Markierungen wie `<gesperrt:form>`, die hier ungueltig sind.
use std::fmt;

/// Eine geprueft gueltige Fall-Kennung.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FallId(String);

/// Der Text ist keine Fall-Kennung. Die Meldung nennt den Text nicht: eine abgewiesene Kennung
/// kann alles sein, auch eine eingefuegte IBAN (vgl. `fehler_log.rs`, `<gesperrt:form>`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ungueltige fall_id (nur [A-Za-z0-9_-]{{1,64}})")]
pub struct UngueltigeFallId(pub String);

impl FallId {
    /// Validiert und baut eine `FallId`.
    ///
    /// # Errors
    /// [`UngueltigeFallId`], wenn `s` nicht ganz aus 1 bis 64 Zeichen `[A-Za-z0-9_-]` besteht.
    ///
    /// ```
    /// use domain::FallId;
    /// assert_eq!(FallId::new("demo-1").unwrap().as_str(), "demo-1");
    /// assert!(FallId::new("a b").is_err());
    /// ```
    pub fn new(s: impl Into<String>) -> Result<Self, UngueltigeFallId> {
        let s = s.into();
        let gueltig = (1..=64).contains(&s.len())
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
        if gueltig {
            Ok(Self(s))
        } else {
            Err(UngueltigeFallId(s))
        }
    }

    /// Die Kennung als `&str`.
    ///
    /// ```
    /// assert_eq!(domain::FallId::new("f1").unwrap().as_str(), "f1");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FallId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::FallId;

    /// `_FALL_RE.fullmatch`, gemessen 3.12.9 und 3.14.7: `True` fuer `demo-1` und 64 Zeichen,
    /// `False` fuer 65 Zeichen, `""`, `"a b"`, `"a\n"`, `"ä"`, `"x.y"`.
    #[test]
    fn wie_fall_re_fullmatch() {
        for s in ["demo-1", "A_z-09", &"a".repeat(64)] {
            assert_eq!(FallId::new(s).unwrap().to_string(), s);
        }
        for s in [
            "",
            "a b",
            "a\n",
            "\u{e4}",
            "x.y",
            "<gesperrt:form>",
            &"a".repeat(65),
        ] {
            assert!(FallId::new(s).is_err(), "{s:?}");
        }
    }

    #[test]
    fn meldung_ohne_kennung() {
        let fehler = FallId::new("DE00 1234").unwrap_err();
        assert!(!fehler.to_string().contains("1234"));
    }
}
