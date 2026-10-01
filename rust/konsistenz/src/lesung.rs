//! Lesen eines Feldes aus dem Snapshot (`produkt/konsistenz/_helpers.py:14-25`).
//!
//! Python liefert `None` für zwei verschiedene Lagen: Feld fehlt ganz (nie beantwortet) und Feld
//! liegt nur vorläufig vor (Nutzer mitten im Dialog). Der Docstring (`_helpers.py:17-20`) nennt die
//! Zweideutigkeit selbst. [`Lesung`] trennt die drei Lagen; [`Lesung::bestaetigt`] ist das
//! Python-Verhalten für Aufrufer, die nur den bestätigten Wert brauchen.
use std::collections::BTreeMap;

use domain::{Lage, PyWert, Veranlagung, Zustand};
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
    ///
    /// [`PyWert`], nicht `serde_json::Value`: hier wird geprüft (`is True`, `> 0`, `strip`),
    /// nicht serialisiert. Die Ausgabetypen der Prüfungen bleiben JSON und konvertieren einmal
    /// an ihrer Grenze.
    Vorlaeufig(&'a PyWert),
    /// Vom Menschen bestätigt.
    Bestaetigt(&'a PyWert),
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
pub(crate) fn test_snap(eintraege: &[(&str, PyWert, Zustand)]) -> Felder {
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

/// Die typisierte Veranlagung eines Snapshots — der Enum-Fall von `Lage`.
///
/// # Was hier typisiert ist
///
/// Text → [`Veranlagung`] entscheidet genau eine Stelle: `Lage::veranlagung`
/// (`domain/src/lage.rs:49`). Die Aufrufer vergleichen das Enum, kein Textliteral — ein
/// Tippfehler im Vergleich kompiliert nicht mehr. Ein Store-Wert, der nicht zum Bindungstyp
/// passt, ist [`Lage::Abweichend`] und leiht den gespeicherten Wert aus dem Snapshot; noch
/// wertet ihn kein Aufrufer aus, das Verhalten bleibt paritätisch.
///
/// Nicht erreicht: Erschöpfung. Die Aufrufer prüfen mit `matches!`, eine neue
/// `Veranlagung`-Variante erzwingt hier keinen Compilerfehler.
///
/// Unbestätigt ist wie fehlend: `Vorlaeufig` gibt [`Lage::Fehlt`], so wie `bestaetigt()` dort
/// `None` liefert (`_helpers.py:22-25`). Ein bestätigtes `null` ist [`Lage::Null`] — Python sieht
/// in beiden Fällen `None`, `Lage` trennt sie. Nur für Enum-Felder, nicht für alles
/// (Entscheidung Julius).
///
/// ```
/// use domain::Lage;
/// use konsistenz::{lage_veranlagung, Felder};
/// let felder = Felder::new();
/// assert!(matches!(lage_veranlagung(&felder), Lage::Fehlt));
/// ```
#[must_use]
pub fn lage_veranlagung(felder: &Felder) -> Lage<'_, Veranlagung> {
    match lies(felder, "veranlagung") {
        Lesung::Bestaetigt(v) => Lage::veranlagung(Some(v)),
        Lesung::Fehlt | Lesung::Vorlaeufig(_) => Lage::Fehlt,
    }
}

impl<'a> Lesung<'a> {
    /// Der bestätigte Wert, sonst `None` — genau `_bestaetigt_wert` (`_helpers.py:22-25`). Ein
    /// bestätigtes `null` ist in Python ebenfalls `None` und damit hier auch.
    ///
    /// ```
    /// use domain::PyWert;
    /// use konsistenz::Lesung;
    /// let v = PyWert::Ganz(5);
    /// assert_eq!(Lesung::Bestaetigt(&v).bestaetigt(), Some(&v));
    /// assert_eq!(Lesung::Vorlaeufig(&v).bestaetigt(), None);
    /// assert_eq!(Lesung::Bestaetigt(&PyWert::Null).bestaetigt(), None);
    /// ```
    #[must_use]
    pub fn bestaetigt(self) -> Option<&'a PyWert> {
        match self {
            Self::Bestaetigt(v) if !matches!(v, PyWert::Null) => Some(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Veranlagung;
    use domain::Zustand::{Bestaetigt, Vorlaeufig};

    fn text(s: &str) -> PyWert {
        PyWert::Text(s.to_owned())
    }

    /// Der gueltige Fall: beide Enum-Werte kommen typisiert heraus.
    #[test]
    fn lage_veranlagung_gueltig() {
        let s = test_snap(&[("veranlagung", text("zusammen"), Bestaetigt)]);
        assert!(matches!(
            lage_veranlagung(&s),
            Lage::Gueltig(Veranlagung::Zusammen)
        ));
        let s = test_snap(&[("veranlagung", text("einzel"), Bestaetigt)]);
        assert!(matches!(
            lage_veranlagung(&s),
            Lage::Gueltig(Veranlagung::Einzel)
        ));
    }

    /// `Fehlt` und `Vorlaeufig` sind wie in Python `None` — und NICHT abweichend.
    #[test]
    fn lage_veranlagung_fehlt_und_vorlaeufig() {
        assert!(matches!(lage_veranlagung(&Felder::new()), Lage::Fehlt));
        let s = test_snap(&[("veranlagung", text("einzel"), Vorlaeufig)]);
        assert!(matches!(lage_veranlagung(&s), Lage::Fehlt));
    }

    /// GEGENPROBE: ein Wert, der nicht zum Bindungstyp passt, ist `Abweichend` und traegt den
    /// gespeicherten Wert. Ohne diesen Test koennte der Zweig unerreichbar sein.
    ///
    /// `"Zusammen"` ist der realistische Fall: das ist genau der Tippfehler, den
    /// `Lage::veranlagung` (`domain/src/lage.rs:45-46`) als `Abweichend` meldet, waehrend
    /// `veranlagung.as_str() == Some("zusammen")` ihn stillschweigend als „nicht zusammen"
    /// durchgelassen haette.
    #[test]
    fn lage_veranlagung_abweichend_traegt_den_wert() {
        for falsch in [
            text("Zusammen"),
            PyWert::Ganz(5),
            PyWert::Bool(true),
            PyWert::Liste(Vec::new()),
        ] {
            let s = test_snap(&[("veranlagung", falsch.clone(), Bestaetigt)]);
            assert!(
                matches!(lage_veranlagung(&s), Lage::Abweichend(w) if *w == falsch),
                "Wert {falsch:?} haette abweichend sein muessen"
            );
        }
    }

    /// Ein bestaetigtes `null` ist `Null` — nicht `Abweichend`. Python liest es als `None`
    /// (`_helpers.py:22-25`); `Abweichend` waere hier eine Falschmeldung.
    #[test]
    fn lage_veranlagung_null_ist_nicht_abweichend() {
        let s = test_snap(&[("veranlagung", PyWert::Null, Bestaetigt)]);
        assert!(matches!(lage_veranlagung(&s), Lage::Null));
    }
}
