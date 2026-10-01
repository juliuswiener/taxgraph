//! `bescheid_via_slots` (`intervall.py:168-193`): `feld_id` → `signatur_slot` über die Bindung,
//! Summanden-Slots addieren, Engine-Ausgabe auf Cent normieren.
//!
//! `NATIV_EINHEIT`/`nach_cent` (`intervall.py:36-56`) entfallen: die Slot-Funktion gibt [`Cent`]
//! oder [`Euro`] typisiert zurück ([`NahtEinheit`]). Eine „unbelegte Quantität" (Python:
//! `ValueError`) ist damit ein Kompilierfehler.
use std::collections::{BTreeMap, HashMap};

use bindung::SlotBeitrag;
use domain::PyWert;
use domain::{Cent, CentUeberlauf, Euro};

use crate::{AchsenBindung, Werte};

/// Slot-Werte `signatur_slot -> wert`, wie die Engine-Funktion sie liest.
///
/// [`PyWert`], weil hier addiert wird und `als_int` Pythons `int`-Sicht liest.
pub type Slots = BTreeMap<String, PyWert>;

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

/// Python-`int`-Sicht eines Werts für `+`: Ganzzahl oder `bool` (0/1), über
/// [`PyWert::int_mit_bool`]; eine Ganzzahl über `i64::MAX` liefert `None` (D3).
///
/// ponytail: `Gleit` liefert hier `None` (wie `Value::as_i64` vorher) — Pythons `+`
/// auf einem Float ergaebe eine Float-Summe. Unerreichbar, weil Summanden-Felder
/// durchweg `typ: cent` sind (gemessen 22 von 22, s. `SlotFehler::SummandNichtGanzzahl`).
/// Upgrade: ein Float-Zweig, wenn je ein Float-Summand auftritt.
fn als_int(v: &PyWert) -> Option<i64> {
    v.int_mit_bool().ok().flatten()
}

/// Baut `bescheid_fn(feld_werte)` aus einer slot-basierten Engine-Funktion. Rein, ohne Zustand.
///
/// - Bindung ohne `signatur_slot` (`geltungsbedingung`): übersprungen.
/// - `slot_beitrag: summand`: addiert auf den Slot (Start 0); sonst setzt das Feld den Slot, das
///   spätere Feld gewinnt.
///
/// ```
/// use domain::{Cent, PyWert};
/// use intervall::{bescheid_via_slots, AchsenBindung, Werte};
/// let b = |f: &str| AchsenBindung { feld_id: f.into(), typ: domain::Feldtyp::Cent, askable: true,
///     enum_werte: vec![], bereich: None, signatur_slot: Some("gesamt".into()),
///     slot_beitrag: bindung::SlotBeitrag::Summand };
/// let bindung = [b("an"), b("ag")];
/// let f = bescheid_via_slots(&bindung, |s| Ok::<_, std::convert::Infallible>(
///     Cent::new(s.get("gesamt").and_then(|v| v.int().ok()).unwrap_or(0))));
/// let mut w = Werte::neu();
/// w.setze("an", PyWert::Ganz(100));
/// w.setze("ag", PyWert::Ganz(40));
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
                    slots.insert(slot.clone(), PyWert::Ganz(summe));
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

/// Aequivalenz mit `domain::PyWert` (D15): Abweichungen nur mit D-Nummer aus der Liste des
/// Helfers, je D-Nummer ein Test mit dem `CPython`-Verhalten.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{d3_d10, json_wert, pruefe, py, Ergebnis};
    use domain::PyWert;
    use proptest::prelude::*;
    use serde_json::json;

    use super::als_int;

    /// Ausnahmen von `als_int`.
    const INT: &[&str] = &["D3", "D10"];

    /// `CPython`s `0 + wert` aus `intervall.py:189` (`slots.get(slot, 0) + wert`): `bool` und
    /// `int` bleiben `int` (`True == 1`), ein `int` ueber `i64::MAX` rechnet exakt weiter, `float`
    /// bleibt `float`, alles andere wirft `TypeError`. `0 + -0.0` ist `0.0` (gemessen an 3.12.9 und
    /// 3.14.7), deshalb `0.0 + f` statt `f`.
    fn plus_null(w: &PyWert) -> Ergebnis<PyWert> {
        match w {
            PyWert::Bool(b) => Ok(PyWert::Ganz(i64::from(*b))),
            PyWert::Ganz(n) => Ok(PyWert::Ganz(*n)),
            PyWert::GrossGanz(u) => Ok(PyWert::GrossGanz(*u)),
            PyWert::Gleit(f) => Ok(PyWert::Gleit(0.0 + *f)),
            _ => Err(Some("TypeError")),
        }
    }

    /// Beide Seiten als `repr`, weil `Ganz(1)` von `Gleit(1.0)` getrennt bleiben muss -- das
    /// abgeleitete `PartialEq` auf `PyWert` ist strukturell, nicht Pythons `==` (s. `py_eq`).
    /// `als_int`s `None` wird zum
    /// `TypeError`, den `CPython` fuer `0 + <Nichtzahl>` wirft; wo Python stattdessen weiterrechnet,
    /// steht die Abweichung als D3 oder D10 in der Liste.
    fn alt_repr(v: &PyWert) -> Ergebnis<String> {
        als_int(v).map_or(Err(Some("TypeError")), |i| Ok(PyWert::Ganz(i).repr()))
    }

    fn neu_repr(v: &PyWert) -> Ergebnis<String> {
        plus_null(v).map(|w| w.repr())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn als_int_wie_pywert(v in json_wert()) {
            let w = py(&v);
            let (alt, neu) = (alt_repr(&w), neu_repr(&w));
            pruefe(&v, &alt, &neu, || d3_d10(&v), INT)?;
        }
    }

    /// D3: `2**64 - 1` ist in `CPython` ein `int`, `0 + wert` bleibt exakt. Der Alt-Helfer liefert
    /// `None` (`SummandNichtGanzzahl`).
    #[test]
    fn d3_ueber_i64() {
        let v = json!(u64::MAX);
        let w = py(&v);
        assert_eq!(als_int(&w), None);
        assert_eq!(neu_repr(&w), Ok("18446744073709551615".to_owned()));
    }

    /// D10: ein `float` zaehlt in `CPython` als Zahl, `0 + wert` bleibt `float`. Der Alt-Helfer
    /// liefert `None` (`SummandNichtGanzzahl`). Gemessen an 3.12.9 und 3.14.7: `0 + 2.5` ist
    /// `2.5`, `0 + 1500.0` ist `1500.0`, `0 + -0.0` ist `0.0`.
    #[test]
    fn d10_float_ist_summand() {
        for (v, text) in [
            (json!(2.5), "2.5"),
            (json!(1500.0), "1500.0"),
            (json!(-0.0), "0.0"),
        ] {
            let w = py(&v);
            assert_eq!(als_int(&w), None);
            assert_eq!(neu_repr(&w), Ok(text.to_owned()));
        }
    }
}
