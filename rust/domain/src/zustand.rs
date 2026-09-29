//! Zwei-Signal-Zustand eines Feldwerts (`produkt/store/store.py:149-150,364-366`).
//!
//! Ein Vorschlag (KI, Beleg-Import, Kontoauszug, Ableitung) ist IMMER `vorlaeufig` und traegt
//! `signal_2=None`; nur ein Mensch kann mit einem zweiten Signal auf `bestaetigt` heben
//! (`store.py:365`: `zustand == "bestaetigt" and not signal.get("signal_2")` ist ein
//! `ValueError`). [`Zustand`] ist die blanke Zwei-Wert-Achse (fuer `meet_zustand`); [`Feldzustand`]
//! traegt den Beleg (`Signal2`) mit, den ein `Bestaetigt`-Wert laut Store zwingend hat.
use std::fmt;

use serde::{Deserialize, Serialize};

/// Vorlaeufig oder bestaetigt — ohne den Beleg dahinter. Die Achse, ueber der `meet_zustand`
/// rechnet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Zustand {
    Vorlaeufig,
    Bestaetigt,
}

/// Das zweite Signal, das einen Wert von `vorlaeufig` auf `bestaetigt` hebt. Nicht-leer nach
/// Trim (Store: `not (signal.get("signal_2") or "").strip()`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signal2(String);

/// `Signal2::new` hat einen leeren oder nur aus Leerraum bestehenden String bekommen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("signal_2 darf nicht leer sein (fail-closed: zustand=bestaetigt braucht ein Signal)")]
pub struct LeeresSignal2;

impl Signal2 {
    /// # Errors
    /// [`LeeresSignal2`], wenn `s` nach Trim leer ist.
    ///
    /// ```
    /// use domain::Signal2;
    /// assert!(Signal2::new("beweist@fam_anzahl_kinder=2").is_ok());
    /// assert!(Signal2::new("   ").is_err());
    /// ```
    pub fn new(s: impl Into<String>) -> Result<Self, LeeresSignal2> {
        let s = s.into();
        if s.trim().is_empty() {
            return Err(LeeresSignal2);
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Signal2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Vorlaeufig ODER bestaetigt-mit-Beleg. Ein `Feldzustand::Bestaetigt` ohne `Signal2` ist im
/// Typsystem unrepresentierbar — die Prüfung, die in Python zur Laufzeit als `ValueError`
/// auftritt (Auflage: "zustand=bestaetigt braucht ein `signal_2`"), entfaellt dadurch komplett.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Feldzustand {
    Vorlaeufig,
    Bestaetigt { signal_2: Signal2 },
}

impl Feldzustand {
    /// Die blanke Zwei-Wert-Achse, ohne den Beleg.
    ///
    /// ```
    /// use domain::{Feldzustand, Signal2, Zustand};
    /// let f = Feldzustand::Bestaetigt { signal_2: Signal2::new("mensch").unwrap() };
    /// assert_eq!(f.zustand(), Zustand::Bestaetigt);
    /// ```
    #[must_use]
    pub fn zustand(&self) -> Zustand {
        match self {
            Self::Vorlaeufig => Zustand::Vorlaeufig,
            Self::Bestaetigt { .. } => Zustand::Bestaetigt,
        }
    }
}

/// Pruef-Tiefe der Herkunft, geordnet vom schwaechsten zum staerksten Beleg
/// (`store.py:47`: `_PRUEF_ORD`). `meet_herkunft` bildet das Minimum ueber diese Ordnung
/// (die schwaechste Kette in einer Summe bestimmt die Pruef-Tiefe der Summe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PruefTiefe {
    Ungeprueft,
    Plausibilisiert,
    OrakelBestaetigt,
    Amtlich,
}

#[cfg(test)]
mod tests {
    use super::PruefTiefe;

    #[test]
    fn pruef_tiefe_ordnung_stimmt_mit_python_dict_ueberein() {
        assert!(PruefTiefe::Ungeprueft < PruefTiefe::Plausibilisiert);
        assert!(PruefTiefe::Plausibilisiert < PruefTiefe::OrakelBestaetigt);
        assert!(PruefTiefe::OrakelBestaetigt < PruefTiefe::Amtlich);
    }
}
