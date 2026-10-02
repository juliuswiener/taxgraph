//! Geldbetraege. Zwei Einheiten, damit ein Euro-Betrag nie versehentlich als Cent-Betrag
//! gelesen wird (Audit A §0: „Unit-Mixing in Rueckgabewerten" war eine der fuenf wichtigsten
//! Python-Fallen). Beide Typen sind `i64`-Newtypes mit privaten Feldern; die einzige Bruecke
//! zwischen ihnen ist `Cent::floor_euro`/`Cent::ceil_euro`/`Euro::to_cent`.
//!
//! Arithmetik ist ausschliesslich `checked_*` (liefert `Option`, nie `Saturating`): ein
//! Ueberlauf in einer Steuerberechnung ist ein Programmfehler, kein Wert, den man stillschweigend
//! kappen darf — `saturating_add` haette einen Rechenfehler in eine falsche, aber plausibel
//! aussehende Zahl verwandelt. Die Wahl folgt `REWRITE_PLAN.md` §4 ("fail-closed statt
//! fail-open") und ist damit dieselbe wie ueberall sonst im Store: lieber ein sichtbarer
//! `None` als eine leise falsche Zahl.
//!
//! Daneben [`Satz`] und [`Km`]: exakte `Decimal`-Werte, die mit Geld multipliziert werden, je ein
//! eigener Typ, damit ein Satz nie als Entfernung in eine Rechnung geht (Geld-Entscheidung
//! Punkt 2: Saetze und km nie als nacktes `Decimal`).

use std::fmt;

use rust_decimal::Decimal;

/// Ein Betrag in Cent, der kleinsten Einheit, in der der Python-Store rechnet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cent(i64);

/// Ganzzahliger Euro-Betrag (z. B. `hoechstbetrag_ohne_kfz: 4500` in einer Params-YAML).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Euro(i64);

/// `Euro::to_cent` haette den Wertebereich von `i64` verlassen (nur bei absurd grossen
/// Betraegen erreichbar, aber `checked_mul` macht daraus einen Typfehler statt eines Wrap-Arounds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Euro-Betrag {0} EUR ueberschreitet den Cent-Wertebereich")]
pub struct CentUeberlauf(pub i64);

/// Ein Satz aus Params-YAML oder Gesetz: Euro je km (`0.30`), Anteil (`0.8`) oder Prozent
/// (`13.2`) -- welche Einheit, sagt das Feld.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Satz(Decimal);

/// Eine Entfernung in km mit Nachkommastellen (`entfernung_km_roh`, z. B. `10.6`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Km(Decimal);

impl Cent {
    /// Baut einen Cent-Betrag aus einer rohen `i64`.
    ///
    /// ```
    /// use domain::Cent;
    /// assert_eq!(Cent::new(150).get(), 150);
    /// ```
    #[must_use]
    pub const fn new(cent: i64) -> Self {
        Self(cent)
    }

    /// Der rohe Cent-Wert.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }

    /// Abrunden auf ganze Euro — Python-`//`-Semantik (`div_euclid`, rundet Richtung `-inf`,
    /// nicht Richtung 0 wie der Rust-Operator `/`). Bei negativen Betraegen (Verlusten) ist das
    /// tragend: `-150.div_euclid(100) == -2`, waehrend `-150 / 100 == -1` waere.
    ///
    /// ```
    /// use domain::{Cent, Euro};
    /// assert_eq!(Cent::new(250).floor_euro(), Euro::new(2));
    /// assert_eq!(Cent::new(-150).floor_euro(), Euro::new(-2));
    /// ```
    #[must_use]
    pub const fn floor_euro(self) -> Euro {
        Euro(self.0.div_euclid(100))
    }

    /// Aufrunden auf ganze Euro.
    ///
    /// ```
    /// use domain::{Cent, Euro};
    /// assert_eq!(Cent::new(201).ceil_euro(), Euro::new(3));
    /// assert_eq!(Cent::new(200).ceil_euro(), Euro::new(2));
    /// assert_eq!(Cent::new(-150).ceil_euro(), Euro::new(-1));
    /// ```
    #[must_use]
    pub const fn ceil_euro(self) -> Euro {
        let q = self.0.div_euclid(100);
        let r = self.0.rem_euclid(100);
        Euro(if r == 0 { q } else { q + 1 })
    }

    /// Checked Addition. `None` bei Ueberlauf statt einer stillschweigend gekappten Zahl.
    ///
    /// ```
    /// use domain::Cent;
    /// assert_eq!(Cent::new(100).checked_add(Cent::new(50)), Some(Cent::new(150)));
    /// assert_eq!(Cent::new(i64::MAX).checked_add(Cent::new(1)), None);
    /// ```
    #[must_use]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.0.checked_add(rhs.0) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Checked Subtraktion.
    ///
    /// ```
    /// use domain::Cent;
    /// assert_eq!(Cent::new(100).checked_sub(Cent::new(150)), Some(Cent::new(-50)));
    /// ```
    #[must_use]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.0.checked_sub(rhs.0) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Checked Negation (fuer Verlustvortraege).
    ///
    /// ```
    /// use domain::Cent;
    /// assert_eq!(Cent::new(100).checked_neg(), Some(Cent::new(-100)));
    /// ```
    #[must_use]
    pub const fn checked_neg(self) -> Option<Self> {
        match self.0.checked_neg() {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }
}

impl Euro {
    /// Baut einen Euro-Betrag aus einer rohen `i64`.
    #[must_use]
    pub const fn new(euro: i64) -> Self {
        Self(euro)
    }

    /// Der rohe Euro-Wert.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }

    /// Nach Cent. `Err` nur, wenn `self * 100` `i64` verlaesst.
    ///
    /// # Errors
    /// [`CentUeberlauf`], wenn der Euro-Betrag ausserhalb des in Cent darstellbaren Bereichs liegt.
    ///
    /// ```
    /// use domain::{Cent, Euro};
    /// assert_eq!(Euro::new(45).to_cent(), Ok(Cent::new(4500)));
    /// ```
    pub const fn to_cent(self) -> Result<Cent, CentUeberlauf> {
        match self.0.checked_mul(100) {
            Some(v) => Ok(Cent::new(v)),
            None => Err(CentUeberlauf(self.0)),
        }
    }
}

impl Satz {
    /// Baut einen Satz aus einem rohen `Decimal`.
    #[must_use]
    pub const fn new(satz: Decimal) -> Self {
        Self(satz)
    }

    /// Der rohe `Decimal`-Wert.
    #[must_use]
    pub const fn get(self) -> Decimal {
        self.0
    }
}

impl Km {
    /// Baut eine Entfernung aus einem rohen `Decimal`.
    #[must_use]
    pub const fn new(km: Decimal) -> Self {
        Self(km)
    }

    /// Der rohe `Decimal`-Wert.
    #[must_use]
    pub const fn get(self) -> Decimal {
        self.0
    }

    /// Volle Kilometer: ein angefangener km bleibt unberuecksichtigt, abgeschnitten Richtung 0
    /// wie Pythons `int()`. `None` ausserhalb von `i64`.
    ///
    /// Rechtsgrundlage: § 9 Abs. 1 S. 3 Nr. 4 S. 2 `EStG` (Pauschale "für jeden vollen Kilometer
    /// der Entfernung"); BMF v. 18.11.2021, Rz. 12.
    ///
    /// ```
    /// use domain::Km;
    /// use rust_decimal::Decimal;
    /// assert_eq!(Km::new(Decimal::new(209, 1)).volle_km(), Some(20));
    /// assert_eq!(Km::new(Decimal::new(-5, 1)).volle_km(), Some(0));
    /// ```
    #[must_use]
    pub fn volle_km(self) -> Option<i64> {
        i64::try_from(self.0.trunc()).ok()
    }
}

impl fmt::Display for Cent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Cent", self.0)
    }
}

impl fmt::Display for Euro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} EUR", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Cent, Euro};
    use proptest::prelude::*;

    proptest! {
        /// `floor_euro` folgt exakt Pythons `//`: Ergebnis*100 <= x < (Ergebnis+1)*100, auch bei
        /// negativen Cent-Betraegen.
        #[test]
        fn floor_euro_liegt_im_richtigen_intervall(x in any::<i64>()) {
            let floor = Cent::new(x).floor_euro().get();
            prop_assert!(floor.checked_mul(100).is_some());
            let lo = floor * 100;
            prop_assert!(lo <= x);
            if let Some(hi) = floor.checked_add(1).and_then(|f| f.checked_mul(100)) {
                prop_assert!(x < hi);
            }
        }

        /// Aequivalent zur Python-Floor-Division per `div_euclid`.
        #[test]
        fn floor_euro_ist_div_euclid(x in any::<i64>()) {
            prop_assert_eq!(Cent::new(x).floor_euro().get(), x.div_euclid(100));
        }
    }

    #[test]
    fn to_cent_und_floor_euro_sind_inverse_fuer_glatte_betraege() {
        let e = Euro::new(4500);
        assert_eq!(e.to_cent().unwrap_or(Cent::new(i64::MIN)).floor_euro(), e);
    }
}
