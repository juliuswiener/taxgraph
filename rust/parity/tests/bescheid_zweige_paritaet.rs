//! Parität `rust/bescheid` Schritt 7c (`zweige`: `bescheid_fn` mit allen vier Zweigen) gegen
//! `produkt/bescheid/bescheid_zweige.py::_bescheid_fn` über `tools/parity/bescheid_oracle.py`
//! (`bescheid.zweig`). EIN Orakel-Prozess je Binary.
//!
//! Das Orakel ruft `_bescheid_fn` wie `_feste_zahl` (`api.py:220-226`) und liefert neben dem Ergebnis
//! (Cent, SolZ-Container, `extras`) die Eingaben, aus denen es sie berechnet hat (`bindung_ids`,
//! `werte_ids`, `quelle`). Rust baut daraus GENAU dieselben Eingaben.
//!
//! Eingabequellen, je mit Vergleichszahl beider Seiten (Zeilen = Aufrufe):
//! - `reale_faelle`: jede Fall-Datei unter `faelle_verzeichnis()` × `nur_bestaetigt` {true,false} ×
//!   4 Quantitäten × {Scheiben-Bindung/Kegel, volle Bindung/Kegel, volle Bindung/alle Scheibenfelder}.
//! - `golden_faelle`: `rust/fixtures/golden_cases.json`, je Quantität, roh und ×100 für Cent-Felder.
//! - `generierte_faelle`: ≥ 1.000 je Quantität (Store aus Events nach Bindungs-Typen, vorläufige und
//!   fehlende Felder, Zusammenveranlagung, Renten A/B, negative Einkünfte, KiSt).
//! - `vorlaeufiges_optionales_feld_bewegt_nie`: bei `nur_bestaetigt=true` ändert ein vorläufiges
//!   Nicht-Kegel-/Nicht-Instanz-Feld den Betrag nie (Rust gegen Rust, dazu Python-Parität des Falls).
//!
//! Fehler-Parität: Python-Ausnahme ↔ Rust-`Err` je Aufruf; ist die Rust-Klasse bekannt und nicht die
//! Sammelklasse `CatalaError`, muss sie gleich heißen.
//! Negativkontrolle: `negativkontrolle_*` stört ein Rust-Ergebnis um 1 und verlangt genau eine
//! Abweichung; `PARITY_STOERUNG=1` schaltet dieselbe Störung in `reale_faelle` (Lauf wird rot).
//!
//! SICHERHEIT: reale Fälle sind echte Steuerdaten. Nur lokal lesen, nur Zählwerte ausgeben.
//!
//!   `PARITY`=1 `cargo` test -p parity --test `bescheid_zweige_paritaet` -- --nocapture
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
#![allow(
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::doc_markdown,
    clippy::struct_excessive_bools
)]

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};

use bescheid::zweige::{bescheid_fn, Extras, Umgebung};
use bescheid::{BescheidFehler, Felder};
use bindung::{Bindung, Params};
use domain::{Cent, Feldtyp, Vz};
use intervall::{AchsenBindung, SlotFehler, Werte};
use parity::Oracle;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use serde_json::{json, Map, Value};
use store::{EventId, Store, StoreDatei};

const QUANTITAETEN: [&str; 4] = [
    "abziehbarer_betrag",
    "festzusetzende_est",
    "festzusetzende_est_gesamt",
    "festzusetzende_est_rentner",
];

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn stoerung_an() -> bool {
    std::env::var("PARITY_STOERUNG").as_deref() == Ok("1")
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

fn params() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(|| Params::lade(&repo_root()).expect("params laden"))
}

fn bindungen() -> &'static [Bindung] {
    static B: OnceLock<Vec<Bindung>> = OnceLock::new();
    B.get_or_init(|| {
        let reg = bindung::lade_registry(&repo_root().join("produkt/bindung")).expect("registry");
        reg.dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn oracle() -> std::sync::MutexGuard<'static, Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn frage(req: &Value) -> Value {
    let antwort = oracle().call_json(req).expect("Orakel antwortet");
    match antwort.get("ok") {
        Some(v) => v.clone(),
        None => panic!("Orakel-Harness-Fehler für {}: {antwort}", req["fn"]),
    }
}

/// Scheiben-Feldlisten je Quantität aus `api_constants.SCHEIBEN`.
fn scheiben() -> &'static Value {
    static S: OnceLock<Value> = OnceLock::new();
    S.get_or_init(|| frage(&json!({"fn": "bescheid.scheiben"})))
}

fn feld_ids(q: &str, art: &str) -> Vec<String> {
    scheiben()[q][art]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

fn bindung_zu(fid: &str) -> Option<&'static Bindung> {
    static I: OnceLock<HashMap<&'static str, &'static Bindung>> = OnceLock::new();
    I.get_or_init(|| {
        bindungen()
            .iter()
            .map(|b| (b.feld_id.as_str(), b))
            .collect()
    })
    .get(fid)
    .copied()
}

/// Bindungs-Sicht (Achsen + Index) je Feld-ID-Menge, einmal gebaut.
struct Umg {
    achsen: Vec<AchsenBindung>,
    index: HashMap<String, &'static Bindung>,
}

fn umg_fuer(ids: &[String]) -> &'static Umg {
    static C: OnceLock<Mutex<HashMap<u64, &'static Umg>>> = OnceLock::new();
    let schluessel = ids.iter().fold(1_469_598_103_934_665_603_u64, |h, s| {
        s.bytes()
            .chain(std::iter::once(0))
            .fold(h, |h, b| (h ^ u64::from(b)).wrapping_mul(1_099_511_628_211))
    });
    let mut cache = C
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.entry(schluessel).or_insert_with(|| {
        let subset: Vec<&'static Bindung> = ids.iter().filter_map(|i| bindung_zu(i)).collect();
        assert_eq!(subset.len(), ids.len(), "Bindung fehlt in Rust");
        Box::leak(Box::new(Umg {
            achsen: subset.iter().map(|b| AchsenBindung::from(*b)).collect(),
            index: subset.iter().map(|b| (b.feld_id.clone(), *b)).collect(),
        }))
    })
}

// ---------------------------------------------------------------- Fall

/// Ein Vergleichsfall: derselbe Kontext geht an Python (JSON) und an Rust.
#[derive(Clone)]
struct Fall {
    q: &'static str,
    store: Option<Value>,
    felder: Option<Value>,
    vz: u16,
    nur: bool,
    /// `"scheibe"` (wie `/ergebnis`) oder `"voll"`.
    bindung: &'static str,
    /// `"kegel"` (wie `/ergebnis`) oder `"alle"`.
    werte: &'static str,
    store_uebergeben: bool,
    solz: bool,
    extras: bool,
}

impl Fall {
    fn request(&self) -> Value {
        json!({"fn": "bescheid.zweig", "quantitaet": self.q, "store": self.store, "felder": self.felder,
            "vz": self.vz, "nur_bestaetigt": self.nur, "bindung": self.bindung, "werte": self.werte,
            "store_uebergeben": self.store_uebergeben, "solz": self.solz, "extras": self.extras})
    }
}

fn extras_json(e: &Extras) -> Value {
    let mut m = Map::new();
    let mut cent = |k: &str, c: Option<Cent>| {
        if let Some(c) = c {
            m.insert(k.into(), json!(c.get()));
        }
    };
    cent("kist_cent", e.kist_cent);
    cent("kist_kap_cent", e.kist_kap_cent);
    cent("mobilitaetspraemie_cent", e.mobilitaetspraemie_cent);
    if let Some(b) = e.kap_guenstiger_gewonnen {
        m.insert("kap_guenstiger_gewonnen".into(), json!(b));
    }
    if let Some(k) = &e.kette {
        let mut km = Map::new();
        km.insert(
            "gesamtbetrag_der_einkuenfte".into(),
            json!(k.gesamtbetrag_der_einkuenfte.get()),
        );
        km.insert(
            "zu_versteuerndes_einkommen".into(),
            json!(k.zu_versteuerndes_einkommen.get()),
        );
        km.insert("tarifliche_est".into(), json!(k.tarifliche_est.get()));
        km.insert(
            "festzusetzende_est".into(),
            json!(k.festzusetzende_est.get()),
        );
        if let Some(p) = &k.p31 {
            let sieger = match p.guenstiger {
                bescheid::zweige::P31Sieger::Freibetraege => "freibetraege",
                bescheid::zweige::P31Sieger::Kindergeld => "kindergeld",
            };
            km.insert(
                "p31".into(),
                json!({"guenstiger": sieger, "kindergeld": p.kindergeld.get(), "text": p.text}),
            );
        }
        m.insert("kette".into(), Value::Object(km));
    }
    Value::Object(m)
}

/// Python-Klasse zum Fehler (`None`: Python kennt hier keinen Fehler).
fn klasse(e: &SlotFehler<BescheidFehler>) -> Option<&'static str> {
    match e {
        SlotFehler::Slot(b) => b.python_klasse(),
        SlotFehler::UnbekanntesFeld(_) => Some("KeyError"),
        SlotFehler::SummandNichtGanzzahl(_) => Some("TypeError"),
        SlotFehler::Ueberlauf(_) => None,
    }
}

/// Ergebnis der Rust-Seite: `None` = kein Accessor, `Some(Ok(json))`, `Some(Err(klasse))`.
type RustErgebnis = Option<Result<Value, (Option<&'static str>, String)>>;

fn rust_lauf(fall: &Fall, antwort: &Value) -> RustErgebnis {
    let (felder, store): (Felder, Option<Store>) = match (&fall.store, &fall.felder) {
        (Some(s), _) => {
            let datei: StoreDatei =
                serde_json::from_value(s.clone()).expect("Store-Datei deserialisiert");
            let st = Store::aus_datei(datei);
            (st.materialisiere(None).expect("materialisiere").0, Some(st))
        }
        (None, Some(fe)) => (serde_json::from_value(fe.clone()).expect("felder"), None),
        (None, None) => (Felder::new(), None),
    };
    let ids = |k: &str| -> Vec<String> {
        antwort[k]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect()
    };
    let umg = umg_fuer(&ids("bindung_ids"));
    let umgebung = Umgebung {
        achsen: &umg.achsen,
        index: &umg.index,
        params: params(),
    };
    let mut werte = Werte::neu();
    for fid in ids("werte_ids") {
        werte.setze(&fid, felder[&fid].wert.clone());
    }
    let solz = Cell::new(None);
    let extras = RefCell::new(Extras::default());
    let vz = Vz::try_from(fall.vz).expect("vz 2024..2026");
    let bf = bescheid_fn(
        fall.q,
        vz,
        &umgebung,
        Some(&felder),
        if fall.store_uebergeben {
            store.as_ref()
        } else {
            None
        },
        fall.nur,
        fall.solz.then_some(&solz),
        fall.extras.then_some(&extras),
    )?;
    Some(match bf(&werte) {
        Ok(c) => Ok(json!({
            "zahl_cent": c.get(),
            "solz": if fall.solz { solz.get().map_or(Value::Null, |c| json!(c.get())) } else { Value::Null },
            "extras": if fall.extras { extras_json(&extras.borrow()) } else { Value::Null },
        })),
        Err(e) => Err((klasse(&e), e.to_string())),
    })
}

// ---------------------------------------------------------------- Bilanz

#[derive(Default)]
struct Zeile {
    fehler: BTreeMap<String, u64>,
    python: u64,
    rust: u64,
    ok_gleich: u64,
    err_gleich: u64,
    none_gleich: u64,
    nicht_leer: u64,
    mit_solz: u64,
    mit_kist: u64,
    mit_kette: u64,
    mit_p31: u64,
    mit_mobil: u64,
    mit_kap: u64,
    mit_kist_kap: u64,
    kap_tariflich: u64,
    abw: u64,
}

#[derive(Default)]
struct Bilanz {
    zeilen: BTreeMap<&'static str, Zeile>,
    abweichungen: Vec<String>,
}

/// Erhöht `zahl_cent` um 1 (Störung der Negativkontrolle).
fn stoere(v: &mut Value) -> bool {
    match v.get_mut("zahl_cent") {
        Some(z) if z.is_i64() => {
            *z = json!(z.as_i64().unwrap() + 1);
            true
        }
        _ => false,
    }
}

impl Bilanz {
    fn vergleiche(
        &mut self,
        fall: &Fall,
        py: &Value,
        mut rust: RustErgebnis,
        ort: &str,
        werte: bool,
        stoere_es: bool,
    ) -> bool {
        let mut gestoert = false;
        if stoere_es {
            if let Some(Ok(v)) = rust.as_mut() {
                gestoert = stoere(v);
            }
        }
        let z = self.zeilen.entry(fall.q).or_default();
        z.python += 1;
        z.rust += 1;
        let melde =
            |abw: &mut Vec<String>, was: String| abw.push(format!("{ort} {}: {was}", fall.q));
        let py_none = py.get("none").and_then(Value::as_bool) == Some(true);
        let py_err = py.get("err").and_then(Value::as_str);
        match (py_none, py_err, rust) {
            (true, _, None) => z.none_gleich += 1,
            (false, None, Some(Ok(r))) => {
                let p = json!({"zahl_cent": py["zahl_cent"], "solz": py["solz"], "extras": py["extras"]});
                if p["zahl_cent"].as_i64().is_some_and(|c| c != 0) {
                    z.nicht_leer += 1;
                }
                if p["solz"].as_i64().is_some_and(|c| c != 0) {
                    z.mit_solz += 1;
                }
                let ex = &p["extras"];
                z.mit_kist += u64::from(ex.get("kist_cent").is_some_and(|v| v.as_i64() != Some(0)));
                z.mit_kette += u64::from(ex.get("kette").is_some());
                z.mit_p31 += u64::from(ex.get("kette").is_some_and(|k| k.get("p31").is_some()));
                z.mit_mobil += u64::from(
                    ex.get("mobilitaetspraemie_cent")
                        .is_some_and(|v| v.as_i64() != Some(0)),
                );
                z.mit_kap += u64::from(ex.get("kap_guenstiger_gewonnen").is_some());
                z.mit_kist_kap += u64::from(
                    ex.get("kist_kap_cent")
                        .is_some_and(|v| v.as_i64() != Some(0)),
                );
                z.kap_tariflich +=
                    u64::from(ex.get("kap_guenstiger_gewonnen") == Some(&json!(true)));
                if p == r {
                    z.ok_gleich += 1;
                } else {
                    z.abw += 1;
                    melde(
                        &mut self.abweichungen,
                        if werte {
                            format!("py={p} rust={r}")
                        } else {
                            "Wert weicht ab".into()
                        },
                    );
                }
            }
            (false, Some(pk), Some(Err((rk, rm)))) => match rk {
                Some(k) if k != "CatalaError" && k != pk => {
                    z.abw += 1;
                    melde(
                        &mut self.abweichungen,
                        format!(
                            "Fehlerklasse rust={k} ({rm}) py={pk} ({})",
                            py.get("msg").unwrap_or(&Value::Null)
                        ),
                    );
                }
                _ => {
                    z.err_gleich += 1;
                    *z.fehler
                        .entry(format!(
                            "{pk}:{}",
                            py.get("msg")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .chars()
                                .take(40)
                                .collect::<String>()
                        ))
                        .or_default() += 1;
                }
            },
            (py_n, pe, r) => {
                z.abw += 1;
                let rs = match &r {
                    None => "None".to_owned(),
                    Some(Ok(_)) => "Ok".to_owned(),
                    Some(Err((k, m))) => format!("Err({k:?}: {m})"),
                };
                melde(
                    &mut self.abweichungen,
                    format!(
                        "Art weicht ab: py none={py_n} err={pe:?} {} / rust {rs}",
                        py.get("msg").unwrap_or(&Value::Null)
                    ),
                );
            }
        }
        gestoert
    }

    fn drucke(&self, titel: &str, faelle: usize) {
        eprintln!("== {titel}: {faelle} Fälle ==");
        eprintln!(
            "{:<27} {:>6} {:>6} {:>6} {:>6} {:>6} {:>7} {:>6} {:>6} {:>6} {:>6} {:>6} {:>5}",
            "quantitaet",
            "python",
            "rust",
            "ok=",
            "err=",
            "none=",
            "nicht-0",
            "solz",
            "kist",
            "kette",
            "p31",
            "mobil",
            "abw"
        );
        for (n, z) in &self.zeilen {
            eprintln!(
                "{n:<27} {:>6} {:>6} {:>6} {:>6} {:>6} {:>7} {:>6} {:>6} {:>6} {:>6} {:>6} {:>5}",
                z.python,
                z.rust,
                z.ok_gleich,
                z.err_gleich,
                z.none_gleich,
                z.nicht_leer,
                z.mit_solz,
                z.mit_kist,
                z.mit_kette,
                z.mit_p31,
                z.mit_mobil,
                z.abw
            );
        }
        for (n, z) in &self.zeilen {
            let mut top: Vec<_> = z.fehler.iter().collect();
            top.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
            let text: Vec<String> = top
                .iter()
                .take(5)
                .map(|(k, c)| format!("{c}x {k}"))
                .collect();
            eprintln!("  Fehler {n}: {}", text.join(" | "));
        }
        let kap: u64 = self.zeilen.values().map(|z| z.mit_kap).sum();
        let (kk, kt): (u64, u64) = self
            .zeilen
            .values()
            .fold((0, 0), |a, z| (a.0 + z.mit_kist_kap, a.1 + z.kap_tariflich));
        eprintln!(
            "§ 32d: Kapital-KiSt != 0 in {kk} Fällen, tariflicher Zweig gewann in {kt} Fällen"
        );
        eprintln!(
            "Abweichungen gesamt: {}  (davon Fälle mit § 32d-Kapital-Zweig: {kap})",
            self.abweichungen()
        );
        for a in self.abweichungen.iter().take(8) {
            eprintln!("  ! {a}");
        }
    }

    fn abweichungen(&self) -> u64 {
        self.zeilen.values().map(|z| z.abw).sum()
    }

    /// Waechter gegen einen gruenen Lauf, der nichts belegt: eine Vergleichszeile, die nie
    /// einen Wert GESEHEN hat (`nicht-0 == 0`), kann nicht "0 Abweichungen" beweisen -- gruen
    /// und leer sehen identisch aus. Rot mit dem Namen jeder solchen Zeile.
    ///
    /// Ticket `parity-lauf-gruen-ohne-dass-die-zeile-rechnet`. Der Waechter greift JE BLOCK:
    /// eine Zeile, die nur in `golden_faelle` rechnet, deckt die Luecke in `reale_faelle` nicht.
    fn wache_rechnet(&self, block: &str) {
        let leer: Vec<&str> = self
            .zeilen
            .iter()
            .filter(|(_, z)| z.nicht_leer == 0)
            .map(|(n, _)| *n)
            .collect();
        assert!(
            leer.is_empty(),
            "{block}: {} von {} Vergleichszeilen sahen NIE einen Wert (nicht-0 == 0): [{}] \
             -- ein gruener Lauf belegt fuer diese Zeilen nichts",
            leer.len(),
            self.zeilen.len(),
            leer.join(", ")
        );
    }
}

fn vergleiche_fall(b: &mut Bilanz, fall: &Fall, ort: &str, werte: bool, stoere_es: bool) -> bool {
    let py = frage(&fall.request());
    if let Some(q) = py.get("quelle").and_then(Value::as_str) {
        let wurzel = repo_root().canonicalize().unwrap();
        assert!(
            std::path::Path::new(q).starts_with(&wurzel),
            "Orakel lädt Python aus {q}, nicht aus {}",
            wurzel.display()
        );
    }
    let r = rust_lauf(fall, &py);
    b.vergleiche(fall, &py, r, ort, werte, stoere_es)
}

// ---------------------------------------------------------------- reale Fälle

#[test]
fn reale_faelle() {
    if skip() {
        return;
    }
    let dateien = walk_json(&faelle_verzeichnis());
    if dateien.is_empty() {
        eprintln!("reale_faelle: 0 Fall-Dateien — Korpus-Lücke, nicht verschwiegen");
        return;
    }
    let mut b = Bilanz::default();
    let (mut faelle, mut kein_store, mut vz_ersatz, mut mit_store) =
        (0usize, 0usize, 0usize, 0usize);
    let mut stoerung_offen = stoerung_an();
    for pfad in &dateien {
        let Ok(text) = std::fs::read_to_string(pfad) else {
            continue;
        };
        let Ok(roh) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if roh.get("events").is_none() {
            kein_store += 1;
            continue;
        }
        mit_store += 1;
        let vz = roh["veranlagungszeitraum"]
            .as_u64()
            .filter(|v| (2024..=2026).contains(v))
            .unwrap_or_else(|| {
                vz_ersatz += 1;
                2025
            }) as u16;
        for nur in [true, false] {
            for q in QUANTITAETEN {
                for (bindung, werte) in [("scheibe", "kegel"), ("voll", "kegel"), ("voll", "alle")]
                {
                    let fall = Fall {
                        q,
                        store: Some(roh.clone()),
                        felder: None,
                        vz,
                        nur,
                        bindung,
                        werte,
                        store_uebergeben: true,
                        solz: true,
                        extras: true,
                    };
                    if vergleiche_fall(&mut b, &fall, "real", false, stoerung_offen) {
                        stoerung_offen = false;
                    }
                    faelle += 1;
                }
            }
        }
    }
    b.drucke("reale_faelle", faelle);
    b.wache_rechnet("reale_faelle");
    eprintln!("reale_faelle: {} Dateien, {mit_store} mit Store, {kein_store} ohne Store übersprungen, {vz_ersatz} mit VZ außerhalb 2024–2026 (Ersatz 2025)", dateien.len());
    assert!(faelle > 0);
    assert_eq!(
        b.abweichungen(),
        0,
        "Abweichungen (Anzahl s. o., keine Werte ausgegeben)"
    );
}

// ---------------------------------------------------------------- Golden

/// Golden-Schlüssel, die im Ring anders heißen (`Catala`-Sachverhalt → Feld).
const GOLDEN_ALIAS: &[(&str, &str)] = &[
    ("einkuenfte_nichtselbststaendig", "bruttoarbeitslohn"),
    ("bruttoarbeitslohn_a", "bruttoarbeitslohn"),
    ("bruttoarbeitslohn_b", "bruttoarbeitslohn_partner"),
    ("einkuenfte_vermietung", "vv_einnahmen"),
    ("kinder_ganzjaehrig", "fam_anzahl_kinder"),
    ("einkuenfte_gewinn", "einkuenfte_gewinn"),
];

/// Neutraler Wert eines Kegel-Felds, das der Golden-Fall nicht nennt (der Ring liest den Slot sonst
/// mit `KeyError`, und die Zeile bliebe eine Fehler-Zeile).
fn neutral(b: &Bindung) -> Value {
    match b.feld_id.as_str() {
        "rentner_renten_art" => return json!("gesetzliche_rente"),
        "rentner_renten_beginn_jahr" => return json!(2020),
        _ => {}
    }
    match b.typ {
        Feldtyp::Cent | Feldtyp::Int => json!(if b.feld_id == "vv_entgelt_quote_prozent" {
            100
        } else {
            0
        }),
        Feldtyp::Bool => json!(false),
        Feldtyp::Enum => json!(b
            .enum_werte
            .clone()
            .unwrap_or_default()
            .first()
            .cloned()
            .unwrap_or_default()),
        Feldtyp::Datum => json!("01.01.2000"),
        Feldtyp::Text => json!(""),
    }
}

/// Golden-Sachverhalt → Feld-Snapshot: jeder Golden-Schlüssel füllt die Felder gleichen Namens, die
/// Felder, deren `signatur_slot` er ist, und die Alias-Felder; Scheiben-Felder ohne Nennung bekommen
/// neutrale Werte. Cent-Felder ×`faktor` (Golden rechnet in EURO).
fn golden_felder(sv: &Map<String, Value>, faktor: i64, q: &str) -> Value {
    let herkunft = json!({"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"});
    let mut out = Map::new();
    let eintrag =
        |wert: Value| json!({"wert": wert, "zustand": "bestaetigt", "herkunft": herkunft});
    for b in bindungen() {
        let slot = match &b.quelle.bindungspunkt {
            bindung::Bindungspunkt::SignaturSlot(s) => Some(s.as_str()),
            bindung::Bindungspunkt::Geltungsbedingung(_) => None,
        };
        let alias = GOLDEN_ALIAS
            .iter()
            .find(|(_, z)| *z == b.feld_id)
            .and_then(|(k, _)| sv.get(*k));
        let treffer = sv
            .get(&b.feld_id)
            .or_else(|| slot.and_then(|s| sv.get(s)))
            .or(alias)
            .filter(|v| v.is_number() || v.is_boolean() || v.is_string());
        let Some(v) = treffer else { continue };
        let wert = match (v.as_i64(), b.typ) {
            (Some(i), Feldtyp::Cent) => json!(i * faktor),
            _ => v.clone(),
        };
        out.insert(b.feld_id.clone(), eintrag(wert));
    }
    for fid in feld_ids(q, "felder") {
        if let (false, Some(b)) = (out.contains_key(&fid), bindung_zu(&fid)) {
            out.insert(fid, eintrag(neutral(b)));
        }
    }
    Value::Object(out)
}

#[test]
fn golden_faelle() {
    if skip() {
        return;
    }
    let text = std::fs::read_to_string(repo_root().join("rust/fixtures/golden_cases.json"))
        .expect("golden_cases.json");
    let faelle: Vec<Value> = serde_json::from_str(&text).unwrap();
    let mut b = Bilanz::default();
    let mut n = 0usize;
    for fall in &faelle {
        let sv = fall["sachverhalt"].as_object().unwrap();
        let vz = sv
            .get("veranlagungszeitraum")
            .and_then(Value::as_u64)
            .filter(|v| (2024..=2026).contains(v))
            .unwrap_or(2025) as u16;
        for faktor in [1_i64, 100] {
            for q in QUANTITAETEN {
                let felder = golden_felder(sv, faktor, q);
                for werte in ["kegel", "alle"] {
                    let f = Fall {
                        q,
                        store: None,
                        felder: Some(felder.clone()),
                        vz,
                        nur: true,
                        bindung: "voll",
                        werte,
                        store_uebergeben: true,
                        solz: true,
                        extras: true,
                    };
                    vergleiche_fall(&mut b, &f, "golden", true, false);
                    n += 1;
                }
            }
        }
    }
    b.drucke("golden_faelle", n);
    b.wache_rechnet("golden_faelle");
    assert_eq!(n, faelle.len() * 2 * QUANTITAETEN.len() * 2);
    assert_eq!(b.abweichungen(), 0);
}

// ---------------------------------------------------------------- Generator

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn byte(&mut self) -> u64 {
        let b = self
            .bytes
            .get(self.pos % self.bytes.len().max(1))
            .copied()
            .unwrap_or(0);
        self.pos += 1;
        u64::from(b)
    }
    fn range(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        ((self.byte() << 16) | (self.byte() << 8) | self.byte()) % n
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.range(100) < pct
    }
    fn waehle<'b, T>(&mut self, xs: &'b [T]) -> &'b T {
        &xs[self.range(xs.len() as u64) as usize]
    }
}

const GRENZEN: &[i64] = &[
    0, 1, 99, 100, 101, 25_000, 25_001, 80_000, 80_001, 99_900, 100_000, 250_000, 1_000_000,
    5_000_000, 20_000_000,
];

/// Ein Wert nach Bindungstyp; `wild` streut Falschtypen ein (fail-open-Pfade).
fn wert(c: &mut Cursor, b: &Bindung, wild: bool, bestaetigt_bias: u64) -> Value {
    if wild && c.chance(6) {
        return match c.range(7) {
            0 => Value::Null,
            1 => json!(true),
            2 => json!("12"),
            // Ein Float in einem Summanden-Slot ist die dokumentierte Grenze von `intervall::slots`
            // (Python rechnet `0 + 12.5`, Rust meldet `SummandNichtGanzzahl`); Store-Typpruefung (Auflage T)
            // schliesst ihn fuer `typ: cent` aus.
            3 if b.slot_beitrag == Some(bindung::SlotBeitrag::Summand) => json!(-7),
            3 => json!(c.range(2_000_000) as i64 as f64 + 0.5),
            4 => json!(-7),
            5 => json!([]),
            _ => json!(""),
        };
    }
    let name = b.feld_id.as_str();
    // Ring-typische Wertebereiche (ohne sie erzeugen Zufallsjahre fast nur Fehler-Zeilen).
    let basis = name.split("__").next().unwrap_or(name);
    if b.typ == Feldtyp::Int && basis.ends_with("_beginn_jahr") {
        return json!(2000 + c.range(27) as i64);
    }
    if b.typ == Feldtyp::Int && basis.starts_with("rentner_alter_bei_rentenbeginn") {
        return json!(55 + c.range(16) as i64);
    }
    if b.typ == Feldtyp::Int && basis.starts_with("geburtsjahr") {
        return json!(1935 + c.range(71) as i64);
    }
    if basis.starts_with("rentner_renten_art") && c.chance(85) {
        return json!(*c.waehle(&[
            "gesetzliche_rente",
            "private_leibrente",
            "private_basisrente",
            "berufsstaendische_versorgung",
            "sonstige_leibrente"
        ]));
    }
    match b.typ {
        Feldtyp::Cent => {
            let mut v = match c.range(20) {
                0..=3 => 0,
                4..=5 => *c.waehle(GRENZEN),
                6..=12 => c.range(200_000) as i64,
                13..=17 => c.range(3_000_000) as i64,
                _ => c.range(30_000_000) as i64,
            };
            if name == "bruttoarbeitslohn" && v == 0 {
                v = c.range(9_000_000) as i64;
            }
            // Negative Betraege (Verluste, Erstattungen): 8 %.
            if c.chance(8) {
                v = -v;
            }
            json!(v)
        }
        Feldtyp::Int => {
            let (lo, hi) = b.bereich.as_ref().map_or((0, 12), |r| (r.min, r.max));
            let (lo, hi) = (lo.max(-1000), hi.min(30_000));
            match c.range(10) {
                0 => json!(lo),
                1 => json!(hi),
                _ => json!(lo + c.range((hi - lo + 1).max(1) as u64) as i64),
            }
        }
        Feldtyp::Bool => json!(c.chance(if bestaetigt_bias > 0 { 60 } else { 40 })),
        Feldtyp::Enum => {
            let ev = b.enum_werte.clone().unwrap_or_default();
            if ev.is_empty() || (wild && c.chance(4)) {
                json!("unbekannt")
            } else {
                json!(c.waehle(&ev))
            }
        }
        Feldtyp::Datum => json!("15.03.2024"),
        Feldtyp::Text => json!(*c.waehle(&[
            "x",
            "",
            "12345678901",
            "bayern",
            "baden_wuerttemberg",
            "Österreich"
        ])),
    }
}

fn event(i: usize, fid: &str, wert: &Value, bestaetigt: bool) -> Value {
    let mut e = json!({
        "ts": format!("2026-01-01T{:02}:{:02}:{:02}+00:00", i / 3600, (i / 60) % 60, i % 60),
        "feld_id": fid, "wert": wert,
        "zustand": if bestaetigt { "bestaetigt" } else { "vorlaeufig" },
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": null, "signal_2": if bestaetigt { json!("ok") } else { Value::Null }},
        "ersetzt": null,
    });
    e["event_id"] = json!(EventId::von_json(&e).to_string());
    e
}

/// Zusätzliche Ring-Felder, die keine Scheibe listet (vom Ring aus dem Snapshot gelesen).
const EXTRA_FELDER: &[&str] = &[
    "kind_idnr",
    "kind_kv",
    "kind_pv",
    "kind_unter_14_haushaltszugehoerig",
    "kinderbetreuungskosten",
    "schulgeld",
    "hh_minijob_betrag",
    "hh_dienstleistung_betrag",
    "hh_handwerker_betrag",
    "gwg_anschaffungskosten_netto",
    "p23_veraeusserungspreis",
    "p23_anschaffung_herstellungskosten",
    "p23_werbungskosten",
    "kind_grad_der_behinderung",
    "kind_behinderten_pb_antrag",
    "kind_pb_nicht_selbst_genutzt",
    "kind_hilflos_blind_taubblind",
    "rentner_renten_art",
    "rentner_jahresrente",
    "rentner_renten_beginn_jahr",
    "rentner_alter_bei_rentenbeginn",
    "rentner_rentenfreibetrag",
    "rentner_renten_art_partner",
    "rentner_jahresrente_partner",
    "rentner_renten_beginn_jahr_partner",
    "rentner_alter_bei_rentenbeginn_partner",
    "rentner_rentenfreibetrag_partner",
    "kist_konfession",
    "kist_bundesland",
    "p32b_progressionseinkuenfte",
    "fam_alleinstehend",
    "fam_anzahl_kinder",
    "geburtsjahr",
    "geburtsjahr_partner",
    "gewst_messbetrag",
    "gewst_hebesatz",
];

/// Die Renten-Felder, ohne die `_rente_instanz` nur Fehler-Zeilen liefert; wie Kegel-Felder behandelt.
const RENTE_KERN: &[&str] = &[
    "rentner_renten_art",
    "rentner_jahresrente",
    "rentner_renten_beginn_jahr",
    "rentner_alter_bei_rentenbeginn",
    "rentner_rentenfreibetrag",
];

/// Erzeugt die Event-Liste (`(feld_id, wert, bestaetigt)`) eines Falls für die Quantität `q`.
fn generiere_events(c: &mut Cursor, q: &str, fest: &[(&str, Value)]) -> Vec<(String, Value, bool)> {
    let dichte = *c.waehle(&[20, 45, 75, 95]);
    let bestaetigt_pct = *c.waehle(&[60, 85, 100, 100]);
    let wild = c.chance(35);
    let kegel = feld_ids(q, "kegel");
    let mut ids: Vec<String> = feld_ids(q, "felder");
    ids.extend(EXTRA_FELDER.iter().map(|s| (*s).to_owned()));
    ids.sort();
    ids.dedup();
    let inst_zustand = [
        c.chance(bestaetigt_pct),
        c.chance(bestaetigt_pct),
        c.chance(bestaetigt_pct),
    ];
    let mut events: Vec<(String, Value, bool)> = Vec::new();
    for fid in &ids {
        let Some(b) = bindung_zu(fid) else { continue };
        let im_kegel = kegel.contains(fid) || RENTE_KERN.contains(&fid.as_str());
        if !c.chance(if im_kegel {
            if wild {
                96
            } else {
                100
            }
        } else {
            dichte
        }) {
            continue;
        }
        let gruppe = b.instanz_gruppe.is_some();
        let n_inst = if gruppe { 1 + c.range(3) as usize } else { 1 };
        for n in 1..=n_inst {
            let id = match n {
                1 if gruppe && c.chance(5) => format!("{fid}__1"),
                1 => fid.clone(),
                n => format!("{fid}__{n}"),
            };
            let z = if gruppe {
                inst_zustand[n - 1] != c.chance(4)
            } else if im_kegel {
                !wild || c.chance(97)
            } else {
                c.chance(bestaetigt_pct)
            };
            events.push((id, wert(c, b, wild, bestaetigt_pct), z));
        }
    }
    for (fid, w) in fest {
        events.retain(|(f, _, _)| f != fid);
        events.push(((*fid).to_owned(), w.clone(), true));
    }
    // Überschreiben: ein späteres Event auf denselben Schlüssel gewinnt (kein `ersetzt`).
    if !events.is_empty() && c.chance(12) {
        let (fid, _, _) = events[c.range(events.len() as u64) as usize].clone();
        if let Some(b) = bindung_zu(fid.split("__").next().unwrap_or(&fid)) {
            events.push((fid, wert(c, b, false, 50), c.chance(50)));
        }
    }
    events
}

fn store_aus(events: &[(String, Value, bool)], vz: u16) -> Value {
    let evs: Vec<Value> = events
        .iter()
        .enumerate()
        .map(|(i, (f, w, b))| event(i, f, w, *b))
        .collect();
    json!({"version": 1, "veranlagungszeitraum": vz, "events": evs})
}

fn generiere_fall(c: &mut Cursor, q: &'static str) -> Fall {
    // Zusammenveranlagung und Kinder erzwingen wir in der Haelfte der Faelle, damit § 26b, § 31,
    // Partnerfelder, SolZ-Splitting und die Kette regelmaessig laufen.
    let zusammen = c.chance(50);
    let kinder = if c.chance(45) {
        1 + c.range(3) as i64
    } else {
        0
    };
    let veranl = if zusammen { "zusammen" } else { "einzel" };
    let mut fest: Vec<(&str, Value)> = vec![("veranlagung", json!(veranl))];
    if c.chance(90) {
        fest.push(("fam_anzahl_kinder", json!(kinder)));
    }
    if c.chance(60) {
        fest.push((
            "kist_konfession",
            json!(*c.waehle(&["evangelisch", "roemisch-katholisch", "keine"])),
        ));
    }
    // Kapital-Fokus: hohes Einkommen + hohe Kapitalertraege + steuererhebende Konfession, damit die
    // Abgeltung (§ 32d Abs. 1, Abs.-1-KiSt-Ermaessigung, q-Anrechnung) regelmaessig gewinnt.
    if c.chance(40) {
        fest.push((
            "rentner_jahresrente",
            json!(6_000_000 + c.range(12_000_000) as i64),
        ));
        fest.push((
            "kap_kapitalertraege",
            json!(500_000 + c.range(20_000_000) as i64),
        ));
        fest.push((
            "bruttoarbeitslohn",
            json!(8_000_000 + c.range(17_000_000) as i64),
        ));
        fest.push((
            "kist_konfession",
            json!(*c.waehle(&["evangelisch", "roemisch-katholisch"])),
        ));
        fest.push((
            "kist_bundesland",
            json!(*c.waehle(&["bayern", "baden_wuerttemberg", "berlin", "hessen"])),
        ));
        if c.chance(50) {
            fest.push(("kap_q_auslaendische_steuer", json!(c.range(300_000) as i64)));
        }
    }
    // Versorgungs-Fokus (§ 19 Abs. 2): Alters-Gate 63./60. Lj rund um die Grenze.
    if c.chance(25) {
        fest.push(("versorgung_art", json!("altersgrenze_sonstige")));
        fest.push(("versorgung_alter_bei_beginn", json!(58 + c.range(9) as i64)));
        fest.push((
            "versorgung_jahresrente",
            json!(1_000_000 + c.range(3_000_000) as i64),
        ));
        fest.push((
            "versorgung_bemessungsgrundlage",
            json!(1_000_000 + c.range(3_000_000) as i64),
        ));
        fest.push(("versorgung_beginn_jahr", json!(2010 + c.range(16) as i64)));
        fest.push((
            "rentner_grad_der_behinderung",
            json!(*c.waehle(&[0, 30, 50, 80])),
        ));
    }
    // Mobilitaetspraemie (§ 101): niedriges Einkommen, weite Strecke, Einzelveranlagung.
    if c.chance(12) && !zusammen {
        fest.push(("bruttoarbeitslohn", json!(c.range(1_800_000) as i64)));
        fest.push(("ep_entfernung_km", json!(22 + c.range(60) as i64)));
        fest.push(("ep_arbeitstage", json!(100 + c.range(140) as i64)));
        fest.push(("ep_oepnv_kosten", json!(0)));
        fest.push(("ep_eigenes_kfz", json!(true)));
    }
    let events = generiere_events(c, q, &fest);
    let vz = *c.waehle(&[2024_u16, 2025, 2026]);
    Fall {
        q,
        store: Some(store_aus(&events, vz)),
        felder: None,
        vz,
        nur: c.chance(60),
        bindung: if c.chance(50) { "voll" } else { "scheibe" },
        werte: if c.chance(35) { "alle" } else { "kegel" },
        store_uebergeben: c.chance(88),
        solz: c.chance(85),
        extras: c.chance(85),
    }
}

fn runner(cases: u32) -> TestRunner {
    let cfg = Config {
        cases,
        failure_persistence: None,
        max_shrink_iters: 0,
        ..Config::default()
    };
    TestRunner::new_with_rng(cfg, TestRng::deterministic_rng(RngAlgorithm::ChaCha))
}

/// `PARITY_N` überschreibt die Fallzahl je Quantität (Standard 1.200; Abnahme verlangt ≥ 1.000).
fn generierte_je_quantitaet() -> u32 {
    std::env::var("PARITY_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1200)
}

#[test]
fn generierte_faelle() {
    if skip() {
        return;
    }
    let b = RefCell::new(Bilanz::default());
    let n = Cell::new(0usize);
    for q in QUANTITAETEN {
        runner(generierte_je_quantitaet())
            .run(&prop::collection::vec(any::<u8>(), 2048..8192), |bytes| {
                let mut c = Cursor {
                    bytes: &bytes,
                    pos: 0,
                };
                let fall = generiere_fall(&mut c, q);
                vergleiche_fall(&mut b.borrow_mut(), &fall, "gen", true, false);
                n.set(n.get() + 1);
                Ok(())
            })
            .unwrap();
    }
    let b = b.into_inner();
    b.drucke("generierte_faelle", n.get());
    b.wache_rechnet("generierte_faelle");
    assert!(n.get() >= 4 * generierte_je_quantitaet().min(1000) as usize);
    assert_eq!(b.abweichungen(), 0);
}

// ---------------------------------------------------------------- vorläufig bewegt nie

/// Felder, die WEDER im Kegel noch in einer Instanz-Gruppe liegen: ihre Wirkung läuft nur über den
/// gefilterten Snapshot (`_c`/`_b`), nie über `werte` oder die Instanz-Σ.
fn optionale_felder(q: &str) -> Vec<String> {
    let kegel = feld_ids(q, "kegel");
    let mut v: Vec<String> = feld_ids(q, "felder");
    v.extend(EXTRA_FELDER.iter().map(|s| (*s).to_owned()));
    v.sort();
    v.dedup();
    v.into_iter()
        .filter(|f| !kegel.contains(f) && bindung_zu(f).is_some_and(|b| b.instanz_gruppe.is_none()))
        .collect()
}

#[test]
fn vorlaeufiges_optionales_feld_bewegt_nie() {
    if skip() {
        return;
    }
    let (faelle, bewegt_roh, gestoert) = (Cell::new(0usize), Cell::new(0usize), Cell::new(0usize));
    for q in &QUANTITAETEN[1..] {
        let opt = optionale_felder(q);
        runner(400)
            .run(&prop::collection::vec(any::<u8>(), 2048..8192), |bytes| {
                let mut c = Cursor {
                    bytes: &bytes,
                    pos: 0,
                };
                let fall0 = generiere_fall(&mut c, q);
                // Basis: Store ohne die gewaehlten optionalen Felder; Variante: dieselben Felder VORLAEUFIG.
                let waehl: Vec<String> = (0..=c.range(8)).map(|_| c.waehle(&opt).clone()).collect();
                let mut evs: Vec<(String, Value, bool)> = fall0.store.as_ref().unwrap()["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| {
                        (
                            e["feld_id"].as_str().unwrap().to_owned(),
                            e["wert"].clone(),
                            e["zustand"] == "bestaetigt",
                        )
                    })
                    .collect();
                evs.retain(|(f, _, _)| !waehl.contains(f));
                let mut mit = evs.clone();
                for f in &waehl {
                    let b = bindung_zu(f).unwrap();
                    mit.push((f.clone(), wert(&mut c, b, false, 50), false));
                }
                // Roh gerechnet (nur=false) MUSS die Variante bewegen koennen; nur=true NIE.
                let mk = |events: &[(String, Value, bool)], nur: bool| Fall {
                    store: Some(store_aus(events, fall0.vz)),
                    nur,
                    bindung: "voll",
                    werte: "kegel",
                    store_uebergeben: true,
                    solz: true,
                    extras: true,
                    ..fall0.clone()
                };
                let ohne = mk(&evs, true);
                let variante = mk(&mit, true);
                let (py_o, py_v) = (frage(&ohne.request()), frage(&variante.request()));
                let (r_o, r_v) = (rust_lauf(&ohne, &py_o), rust_lauf(&variante, &py_v));
                assert_eq!(
                    format!("{r_o:?}"),
                    format!("{r_v:?}"),
                    "vorlaeufige Felder {waehl:?} bewegen {q} bei nur_bestaetigt=true"
                );
                let roh_o = mk(&evs, false);
                let roh_v = mk(&mit, false);
                let (a, b2) = (frage(&roh_o.request()), frage(&roh_v.request()));
                if a != b2 {
                    bewegt_roh.set(bewegt_roh.get() + 1);
                }
                gestoert.set(gestoert.get() + usize::from(py_o != py_v));
                faelle.set(faelle.get() + 1);
                Ok(())
            })
            .unwrap();
    }
    let (faelle, bewegt_roh, gestoert) = (faelle.get(), bewegt_roh.get(), gestoert.get());
    eprintln!(
        "vorlaeufig_bewegt_nie: {faelle} Fälle × (ohne/mit vorläufigem Feld); nur_bestaetigt=false bewegte in {bewegt_roh} Fällen (Wirksamkeit der Probe); Python-Antwort verschieden bei nur=true: {gestoert}"
    );
    assert!(faelle >= 3 * 400);
    assert!(
        bewegt_roh > 0,
        "die Probe bewegt auch roh nie etwas: wirkungslos"
    );
    assert_eq!(gestoert, 0, "Python bewegt sich bei nur_bestaetigt=true");
}

// ---------------------------------------------------------------- Negativkontrolle

#[test]
fn negativkontrolle_erkennt_genau_eine_abweichung() {
    if skip() {
        return;
    }
    let mut b = Bilanz::default();
    let mut c = Cursor {
        bytes: &[7, 13, 99, 200, 1, 55, 3, 42, 250, 17, 88, 9],
        pos: 0,
    };
    let mut gestoert_faelle = 0;
    for _ in 0..40 {
        let fall = generiere_fall(&mut c, "festzusetzende_est_gesamt");
        let py = frage(&fall.request());
        let r = rust_lauf(&fall, &py);
        // nur Faelle mit Ergebnis koennen gestoert werden
        if matches!(r, Some(Ok(_))) {
            b.vergleiche(&fall, &py, r, "neg", true, true);
            gestoert_faelle += 1;
            break;
        }
    }
    assert_eq!(gestoert_faelle, 1);
    assert_eq!(
        b.abweichungen(),
        1,
        "eine um 1 Cent gestörte Zahl muss genau eine Abweichung geben"
    );
}

// ---------------------------------------------------------------- _abschlusszahlung_cent

/// `_abschlusszahlung_cent` gegen Python: reale Fälle und generierte Snapshots (die `p36_*`-Felder
/// stehen in `SCHEIBEN`), je mit mehreren festgesetzten Beträgen (Erstattung, 0, Nachzahlung).
#[test]
fn abschlusszahlung_paritaet() {
    if skip() {
        return;
    }
    let (n, mit_wert, nur_none, abw) = (Cell::new(0), Cell::new(0), Cell::new(0), Cell::new(0));
    let pruefe = |store: &Value| {
        let datei: StoreDatei = serde_json::from_value(store.clone()).expect("Store");
        let felder = Store::aus_datei(datei).materialisiere(None).unwrap().0;
        for zahl in [-500_000_i64, 0, 123_456, 9_876_543] {
            let py = frage(
                &json!({"fn": "bescheid.abschlusszahlung", "store": store, "zahl_cent": zahl}),
            );
            let r = bescheid::zweige::abschlusszahlung_cent(&felder, Cent::new(zahl));
            n.set(n.get() + 1);
            let gleich = match (py.get("ok"), py.get("err"), &r) {
                (Some(Value::Null), _, Ok(None)) => {
                    nur_none.set(nur_none.get() + 1);
                    true
                }
                (Some(v), _, Ok(Some(c))) => {
                    mit_wert.set(mit_wert.get() + 1);
                    v.as_i64() == Some(c.get())
                }
                (_, Some(k), Err(e)) => e
                    .python_klasse()
                    .is_none_or(|rk| rk == "CatalaError" || rk == k),
                _ => false,
            };
            if !gleich {
                abw.set(abw.get() + 1);
            }
        }
    };
    for pfad in walk_json(&faelle_verzeichnis()) {
        if let Some(roh) = std::fs::read_to_string(&pfad)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        {
            if roh.get("events").is_some() {
                pruefe(&roh);
            }
        }
    }
    runner(300)
        .run(&prop::collection::vec(any::<u8>(), 2048..8192), |bytes| {
            let mut c = Cursor {
                bytes: &bytes,
                pos: 0,
            };
            // Die p36-Felder jeder Konstellation: hoch dosiert, damit der Zweig "mindestens ein Feld" oft greift.
            let mut fest: Vec<(&str, Value)> = Vec::new();
            for f in [
                "p36_lohnsteuer",
                "p36_vorauszahlungen",
                "p36_kapitalertragsteuer",
                "p36_kapitalertragsteuer_solz",
                "p36_kapitalertragsteuer_kist",
            ] {
                if c.chance(45) {
                    fest.push((f, json!(c.range(2_000_000) as i64)));
                }
            }
            let events = generiere_events(&mut c, "festzusetzende_est_gesamt", &fest);
            pruefe(&store_aus(&events, 2025));
            Ok(())
        })
        .unwrap();
    eprintln!(
        "abschlusszahlung: {} Vergleiche; mit Betrag {}, ohne Anrechnungsfeld (None) {}; Abweichungen {}",
        n.get(),
        mit_wert.get(),
        nur_none.get(),
        abw.get()
    );
    assert!(mit_wert.get() > 100 && nur_none.get() > 100);
    assert_eq!(abw.get(), 0);
}
