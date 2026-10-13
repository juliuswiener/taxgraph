//! § 24a `EStG`, Rueckfall bei offenem Geburtsdatum: die Bindung fragt das GEBURTSJAHR, nicht ein Feld, das niemand setzen kann.
//! Gegenstueck zu `tests/test_p24a_rueckfall_ist_geburtsjahr.py`, gegen die echten Daten (`rust/bindung/daten`).
//!
//! **Worum es geht.** Der Altersentlastungsbetrag steht zu, wer vor Beginn des Jahres 64 war. `rentner_alter_64_erfuellt` wird aus
//! dem Geburtsdatum BERECHNET und nie gefragt; fehlt das Geburtsdatum, tragen die Frage nach `geburtsjahr` und die Regel den
//! Rueckfall. Stand die Bindung einmal falsch (das abgeleitete Feld als fragbarer Rueckfall), blieb die Regel nach der Antwort
//! auf `geburtsjahr` fuer immer "unentschieden" und das tote Feld in der Frageliste (Julius 2026-10-03, Form B). Bei Fall und Zahl
//! gab es keinen Unterschied, nur die Erklaerung und die Fragenliste waren falsch; deshalb prueft der Test die Bindungsform und das
//! Verhalten von `relevanz` und `naechste_fragen`.
//!
//! Die Zahl selbst (Altersentlastungsbetrag mit Geburtsjahr gegen ohne) prueft
//! `rust/bescheid/tests/altersentlastung_sonstige_hermetisch.rs`; die Ableitung Geburtsdatum -> Feld und Jahr prueft
//! `rust/store/tests/entscheidungsstellen.rs`. Hier steht, was dazwischen haengt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::sync::OnceLock;

use bindung::{Bindung, Registry};
use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, PyWert, Schreiber, Signal2};
use interview::{Graph, Regelstatus};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEvent, Store};

const FELD: &str = "rentner_alter_64_erfuellt";
const REGEL: &str = "p24a_altersentlastungsbetrag";

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| interview::doctest_registry().expect("Registry laedt"))
}

fn graph() -> &'static Graph<'static> {
    static CELL: OnceLock<Graph<'static>> = OnceLock::new();
    CELL.get_or_init(|| Graph::aus_registry(registry()))
}

fn bindungen() -> &'static Vec<Bindung> {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel)
            .expect("Registry laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn bindung(feld: &str) -> &'static Bindung {
    bindungen().iter().find(|b| b.feld_id == feld).unwrap_or_else(|| panic!("{feld} ist nicht gebunden"))
}

/// Ein Store im vollen Bestand, in den die bestaetigten Antworten ueber den echten Schreibweg gehen (die Ableitung feuert dort).
fn store_mit(antworten: &[(&str, Value)]) -> Store {
    let karte: HashMap<String, &'static Bindung> = store::baue_nachschlag(bindungen());
    let mut s = Store::leer(2025, None);
    for (feld, wert) in antworten {
        let neu = NeuesEvent {
            feld_id: (*feld).to_owned(),
            wert: wert.clone().into(),
            feldzustand: Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() },
            herkunft: Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            },
            schreiber: Schreiber::Mensch("laie".to_owned()),
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-08-27T12:00:00+00:00".to_owned()),
        };
        s.append(&neu, None, BindungNachschlag::neu(&karte)).unwrap();
    }
    s
}

fn regel(s: &Store) -> interview::RegelRelevanz<'static> {
    interview::relevanz(s, graph().alle(), graph()).remove(REGEL).expect("p24a steht in der Relevanz")
}

fn queue(s: &Store) -> Vec<&'static str> {
    let ohne: Option<&HashMap<String, i64>> = None;
    interview::naechste_fragen(s, graph().alle(), graph(), ohne)
}

#[test]
fn das_rueckfallfeld_ist_abgeleitet_nicht_gefragt_und_die_erklaerung_nennt_das_geburtsjahr() {
    let b = bindung(FELD);
    assert_eq!(b.quelle.regel_id, REGEL, "das Rueckfallfeld haengt an einer anderen Regel als seine Regelbedingung");
    assert!(!b.askable, "das Feld laesst sich in keiner Scheibe setzen: nicht askable");
    assert!(!b.eingangsfrage, "das Feld ist keine Eingangsfrage");
    let ableitung = b.ableitung.as_ref().expect("die Ableitung bleibt");
    assert_eq!(ableitung.aus, "stammdaten_geburtsdatum");
    assert!(ableitung.grund.contains("geburtsjahr"), "die Erklaerung der Ableitung nennt den Rueckfall nicht");
    let bedingungen: Vec<_> = graph().regel_bedingungen(REGEL).iter().filter(|c| c.feld == FELD).collect();
    assert_eq!(bedingungen.len(), 1, "genau eine Regelbedingung auf {FELD}");
    assert!(bedingungen[0].grund.contains("geburtsjahr"), "die Erklaerung der Regelbedingung nennt den Rueckfall nicht");
    assert!(bindung("geburtsjahr").askable, "der Rueckfall muss fragbar sein");
}

#[test]
fn jahrgang_unter_64_schliesst_die_regel_aus_und_1958_laesst_sie_gelten() {
    let jung = regel(&store_mit(&[("stammdaten_geburtsdatum", json!("01.01.1975"))]));
    assert_eq!(jung.status, Regelstatus::Ausgeschlossen);
    let alt = regel(&store_mit(&[("stammdaten_geburtsdatum", json!("01.01.1958"))]));
    assert_eq!(alt.status, Regelstatus::Relevant);
    assert!(alt.gates_offen.is_empty(), "offene Gates trotz Geburtsdatum: {:?}", alt.gates_offen);
}

#[test]
fn ohne_angabe_ist_nur_geburtsjahr_ein_offenes_gate_und_das_feld_nicht_in_der_queue() {
    let s = store_mit(&[]);
    let rel = regel(&s);
    assert_eq!(rel.status, Regelstatus::Unentschieden);
    assert_eq!(rel.gates_offen, vec!["geburtsjahr"], "vorher: geburtsjahr UND {FELD}, ein Gate ohne Antwortweg");
    assert_eq!(rel.annahmen_offen, vec!["alter_64_vor_bezugsjahr_vollendet"], "die abgeleitete Voraussetzung bleibt eine offene Annahme der Regel");
    let q = queue(&s);
    assert!(q.contains(&"geburtsjahr"));
    assert!(!q.contains(&FELD), "das abgeleitete Feld darf nie gefragt werden");
}

#[test]
fn die_antwort_auf_geburtsjahr_allein_entscheidet_die_regel() {
    for jahr in [1958, 1990] {
        let s = store_mit(&[("geburtsjahr", json!(jahr))]);
        let rel = regel(&s);
        assert_eq!(rel.status, Regelstatus::Relevant, "Geburtsjahr {jahr}: die Regel blieb offen");
        assert!(rel.gates_offen.is_empty(), "Geburtsjahr {jahr}: offene Gates {:?}", rel.gates_offen);
        assert!(!queue(&s).contains(&FELD), "Geburtsjahr {jahr}: das tote Feld steht in der Queue");
        assert!(s.aktives(FELD).is_none(), "ohne Geburtsdatum darf es keine Ableitung geben");
    }
}

#[test]
fn die_ableitung_aus_dem_geburtsdatum_feuert_obwohl_das_feld_nicht_fragbar_ist() {
    for (datum, erwartet) in [("01.01.1958", true), ("01.01.1975", false)] {
        let s = store_mit(&[("stammdaten_geburtsdatum", json!(datum))]);
        let e = s.aktives(FELD).unwrap_or_else(|| panic!("{datum}: keine Ableitung auf {FELD}"));
        assert_eq!(e.wert, PyWert::Bool(erwartet), "{datum}");
        assert_eq!(s.aktives("geburtsjahr").unwrap().wert, PyWert::Ganz(i64::from(datum[6..].parse::<i32>().unwrap())), "{datum}");
    }
}
