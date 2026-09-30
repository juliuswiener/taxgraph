//! `intervall` — [min, max]-Bescheid-Intervall und Beitrag je unsicherem Feld
//! (`produkt/unsicherheit/intervall.py`). Gehört später zur Crate `bescheid`; bis dahin eigenständig.
//!
//! Rein deterministisch. Rechnet über eine INJIZIERTE reine Funktion `Werte -> Cent`; in Produktion
//! baut [`bescheid_via_slots`] sie aus einer slot-basierten Engine-Funktion.
//!
//! Zwei Stufen (`produkt/unsicherheit/KONZEPT.md`):
//! 1. One-at-a-time: Beitrag je unsicherer Achse (Ranking-Heuristik, kann Interaktionen
//!    unterschätzen).
//! 2. Gedeckelter kartesischer Raum über die Top-K-Treiber: exakt bezüglich dieser Felder,
//!    `gedeckelt` + `rest_felder`, wenn nicht alle Achsen passen.
//!
//! Ehrlichkeit: ein unbeschränktes Feld ohne Vorschlag ist nicht fixierbar — dann gibt es keine
//! Zahl ([`Spanne::NichtFixierbar`]), kein erfundener Ersatzwert.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

mod rechnung;
mod slots;

use bindung::{Bindungspunkt, SlotBeitrag};
use domain::Feldtyp;
use serde_json::Value;

pub use rechnung::{
    intervall, Beitrag, Intervall, IntervallErgebnis, IntervallFehler, Spanne, CAP_DEFAULT,
};
pub use slots::{bescheid_via_slots, NahtEinheit, SlotFehler, Slots};

/// Die Sicht auf einen Bindungseintrag, die Intervall und Slot-Adapter lesen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchsenBindung {
    pub feld_id: String,
    pub typ: Feldtyp,
    pub askable: bool,
    /// `enum_werte`; fehlend = leer (Python: `b.get("enum_werte") or []`).
    pub enum_werte: Vec<String>,
    /// `bereich` als `(min, max)`.
    pub bereich: Option<(i64, i64)>,
    /// `quelle.signatur_slot`; `None` bei einer `geltungsbedingung`-Bindung (kein Zahl-Slot).
    pub signatur_slot: Option<String>,
    /// Fehlend = `Exakt` (Python: nur `== "summand"` addiert).
    pub slot_beitrag: SlotBeitrag,
}

/// Beleg der Abbildung: `intervall_paritaet.rs` gibt Python dieselben `feld_id`s, Python liest die
/// YAMLs roh (`TR.lade_bindung()`).
impl From<&bindung::Bindung> for AchsenBindung {
    fn from(b: &bindung::Bindung) -> Self {
        Self {
            feld_id: b.feld_id.clone(),
            typ: b.typ,
            askable: b.askable,
            enum_werte: b.enum_werte.clone().unwrap_or_default(),
            bereich: b.bereich.as_ref().map(|r| (r.min, r.max)),
            signatur_slot: match &b.quelle.bindungspunkt {
                Bindungspunkt::SignaturSlot(s) => Some(s.clone()),
                Bindungspunkt::Geltungsbedingung(_) => None,
            },
            slot_beitrag: b.slot_beitrag.unwrap_or(SlotBeitrag::Exakt),
        }
    }
}

/// Feldwerte in Einfügereihenfolge (`feld_id -> wert`). Die Reihenfolge trägt: teilen sich zwei
/// `exakt`-Felder einen Slot (gemessen: 9 Slot-Namen, z. B. `monate`), gewinnt in
/// [`bescheid_via_slots`] das spätere — wie beim Python-`dict`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Werte(Vec<(String, Value)>);

impl Werte {
    /// Leere Werte.
    ///
    /// ```
    /// assert!(intervall::Werte::neu().get("x").is_none());
    /// ```
    #[must_use]
    pub const fn neu() -> Self {
        Self(Vec::new())
    }

    /// Setzt `feld_id`; ein vorhandener Schlüssel behält seine Position (Python-`dict`).
    ///
    /// ```
    /// let mut w = intervall::Werte::neu();
    /// w.setze("a", 1.into());
    /// w.setze("b", 2.into());
    /// w.setze("a", 3.into());
    /// assert_eq!(w.iter().map(|(k, _)| k).collect::<Vec<_>>(), ["a", "b"]);
    /// ```
    pub fn setze(&mut self, feld_id: &str, wert: Value) {
        match self.0.iter_mut().find(|(k, _)| k == feld_id) {
            Some((_, v)) => *v = wert,
            None => self.0.push((feld_id.to_owned(), wert)),
        }
    }

    /// Wert zu `feld_id`.
    ///
    /// ```
    /// let mut w = intervall::Werte::neu();
    /// w.setze("a", 1.into());
    /// assert_eq!(w.get("a"), Some(&1.into()));
    /// ```
    #[must_use]
    pub fn get(&self, feld_id: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k == feld_id).map(|(_, v)| v)
    }

    /// Alle Paare in Einfügereihenfolge.
    ///
    /// ```
    /// assert_eq!(intervall::Werte::neu().iter().count(), 0);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }
}
