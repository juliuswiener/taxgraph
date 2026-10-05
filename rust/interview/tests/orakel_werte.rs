//! Verhalten der Crate `interview` gegen das PYTHON-Orakel, hermetisch.
//!
//! `rust/fixtures/interview_orakel.json` haelt Szenarien (Events + Sicht) und die Antworten von
//! `produkt/traverser/traverser.py` bzw. `produkt/haut/bindung_rollen.py`, erzeugt von
//! `tools/parity/extract_interview_orakel.py`. Dieser Test spielt dieselben Szenarien gegen
//! `interview` und vergleicht: Frageliste (mit und ohne Unsicherheits-Beitrag), Relevanz je Regel,
//! Instanz-Zahl und fehlende Instanzen, Gate-Gewicht je Sicht, `justification`/`trace_ergebnis`,
//! Kegel und Achsen-Bindung. Ohne Python zur Laufzeit; die Paritaet mit echten Fall-Dateien
//! bleibt `rust/parity/tests/interview_paritaet.rs` (PARITY=1).
//!
//! Warum das noetig ist: `eigenschaften.rs` prueft Invarianten, aber nicht, WELCHE Frage an welcher
//! Stelle steht. Ein gekippter Vergleich in der Ordnung, im Gate-Gewicht oder in der
//! Instanz-Zaehlung liess alle Tests gruen (Mutationslauf 2026-10-04, `berichte/interview-mutation.md`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use bindung::{Ableitung, Bindung, Bindungspunkt, Registry, Vorjahr};
use domain::{Achsenwert, Feldtyp, Herkunft, Kz, PruefTiefe, Schreiber, Zustand};
use interview::{Graph, Sicht};
use serde_json::Value;
use store::{Event, EventId, Signal, Store, StoreDatei, Veranlagungsjahr};

const FIXTURE: &str = include_str!("../../fixtures/interview_orakel.json");

/// Szenarien im Fixture. Faellt die Zahl, wurde das Fixture gekuerzt -- dann waere der Test gruen,
/// weil er weniger prueft.
///
/// Das Fixture ist Pythons Antwort ueber die gemeinsame Bindung `produkt/bindung`;
/// `interview::doctest_registry` laedt genau sie. Felder in `rust/bindung/felder` (Weg B leicht,
/// 2026-10-05) kennt dieser Test nicht und verschieben keine der drei Zahlen.
const N_SZENARIEN: usize = 83;
const N_SICHTEN: usize = 13;
const N_ROLLEN: usize = 5;

fn fixture() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    CELL.get_or_init(|| serde_json::from_str(FIXTURE).expect("Fixture ist JSON"))
}

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| interview::doctest_registry().expect("registry"))
}

fn graph() -> &'static Graph<'static> {
    static CELL: OnceLock<Graph<'static>> = OnceLock::new();
    CELL.get_or_init(|| Graph::aus_registry(registry()))
}

fn echte(feld: &str) -> &'static Bindung {
    graph()
        .alle()
        .get(feld)
        .unwrap_or_else(|| panic!("{feld} nicht in der Registry"))
}

fn text<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

fn liste(v: &Value) -> &Vec<Value> {
    v.as_array().expect("Liste")
}

fn strings(v: &Value) -> Vec<String> {
    liste(v)
        .iter()
        .map(|x| x.as_str().expect("String").to_owned())
        .collect()
}

// ---------------------------------------------------------------------------- Sichten

/// Ein Feld, das es in der echten Bindung nicht gibt: Klon einer Vorlage, Eigenschaften aus der Spec.
fn synth(spec: &Value) -> Bindung {
    let mut b = echte("veranlagung").clone();
    text(spec, "id").expect("id").clone_into(&mut b.feld_id);
    text(spec, "regel")
        .expect("regel")
        .clone_into(&mut b.quelle.regel_id);
    b.quelle.bindungspunkt = match spec.get("punkt").map(liste) {
        Some(p) if p[0] == "gelt" => {
            Bindungspunkt::Geltungsbedingung(p[1].as_str().unwrap().to_owned())
        }
        Some(p) => Bindungspunkt::SignaturSlot(p[1].as_str().unwrap().to_owned()),
        None => Bindungspunkt::SignaturSlot(format!("s_{}", b.feld_id)),
    };
    b.askable = spec.get("askable").and_then(Value::as_bool).unwrap_or(true);
    b.gate = spec.get("gate").and_then(Value::as_bool);
    b.eingangsfrage = spec
        .get("eingang")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    b.typ = match text(spec, "typ").unwrap_or("bool") {
        "bool" => Feldtyp::Bool,
        "int" => Feldtyp::Int,
        "cent" => Feldtyp::Cent,
        "text" => Feldtyp::Text,
        anders => panic!("Typ {anders}"),
    };
    b.elster_kz = text(spec, "kz").map(|k| Kz::new(k).unwrap());
    b.vorjahr = match text(spec, "vorjahr") {
        Some("uebernehmbar") => Some(Vorjahr::Uebernehmbar),
        Some("vorschlag") => Some(Vorjahr::Vorschlag),
        _ => None,
    };
    b.instanz_gruppe = None;
    b.feld_bedingung = None;
    b.ableitung = spec.get("ableitung").map(|a| {
        let mut x: Ableitung = echte("kind_unter_14_haushaltszugehoerig")
            .ableitung
            .clone()
            .expect("Vorlage");
        text(a, "aus").unwrap().clone_into(&mut x.aus);
        x.und_feld = text(a, "und").map(str::to_owned);
        x
    });
    b
}

struct SichtDaten {
    bindungen: Vec<Bindung>,
    gate_gewicht: BTreeMap<String, usize>,
    relevanz_leer: BTreeMap<String, Rel>,
}

type Rel = (String, Vec<String>, Vec<String>);

fn rel_aus(v: &Value) -> Rel {
    let a = liste(v);
    (
        a[0].as_str().unwrap().to_owned(),
        strings(&a[1]),
        strings(&a[2]),
    )
}

fn sichten() -> &'static HashMap<String, SichtDaten> {
    static CELL: OnceLock<HashMap<String, SichtDaten>> = OnceLock::new();
    CELL.get_or_init(|| {
        liste(&fixture()["sichten"])
            .iter()
            .map(|s| {
                let bindungen: Vec<Bindung> = match s.get("felder") {
                    Some(Value::Array(fs)) => fs
                        .iter()
                        .map(|f| echte(f.as_str().unwrap()).clone())
                        .collect(),
                    Some(Value::Object(o)) => liste(&o["synth"]).iter().map(synth).collect(),
                    _ => graph().alle().iter().cloned().collect(),
                };
                let gewicht = s["gate_gewicht"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, n)| (k.clone(), usize::try_from(n.as_u64().unwrap()).unwrap()))
                    .collect();
                let leer = s["relevanz_leer"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), rel_aus(v)))
                    .collect();
                (
                    text(s, "id").unwrap().to_owned(),
                    SichtDaten {
                        bindungen,
                        gate_gewicht: gewicht,
                        relevanz_leer: leer,
                    },
                )
            })
            .collect()
    })
}

fn sicht(id: &str) -> Sicht<'static> {
    Sicht::aus(sichten()[id].bindungen.iter())
}

// ---------------------------------------------------------------------------- Stores

fn event(
    i: usize,
    feld_id: &str,
    wert: Value,
    zustand: Zustand,
    herkunft: &str,
    signal: bool,
) -> Event {
    let mut e = Event {
        event_id: EventId::aus_bytes([0; 32]),
        ts: format!("2026-09-29T10:{:02}:{:02}Z", (i / 60) % 60, i % 60),
        feld_id: feld_id.to_owned(),
        wert: wert.into(),
        zustand,
        herkunft: Herkunft {
            herkunft: Achsenwert::new(herkunft).unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        }
        .into(),
        schreiber: Schreiber::Mensch("julius".to_owned()),
        signal: signal.then(|| Signal {
            signal_1: Some(None),
            signal_2: Some(format!("ok@{feld_id}")),
            signal_2_fehlt: false,
        }),
        ersetzt: None,
    };
    e.event_id = e.berechne_event_id().expect("Testwert ist darstellbar");
    e
}

fn store_aus(events: &Value) -> Store {
    let events = liste(events)
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let e = liste(e);
            let zustand = if e[2] == "b" {
                Zustand::Bestaetigt
            } else {
                Zustand::Vorlaeufig
            };
            let herkunft = e.get(3).and_then(Value::as_str).unwrap_or("laie");
            let signal = e.get(4).and_then(Value::as_bool).unwrap_or(false);
            event(
                i,
                e[0].as_str().unwrap(),
                e[1].clone(),
                zustand,
                herkunft,
                signal,
            )
        })
        .collect();
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

/// Deterministischer Unsicherheits-Beitrag, wie im Generator: `(Position % 7) * 1000`; jede vierte
/// Position (`n % 4 == 3`) fehlt in der Map. Fehlend gilt wie 0.
fn beitrag(s: &Sicht<'_>) -> HashMap<String, i64> {
    s.feld_ids()
        .enumerate()
        .filter(|(n, _)| n % 4 != 3)
        .map(|(n, f)| (f.to_owned(), i64::try_from(n % 7).unwrap() * 1000))
        .collect()
}

// ---------------------------------------------------------------------------- Vergleich

/// Sammelt Abweichungen; der Test schlaegt am Ende mit ALLEN fehl, nicht beim ersten.
#[derive(Default)]
struct Abweichungen(Vec<String>);

impl Abweichungen {
    fn gleich<T: PartialEq + std::fmt::Debug>(&mut self, kontext: &str, rust: &T, orakel: &T) {
        if rust != orakel {
            self.0.push(format!(
                "{kontext}\n    Rust:   {rust:?}\n    Orakel: {orakel:?}"
            ));
        }
    }

    /// Frageliste: nennt die erste abweichende Position mit ihrer Umgebung.
    fn liste(&mut self, kontext: &str, rust: &[&str], orakel: &[&str]) {
        if rust == orakel {
            return;
        }
        let pos = rust
            .iter()
            .zip(orakel)
            .position(|(a, b)| a != b)
            .unwrap_or(rust.len().min(orakel.len()));
        let fenster = |l: &[&str]| -> Vec<String> {
            l[pos.saturating_sub(2)..(pos + 3).min(l.len())]
                .iter()
                .map(|f| (*f).to_owned())
                .collect()
        };
        self.0.push(format!(
            "{kontext}: erste Abweichung an Position {pos} (Laenge Rust {}, Orakel {})\n    Rust:   {:?}\n    Orakel: {:?}",
            rust.len(), orakel.len(), fenster(rust), fenster(orakel)
        ));
    }

    fn pruefen(self, was: &str, n: usize) {
        assert!(
            self.0.is_empty(),
            "{was}: {} von {n} Pruefungen weichen vom Python-Orakel ab:\n{}",
            self.0.len(),
            self.0
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

fn szenarien() -> &'static Vec<Value> {
    liste(&fixture()["szenarien"])
}

fn felder_von(s: &Sicht<'static>) -> Vec<&'static str> {
    s.feld_ids().collect()
}

// ---------------------------------------------------------------------------- Tests

/// Ein gekuerztes Fixture ist kein gruener Test.
#[test]
fn fixture_unverkuerzt() {
    assert_eq!(szenarien().len(), N_SZENARIEN);
    assert_eq!(sichten().len(), N_SICHTEN);
    assert_eq!(liste(&fixture()["rollen"]).len(), N_ROLLEN);
    assert_eq!(
        liste(&fixture()["universum"]).len(),
        graph().alle().len(),
        "Registry und Fixture haben verschiedene Felder"
    );
    // Das Fixture folgt der Reihenfolge der Registry: Dateien alphabetisch, in der Datei wie geschrieben.
    let rust: Vec<&str> = graph().alle().feld_ids().collect();
    let orakel: Vec<&str> = liste(&fixture()["universum"])
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert_eq!(rust, orakel);
}

/// `naechste_fragen` ohne und mit Unsicherheits-Beitrag: dieselbe Liste in derselben Reihenfolge.
#[test]
fn warteschlange_wie_orakel() {
    let mut abw = Abweichungen::default();
    let mut n = 0;
    for s in szenarien() {
        let name = text(s, "name").unwrap();
        let sicht = sicht(text(s, "sicht").unwrap());
        let felder = felder_von(&sicht);
        let store = store_aus(&s["events"]);
        let ohne: Option<&HashMap<String, i64>> = None;
        let erwartet = |schluessel: &str| -> Option<Vec<&'static str>> {
            s.get(schluessel).map(|q| {
                liste(q)
                    .iter()
                    .map(|i| felder[usize::try_from(i.as_u64().unwrap()).unwrap()])
                    .collect()
            })
        };
        abw.liste(
            &format!("{name} / queue"),
            &interview::naechste_fragen(&store, &sicht, graph(), ohne),
            &erwartet("queue").unwrap(),
        );
        n += 1;
        if let Some(q) = erwartet("queue_b") {
            let b = beitrag(&sicht);
            abw.liste(
                &format!("{name} / queue_b"),
                &interview::naechste_fragen(&store, &sicht, graph(), Some(&b)),
                &q,
            );
            n += 1;
        }
        // Python `if beitrag:` -- eine LEERE Map gilt wie keine (Klasse "wert" bleibt nach Kennzahl sortiert).
        if let Some(q) = erwartet("queue_e") {
            let leer = HashMap::new();
            abw.liste(
                &format!("{name} / queue_e"),
                &interview::naechste_fragen(&store, &sicht, graph(), Some(&leer)),
                &q,
            );
            n += 1;
        }
    }
    abw.pruefen("Frageliste", n);
}

/// `relevanz`: Status, offene Gates und offene Annahmen je Regel.
#[test]
fn relevanz_wie_orakel() {
    let mut abw = Abweichungen::default();
    for s in szenarien() {
        let name = text(s, "name").unwrap();
        let daten = &sichten()[text(s, "sicht").unwrap()];
        let sicht = sicht(text(s, "sicht").unwrap());
        let store = store_aus(&s["events"]);
        let mut erwartet = daten.relevanz_leer.clone();
        for (regel, v) in s["relevanz_abweichung"].as_object().unwrap() {
            erwartet.insert(regel.clone(), rel_aus(v));
        }
        let rust: BTreeMap<String, Rel> = interview::relevanz(&store, &sicht, graph())
            .into_iter()
            .map(|(r, v)| {
                let status = serde_json::to_value(v.status)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned();
                (
                    r.to_owned(),
                    (
                        status,
                        v.gates_offen.iter().map(|x| (*x).to_owned()).collect(),
                        v.annahmen_offen.iter().map(|x| (*x).to_owned()).collect(),
                    ),
                )
            })
            .collect();
        let keys_rust: Vec<_> = rust.keys().cloned().collect();
        let keys_orakel: Vec<_> = erwartet.keys().cloned().collect();
        abw.gleich(&format!("{name} / Regelmenge"), &keys_rust, &keys_orakel);
        for (regel, r) in &rust {
            if let Some(o) = erwartet.get(regel) {
                abw.gleich(&format!("{name} / relevanz[{regel}]"), r, o);
            }
        }
    }
    abw.pruefen("Relevanz", szenarien().len());
}

/// `instanz_anzahl` (bestaetigte Zahl, gekappt, Etikett) und `fehlende_instanzen`.
#[test]
fn instanzen_wie_orakel() {
    let mut abw = Abweichungen::default();
    for s in szenarien() {
        let name = text(s, "name").unwrap();
        let sicht = sicht(text(s, "sicht").unwrap());
        let store = store_aus(&s["events"]);
        for (feld, erwartet) in s["instanz_anzahl"].as_object().unwrap() {
            let (n, etikett) = interview::instanz_anzahl(&store, &sicht, graph(), feld);
            let rust = (u64::from(n.get()), etikett.to_owned());
            let orakel = (
                erwartet[0].as_u64().unwrap(),
                erwartet[1].as_str().unwrap().to_owned(),
            );
            abw.gleich(&format!("{name} / instanz_anzahl[{feld}]"), &rust, &orakel);
        }
        let (felder, _) = store.materialisiere(None).unwrap();
        let rust =
            serde_json::to_value(interview::fehlende_instanzen(&felder, &sicht, graph())).unwrap();
        abw.gleich(
            &format!("{name} / fehlende_instanzen"),
            &rust,
            &s["fehlende"],
        );
    }
    abw.pruefen("Instanzen", szenarien().len());
}

/// `gate_gewicht` je Sicht: wie viele Fragen die Antwort auf ein Gate abschaltet.
#[test]
fn gate_gewicht_wie_orakel() {
    let mut abw = Abweichungen::default();
    for (id, daten) in sichten() {
        let rust: BTreeMap<String, usize> = interview::gate_gewicht(&sicht(id), graph())
            .into_iter()
            .map(|(f, n)| (f.to_owned(), n))
            .collect();
        abw.gleich(&format!("gate_gewicht[{id}]"), &rust, &daten.gate_gewicht);
    }
    abw.pruefen("Gate-Gewicht", sichten().len());
}

/// `justification` und `trace_ergebnis`: Regel, Slot, Geltungsbedingung, Anker -- ohne `event_id`
/// (Python und Rust bilden sie aus demselben Inhalt, das prueft `event_id` im Store-Crate).
#[test]
fn beweis_wie_orakel() {
    let mut abw = Abweichungen::default();
    let mut n = 0;
    for s in szenarien() {
        let Some(js) = s.get("justification") else {
            continue;
        };
        let name = text(s, "name").unwrap();
        let sicht = sicht(text(s, "sicht").unwrap());
        let store = store_aus(&s["events"]);
        for (feld, erwartet) in js.as_object().unwrap() {
            let rust = interview::justification(&store, feld, &sicht).map_or(Value::Null, |j| {
                let mut v = serde_json::to_value(j).unwrap();
                v.as_object_mut().unwrap().remove("event_id");
                v
            });
            abw.gleich(&format!("{name} / justification[{feld}]"), &rust, erwartet);
            n += 1;
        }
        let t = interview::trace_ergebnis(&store, &sicht, Some("snap"));
        let regeln: BTreeMap<String, Vec<String>> = t
            .regeln
            .iter()
            .map(|(r, js)| {
                (
                    (*r).to_owned(),
                    js.iter().map(|j| j.feld_id.to_owned()).collect(),
                )
            })
            .collect();
        let orakel: BTreeMap<String, Vec<String>> = s["trace"]["regeln"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(r, fs)| (r.clone(), strings(fs)))
            .collect();
        abw.gleich(&format!("{name} / trace.regeln"), &regeln, &orakel);
        abw.gleich(
            &format!("{name} / trace.basis_snapshot"),
            &t.basis_snapshot.map(str::to_owned),
            &Some("snap".to_owned()),
        );
        n += 2;
    }
    assert!(n > 0, "kein Beweis-Szenario im Fixture");
    abw.pruefen("Beweis", n);
}

/// `relevante_kegel_felder` und `ring_bindung` (Kegel der Spannen-Achsen).
#[test]
fn rollen_wie_orakel() {
    let mut abw = Abweichungen::default();
    let voll = graph().alle();
    for r in liste(&fixture()["rollen"]) {
        let name = text(r, "name").unwrap();
        let kegel_s = strings(&r["kegel"]);
        let kegel: Vec<&str> = kegel_s.iter().map(String::as_str).collect();
        let store = store_aus(&r["events"]);
        let store = r["mit_store"].as_bool().unwrap().then_some(&store);
        let relevante = interview::relevante_kegel_felder(&kegel, voll, store, graph());
        abw.gleich(
            &format!("{name} / relevante_kegel_felder"),
            &relevante
                .iter()
                .map(|f| (*f).to_owned())
                .collect::<Vec<_>>(),
            &strings(&r["relevante"]),
        );
        let ring = interview::ring_bindung(Some(kegel.as_slice()), voll, store, graph());
        abw.gleich(
            &format!("{name} / ring_bindung"),
            &ring.0.feld_ids().map(str::to_owned).collect::<Vec<_>>(),
            &strings(&r["ring"]),
        );
        // `rollen`: Aufbau bleibt die volle Bindung, nur die Achsen folgen dem Kegel.
        let (aufbau, achsen) = interview::rollen(Some(kegel.as_slice()), voll, store, graph());
        abw.gleich(
            &format!("{name} / rollen.aufbau"),
            &aufbau.0.len(),
            &voll.len(),
        );
        abw.gleich(
            &format!("{name} / rollen.achsen"),
            &achsen.0.feld_ids().map(str::to_owned).collect::<Vec<_>>(),
            &strings(&r["ring"]),
        );
    }
    abw.pruefen("Rollen", N_ROLLEN);
}
