//! `bescheid_via_slots` (`intervall.py:168-193`): `feld_id` → `signatur_slot` über die Bindung,
//! Summanden-Slots addieren, Engine-Ausgabe auf Cent normieren.
//!
//! `NATIV_EINHEIT`/`nach_cent` (`intervall.py:36-56`) entfallen: die Slot-Funktion gibt [`Cent`]
//! oder [`Euro`] typisiert zurück ([`NahtEinheit`]). Eine „unbelegte Quantität" (Python:
//! `ValueError`) ist damit ein Kompilierfehler.
use std::collections::{BTreeMap, HashMap};

use bindung::SlotBeitrag;
use domain::{Cent, CentUeberlauf, Euro};
use serde_json::Value;

use crate::{AchsenBindung, Werte};

/// Slot-Werte `signatur_slot -> wert`, wie die Engine-Funktion sie liest.
pub type Slots = BTreeMap<String, Value>;

/// Ergebnis-Einheit einer Engine-Funktion, verlustfrei nach Cent.
pub trait NahtEinheit {
    /// Nach Cent (Euro × 100, Cent unverändert).
    ///
    /// # Errors
    /// [`CentUeberlauf`], wenn Euro × 100 `i64` verlässt.
    ///
    /// ```
    /// use domain::{Cent, Euro};
    /// use intervall::NahtEinheit;
    /// assert_eq!(Euro::new(7).in_cent(), Ok(Cent::new(700)));
    /// assert_eq!(Cent::new(7).in_cent(), Ok(Cent::new(7)));
    /// ```
    fn in_cent(self) -> Result<Cent, CentUeberlauf>;
}

impl NahtEinheit for Cent {
    fn in_cent(self) -> Result<Cent, CentUeberlauf> {
        Ok(self)
    }
}

impl NahtEinheit for Euro {
    fn in_cent(self) -> Result<Cent, CentUeberlauf> {
        self.to_cent()
    }
}

/// Fehler der aus [`bescheid_via_slots`] gebauten Funktion.
#[derive(Debug, thiserror::Error)]
pub enum SlotFehler<E: std::error::Error + 'static> {
    /// `feld_id` fehlt in der Bindung. Python: `bindung[fid]` → `KeyError`.
    #[error("feld_id {0} fehlt in der Bindung")]
    UnbekanntesFeld(String),
    /// Summand-Addition über einen Nicht-Ganzzahlwert. Python: `TypeError` (bzw.
    /// String-Verkettung bei `str + str` — PARITÄT: außerhalb des Wertebereichs, Summanden-Felder
    /// sind durchweg `typ: cent`, gemessen 22 von 22).
    #[error("Summand {0} ist keine Ganzzahl")]
    SummandNichtGanzzahl(String),
    /// Summe oder Euro→Cent verlässt `i64` (Python rechnet unbeschränkt).
    #[error("Überlauf im Slot {0}")]
    Ueberlauf(String),
    #[error(transparent)]
    Slot(E),
}

/// Python-`int`-Sicht eines Werts für `+`: Ganzzahl oder `bool` (0/1).
fn als_int(v: &Value) -> Option<i64> {
    match v {
        Value::Bool(b) => Some(i64::from(*b)),
        _ => v.as_i64(),
    }
}

/// Baut `bescheid_fn(feld_werte)` aus einer slot-basierten Engine-Funktion. Rein, ohne Zustand.
///
/// - Bindung ohne `signatur_slot` (`geltungsbedingung`): übersprungen.
/// - `slot_beitrag: summand`: addiert auf den Slot (Start 0); sonst setzt das Feld den Slot, das
///   spätere Feld gewinnt.
///
/// ```
/// use domain::Cent;
/// use intervall::{bescheid_via_slots, AchsenBindung, Werte};
/// let b = |f: &str| AchsenBindung { feld_id: f.into(), typ: domain::Feldtyp::Cent, askable: true,
///     enum_werte: vec![], bereich: None, signatur_slot: Some("gesamt".into()),
///     slot_beitrag: bindung::SlotBeitrag::Summand };
/// let bindung = [b("an"), b("ag")];
/// let f = bescheid_via_slots(&bindung, |s| Ok::<_, std::convert::Infallible>(
///     Cent::new(s.get("gesamt").and_then(|v| v.as_i64()).unwrap_or(0))));
/// let mut w = Werte::neu();
/// w.setze("an", 100.into());
/// w.setze("ag", 40.into());
/// assert_eq!(f(&w).unwrap(), Cent::new(140));
/// ```
pub fn bescheid_via_slots<'b, T, E, F>(
    bindung: &'b [AchsenBindung],
    slot_fn: F,
) -> impl Fn(&Werte) -> Result<Cent, SlotFehler<E>> + 'b
where
    T: NahtEinheit,
    E: std::error::Error + 'static,
    F: Fn(&Slots) -> Result<T, E> + 'b,
{
    let nach_id: HashMap<&str, &AchsenBindung> =
        bindung.iter().map(|b| (b.feld_id.as_str(), b)).collect();
    move |feld_werte: &Werte| {
        let mut slots = Slots::new();
        for (fid, wert) in feld_werte.iter() {
            let b = nach_id
                .get(fid)
                .ok_or_else(|| SlotFehler::UnbekanntesFeld(fid.to_owned()))?;
            let Some(slot) = &b.signatur_slot else {
                continue;
            };
            match b.slot_beitrag {
                SlotBeitrag::Summand => {
                    let bisher = slots.get(slot).map_or(Some(0), als_int);
                    let summe = bisher
                        .zip(als_int(wert))
                        .ok_or_else(|| SlotFehler::SummandNichtGanzzahl(fid.to_owned()))?;
                    let summe = summe
                        .0
                        .checked_add(summe.1)
                        .ok_or_else(|| SlotFehler::Ueberlauf(slot.clone()))?;
                    slots.insert(slot.clone(), summe.into());
                }
                SlotBeitrag::Exakt => {
                    slots.insert(slot.clone(), wert.clone());
                }
            }
        }
        slot_fn(&slots)
            .map_err(SlotFehler::Slot)?
            .in_cent()
            .map_err(|_| SlotFehler::Ueberlauf(String::new()))
    }
}
