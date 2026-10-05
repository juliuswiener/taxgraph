//! Verhalten der Crate `intervall` gegen das PYTHON-Orakel, hermetisch.
//!
//! `rust/fixtures/intervall_orakel.json` haelt Szenarien (synthetische Bindung in Python-Form, Snapshot,
//! Deckel, Feldwerte) samt den Antworten von `produkt/unsicherheit/intervall.py`, erzeugt von
//! `tools/parity/extract_intervall_orakel.py`. Dieser Test spielt dieselben Szenarien gegen `intervall`:
//! `AchsenBindung::from` auf allen echten Bindungen, `intervall(...)` und `bescheid_via_slots(...)`.
//! Beide Seiten rechnen ueber dieselbe SYNTHETISCHE Engine: Summe ueber `gewicht(name) * zahl(wert)` mit
//! `gewicht = (Summe der UTF-8-Bytes) mod 7 - 3`; `zahl` = int, bool 0/1, String-Laenge in Codepoints,
//! sonst 0 (`tools/parity/oracle_konsistenz.py::_synth`). Ohne Python zur Laufzeit; die Paritaet mit echten
//! Fall-Dateien bleibt `rust/parity/tests/intervall_paritaet.rs` (PARITY=1).
//!
//! Warum das noetig ist: die Tests im Crate vergleichen `intervall` mit einer Neuformulierung desselben
//! Autors (`nachgerechnet`) und pruefen Invarianten; was Python an Grenzen tut (Deckel, Mittelpunkt,
//! Gleichstand, Slot-Summen), steht nur dort, wo ein Zufallsfall es trifft (Mutationslauf 2026-10-04,
//! `berichte/konsistenz-intervall-mutation.md`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::OnceLock;

use bindung::{Bindung, SlotBeitrag};
use domain::{Achsenwert, Cent, Euro, Feldtyp, Herkunft, PruefTiefe, PyWert, Zustand};
use intervall::{
    bescheid_via_slots, intervall, AchsenBindung, IntervallErgebnis, IntervallFehler, SlotFehler,
    Spanne, Werte, CAP_DEFAULT,
};
use serde_json::{json, Value};
use store::SnapshotFeld;

const FIXTURE: &str = include_str!("../../fixtures/intervall_orakel.json");

/// Faellt eine Zahl, wurde das Fixture gekuerzt -- dann waere der Test gruen, weil er weniger prueft.
///
/// Die 368 Felder sind `produkt/bindung` (Pythons Eingabe, eingefroren). Die Tests laden sie mit
/// `lade_registry` auf diesem Verzeichnis, nicht mit `lade_registry_der_wurzel`: ein Feld in
/// `rust/bindung/daten` (Weg B voll, 2026-10-05) aendert diese Zahl nicht. Faellt mit dem Fixture
/// weg (Stufe 2).
const N_SICHT: usize = 368;
const N_IV: usize = 287;
const N_SLOTS: usize = 106;

type Felder = BTreeMap<String, SnapshotFeld>;

fn fixture() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    CELL.get_or_init(|| serde_json::from_str(FIXTURE).expect("Fixture ist JSON"))
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

// ---------------------------------------------------------------------------- synthetische Engine

fn zahl(v: &PyWert) -> i128 {
    match v {
        PyWert::Bool(b) => i128::from(*b),
        PyWert::Ganz(n) => i128::from(*n),
        PyWert::Text(s) => i128::try_from(s.chars().count()).unwrap(),
        _ => 0,
    }
}

fn gewicht(name: &str) -> i128 {
    i128::from(name.bytes().map(u64::from).sum::<u64>() % 7) - 3
}

fn synth<'a>(paare: impl Iterator<Item = (&'a str, &'a PyWert)>) -> i64 {
    i64::try_from(paare.map(|(k, v)| gewicht(k) * zahl(v)).sum::<i128>()).expect("synth in i64")
}

#[allow(clippy::unnecessary_wraps)] // Signatur von `intervall()` vorgegeben
fn bescheid(w: &Werte) -> Result<Cent, Infallible> {
    Ok(Cent::new(synth(w.iter())))
}

// ---------------------------------------------------------------------------- Fixture -> Rust-Eingabe

/// Python-Eintrag (`TR.lade_bindung()`-Form) -> die Sicht, die `intervall` liest.
fn achse(e: &Value) -> AchsenBindung {
    let typ = match e["typ"].as_str().unwrap() {
        "bool" => Feldtyp::Bool,
        "enum" => Feldtyp::Enum,
        "int" => Feldtyp::Int,
        "cent" => Feldtyp::Cent,
        "text" => Feldtyp::Text,
        "datum" => Feldtyp::Datum,
        t => panic!("Typ {t}"),
    };
    AchsenBindung {
        feld_id: e["feld_id"].as_str().unwrap().to_owned(),
        typ,
        askable: e["askable"].as_bool().unwrap(),
        enum_werte: e
            .get("enum_werte")
            .map(|a| {
                liste(a)
                    .iter()
                    .map(|x| x.as_str().unwrap().to_owned())
                    .collect()
            })
            .unwrap_or_default(),
        bereich: e
            .get("bereich")
            .map(|r| (r["min"].as_i64().unwrap(), r["max"].as_i64().unwrap())),
        signatur_slot: e["quelle"]
            .get("signatur_slot")
            .map(|s| s.as_str().unwrap().to_owned()),
        slot_beitrag: if e.get("slot_beitrag").and_then(Value::as_str) == Some("summand") {
            SlotBeitrag::Summand
        } else {
            SlotBeitrag::Exakt
        },
    }
}

fn felder(snap: &Value) -> Felder {
    snap.as_object()
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
        .collect()
}

/// Antwort des Orakels: `{"ok": x}` -> `x`, `{"err": e}` -> `{"err": e}` (wie die Rust-Seite sie bildet).
fn orakel(erg: &Value) -> Value {
    erg.get("ok")
        .cloned()
        .unwrap_or_else(|| json!({"err": erg["err"]}))
}

// ---------------------------------------------------------------------------- Rust-Ergebnis -> Python-Form

fn iv_json(r: &Result<IntervallErgebnis, IntervallFehler<Infallible>>) -> Value {
    let r = match r {
        Ok(r) => r,
        Err(IntervallFehler::LeereAchse(_)) => return json!({"err": "ValueError"}),
        Err(e) => return json!({"err": format!("rust: {e}")}),
    };
    let iv = &r.intervall;
    let (min, max, offen) = match iv.spanne {
        Spanne::NichtFixierbar => (Value::Null, Value::Null, true),
        Spanne::Zahl { min, max, offen } => (min.get().into(), max.get().into(), offen),
    };
    json!({
        "basis_snapshot": r.basis_snapshot,
        "intervall": {"min_cent": min, "max_cent": max, "min_offen": offen, "max_offen": offen,
            "gedeckelt": iv.gedeckelt, "exakt_bzgl_top_k": iv.exakt_bzgl_top_k, "rest_felder": iv.rest_felder,
            "offene_achsen": iv.offene_achsen, "nicht_fixierbar": iv.nicht_fixierbar},
        "beitraege": r.beitraege.iter().map(|b| json!({"feld_id": b.feld_id, "spanne_cent": b.spanne.get(),
            "min_cent": b.min.get(), "max_cent": b.max.get()})).collect::<Vec<_>>(),
    })
}

fn slots_json(r: Result<Cent, SlotFehler<Infallible>>) -> Value {
    match r {
        Ok(c) => c.get().into(),
        Err(SlotFehler::UnbekanntesFeld(_)) => json!({"err": "KeyError"}),
        Err(SlotFehler::SummandNichtGanzzahl(_)) => json!({"err": "TypeError"}),
        Err(e) => json!({"err": format!("rust: {e}")}),
    }
}

/// `bescheid_via_slots` fuer beide Einheiten (`festzusetzende_est` = Euro, sonst Cent).
fn via_slots_rust(bindung: &[AchsenBindung], werte: &Werte, quantitaet: &str) -> Value {
    // `Slots` ist `BTreeMap<String, PyWert>`; `synth` will `&str`, also entpackt der Aufruf.
    let paare = |s: &intervall::Slots| -> Vec<(String, PyWert)> {
        s.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    };
    if quantitaet == "festzusetzende_est" {
        let f = bescheid_via_slots(bindung, |s| {
            let p = paare(s);
            Ok::<_, Infallible>(Euro::new(synth(p.iter().map(|(k, v)| (k.as_str(), v)))))
        });
        slots_json(f(werte))
    } else {
        let f = bescheid_via_slots(bindung, |s| {
            let p = paare(s);
            Ok::<_, Infallible>(Cent::new(synth(p.iter().map(|(k, v)| (k.as_str(), v)))))
        });
        slots_json(f(werte))
    }
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
                if s.len() > 600 {
                    format!("{}...", s.chars().take(600).collect::<String>())
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

// ---------------------------------------------------------------------------- Tests

/// Ein gekuerztes Fixture ist kein gruener Test: Fallzahlen, und jeder Pfad kommt vor (sonst belegte
/// "0 Abweichungen" nichts).
#[test]
fn fixture_unverkuerzt() {
    let f = fixture();
    assert_eq!(liste(&f["sicht"]).len(), N_SICHT, "Sichten");
    assert_eq!(liste(&f["iv"]).len(), N_IV, "intervall-Szenarien");
    assert_eq!(liste(&f["slots"]).len(), N_SLOTS, "Slot-Szenarien");
    let iv = liste(&f["iv"]);
    let ok = |s: &Value| s["erg"].get("ok").cloned();
    let mit_zahl = |s: &Value| ok(s).is_some_and(|o| !o["intervall"]["min_cent"].is_null());
    assert!(iv.iter().filter(|s| mit_zahl(s)).count() >= 150);
    assert!(
        iv.iter()
            .filter(|s| ok(s).is_some_and(|o| o["intervall"]["min_cent"].is_null()))
            .count()
            >= 8,
        "nicht fixierbar"
    );
    assert!(iv
        .iter()
        .any(|s| ok(s).is_some_and(|o| o["intervall"]["gedeckelt"] == true)));
    assert!(iv.iter().any(|s| ok(s).is_some_and(
        |o| o["intervall"]["min_offen"] == true && !o["intervall"]["min_cent"].is_null()
    )));
    assert!(
        iv.iter().any(|s| s["erg"].get("err").is_some()),
        "ValueError"
    );
    assert!(iv.iter().any(|s| s["sid"].is_string()));
    let sl = liste(&f["slots"]);
    for fehler in ["KeyError", "TypeError"] {
        assert!(
            sl.iter().any(|s| s["erg"]["err"] == fehler),
            "kein Slot-Szenario mit {fehler}"
        );
    }
    assert!(sl.iter().any(|s| s["erg"]["ok"].is_i64()));
}

#[test]
fn sicht_wie_orakel() {
    let reg = bindung::lade_registry(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung"),
    )
    .expect("registry");
    let bindungen: Vec<Bindung> = reg
        .dateien
        .into_iter()
        .flat_map(|(_, d)| d.bindungen)
        .collect();
    let rust: Value = bindungen
        .iter()
        .map(|b| {
            let a = AchsenBindung::from(b);
            json!({"feld_id": a.feld_id, "typ": a.typ.als_str(), "askable": a.askable, "enum_werte": a.enum_werte,
                "bereich": a.bereich.map(|(lo, hi)| json!([lo, hi])), "signatur_slot": a.signatur_slot,
                "summand": a.slot_beitrag == SlotBeitrag::Summand})
        })
        .collect();
    let py = &fixture()["sicht"];
    assert_eq!(rust.as_array().unwrap().len(), liste(py).len());
    let mut abw = Abweichungen::default();
    for (r, p) in liste(&rust).iter().zip(liste(py)) {
        abw.gleich(r["feld_id"].as_str().unwrap(), r, p);
    }
    abw.pruefen("AchsenBindung::from", bindungen.len());
}

#[test]
fn intervall_wie_orakel() {
    let mut abw = Abweichungen::default();
    for s in liste(&fixture()["iv"]) {
        let achsen: Vec<AchsenBindung> = liste(&s["bindung"]).iter().map(achse).collect();
        let cap = match &s["cap"] {
            Value::String(t) if t == "default" => CAP_DEFAULT,
            c => usize::try_from(c.as_u64().unwrap()).unwrap(),
        };
        let rust = iv_json(&intervall(
            &felder(&s["snap"]),
            &achsen,
            bescheid,
            cap,
            s["sid"].as_str(),
        ));
        abw.gleich(s["n"].as_str().unwrap(), &rust, &orakel(&s["erg"]));
    }
    abw.pruefen("intervall", liste(&fixture()["iv"]).len());
}

#[test]
fn via_slots_wie_orakel() {
    let mut abw = Abweichungen::default();
    for s in liste(&fixture()["slots"]) {
        let achsen: Vec<AchsenBindung> = liste(&s["bindung"]).iter().map(achse).collect();
        let mut werte = Werte::neu();
        for p in liste(&s["werte"]) {
            werte.setze(p[0].as_str().unwrap(), PyWert::from(p[1].clone()));
        }
        let rust = via_slots_rust(&achsen, &werte, s["q"].as_str().unwrap());
        abw.gleich(s["n"].as_str().unwrap(), &rust, &orakel(&s["erg"]));
    }
    abw.pruefen("bescheid_via_slots", liste(&fixture()["slots"]).len());
}
