//! Parität `rust/konsistenz` gegen `produkt/konsistenz/*` (über `tools/parity/oracle_konsistenz.py`).
//!
//! Drei Tests, EIN Orakel-Prozess für die ganze Binary:
//! - `konstanten_gleich`: `FLAG_NEGIERT`, `PARTNER_FELDER`, `RING_BETRAGSFELDER`, `PAUSCHAL_CHECKS`,
//!   `NICHT_GERECHNET` gegen die Python-Tabellen.
//! - `reale_faelle`: jede Store-Datei unter `faelle_verzeichnis()` (Rust materialisiert), je drei
//!   Scheiben-Varianten (`None`, volle Bindung, volle Bindung ohne Flags).
//! - `generierte_faelle`: 1000 proptest-Snapshots über einem Feld-Pool aus allen Prüfungen.
//!
//! Jeder Test zählt je Funktion Fälle, nicht-leere Ergebnisse und Abweichungen; Abnahme 0.
//! Negativkontrolle: ein gestörtes Rust-Ergebnis (1 Cent / 1 Zeichen) muss als Abweichung zählen.
//!
//! SICHERHEIT: reale Fälle sind echte Steuerdaten. Nur lokal lesen, nur Zählwerte ausgeben.
//!
//! Wertebereich: keine Floats (s. `konsistenz`-Crate-Doku; reale Fälle: 0 Floats).
//!
//!   `PARITY`=1 `cargo` test -p parity --test `konsistenz_paritaet` -- --nocapture
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, HashSet};
use std::sync::{Mutex, OnceLock};

use bindung::Bindung;
use domain::{Achsenwert, Herkunft, PruefTiefe, PyWert, Zustand};
use konsistenz as k;
use parity::Oracle;
use proptest::prelude::*;
use serde_json::{json, Value};
use store::SnapshotFeld;

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

/// Die volle Bindung als `interview::Graph` (`TR.lade_bindung()` in Python).
fn graph() -> &'static interview::Graph<'static> {
    static REG: OnceLock<bindung::Registry> = OnceLock::new();
    static CELL: OnceLock<interview::Graph<'static>> = OnceLock::new();
    CELL.get_or_init(|| {
        interview::Graph::aus_registry(REG.get_or_init(|| {
            bindung::lade_registry(&repo_root().join("produkt/bindung")).expect("registry")
        }))
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

// ------------------------------------------------------------------ Rust-Ergebnis → Python-Form

/// Scheitert nur an NaN/inf: die weist `store::Store::append` ab (Auflage 3), `1e999` lehnt
/// `serde_json` schon beim Laden ab.
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

fn preflight_json(e: &k::PreflightErgebnis) -> Value {
    json!({
        "widersprueche_flag": flag_json(&e.widersprueche_flag),
        "widersprueche_partner": partner_json(&e.widersprueche_partner),
        "widersprueche_alleinerziehend": partner_json(&e.widersprueche_alleinerziehend),
        "widersprueche_plausibilitaet": plausi_json(&e.widersprueche_plausibilitaet),
        "hinweise_pauschalen": pauschal_json(&e.hinweise_pauschalen),
        "hinweise_nicht_gerechnet": nicht_gerechnet_json(&e.hinweise_nicht_gerechnet),
        "hinweise_betrag_vorlaeufig": e.hinweise_betrag_vorlaeufig.iter()
            .map(|h| json!({"feld_id": h.feld_id, "wert": h.wert, "hinweis": h.hinweis})).collect::<Vec<_>>(),
        "status": e.status.als_str(),
    })
}

// ------------------------------------------------------------------ Vergleich

#[derive(Default)]
struct Zaehler {
    faelle: usize,
    /// Wie oft diese Zeile durch [`buche`] lief, also WIRKLICH zwei Werte verglich. Eine reine
    /// Abdeckungszeile (nur `faelle`, s. `luecken_nicht_leer`) bleibt hier 0 -- der Waechter
    /// beurteilt darum strukturell, nicht ueber eine Namensliste.
    vergleiche: usize,
    nicht_leer: usize,
    diffs: usize,
}

type Bilanz = BTreeMap<&'static str, Zaehler>;

/// Bekannt leere Zeilen in `generierte Faelle (1000)`. `nicht_gerechnete_angaben` ist LEER PER BAUART,
/// nicht per Korpus: `konsistenz/src/nicht_gerechnet.rs:10` ist `NICHT_GERECHNET = &[]`
/// (seit 2026-09-26), und die Funktion filtert ueber genau diese Konstante -- sie kann nie
/// etwas liefern. Das Python-Gegenstueck ist ebenso leer. Wird die Tabelle gefuellt, MUSS diese
/// Zeile rechnen und der Eintrag hier fallen; sonst wird der Block rot (zweite Richtung).
const LEER_GENERIERTE: &[(&str, &str)] = &[
    ("nicht_gerechnete_angaben", "leer per Bauart: NICHT_GERECHNET = []"),
];

/// Bekannt leere Zeilen in `reale Faelle`.
///
/// - `nicht_gerechnete_angaben`: wie oben, leer per Bauart.
/// - `alleinerziehend_mit_zusammen`: die Vorbedingung (`veranlagung=zusammen` UND
///   `fam_alleinstehend=true`) trifft der reale Korpus nie; im generierte-Block rechnet die
///   Zeile 72-mal von 1000.
const LEER_REALE: &[(&str, &str)] = &[
    ("nicht_gerechnete_angaben", "leer per Bauart: NICHT_GERECHNET = []"),
    (
        "alleinerziehend_mit_zusammen",
        "realer Korpus trifft die Vorbedingung nicht; generierte: 72",
    ),
];

fn buche(bilanz: &mut Bilanz, name: &'static str, rust: &Value, py: &Value) {
    let z = bilanz.entry(name).or_default();
    z.faelle += 1;
    z.vergleiche += 1;
    let leer = match rust {
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.get("status").is_some_and(|s| s == "GREEN"),
        _ => false,
    };
    if !leer {
        z.nicht_leer += 1;
    }
    if rust != py {
        z.diffs += 1;
        if z.diffs <= 3 {
            // Nur Strukturhinweis, keine Werte (reale Daten): die Schlüssel der ersten Abweichung.
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
            "  {name:<40} faelle={:>6} nicht_leer={:>6} diffs={}",
            z.faelle, z.nicht_leer, z.diffs
        );
        summe += z.diffs;
    }
    summe
}

/// Die `nicht_leer`-Zaehler der beurteilten Zeilen als [`parity::pin::Gesehen`].
///
/// Beurteilt werden nur Vergleichszeilen (`vergleiche > 0`). Eine reine Abdeckungszeile, die
/// nie zwei Werte gegeneinander hielt, kann nichts belegen und wird nicht beurteilt -- das ist
/// eine Eigenschaft ihrer Bauart, keine Ausnahmeliste mit Namen.
fn gesehen(bilanz: &Bilanz) -> parity::pin::Gesehen {
    bilanz
        .iter()
        .filter(|(_, z)| z.vergleiche > 0)
        .map(|(n, z)| (*n, z.nicht_leer as u64))
        .collect()
}

/// Waechter gegen einen gruenen Lauf, der nichts belegt: eine Vergleichszeile, die nie einen
/// Wert GESEHEN hat (`nicht_leer == 0`), kann nicht "0 Abweichungen" beweisen -- gruen und leer
/// sehen identisch aus.
///
/// Ticket `parity-lauf-gruen-ohne-dass-die-zeile-rechnet`. Der Waechter greift JE BLOCK: eine
/// Zeile, die nur in einem Block rechnet, deckt die Luecke in einem anderen nicht.
/// Gepinnte, heute nachweislich leere Zeilen stehen in der `LEER`-Liste des Blocks.
fn wache_rechnet(block: &str, liste: &[(&str, &str)], bilanz: &Bilanz) {
    parity::pin::pruefe(block, parity::pin::KORPUS, liste, &gesehen(bilanz));
}

/// `vorjahr_referenz` → der Ganzzahl-Parameter von `plausibilitaets_widersprueche`
/// (`preflight.py:349-353`: `if vorjahr_referenz:` und `(… or {}).get("wert")`, int ohne bool).
fn vorjahr_wert(v: Option<&Value>) -> Option<i64> {
    v?.get("verlustvortrag_bestand")?.get("wert")?.as_i64()
}

/// Alle Funktionen gegen Python für EINEN Snapshot.
fn pruefe(
    bilanz: &mut Bilanz,
    felder: &k::Felder,
    scheibe: Option<&Vec<String>>,
    vorjahr: Option<&Value>,
) {
    let snap = serde_json::to_value(felder).unwrap();
    let set: Option<HashSet<String>> = scheibe.map(|s| s.iter().cloned().collect());
    let einfach = |fn_name: &str| json!({"fn": fn_name, "snapshot": snap});
    let py = frage(
        &json!({"fn": "konsistenz.flag_widersprueche", "snapshot": snap, "bindung": scheibe}),
    );
    buche(
        bilanz,
        "flag_widersprueche",
        &flag_json(&k::flag_widersprueche(felder, set.as_ref())),
        &py,
    );
    let py = frage(&einfach("konsistenz.partner_ohne_zusammen"));
    buche(
        bilanz,
        "partner_ohne_zusammen",
        &partner_json(&k::partner_ohne_zusammen(felder)),
        &py,
    );
    let py = frage(&einfach("konsistenz.alleinerziehend_mit_zusammen"));
    buche(
        bilanz,
        "alleinerziehend_mit_zusammen",
        &partner_json(&k::alleinerziehend_mit_zusammen(felder)),
        &py,
    );
    let py = frage(&einfach("konsistenz.pauschal_hinweise"));
    buche(
        bilanz,
        "pauschal_hinweise",
        &pauschal_json(&k::pauschal_hinweise(felder)),
        &py,
    );
    let py = frage(&einfach("konsistenz.nicht_gerechnete_angaben"));
    buche(
        bilanz,
        "nicht_gerechnete_angaben",
        &nicht_gerechnet_json(&k::nicht_gerechnete_angaben(felder)),
        &py,
    );
    let py = frage(
        &json!({"fn": "konsistenz.preflight", "snapshot": snap, "bindung": scheibe, "vorjahr_referenz": vorjahr}),
    );
    let rust = k::preflight(felder, set.as_ref(), vorjahr_wert(vorjahr), graph());
    buche(bilanz, "preflight", &preflight_json(&rust), &py["ergebnis"]);
    // Zählt nur die Abdeckung: Rust rechnet die Lücken selbst (`interview::fehlende_instanzen`).
    if py["luecken"].as_array().is_some_and(|a| !a.is_empty()) {
        bilanz
            .entry("  davon luecken_nicht_leer")
            .or_default()
            .faelle += 1;
    }
}

// ------------------------------------------------------------------ Tests

#[test]
fn konstanten_gleich() {
    // Abweichung Nr. 28 und Nr. 31: diese zwei Felder stehen nur in Rust (wie `konsistenz/tests/orakel_werte.rs`).
    const NUR_RUST: [&str; 2] = ["ep_unfallkosten", "parteispenden_betrag"];
    if skip() {
        return;
    }
    let py = frage(&json!({"fn": "konsistenz.konstanten"}));
    let flag: Value = k::FLAG_NEGIERT.iter().map(|(f, l)| json!([f, l])).collect();
    assert_eq!(flag, py["flag_negiert"]);
    assert_eq!(json!(k::PARTNER_FELDER), py["partner_felder"]);
    let ohne_rust_felder: Vec<&str> = k::RING_BETRAGSFELDER
        .iter()
        .copied()
        .filter(|f| !NUR_RUST.contains(f))
        .collect();
    assert_eq!(
        ohne_rust_felder.len() + NUR_RUST.len(),
        k::RING_BETRAGSFELDER.len(),
        "{NUR_RUST:?} fehlt in RING_BETRAGSFELDER"
    );
    assert_eq!(json!(ohne_rust_felder), py["ring_betragsfelder"]);
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
    assert_eq!(pc, py["pauschal_checks"]);
    // `nur_wenn_alle_leer` ersetzt Pythons `check["id"] == "vv_wk"`:
    assert!(k::PAUSCHAL_CHECKS
        .iter()
        .all(|c| c.nur_wenn_alle_leer == (c.id == "vv_wk")));
    println!("konstanten: 5 Tabellen gleich");
}

#[test]
fn reale_faelle() {
    if skip() {
        return;
    }
    let alle: Vec<String> = bindungen().iter().map(|b| b.feld_id.clone()).collect();
    let flags: HashSet<&str> = k::FLAG_NEGIERT.iter().map(|(f, _)| *f).collect();
    let ohne_flags: Vec<String> = alle
        .iter()
        .filter(|f| !flags.contains(f.as_str()))
        .cloned()
        .collect();
    let mut dateien = 0;
    let mut bilanz = Bilanz::new();
    // P10: store::lade laedt inzwischen ALLE realen Faelle (legacy Herkunft, unbegrenzte VZ) —
    // keine rohe Fallback-Faltung mehr noetig.
    for pfad in walk_json(&faelle_verzeichnis()) {
        dateien += 1;
        let datei =
            store::lade(&pfad).unwrap_or_else(|e| panic!("store::lade({}): {e}", pfad.display()));
        let (felder, _) = store::Store::aus_datei(datei).materialisiere(None).unwrap();
        for scheibe in [None, Some(&alle), Some(&ohne_flags)] {
            pruefe(&mut bilanz, &felder, scheibe, None);
        }
    }
    println!("reale Faelle: {dateien} Dateien via store::lade (x3 Scheiben-Varianten)");
    assert!(
        dateien > 0,
        "keine realen Faelle gefunden — Paritaet waere leer"
    );
    wache_rechnet("reale Faelle", LEER_REALE, &bilanz);
    assert_eq!(berichte("reale Faelle", &bilanz), 0);
}

fn herkunft() -> Herkunft {
    let a = Achsenwert::new("parity").unwrap();
    Herkunft {
        herkunft: a.clone(),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: a,
    }
}

fn pool() -> Vec<String> {
    let mut p: Vec<String> = Vec::new();
    for (flag, felder) in k::FLAG_NEGIERT {
        p.push(flag.to_owned());
        for f in felder {
            p.push((*f).to_owned());
            for s in ["__2", "__3", "__10", "__02", "__0", "__1", "__x"] {
                p.push(format!("{f}{s}"));
            }
        }
    }
    p.extend(k::PARTNER_FELDER.iter().map(|s| (*s).to_owned()));
    for c in &k::PAUSCHAL_CHECKS {
        p.extend(
            c.ausloeser_felder
                .iter()
                .chain(c.pauschal_felder)
                .map(|s| (*s).to_owned()),
        );
    }
    p.extend(
        [
            "veranlagung",
            "fam_alleinstehend",
            "bruttoarbeitslohn",
            "p36_lohnsteuer",
            "vor_an_anteil_rv",
            "vor_ag_anteil_rv",
            "kist_gezahlt",
            "kirchensteuer_arbeitgeber",
            "kist_erstattet",
            "kist_konfession",
            "kist_bundesland",
            "schulgeld",
            "schulgeld__2",
            "schulgeld__3",
            "schulgeld__0",
            "schulgeld__x",
            "schulgeld__",
            "stammdaten_keine_bankverbindung",
            "stammdaten_iban",
            "verlustvortrag_bestand",
            "fam_anzahl_kinder",
            "kinderbetreuungskosten",
            "kinderbetreuungskosten__2",
            "kinderbetreuungskosten__3",
            "rentner_anzahl_renten",
            "p23_anzahl_verkaeufe",
            "gwg_anzahl",
            "vv_anzahl_objekte",
            "hh_dienstleistungen",
            "spenden_betrag",
            "tage_24h",
            "p36_vorauszahlungen",
            "basis_kv",
        ]
        .map(str::to_owned),
    );
    p.sort();
    p.dedup();
    p
}

fn wert() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        (-3_i64..=4).prop_map(Value::from),
        prop::sample::select(vec![
            0_i64,
            1,
            999,
            1000,
            1001,
            1500,
            100_000,
            1_212_321_300,
            16_666_660,
            16_666_661,
            5_000_000_000,
            -150,
            i64::MAX,
            i64::MIN
        ])
        .prop_map(Value::from),
        (-10_i64.pow(13)..10_i64.pow(13)).prop_map(Value::from),
        prop::sample::select(vec![
            "", " ", "\u{1f}", "\u{a0}", "einzel", "zusammen", "DE12", "rk", "by", " x "
        ])
        .prop_map(|s| Value::String(s.to_owned())),
    ]
}

const INSTANZ_REIHEN: [(&str, &str); 4] = [
    ("fam_anzahl_kinder", "kinderbetreuungskosten"),
    ("rentner_anzahl_renten", "rentner_jahresrente"),
    ("p23_anzahl_verkaeufe", "p23_veraeusserungspreis"),
    ("gwg_anzahl", "gwg_anschaffungskosten_netto"),
];

type Fall = (k::Felder, Option<Vec<String>>, Option<Value>);

fn fall() -> impl Strategy<Value = Fall> {
    let p = pool();
    let eintraege = prop::collection::vec(
        (prop::sample::select(p.clone()), wert(), any::<bool>()),
        0..30,
    );
    // KiSt-Schwelle gezielt: kist = floor(3·brutto/10) + d.
    let kist = prop::option::of((1_i64..=10_i64.pow(12), -2_i64..=2));
    let scheibe = prop::option::of(prop::collection::vec(prop::sample::select(p), 0..40));
    let vorjahr = prop::option::of(prop_oneof![
        Just(json!({})),
        wert().prop_map(|w| json!({"verlustvortrag_bestand": {"wert": w}})),
    ]);
    // Anker: Veranlagung, § 24b, ein Partnerfeld und eine Instanzreihe — sonst trifft der
    // Zufall aus ~300 Schluesseln diese Pruefungen kaum.
    let anker = (
        prop::option::of((
            prop::sample::select(vec!["einzel", "zusammen", "zusammen", "getrennt"]),
            prop::bool::weighted(0.8),
        )),
        prop::option::of((prop::bool::weighted(0.8), prop::bool::weighted(0.8))),
        prop::option::of((prop::sample::select(k::PARTNER_FELDER.to_vec()), wert())),
        prop::option::of((
            prop::sample::select(INSTANZ_REIHEN.to_vec()),
            0_i64..5,
            any::<u8>(),
        )),
    );
    (eintraege, kist, scheibe, vorjahr, anker).prop_map(
        |(eintraege, kist, scheibe, vorjahr, anker)| {
            let mut felder = k::Felder::new();
            let mut setze = |f: &str, w: Value, b: bool| {
                let zustand = if b {
                    Zustand::Bestaetigt
                } else {
                    Zustand::Vorlaeufig
                };
                felder.insert(
                    f.to_owned(),
                    SnapshotFeld {
                        wert: PyWert::from(w),
                        zustand,
                        herkunft: herkunft().into(),
                    },
                );
            };
            for (f, w, b) in eintraege {
                setze(&f, w, b);
            }
            let (veranlagung, allein, partner, reihe) = anker;
            if let Some((v, b)) = veranlagung {
                setze("veranlagung", json!(v), b);
            }
            if let Some((w, b)) = allein {
                setze("fam_alleinstehend", json!(w), b);
            }
            if let Some((f, w)) = partner {
                setze(f, w, true);
            }
            if let Some(((anzahl_feld, basis), n, maske)) = reihe {
                setze(anzahl_feld, json!(n), true);
                for i in 1..=5_u8 {
                    if maske & (1 << i) != 0 {
                        let fid = if i == 1 {
                            basis.to_owned()
                        } else {
                            format!("{basis}__{i}")
                        };
                        setze(&fid, json!(1000), maske & 1 == 0 || i != 2);
                    }
                }
            }
            if let Some((brutto, d)) = kist {
                let k = brutto * 3 / 10 + d;
                for (f, w) in [("bruttoarbeitslohn", brutto), ("kist_gezahlt", k)] {
                    felder.insert(
                        f.to_owned(),
                        SnapshotFeld {
                            wert: PyWert::Ganz(w),
                            zustand: Zustand::Bestaetigt,
                            herkunft: herkunft().into(),
                        },
                    );
                }
            }
            (felder, scheibe, vorjahr)
        },
    )
}

#[test]
fn generierte_faelle() {
    if skip() {
        return;
    }
    let bilanz = Mutex::new(Bilanz::new());
    let n_faelle = parity::fallzahl::holen_u32("konsistenz_paritaet generierte_faelle", 1000);
    let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig {
        cases: n_faelle,
        ..ProptestConfig::default()
    });
    runner
        .run(&fall(), |(felder, scheibe, vorjahr)| {
            pruefe(
                &mut bilanz.lock().unwrap(),
                &felder,
                scheibe.as_ref(),
                vorjahr.as_ref(),
            );
            Ok(())
        })
        .unwrap();
    let bilanz = bilanz.into_inner().unwrap();
    let block = format!("generierte Faelle ({n_faelle})");
    // Pin der leeren Zeilen und Untergrenzen sind an die 1000 Faelle des Standards gebunden.
    let wachen = parity::fallzahl::wache_gilt(
        "konsistenz_paritaet generierte_faelle",
        n_faelle as usize,
        1000,
    );
    if wachen {
        wache_rechnet(&block, LEER_GENERIERTE, &bilanz);
    }
    assert_eq!(berichte(&block, &bilanz), 0);
    if wachen {
        assert!(
            bilanz["flag_widersprueche"].nicht_leer > 50,
            "Generator trifft flag_check zu selten"
        );
        assert!(
            bilanz["preflight"].nicht_leer > 500,
            "Generator trifft preflight zu selten"
        );
        assert!(
            bilanz["partner_ohne_zusammen"].nicht_leer > 20,
            "Generator trifft partner_check zu selten"
        );
        assert!(
            bilanz["alleinerziehend_mit_zusammen"].nicht_leer > 20,
            "Generator trifft § 24b zu selten"
        );
        assert!(
            bilanz["  davon luecken_nicht_leer"].faelle > 20,
            "Generator trifft fehlende_instanzen zu selten"
        );
    }
}

#[test]
fn negativkontrolle() {
    if skip() {
        return;
    }
    let mut felder = k::Felder::new();
    for (f, w) in [
        ("kein_vuv", json!(true)),
        ("vv_einnahmen", json!(1_200_000)),
        ("bruttoarbeitslohn", json!(1000)),
        ("p36_lohnsteuer", json!(2000)),
    ] {
        felder.insert(
            f.to_owned(),
            SnapshotFeld {
                wert: PyWert::from(w),
                zustand: Zustand::Bestaetigt,
                herkunft: herkunft().into(),
            },
        );
    }
    let snap = serde_json::to_value(&felder).unwrap();
    let py =
        frage(&json!({"fn": "konsistenz.flag_widersprueche", "snapshot": snap, "bindung": null}));
    let mut rust = k::flag_widersprueche::<std::hash::RandomState>(&felder, None);
    let mut bilanz = Bilanz::new();
    buche(&mut bilanz, "unveraendert", &flag_json(&rust), &py);
    rust[0].grund.push('x');
    buche(&mut bilanz, "grund + 1 Zeichen", &flag_json(&rust), &py);
    rust[0].grund.pop();
    rust[0].wert = PyWert::Ganz(1_200_001);
    buche(&mut bilanz, "wert + 1 Cent", &flag_json(&rust), &py);
    let py = frage(
        &json!({"fn": "konsistenz.preflight", "snapshot": snap, "bindung": null, "vorjahr_referenz": null}),
    );
    let mut e = k::preflight::<std::hash::RandomState>(&felder, None, None, graph());
    e.widersprueche_plausibilitaet[0].bezug = Some(1001);
    buche(
        &mut bilanz,
        "preflight bezug + 1 Cent",
        &preflight_json(&e),
        &py["ergebnis"],
    );
    berichte("Negativkontrolle", &bilanz);
    assert_eq!(bilanz["unveraendert"].diffs, 0);
    assert_eq!(bilanz["grund + 1 Zeichen"].diffs, 1);
    assert_eq!(bilanz["wert + 1 Cent"].diffs, 1);
    assert_eq!(bilanz["preflight bezug + 1 Cent"].diffs, 1);
}
