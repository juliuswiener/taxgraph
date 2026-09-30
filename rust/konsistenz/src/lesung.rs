//! Lesen eines Feldes aus dem Snapshot (`produkt/konsistenz/_helpers.py:14-25`).
//!
//! Python liefert `None` für zwei verschiedene Lagen: Feld fehlt ganz (nie beantwortet) und Feld
//! liegt nur vorläufig vor (Nutzer mitten im Dialog). Der Docstring (`_helpers.py:17-20`) nennt die
//! Zweideutigkeit selbst. [`Lesung`] trennt die drei Lagen; [`Lesung::bestaetigt`] ist das
//! Python-Verhalten für Aufrufer, die nur den bestätigten Wert brauchen.
use std::collections::BTreeMap;

use domain::Zustand;
use serde_json::Value;
use store::SnapshotFeld;

/// Materialisierter Snapshot `feld_id -> Feld` (`store.py::materialisiere`), feld_id-sortiert wie
/// in Python (`store.py:601-602`). Die Sortierung trägt: `flag_check` iteriert die Schlüssel.
pub type Felder = BTreeMap<String, SnapshotFeld>;

/// Die drei Lagen eines Feldes im Snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lesung<'a> {
    /// Kein Event für dieses Feld — nie beantwortet.
    Fehlt,
    /// Vorhanden, aber nur vorläufig: zählt nicht als Beleg.
    Vorlaeufig(&'a Value),
    /// Vom Menschen bestätigt.
    Bestaetigt(&'a Value),
}

/// Liest `feld_id` aus dem Snapshot.
///
/// ```
/// use konsistenz::{lies, Felder, Lesung};
/// let felder = Felder::new();
/// assert_eq!(lies(&felder, "veranlagung"), Lesung::Fehlt);
/// ```
#[must_use]
pub fn lies<'a>(felder: &'a Felder, feld_id: &str) -> Lesung<'a> {
    match felder.get(feld_id) {
        None => Lesung::Fehlt,
        Some(f) => match f.zustand {
            Zustand::Bestaetigt => Lesung::Bestaetigt(&f.wert),
            Zustand::Vorlaeufig => Lesung::Vorlaeufig(&f.wert),
        },
    }
}

/// Test-Snapshot aus `(feld_id, wert, zustand)`; Herkunft ist für keine Prüfung hier relevant.
#[cfg(test)]
pub(crate) fn test_snap(eintraege: &[(&str, Value, Zustand)]) -> Felder {
    use domain::{Achsenwert, Herkunft, PruefTiefe};
    let herkunft = Herkunft {
        herkunft: Achsenwert::new("test").unwrap(),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: Achsenwert::new("test").unwrap(),
    };
    eintraege
        .iter()
        .map(|(k, v, z)| {
            (
                (*k).to_owned(),
                SnapshotFeld {
                    wert: v.clone(),
                    zustand: *z,
                    herkunft: herkunft.clone().into(),
                },
            )
        })
        .collect()
}

impl<'a> Lesung<'a> {
    /// Der bestätigte Wert, sonst `None` — genau `_bestaetigt_wert` (`_helpers.py:22-25`). Ein
    /// bestätigtes `null` ist in Python ebenfalls `None` und damit hier auch.
    ///
    /// ```
    /// use konsistenz::Lesung;
    /// let v = serde_json::json!(5);
    /// assert_eq!(Lesung::Bestaetigt(&v).bestaetigt(), Some(&v));
    /// assert_eq!(Lesung::Vorlaeufig(&v).bestaetigt(), None);
    /// assert_eq!(Lesung::Bestaetigt(&serde_json::Value::Null).bestaetigt(), None);
    /// ```
    #[must_use]
    pub fn bestaetigt(self) -> Option<&'a Value> {
        match self {
            Self::Bestaetigt(v) if !v.is_null() => Some(v),
            _ => None,
        }
    }
}
