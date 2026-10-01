//! Lesen eines Feldes aus dem Snapshot (`produkt/konsistenz/_helpers.py:14-25`).
//!
//! Python liefert `None` für zwei verschiedene Lagen: Feld fehlt ganz (nie beantwortet) und Feld
//! liegt nur vorläufig vor (Nutzer mitten im Dialog). Der Docstring (`_helpers.py:17-20`) nennt die
//! Zweideutigkeit selbst. [`Lesung`] trennt die drei Lagen; [`Lesung::bestaetigt`] ist das
//! Python-Verhalten für Aufrufer, die nur den bestätigten Wert brauchen.
use std::collections::BTreeMap;

use domain::{Lage, PyWert, Veranlagung, Zustand};
use serde::Deserialize as _;
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

/// Der bestätigte Wert eines Feldes als [`PyWert`] — die Naht, die K4 selbst schlägt.
///
/// `store::SnapshotFeld::wert` ist heute `serde_json::Value` (`store.rs:154`). K2 sollte das
/// typisieren; die Rücknahme auf `77ea7a5` hat es mitgenommen (`PyWert` steht in **0** Refs
/// unter `rust/store/`, gemessen 2026-10-01). `Lage<T>` braucht aber `&PyWert`
/// (`domain/src/lage.rs:49`), also entsteht der `PyWert` hier.
///
/// ponytail: ein `PyWert` je gelesenem Feld, an genau dieser einen Naht — nicht je Feldzugriff
/// und ohne Zwischenspeicher. K2 baut den Hop aus, sobald `SnapshotFeld::wert` selbst `PyWert`
/// ist; dann wird aus dem Aufruf ein Leihvorgang und diese Funktion verschwindet. Ohne diesen
/// Kommentar sieht der Umweg in einem Monat aus wie Absicht.
///
/// Der `expect` ist kein stiller Fehlerpfad: [`PyWert`]s `Deserialize` liegt im Produktionspfad
/// (`py_wert.rs:423`, vor `#[cfg(test)]`) und nimmt jeden JSON-Wert an — `testhilfe::py` verlässt
/// sich darauf. Ein `Value`, den `PyWert` nicht annähme, gäbe es nicht.
#[must_use]
#[allow(
    clippy::expect_used,
    reason = "PyWert nimmt jeden JSON-Wert an (wie testhilfe::py, domain/src/testhilfe.rs:286)"
)]
pub(crate) fn als_pywert(wert: &Value) -> PyWert {
    PyWert::deserialize(wert).expect("PyWert nimmt jeden JSON-Wert an")
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

/// Die typisierte Veranlagung eines Snapshots — der Enum-Fall von `Lage`, soweit er ohne K2
/// darstellbar ist.
///
/// # Was hier typisiert ist
///
/// Text → [`Veranlagung`] entscheidet genau eine Stelle: `Lage::veranlagung`
/// (`domain/src/lage.rs:49`). Die Aufrufer vergleichen das Enum, kein Textliteral — ein
/// Tippfehler im Vergleich kompiliert nicht mehr. Ein Store-Wert, der nicht zum Bindungstyp
/// passt, ist als `abweichend` von „einzel" unterscheidbar; noch wertet ihn kein Aufrufer aus,
/// das Verhalten bleibt paritätisch.
///
/// Nicht erreicht: Erschöpfung. Die Aufrufer prüfen mit `matches!`, eine neue
/// `Veranlagung`-Variante erzwingt hier keinen Compilerfehler.
///
/// # Warum `Lage::Abweichend` hier fehlt
///
/// `Lage<'a, T>::Abweichend(&'a PyWert)` (`domain/src/lage.rs:23`) verlangt eine Referenz mit
/// der Lebensdauer des Snapshots. An dieser Naht entsteht der [`PyWert`] aber **lokal** aus
/// `SnapshotFeld::wert: serde_json::Value` (`store.rs:154`) — eine Referenz darauf kann die
/// Funktion nicht zurückgeben (`error[E0515]`, gemessen 2026-10-01). Der Zweig fehlt also
/// **nicht aus Versehen**: er ist ohne K2 nicht darstellbar. K2 typisiert
/// `SnapshotFeld::wert` selbst; dann wird `Abweichend` ein Leihvorgang und dieser Ersatz
/// verschwindet.
///
/// ponytail: deshalb `abweichend: bool` statt `Lage::Abweichend` — der Fall bleibt sichtbar,
/// aber er trägt keinen `PyWert`. K2 macht daraus `Lage<Veranlagung>`. Bis dahin gilt: `true`
/// heißt „der bestätigte Wert passt nicht zum Bindungstyp" und ist **nicht** dasselbe wie
/// „Feld fehlt" (`Fehlt`) oder „Feld steht als null" (`Null`) — die drei zu verschmelzen wäre
/// der Fehler, den `Lage` gerade verhindert.
///
/// Unbestätigt ist wie fehlend: `bestaetigt()` liefert für `Vorlaeufig` `None`
/// (`_helpers.py:22-25`). Nur für Enum-Felder, nicht für alles (Entscheidung Julius), und
/// `store` bleibt unangetastet.
///
/// ```
/// use konsistenz::{lage_veranlagung, Felder};
/// use domain::Veranlagung;
/// let felder = Felder::new();
/// assert_eq!(lage_veranlagung(&felder), (None, false));
/// ```
#[must_use]
pub fn lage_veranlagung(felder: &Felder) -> (Option<Veranlagung>, bool) {
    let wert = match lies(felder, "veranlagung") {
        Lesung::Bestaetigt(v) => als_pywert(v),
        Lesung::Fehlt | Lesung::Vorlaeufig(_) => return (None, false),
    };
    match Lage::veranlagung(Some(&wert)) {
        Lage::Gueltig(v) => (Some(v), false),
        Lage::Abweichend(_) => (None, true),
        // Ein bestätigtes `null` ist in Python ebenfalls `None` (`_helpers.py:22-25`).
        Lage::Null | Lage::Fehlt => (None, false),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Veranlagung;
    use domain::Zustand::{Bestaetigt, Vorlaeufig};
    use serde_json::json;

    /// Der gueltige Fall: beide Enum-Werte kommen typisiert heraus.
    #[test]
    fn lage_veranlagung_gueltig() {
        let s = test_snap(&[("veranlagung", json!("zusammen"), Bestaetigt)]);
        assert_eq!(lage_veranlagung(&s), (Some(Veranlagung::Zusammen), false));
        let s = test_snap(&[("veranlagung", json!("einzel"), Bestaetigt)]);
        assert_eq!(lage_veranlagung(&s), (Some(Veranlagung::Einzel), false));
    }

    /// `Fehlt` und `Vorlaeufig` sind wie in Python `None` — und NICHT abweichend.
    /// Ohne diesen Test waere `false` im Fehlt-Fall nicht von „passt" zu unterscheiden.
    #[test]
    fn lage_veranlagung_fehlt_und_vorlaeufig() {
        assert_eq!(lage_veranlagung(&Felder::new()), (None, false));
        let s = test_snap(&[("veranlagung", json!("einzel"), Vorlaeufig)]);
        assert_eq!(lage_veranlagung(&s), (None, false));
    }

    /// GEGENPROBE: ein Wert, der nicht zum Bindungstyp passt, setzt `abweichend` auf `true`.
    /// Ohne diesen Test waere der `bool` Dekoration — er koennte nie `true` werden.
    ///
    /// `"Zusammen"` ist der realistische Fall: das ist genau der Tippfehler, den
    /// `Lage::veranlagung` (`domain/src/lage.rs:45-46`) als `Abweichend` meldet, waehrend
    /// `veranlagung.as_str() == Some("zusammen")` ihn stillschweigend als „nicht zusammen"
    /// durchgelassen haette.
    #[test]
    fn lage_veranlagung_abweichend_wird_true() {
        for falsch in [json!("Zusammen"), json!(5), json!(true), json!([])] {
            let s = test_snap(&[("veranlagung", falsch.clone(), Bestaetigt)]);
            assert_eq!(
                lage_veranlagung(&s),
                (None, true),
                "Wert {falsch} haette abweichend sein muessen"
            );
        }
    }

    /// Ein bestaetigtes `null` ist `Null` — nicht `Abweichend`. Python liest es als `None`
    /// (`_helpers.py:22-25`); `Abweichend` waere hier eine Falschmeldung.
    #[test]
    fn lage_veranlagung_null_ist_nicht_abweichend() {
        let s = test_snap(&[("veranlagung", Value::Null, Bestaetigt)]);
        assert_eq!(lage_veranlagung(&s), (None, false));
    }
}
