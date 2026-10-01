//! Wie ein gespeichertes Feld zu seinem Bindungstyp steht (`REWRITE_PLAN` §7, 9b-B).
//!
//! Python liest `.get(feld_id)` und bekommt `None` fuer "fehlt" wie fuer "steht als null da";
//! danach prueft jeder Leser von Hand. `Lage` trennt die vier Faelle an einer Stelle. Laden ist
//! tolerant (`REWRITE_PLAN` P10): ein Wert, der nicht zum Bindungstyp passt, bleibt als
//! [`Lage::Abweichend`] lesbar, statt die Datei unladbar zu machen. Die Schreibpruefung
//! (`store.py:167-190`, `_typ_konform`) bleibt davon unberuehrt.
//!
//! Konstruktoren gibt es fuer [`Veranlagung`] und [`Cent`]. Konfession, Bundesland und Rentenart
//! haben noch kein Enum in `domain`; Text-Felder bekommen kein `Lage` (Laden prueft kein `muster`).
use crate::{Cent, PyWert, Veranlagung};

/// Ein Feld, gelesen gegen seinen Bindungstyp `T`.
#[derive(Debug, Clone, Copy)]
pub enum Lage<'a, T> {
    /// Das Feld steht nicht im Store.
    Fehlt,
    /// Das Feld steht als `null` im Store.
    Null,
    /// Der Wert passt zum Bindungstyp.
    Gueltig(T),
    /// Der Wert passt nicht zum Bindungstyp; was das heisst, entscheidet der Leser.
    Abweichend(&'a PyWert),
}

impl<'a, T> Lage<'a, T> {
    fn aus(wert: Option<&'a PyWert>, gueltig: impl FnOnce(&PyWert) -> Option<T>) -> Self {
        match wert {
            None => Self::Fehlt,
            Some(PyWert::Null) => Self::Null,
            Some(w) => gueltig(w).map_or(Self::Abweichend(w), Self::Gueltig),
        }
    }
}

impl<'a> Lage<'a, Veranlagung> {
    /// Bindung `typ: enum`, `enum_werte: ["einzel", "zusammen"]` (`bindung_an_gesamt.yaml:173-177`).
    /// Python prueft `wert in enum_werte` (`store.py:178`): exakter Textvergleich, kein `strip`,
    /// keine Gross-/Kleinschreibung.
    ///
    /// ```
    /// use domain::{Lage, PyWert, Veranlagung};
    /// let w = PyWert::Text("zusammen".into());
    /// assert!(matches!(Lage::veranlagung(Some(&w)), Lage::Gueltig(Veranlagung::Zusammen)));
    /// let w = PyWert::Text("Zusammen".into());
    /// assert!(matches!(Lage::veranlagung(Some(&w)), Lage::Abweichend(_)));
    /// ```
    #[must_use]
    pub fn veranlagung(wert: Option<&'a PyWert>) -> Self {
        Self::aus(wert, |w| match w {
            PyWert::Text(s) if s == "einzel" => Some(Veranlagung::Einzel),
            PyWert::Text(s) if s == "zusammen" => Some(Veranlagung::Zusammen),
            _ => None,
        })
    }
}

impl<'a> Lage<'a, Cent> {
    /// Bindung `typ: cent`: Python `isinstance(wert, int) and not isinstance(wert, bool)`
    /// (`store.py:174`). Rust-Grenze: `Cent` ist `i64`. Eine Ganzzahl ueber `i64::MAX`
    /// ([`PyWert::GrossGanz`]) nimmt Python an, hier ist sie `Abweichend` — wie in der
    /// Rust-Schreibpruefung `Wert::aus_json` (`as_i64`).
    ///
    /// ```
    /// use domain::{Cent, Lage, PyWert};
    /// assert!(matches!(Lage::cent(Some(&PyWert::Ganz(1500))), Lage::Gueltig(c) if c == Cent::new(1500)));
    /// assert!(matches!(Lage::cent(Some(&PyWert::Bool(true))), Lage::Abweichend(_)));
    /// assert!(matches!(Lage::cent(None), Lage::Fehlt));
    /// ```
    #[must_use]
    pub fn cent(wert: Option<&'a PyWert>) -> Self {
        Self::aus(wert, |w| match w {
            PyWert::Ganz(n) => Some(Cent::new(*n)),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Lage;
    use crate::{Cent, PyWert, Veranlagung};

    /// `_typ_konform(w, "enum", ["einzel", "zusammen"])`, gemessen 3.12.9 und 3.14.7: `True` nur
    /// fuer `"einzel"` und `"zusammen"`; `False` fuer jeden Wert der Schleife unten.
    #[test]
    fn veranlagung_nur_exakter_text() {
        assert!(matches!(Lage::veranlagung(None), Lage::Fehlt));
        assert!(matches!(Lage::veranlagung(Some(&PyWert::Null)), Lage::Null));
        let einzel = PyWert::Text("einzel".into());
        assert!(matches!(
            Lage::veranlagung(Some(&einzel)),
            Lage::Gueltig(Veranlagung::Einzel)
        ));
        for w in [
            PyWert::Text(" einzel".into()),
            PyWert::Text("EINZEL".into()),
            PyWert::Text(String::new()),
            PyWert::Bool(false),
            PyWert::Ganz(0),
            PyWert::Liste(vec![PyWert::Text("einzel".into())]),
        ] {
            assert!(
                matches!(Lage::veranlagung(Some(&w)), Lage::Abweichend(a) if std::ptr::eq(a, &raw const w)),
                "{w:?}"
            );
        }
    }

    /// `_typ_konform(w, "cent", None)`, gemessen 3.12.9 und 3.14.7: `True` nur fuer `1500`,
    /// `-3` und `2**63`; `False` fuer `True`, `1500.0`, `"1500"`, `None`. `2**63` ist die
    /// Rust-Grenze (`Abweichend`), `None` ist `Lage::Null`.
    #[test]
    fn cent_wie_typ_konform() {
        assert!(
            matches!(Lage::cent(Some(&PyWert::Ganz(1500))), Lage::Gueltig(c) if c == Cent::new(1500))
        );
        assert!(
            matches!(Lage::cent(Some(&PyWert::Ganz(-3))), Lage::Gueltig(c) if c == Cent::new(-3))
        );
        assert!(matches!(Lage::cent(Some(&PyWert::Null)), Lage::Null));
        for w in [
            PyWert::GrossGanz(1 << 63),
            PyWert::Bool(true),
            PyWert::Gleit(1500.0),
            PyWert::Text("1500".into()),
        ] {
            assert!(matches!(Lage::cent(Some(&w)), Lage::Abweichend(_)), "{w:?}");
        }
    }
}
