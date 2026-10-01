//! Vorschlags-Typ fuer den Feld-Katalog (Auflage K1): welcher Kanal ein Feld vorschlagen darf
//! (`store.py:111-118`, `_vorschlag_typ`; `store.py:154`, `lade_katalog`). Genau vier Typen; ein
//! unbekannter Typ ist kein Typ (`Katalog::erlaubt`: fail-closed `false`).
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Ein Vorschlags-Kanal, in der Reihenfolge von `lade_katalog`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VorschlagTyp {
    /// Schreiber `llm:*`.
    Llm,
    /// Schreiber `import:beleg`.
    Beleg,
    /// Schreiber `import:kontoauszug`.
    Kontoauszug,
    /// Schreiber `berechnet:*`.
    Maps,
}

/// Der Text ist keiner der vier Vorschlags-Typen.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unbekannter Vorschlags-Typ {0:?} (erwartet llm, beleg, kontoauszug oder maps)")]
pub struct UnbekannterVorschlagTyp(pub String);

impl VorschlagTyp {
    /// Der Katalog-Schluessel.
    ///
    /// ```
    /// assert_eq!(domain::VorschlagTyp::Kontoauszug.als_str(), "kontoauszug");
    /// ```
    #[must_use]
    pub const fn als_str(self) -> &'static str {
        match self {
            Self::Llm => "llm",
            Self::Beleg => "beleg",
            Self::Kontoauszug => "kontoauszug",
            Self::Maps => "maps",
        }
    }
}

impl FromStr for VorschlagTyp {
    type Err = UnbekannterVorschlagTyp;

    /// ```
    /// use domain::VorschlagTyp;
    /// assert_eq!("maps".parse::<VorschlagTyp>().unwrap(), VorschlagTyp::Maps);
    /// assert!("Maps".parse::<VorschlagTyp>().is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "llm" => Ok(Self::Llm),
            "beleg" => Ok(Self::Beleg),
            "kontoauszug" => Ok(Self::Kontoauszug),
            "maps" => Ok(Self::Maps),
            andere => Err(UnbekannterVorschlagTyp(andere.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VorschlagTyp;

    #[test]
    fn vier_typen_rundtrippen() {
        for t in [
            VorschlagTyp::Llm,
            VorschlagTyp::Beleg,
            VorschlagTyp::Kontoauszug,
            VorschlagTyp::Maps,
        ] {
            assert_eq!(t.als_str().parse::<VorschlagTyp>().unwrap(), t);
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, format!("\"{}\"", t.als_str()));
            assert_eq!(serde_json::from_str::<VorschlagTyp>(&json).unwrap(), t);
        }
        for s in ["", "LLM", "berechnet", "vorjahr", "elster", " llm"] {
            assert!(s.parse::<VorschlagTyp>().is_err(), "{s:?}");
            assert!(serde_json::from_str::<VorschlagTyp>(&format!("{s:?}")).is_err());
        }
    }
}
