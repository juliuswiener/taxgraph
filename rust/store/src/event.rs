//! Das Sachverhalts-Event: Wire-Format 1:1 zu `produkt/store/schema.json#/$defs/event`.
//!
//! `Event` ist die reine Lese-/Speicherform (auch was `lade()` von einer Bestandsdatei liest,
//! ohne jede Pruefung — wie Pythons `lade()`). Die fail-closed-Pruefung sitzt an der EINEN
//! Schreibstelle, [`crate::store::Store::append`], deren Eingabetyp [`NeuesEvent`] die Zwei-
//! Signal-Regel bereits im Typsystem erzwingt (`Feldzustand::Bestaetigt` traegt zwingend ein
//! `Signal2` — der Python-Laufzeitfehler "zustand=bestaetigt braucht ein `signal_2`" kann in Rust
//! gar nicht erst konstruiert werden).
use domain::{Feldzustand, Herkunft, Schreiber, Zustand};
use serde::{Deserialize, Serialize};

use crate::canonical::EventId;

/// Zwei-Signal-Beleg (`schema.json#/$defs/signal`). `signal_1` bleibt bewusst offenes JSON
/// (String, Objekt oder `null` je nach Schreiber, s. Schema-Beschreibung); `signal_2` ist ein
/// simpler `Option<String>` und NICHT `domain::Signal2` — eine geladene Bestandsdatei behaelt
/// dieselbe Freiheit wie Pythons ungeprueftes `lade()`, die Signal2-Nichtleer-Regel gilt nur beim
/// Schreiben (s. [`NeuesEvent`]).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    #[serde(default)]
    pub signal_1: Option<serde_json::Value>,
    #[serde(default)]
    pub signal_2: Option<String>,
}

/// Ein Sachverhalts-Event, wie im Log gespeichert. Feldnamen/Optionalitaet 1:1
/// `schema.json#/$defs/event`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub event_id: EventId,
    pub ts: String,
    pub feld_id: String,
    /// `schema.json`: `["number","string","boolean","null"]` — Typkonformitaet zum
    /// Bindungstyp ist Auflage T, keine JSON-Schema-Eigenschaft.
    pub wert: serde_json::Value,
    pub zustand: Zustand,
    pub herkunft: Herkunft,
    pub schreiber: Schreiber,
    #[serde(default)]
    pub signal: Option<Signal>,
    #[serde(default)]
    pub ersetzt: Option<EventId>,
}

impl Event {
    /// Das Event als `canonical_json`-Zahlwert OHNE `event_id` (`store.py:33`: `payload = {k: v
    /// for k, v in event.items() if k != "event_id"}`). Serialisierung kann nur scheitern, wenn
    /// `wert` einen nicht-endlichen Float traegt — s. Moduldoku `canonical.rs`.
    fn payload_ohne_event_id(&self) -> serde_json::Value {
        serde_json::json!({
            "ts": self.ts,
            "feld_id": self.feld_id,
            "wert": self.wert,
            "zustand": self.zustand,
            "herkunft": self.herkunft,
            "schreiber": self.schreiber,
            "signal": self.signal,
            "ersetzt": self.ersetzt,
        })
    }

    /// Der content-adressierte `event_id` ueber die aktuellen Feldwerte (ohne `event_id`
    /// selbst). Zum Nachrechnen/Verifizieren; die Konstruktion via [`crate::store::Store::append`]
    /// setzt ihn bereits korrekt.
    #[must_use]
    pub fn berechne_event_id(&self) -> EventId {
        EventId::von_json(&self.payload_ohne_event_id())
    }
}

/// Eingabe fuer [`crate::store::Store::append`]. `feldzustand: Feldzustand` statt getrennter
/// `zustand`/`signal_2`-Felder macht "bestaetigt ohne Beleg" im Typsystem unrepresentierbar
/// (ersetzt Pythons Laufzeitpruefung `zustand=="bestaetigt" and not signal_2`, `store.py:365f`).
#[derive(Debug, Clone)]
pub struct NeuesEvent {
    pub feld_id: String,
    pub wert: serde_json::Value,
    pub feldzustand: Feldzustand,
    pub herkunft: Herkunft,
    pub schreiber: Schreiber,
    pub signal_1: Option<serde_json::Value>,
    pub ersetzt: Option<EventId>,
    /// `None` -> `_now()` beim Anhaengen (Tests uebergeben einen festen Zeitstempel).
    pub ts: Option<String>,
}

impl NeuesEvent {
    #[must_use]
    pub fn zustand(&self) -> Zustand {
        self.feldzustand.zustand()
    }

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
            wert: json!(220),
            zustand: Zustand::Bestaetigt,
            herkunft,
            schreiber: "ui:laie".parse().unwrap(),
            signal: Some(Signal { signal_1: None, signal_2: Some("klick@ui".to_string()) }),
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
}
