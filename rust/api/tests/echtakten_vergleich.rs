//! Echtakten-Vergleich: der Rust-Dienst gegen die einmal aufgezeichneten Python-Antworten auf den
//! echten Akten. Ersatz fuer den Teil von `rust/parity` und `tools/parity/rauchprobe_echtdaten.py`, der
//! nur mit Python lebt (`REWRITE_PLAN`, Klassifikation 2026-10-05, Befund D).
//!
//! Aufzeichnen (einmal, solange Python noch laeuft): `python3 tools/parity/echtakten_aufzeichnen.py aufzeichnen`
//! schreibt `~/.local/share/taxgraph-aufzeichnung/<Datum>/aufzeichnung.json` (Pruefsummen und Zaehler,
//! nie Werte, nie Fall-IDs, nie im Repo).
//!
//! Vergleichen (lokal, von Hand): `TAXGRAPH_AUFZEICHNUNG=<Verzeichnis> cargo test -p api --test
//! echtakten_vergleich -- --ignored`. Der Test kopiert die Fall-Akten aus `TAXGRAPH_DATEN` (sonst
//! `~/.local/share/taxgraph`) in ein Scratch-Verzeichnis, ruft den Dienst im Prozess und vergleicht je Fall
//! und Route Status und kanonische Pruefsumme. Er schreibt nie in die Quelle.
//!
//! Drei Pruefungen je Fall: (1) Eingabedatei unveraendert seit der Aufzeichnung, sonst zaehlt der Fall als
//! `nicht vergleichbar`; (2) `event_id`: die Liste der gespeicherten Kennungen ist die aufgezeichnete, und
//! `berechne_event_id` reproduziert jede (die Akten tragen Pythons Hashes, das ist die Python-Aufzeichnung der
//! Ereigniskennungen); (3) je lesender Route gleicher Status und gleiche Pruefsumme.
//!
//! Bekannte Abweichungen (Rust antwortet absichtlich anders als Python) stehen in `erwartung.json` neben der
//! Aufzeichnung. Der Test verlangt GENAU diese Menge: eine neue Abweichung ist rot, eine behobene auch. Anlegen:
//! `TAXGRAPH_AUFZEICHNUNG_ERWARTUNG=schreiben` (schreibt die heutige Menge und besteht).
//!
//! `#[ignore]`, weil die Akten personenbezogen sind und nur lokal liegen. `TAXGRAPH_AUFZEICHNUNG_TEILWEISE=1`
//! erlaubt Faelle, die sich seit der Aufzeichnung geaendert haben; ohne das Flag sind sie rot.
//!
//! Der Known-Answer-Test `kanonische_pruefsumme_gleich_python` laeuft in der CI: er haelt die Pruefsumme
//! byte-gleich zu `tools/parity/echtakten_aufzeichnen.py`.
//!
//! NICHT gedeckt: schreibende Routen, `chat`, `entfernung`, `einreichen`, `kontoauszug`, Fragen jenseits der
//! ersten drei je Fall, feste Uhr, die XML-Bildung der `elster`-Suite und die Append-Folgen der
//! `store_append`-Suite (beide pruefen Bibliotheksfunktionen, keine Routen), Faelle, die seit der
//! Aufzeichnung neu hinzukamen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cast_possible_truncation
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tower::ServiceExt;

/// Dieselbe Zahl steht in `tools/parity/echtakten_aufzeichnen.py` (`VEKTOR_SHA256`).
const VEKTOR_SHA256: &str = "3ffffed035be23fb820d672437451d5d555315c4991f6797cce0b870cc27f110";
const VEKTOR: &str = r#"{"b":[1,2.0,2.5,"äö",null,true,false],"a":{"x":-0.0,"y":1e300,"z":"","w":-7,"ké":[[],{}]}}"#;
const GRENZE_GLEITKOMMA: f64 = 9_007_199_254_740_992.0; // 2^53
const FALL_ROUTEN: [&str; 6] = ["stand", "fragen", "ergebnis", "preflight", "deklaration", "graph"];
const FELD_ROUTEN: [&str; 2] = ["frage", "warum"];
const FELDER_JE_FALL: usize = 3;

// ---------------------------------------------------------------- kanonische Pruefsumme

// Exakter Vergleich gewollt: ganzzahlig heisst `x == x.trunc()` genau wie in Python `x == int(x)`.
#[allow(clippy::float_cmp)]
fn speise(h: &mut Sha256, v: &Value) {
    match v {
        Value::Null => h.update(b"n"),
        Value::Bool(true) => h.update(b"t"),
        Value::Bool(false) => h.update(b"f"),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                h.update(format!("i{i};"));
            } else if let Some(u) = n.as_u64() {
                h.update(format!("i{u};"));
            } else {
                let x = n.as_f64().unwrap_or(f64::NAN);
                if x.is_finite() && x.abs() < GRENZE_GLEITKOMMA && x == x.trunc() {
                    h.update(format!("i{};", x as i64));
                } else {
                    h.update(b"d");
                    h.update(x.to_bits().to_be_bytes());
                }
            }
        }
        Value::String(s) => speise_text(h, s),
        Value::Array(a) => {
            h.update(format!("[{}:", a.len()));
            for e in a {
                speise(h, e);
            }
        }
        Value::Object(o) => {
            h.update(format!("{{{}:", o.len()));
            let mut schluessel: Vec<&String> = o.keys().collect();
            schluessel.sort();
            for k in schluessel {
                speise_text(h, k);
                speise(h, &o[k]);
            }
        }
    }
}

fn speise_text(h: &mut Sha256, s: &str) {
    h.update(format!("s{}:", s.len()));
    h.update(s.as_bytes());
}

fn pruefsumme(v: &Value) -> String {
    let mut h = Sha256::new();
    speise(&mut h, v);
    format!("{:x}", h.finalize())
}

fn antwort_pruefsumme(status: u16, body: &[u8]) -> (u16, String) {
    match serde_json::from_slice::<Value>(body) {
        Ok(v) => (status, pruefsumme(&v)),
        Err(_) => (status, format!("r{:x}", Sha256::digest(body))),
    }
}

fn fall_schluessel(id: &str) -> String {
    format!("{:x}", Sha256::digest(id.as_bytes()))[..16].to_owned()
}

#[test]
fn kanonische_pruefsumme_gleich_python() {
    let v: Value = serde_json::from_str(VEKTOR).unwrap();
    assert_eq!(pruefsumme(&v), VEKTOR_SHA256, "Known-Answer-Vektor weicht von Python ab");
    // Dieselben Eigenschaften wie `selbsttest()` in Python.
    let p = |t: &str| pruefsumme(&serde_json::from_str::<Value>(t).unwrap());
    assert_eq!(p(r#"{"a":1,"b":2}"#), p(r#"{"b":2,"a":1}"#));
    assert_eq!(p("1"), p("1.0"));
    assert_eq!(p("-0.0"), p("0"));
    assert_ne!(p("true"), p("1"));
    assert_ne!(p("2.5"), p("2"));
    assert_ne!(p("[1,2]"), p("[2,1]"));
    assert_ne!(p(r#""ab""#), p(r#"["a","b"]"#));
    assert_ne!(p(r#"{"a":["b"]}"#), p(r#"{"a":"b"}"#));
    assert_ne!(p(r#""ä""#), p(r#""a""#));
    assert_eq!(antwort_pruefsumme(404, b"kein json").1.chars().next(), Some('r'));
    assert_eq!(fall_schluessel("x").len(), 16);
}

/// Die Pruefsumme des Vektors steht in Rust und in Python. Beide Seiten pruefen ihre eigene Zahl; dieser Test
/// haelt die beiden Zahlen gleich (sonst bestuenden beide Seiten gegen verschiedene Vektoren).
#[test]
fn vektor_zahl_steht_auch_im_aufzeichnungsskript() {
    let skript = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/parity/echtakten_aufzeichnen.py");
    let text = std::fs::read_to_string(&skript).expect(
        "tools/parity/echtakten_aufzeichnen.py fehlt: wird das Skript gestrichen, diese Pruefung mit ihm streichen",
    );
    assert!(
        text.contains(&format!("VEKTOR_SHA256 = \"{VEKTOR_SHA256}\"")),
        "VEKTOR_SHA256 im Aufzeichnungsskript weicht von der Zahl in diesem Test ab"
    );
}

// ---------------------------------------------------------------- Dienst auf einer Kopie

struct Dienst {
    zustand: Zustand,
    tmp: tempfile::TempDir,
}

fn daten_quelle() -> PathBuf {
    let eigen = std::env::var("TAXGRAPH_DATEN").unwrap_or_default();
    if eigen.trim().is_empty() {
        PathBuf::from(std::env::var("HOME").expect("HOME")).join(".local/share/taxgraph/faelle")
    } else {
        PathBuf::from(eigen.trim()).join("faelle")
    }
}

/// Kopiert nur `*.json` der Fallakten (Ebene 1). Die Quelle wird nur gelesen.
fn dienst_auf_kopie(quelle: &Path) -> Dienst {
    let tmp = tempfile::tempdir().unwrap();
    let faelle = tmp.path().join("faelle");
    std::fs::create_dir_all(&faelle).unwrap();
    for e in std::fs::read_dir(quelle).unwrap().flatten() {
        let p = e.path();
        if p.is_file() && p.extension().is_some_and(|x| x == "json") {
            std::fs::copy(&p, faelle.join(e.file_name())).unwrap();
        }
    }
    let konfig = Konfig {
        wurzel: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        faelle: faelle.clone(),
        audit_dir: faelle,
    };
    let auth = Auth::neu("echtakten".into(), tmp.path().join("users.json"), Some(konfig.audit_pfad()));
    Dienst { zustand: Zustand::neu(konfig, auth), tmp }
}

async fn hole(d: &Dienst, pfad: &str) -> (u16, Vec<u8>) {
    let req = Request::builder().method("GET").uri(pfad).body(Body::empty()).unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    (teile.status.as_u16(), rumpf.collect().await.unwrap().to_bytes().to_vec())
}

fn id_gueltig(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Route -> (Status, Pruefsumme), im Schluesselschema der Aufzeichnung (`feld/frage/0`).
async fn routen_rust(d: &Dienst, id: &str) -> BTreeMap<String, (u16, String)> {
    let mut aus = BTreeMap::new();
    let mut fragen: Vec<String> = Vec::new();
    for route in FALL_ROUTEN {
        let (status, body) = hole(d, &format!("/fall/{id}/{route}")).await;
        if route == "fragen" && status == 200 {
            if let Ok(v) = serde_json::from_slice::<Value>(&body) {
                fragen = v["fragen"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|q| q["feld_id"].as_str().map(str::to_owned)).collect())
                    .unwrap_or_default();
            }
        }
        aus.insert((*route).to_owned(), antwort_pruefsumme(status, &body));
    }
    for (i, feld) in fragen.iter().filter(|f| id_gueltig(f, 64) && !f.contains('-')).take(FELDER_JE_FALL).enumerate() {
        for route in FELD_ROUTEN {
            let (status, body) = hole(d, &format!("/fall/{id}/feld/{feld}/{route}")).await;
            aus.insert(format!("feld/{route}/{i}"), antwort_pruefsumme(status, &body));
        }
    }
    aus
}

// ---------------------------------------------------------------- Vergleich

#[derive(Default)]
struct Ergebnis {
    verglichen: usize,
    nicht_vergleichbar: usize,
    fehlt_in_daten: usize,
    unbekannt: usize,
    abfragen: usize,
    ereignisse: usize,
    event_id_summe_abweichend: usize,
    event_id_selbst_abweichend: usize,
    verglichen_schluessel: BTreeSet<String>,
    /// (Fall-Schluessel, Route)
    abweichend: BTreeSet<(String, String)>,
}

fn event_ids(roh: &[u8]) -> Value {
    let v: Value = serde_json::from_slice(roh).unwrap();
    Value::Array(
        v["events"]
            .as_array()
            .map(|a| a.iter().map(|e| e["event_id"].clone()).collect())
            .unwrap_or_default(),
    )
}

fn event_id_selbstpruefung(datei: &Path) -> (usize, usize) {
    let d = store::lade(datei).unwrap();
    let falsch = d.events.iter().filter(|e| e.berechne_event_id() != Ok(e.event_id)).count();
    (d.events.len(), falsch)
}

async fn vergleiche_fall(d: &Dienst, quelle: &Path, id: &str, rec: &Value, erg: &mut Ergebnis) {
    let schluessel = fall_schluessel(id);
    let roh = std::fs::read(quelle.join(format!("{id}.json"))).unwrap();
    if format!("{:x}", Sha256::digest(&roh)) != rec["eingabe"].as_str().unwrap_or_default() {
        erg.nicht_vergleichbar += 1;
        return;
    }
    erg.verglichen += 1;
    erg.verglichen_schluessel.insert(schluessel.clone());
    if pruefsumme(&event_ids(&roh)) != rec["event_ids"].as_str().unwrap_or_default() {
        erg.event_id_summe_abweichend += 1;
    }
    let (n, falsch) = event_id_selbstpruefung(&d.tmp.path().join("faelle").join(format!("{id}.json")));
    erg.ereignisse += n;
    erg.event_id_selbst_abweichend += falsch;
    let rust = routen_rust(d, id).await;
    let python: BTreeMap<String, (u16, String)> = rec["routen"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), (v[0].as_u64().map_or(0, |s| s as u16), v[1].as_str().unwrap_or_default().to_owned())))
        .collect();
    let alle: BTreeSet<&String> = rust.keys().chain(python.keys()).collect();
    for route in alle {
        erg.abfragen += 1;
        if rust.get(route) != python.get(route) {
            erg.abweichend.insert((schluessel.clone(), route.clone()));
        }
    }
}

fn lies_json(pfad: &Path) -> Option<Value> {
    serde_json::from_slice(&std::fs::read(pfad).ok()?).ok()
}

fn zaehler_je_route(menge: &BTreeSet<(String, String)>) -> BTreeMap<String, usize> {
    let mut z = BTreeMap::new();
    for (_, route) in menge {
        // `feld/frage/2` zaehlt als `feld/frage`
        let r = route.rsplit_once('/').filter(|(_, i)| i.chars().all(|c| c.is_ascii_digit())).map_or(route.as_str(), |(r, _)| r);
        *z.entry(r.to_owned()).or_insert(0) += 1;
    }
    z
}

#[tokio::test]
#[ignore = "braucht die Aufzeichnung (TAXGRAPH_AUFZEICHNUNG) und die echten Akten, nur lokal"]
async fn echtakten_gegen_aufzeichnung() {
    let Ok(verz) = std::env::var("TAXGRAPH_AUFZEICHNUNG") else {
        panic!("TAXGRAPH_AUFZEICHNUNG=<Verzeichnis mit aufzeichnung.json> setzen; ohne Aufzeichnung misst der Test nichts");
    };
    let verz = PathBuf::from(verz);
    let aufz = lies_json(&verz.join("aufzeichnung.json")).expect("aufzeichnung.json lesbar");
    assert_eq!(aufz["vektor"], VEKTOR_SHA256, "die Aufzeichnung nutzt eine andere kanonische Pruefsumme");
    let faelle = aufz["faelle"].as_object().expect("faelle");
    std::env::set_var("TAXGRAPH_NO_AUTH", "1");
    let quelle = daten_quelle();
    let d = dienst_auf_kopie(&quelle);
    let mut erg = Ergebnis::default();
    let mut gesehen: BTreeSet<String> = BTreeSet::new();
    let mut ids: Vec<String> = std::fs::read_dir(&quelle)
        .unwrap()
        .flatten()
        .filter_map(|e| e.path().file_stem().and_then(|s| s.to_str()).map(str::to_owned).filter(|_| e.path().extension().is_some_and(|x| x == "json")))
        .collect();
    ids.sort();
    for id in &ids {
        let schluessel = fall_schluessel(id);
        match faelle.get(&schluessel) {
            Some(rec) => {
                gesehen.insert(schluessel);
                vergleiche_fall(&d, &quelle, id, rec, &mut erg).await;
            }
            None => erg.unbekannt += 1,
        }
    }
    erg.fehlt_in_daten = faelle.keys().filter(|k| !gesehen.contains(*k)).count();
    eprintln!(
        "echtakten: {} Faelle aufgezeichnet, {} verglichen, {} nicht vergleichbar (Eingabe geaendert), {} fehlen in den Daten, {} neu; \
         {} Abfragen, {} abweichend {:?}; event_id: {} Ereignisse, {} Listen abweichend, {} Selbstpruefungen abweichend (keine Pfade, keine Werte)",
        faelle.len(), erg.verglichen, erg.nicht_vergleichbar, erg.fehlt_in_daten, erg.unbekannt,
        erg.abfragen, erg.abweichend.len(), zaehler_je_route(&erg.abweichend), erg.ereignisse,
        erg.event_id_summe_abweichend, erg.event_id_selbst_abweichend,
    );
    let erwartung_pfad = verz.join("erwartung.json");
    if std::env::var("TAXGRAPH_AUFZEICHNUNG_ERWARTUNG").is_ok_and(|v| v == "schreiben") {
        let liste: Vec<&(String, String)> = erg.abweichend.iter().collect();
        std::fs::write(&erwartung_pfad, serde_json::to_string_pretty(&serde_json::json!({ "abweichend": liste })).unwrap()).unwrap();
        eprintln!("erwartung.json geschrieben: {} bekannte Abweichungen", erg.abweichend.len());
        return;
    }
    assert!(erg.verglichen > 0, "kein Fall verglichen: ein gruener Lauf ohne Messung belegt nichts");
    assert_eq!(erg.event_id_selbst_abweichend, 0, "berechne_event_id reproduziert gespeicherte Kennungen nicht");
    assert_eq!(erg.event_id_summe_abweichend, 0, "Liste der event_id weicht von der Aufzeichnung ab");
    if std::env::var("TAXGRAPH_AUFZEICHNUNG_TEILWEISE").is_err() {
        assert_eq!(
            (erg.nicht_vergleichbar, erg.fehlt_in_daten, erg.unbekannt),
            (0, 0, 0),
            "Akten seit der Aufzeichnung geaendert; TAXGRAPH_DATEN auf den Stand der Aufzeichnung setzen oder TAXGRAPH_AUFZEICHNUNG_TEILWEISE=1"
        );
    }
    let erwartet: BTreeSet<(String, String)> = lies_json(&erwartung_pfad)
        .map(|v| {
            v["abweichend"]
                .as_array()
                .map(|a| a.iter().map(|p| (p[0].as_str().unwrap_or_default().to_owned(), p[1].as_str().unwrap_or_default().to_owned())).collect())
                .unwrap_or_default()
        })
        .unwrap_or_default();
    // Bei nicht vergleichbaren Faellen kennt der Lauf deren Abweichungen nicht: nur verglichene Faelle zaehlen.
    let erwartet: BTreeSet<_> = erwartet.into_iter().filter(|(k, _)| erg.verglichen_schluessel.contains(k)).collect();
    let neu: BTreeSet<(String, String)> = erg.abweichend.difference(&erwartet).cloned().collect();
    let behoben: BTreeSet<(String, String)> = erwartet.difference(&erg.abweichend).cloned().collect();
    assert!(
        neu.is_empty() && behoben.is_empty(),
        "{} neue Abweichungen {:?}, {} erwartete nicht mehr da {:?} (je Route gezaehlt, keine Werte)",
        neu.len(),
        zaehler_je_route(&neu),
        behoben.len(),
        zaehler_je_route(&behoben)
    );
}
