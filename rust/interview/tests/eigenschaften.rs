//! Eigenschaften der Crate `interview` ohne Orakel, gegen die echte Bindung.
//!
//! 1. Vorlaeufig = unbeantwortet: ein vorlaeufiges Event (Herkunft nicht `vorjahr`) auf einem
//!    Basisfeld aendert die Queue nicht.
//! 2. Die Queue enthaelt nur askable Felder der Sicht, jedes hoechstens einmal.
//! 3. Eine Bedingung schliesst erst aus, wenn JEDE Instanz bestaetigt abweicht.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use bindung::Registry;
use domain::{Achsenwert, Herkunft, PruefTiefe, Schreiber, Zustand};
use interview::{Antwort, Bedingungsstand, Graph};
use proptest::prelude::*;
use serde_json::{json, Value};
use store::{Event, EventId, Store, StoreDatei, Veranlagungsjahr};

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| interview::doctest_registry().expect("registry"))
}

fn graph() -> &'static Graph<'static> {
    static CELL: OnceLock<Graph<'static>> = OnceLock::new();
    CELL.get_or_init(|| Graph::aus_registry(registry()))
}

fn event(i: usize, feld_id: &str, wert: Value, zustand: Zustand) -> Event {
    let mut e = Event {
        event_id: EventId::aus_bytes([0; 32]),
        ts: format!("2026-09-29T10:00:{:02}Z", i % 60),
        feld_id: feld_id.to_owned(),
        wert,
        zustand,
        herkunft: Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        }
        .into(),
        schreiber: Schreiber::Mensch("julius".to_owned()),
        signal: None,
        ersetzt: None,
    };
    e.event_id = e.berechne_event_id();
    e
}

fn store(events: Vec<Event>) -> Store {
    Store::aus_datei(StoreDatei {
        version: 1,
        veranlagungszeitraum: Veranlagungsjahr(2025),
        fall_id: None,
        scheibe: None,
        user_id: None,
        events,
        snapshots: Vec::new(),
        vorjahr_referenz: None,
    })
}

fn queue(s: &Store) -> Vec<&'static str> {
    let ohne: Option<&HashMap<String, i64>> = None;
    interview::naechste_fragen(s, graph().alle(), graph(), ohne)
}

/// Ein Store aus bestaetigten Antworten (Werte aus einer kleinen, typ-gemischten Menge).
fn basis_events() -> impl Strategy<Value = Vec<(usize, usize, bool)>> {
    proptest::collection::vec((0..usize::MAX, 0..6usize, any::<bool>()), 0..25)
}

fn wert(k: usize) -> Value {
    [json!(true), json!(false), json!(0), json!(2), json!("zusammen"), json!("keine")][k % 6].clone()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn vorlaeufig_aendert_die_queue_nicht(basis in basis_events(), ziel in 0..usize::MAX, w in 0..6usize) {
        let felder: Vec<&str> = graph().alle().feld_ids().collect();
        let events: Vec<Event> = basis
            .iter()
            .enumerate()
            .map(|(i, (f, k, best))| {
                let z = if *best { Zustand::Bestaetigt } else { Zustand::Vorlaeufig };
                event(i, felder[f % felder.len()], wert(*k), z)
            })
            .collect();
        let feld = felder[ziel % felder.len()];
        // Nur auf ein Feld OHNE aktives Event (sonst waere es ein Ersatz, kein Zusatz).
        prop_assume!(!events.iter().any(|e| e.feld_id == feld));
        let vorher = queue(&store(events.clone()));
        let mut mit = events;
        mit.push(event(99, feld, wert(w), Zustand::Vorlaeufig));
        prop_assert_eq!(vorher, queue(&store(mit)));
    }

    #[test]
    fn queue_nur_askable_ohne_doppel(basis in basis_events()) {
        let felder: Vec<&str> = graph().alle().feld_ids().collect();
        let events: Vec<Event> = basis
            .iter()
            .enumerate()
            .map(|(i, (f, k, best))| {
                let z = if *best { Zustand::Bestaetigt } else { Zustand::Vorlaeufig };
                event(i, felder[f % felder.len()], wert(*k), z)
            })
            .collect();
        let q = queue(&store(events));
        let mut gesehen = HashSet::new();
        for f in &q {
            prop_assert!(graph().alle().get(f).is_some_and(|b| b.askable), "{f} nicht askable");
            prop_assert!(gesehen.insert(*f), "{f} doppelt");
        }
    }

    #[test]
    fn ausschluss_nur_wenn_jede_instanz_bestaetigt_abweicht(stand in proptest::collection::vec(0..3u8, 1..6)) {
        let (ja, nein) = (json!(true), json!(false));
        let antworten: Vec<Antwort> = stand
            .iter()
            .map(|s| match s {
                0 => Antwort::Offen,
                1 => Antwort::Bestaetigt(&ja),
                _ => Antwort::Bestaetigt(&nein),
            })
            .collect();
        let ergebnis = Bedingungsstand::aus(&antworten, |w| *w == Value::Bool(false));
        let erwartet = if stand.contains(&0) {
            Bedingungsstand::Offen
        } else if stand.iter().all(|s| *s == 2) {
            Bedingungsstand::Ausgeschlossen
        } else {
            Bedingungsstand::Erfuellt
        };
        prop_assert_eq!(ergebnis, erwartet);
    }
}
