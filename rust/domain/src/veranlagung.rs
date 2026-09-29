//! Veranlagungsform, Person und Scheibe (`produkt/haut/api_constants.py`).
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Einzel- oder Zusammenveranlagung (Ehe-/Lebenspartner).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Veranlagung {
    Einzel,
    Zusammen,
}

/// Die veranlagte Person: A (Hauptperson) oder B (Ehe-/Lebenspartner bei Zusammenveranlagung).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Person {
    A,
    B,
}

/// Eine Scheibe: ein eigenstaendiger Rechen-/Fragebogen-Ausschnitt mit eigenem Feld-Kegel
/// (`produkt/haut/api_constants.py: SCHEIBEN`). Fuenf Scheiben existieren heute; eine neue
/// Scheibe braucht ohnehin neuen Python-Code (Kegel, `_bescheid_fn`-Zweig, Bindungs-YAML) — ein
/// geschlossenes `enum` ist hier anders als bei [`super::Achsenwert`] angemessen, weil die Menge
/// nicht durch YAML-Daten, sondern durch `SCHEIBEN` im Python-Quellcode selbst festgelegt ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scheibe {
    /// § 19 Einkuenfte aus nichtselbststaendiger Arbeit (Einzel-Posten-Ring).
    Ep,
    /// § 7 AfA-Ring ohne GWG-Sofortabzug.
    NVorGwg,
    /// Arbeitnehmer-Gesamtbescheid (`festzusetzende_est`).
    AnGesamt,
    /// Vollstaendiger Gesamtbescheid ueber alle Einkunftsarten (`festzusetzende_est_gesamt`).
    Gesamt,
    /// Gesamtbescheid fuer Rentner (`festzusetzende_est_rentner`).
    RentnerGesamt,
}

/// Ein Scheibe-String, der zu keiner der fuenf bekannten `SCHEIBEN`-Eintraege passt.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unbekannte Scheibe {0:?} (erwartet ep, n_vor_gwg, an_gesamt, gesamt oder rentner_gesamt)")]
pub struct UnbekannteScheibe(pub String);

impl Scheibe {
    #[must_use]
    pub const fn als_str(self) -> &'static str {
        match self {
            Self::Ep => "ep",
            Self::NVorGwg => "n_vor_gwg",
            Self::AnGesamt => "an_gesamt",
            Self::Gesamt => "gesamt",
            Self::RentnerGesamt => "rentner_gesamt",
        }
    }
}

impl fmt::Display for Scheibe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.als_str())
    }
}

impl FromStr for Scheibe {
    type Err = UnbekannteScheibe;

    /// ```
    /// use domain::Scheibe;
    /// assert_eq!("gesamt".parse::<Scheibe>().unwrap(), Scheibe::Gesamt);
    /// assert!("unbekannt".parse::<Scheibe>().is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ep" => Ok(Self::Ep),
            "n_vor_gwg" => Ok(Self::NVorGwg),
            "an_gesamt" => Ok(Self::AnGesamt),
            "gesamt" => Ok(Self::Gesamt),
            "rentner_gesamt" => Ok(Self::RentnerGesamt),
            other => Err(UnbekannteScheibe(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Scheibe;

    #[test]
    fn alle_fuenf_scheiben_rundtrippen() {
        for s in [
            Scheibe::Ep,
            Scheibe::NVorGwg,
            Scheibe::AnGesamt,
            Scheibe::Gesamt,
            Scheibe::RentnerGesamt,
        ] {
            assert_eq!(s.als_str().parse::<Scheibe>().unwrap(), s);
        }
    }
}
