//! Typestate der Zwei-Signal-Invariante am Ring (`bescheid_zweige.py:1508`, `REWRITE_PLAN.md` §4):
//! `Snapshot<Roh>` → [`Snapshot::nur_bestaetigt`] → `Snapshot<Bestaetigt>`.
//!
//! Die Zweige sind ueber die [`Marke`] generisch. `Marke::NUR_BESTAETIGT` speist die
//! Instanz-Summen (`Instanzquelle`): Snapshot-Typ und Instanz-Filter koennen nicht auseinanderlaufen.
//! Ein `Snapshot<Bestaetigt>` entsteht nur ueber [`Snapshot::nur_bestaetigt`] (private Felder).
use std::marker::PhantomData;

use domain::Zustand;

use crate::Felder;

mod versiegelt {
    pub trait Versiegelt {}
}

/// Zustandsmarke eines [`Snapshot`]. Nur [`Roh`] und [`Bestaetigt`] (versiegelt).
pub trait Marke: versiegelt::Versiegelt {
    /// Python `nur_bestaetigt`: filtert der Ring die Instanz-Summen auf bestaetigte Instanzen?
    const NUR_BESTAETIGT: bool;
}

/// Alle Felder, wie sie im Store stehen (Estimate-Pfad: `nur_bestaetigt=False`).
#[derive(Debug, Clone, Copy)]
pub struct Roh;
/// Nur `zustand == bestaetigt` (festgesetzter Pfad: `nur_bestaetigt=True`).
#[derive(Debug, Clone, Copy)]
pub struct Bestaetigt;

impl versiegelt::Versiegelt for Roh {}
impl versiegelt::Versiegelt for Bestaetigt {}
impl Marke for Roh {
    const NUR_BESTAETIGT: bool = false;
}
impl Marke for Bestaetigt {
    const NUR_BESTAETIGT: bool = true;
}

/// Ein Feld-Snapshot (`feld_id -> {wert, zustand, herkunft}`) mit seinem Filterzustand im Typ.
#[derive(Debug, Clone)]
pub struct Snapshot<Z: Marke> {
    felder: Felder,
    _z: PhantomData<Z>,
}

impl Snapshot<Roh> {
    /// Ungefilterter Snapshot.
    ///
    /// ```
    /// use bescheid::testhilfe::{felder, store};
    /// use bescheid::zweige::Snapshot;
    /// use serde_json::json;
    /// let roh = Snapshot::roh(felder(&store(&[("a", json!(1), true), ("b", json!(2), false)])));
    /// assert_eq!(roh.felder().len(), 2); // der Rohsnapshot behaelt Vorlaeufiges
    /// ```
    #[must_use]
    pub fn roh(felder: Felder) -> Self {
        Self {
            felder,
            _z: PhantomData,
        }
    }

    /// DIE Filterstelle: `{fid: ev for ... if ev.get("zustand") == "bestaetigt"}`
    /// (`bescheid_zweige.py:1509`). Ein vorlaeufiger Wert ist danach absent.
    ///
    /// ```
    /// use bescheid::testhilfe::{felder, store};
    /// use bescheid::zweige::Snapshot;
    /// use serde_json::json;
    /// let fest = Snapshot::roh(felder(&store(&[("a", json!(1), true), ("b", json!(2), false)]))).nur_bestaetigt();
    /// assert!(fest.felder().contains_key("a"));
    /// assert!(!fest.felder().contains_key("b")); // vorlaeufig: fuer den Ring nicht vorhanden
    /// ```
    #[must_use]
    pub fn nur_bestaetigt(self) -> Snapshot<Bestaetigt> {
        let felder = self
            .felder
            .into_iter()
            .filter(|(_, ev)| ev.zustand == Zustand::Bestaetigt)
            .collect::<Felder>();
        // Die EINE Filterstelle des Rings: danach ist jedes Feld bestaetigt.
        debug_assert!(felder.values().all(|ev| ev.zustand == Zustand::Bestaetigt));
        Snapshot {
            felder,
            _z: PhantomData,
        }
    }
}

impl<Z: Marke> Snapshot<Z> {
    /// Die Felder, wie der Zweig sie liest (Python `f = felder or {}`).
    ///
    /// ```
    /// use bescheid::{zweige::Snapshot, Felder};
    /// assert!(Snapshot::roh(Felder::new()).felder().is_empty());
    /// ```
    #[must_use]
    pub fn felder(&self) -> &Felder {
        &self.felder
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::testhilfe::{felder, store};

    /// Der Filter laesst nur `bestaetigt` durch; `Roh` behaelt beides, der Typ traegt das Flag.
    #[test]
    fn nur_bestaetigt_filtert_vorlaeufige() {
        let roh = Snapshot::roh(felder(&store(&[
            ("bruttoarbeitslohn", json!(5_000_000), true),
            ("agb_aufwendungen", json!(900_000), false),
        ])));
        assert_eq!(roh.felder().len(), 2);
        const { assert!(!Roh::NUR_BESTAETIGT) };
        let fest = roh.nur_bestaetigt();
        const { assert!(Bestaetigt::NUR_BESTAETIGT) };
        assert!(fest.felder().contains_key("bruttoarbeitslohn"));
        assert!(!fest.felder().contains_key("agb_aufwendungen"));
    }
}
