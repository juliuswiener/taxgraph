//! Eigenschaften der Crate `interview` ohne Orakel, gegen die echte Bindung.
//!
//! 1. Vorlaeufig = unbeantwortet: ein vorlaeufiges Event (Herkunft nicht `vorjahr`) auf einem
//!    Basisfeld aendert die Queue nicht.
//! 2. Die Queue enthaelt nur askable Felder der Sicht, jedes hoechstens einmal.
//! 3. Eine Bedingung schliesst erst aus, wenn JEDE Instanz bestaetigt abweicht.
//! 4. Themen schichtweise nach Tiefe, gegen eine unabhaengige Formulierung von `_themen_folge`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

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
        wert: wert.into(),
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
    e.event_id = e.berechne_event_id().expect("Testwert ist darstellbar");
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
    [
        json!(true),
        json!(false),
        json!(0),
        json!(2),
        json!("zusammen"),
        json!("keine"),
    ][k % 6]
        .clone()
}

/// Thema eines Felds der Sicht (`quelle.regel_id`).
fn thema(f: &str) -> &'static str {
    graph()
        .alle()
        .get(f)
        .map_or("", |b| b.quelle.regel_id.as_str())
}

/// Voraussetzungen je Thema der Queue nach `traverser.py:650-683`: Feld einer Regelbedingung,
/// `ableitung.aus`, Zaehlfeld der Instanz-Gruppe. Nur Themen, die selbst in der Queue stehen.
fn voraussetzungen(q: &[&'static str]) -> HashMap<&'static str, HashSet<&'static str>> {
    let da: HashSet<&str> = q.iter().map(|f| thema(f)).collect();
    let mut vor: HashMap<&'static str, HashSet<&'static str>> = HashMap::new();
    for &f in q {
        let (t, b) = (thema(f), graph().alle().get(f).unwrap());
        let quellen = graph()
            .regel_bedingungen(t)
            .iter()
            .map(|c| c.feld.as_str())
            .chain(b.ableitung.as_ref().map(|a| a.aus.as_str()))
            .chain(
                b.instanz_gruppe
                    .as_deref()
                    .and_then(|g| graph().instanz_gruppe(g))
                    .map(|g| g.anzahl_feld.as_str()),
            );
        for v in quellen.map(thema) {
            if !v.is_empty() && v != t && da.contains(v) {
                vor.entry(t).or_default().insert(v);
            }
        }
    }
    vor
}

/// Laengste Kette von Voraussetzungen, rekursiv statt in Runden. Ein Ring zaehlt unendlich.
fn tiefe(
    t: &'static str,
    vor: &HashMap<&'static str, HashSet<&'static str>>,
    pfad: &mut Vec<&'static str>,
) -> usize {
    if pfad.contains(&t) {
        return usize::MAX;
    }
    pfad.push(t);
    let d = vor
        .get(t)
        .into_iter()
        .flatten()
        .map(|v| tiefe(v, vor, pfad).saturating_add(1))
        .max()
        .unwrap_or(0);
    pfad.pop();
    d
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
        let (ja, nein) = (domain::PyWert::Bool(true), domain::PyWert::Bool(false));
        let antworten: Vec<Antwort> = stand
            .iter()
            .map(|s| match s {
                0 => Antwort::Offen,
                1 => Antwort::Bestaetigt(&ja),
                _ => Antwort::Bestaetigt(&nein),
            })
            .collect();
        let ergebnis = Bedingungsstand::aus(&antworten, |w| *w == domain::PyWert::Bool(false));
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

proptest! {
    #![proptest_config(ProptestConfig { cases: 1_000, failure_persistence: None, ..ProptestConfig::default() })]

    /// `_themen_folge` (`traverser.py:723-733`) setzt in Runde k genau die Themen der Tiefe k, den
    /// Ring-Rest zuletzt. Also steht jedes Thema am Stueck, und die Tiefe faellt entlang der Queue
    /// nie. Daraus folgt: jedes Thema steht hinter allen seinen Voraussetzungen.
    #[test]
    fn themen_schichtweise_nach_tiefe(basis in basis_events()) {
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
        let mut folge: Vec<&'static str> = q.iter().map(|f| thema(f)).collect();
        folge.dedup();
        let vor = voraussetzungen(&q);
        let tiefen: Vec<usize> = folge.iter().map(|t| tiefe(t, &vor, &mut Vec::new())).collect();
        prop_assert_eq!(folge.iter().collect::<HashSet<_>>().len(), folge.len(), "Thema zerteilt: {:?}", folge);
        prop_assert!(
            tiefen.windows(2).all(|w| w[0] <= w[1]),
            "Tiefe faellt: {:?}",
            folge.iter().zip(&tiefen).collect::<Vec<_>>()
        );
    }
}
