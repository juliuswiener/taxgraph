//! Parität `rust/intervall` gegen `produkt/unsicherheit/intervall.py` (über
//! `tools/parity/oracle_konsistenz.py`). EIN Orakel-Prozess für die ganze Binary.
//!
//! Die echte Bescheid-Funktion kommt mit der Crate `bescheid`. Beide Seiten rechnen deshalb über
//! dieselbe SYNTHETISCHE reine Funktion: Summe über `gewicht(name) · zahl(wert)` mit
//! `gewicht = (Σ UTF-8-Bytes des Namens) mod 7 − 3` und `zahl` = int, bool 0/1, String-Länge in
//! Codepoints, sonst 0. Für `bescheid_via_slots` dieselbe Summe über die Slots; Einheit über die
//! Quantität (`festzusetzende_est` ↔ `Euro`, `gewst_cent` ↔ `Cent`).
//!
//! Tests: `sicht_gleich` (`AchsenBindung::from` gegen Pythons Rohlesung aller Bindungen),
//! `reale_faelle`, `generierte_faelle` (1000), `negativkontrolle`.
//!
//! SICHERHEIT: reale Fälle sind echte Steuerdaten. Nur lokal lesen, nur Zählwerte ausgeben.
//!
//!   `PARITY`=1 `cargo` test -p parity --test `intervall_paritaet` -- --nocapture
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, HashSet};
use std::convert::Infallible;
use std::sync::{Mutex, OnceLock};

use bindung::{Bindung, SlotBeitrag};
use domain::{Achsenwert, Cent, Euro, Feldtyp, Herkunft, PruefTiefe, Zustand};
use intervall::{
    bescheid_via_slots, intervall, AchsenBindung, IntervallErgebnis, IntervallFehler, SlotFehler,
    Spanne, Werte,
};
use parity::Oracle;
use proptest::prelude::*;
use serde_json::{json, Value};
use store::SnapshotFeld;

type Felder = BTreeMap<String, SnapshotFeld>;

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn faelle_verzeichnis() -> std::path::PathBuf {
    let home = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default());
    match std::env::var("TAXGRAPH_DATEN") {
        Ok(p) if !p.trim().is_empty() => std::path::PathBuf::from(p.trim()).join("faelle"),
        _ => match std::env::var("XDG_DATA_HOME") {
            Ok(x) if !x.trim().is_empty() => {
                std::path::PathBuf::from(x.trim()).join("taxgraph/faelle")
            }
            _ => home.join(".local/share/taxgraph/faelle"),
        },
    }
}

fn walk_json(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in read.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk_json(&p));
        } else if p.extension().is_some_and(|x| x == "json") {
            out.push(p);
        }
    }
    out.sort();
    out
}

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let reg = bindung::lade_registry(&repo_root().join("produkt/bindung")).expect("registry");
        reg.dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn oracle() -> &'static Mutex<Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
}

fn frage(anfrage: &Value) -> Value {
    let antwort = oracle()
        .lock()
        .unwrap()
        .call_json(anfrage)
        .expect("orakel antwortet");
    antwort
        .get("ok")
        .cloned()
        .unwrap_or_else(|| json!({ "err": antwort["err"] }))
}

fn herkunft() -> Herkunft {
    let a = Achsenwert::new("parity").unwrap();
    Herkunft {
        herkunft: a.clone(),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: a,
    }
}

// ------------------------------------------------------------------ synthetische Engine

fn zahl(v: &Value) -> i128 {
    match v {
        Value::Bool(b) => i128::from(*b),
        Value::Number(n) => n.as_i64().map_or(0, i128::from),
        Value::String(s) => i128::try_from(s.chars().count()).unwrap(),
        _ => 0,
    }
}

fn gewicht(name: &str) -> i128 {
    i128::from(name.bytes().map(u64::from).sum::<u64>() % 7) - 3
}

fn synth<'a>(paare: impl Iterator<Item = (&'a str, &'a Value)>) -> i64 {
    i64::try_from(paare.map(|(k, v)| gewicht(k) * zahl(v)).sum::<i128>()).expect("synth in i64")
}

#[allow(clippy::unnecessary_wraps)] // Signatur von `intervall()` vorgegeben
fn bescheid(w: &Werte) -> Result<Cent, Infallible> {
    Ok(Cent::new(synth(w.iter())))
}

// ------------------------------------------------------------------ Rust-Ergebnis → Python-Form

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

/// `bescheid_via_slots` für beide Einheiten, Rust-Seite.
fn via_slots_rust(bindung: &[AchsenBindung], werte: &Werte, quantitaet: &str) -> Value {
    if quantitaet == "festzusetzende_est" {
        let f = bescheid_via_slots(bindung, |s| {
            Ok::<_, Infallible>(Euro::new(synth(s.iter().map(|(k, v)| (k.as_str(), v)))))
        });
        slots_json(f(werte))
    } else {
        let f = bescheid_via_slots(bindung, |s| {
            Ok::<_, Infallible>(Cent::new(synth(s.iter().map(|(k, v)| (k.as_str(), v)))))
        });
        slots_json(f(werte))
    }
}

// ------------------------------------------------------------------ Vergleich

#[derive(Default)]
struct Zaehler {
    faelle: usize,
    fehler: usize,
    /// Ergebnis mit Zahl (Intervall `min_cent` gesetzt bzw. Slot-Betrag).
    zahl: usize,
    diffs: usize,
}

type Bilanz = BTreeMap<&'static str, Zaehler>;

fn buche(bilanz: &mut Bilanz, name: &'static str, rust: &Value, py: &Value) {
    let z = bilanz.entry(name).or_default();
    z.faelle += 1;
    if rust.get("err").is_some() {
        z.fehler += 1;
    }
    if rust.is_i64() || rust["intervall"]["min_cent"].is_i64() {
        z.zahl += 1;
    }
    if rust != py {
        z.diffs += 1;
        if z.diffs <= 3 {
            eprintln!(
                "ABWEICHUNG {name}: rust_len={} py_len={}",
                rust.to_string().len(),
                py.to_string().len()
            );
        }
    }
}

fn berichte(titel: &str, bilanz: &Bilanz) -> usize {
    println!("== {titel}");
    let mut summe = 0;
    for (name, z) in bilanz {
        println!(
            "  {name:<34} faelle={:>6} fehler={:>5} zahl={:>5} diffs={}",
            z.faelle, z.fehler, z.zahl, z.diffs
        );
        summe += z.diffs;
    }
    summe
}

fn werte_json(w: &Werte) -> Value {
    w.iter().map(|(k, v)| json!([k, v])).collect()
}

// ------------------------------------------------------------------ Tests

#[test]
fn sicht_gleich() {
    if skip() {
        return;
    }
    let ids: Vec<&str> = bindungen().iter().map(|b| b.feld_id.as_str()).collect();
    let py = frage(&json!({"fn": "intervall.sicht", "bindung_ids": ids}));
    let rust: Value = bindungen()
        .iter()
        .map(|b| {
            let a = AchsenBindung::from(b);
            json!({"feld_id": a.feld_id, "typ": a.typ.als_str(), "askable": a.askable, "enum_werte": a.enum_werte,
                "bereich": a.bereich.map(|(lo, hi)| json!([lo, hi])), "signatur_slot": a.signatur_slot,
                "summand": a.slot_beitrag == SlotBeitrag::Summand})
        })
        .collect();
    assert_eq!(rust, py);
    println!("sicht: {} Bindungen gleich", ids.len());
}

fn pruefe_intervall(
    bilanz: &mut Bilanz,
    name: &'static str,
    felder: &Felder,
    auswahl: &[&Bindung],
    cap: usize,
) {
    let achsen: Vec<AchsenBindung> = auswahl.iter().map(|b| AchsenBindung::from(*b)).collect();
    let ids: Vec<&str> = auswahl.iter().map(|b| b.feld_id.as_str()).collect();
    let py = frage(
        &json!({"fn": "intervall.intervall", "snapshot": felder, "bindung_ids": ids, "cap": cap,
        "snapshot_id": "sid"}),
    );
    let rust = iv_json(&intervall(felder, &achsen, bescheid, cap, Some("sid")));
    buche(bilanz, name, &rust, &py);
}

#[test]
fn reale_faelle() {
    if skip() {
        return;
    }
    let alle_achsen: Vec<AchsenBindung> = bindungen().iter().map(AchsenBindung::from).collect();
    let alle_ids: Vec<&str> = bindungen().iter().map(|b| b.feld_id.as_str()).collect();
    let mut bilanz = Bilanz::new();
    let (mut dateien, mut n) = (0, 0);
    // P10: store::lade laedt inzwischen ALLE realen Faelle (legacy Herkunft, unbegrenzte VZ) —
    // keine rohe Fallback-Faltung mehr noetig.
    for pfad in walk_json(&faelle_verzeichnis()) {
        dateien += 1;
        let d =
            store::lade(&pfad).unwrap_or_else(|e| panic!("store::lade({}): {e}", pfad.display()));
        let (felder, _) = store::Store::aus_datei(d).materialisiere(None).unwrap();
        n += 1;
        // A: nur Felder mit Snapshot-Wert oder Bereich -> meist eine echte Zahl.
        let a: Vec<&Bindung> = bindungen()
            .iter()
            .filter(|b| {
                felder.contains_key(&b.feld_id)
                    || (matches!(b.typ, Feldtyp::Cent | Feldtyp::Int) && b.bereich.is_some())
            })
            .collect();
        for cap in [256, 8] {
            pruefe_intervall(
                &mut bilanz,
                if cap == 256 {
                    "intervall A cap256"
                } else {
                    "intervall A cap8"
                },
                &felder,
                &a,
                cap,
            );
        }
        // B: alle Bindungen (meist nicht fixierbar).
        let b: Vec<&Bindung> = bindungen().iter().collect();
        pruefe_intervall(&mut bilanz, "intervall B alle", &felder, &b, 256);
        // Slot-Adapter über die volle Bindung und alle Snapshot-Werte mit Bindung.
        let mut werte = Werte::neu();
        for (fid, f) in &felder {
            if alle_ids.contains(&fid.as_str()) {
                werte.setze(fid, f.wert.clone());
            }
        }
        for q in ["festzusetzende_est", "gewst_cent"] {
            let py = frage(
                &json!({"fn": "intervall.bescheid_via_slots", "bindung_ids": alle_ids, "quantitaet": q,
                "feld_werte": werte_json(&werte)}),
            );
            buche(
                &mut bilanz,
                if q == "gewst_cent" {
                    "via_slots cent"
                } else {
                    "via_slots euro"
                },
                &via_slots_rust(&alle_achsen, &werte, q),
                &py,
            );
        }
    }
    println!("reale Faelle: {dateien} Dateien, {n} Snapshots");
    assert!(n > 0, "keine realen Faelle gefunden");
    let zahl_a = bilanz["intervall A cap256"].faelle;
    assert_eq!(berichte("reale Faelle", &bilanz), 0);
    assert!(zahl_a > 0);
}

// ------------------------------------------------------------------ Generator

fn wert() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        (-60_i64..60).prop_map(Value::from),
        (-10_i64.pow(12)..10_i64.pow(12)).prop_map(Value::from),
        prop::sample::select(vec!["", "a", "bbb", "zusammen"])
            .prop_map(|s| Value::String(s.to_owned())),
    ]
}

/// Slot-Werte: überwiegend Ganzzahlen, damit der Erfolgspfad dominiert.
fn slot_wert() -> impl Strategy<Value = Value> {
    prop_oneof![
        1 => Just(Value::Null),
        2 => any::<bool>().prop_map(Value::Bool),
        5 => (-60_i64..60).prop_map(Value::from),
        2 => (-10_i64.pow(12)..10_i64.pow(12)).prop_map(Value::from),
        1 => prop::sample::select(vec!["", "a", "bbb"]).prop_map(|s| Value::String(s.to_owned())),
    ]
}

/// `(Rust-Sicht, Python-Eintrag)` je synthetischer Bindung.
fn achse(i: usize) -> impl Strategy<Value = (AchsenBindung, Value)> {
    let typ = prop::sample::select(vec![
        Feldtyp::Bool,
        Feldtyp::Enum,
        Feldtyp::Int,
        Feldtyp::Cent,
        Feldtyp::Text,
        Feldtyp::Datum,
    ]);
    let enum_werte =
        prop::collection::vec(prop::sample::select(vec!["a", "bb", "ccc", "dddd"]), 0..4);
    let bereich = prop::option::weighted(0.7, (-60_i64..60, -60_i64..60));
    // Slot: exakt-Pool s*, summand-Pool t* (getrennt: `str + str` ist außerhalb des Wertebereichs),
    // oder geltungsbedingung.
    let slot = prop::sample::select(vec![
        Some(("s1", false)),
        Some(("s2", false)),
        Some(("t1", true)),
        Some(("t2", true)),
        None,
    ]);
    (
        typ,
        enum_werte,
        bereich,
        slot,
        prop::bool::weighted(0.85),
        0_usize..4,
    )
        .prop_map(move |(typ, ew, bereich, slot, askable, suffix)| {
            let feld_id = format!("f{i}{}", "z".repeat(suffix));
            let ew: Vec<String> = ew.into_iter().map(str::to_owned).collect();
            let a = AchsenBindung {
                feld_id: feld_id.clone(),
                typ,
                askable,
                enum_werte: ew.clone(),
                bereich,
                signatur_slot: slot.map(|(s, _)| s.to_owned()),
                slot_beitrag: if slot.is_some_and(|(_, summand)| summand) {
                    SlotBeitrag::Summand
                } else {
                    SlotBeitrag::Exakt
                },
            };
            let mut py = json!({"feld_id": feld_id, "typ": typ.als_str(), "askable": askable,
            "quelle": match slot { Some((s, _)) => json!({"regel_id": "r", "signatur_slot": s}),
                                   None => json!({"regel_id": "r", "geltungsbedingung": "g"}) }});
            if !ew.is_empty() || typ == Feldtyp::Enum {
                py["enum_werte"] = json!(ew);
            }
            if let Some((lo, hi)) = bereich {
                py["bereich"] = json!({"min": lo, "max": hi});
            }
            if a.slot_beitrag == SlotBeitrag::Summand {
                py["slot_beitrag"] = json!("summand");
            }
            (a, py)
        })
}

type Fall = (Vec<(AchsenBindung, Value)>, Felder, usize, Werte);

fn fall() -> impl Strategy<Value = Fall> {
    (0_usize..8)
        .prop_flat_map(|n| {
            let achsen: Vec<_> = (0..n).map(achse).collect();
            let snap =
                prop::collection::vec(prop::option::of((wert(), prop::bool::weighted(0.4))), n);
            let fw = prop::collection::vec((any::<u8>(), slot_wert()), 0..8);
            (achsen, snap, 0_usize..300, fw)
        })
        .prop_map(|(achsen, snap, cap, fw)| {
            let mut felder = Felder::new();
            for ((a, _), s) in achsen.iter().zip(snap) {
                if let Some((w, bestaetigt)) = s {
                    let zustand = if bestaetigt {
                        Zustand::Bestaetigt
                    } else {
                        Zustand::Vorlaeufig
                    };
                    felder.insert(
                        a.feld_id.clone(),
                        SnapshotFeld {
                            wert: w,
                            zustand,
                            herkunft: herkunft().into(),
                        },
                    );
                }
            }
            // feld_werte fuer den Slot-Adapter: Index n = unbekanntes Feld (KeyError-Pfad).
            let mut werte = Werte::neu();
            for (u, w) in fw {
                let i = if u < 25 {
                    achsen.len()
                } else {
                    usize::from(u) % achsen.len().max(1)
                };
                let fid = achsen
                    .get(i)
                    .map_or_else(|| "unbekannt".to_owned(), |(a, _)| a.feld_id.clone());
                werte.setze(&fid, w);
            }
            (achsen, felder, cap, werte)
        })
}

#[test]
fn generierte_faelle() {
    if skip() {
        return;
    }
    let bilanz = Mutex::new(Bilanz::new());
    let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig {
        cases: 1000,
        ..ProptestConfig::default()
    });
    runner
        .run(&fall(), |(achsen, felder, cap, werte)| {
            let rust_b: Vec<AchsenBindung> = achsen.iter().map(|(a, _)| a.clone()).collect();
            // Python-dict hat eindeutige Schlüssel; Duplikate (gleiche feld_id) sind außerhalb des
            // Wertebereichs — die Registry lehnt sie ab.
            let ids: HashSet<&str> = rust_b.iter().map(|a| a.feld_id.as_str()).collect();
            if ids.len() != rust_b.len() {
                return Ok(());
            }
            let py_b: Vec<&Value> = achsen.iter().map(|(_, p)| p).collect();
            let mut b = bilanz.lock().unwrap();
            let py = frage(&json!({"fn": "intervall.intervall", "snapshot": felder, "bindung": py_b, "cap": cap}));
            buche(&mut b, "intervall", &iv_json(&intervall(&felder, &rust_b, bescheid, cap, None)), &py);
            for q in ["festzusetzende_est", "gewst_cent"] {
                let py = frage(&json!({"fn": "intervall.bescheid_via_slots", "bindung": py_b, "quantitaet": q,
                    "feld_werte": werte_json(&werte)}));
                buche(&mut b, if q == "gewst_cent" { "via_slots cent" } else { "via_slots euro" },
                    &via_slots_rust(&rust_b, &werte, q), &py);
            }
            Ok(())
        })
        .unwrap();
    let bilanz = bilanz.into_inner().unwrap();
    assert_eq!(berichte("generierte Faelle (1000)", &bilanz), 0);
    assert!(bilanz["intervall"].faelle > 900);
    assert!(
        bilanz["intervall"].zahl > 300,
        "Generator trifft den Zahl-Pfad zu selten"
    );
    assert!(
        bilanz["via_slots euro"].zahl > 400,
        "Generator trifft den Slot-Erfolgspfad zu selten"
    );
}

#[test]
fn negativkontrolle() {
    if skip() {
        return;
    }
    let a = AchsenBindung {
        feld_id: "tage".into(),
        typ: Feldtyp::Int,
        askable: true,
        enum_werte: vec![],
        bereich: Some((0, 10)),
        signatur_slot: Some("tage".into()),
        slot_beitrag: SlotBeitrag::Exakt,
    };
    let py_b = json!([{"feld_id": "tage", "typ": "int", "askable": true, "bereich": {"min": 0, "max": 10},
        "quelle": {"regel_id": "r", "signatur_slot": "tage"}}]);
    let felder = Felder::new();
    let py = frage(
        &json!({"fn": "intervall.intervall", "snapshot": felder, "bindung": py_b, "cap": 256}),
    );
    let mut rust = iv_json(&intervall(
        &felder,
        std::slice::from_ref(&a),
        bescheid,
        256,
        None,
    ));
    let mut bilanz = Bilanz::new();
    buche(&mut bilanz, "unveraendert", &rust, &py);
    let min = rust["intervall"]["min_cent"].as_i64().unwrap();
    rust["intervall"]["min_cent"] = (min + 1).into();
    buche(&mut bilanz, "min_cent + 1", &rust, &py);
    let mut w = Werte::neu();
    w.setze("tage", 7.into());
    let py = frage(
        &json!({"fn": "intervall.bescheid_via_slots", "bindung": py_b, "quantitaet": "festzusetzende_est",
        "feld_werte": werte_json(&w)}),
    );
    let r = via_slots_rust(std::slice::from_ref(&a), &w, "festzusetzende_est");
    buche(&mut bilanz, "via_slots unveraendert", &r, &py);
    buche(
        &mut bilanz,
        "via_slots + 1 Cent",
        &json!(r.as_i64().unwrap() + 1),
        &py,
    );
    buche(
        &mut bilanz,
        "via_slots Einheit Cent statt Euro",
        &via_slots_rust(std::slice::from_ref(&a), &w, "gewst_cent"),
        &py,
    );
    berichte("Negativkontrolle", &bilanz);
    assert_eq!(bilanz["unveraendert"].diffs, 0);
    assert_eq!(bilanz["via_slots unveraendert"].diffs, 0);
    assert_eq!(bilanz["min_cent + 1"].diffs, 1);
    assert_eq!(bilanz["via_slots + 1 Cent"].diffs, 1);
    assert_eq!(bilanz["via_slots Einheit Cent statt Euro"].diffs, 1);
}
