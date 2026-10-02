//! Antwortstand eines Feldes und die Zaehl-Primitive (`traverser.py:128-141, 237-265`).
use std::collections::{BTreeSet, HashMap};
use std::num::NonZeroU16;
use std::str::FromStr;

use bindung::InstanzGruppe;
use domain::{FeldId, Zustand};
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
    use domain::testhilfe::{d3, json_wert, klasse, pruefe, py, Ergebnis};
    use proptest::prelude::*;
    use serde_json::{json, Value};

    use super::{Antwort, InstanzAnzahl};

    /// Ausnahmen von `aus_zaehlfeld`.
    const ANZAHL: &[&str] = &["D3"];

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
}
