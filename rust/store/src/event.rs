//! Das Sachverhalts-Event: Wire-Format 1:1 zu `produkt/store/schema.json#/$defs/event`.
//!
//! `Event` ist die reine Lese-/Speicherform (auch was `lade()` von einer Bestandsdatei liest,
//! ohne jede Pruefung — wie Pythons `lade()`). Die fail-closed-Pruefung sitzt an der EINEN
//! Schreibstelle, [`crate::store::Store::append`], deren Eingabetyp [`NeuesEvent`] die Zwei-
//! Signal-Regel bereits im Typsystem erzwingt (`Feldzustand::Bestaetigt` traegt zwingend ein
//! `Signal2` — der Python-Laufzeitfehler "zustand=bestaetigt braucht ein `signal_2`" kann in Rust
//! gar nicht erst konstruiert werden).
use domain::{Feldzustand, Herkunft, HerkunftVektor, PyFehler, PyWert, Schreiber, Zustand};
use serde::{Deserialize, Serialize};

use crate::canonical::EventId;

/// Zwei-Signal-Beleg (`schema.json#/$defs/signal`). `signal_1` bleibt bewusst offen (`PyWert`:
/// String, Objekt oder `null` je nach Schreiber, s. Schema-Beschreibung); `signal_2` ist ein
/// simpler `Option<String>` und NICHT `domain::Signal2` — eine geladene Bestandsdatei behaelt
/// dieselbe Freiheit wie Pythons ungeprueftes `lade()`, die Signal2-Nichtleer-Regel gilt nur beim
/// Schreiben (s. [`NeuesEvent`]).
///
/// `signal_1` ist `Option<Option<PyWert>>` (doppeltes `Option`), nicht das naheliegende einfache
/// `Option<PyWert>`: Gemessen ueber alle 192 realen Fallakten (Zaehlung, keine Werte) traegt der
/// `signal`-Schluessel bei 11.294 Events IMMER den Schluessel `signal_1` — 10.298x mit explizitem
/// `null`, 877x mit einem Wert. Bei 119 Events (einer frueheren Store-Schema-Version, strikte
/// Teilmenge der 990 Alt-Herkunft-Events, s. `domain::HerkunftVektor`-Moduldoku) FEHLT der
/// Schluessel `signal_1` komplett. Ein einfaches `Option<PyWert>` kann "Schluessel fehlt" und
/// "Schluessel da, Wert `null`" nicht unterscheiden (beides deserialisiert zu `None`) — jeder
/// Rueck-Write haette den fehlenden Schluessel lautlos durch ein explizites `null` ersetzt, byte-
/// verschieden vom Original und damit `event_id`-fremd (`sha256(canonical_json(...))` haengt am
/// `signal`-Feld). Aeussere Ebene = Schluessel da/fehlt (`None` = fehlt, per `#[serde(default)]`
/// bei fehlendem Schluessel); innere Ebene = `null`/Wert. Python-Schreibpfad setzt den Schluessel
/// IMMER (`store.py`: `signal or {"signal_1": None, ...}`), deshalb ist die aeussere Ebene beim
/// Schreiben in diesem Crate immer `Some(...)` (s. Konstruktionsstellen).
///
/// `Serialize` geht wie bei [`Event`] ueber den Konvertierer `PyWert -> Value` (s. dort).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Signal {
    // Bewusstes `Option<Option<T>>` (s. Typdoku oben: Schluessel-Anwesenheit vs. `null`-Wert
    // sind zwei verschiedene, real gemessene Zustaende, kein Sonderfall der Faelle 1-2).
    #[allow(clippy::option_option)]
    pub signal_1: Option<Option<PyWert>>,
    pub signal_2: Option<String>,
    /// Der Schluessel `signal_2` fehlt in der Akte. Der HTTP-Weg legt ein `signal` ohne `signal_2`
    /// so ab (`POST /event` mit `{"signal": {"signal_1": 5}}`); ohne dieses Merkmal schriebe Rust
    /// `signal_2: null` dazu, und die `event_id` wiche von der aus Python ab.
    pub signal_2_fehlt: bool,
}

impl<'de> Deserialize<'de> for Signal {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Beide Schluessel mit sichtbarer Anwesenheit.
        #[derive(Deserialize)]
        #[allow(clippy::option_option)]
        struct Roh {
            #[serde(default, deserialize_with = "signal_1_praesenz")]
            signal_1: Option<Option<PyWert>>,
            #[serde(default, deserialize_with = "signal_2_praesenz")]
            signal_2: Option<Option<String>>,
        }
        let roh = Roh::deserialize(deserializer)?;
        Ok(Self {
            signal_1: roh.signal_1,
            signal_2_fehlt: roh.signal_2.is_none(),
            signal_2: roh.signal_2.flatten(),
        })
    }
}

/// Macht die Anwesenheit des Schluessels sichtbar (Standard-`Option<Option<T>>`-Deserialize
/// wuerde `null` und einen fehlenden Schluessel gleichermassen zu `None` zusammenfalten) — s.
/// Typdoku oben.
#[allow(clippy::option_option)]
fn signal_1_praesenz<'de, D>(deserializer: D) -> Result<Option<Option<PyWert>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<PyWert>::deserialize(deserializer).map(Some)
}

/// Wie [`signal_1_praesenz`], fuer `signal_2`.
#[allow(clippy::option_option)]
fn signal_2_praesenz<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

impl Signal {
    /// `signal` als JSON. Ein fehlender `signal_1`-Schluessel bleibt fehlend, ein `null` bleibt
    /// `null` (s. Typdoku); `signal_2` steht da, ausser es fehlte in der Akte (`signal_2_fehlt`).
    fn zu_json(&self) -> Result<serde_json::Value, PyFehler> {
        let mut objekt = serde_json::Map::new();
        if let Some(signal_1) = &self.signal_1 {
            let wert = match signal_1 {
                None => serde_json::Value::Null,
                Some(w) => w.zu_json()?,
            };
            objekt.insert("signal_1".to_string(), wert);
        }
        if !self.signal_2_fehlt {
            objekt.insert("signal_2".to_string(), serde_json::json!(self.signal_2));
        }
        Ok(serde_json::Value::Object(objekt))
    }
}

impl Serialize for Signal {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error as _;
        self.zu_json()
            .map_err(|e| S::Error::custom(e.to_string()))?
            .serialize(serializer)
    }
}

/// Ein Sachverhalts-Event, wie im Log gespeichert. Feldnamen/Optionalitaet 1:1
/// `schema.json#/$defs/event`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Event {
    pub event_id: EventId,
    pub ts: String,
    pub feld_id: String,
    /// `schema.json`: `["number","string","boolean","null"]` — Typkonformitaet zum
    /// Bindungstyp ist Auflage T, keine JSON-Schema-Eigenschaft.
    pub wert: domain::PyWert,
    pub zustand: Zustand,
    /// `HerkunftVektor` statt der strengen `Herkunft`: 32 reale Bestandsdateien (990 Events)
    /// tragen die Alt-Form ohne `pruef_tiefe`/`haftung` (s. `domain::HerkunftVektor`-Moduldoku).
    /// `lade()` liest sie unveraendert; der Schreibpfad ([`NeuesEvent::herkunft`]) bleibt streng.
    pub herkunft: HerkunftVektor,
    pub schreiber: Schreiber,
    #[serde(default)]
    pub signal: Option<Signal>,
    #[serde(default)]
    pub ersetzt: Option<EventId>,
}

/// `Event` serialisiert ueber den Konvertierer `PyWert -> Value` -- NICHT ueber ein
/// abgeleitetes `Serialize`. Grund (Auflage 1/2): `canonical_json` erbt seine
/// Schluesselsortierung von `serde_json::Map` (`BTreeMap`). Ein direktes Serialisieren von
/// `PyWert::Objekt` uebernaehme dessen Einfuegereihenfolge und braeche jede `event_id`.
/// Der Umweg ueber `Value` erhaelt die Sortierung; gemessen: 11294/11294 identisch.
///
/// ponytail: `wert` kann hier nicht scheitern (der Konvertierer an der Append-Grenze prueft),
/// aber ein NaN/inf in `wert` oder `signal_1` wuerde als Serialisierungsfehler auftreten.
/// Upgrade: eigener Feldfehler statt `serde`-Fehler, wenn die Form je real auftritt.
impl Serialize for Event {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error as _;
        let payload = self
            .payload_ohne_event_id()
            .map_err(|e| S::Error::custom(e.to_string()))?;
        // `event_id` VOR den uebrigen Schluesseln; die Sortierung macht `Value` beim
        // Serialisieren selbst, die Einfuegereihenfolge hier ist daher ohne Wirkung.
        serde_json::Value::Object(
            [(
                "event_id".to_string(),
                serde_json::Value::String(self.event_id.to_string()),
            )]
            .into_iter()
            .chain(payload.as_object().cloned().unwrap_or_default())
            .collect(),
        )
        .serialize(serializer)
    }
}

impl Event {
    /// Das Event als `canonical_json`-Zahlwert OHNE `event_id` (`store.py:33`: `payload = {k: v
    /// for k, v in event.items() if k != "event_id"}`). Serialisierung kann nur scheitern, wenn
    /// `wert` oder `signal_1` einen nicht-endlichen Float traegt — s. Moduldoku `canonical.rs`.
    fn payload_ohne_event_id(&self) -> Result<serde_json::Value, PyFehler> {
        let wert = self.wert.zu_json()?;
        // `signal` ausdruecklich hier, nicht ueber `json!`: das ruft `Serialize` mit `unwrap` --
        // ein NaN in `signal_1` waere eine Panik statt eines Fehlers.
        let signal = self.signal.as_ref().map(Signal::zu_json).transpose()?;
        Ok(serde_json::json!({
            "ts": self.ts,
            "feld_id": self.feld_id,
            "wert": wert,
            "zustand": self.zustand,
            "herkunft": self.herkunft,
            "schreiber": self.schreiber,
            "signal": signal,
            "ersetzt": self.ersetzt,
        }))
    }

    /// Der content-adressierte `event_id` ueber die aktuellen Feldwerte (ohne `event_id`
    /// selbst). Zum Nachrechnen/Verifizieren; die Konstruktion via [`crate::store::Store::append`]
    /// setzt ihn bereits korrekt.
    /// # Errors
    /// [`PyFehler`], wenn `wert` oder `signal_1` nicht nach JSON geht (NaN/inf, auch in
    /// `Liste`/`Objekt`).
    ///
    /// ```
    /// # use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
    /// # let neu = store::NeuesEvent {
    /// #     feld_id: "ep_arbeitstage".to_string(),
    /// #     wert: serde_json::json!(220).into(),
    /// #     feldzustand: Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() },
    /// #     herkunft: Herkunft {
    /// #         herkunft: Achsenwert::new("mensch").unwrap(),
    /// #         pruef_tiefe: PruefTiefe::Ungeprueft,
    /// #         haftung: Achsenwert::new("nutzer").unwrap(),
    /// #     },
    /// #     schreiber: Schreiber::Mensch("julius".to_string()),
    /// #     signal_1: None,
    /// #     ersetzt: None,
    /// #     ts: Some("2026-01-01T00:00:00+00:00".to_string()),
    /// # };
    /// let leer = std::collections::HashMap::new();
    /// let mut s = store::Store::leer(2025, None);
    /// let id = s.append(&neu, None, store::BindungNachschlag::neu(&leer)).unwrap();
    /// let mut event = s.events()[0].clone();
    /// assert_eq!(event.berechne_event_id(), Ok(id));
    /// event.wert = domain::PyWert::Gleit(f64::NAN);
    /// assert!(event.berechne_event_id().is_err());
    /// ```
    pub fn berechne_event_id(&self) -> Result<EventId, PyFehler> {
        Ok(EventId::von_json(&self.payload_ohne_event_id()?))
    }
}

/// Eingabe fuer [`crate::store::Store::append`]. `feldzustand: Feldzustand` statt getrennter
/// `zustand`/`signal_2`-Felder macht "bestaetigt ohne Beleg" im Typsystem unrepresentierbar
/// (ersetzt Pythons Laufzeitpruefung `zustand=="bestaetigt" and not signal_2`, `store.py:365f`).
#[derive(Debug, Clone)]
pub struct NeuesEvent {
    pub feld_id: String,
    pub wert: domain::PyWert,
    pub feldzustand: Feldzustand,
    pub herkunft: Herkunft,
    pub schreiber: Schreiber,
    pub signal_1: Option<PyWert>,
    pub ersetzt: Option<EventId>,
    /// `None` -> `_now()` beim Anhaengen (Tests uebergeben einen festen Zeitstempel).
    pub ts: Option<String>,
}

/// Eingabe fuer [`crate::store::Store::append_roh`]: die Formen, die Pythons `append_event`
/// (`store.py:365`) annimmt und die [`NeuesEvent`] im Typsystem ausschliesst — `zustand`
/// vorlaeufig mit `signal_2`, die Alt-Form der Herkunft, ein `signal` ohne `signal_1`, ein
/// `ersetzt` ohne gueltige Kennung. Der Schreibweg der HTTP-Schicht (`api.event`) bekommt seine
/// Eingabe roh vom Client und muss sie so ablegen, wie Python es tut; die Event-Kennung haengt am
/// ganzen Inhalt.
#[derive(Debug, Clone)]
pub struct NeuesEventRoh {
    pub feld_id: String,
    pub wert: PyWert,
    pub zustand: Zustand,
    pub herkunft: HerkunftVektor,
    /// Der rohe Schreiber-String. Die Auflagen A/K1/F2 klassifizieren ihn wie Python nach Praefix
    /// (`import:beleg2` ist ein Beleg-Schreiber), abgelegt wird er unveraendert.
    pub schreiber: String,
    /// Wie abgelegt: `signal_1` Schluessel da/fehlt, `signal_2` Text oder `null`.
    pub signal: Signal,
    /// `Some(Typname)`, wenn `signal_2` weder Text noch `null` war (Python: `int`, `list`, ...).
    /// `signal.signal_2` ist dann `None`; abgelegt wird so ein Event nie.
    pub signal_2_fremd: Option<String>,
    /// Pythons `str(ersetzt)`; ein Text, der keine Kennung ist, trifft kein Event.
    pub ersetzt: Option<String>,
    /// `None` oder leer -> `_now()` (Python: `ts or _now()`).
    pub ts: Option<String>,
}

impl NeuesEvent {
    /// ```
    /// # use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2, Zustand};
    /// # let mut neu = store::NeuesEvent {
    /// #     feld_id: "ep_arbeitstage".to_string(),
    /// #     wert: serde_json::json!(220).into(),
    /// #     feldzustand: Feldzustand::Vorlaeufig,
    /// #     herkunft: Herkunft {
    /// #         herkunft: Achsenwert::new("mensch").unwrap(),
    /// #         pruef_tiefe: PruefTiefe::Ungeprueft,
    /// #         haftung: Achsenwert::new("nutzer").unwrap(),
    /// #     },
    /// #     schreiber: Schreiber::Mensch("julius".to_string()),
    /// #     signal_1: None,
    /// #     ersetzt: None,
    /// #     ts: None,
    /// # };
    /// assert_eq!(neu.zustand(), Zustand::Vorlaeufig);
    /// neu.feldzustand = Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() };
    /// assert_eq!(neu.zustand(), Zustand::Bestaetigt);
    /// ```
    #[must_use]
    pub fn zustand(&self) -> Zustand {
        self.feldzustand.zustand()
    }

    /// ```
    /// # use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
    /// # let mut neu = store::NeuesEvent {
    /// #     feld_id: "ep_arbeitstage".to_string(),
    /// #     wert: serde_json::json!(220).into(),
    /// #     feldzustand: Feldzustand::Vorlaeufig,
    /// #     herkunft: Herkunft {
    /// #         herkunft: Achsenwert::new("mensch").unwrap(),
    /// #         pruef_tiefe: PruefTiefe::Ungeprueft,
    /// #         haftung: Achsenwert::new("nutzer").unwrap(),
    /// #     },
    /// #     schreiber: Schreiber::Mensch("julius".to_string()),
    /// #     signal_1: None,
    /// #     ersetzt: None,
    /// #     ts: None,
    /// # };
    /// assert_eq!(neu.signal_2_roh(), None);
    /// neu.feldzustand = Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() };
    /// assert_eq!(neu.signal_2_roh(), Some("klick"));
    /// ```
    #[must_use]
    pub fn signal_2_roh(&self) -> Option<&str> {
        match &self.feldzustand {
            Feldzustand::Vorlaeufig => None,
            Feldzustand::Bestaetigt { signal_2 } => Some(signal_2.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, Signal};
    use domain::{Achsenwert, Herkunft, PruefTiefe, Zustand};
    use serde_json::json;

    fn beispiel() -> Event {
        let herkunft = Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        };
        Event {
            event_id: crate::EventId::von_json(&json!({"x": 1})),
            ts: "2026-07-17T14:00:00+00:00".to_string(),
            feld_id: "ep_arbeitstage".to_string(),
            wert: json!(220).into(),
            zustand: Zustand::Bestaetigt,
            herkunft: herkunft.into(),
            schreiber: "ui:laie".parse().unwrap(),
            signal: Some(Signal {
                signal_1: None,
                signal_2: Some("klick@ui".to_string()),
                signal_2_fehlt: false,
            }),
            ersetzt: None,
        }
    }

    #[test]
    fn wire_roundtrip_ueber_json() {
        let e = beispiel();
        let json = serde_json::to_value(&e).unwrap();
        let zurueck: Event = serde_json::from_value(json).unwrap();
        assert_eq!(zurueck, e);
    }

    #[test]
    fn berechne_event_id_ist_deterministisch() {
        let e = beispiel();
        assert_eq!(e.berechne_event_id(), e.berechne_event_id());
    }

    /// Die drei `signal_1`-Formen (Schluessel fehlt, `null`, Wert) ueberleben den Rueckweg; ein
    /// NaN in `signal_1` wird ein Fehler, keine Panik im `json!` und kein stilles `null`.
    #[test]
    fn signal_1_formen_und_nan() {
        for roh in [
            json!({"signal_2": "klick@ui"}),
            json!({"signal_1": null, "signal_2": null}),
            json!({"signal_1": {"typ": "beleg", "ref": "b#1"}, "signal_2": null}),
            // `signal_2` fehlt: bleibt fehlend (Python legt ein `signal` ohne den Schluessel so ab).
            json!({"signal_1": 5}),
            json!({"signal_1": null}),
            json!({}),
        ] {
            let signal: Signal = serde_json::from_value(roh.clone()).unwrap();
            assert_eq!(serde_json::to_value(&signal).unwrap(), roh);
        }
        let mut e = beispiel();
        e.signal = Some(Signal {
            signal_1: Some(Some(domain::PyWert::Gleit(f64::NAN))),
            signal_2: None,
            signal_2_fehlt: false,
        });
        assert!(e.berechne_event_id().is_err());
        assert!(serde_json::to_value(&e).is_err());
    }
}
