//! Verhalten der Crate `konsistenz` gegen das PYTHON-Orakel, hermetisch.
//!
//! `rust/fixtures/konsistenz_orakel.json` haelt Snapshots (Feld -> Wert, bestaetigt/vorlaeufig), Scheibe und
//! Vorjahr samt den Antworten von `produkt/konsistenz/*` (`preflight(...)`, `unvollstaendige_instanzen`); eingefroren,
//! der Erzeuger ist geloescht (Verlauf: `git show 2dd056a6:tools/parity/extract_konsistenz_orakel.py`). Dieser Test spielt dieselben Snapshots gegen `konsistenz`
//! und vergleicht je oeffentliche Funktion: `flag_widersprueche`, `partner_ohne_zusammen`,
//! `alleinerziehend_mit_zusammen`, `pauschal_hinweise`, `nicht_gerechnete_angaben`,
//! `plausibilitaets_widersprueche`, `unvollstaendige_instanzen`, `vorlaeufige_ring_betraege`, `preflight`,
//! dazu `eur` und die fuenf Tabellen. Ohne Python zur Laufzeit; die Paritaet mit echten Fall-Dateien bleibt
//! `rust/parity/tests/konsistenz_paritaet.rs` (PARITY=1).
//!
//! Warum das noetig ist: die Tests im Crate pruefen Hilfsfunktionen gegen ihre Vor-K2-Fassung und die
//! KiSt-Schwelle, aber kaum eine oeffentliche Pruefung gegen Werte. Ein gekippter Vergleich in
//! `plausibilitaets_widersprueche`, `vorlaeufige_ring_betraege` oder in der Ampel liess alle Tests gruen
//! (Mutationslauf 2026-10-04, `berichte/konsistenz-intervall-mutation.md`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::HashSet;
use std::sync::OnceLock;

use domain::{Achsenwert, Herkunft, PruefTiefe, PyWert, Zustand};
use interview::Graph;
use konsistenz as k;
use serde_json::{json, Value};
use store::SnapshotFeld;

const FIXTURE: &str = include_str!("../../fixtures/konsistenz_orakel.json");

/// Szenarien im Fixture. Faellt die Zahl, wurde das Fixture gekuerzt -- dann waere der Test gruen, weil er
/// weniger prueft.
const N_SZENARIEN: usize = 1369;
const N_EUR: usize = 37;

fn fixture() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    CELL.get_or_init(|| serde_json::from_str(FIXTURE).expect("Fixture ist JSON"))
}

fn graph() -> &'static Graph<'static> {
    static CELL: OnceLock<Graph<'static>> = OnceLock::new();
    CELL.get_or_init(|| {
        // Die Registry lebt so lange wie der Testprozess.
        let registry = Box::leak(Box::new(
            interview::python_orakel_registry().expect("Registry laedt"),
        ));
        Graph::aus_registry(registry)
    })
}

fn liste(v: &Value) -> &Vec<Value> {
    v.as_array().expect("Liste")
}

fn herkunft() -> Herkunft {
    let a = Achsenwert::new("parity").unwrap();
    Herkunft {
        herkunft: a.clone(),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: a,
    }
}

// ---------------------------------------------------------------------------- Szenario -> Rust-Eingabe

struct Fall {
    name: String,
    felder: k::Felder,
    scheibe: Option<HashSet<String>>,
    vorjahr: Option<i64>,
    /// Antwort des Orakels, lange Texte aufgeloest.
    pf: Value,
    inst: Value,
}

/// `{"$t": i}` -> `texte[i]` (der Generator legt lange Texte einmal ab).
fn aufloesen(v: &Value, texte: &[Value]) -> Value {
    match v {
        Value::Object(o) if o.len() == 1 && o.contains_key("$t") => {
            texte[usize::try_from(o["$t"].as_u64().unwrap()).unwrap()].clone()
        }
        Value::Object(o) => o
            .iter()
            .map(|(k, x)| (k.clone(), aufloesen(x, texte)))
            .collect(),
        Value::Array(a) => a.iter().map(|x| aufloesen(x, texte)).collect(),
        andere => andere.clone(),
    }
}

/// `vorjahr_referenz` -> der Ganzzahl-Parameter von `plausibilitaets_widersprueche`
/// (`preflight.py`: `if vorjahr_referenz:` und `(... or {}).get("wert")`, int ohne bool).
fn vorjahr_wert(v: Option<&Value>) -> Option<i64> {
    v?.get("verlustvortrag_bestand")?.get("wert")?.as_i64()
}

fn faelle() -> &'static Vec<Fall> {
    static CELL: OnceLock<Vec<Fall>> = OnceLock::new();
    CELL.get_or_init(|| {
        let texte = liste(&fixture()["texte"]);
        liste(&fixture()["szenarien"])
            .iter()
            .map(|s| {
                let felder: k::Felder = s["snap"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(feld, e)| {
                        let zustand = match e[1].as_str().unwrap() {
                            "b" => Zustand::Bestaetigt,
                            "v" => Zustand::Vorlaeufig,
                            z => panic!("Zustand {z}"),
                        };
                        (
                            feld.clone(),
                            SnapshotFeld {
                                wert: PyWert::from(e[0].clone()),
                                zustand,
                                herkunft: herkunft().into(),
                            },
                        )
                    })
                    .collect();
                Fall {
                    name: s["n"].as_str().unwrap().to_owned(),
                    felder,
                    scheibe: s["scheibe"]
                        .as_array()
                        .map(|a| a.iter().map(|f| f.as_str().unwrap().to_owned()).collect()),
                    vorjahr: vorjahr_wert(Some(&s["vorjahr"]).filter(|v| !v.is_null())),
                    pf: aufloesen(&s["pf"], texte),
                    inst: aufloesen(&s["inst"], texte),
                }
            })
            .collect()
    })
}

// ---------------------------------------------------------------------------- Rust-Ergebnis -> Python-Form

fn wert_json(w: &PyWert) -> Value {
    w.zu_json().expect("Store-Wert ist darstellbar (Auflage 3)")
}

fn flag_json(v: &[k::FlagWiderspruch]) -> Value {
    v.iter()
        .map(|w| json!({"flag": w.flag, "feld_id": w.feld_id, "wert": wert_json(&w.wert), "grund": w.grund}))
        .collect()
}

fn partner_json(v: &[k::PartnerWiderspruch]) -> Value {
    v.iter()
        .map(|w| json!({"feld_id": w.feld_id, "wert": wert_json(&w.wert), "veranlagung": wert_json(&w.veranlagung), "grund": w.grund}))
        .collect()
}

fn pauschal_json(v: &[k::PauschalHinweis]) -> Value {
    v.iter()
        .map(|h| {
            json!({"check_id": h.check_id, "label": h.label, "hinweis": h.hinweis,
                "ausloeser_felder": h.ausloeser_felder.iter().map(|(f, w)| json!({"feld_id": f, "wert": wert_json(w)})).collect::<Vec<_>>(),
                "fehlende_felder": h.fehlende_felder})
        })
        .collect()
}

fn nicht_gerechnet_json(v: &[k::NichtGerechnet]) -> Value {
    v.iter()
        .map(|n| json!({"feld_id": n.feld_id, "hinweis": n.hinweis}))
        .collect()
}

fn plausi_json(v: &[k::PlausiWiderspruch]) -> Value {
    v.iter()
        .map(|w| {
            let mut o = json!({"feld_id": w.feld_id, "wert": wert_json(&w.wert), "grund": w.grund});
            if let Some(b) = w.bezug {
                o["bezug"] = b.into();
            }
            o
        })
        .collect()
}

fn vorlaeufig_json(v: &[k::VorlaeufigerBetrag]) -> Value {
    v.iter()
        .map(|h| json!({"feld_id": h.feld_id, "wert": h.wert, "hinweis": h.hinweis}))
        .collect()
}

/// Der Generator legt nur nicht-leere Schluessel ab (und `status`).
fn kompakt(v: Value) -> Value {
    let Value::Object(o) = v else { return v };
    Value::Object(
        o.into_iter()
            .filter(|(k, x)| k == "status" || x.as_array().is_none_or(|a| !a.is_empty()))
            .collect(),
    )
}

fn preflight_json(e: &k::PreflightErgebnis) -> Value {
    kompakt(json!({
        "widersprueche_flag": flag_json(&e.widersprueche_flag),
        "widersprueche_partner": partner_json(&e.widersprueche_partner),
        "widersprueche_alleinerziehend": partner_json(&e.widersprueche_alleinerziehend),
        "widersprueche_plausibilitaet": plausi_json(&e.widersprueche_plausibilitaet),
        "hinweise_pauschalen": pauschal_json(&e.hinweise_pauschalen),
        "hinweise_nicht_gerechnet": nicht_gerechnet_json(&e.hinweise_nicht_gerechnet),
        "hinweise_betrag_vorlaeufig": vorlaeufig_json(&e.hinweise_betrag_vorlaeufig),
        "status": e.status.als_str(),
    }))
}

// ---------------------------------------------------------------------------- Vergleich

/// Sammelt Abweichungen; der Test schlaegt am Ende mit ALLEN fehl, nicht beim ersten.
#[derive(Default)]
struct Abweichungen(Vec<String>);

impl Abweichungen {
    fn gleich(&mut self, kontext: &str, rust: &Value, orakel: &Value) {
        if rust != orakel {
            let kurz = |v: &Value| {
                let s = v.to_string();
                if s.len() > 700 {
                    format!("{}...", s.chars().take(700).collect::<String>())
                } else {
                    s
                }
            };
            self.0.push(format!(
                "{kontext}\n    Rust:   {}\n    Orakel: {}",
                kurz(rust),
                kurz(orakel)
            ));
        }
    }

    fn pruefen(self, was: &str, n: usize) {
        assert!(
            self.0.is_empty(),
            "{was}: {} von {n} Pruefungen weichen vom Python-Orakel ab:\n{}",
            self.0.len(),
            self.0
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Der Schluessel `key` aus der Orakel-Antwort; fehlt er, war die Liste leer.
fn orakel(f: &Fall, key: &str) -> Value {
    f.pf.get(key).cloned().unwrap_or_else(|| json!([]))
}

// ---------------------------------------------------------------------------- Tests

/// Ein gekuerztes Fixture ist kein gruener Test: Fallzahl, und jede Pruefung sieht mindestens einen
/// nicht-leeren Fall (sonst belegte "0 Abweichungen" nichts).
#[test]
fn fixture_unverkuerzt() {
    assert_eq!(faelle().len(), N_SZENARIEN, "Szenarien im Fixture");
    assert_eq!(liste(&fixture()["eur"]).len(), N_EUR);
    let zaehle = |key: &str| faelle().iter().filter(|f| f.pf.get(key).is_some()).count();
    for key in [
        "widersprueche_flag",
        "widersprueche_partner",
        "widersprueche_alleinerziehend",
        "widersprueche_plausibilitaet",
        "hinweise_pauschalen",
        "hinweise_betrag_vorlaeufig",
    ] {
        assert!(
            zaehle(key) >= 10,
            "{key}: nur {} nicht-leere Faelle",
            zaehle(key)
        );
    }
    assert!(faelle()
        .iter()
        .any(|f| f.inst.as_array().is_some_and(|a| !a.is_empty())));
    for status in ["RED", "AMBER", "GREEN"] {
        assert!(
            faelle().iter().any(|f| f.pf["status"] == status),
            "kein Szenario mit Status {status}"
        );
    }
    // nicht_gerechnete_angaben ist leer per Bauart (NICHT_GERECHNET = []), auch im Orakel.
    assert_eq!(zaehle("hinweise_nicht_gerechnet"), 0);
}

#[test]
fn konstanten_wie_orakel() {
    let py = &fixture()["konstanten"];
    let flag: Value = k::FLAG_NEGIERT.iter().map(|(f, l)| json!([f, l])).collect();
    assert_eq!(flag, py["flag_negiert"], "FLAG_NEGIERT");
    assert_eq!(json!(k::PARTNER_FELDER), py["partner_felder"]);
    assert_eq!(
        json!(k::RING_BETRAGSFELDER.as_slice()),
        py["ring_betragsfelder"]
    );
    let ng: Value = k::NICHT_GERECHNET
        .iter()
        .map(|(f, t)| json!([f, t]))
        .collect();
    assert_eq!(ng, py["nicht_gerechnet"]);
    let pc: Value = k::PAUSCHAL_CHECKS
        .iter()
        .map(|c| json!({"id": c.id, "ausloeser_felder": c.ausloeser_felder, "pauschal_felder": c.pauschal_felder,
            "label": c.label, "hinweis": c.hinweis}))
        .collect();
    assert_eq!(pc, py["pauschal_checks"], "PAUSCHAL_CHECKS");
    // `nur_wenn_alle_leer` ersetzt Pythons `check["id"] == "vv_wk"`:
    assert!(k::PAUSCHAL_CHECKS
        .iter()
        .all(|c| c.nur_wenn_alle_leer == (c.id == "vv_wk")));
}

#[test]
fn eur_wie_orakel() {
    let mut abw = Abweichungen::default();
    let eintraege = liste(&fixture()["eur"]);
    for e in eintraege {
        let cent = e[0].as_i64().unwrap();
        abw.gleich(
            &format!("eur({cent})"),
            &json!(k::eur(cent)),
            &json!(e[1].as_str().unwrap()),
        );
    }
    abw.pruefen("eur", eintraege.len());
}

#[test]
fn flag_wie_orakel() {
    let mut abw = Abweichungen::default();
    for f in faelle() {
        abw.gleich(
            &f.name,
            &flag_json(&k::flag_widersprueche(&f.felder, f.scheibe.as_ref())),
            &orakel(f, "widersprueche_flag"),
        );
    }
    abw.pruefen("flag_widersprueche", faelle().len());
}

#[test]
fn partner_wie_orakel() {
    let mut abw = Abweichungen::default();
    for f in faelle() {
        abw.gleich(
            &format!("{}: partner_ohne_zusammen", f.name),
            &partner_json(&k::partner_ohne_zusammen(&f.felder)),
            &orakel(f, "widersprueche_partner"),
        );
        abw.gleich(
            &format!("{}: alleinerziehend_mit_zusammen", f.name),
            &partner_json(&k::alleinerziehend_mit_zusammen(&f.felder)),
            &orakel(f, "widersprueche_alleinerziehend"),
        );
    }
    abw.pruefen("partner", faelle().len());
}

#[test]
fn pauschalen_wie_orakel() {
    let mut abw = Abweichungen::default();
    for f in faelle() {
        abw.gleich(
            &format!("{}: pauschal_hinweise", f.name),
            &pauschal_json(&k::pauschal_hinweise(&f.felder)),
            &orakel(f, "hinweise_pauschalen"),
        );
        abw.gleich(
            &format!("{}: nicht_gerechnete_angaben", f.name),
            &nicht_gerechnet_json(&k::nicht_gerechnete_angaben(&f.felder)),
            &orakel(f, "hinweise_nicht_gerechnet"),
        );
    }
    abw.pruefen("pauschalen", faelle().len());
}

#[test]
fn plausibilitaet_wie_orakel() {
    let mut abw = Abweichungen::default();
    for f in faelle() {
        abw.gleich(
            &format!("{}: plausibilitaets_widersprueche", f.name),
            &plausi_json(&k::plausibilitaets_widersprueche(
                &f.felder,
                f.vorjahr,
                graph(),
            )),
            &orakel(f, "widersprueche_plausibilitaet"),
        );
        abw.gleich(
            &format!("{}: unvollstaendige_instanzen", f.name),
            &plausi_json(&k::unvollstaendige_instanzen(&f.felder, graph())),
            &f.inst,
        );
    }
    abw.pruefen("plausibilitaet", faelle().len());
}

#[test]
fn vorlaeufige_betraege_wie_orakel() {
    let mut abw = Abweichungen::default();
    for f in faelle() {
        abw.gleich(
            &f.name,
            &vorlaeufig_json(&k::vorlaeufige_ring_betraege(&f.felder, graph())),
            &orakel(f, "hinweise_betrag_vorlaeufig"),
        );
    }
    abw.pruefen("vorlaeufige_ring_betraege", faelle().len());
}

#[test]
fn preflight_wie_orakel() {
    let mut abw = Abweichungen::default();
    for f in faelle() {
        let rust = preflight_json(&k::preflight(
            &f.felder,
            f.scheibe.as_ref(),
            f.vorjahr,
            graph(),
        ));
        abw.gleich(&f.name, &rust, &f.pf);
    }
    abw.pruefen("preflight", faelle().len());
}
