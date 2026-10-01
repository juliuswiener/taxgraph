//! Antwortstand eines Feldes und die Zaehl-Primitive (`traverser.py:128-141, 237-265`).
use std::collections::{BTreeSet, HashMap};
use std::num::NonZeroU16;
use std::str::FromStr;

use bindung::InstanzGruppe;
use domain::{FeldId, Zustand};
use serde_json::Value;
use store::{Event, SnapshotFeld, Store};

/// Gemeinsame Gestalt von Store-Event und Snapshot-Eintrag (`{wert, zustand, …}`) — EINE
/// Zaehlregel fuer beide Leserichtungen (`traverser.py:128-135`).
pub trait Eintrag {
    /// `vorlaeufig` oder `bestaetigt`.
    fn zustand(&self) -> Zustand;
    /// Der gespeicherte Wert.
    ///
    /// [`domain::PyWert`], nicht `serde_json::Value`: die Zaehlregel hier ist Pythons
    /// Semantik (`True` zaehlt als 1, `NaN` ist truthy), nicht Serialisierung.
    fn wert(&self) -> &domain::PyWert;
}

impl Eintrag for Event {
    fn zustand(&self) -> Zustand {
        self.zustand
    }
    fn wert(&self) -> &domain::PyWert {
        &self.wert
    }
}

impl Eintrag for SnapshotFeld {
    fn zustand(&self) -> Zustand {
        self.zustand
    }
    fn wert(&self) -> &domain::PyWert {
        &self.wert
    }
}

/// Offen (kein Event ODER vorlaeufig) oder bestaetigt mit Wert (`_unbeantwortet`,
/// `traverser.py:248-249`).
///
/// Ein vorlaeufiger KI-Vorschlag ist keine Antwort des Nutzers: er darf weder eine Frage aus der
/// Queue nehmen noch eine Bedingung entscheiden noch die Instanz-Zahl setzen. Weil der Wert nur
/// in [`Antwort::Bestaetigt`] steckt, kann keine Lesestelle einen vorlaeufigen Wert pruefen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Antwort<'e> {
    /// Kein Event oder `vorlaeufig`.
    Offen,
    /// `bestaetigt`, mit dem Wert.
    ///
    /// [`domain::PyWert`]: wer hier liest, prueft einen Wert (`traverser.py`), er
    /// serialisiert ihn nicht.
    Bestaetigt(&'e domain::PyWert),
}

impl<'e> Antwort<'e> {
    /// Der Antwortstand eines (moeglicherweise fehlenden) Eintrags.
    ///
    /// ```
    /// use interview::Antwort;
    /// assert_eq!(Antwort::aus::<store::Event>(None), Antwort::Offen);
    /// ```
    #[must_use]
    pub fn aus<E: Eintrag>(eintrag: Option<&'e E>) -> Self {
        match eintrag {
            Some(e) if e.zustand() == Zustand::Bestaetigt => Self::Bestaetigt(e.wert()),
            _ => Self::Offen,
        }
    }

    /// `true` fuer [`Antwort::Offen`].
    ///
    /// ```
    /// assert!(interview::Antwort::Offen.ist_offen());
    /// ```
    #[must_use]
    pub fn ist_offen(self) -> bool {
        matches!(self, Self::Offen)
    }
}

/// Python-Gleichheit `a == b` auf JSON-Werten: `True == 1 == 1.0`, `False == 0`, Zahl gegen
/// String nie gleich. Die Bedingungen in der Bindung vergleichen mit `==`/`!=`
/// (`traverser.py:324, 555, 557`), und ein Wert vom falschen Typ im Store (`1` auf einem
/// bool-Feld) muss hier genauso ausfallen wie dort.
///
/// ```
/// use interview::py_eq;
/// use serde_json::json;
/// assert!(py_eq(&json!(true), &json!(1)));
/// assert!(py_eq(&json!(0), &json!(false)));
/// assert!(py_eq(&json!(2), &json!(2.0)));
/// assert!(!py_eq(&json!("1"), &json!(1)));
/// assert!(!py_eq(&json!(null), &json!(false)));
/// ```
#[must_use]
pub fn py_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| py_eq(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| py_eq(v, w)))
        }
        _ => match (py_zahl(a), py_zahl(b)) {
            (Some(x), Some(y)) => x.gleich(y),
            _ => false,
        },
    }
}

#[derive(Clone, Copy)]
enum PyZahl {
    Int(i128),
    Float(f64),
}

fn py_zahl(v: &Value) -> Option<PyZahl> {
    match v {
        Value::Bool(b) => Some(PyZahl::Int(i128::from(*b))),
        Value::Number(n) => n
            .as_i64()
            .map(|i| PyZahl::Int(i128::from(i)))
            .or_else(|| n.as_u64().map(|u| PyZahl::Int(i128::from(u))))
            .or_else(|| n.as_f64().map(PyZahl::Float)),
        _ => None,
    }
}

impl PyZahl {
    /// Python vergleicht `int` und `float` exakt (nicht ueber eine Rundung nach `float`).
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::float_cmp,
        reason = "exakter int/float-Vergleich wie Python: nur ganzzahlige, betraglich < 2^64 \
                  Floats werden nach i128 gewandelt, dort verlustfrei"
    )]
    fn gleich(self, other: Self) -> bool {
        match (self, other) {
            (Self::Int(x), Self::Int(y)) => x == y,
            (Self::Float(x), Self::Float(y)) => x == y,
            (Self::Int(i), Self::Float(f)) | (Self::Float(f), Self::Int(i)) => {
                f.is_finite() && f.fract() == 0.0 && f.abs() < 1.8e19 && f as i128 == i
            }
        }
    }
}

/// Zahl der Instanzen einer Gruppe: mindestens 1, hoechstens `gruppe.max`
/// (`_anzahl_aus_eintrag`, `traverser.py:128-141`).
///
/// Nur ein BESTAETIGTES Zaehlfeld zaehlt: ein vorlaeufiger Vorschlag der KI ("2 Kinder") darf
/// nicht entscheiden, wie viele Felder der Nutzer ausfuellen soll. Nur eine echte Ganzzahl >= 1
/// zaehlt (`bool` und `2.0` sind in Python kein `int`), sonst bleibt es bei 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstanzAnzahl(NonZeroU16);

impl InstanzAnzahl {
    /// Eine Instanz.
    pub const EINS: Self = Self(NonZeroU16::MIN);

    /// Die Zahl aus dem Zaehlfeld-Eintrag.
    ///
    /// PARITAET: Python kappt mit `min(n, int(max))` und liefert bei `max = 0` eine 0; das
    /// Schema verlangt `max >= 1` (`schema.json`, `instanz_gruppe.max.minimum`), Rust nimmt
    /// dann 1. Ebenso kappt Rust bei `u16::MAX` (alle gepflegten Gruppen: `max <= 9`).
    ///
    /// ```
    /// use interview::{Antwort, InstanzAnzahl};
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// let kind = g.instanz_gruppe("kind").unwrap();
    /// let drei = domain::PyWert::Ganz(3);
    /// assert_eq!(InstanzAnzahl::aus_zaehlfeld(Antwort::Bestaetigt(&drei), kind).get(), 3);
    /// assert_eq!(InstanzAnzahl::aus_zaehlfeld(Antwort::Offen, kind).get(), 1);
    /// let viel = domain::PyWert::Ganz(1000);
    /// assert_eq!(InstanzAnzahl::aus_zaehlfeld(Antwort::Bestaetigt(&viel), kind).get(), u16::try_from(kind.max).unwrap());
    /// ```
    #[must_use]
    pub fn aus_zaehlfeld(antwort: Antwort<'_>, gruppe: &InstanzGruppe) -> Self {
        // PARITAET: `int_ohne_bool` bildet Pythons `int(...)`-Sicht ab (ein `bool` zaehlt
        // NICHT als Zahl -- der Alt-Pfad verlangte `Value::Number`), und `GrossGanz`
        // saettigt auf `i64::MAX` wie vorher `n.as_u64().map(|_| i64::MAX)`.
        let Antwort::Bestaetigt(w) = antwort else {
            return Self::EINS;
        };
        let n = match w {
            domain::PyWert::Ganz(n) => *n,
            domain::PyWert::GrossGanz(_) => i64::MAX,
            _ => return Self::EINS,
        };
        if n < 1 {
            return Self::EINS;
        }
        let max = i64::from(gruppe.max).clamp(1, i64::from(u16::MAX));
        u16::try_from(n.min(max))
            .ok()
            .and_then(NonZeroU16::new)
            .map_or(Self::EINS, Self)
    }

    /// Die Zahl.
    ///
    /// ```
    /// assert_eq!(interview::InstanzAnzahl::EINS.get(), 1);
    /// ```
    #[must_use]
    pub fn get(self) -> u16 {
        self.0.get()
    }
}

/// `feld_id -> aktives Event` plus `basis -> Instanznummern >= 2` (`_aktive_events`,
/// `traverser.py:237-245`). Die Aktiv-Aufloesung selbst ist [`Store::aktive`] — EINE
/// Implementierung (`store.py:_aktives`), hier nur einmal je Aufruf eingesammelt.
pub(crate) struct Aktiv<'s> {
    events: HashMap<&'s str, &'s Event>,
    instanzen: HashMap<String, BTreeSet<NonZeroU16>>,
}

impl<'s> Aktiv<'s> {
    pub(crate) fn aus(store: &'s Store) -> Self {
        let events: HashMap<&str, &Event> = store.aktive().collect();
        let mut instanzen: HashMap<String, BTreeSet<NonZeroU16>> = HashMap::new();
        for k in events.keys() {
            // `_instanz_antworten` (`traverser.py:261-264`): Namensmuster, gegengeprobt gegen
            // `instanz_feld_id` — `x__02`, `x__1`, `x__0` sind keine Instanz. `FeldId::from_str`
            // ist genau diese Regel.
            // PARITAET: Python akzeptiert jede Zahl, Rust nur n <= u16::MAX; Python wirft bei
            // Unicode-Ziffern wie `x__²` (`isdigit` ja, `int()` nein), Rust ignoriert sie.
            if let Ok(fid) = FeldId::from_str(k) {
                if fid.instanznummer().get() >= 2 {
                    instanzen
                        .entry(fid.basis().as_str().to_owned())
                        .or_default()
                        .insert(fid.instanznummer());
                }
            }
        }
        Self { events, instanzen }
    }

    pub(crate) fn get(&self, feld_id: &str) -> Option<&'s Event> {
        self.events.get(feld_id).copied()
    }

    pub(crate) fn antwort(&self, feld_id: &str) -> Antwort<'s> {
        Antwort::aus(self.get(feld_id))
    }

    /// Instanznummern >= 2, die fuer `basis` im Store stehen.
    pub(crate) fn instanzen_von(&self, basis: &str) -> impl Iterator<Item = NonZeroU16> + '_ {
        self.instanzen.get(basis).into_iter().flatten().copied()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&'s str, &'s Event)> + '_ {
        self.events.iter().map(|(k, v)| (*k, *v))
    }
}

/// Aequivalenz mit `domain::PyWert` (D15): Abweichungen nur mit D-Nummer aus der Liste des
/// Helfers, je D-Nummer ein Test mit dem `CPython`-Verhalten.
#[cfg(test)]
mod aequivalenz {
    use bindung::InstanzGruppe;
    use domain::testhilfe::{d3, json_wert, klasse, pruefe, py, py_absteigend, zahl, Ergebnis};
    use proptest::prelude::*;
    use serde_json::{json, Number, Value};

    use super::{py_eq, Antwort, InstanzAnzahl};

    /// Ausnahmen von `py_eq`.
    const GLEICH: &[&str] = &["D14"];
    /// Ausnahmen von `aus_zaehlfeld`.
    const ANZAHL: &[&str] = &["D3"];

    /// `v` mit jeder Zahl als Float und jedem `bool` als Ganzzahl. In Python gleich, ausser eine
    /// Ganzzahl rundet als Float.
    fn umgetypt(v: &Value) -> Value {
        match v {
            Value::Bool(b) => Value::from(u8::from(*b)),
            Value::Number(n) => n
                .as_f64()
                .and_then(Number::from_f64)
                .map_or_else(|| v.clone(), Value::Number),
            Value::Array(l) => Value::Array(l.iter().map(umgetypt).collect()),
            Value::Object(o) => {
                Value::Object(o.iter().map(|(k, w)| (k.clone(), umgetypt(w))).collect())
            }
            w => w.clone(),
        }
    }

    /// Paare fuer `==`: ein Wert und seine [`umgetypt`]-Fassung, zwei Zahlen, zwei Werte.
    fn paar() -> impl Strategy<Value = (Value, Value)> {
        prop_oneof![
            json_wert().prop_map(|a| (umgetypt(&a), a)),
            (zahl(), zahl()).prop_map(|(a, b)| (Value::Number(a), Value::Number(b))),
            (json_wert(), json_wert()),
        ]
    }

    /// D14: an derselben Stelle stehen eine Ganzzahl und ein gleicher Float ab 1,8e19.
    fn d14(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Array(x), Value::Array(y)) => x.iter().zip(y).any(|(p, q)| d14(p, q)),
            (Value::Object(x), Value::Object(y)) => {
                x.iter().any(|(k, p)| y.get(k).is_some_and(|q| d14(p, q)))
            }
            (Value::Number(x), Value::Number(y)) => {
                x.is_f64() != y.is_f64()
                    && x.as_f64().is_some_and(|f| f.abs() >= 1.8e19)
                    && py(a).py_eq(&py(b))
            }
            _ => false,
        }
    }

    /// `_anzahl_aus_eintrag` (`traverser.py:136-141`) mit `PyWert` fuer ein bestaetigtes `v`: nur
    /// ein `int` ab 1 zaehlt, gekappt auf `max`.
    fn anzahl(v: &Value, max: u32) -> Ergebnis<u16> {
        let n = klasse(py(v).int_ohne_bool())?.filter(|n| *n >= 1);
        Ok(n.map_or(1, |n| u16::try_from(n.min(i64::from(max))).unwrap_or(0)))
    }

    fn gruppe(max: u32) -> InstanzGruppe {
        InstanzGruppe {
            gruppe: "kind".to_owned(),
            anzahl_feld: "anzahl_kinder".to_owned(),
            etikett: String::new(),
            max,
            grund: String::new(),
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        /// Beide Richtungen; `PyWert` mit umgekehrter Schluessel-Reihenfolge (`dict` ohne
        /// Reihenfolge).
        #[test]
        fn py_eq_wie_pywert((a, b) in paar()) {
            for (x, y) in [(&a, &b), (&b, &a)] {
                let (alt, neu) = (py_eq(x, y), py(x).py_eq(&py_absteigend(y)));
                let d = || if !alt && neu && d14(x, y) { vec!["D14"] } else { Vec::new() };
                pruefe(&(x, y), &alt, &neu, d, GLEICH)?;
            }
        }

        /// `max` aus dem Schema-Bereich (`minimum: 1`), gepflegt sind hoechstens 9.
        #[test]
        fn aus_zaehlfeld_wie_pywert(v in json_wert(), max in 1u32..=20) {
            let w = py(&v);
            let alt = Ok(InstanzAnzahl::aus_zaehlfeld(Antwort::Bestaetigt(&w), &gruppe(max)).get());
            let neu = anzahl(&v, max);
            pruefe(&v, &alt, &neu, || d3(&v, &neu), ANZAHL)?;
        }
    }

    /// D3: `min(2**64 - 1, 9)` ist in `CPython` 9. Der Alt-Helfer liefert 9, `PyWert` meldet die
    /// i64-Grenze.
    #[test]
    fn d3_ueber_i64() {
        let v = json!(u64::MAX);
        let w = py(&v);
        let alt = InstanzAnzahl::aus_zaehlfeld(Antwort::Bestaetigt(&w), &gruppe(9));
        assert_eq!(alt.get(), 9);
        assert_eq!(klasse(py(&v).int_ohne_bool()), Err(None));
    }

    /// D14: `18000000000000000000 == 1.8e19` ist in `CPython` wahr. Der Alt-Helfer vergleicht
    /// Ganzzahl und Float nur unter 1,8e19.
    #[test]
    fn d14_ab_1_8e19() {
        for (i, f) in [
            (json!(18_000_000_000_000_000_000u64), json!(1.8e19)),
            (
                json!(18_446_744_073_709_549_568u64),
                json!(18_446_744_073_709_549_568.0),
            ),
        ] {
            assert!(!py_eq(&i, &f));
            assert!(py(&i).py_eq(&py(&f)));
        }
    }
}
