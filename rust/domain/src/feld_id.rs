//! Feld-Ids und die Repeated-Instance-Konvention.
//!
//! Zwei Python-Module halten dieselbe Konvention:
//! `produkt/traverser/traverser.py:144-152` (`instanz_feld_id`) erzeugt sie — Instanz 1 ist die
//! Basis-`feld_id` OHNE Suffix, Instanz `n >= 2` traegt `basis__n`. `produkt/mapping/est_mapping.py:702`
//! (`parse_instanz`, Regex `^(?P<base>[a-z][a-z0-9_]*)__(?P<idx>[2-9]|[1-9][0-9]+)$`) *liest* sie.
//! Bis 2026-10-03 las `parse_instanz` auch `basis__1`, obwohl der Traverser diese Form nie erzeugt; die
//! Schreib-Route weist `basis__1` seitdem ab (vault `die-schreib-route-weist-eine-kennung-mit-instanz-eins-ab`),
//! und beide Seiten lesen es nicht mehr als Instanz. `FromStr` weist `basis__1` als ungueltige
//! Instanz-Kodierung zurueck, damit `Display`/`FromStr` ein echtes Roundtrip-Paar bleiben (jede
//! `FeldId` hat genau EINE String-Darstellung).
use std::fmt;
use std::num::NonZeroU16;
use std::str::FromStr;

/// Prueft die Schema-Regel fuer einen Feldnamen, `^[a-z][a-z0-9_]*$` (`pattern` in
/// `produkt/store/schema.json` und `produkt/bindung/schema.json`), ohne Regex-Abhaengigkeit.
///
/// Das ist die EINE Regel fuer einen gueltigen Namen (Entscheidung
/// feld-kennung-folgt-der-schema-regel-und-instanz-eins-bleibt-gepinnt, Punkt 1): `bindung` und
/// [`BasisId::new`] rufen diese Funktion. Eine Laengengrenze gehoert nicht dazu; die 64 des
/// URL-Musters (`server.py` `_FID`, `rust/api/src/routen.rs`) gelten nur im Router.
///
/// Abweichung von Python, bekannt: Pythons `$` passt auch vor einem abschliessenden `\n`, die
/// Pruefung des Schemas (`jsonschema`, `re.search`) nimmt also `"x\n"` an. Hier nicht; fail-closed.
///
/// ```
/// assert!(domain::ist_gueltige_feld_id("vv_einnahmen"));
/// assert!(!domain::ist_gueltige_feld_id("Vv_einnahmen"));
/// assert!(!domain::ist_gueltige_feld_id("2vv"));
/// ```
#[must_use]
pub fn ist_gueltige_feld_id(s: &str) -> bool {
    let mut bytes = s.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Eine unveraenderliche Basis-Feld-Kennung ohne Instanz-Suffix (z. B. `vv_einnahmen`).
///
/// Validiert die Schema-Regel [`ist_gueltige_feld_id`] (`^[a-z][a-z0-9_]*$`) OHNE einen
/// `__<Zahl>`-Suffix — ein solcher Suffix gehoert zu [`FeldId::instanz`], nicht zur Basis selbst.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BasisId(String);

/// `FeldId::from_str`/`BasisId::new` sind an einer der beiden Regeln gescheitert.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeldIdFehler {
    /// Weder die Schema-Regel `^[a-z][a-z0-9_]*$` noch "kein `__<Zahl>`-Suffix" erfuellt.
    #[error("ungueltige Basis-Feld-Id {0:?} (erwartet ^[a-z][a-z0-9_]*$ ohne __<Zahl>-Suffix)")]
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
    /// [`FeldIdFehler::UngueltigeBasis`], wenn `s` die Schema-Regel [`ist_gueltige_feld_id`]
    /// verfehlt (Grossbuchstabe oder Ziffer vorn, anderes Zeichen, leer) oder auf einen
    /// `__<Zahl>`-Suffix endet.
    ///
    /// ```
    /// use domain::BasisId;
    /// assert!(BasisId::new("vv_einnahmen").is_ok());
    /// assert!(BasisId::new("a".repeat(65)).is_ok());
    /// assert!(BasisId::new("vv_einnahmen__2").is_err());
    /// assert!(BasisId::new("Vv_einnahmen").is_err());
    /// assert!(BasisId::new("2vv").is_err());
    /// ```
    pub fn new(s: impl Into<String>) -> Result<Self, FeldIdFehler> {
        let s = s.into();
        if !ist_gueltige_feld_id(&s) || instanz_suffix(&s).is_some() {
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
        Self {
            basis,
            instanz: NonZeroU16::MIN,
        }
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
    /// gueltige Instanz-Kodierung, so wenig wie fuer `est_mapping.parse_instanz`.
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
                let instanz = NonZeroU16::new(n)
                    .ok_or_else(|| FeldIdFehler::UngueltigeInstanz(s.to_owned()))?;
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
        "[a-z][a-z0-9_]{0,20}"
            .prop_filter("kein __<Zahl>-Suffix", |s| BasisId::new(s.clone()).is_ok())
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
    fn basis_1_ist_keine_instanz() {
        // Wie est_mapping.parse_instanz("x__1"), das seit 2026-10-03 None liefert: FeldId weist die
        // Zeichenkette zurueck.
        assert!("x__1".parse::<FeldId>().is_err());
    }

    /// Die Schema-Regel `^[a-z][a-z0-9_]*$` (`produkt/{store,bindung}/schema.json`) ist die eine Regel fuer
    /// einen Feldnamen (Entscheidung feld-kennung-folgt-der-schema-regel-und-instanz-eins-bleibt-gepinnt):
    /// ein Grossbuchstabe oder eine Ziffer vorn ist kein Name, eine Laengengrenze gibt es nicht (die 64 des
    /// URL-Musters gehoeren dem Router).
    #[test]
    fn basis_id_nimmt_genau_die_schema_regel_an() {
        for gut in ["vv_einnahmen", "a", "a1", "a_", "a__b", "z9_9"] {
            assert!(BasisId::new(gut).is_ok(), "{gut:?} muss gelten");
        }
        for lang in [64, 65, 200] {
            assert!(
                BasisId::new("a".repeat(lang)).is_ok(),
                "{lang} x a muss gelten"
            );
        }
        for schlecht in [
            "Vv_einnahmen",
            "vV_einnahmen",
            "2vv",
            "_vv",
            "",
            "vv-einnahmen",
            "vv einnahmen",
            "vv_einnahmen\n",
            "vv\u{e4}",
            "vv_einnahmen__2",
            "vv_einnahmen__1",
            "vv_einnahmen__0",
            "vv_einnahmen__02",
        ] {
            assert!(
                BasisId::new(schlecht).is_err(),
                "{schlecht:?} muss scheitern"
            );
        }
    }

    #[test]
    fn feld_id_folgt_der_basis_regel() {
        assert!("a".repeat(65).parse::<FeldId>().is_ok());
        assert!(format!("{}__2", "a".repeat(65)).parse::<FeldId>().is_ok());
        for schlecht in ["Vv_einnahmen", "2vv", "Vv_einnahmen__2", "2vv__2"] {
            assert!(
                schlecht.parse::<FeldId>().is_err(),
                "{schlecht:?} muss scheitern"
            );
        }
    }

    #[test]
    fn die_fehlermeldung_nennt_keine_laengengrenze() {
        let text = BasisId::new("Vv_einnahmen").unwrap_err().to_string();
        assert!(!text.contains("64"), "{text}");
        assert!(text.contains("^[a-z][a-z0-9_]*$"), "{text}");
    }
}
