//! Feld-Ids und die Repeated-Instance-Konvention.
//!
//! Zwei Python-Module schreiben dieselbe Konvention leicht verschieden nach:
//! `produkt/traverser/traverser.py:144-152` (`instanz_feld_id`) erzeugt sie — Instanz 1 ist die
//! Basis-`feld_id` OHNE Suffix, Instanz `n >= 2` traegt `basis__n`. `produkt/mapping/est_mapping.py:543-552`
//! (`parse_instanz`, Regex `^(?P<base>[a-z][a-z0-9_]*)__(?P<idx>[1-9][0-9]*)$`) *liest* sie und
//! akzeptiert dabei auch `basis__1`, obwohl der Traverser diese Form nie erzeugt. `FeldId` folgt
//! hier der ERZEUGENDEN Seite (Traverser): `FromStr` weist `basis__1` als ungueltige
//! Instanz-Kodierung zurueck, damit `Display`/`FromStr` ein echtes Roundtrip-Paar bleiben (jede
//! `FeldId` hat genau EINE String-Darstellung). Ein Aufrufer, der `est_mapping`-kompatibel auch
//! `basis__1` lesen muss, tut das VOR dem Parsen in `FeldId` (ponytail: kein zweiter Parse-Pfad
//! in dieser Crate, bis ein Rust-Aufrufer diese Divergenz tatsaechlich braucht).
use std::fmt;
use std::num::NonZeroU16;
use std::str::FromStr;

/// Eine unveraenderliche Basis-Feld-Kennung ohne Instanz-Suffix (z. B. `vv_einnahmen`).
///
/// Validiert `[A-Za-z0-9_]{1,64}` OHNE einen `__<Zahl>`-Suffix — ein solcher Suffix gehoert zu
/// [`FeldId::instanz`], nicht zur Basis selbst.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BasisId(String);

/// `FeldId::from_str`/`BasisId::new` sind an einer der beiden Regeln gescheitert.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeldIdFehler {
    /// Weder Zeichensatz `[A-Za-z0-9_]{1,64}` noch "kein `__<Zahl>`-Suffix" erfuellt.
    #[error("ungueltige Basis-Feld-Id {0:?} (erwartet [A-Za-z0-9_]{{1,64}} ohne __<Zahl>-Suffix)")]
    UngueltigeBasis(String),
    /// Ein `__`-Suffix ist vorhanden, kodiert aber keine gueltige Instanz (`n >= 2`, keine
    /// fuehrende Null). Das schliesst insbesondere `basis__1` ein (siehe Modul-Dokumentation).
    #[error(
        "ungueltige Instanz-Kodierung in {0:?} (erwartet __<n> mit n >= 2, keine fuehrende Null)"
    )]
    UngueltigeInstanz(String),
}

/// Findet einen `__<Ziffern>`-Suffix. `None`, wenn keiner vorliegt.
fn instanz_suffix(s: &str) -> Option<(&str, &str)> {
    let pos = s.rfind("__")?;
    let (basis, ziffern) = (&s[..pos], &s[pos + 2..]);
    if basis.is_empty() || ziffern.is_empty() || !ziffern.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((basis, ziffern))
}

impl BasisId {
    /// Validiert und baut eine `BasisId`.
    ///
    /// # Errors
    /// [`FeldIdFehler::UngueltigeBasis`] bei falschem Zeichensatz, falscher Laenge oder einem
    /// `__<Zahl>`-Suffix.
    ///
    /// ```
    /// use domain::BasisId;
    /// assert!(BasisId::new("vv_einnahmen").is_ok());
    /// assert!(BasisId::new("vv_einnahmen__2").is_err());
    /// ```
    pub fn new(s: impl Into<String>) -> Result<Self, FeldIdFehler> {
        let s = s.into();
        let gueltiger_zeichensatz =
            !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !gueltiger_zeichensatz || instanz_suffix(&s).is_some() {
            return Err(FeldIdFehler::UngueltigeBasis(s));
        }
        Ok(Self(s))
    }

    /// Die Basis als `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BasisId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Eine Feld-Id inklusive Instanz: `basis` fuer Instanz 1, `basis__n` (n >= 2) fuer weitere
/// Instanzen einer `instanz_gruppe` (mehrere Kinder, mehrere Vermietungsobjekte, ...).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FeldId {
    basis: BasisId,
    instanz: NonZeroU16,
}

impl FeldId {
    /// Instanz 1 einer Basis-Feld-Id.
    ///
    /// ```
    /// use domain::{BasisId, FeldId};
    /// let f = FeldId::erste_instanz(BasisId::new("vv_einnahmen").unwrap());
    /// assert_eq!(f.to_string(), "vv_einnahmen");
    /// ```
    #[must_use]
    pub const fn erste_instanz(basis: BasisId) -> Self {
        Self { basis, instanz: NonZeroU16::MIN }
    }

    /// Instanz `n` (n >= 2) einer Basis-Feld-Id.
    ///
    /// # Errors
    /// [`FeldIdFehler::UngueltigeInstanz`], wenn `n < 2`.
    pub fn instanz(basis: BasisId, n: NonZeroU16) -> Result<Self, FeldIdFehler> {
        if n.get() < 2 {
            return Err(FeldIdFehler::UngueltigeInstanz(format!("{basis}__{n}")));
        }
        Ok(Self { basis, instanz: n })
    }

    /// Die Basis ohne Instanz-Suffix.
    #[must_use]
    pub fn basis(&self) -> &BasisId {
        &self.basis
    }

    /// Die Instanznummer (1 fuer die Basis selbst).
    #[must_use]
    pub const fn instanznummer(&self) -> NonZeroU16 {
        self.instanz
    }
}

impl fmt::Display for FeldId {
    /// ```
    /// use domain::{BasisId, FeldId};
    /// use std::num::NonZeroU16;
    /// let f = FeldId::instanz(BasisId::new("vv_einnahmen").unwrap(), NonZeroU16::new(2).unwrap()).unwrap();
    /// assert_eq!(f.to_string(), "vv_einnahmen__2");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.instanz.get() == 1 {
            write!(f, "{}", self.basis)
        } else {
            write!(f, "{}__{}", self.basis, self.instanz)
        }
    }
}

impl FromStr for FeldId {
    type Err = FeldIdFehler;

    /// Parst nach der Traverser-Konvention (siehe Modul-Dokumentation): `basis__1` ist KEINE
    /// gueltige Instanz-Kodierung, auch wenn `est_mapping.parse_instanz` sie akzeptieren wuerde.
    ///
    /// ```
    /// use domain::FeldId;
    /// let f: FeldId = "vv_einnahmen__2".parse().unwrap();
    /// assert_eq!(f.to_string(), "vv_einnahmen__2");
    /// assert!("vv_einnahmen__1".parse::<FeldId>().is_err());
    /// assert!("vv_einnahmen__0".parse::<FeldId>().is_err());
    /// assert!("vv_einnahmen__02".parse::<FeldId>().is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match instanz_suffix(s) {
            Some((basis, ziffern)) => {
                let hat_fuehrende_null = ziffern.starts_with('0');
                let n: u16 = ziffern
                    .parse()
                    .map_err(|_| FeldIdFehler::UngueltigeInstanz(s.to_owned()))?;
                if hat_fuehrende_null || n < 2 {
                    return Err(FeldIdFehler::UngueltigeInstanz(s.to_owned()));
                }
                let instanz = NonZeroU16::new(n).ok_or_else(|| FeldIdFehler::UngueltigeInstanz(s.to_owned()))?;
                Self::instanz(BasisId::new(basis)?, instanz)
            }
            None => Ok(Self::erste_instanz(BasisId::new(s)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BasisId, FeldId};
    use proptest::prelude::*;

    fn gueltige_basis() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,20}".prop_filter("kein __<Zahl>-Suffix", |s| {
            BasisId::new(s.clone()).is_ok()
        })
    }

    proptest! {
        /// Display/FromStr sind fuer jede gueltige (Basis, Instanz)-Kombination ein Roundtrip.
        #[test]
        fn display_from_str_roundtrip(basis in gueltige_basis(), n in 1u16..500) {
            let basis_id = BasisId::new(basis).map_err(|e| TestCaseError::fail(e.to_string()))?;
            let feld = if n == 1 {
                FeldId::erste_instanz(basis_id)
            } else {
                let nz = std::num::NonZeroU16::new(n).ok_or_else(|| TestCaseError::fail("n==0"))?;
                FeldId::instanz(basis_id, nz).map_err(|e| TestCaseError::fail(e.to_string()))?
            };
            let s = feld.to_string();
            let zurueck: FeldId = s.parse().map_err(|e: super::FeldIdFehler| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(zurueck, feld);
        }
    }

    #[test]
    fn basis_ohne_suffix_ist_instanz_eins() {
        let f: FeldId = "vv_einnahmen".parse().unwrap();
        assert_eq!(f.instanznummer().get(), 1);
    }

    #[test]
    fn est_mapping_akzeptiert_basis_1_traverser_nicht() {
        // Dokumentiert die Divergenz aus der Modul-Doc: est_mapping.parse_instanz("x__1") liefert
        // ("x", 1); FeldId weist dieselbe Zeichenkette zurueck.
        assert!("x__1".parse::<FeldId>().is_err());
    }
}
