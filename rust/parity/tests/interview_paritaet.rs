//! Paritaet der Crate `interview` (Rust) gegen `produkt/traverser/traverser.py` und
//! `produkt/haut/bindung_rollen.py` (Python, ueber `tools/parity/oracle.py::_interview`).
//!
//! Eingaben:
//! - jede reale Fall-Datei `faelle/*.json` (`TAXGRAPH_DATEN`, sonst XDG): alle Funktionen, je mit
//!   der vollen Bindung und mit der Scheiben-Bindung des Falls (die Produktions-Sicht,
//!   `api._scheibe_bindung`); dazu der Interview-Ablauf: die naechste-Fragen-Queue nach JEDEM
//!   Event-Praefix;
//! - 1000 generierte Stores aus der echten Bindung (Gates, Bedingungsfelder, Zaehlfelder,
//!   `__n`-Instanzen samt ungueltiger Formen, falsche Werttypen, vorlaeufig/bestaetigt,
//!   `ersetzt`-Ketten, Vorjahres-Herkunft), mit zufaelliger Sicht, Beitrag und Kegel.
//!
//! Reihenfolge wird exakt verglichen, wo sie Semantik ist (Queue, Kegel, Achsen-Bindung); Dicts
//! als JSON-Objekte. Negativkontrolle: gestoerte Rust-Ergebnisse muessen als Abweichung zaehlen.
//!
//! SICHERHEIT: reale Fall-Dateien tragen echte Steuerdaten. Nur lokal lesen, nie kopieren; bei
//! realen Faellen erscheinen nur Funktionsname, Fall-Nummer und Zaehlwerte in der Ausgabe.
//!
//!   `PARITY`=1 `cargo` test -p parity --test `interview_paritaet` -- --nocapture --test-threads=1
#![allow(
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::num::NonZeroU16;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};

use bindung::{Bindung, Bindungspunkt, Registry, Vorjahr};
use domain::{Achsenwert, BasisId, Feldtyp, Herkunft, PruefTiefe, Schreiber, Zustand};
use interview::{Graph, Sicht};
use proptest::test_runner::{Config, TestCaseError, TestRunner};
use serde_json::{json, Value};
use store::{Event, EventId, Signal, Store, StoreDatei, Veranlagungsjahr};

// ------------------------------------------------------------------ Umgebung + Orakel

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip_ohne_parity_env() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn faelle_verzeichnis() -> std::path::PathBuf {
    let home = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let expand = |p: &str| {
        p.strip_prefix("~/")
            .map_or_else(|| std::path::PathBuf::from(p), |r| home.join(r))
    };
    let eigen = std::env::var("TAXGRAPH_DATEN").unwrap_or_default();
    if !eigen.trim().is_empty() {
        return expand(eigen.trim()).join("faelle");
    }
    let xdg = std::env::var("XDG_DATA_HOME").unwrap_or_default();
    let basis = if xdg.trim().is_empty() {
        home.join(".local/share")
    } else {
        expand(xdg.trim())
    };
    basis.join("taxgraph/faelle")
}

struct Orakel {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Orakel {
    fn spawn() -> Self {
        let mut child = Command::new("python3")
            .arg("tools/parity/oracle.py")
            .current_dir(repo_root())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("oracle.py startet");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        Self {
            child,
            stdin,
            stdout,
        }
    }

    /// `{"ok": x}` -> `Ok(x)`, `{"err": s}` -> `Err(s)`.
    fn rufe(&mut self, anfrage: &Value) -> Result<Value, String> {
        let mut zeile = serde_json::to_string(anfrage).expect("json");
        zeile.push('\n');
        self.stdin.write_all(zeile.as_bytes()).expect("schreiben");
        self.stdin.flush().expect("flush");
        let mut antwort = String::new();
        assert!(
            self.stdout.read_line(&mut antwort).expect("lesen") > 0,
            "Orakel geschlossen"
        );
        let mut v: Value = serde_json::from_str(antwort.trim()).expect("Orakel-JSON");
        match v.get_mut("ok") {
            Some(ok) => Ok(ok.take()),
            None => Err(v
                .get("err")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_owned()),
        }
    }
}

impl Drop for Orakel {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn orakel() -> &'static Mutex<Orakel> {
    static CELL: OnceLock<Mutex<Orakel>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Orakel::spawn()))
}

fn py(
    funktion: &str,
    store: Option<&Value>,
    felder: Option<&[&str]>,
    args: Value,
) -> Result<Value, String> {
    let mut anfrage = json!({ "fn": funktion, "store": store, "felder": felder });
    anfrage["args"] = args;
    orakel()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .rufe(&anfrage)
}

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| {
        bindung::lade_registry(&repo_root().join("produkt/bindung")).expect("registry")
    })
}

fn graph() -> &'static Graph<'static> {
    static CELL: OnceLock<Graph<'static>> = OnceLock::new();
    CELL.get_or_init(|| Graph::aus_registry(registry()))
}

/// Scheibe -> (Felder in Scheiben-Reihenfolge, Kegel).
type Scheiben = BTreeMap<String, (Vec<String>, Option<Vec<String>>)>;

fn scheiben() -> &'static Scheiben {
    static CELL: OnceLock<Scheiben> = OnceLock::new();
    CELL.get_or_init(|| {
        let v = py("traverser.scheiben", None, None, json!({})).expect("scheiben");
        v.as_object()
            .expect("objekt")
            .iter()
            .map(|(k, c)| {
                let liste = |x: &Value| -> Vec<String> {
                    x.as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                };
                (
                    k.clone(),
                    (
                        liste(&c["felder"]),
                        c["kegel"].as_array().map(|_| liste(&c["kegel"])),
                    ),
                )
            })
            .collect()
    })
}

// ------------------------------------------------------------------ Zaehlung + Vergleich

#[derive(Default)]
struct Zaehler {
    je_fn: BTreeMap<String, (usize, usize)>,
    abdeckung: BTreeMap<String, usize>,
    beispiele: Vec<String>,
}

thread_local! {
    /// Je Test-Thread ein Zaehler: parallel laufende Tests mischen ihre Zahlen nicht.
    static ZAEHLER: std::cell::RefCell<Zaehler> = std::cell::RefCell::new(Zaehler::default());
}

/// Zaehlt einen Vergleich; `kontext` nennt bei realen Faellen nur Nummern, nie Werte.
fn vergleiche(
    funktion: &str,
    rust: &Value,
    python: &Result<Value, String>,
    kontext: &str,
    zeige_werte: bool,
) -> bool {
    let gleich = matches!(python, Ok(p) if p == rust);
    ZAEHLER.with_borrow_mut(|z| {
        zaehle(
            z,
            funktion,
            gleich,
            || {
                if zeige_werte {
                    let kurz = |v: String| v.chars().take(600).collect::<String>();
                    format!(
                        " rust={} python={}",
                        kurz(rust.to_string()),
                        kurz(format!("{python:?}"))
                    )
                } else {
                    String::new()
                }
            },
            kontext,
        );
    });
    gleich
}

fn zaehle(
    z: &mut Zaehler,
    funktion: &str,
    gleich: bool,
    detail: impl FnOnce() -> String,
    kontext: &str,
) {
    let e = z.je_fn.entry(funktion.to_owned()).or_default();
    e.0 += 1;
    if !gleich {
        e.1 += 1;
        if z.beispiele.len() < 12 {
            z.beispiele
                .push(format!("{funktion} [{kontext}]{}", detail()));
        }
    }
}

fn bericht(titel: &str) -> usize {
    ZAEHLER.with_borrow(|z| {
        let mut summe = 0;
        for (f, (n, d)) in &z.je_fn {
            println!("interview-paritaet {titel}: {f:<42} Rust {n:>6} / Python {n:>6} Aufrufe, {d} Abweichungen");
            summe += d;
        }
        for (f, n) in &z.abdeckung {
            println!("interview-paritaet {titel}: Abdeckung {f:<36} in {n} Stores");
        }
        for b in &z.beispiele {
            println!("  ABWEICHUNG {b}");
        }
        summe
    })
}

fn js<T: serde::Serialize>(x: &T) -> Value {
    serde_json::to_value(x).expect("serialisierbar")
}

// ------------------------------------------------------------------ ein Store, alle Funktionen

/// Zusatz-Eingaben je Store.
struct Extras<'a> {
    beitrag: Option<HashMap<String, i64>>,
    kegel: Option<Vec<&'a str>>,
    rollen_mit_store: bool,
    snapshot_id: Option<&'a str>,
}

fn sicht_aus(felder: Option<&[&str]>) -> Sicht<'static> {
    felder.map_or_else(
        || graph().alle().clone(),
        |f| graph().sicht(f.iter().copied()).expect("sicht"),
    )
}

/// Die Store-Datei fuer Rust. P10: `domain::HerkunftVektor` laedt beide realen Formen direkt —
/// die volle `Herkunft` UND die 990 Alt-Events (32 der 192 realen Dateien) ohne `pruef_tiefe`/
/// `haftung` — keine im Speicher ergaenzte Kopie mehr noetig.
fn rust_datei(raw: &Value) -> StoreDatei {
    serde_json::from_value(raw.clone())
        .expect("StoreDatei laedt (reale_faelle_paritaet filtert vorher)")
}

fn pruefe_store(
    raw: &Value,
    felder: Option<&[&str]>,
    ex: &Extras<'_>,
    kontext: &str,
    zeige: bool,
) -> bool {
    let datei = rust_datei(raw);
    let store = Store::aus_datei(datei);
    let (g, s) = (graph(), sicht_aus(felder));
    let mut ok = true;
    let mut v = |f: &str, rust: Value, python: Result<Value, String>| {
        ok &= vergleiche(f, &rust, &python, kontext, zeige);
    };

    v(
        "traverser.relevanz",
        js(&interview::relevanz(&store, &s, g)),
        py("traverser.relevanz", Some(raw), felder, json!({})),
    );
    let ohne: Option<&HashMap<String, i64>> = None;
    v(
        "traverser.naechste_fragen",
        js(&interview::naechste_fragen(&store, &s, g, ohne)),
        py("traverser.naechste_fragen", Some(raw), felder, json!({})),
    );
    if let Some(b) = &ex.beitrag {
        v(
            "traverser.naechste_fragen(beitrag)",
            js(&interview::naechste_fragen(&store, &s, g, Some(b))),
            py(
                "traverser.naechste_fragen",
                Some(raw),
                felder,
                json!({ "beitrag": b }),
            ),
        );
    }
    v(
        "traverser.gate_gewicht",
        js(&interview::gate_gewicht(&s, g)),
        py("traverser.gate_gewicht", None, felder, json!({})),
    );

    let mut jf: Vec<&str> = store.aktive().map(|(f, _)| f).collect();
    jf.sort_unstable();
    jf.extend(["gibt_es_nicht", "kind_vorname__2"]);
    let rust_j: Vec<_> = jf
        .iter()
        .map(|f| interview::justification(&store, f, &s))
        .collect();
    // P10: `Justification.herkunft` ist `HerkunftVektor` (Passthrough des Store-Events) — auch
    // fuer die 32 realen Alt-Dateien vergleichbar, kein Uebersprungen mehr noetig.
    v(
        "traverser.justification",
        js(&rust_j),
        py(
            "traverser.justification",
            Some(raw),
            felder,
            json!({ "feld_ids": jf }),
        ),
    );
    v(
        "traverser.trace_ergebnis",
        js(&interview::trace_ergebnis(&store, &s, ex.snapshot_id)),
        py(
            "traverser.trace_ergebnis",
            Some(raw),
            felder,
            json!({ "snapshot_id": ex.snapshot_id }),
        ),
    );

    let mut af: Vec<&str> = s
        .iter()
        .filter(|b| b.instanz_gruppe.is_some())
        .map(|b| b.feld_id.as_str())
        .collect();
    af.extend(["veranlagung", "gibt_es_nicht"]);
    let rust_a: Vec<Value> = af
        .iter()
        .map(|f| interview::instanz_anzahl(&store, &s, g, f))
        .map(|(n, e)| json!([n.get(), e]))
        .collect();
    v(
        "traverser.instanz_anzahl",
        Value::Array(rust_a),
        py(
            "traverser.instanz_anzahl",
            Some(raw),
            felder,
            json!({ "feld_ids": af }),
        ),
    );

    let (snap, _) = store.materialisiere(None).expect("materialisiere");
    v(
        "traverser.fehlende_instanzen",
        js(&interview::fehlende_instanzen(&snap, &s, g)),
        py("traverser.fehlende_instanzen", Some(raw), felder, json!({})),
    );
    ok &= pruefe_rollen(raw, &store, felder, &s, ex, kontext, zeige);
    ok
}

fn pruefe_rollen(
    raw: &Value,
    store: &Store,
    felder: Option<&[&str]>,
    s: &Sicht<'static>,
    ex: &Extras<'_>,
    kontext: &str,
    zeige: bool,
) -> bool {
    let g = graph();
    let st = ex.rollen_mit_store.then_some(store);
    let raw_st = ex.rollen_mit_store.then_some(raw);
    let kegel = ex.kegel.as_deref();
    let args = json!({ "kegel": kegel });
    let mut ok = true;
    let rel = interview::relevante_kegel_felder(kegel.unwrap_or(&[]), s, st, g);
    ok &= vergleiche(
        "bindung_rollen.relevante_kegel_felder",
        &js(&rel),
        &py(
            "bindung_rollen.relevante_kegel_felder",
            raw_st,
            felder,
            args.clone(),
        ),
        kontext,
        zeige,
    );
    let ring: Vec<&str> = interview::ring_bindung(kegel, s, st, g)
        .0
        .feld_ids()
        .collect();
    ok &= vergleiche(
        "bindung_rollen.ring_bindung",
        &js(&ring),
        &py("bindung_rollen.ring_bindung", raw_st, felder, args.clone()),
        kontext,
        zeige,
    );
    let (aufbau, achsen) = interview::rollen(kegel, s, st, g);
    let rust = json!({
        "aufbau": aufbau.0.feld_ids().collect::<Vec<_>>(),
        "achsen": achsen.0.feld_ids().collect::<Vec<_>>(),
    });
    ok &= vergleiche(
        "bindung_rollen.rollen",
        &rust,
        &py("bindung_rollen.rollen", raw_st, felder, args),
        kontext,
        zeige,
    );
    ok
}

/// Der Interview-Ablauf: Queue nach jedem Event-Praefix.
fn pruefe_praefixe(raw: &Value, felder: Option<&[&str]>, kontext: &str) -> bool {
    let datei = rust_datei(raw);
    let s = sicht_aus(felder);
    let python = py("traverser.praefix_fragen", Some(raw), felder, json!({}));
    let Ok(Value::Array(py_listen)) = python else {
        return vergleiche(
            "traverser.naechste_fragen(praefix)",
            &Value::Null,
            &python,
            kontext,
            false,
        );
    };
    let mut ok = true;
    for k in 0..=datei.events.len() {
        let teil = StoreDatei {
            events: datei.events[..k].to_vec(),
            ..datei.clone()
        };
        let ohne: Option<&HashMap<String, i64>> = None;
        let rust = js(&interview::naechste_fragen(
            &Store::aus_datei(teil),
            &s,
            graph(),
            ohne,
        ));
        let p = py_listen.get(k).cloned().ok_or_else(|| "fehlt".to_owned());
        ok &= vergleiche(
            "traverser.naechste_fragen(praefix)",
            &rust,
            &p,
            &format!("{kontext} praefix {k}"),
            false,
        );
    }
    ok
}

// ------------------------------------------------------------------ Tests

#[test]
fn lader_paritaet() {
    if skip_ohne_parity_env() {
        return;
    }
    let g = graph();
    let ok = |f: &str| py(f, None, None, json!({}));
    vergleiche(
        "traverser.lade_themen_zuerst",
        &js(&g.themen_zuerst()),
        &ok("traverser.lade_themen_zuerst"),
        "-",
        true,
    );

    let mut gruppen = serde_json::Map::new();
    let mut bed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (_, d) in &registry().dateien {
        for gr in &d.instanz_gruppen {
            gruppen.insert(
                gr.gruppe.clone(),
                json!({"gruppe": gr.gruppe, "anzahl_feld": gr.anzahl_feld, "etikett": gr.etikett, "max": gr.max, "grund": gr.grund}),
            );
        }
        for rb in &d.regel_bedingungen {
            let e = json!({"regel_id": rb.regel_id, "feld": rb.feld, "wert": rb.wert, "grund": rb.grund});
            bed.entry(rb.regel_id.clone())
                .or_default()
                .push(e.to_string());
        }
    }
    vergleiche(
        "traverser.lade_instanz_gruppen",
        &Value::Object(gruppen),
        &ok("traverser.lade_instanz_gruppen"),
        "-",
        true,
    );

    // regel_bedingungen: Reihenfolge je Regel folgt der Datei-Reihenfolge -> als Multimenge.
    let norm = |m: BTreeMap<String, Vec<String>>| -> Value {
        js(&m
            .into_iter()
            .map(|(k, mut v)| {
                v.sort();
                (k, v)
            })
            .collect::<BTreeMap<_, _>>())
    };
    let py_bed = ok("traverser.lade_regel_bedingungen").map(|v| {
        let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (rid, conds) in v.as_object().into_iter().flatten() {
            m.insert(
                rid.clone(),
                conds
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(Value::to_string)
                    .collect(),
            );
        }
        norm(m)
    });
    vergleiche(
        "traverser.lade_regel_bedingungen",
        &norm(bed),
        &py_bed,
        "-",
        true,
    );

    // lade_bindung: Schluesselmenge gleich; Reihenfolge (glob vs. sortiert) nur berichtet.
    let py_keys = ok("traverser.lade_bindung").expect("lade_bindung");
    let mut p: Vec<String> = serde_json::from_value(py_keys.clone()).expect("liste");
    let r_ord: Vec<&str> = g.alle().feld_ids().collect();
    let gleiche_ordnung = p.iter().map(String::as_str).eq(r_ord.iter().copied());
    p.sort();
    let mut r = r_ord.clone();
    r.sort_unstable();
    vergleiche(
        "traverser.lade_bindung(schluesselmenge)",
        &js(&r),
        &Ok(js(&p)),
        "-",
        true,
    );
    println!("interview-paritaet lader: lade_bindung {} Felder, Python-glob-Reihenfolge == Rust-Reihenfolge: {gleiche_ordnung}", p.len());

    // instanz_feld_id: basis fuer i=1, basis__i ab 2.
    let paare: Vec<(String, u16)> = g
        .alle()
        .feld_ids()
        .take(40)
        .flat_map(|f| [(f.to_owned(), 1), (f.to_owned(), 2), (f.to_owned(), 11)])
        .collect();
    let rust: Vec<String> = paare
        .iter()
        .map(|(b, i)| {
            interview::instanz_feld_id(
                &BasisId::new(b.as_str()).unwrap(),
                NonZeroU16::new(*i).unwrap(),
            )
            .to_string()
        })
        .collect();
    vergleiche(
        "traverser.instanz_feld_id",
        &js(&rust),
        &py(
            "traverser.instanz_feld_id",
            None,
            None,
            json!({ "paare": paare }),
        ),
        "-",
        true,
    );
    assert_eq!(bericht("lader"), 0, "Lader-Abweichungen");
}

fn reale_faelle() -> Vec<Value> {
    let Ok(rd) = std::fs::read_dir(faelle_verzeichnis()) else {
        return Vec::new();
    };
    let mut pfade: Vec<_> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    pfade.sort();
    pfade
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .filter_map(|t| serde_json::from_str::<Value>(&t).ok())
        .filter(|v| v.get("events").is_some())
        .collect()
}

#[test]
fn reale_faelle_paritaet() {
    if skip_ohne_parity_env() {
        return;
    }
    // P10: store::lade laedt inzwischen ALLE realen Faelle (legacy Herkunft, unbegrenzte VZ);
    // `rust_datei` schlaegt hart fehl, statt still zu ueberspringen.
    let faelle = reale_faelle();
    let (mut events, mut mit_scheibe, mut alt) = (0, 0, 0);
    for (i, raw) in faelle.iter().enumerate() {
        let datei = rust_datei(raw);
        events += datei.events.len();
        alt += datei
            .events
            .iter()
            .filter(|e| e.herkunft.als_voll().is_none())
            .count();
        let scheibe = raw
            .get("scheibe")
            .and_then(Value::as_str)
            .and_then(|s| scheiben().get(s));
        let felder: Option<Vec<&str>> =
            scheibe.map(|(f, _)| f.iter().map(String::as_str).collect());
        mit_scheibe += usize::from(felder.is_some());
        let kegel: Option<Vec<&str>> = scheibe
            .and_then(|(_, k)| k.as_ref())
            .map(|k| k.iter().map(String::as_str).collect());
        let beitrag: HashMap<String, i64> = felder
            .iter()
            .flatten()
            .enumerate()
            .map(|(n, f)| ((*f).to_owned(), i64::try_from(n % 7).unwrap() * 1000))
            .collect();
        let ex = Extras {
            beitrag: Some(beitrag),
            kegel,
            rollen_mit_store: true,
            snapshot_id: Some("s"),
        };
        pruefe_store(raw, None, &ex, &format!("fall {i} voll"), false);
        pruefe_store(
            raw,
            felder.as_deref(),
            &ex,
            &format!("fall {i} scheibe"),
            false,
        );
        pruefe_praefixe(raw, felder.as_deref(), &format!("fall {i}"));
    }
    println!(
        "interview-paritaet real: {} Faelle, {mit_scheibe} mit bekannter Scheibe, {events} Events ({alt} Alt-Herkunft)",
        faelle.len()
    );
    assert_eq!(bericht("real"), 0, "Abweichungen auf realen Faellen");
}

#[test]
fn negativkontrolle() {
    if skip_ohne_parity_env() {
        return;
    }
    let raw = json!({"version": 1, "veranlagungszeitraum": 2025, "events": []});
    let store = Store::aus_datei(serde_json::from_value(raw.clone()).unwrap());
    let ohne: Option<&HashMap<String, i64>> = None;
    let mut q = interview::naechste_fragen(&store, graph().alle(), graph(), ohne);
    let py_q = py("traverser.naechste_fragen", Some(&raw), None, json!({}));
    assert_eq!(
        py_q.as_ref().ok(),
        Some(&js(&q)),
        "Ausgangslage muss gleich sein"
    );
    q.swap(0, 1);
    assert_ne!(
        py_q.as_ref().ok(),
        Some(&js(&q)),
        "vertauschte Queue muss abweichen"
    );
    let mut rel = js(&interview::relevanz(&store, graph().alle(), graph()));
    let py_rel = py("traverser.relevanz", Some(&raw), None, json!({}));
    assert_eq!(py_rel.as_ref().ok(), Some(&rel));
    rel["p2_festzusetzung_zusammen"]["status"] = json!("ausgeschlossen");
    assert_ne!(
        py_rel.as_ref().ok(),
        Some(&rel),
        "gestoerter Status muss abweichen"
    );
    println!("interview-paritaet negativkontrolle: vertauschte Queue und gestoerter Status als Abweichung erkannt");
}

// ------------------------------------------------------------------ Generator

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn byte(&mut self) -> u8 {
        let b = self.bytes.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b
    }
    fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (usize::from(self.byte()) * 256 + usize::from(self.byte())) % n
        }
    }
    fn waehle<'p, T>(&mut self, pool: &'p [T]) -> Option<&'p T> {
        pool.get(self.range(pool.len()))
    }
}

struct Pools {
    gates: Vec<&'static Bindung>,
    bedingungsfelder: Vec<&'static Bindung>,
    zaehlfelder: Vec<&'static Bindung>,
    gruppenfelder: Vec<&'static Bindung>,
    vorjahr: Vec<&'static Bindung>,
    alle: Vec<&'static Bindung>,
    besondere: HashMap<&'static str, Vec<Value>>,
}

fn pools() -> &'static Pools {
    static CELL: OnceLock<Pools> = OnceLock::new();
    CELL.get_or_init(|| {
        let g = graph();
        let alle: Vec<&'static Bindung> = g.alle().iter().collect();
        let mut besondere: HashMap<&str, Vec<Value>> = HashMap::new();
        for (_, d) in &registry().dateien {
            for rb in &d.regel_bedingungen {
                besondere
                    .entry(rb.feld.as_str())
                    .or_default()
                    .push(rb.wert.clone());
            }
        }
        for b in &alle {
            if let Some(fb) = &b.feld_bedingung {
                let e = besondere.entry(fb.feld.as_str()).or_default();
                e.extend(fb.wert.clone());
                e.extend(fb.wert_nicht.clone());
            }
        }
        let zaehl: Vec<&str> = registry()
            .dateien
            .iter()
            .flat_map(|(_, d)| d.instanz_gruppen.iter().map(|x| x.anzahl_feld.as_str()))
            .collect();
        Pools {
            gates: alle
                .iter()
                .copied()
                .filter(|b| {
                    b.askable
                        && matches!(b.quelle.bindungspunkt, Bindungspunkt::Geltungsbedingung(_))
                })
                .collect(),
            bedingungsfelder: alle
                .iter()
                .copied()
                .filter(|b| besondere.contains_key(b.feld_id.as_str()))
                .collect(),
            zaehlfelder: alle
                .iter()
                .copied()
                .filter(|b| zaehl.contains(&b.feld_id.as_str()))
                .collect(),
            gruppenfelder: alle
                .iter()
                .copied()
                .filter(|b| b.instanz_gruppe.is_some())
                .collect(),
            vorjahr: alle
                .iter()
                .copied()
                .filter(|b| b.vorjahr == Some(Vorjahr::Uebernehmbar))
                .collect(),
            alle,
            besondere,
        }
    })
}

fn wert_fuer(c: &mut Cursor<'_>, b: &Bindung, p: &Pools) -> Value {
    let falsch = [
        json!(1),
        json!(0),
        json!(1.0),
        json!(2.0),
        json!("ja"),
        Value::Null,
        json!(true),
        json!(false),
        json!("3"),
        json!(-1),
        json!(3),
    ];
    match c.range(10) {
        0..=4 => match b.typ {
            Feldtyp::Bool => json!(c.range(2) == 0),
            Feldtyp::Int | Feldtyp::Cent => json!(i64::try_from(c.range(13)).unwrap()),
            Feldtyp::Enum => b
                .enum_werte
                .as_ref()
                .and_then(|w| c.waehle(w))
                .map_or(json!("x"), |w| json!(w)),
            Feldtyp::Datum => json!("05.05.1990"),
            Feldtyp::Text => json!("text"),
        },
        5 | 6 => p
            .besondere
            .get(b.feld_id.as_str())
            .and_then(|w| c.waehle(w))
            .cloned()
            .unwrap_or(json!(true)),
        _ => c.waehle(&falsch).cloned().unwrap_or(Value::Null),
    }
}

fn feld_fuer(c: &mut Cursor<'_>, p: &Pools) -> String {
    let pool = match c.range(20) {
        0..=5 => &p.gates,
        6..=9 => &p.bedingungsfelder,
        10 | 11 => &p.zaehlfelder,
        12..=14 => &p.gruppenfelder,
        15 => &p.vorjahr,
        16..=18 => &p.alle,
        _ => {
            return [
                "gibt_es_nicht",
                "kind_vorname__1",
                "kind_vorname__02",
                "kind_vorname__0",
                "x__2",
            ][c.range(5)]
            .to_owned()
        }
    };
    let b = c.waehle(pool).unwrap_or(&p.alle[0]);
    if b.instanz_gruppe.is_some() && c.range(2) == 0 {
        format!("{}__{}", b.feld_id, 2 + c.range(10))
    } else {
        b.feld_id.clone()
    }
}

fn erzeuge_store(c: &mut Cursor<'_>) -> StoreDatei {
    let p = pools();
    let mut events: Vec<Event> = Vec::new();
    for i in 0..c.range(45) {
        let feld_id = feld_fuer(c, p);
        let basis = feld_id.split("__").next().unwrap_or(&feld_id).to_owned();
        let wert = graph()
            .alle()
            .get(&basis)
            .map_or(json!(1), |b| wert_fuer(c, b, p));
        let herkunft = Herkunft {
            herkunft: Achsenwert::new(if c.range(4) == 0 { "vorjahr" } else { "laie" }).unwrap(),
            pruef_tiefe: [
                PruefTiefe::Ungeprueft,
                PruefTiefe::Plausibilisiert,
                PruefTiefe::Amtlich,
            ][c.range(3)],
            haftung: Achsenwert::new("nutzer").unwrap(),
        };
        let signal = match c.range(3) {
            0 => None,
            1 => Some(Signal {
                signal_1: Some(Some(json!("a"))),
                signal_2: Some("b".to_owned()),
            }),
            _ => Some(Signal {
                signal_1: Some(None),
                signal_2: None,
            }),
        };
        let ersetzt = if i > 0 && c.range(5) == 0 {
            events.get(c.range(i)).map(|e| e.event_id)
        } else {
            None
        };
        let mut ev = Event {
            event_id: EventId::aus_bytes([0; 32]),
            ts: format!("2026-09-29T10:00:{:02}Z", i % 60),
            feld_id,
            wert,
            zustand: if c.range(2) == 0 {
                Zustand::Bestaetigt
            } else {
                Zustand::Vorlaeufig
            },
            herkunft: herkunft.into(),
            schreiber: if c.range(3) == 0 {
                Schreiber::ImportVorjahr
            } else {
                Schreiber::Mensch("julius".to_owned())
            },
            signal,
            ersetzt,
        };
        ev.event_id = ev.berechne_event_id();
        events.push(ev);
    }
    StoreDatei {
        version: 1,
        veranlagungszeitraum: Veranlagungsjahr(2025),
        fall_id: None,
        scheibe: None,
        user_id: None,
        events,
        snapshots: Vec::new(),
        vorjahr_referenz: None,
    }
}

/// Sicht: voll, eine Scheibe oder eine zufaellige Teilmenge in zufaelliger Reihenfolge.
fn erzeuge_felder(c: &mut Cursor<'_>) -> Option<Vec<&'static str>> {
    match c.range(3) {
        0 => None,
        1 => {
            let sch: Vec<&(Vec<String>, Option<Vec<String>>)> = scheiben().values().collect();
            c.waehle(&sch)
                .map(|(f, _)| f.iter().map(String::as_str).collect())
        }
        _ => {
            let mut teil: Vec<(usize, &'static str)> = Vec::new();
            for f in graph().alle().feld_ids() {
                if c.range(3) == 0 {
                    teil.push((c.range(10_000), f));
                }
            }
            teil.sort_unstable();
            Some(teil.into_iter().map(|(_, f)| f).collect())
        }
    }
}

fn erzeuge_extras(c: &mut Cursor<'_>, felder: &[&'static str]) -> Extras<'static> {
    let beitrag = match c.range(6) {
        0..=2 => None,
        3 => Some(HashMap::new()),
        _ => {
            let mut m = HashMap::new();
            for f in felder {
                if c.range(3) == 0 {
                    m.insert((*f).to_owned(), i64::try_from(c.range(5)).unwrap() * 100);
                }
            }
            Some(m)
        }
    };
    let kegel = match c.range(4) {
        0 => None,
        1 => Some(Vec::new()),
        2 => {
            let k: Vec<&Vec<String>> = scheiben()
                .values()
                .filter_map(|(_, k)| k.as_ref())
                .collect();
            c.waehle(&k).map(|k| k.iter().map(String::as_str).collect())
        }
        _ => {
            let mut k: Vec<&str> = felder.iter().copied().filter(|_| c.range(4) == 0).collect();
            k.extend(["gibt_es_nicht", "veranlagung", "veranlagung"]);
            Some(k)
        }
    };
    Extras {
        beitrag,
        kegel,
        rollen_mit_store: c.range(4) != 0,
        snapshot_id: if c.range(2) == 0 { None } else { Some("snap") },
    }
}

/// Welche Zweige der Generator trifft (je Merkmal: Zahl der Stores, in denen es auftritt).
fn abdeckung(datei: &StoreDatei, felder: Option<&[&'static str]>, ex: &Extras<'_>) {
    use interview::Regelstatus as R;
    let store = Store::aus_datei(datei.clone());
    let (g, s) = (graph(), sicht_aus(felder));
    let rel = interview::relevanz(&store, &s, g);
    let hat = |st: R| rel.values().any(|r| r.status == st);
    let voll_q: Vec<&str> = {
        let ohne: Option<&HashMap<String, i64>> = None;
        interview::naechste_fragen(&store, &s, g, ohne)
    };
    let offene_askable = s
        .iter()
        .filter(|b| {
            b.askable
                && store
                    .aktives(&b.feld_id)
                    .is_none_or(|e| e.zustand != Zustand::Bestaetigt)
        })
        .count();
    let (snap, _) = store.materialisiere(None).expect("materialisiere");
    let merkmale = [
        ("regel ausgeschlossen", hat(R::Ausgeschlossen)),
        ("regel unentschieden", hat(R::Unentschieden)),
        (
            "gates_offen bei ausgeschlossen",
            rel.values()
                .any(|r| r.status == R::Ausgeschlossen && !r.gates_offen.is_empty()),
        ),
        (
            "queue kuerzer als offene askable",
            voll_q.len() < offene_askable,
        ),
        (
            "instanz_anzahl > 1",
            s.iter()
                .any(|b| interview::instanz_anzahl(&store, &s, g, &b.feld_id).0.get() > 1),
        ),
        (
            "fehlende_instanzen nicht leer",
            !interview::fehlende_instanzen(&snap, &s, g).is_empty(),
        ),
        (
            "ersetzt-Kette",
            datei.events.iter().any(|e| e.ersetzt.is_some()),
        ),
        (
            "beitrag nicht leer",
            ex.beitrag.as_ref().is_some_and(|b| !b.is_empty()),
        ),
        (
            "kegel nicht leer",
            ex.kegel.as_ref().is_some_and(|k| !k.is_empty()),
        ),
        ("sicht teilmenge", felder.is_some()),
    ];
    ZAEHLER.with_borrow_mut(|z| {
        for (name, ja) in merkmale {
            if ja {
                *z.abdeckung.entry(name.to_owned()).or_default() += 1;
            }
        }
    });
}

#[test]
fn generierte_paritaet() {
    if skip_ohne_parity_env() {
        return;
    }
    let mut runner = TestRunner::new(Config {
        cases: 1000,
        failure_persistence: None,
        ..Config::default()
    });
    let strategie = proptest::collection::vec(proptest::num::u8::ANY, 2000..6000);
    let n = std::cell::Cell::new(0usize);
    let ergebnis = runner.run(&strategie, |bytes| {
        let mut c = Cursor {
            bytes: &bytes,
            pos: 0,
        };
        let datei = erzeuge_store(&mut c);
        let felder = erzeuge_felder(&mut c);
        let alle: Vec<&'static str> = graph().alle().feld_ids().collect();
        let ex = erzeuge_extras(&mut c, felder.as_deref().unwrap_or(&alle));
        let raw = serde_json::to_value(&datei).unwrap();
        n.set(n.get() + 1);
        abdeckung(&datei, felder.as_deref(), &ex);
        if pruefe_store(
            &raw,
            felder.as_deref(),
            &ex,
            &format!("generiert {}", n.get()),
            true,
        ) {
            Ok(())
        } else {
            Err(TestCaseError::fail("Abweichung, s. Bericht"))
        }
    });
    println!(
        "interview-paritaet generiert: {} Stores (inkl. Shrinking)",
        n.get()
    );
    let summe = bericht("generiert");
    assert!(ergebnis.is_ok(), "{ergebnis:?}");
    assert_eq!(summe, 0);
}
