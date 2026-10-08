//! Parität `rust/bescheid::deklaration` (Schritt 7b) gegen `produkt/bescheid/bescheid_deklaration.py`
//! über `tools/parity/bescheid_oracle.py` (Präfix `bescheid.`). EIN Orakel-Prozess je Binary.
//!
//! Fünf Funktionen je Fall (Zeilen = Funktionsaufrufe): `mit_ring_werten`, `sperrgrund_felder`,
//! `rentenbeginn_offen_stand`, `vorlaeufige_ring_betraege`, `an_gesamt_sperrgrund`;
//! `sperrgrund_klartext` läuft über ALLE Python-Literale (`sperrgrund_klartext_literale`).
//!
//! Vier Eingabequellen:
//! - `reale_faelle`: jede Fall-Datei unter `faelle_verzeichnis()`, je Scheiben-Kontext (die eigene
//!   Scheibe des Falls, die drei Guard-Scheiben, ohne Scheibe) und Aufrufform.
//! - `golden_faelle`: `rust/fixtures/golden_cases.json`, Sachverhalt als Snapshot (roh und ×100).
//! - `generierte_faelle`: ≥ 1.000 proptest-Fälle (Store aus Events, vorläufige/fehlende/falsch
//!   typisierte Werte, negative Beträge, Partner bei Einzelveranlagung), je Fall mehrere Aufrufformen.
//! - `konstanten_gleich`, `sperrgrund_klartext_literale`.
//!
//! Aufrufformen von `an_gesamt_sperrgrund` (alle, die Python-Aufrufer nutzen: `api.py:353/474/583/728`
//! rufen mit `cfg, vz, store, bindung` je vollständig; Alt-Aufrufer und Tests lassen Teile weg):
//! cfg ∈ {keine, 5 Scheiben} × Store ∈ {da, fehlt} × Bindung ∈ {da, fehlt} × VZ ∈ {da, fehlt}.
//!
//! Fehler-Parität: Python-Ausnahme ↔ Rust-`Err` je Aufruf; die Klasse muss gleich heißen.
//! Negativkontrolle: `negativkontrolle_*` stört ein Rust-Ergebnis und verlangt genau eine Abweichung;
//! `PARITY_STOERUNG=1` schaltet dieselbe Störung in `reale_faelle` (Lauf wird rot).
//!
//! SICHERHEIT: reale Fälle sind echte Steuerdaten. Nur lokal lesen, nur Zählwerte ausgeben.
//!
//!   `PARITY`=1 `cargo` test -p parity --test `bescheid_deklaration_paritaet` -- --nocapture
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
    clippy::struct_excessive_bools,
    clippy::type_complexity
)]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};

use bescheid::deklaration::{self as dk, Cfg};
use bescheid::{BescheidFehler, Felder, Instanzquelle};
use bindung::{Bindung, Params};
use domain::{Scheibe, Sperrgrund, Vz};
use parity::Oracle;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use serde_json::{json, Value};
use store::{EventId, Store, StoreDatei};

/// Bekannt leere Zeilen in `gezielte_faelle`. Der Block faehrt handgebaute Faelle, die vor allem
/// `an_gesamt_sperrgrund` pruefen; `mit_ring_werten` und `vorlaeufige_ring_betraege` rechnen nur dank
/// der KAP-Faelle (vorlaeufiger Topf/Aggregat, gemischter Zustand, Partner in `rentner_gesamt`), die
/// anderen beiden Ring-Zeilen sind dort nicht erreichbar.
///
/// Grund je Eintrag: der Fall traegt nur die Felder fuer den Sperrgrund, nicht die Ring-Werte.
const LEER_GEZIELTE: &[(&str, &str)] = &[
    ("rentenbeginn_offen_stand", "gezielter Fall traegt keinen Rentenbeginn"),
    ("sperrgrund_felder", "gezielter Fall traegt keine Sperrgrund-Feldliste"),
];

/// Bekannt leere Zeilen in `golden_faelle`. Grund wie in `bescheid_blatt`: die Vorlage traegt
/// Aggregat-Schluessel, der Ring erwartet Feld-IDs. `an_gesamt_sperrgrund` rechnet dort.
/// Alle vier rechnen im Block `generierte_faelle` (14208/820/2794/5764).
const LEER_GOLDEN: &[(&str, &str)] = &[
    ("mit_ring_werten", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("rentenbeginn_offen_stand", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("sperrgrund_felder", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("vorlaeufige_ring_betraege", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
];

/// Bekannt leere Zeilen in `reale_faelle`: der reale Korpus traegt keinen offenen Rentenbeginn.
/// Keine tote Zeile -- im Block `generierte_faelle` derselben Suite rechnet sie 820-mal.
const LEER_REALE: &[(&str, &str)] = &[
    ("rentenbeginn_offen_stand", "realer Korpus traegt keinen offenen Rentenbeginn; generierte: 820"),
];

const FUNKTIONEN: &[&str] = &[
    "mit_ring_werten",
    "sperrgrund_felder",
    "rentenbeginn_offen_stand",
    "vorlaeufige_ring_betraege",
    "an_gesamt_sperrgrund",
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

fn index() -> &'static HashMap<String, &'static Bindung> {
    static I: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    I.get_or_init(|| store::baue_nachschlag(bindungen()))
}

fn oracle() -> std::sync::MutexGuard<'static, Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn frage_roh(req: &Value) -> Value {
    let antwort = oracle().call_json(req).expect("Orakel antwortet");
    match antwort.get("ok") {
        Some(v) => v.clone(),
        None => panic!("Orakel-Harness-Fehler für {}: {antwort}", req["fn"]),
    }
}

/// Die Scheiben-Namen und ihr `Cfg` (`None` = Aufruf ohne Scheiben-Kontext).
fn scheibe_von(name: Option<&str>) -> Option<Scheibe> {
    name.map(|n| n.parse::<Scheibe>().expect("bekannte Scheibe"))
}

/// Bindung der Scheibe wie `api._scheibe_bindung`: die Feld-Ids kommen vom Orakel.
fn scheiben_bindung(name: &str) -> &'static HashMap<String, &'static Bindung> {
    static C: OnceLock<Mutex<HashMap<String, &'static HashMap<String, &'static Bindung>>>> =
        OnceLock::new();
    let mut m = C
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(b) = m.get(name) {
        return b;
    }
    let felder = frage_roh(&json!({"fn": "bescheid.scheibe_felder", "scheibe": name}));
    let sub: HashMap<String, &'static Bindung> = felder
        .as_array()
        .expect("Feldliste")
        .iter()
        .filter_map(|f| {
            let id = f.as_str()?;
            Some((id.to_owned(), *index().get(id)?))
        })
        .collect();
    let leak: &'static HashMap<String, &'static Bindung> = Box::leak(Box::new(sub));
    m.insert(name.to_owned(), leak);
    leak
}

// ---------------------------------------------------------------- Eingaben

/// Ein Vergleichsfall: derselbe Kontext geht an Python (JSON) und an Rust.
#[derive(Clone)]
struct Fall {
    store: Value,
    vz: u16,
    /// Ohne Scheiben-Kontext: `None`.
    scheibe: Option<&'static str>,
    store_uebergeben: bool,
    bindung_uebergeben: bool,
    /// `vz is None` im Guard bzw. ein Jahr ohne Params in `mit_ring_werten` (Python: VZ 0).
    vz_ohne: bool,
}

/// Golden-Fälle tragen keinen Store, nur Snapshot-Felder.
#[derive(Clone)]
enum Quelle {
    Store(Value),
    Felder(Value),
}

#[derive(Clone)]
struct Kontext {
    quelle: Quelle,
    vz: u16,
    scheibe: Option<&'static str>,
    store_uebergeben: bool,
    bindung_uebergeben: bool,
    vz_ohne: bool,
    /// Der Snapshot enthält Floats in Betragsfeldern, die `konsistenz` liest. PARITÄT: `konsistenz`
    /// zählt einen Float dort nicht als Betrag, Python schon (Crate-Doku `konsistenz`, gemessen: 0
    /// Floats in 192 realen Stores; Auflage T des Stores lässt auf `cent`/`int` nur Ganzzahlen zu).
    /// In diesem Modus entfallen die zwei Funktionen, die durch `konsistenz` laufen.
    float_modus: bool,
}

impl From<Fall> for Kontext {
    fn from(f: Fall) -> Self {
        Self {
            quelle: Quelle::Store(f.store),
            vz: f.vz,
            scheibe: f.scheibe,
            store_uebergeben: f.store_uebergeben,
            bindung_uebergeben: f.bindung_uebergeben,
            vz_ohne: f.vz_ohne,
            float_modus: false,
        }
    }
}

impl Kontext {
    /// `vorlaeufige_ring_betraege` verlangt Scheibe UND Bindung (Python: `cfg.get` auf `None`).
    fn funktionen(&self) -> Vec<&'static str> {
        FUNKTIONEN
            .iter()
            .copied()
            .filter(|n| {
                *n != "vorlaeufige_ring_betraege"
                    || (self.scheibe.is_some() && self.bindung_uebergeben)
            })
            .filter(|n| {
                !self.float_modus || !["an_gesamt_sperrgrund", "sperrgrund_felder"].contains(n)
            })
            .collect()
    }

    fn request(&self, funktionen: &[&str]) -> Value {
        let (store, felder) = match &self.quelle {
            Quelle::Store(s) => (s.clone(), Value::Null),
            Quelle::Felder(f) => (Value::Null, f.clone()),
        };
        json!({"fn": "bescheid.fall",
            "funktionen": funktionen.iter().map(|n| format!("bescheid.{n}")).collect::<Vec<_>>(),
            "store": store, "felder": felder, "vz": if self.vz_ohne { 0 } else { u64::from(self.vz) },
            "nur_bestaetigt": false, "scheibe": self.scheibe,
            "store_uebergeben": self.store_uebergeben, "bindung_uebergeben": self.bindung_uebergeben,
            "args": {"grund": "partner_konsistenz_offen", "vz_none": self.vz_ohne}})
    }
}

/// Rust-Kontext eines Falls (Snapshot roh, wie `api.py`: materialisiert, NICHT auf bestätigt gefiltert).
struct Ctx {
    f: Felder,
    store: Option<Store>,
    bindung: Option<&'static HashMap<String, &'static Bindung>>,
    cfg: Option<Cfg>,
    vz: Option<Vz>,
}

fn baue_ctx(k: &Kontext) -> Ctx {
    let (f, store) = match &k.quelle {
        Quelle::Store(s) => {
            let datei: StoreDatei =
                serde_json::from_value(s.clone()).expect("Store-Datei deserialisiert");
            let st = Store::aus_datei(datei);
            (st.materialisiere(None).expect("materialisiere").0, Some(st))
        }
        Quelle::Felder(fe) => (serde_json::from_value(fe.clone()).expect("felder"), None),
    };
    Ctx {
        f,
        store: if k.store_uebergeben { store } else { None },
        bindung: k
            .scheibe
            .filter(|_| k.bindung_uebergeben)
            .map(scheiben_bindung),
        cfg: scheibe_von(k.scheibe).map(Cfg::fuer),
        vz: if k.vz_ohne {
            None
        } else {
            Some(Vz::try_from(k.vz).expect("vz 2024..2026"))
        },
    }
}

fn grund_json(g: Option<Sperrgrund>) -> Value {
    g.map_or(Value::Null, |g| json!(g.als_str()))
}

/// Ruft die Rust-Funktion `name`; das Ergebnis hat dieselbe JSON-Form wie die Orakel-Antwort.
fn rust_run(c: &Ctx, name: &str) -> Result<Value, BescheidFehler> {
    // Ohne Scheibe, aber mit Bindung: Python reicht die volle Bindung — die Feldmenge der Bindung
    // wird nur im Guard gelesen und dort nur, wenn eine Scheibe da ist; hier bleibt `None`.
    let q = Instanzquelle {
        store: c.store.as_ref(),
        bindung: c.bindung,
        nur_bestaetigt: false,
    };
    Ok(match name {
        "mit_ring_werten" => {
            let mut nachher = c.f.clone();
            dk::mit_ring_werten(&mut nachher, c.vz, params())?;
            let neu: serde_json::Map<String, Value> = nachher
                .iter()
                .filter(|(id, v)| c.f.get(*id) != Some(v))
                .map(|(id, v)| {
                    (
                        id.clone(),
                        // K2/Auflage 2: `zu_json`, nicht `Serialize` fuer `PyWert` — die
                        // Orakel-Form ist `canonical_json` mit der Sortierung von `Value`.
                        json!({"wert": v.wert.zu_json().expect("Ring-Werte sind endlich"),
                            "zustand": serde_json::to_value(v.zustand).unwrap(),
                            "herkunft": serde_json::to_value(&v.herkunft).unwrap()}),
                    )
                })
                .collect();
            Value::Object(neu)
        }
        "sperrgrund_felder" => Value::Array(
            dk::sperrgrund_felder(Some(Sperrgrund::PartnerKonsistenzOffen), &c.f)
                .into_iter()
                .map(|w| {
                    json!({"feld_id": w.feld_id,
                        "wert": w.wert.zu_json().expect("Store-Wert ist darstellbar (Auflage 3)"),
                        "veranlagung": w.veranlagung.zu_json().expect("Store-Wert ist darstellbar (Auflage 3)"),
                        "grund": w.grund})
                })
                .collect(),
        ),
        "rentenbeginn_offen_stand" => {
            grund_json(dk::rentenbeginn_offen_stand(&c.f, c.cfg.as_ref()))
        }
        "vorlaeufige_ring_betraege" => {
            let cfg = c.cfg.as_ref().expect("Scheibe");
            let bindung = c.bindung.expect("Bindung");
            json!(dk::vorlaeufige_ring_betraege(&c.f, cfg, bindung))
        }
        "an_gesamt_sperrgrund" => {
            grund_json(dk::an_gesamt_sperrgrund(&c.f, c.cfg.as_ref(), c.vz, &q)?)
        }
        other => panic!("unbekannte Funktion {other}"),
    })
}

// ---------------------------------------------------------------- Vergleich und Zählung

#[derive(Default, Clone)]
struct Zeile {
    python: u64,
    rust: u64,
    ok_gleich: u64,
    err_gleich: u64,
    nicht_leer: u64,
    abw: u64,
}

#[derive(Default)]
struct Bilanz {
    zeilen: BTreeMap<&'static str, Zeile>,
    abweichungen: Vec<String>,
    /// Verteilung der Python-Sperrgründe (`an_gesamt_sperrgrund`, `null` = "keine Sperre").
    gruende: BTreeMap<String, u64>,
    /// Python-Fehlerklassen je Funktion.
    fehler: BTreeMap<String, u64>,
}

fn nicht_leer(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Stört das erste Blatt im Ergebnis (Negativkontrolle): Ganzzahl +1, Text + "!".
fn stoere(v: &mut Value) -> bool {
    match v {
        Value::Number(n) if n.is_i64() => {
            *v = json!(n.as_i64().unwrap() + 1);
            true
        }
        Value::String(s) => {
            s.push('!');
            true
        }
        Value::Array(a) => a.iter_mut().any(stoere),
        Value::Object(o) => o.values_mut().any(stoere),
        _ => false,
    }
}

impl Bilanz {
    fn vergleiche(
        &mut self,
        name: &'static str,
        py: &Value,
        rust: Result<Value, BescheidFehler>,
        ort: &str,
        werte: bool,
    ) {
        let z = self.zeilen.entry(name).or_default();
        z.python += 1;
        z.rust += 1;
        let melde = |abw: &mut Vec<String>, was: String| abw.push(format!("{ort} {name}: {was}"));
        if name == "an_gesamt_sperrgrund" {
            let schluessel = py.get("ok").map_or_else(
                || format!("Fehler {}", py["err"].as_str().unwrap_or("?")),
                |g| g.as_str().unwrap_or("(keine Sperre)").to_owned(),
            );
            *self.gruende.entry(schluessel).or_default() += 1;
        }
        if let Some(e) = py.get("err") {
            *self
                .fehler
                .entry(format!("{name}: {}", e.as_str().unwrap_or("?")))
                .or_default() += 1;
        }
        match (py.get("ok"), rust) {
            (Some(p), Ok(r)) => {
                if nicht_leer(p) {
                    z.nicht_leer += 1;
                }
                if *p == r {
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
            (None, Err(e)) => {
                let py_klasse = py["err"].as_str().unwrap_or("?");
                match e.python_klasse() {
                    Some(k) if k != "CatalaError" && k != py_klasse => {
                        z.abw += 1;
                        melde(
                            &mut self.abweichungen,
                            format!("Fehlerklasse rust={k} py={py_klasse}"),
                        );
                    }
                    _ => z.err_gleich += 1,
                }
            }
            (Some(p), Err(e)) => {
                z.abw += 1;
                melde(
                    &mut self.abweichungen,
                    if werte {
                        format!("py ok={p}, rust Err={e}")
                    } else {
                        "py ok, rust Err".into()
                    },
                );
            }
            (None, Ok(r)) => {
                z.abw += 1;
                melde(
                    &mut self.abweichungen,
                    if werte {
                        format!("py Err={}, rust ok={r}", py["err"])
                    } else {
                        "py Err, rust ok".into()
                    },
                );
            }
        }
    }

    fn drucke(&self, titel: &str, faelle: usize) {
        eprintln!("== {titel}: {faelle} Kontexte ==");
        eprintln!(
            "{:<28} {:>7} {:>7} {:>8} {:>8} {:>10} {:>5}",
            "funktion", "python", "rust", "ok=", "err=", "nicht-leer", "abw"
        );
        for (n, z) in &self.zeilen {
            eprintln!(
                "{n:<28} {:>7} {:>7} {:>8} {:>8} {:>10} {:>5}",
                z.python, z.rust, z.ok_gleich, z.err_gleich, z.nicht_leer, z.abw
            );
        }
        eprintln!("Abweichungen gesamt: {}", self.abweichungen());
        for a in self.abweichungen.iter().take(8) {
            eprintln!("  ! {a}");
        }
    }

    fn drucke_gruende(&self) {
        let sperren: u64 = self
            .gruende
            .iter()
            .filter(|(k, _)| !k.starts_with('(') && !k.starts_with("Fehler"))
            .map(|(_, n)| n)
            .sum();
        eprintln!(
            "Sperrgrund-Verteilung (Python, an_gesamt_sperrgrund): {} verschiedene, {sperren} Sperren, {} Aufrufe",
            self.gruende
                .keys()
                .filter(|k| !k.starts_with('(') && !k.starts_with("Fehler"))
                .count(),
            self.gruende.values().sum::<u64>()
        );
        for (g, n) in &self.gruende {
            eprintln!("  {n:>6}  {g}");
        }
        for (f, n) in &self.fehler {
            eprintln!("  Python-Ausnahme {n:>5}× {f}");
        }
    }

    fn abweichungen(&self) -> u64 {
        self.zeilen.values().map(|z| z.abw).sum()
    }

    /// Die `nicht_leer`-Zaehler aller Zeilen als [`parity::pin::Gesehen`].
    fn gesehen(&self) -> parity::pin::Gesehen {
        self.zeilen
            .iter()
            .map(|(n, z)| (*n, z.nicht_leer))
            .collect()
    }

    /// Waechter gegen einen gruenen Lauf, der nichts belegt: eine Vergleichszeile, die nie
    /// einen Wert GESEHEN hat (`nicht-leer == 0`), kann nicht "0 Abweichungen" beweisen --
    /// gruen und leer sehen identisch aus.
    ///
    /// Ticket `parity-lauf-gruen-ohne-dass-die-zeile-rechnet`. Der Waechter greift JE BLOCK:
    /// eine Zeile, die nur in `golden_faelle` rechnet, deckt die Luecke in `reale_faelle` nicht.
    /// Gepinnte, heute nachweislich leere Zeilen stehen in der `LEER`-Liste des Blocks -- mit
    /// Grund und Korpus. Beide Richtungen sind streng (s. [`parity::pin::pruefe`]).
    fn wache_rechnet(&self, block: &str, liste: &[(&str, &str)]) {
        parity::pin::pruefe(block, parity::pin::KORPUS, liste, &self.gesehen());
    }
}

/// Ein Kontext gegen Python und Rust; `stoere_erste` verfälscht das erste Rust-Ergebnis.
fn vergleiche(b: &mut Bilanz, k: &Kontext, ort: &str, werte: bool, stoere_erste: bool) {
    let namen = k.funktionen();
    let py = frage_roh(&k.request(&namen));
    let c = baue_ctx(k);
    let mut gestoert = !stoere_erste;
    let vorher = b.abweichungen();
    for n in FUNKTIONEN.iter().filter(|n| namen.contains(n)) {
        let mut r = rust_run(&c, n);
        if !gestoert {
            if let Ok(v) = r.as_mut() {
                gestoert = stoere(v);
            }
        }
        b.vergleiche(n, &py[format!("bescheid.{n}")], r, ort, werte);
    }
    // Nur erzeugte Daten (`werte`): die Anfrage der ersten Abweichung zur Nachstellung ablegen.
    if werte && b.abweichungen() > vorher {
        let pfad = std::env::temp_dir().join("parity-deklaration-abweichung.json");
        let _ = std::fs::write(pfad, k.request(&namen).to_string());
    }
}

/// Alle Aufrufformen eines Snapshots: Scheiben-Kontext × (Store, Bindung, VZ) da/fehlt.
fn formen(
    quelle: &Quelle,
    vz: u16,
    eigene: Option<&'static str>,
    dicht: bool,
    float_modus: bool,
) -> Vec<Kontext> {
    let mk = |scheibe, store_uebergeben, bindung_uebergeben, vz_ohne| Kontext {
        quelle: quelle.clone(),
        vz,
        scheibe,
        store_uebergeben,
        bindung_uebergeben,
        vz_ohne,
        float_modus,
    };
    let mut out = Vec::new();
    // Die Form der Python-Aufrufer: cfg, vz, store, bindung — je vollständig.
    for s in [
        None,
        Some("an_gesamt"),
        Some("gesamt"),
        Some("rentner_gesamt"),
    ] {
        out.push(mk(s, true, true, false));
    }
    if let Some(e) = eigene.filter(|e| !["an_gesamt", "gesamt", "rentner_gesamt"].contains(e)) {
        out.push(mk(Some(e), true, true, false));
    }
    if dicht {
        // Alt-Aufrufer und Tests: Teile weglassen.
        for s in [
            None,
            Some("an_gesamt"),
            Some("gesamt"),
            Some("rentner_gesamt"),
            Some("ep"),
            Some("n_vor_gwg"),
        ] {
            out.push(mk(s, false, true, false));
            out.push(mk(s, true, false, false));
            out.push(mk(s, true, true, true));
        }
    } else {
        for s in [Some("gesamt"), Some("rentner_gesamt")] {
            out.push(mk(s, false, true, false));
            out.push(mk(s, true, false, false));
            out.push(mk(s, true, true, true));
        }
    }
    out
}

// ---------------------------------------------------------------- Konstanten und Literale

/// Ohne die Scheibenlisten: `felder`, `kegel`, `teil_ringe` je Scheibe und die daraus geschnittenen
/// `ring_kandidaten`. Seit Weg B leicht (2026-10-05) sind die Listen in `scheiben_tabellen.rs` von
/// Hand gepflegte Rust-Quelle; ein Feld nur fuer Rust darf dort stehen, ohne dass dieser Vergleich
/// rot wird. Ihre Eigenschaften haelt `rust/bescheid/tests/scheiben_tabellen_konsistenz.rs`.
fn ohne_scheibenlisten(mut v: Value) -> Value {
    if let Some(o) = v.as_object_mut() {
        o.remove("ring_kandidaten");
        for cfg in o
            .get_mut("cfg")
            .and_then(Value::as_object_mut)
            .into_iter()
            .flat_map(|c| c.values_mut())
        {
            if let Some(c) = cfg.as_object_mut() {
                for schluessel in ["felder", "kegel", "teil_ringe"] {
                    assert!(c.remove(schluessel).is_some(), "Cfg ohne {schluessel}");
                }
            }
        }
    }
    v
}

#[test]
fn konstanten_gleich() {
    if skip() {
        return;
    }
    let py = frage_roh(&json!({"fn": "bescheid.konstanten"}));
    let rust = dk::testhilfe::konstanten_json();
    assert_eq!(
        ohne_scheibenlisten(py["deklaration"].clone()),
        ohne_scheibenlisten(rust.clone()),
        "Tabellen weichen ab"
    );
    let n = rust["tabellen"].as_object().unwrap().len();
    let d = frage_roh(&json!({"fn": "bescheid.dateien"}));
    eprintln!("konstanten_gleich: {n} Tabellen, 5 Cfg ohne Scheibenlisten, 0 Abweichungen");
    eprintln!("Python-Dateien (Orakel): {d}");
}

/// Gruende, die Python kennt und deren Klartext Rust absichtlich anders sagt (Abweichung Nr. 27 in
/// `rust/fixtures/README.md`: `gwg_mehrwertsteuer_offen` nennt seit der Folgefrage `gwg_ohne_vorsteuerabzug` den Weg des
/// Kleinunternehmers; Abweichung Nr. 37: `versorgungsfreibetrag_offen` nennt den Ehegatten; Abweichung Nr. 45:
/// `gwg_abschreibung_offen` nennt Nutzungsdauer und Kaufmonat der Einzel-AfA). Dieselbe Liste und dieselbe
/// Strenge wie `domain::sperrgrund::tests::ABWEICHENDER_KLARTEXT`.
const ABWEICHENDER_KLARTEXT: [&str; 3] = [
    "gwg_mehrwertsteuer_offen",
    "versorgungsfreibetrag_offen",
    "gwg_abschreibung_offen",
];

#[test]
fn sperrgrund_klartext_literale() {
    if skip() {
        return;
    }
    let lit = frage_roh(&json!({"fn": "bescheid.sperrgrund_literale"}));
    let strs = |k: &str| -> Vec<String> {
        lit[k]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect()
    };
    let (schluessel, rueck) = (strs("klartext_schluessel"), strs("rueckgaben"));
    let mut alle: Vec<String> = schluessel.iter().chain(&rueck).cloned().collect();
    alle.extend(["bestaetigt", "unbekannt_xyz", "", "Abs3_Ueber_5mio_offen"].map(String::from));
    alle.sort();
    alle.dedup();
    let (mut gleich, mut typisiert, mut abweichend) = (0_usize, 0_usize, 0_usize);
    for l in &alle {
        let py = frage_roh(
            &json!({"fn": "bescheid.fall", "funktionen": ["bescheid.sperrgrund_klartext"],
            "felder": {}, "vz": 2025, "nur_bestaetigt": false, "args": {"grund": l}}),
        );
        let python = py["bescheid.sperrgrund_klartext"]["ok"].as_str().unwrap();
        // Streng: ein Grund, dessen Klartext Rust absichtlich anders sagt (Abweichung Nr. 27, Nr. 37, Nr. 45), muss ABWEICHEN. Wird er
        // gleich, ist die Liste falsch; eine Zeile, die nie vergleicht, waere sonst gruen und leer.
        let erwartet = if ABWEICHENDER_KLARTEXT.contains(&l.as_str()) {
            let rust = dk::sperrgrund_klartext_text(Some(l));
            assert!(rust.is_some_and(|t| t != python), "Literal {l:?}: gleich dem Python-Text, dann raus aus der Liste");
            abweichend += 1;
            rust.unwrap()
        } else {
            python
        };
        assert_eq!(
            dk::sperrgrund_klartext_text(Some(l)),
            Some(erwartet),
            "Literal {l:?}"
        );
        gleich += 1;
        if let Ok(g) = l.parse::<Sperrgrund>() {
            assert_eq!(
                dk::sperrgrund_klartext(Some(g)),
                Some(erwartet),
                "typisiert {l:?}"
            );
            typisiert += 1;
        }
    }
    assert_eq!(
        abweichend,
        ABWEICHENDER_KLARTEXT.len(),
        "jeder abweichende Grund steht unter den Python-Literalen"
    );
    // None bleibt None (Python `if grund is None: return None`).
    let py_none = frage_roh(
        &json!({"fn": "bescheid.fall", "funktionen": ["bescheid.sperrgrund_klartext"],
        "felder": {}, "vz": 2025, "nur_bestaetigt": false, "args": {"grund": null}}),
    );
    assert!(py_none["bescheid.sperrgrund_klartext"]["ok"].is_null());
    assert_eq!(dk::sperrgrund_klartext(None), None);
    assert_eq!(dk::sperrgrund_klartext_text(None), None);
    // Jede Rückgabe der Guards ist ein Sperrgrund der Enum UND hat einen Klartext-Eintrag.
    for r in &rueck {
        let g: Sperrgrund = r
            .parse()
            .unwrap_or_else(|_| panic!("Rückgabe {r:?} fehlt in domain::Sperrgrund"));
        assert!(g.klartext().is_some(), "{r:?} ohne Klartext");
        assert!(schluessel.contains(r), "{r:?} fehlt in SPERRGRUND_KLARTEXT");
    }
    eprintln!(
        "sperrgrund_klartext_literale: {} Literale (Python: {} Klartext-Schlüssel, {} Rückgaben der Guards), {gleich} geprüft, davon {abweichend} mit absichtlich anderem Text (Abweichung Nr. 27), {typisiert} typisiert, 0 ungewollte Abweichungen",
        alle.len(),
        schluessel.len(),
        rueck.len()
    );
}

// ---------------------------------------------------------------- reale Fälle

fn eigene_scheibe(roh: &Value) -> Option<&'static str> {
    ["ep", "n_vor_gwg", "an_gesamt", "gesamt", "rentner_gesamt"]
        .into_iter()
        .find(|s| roh["scheibe"] == *s)
}

/// Ein Parity-Lauf ohne Korpus ist kein gruener Lauf: er vergleicht nichts und meldet
#[test]
fn reale_faelle() {
    if skip() {
        return;
    }
    // EIN gemeinsamer Leser, der ZAEHLT (s. parity::korpus). Die frueher hier gestandene
    // stille `continue` machte einen Lauf auf einem Bruchteil des Korpus gruen.
    let korpus = parity::korpus::Korpus::lies(&faelle_verzeichnis());
    korpus.pflicht(&faelle_verzeichnis(), "reale_faelle");
    let mut b = Bilanz::default();
    let (mut kontexte, mut kein_store, mut vz_ersatz, mut stores) =
        (0_usize, 0_usize, 0_usize, 0_usize);
    let mut stoerung_offen = stoerung_an();
    for (_pfad, roh) in &korpus.gelesen {
        let roh = roh.clone();
        if roh.get("events").is_none() {
            kein_store += 1;
            continue;
        }
        let vz = roh["veranlagungszeitraum"]
            .as_u64()
            .filter(|v| (2024..=2026).contains(v))
            .unwrap_or_else(|| {
                vz_ersatz += 1;
                2025
            }) as u16;
        // PARITÄT: VZ außerhalb 2024–2026 ist in Rust nicht darstellbar; beide Seiten bekommen 2025.
        let mut roh = roh;
        roh["veranlagungszeitraum"] = json!(vz);
        stores += 1;
        for k in formen(
            &Quelle::Store(roh.clone()),
            vz,
            eigene_scheibe(&roh),
            true,
            false,
        ) {
            vergleiche(
                &mut b,
                &k,
                "real",
                false,
                std::mem::take(&mut stoerung_offen),
            );
            kontexte += 1;
        }
    }
    b.drucke("reale_faelle", kontexte);
    b.wache_rechnet("reale_faelle", LEER_REALE);
    b.drucke_gruende();
    eprintln!(
        "reale_faelle: {} von {} Dateien GELESEN, {stores} Stores, {kein_store} ohne Store \
         übersprungen, {vz_ersatz} mit VZ außerhalb 2024–2026 (Ersatz 2025), {} unlesbar",
        korpus.gelesen_zahl(),
        korpus.gefunden.len(),
        korpus.uebersprungen.len()
    );
    assert!(kontexte > 0);
    assert_eq!(
        b.abweichungen(),
        0,
        "Abweichungen (Anzahl s. o., keine Werte ausgegeben)"
    );
}

// ---------------------------------------------------------------- Golden

#[test]
fn golden_faelle() {
    if skip() {
        return;
    }
    let text = std::fs::read_to_string(repo_root().join("rust/fixtures/golden_cases.json"))
        .expect("golden_cases.json");
    let faelle: Vec<Value> = serde_json::from_str(&text).unwrap();
    let herkunft = json!({"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"});
    let mut b = Bilanz::default();
    let mut n = 0_usize;
    for fall in &faelle {
        let sv = fall["sachverhalt"].as_object().unwrap();
        let vz = sv
            .get("veranlagungszeitraum")
            .and_then(Value::as_u64)
            .filter(|v| (2024..=2026).contains(v))
            .unwrap_or(2025) as u16;
        for faktor in [1_i64, 100] {
            let felder: serde_json::Map<String, Value> = sv
                .iter()
                .filter(|(_, v)| v.is_number() || v.is_boolean() || v.is_string())
                .map(|(k, v)| {
                    let wert = match (v.as_i64(), k.as_str()) {
                        (Some(i), k) if k != "veranlagungszeitraum" && k != "anzahl_kinder" => {
                            json!(i * faktor)
                        }
                        _ => v.clone(),
                    };
                    (
                        k.clone(),
                        json!({"wert": wert, "zustand": "bestaetigt", "herkunft": herkunft}),
                    )
                })
                .collect();
            for k in formen(
                &Quelle::Felder(Value::Object(felder)),
                vz,
                None,
                false,
                false,
            ) {
                vergleiche(&mut b, &k, "golden", true, false);
                n += 1;
            }
        }
    }
    b.drucke("golden_faelle", n);
    b.wache_rechnet("golden_faelle", LEER_GOLDEN);
    b.drucke_gruende();
    assert!(n > 0);
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

#[derive(Clone, Copy)]
enum Kind {
    Cent,
    Bool,
    Gdb,
    Jahr,
    Anz,
    Monate,
    Hebesatz,
    Idnr,
    Veranl,
    Betriebsart,
    Rentenart,
    Konfession,
    Text,
    Prozent,
    Selten,
}
use Kind::{Anz, Bool, Cent, Gdb, Hebesatz, Idnr, Jahr, Monate, Prozent, Rentenart, Text, Veranl};

/// `(feld_id, Art)`. Ob ein Feld Instanzen trägt, sagt die Bindung (`instanz_gruppe`).
const FELDER: &[(&str, Kind)] = &[
    ("veranlagung", Veranl),
    ("bruttoarbeitslohn_partner", Cent),
    ("vor_an_anteil_rv", Cent),
    ("vor_ag_anteil_rv", Cent),
    ("vor_rv_ausserhalb_lstb", Cent),
    ("vor_an_anteil_rv_partner", Cent),
    ("vor_ag_anteil_rv_partner", Cent),
    ("vor_rv_ausserhalb_lstb_partner", Cent),
    // Screening-Flags
    ("kein_gewinn", Bool),
    ("kein_kap", Bool),
    ("kein_vuv", Bool),
    ("kein_sonstige", Bool),
    ("kein_kap_partner", Bool),
    ("kein_sonstige_partner", Bool),
    ("kein_p23_verkauf", Bool),
    ("keine_lohnersatzleistungen", Bool),
    ("kein_verlustvortrag", Bool),
    ("kein_unterhalt", Bool),
    ("kein_realsplitting", Bool),
    ("keine_behinderung_pflege", Bool),
    // Familie, § 34, § 32b
    ("fam_alleinstehend", Kind::Selten),
    ("fam_anzahl_kinder", Anz),
    ("verlustvortrag_bestand", Cent),
    ("p32b_progressionseinkuenfte", Cent),
    ("antrag_ermaessigter_satz", Kind::Selten),
    ("geburtsjahr", Jahr),
    ("dauernd_berufsunfaehig", Bool),
    ("ermaessigung_einmal_genutzt", Bool),
    ("rentner_veraeusserungsgewinn", Cent),
    ("rentner_veraeusserungsgewinn_partner", Cent),
    ("rentner_alter_55_oder_berufsunfaehig", Bool),
    ("rentner_freibetrag_erstmalig", Bool),
    ("rentner_alter_55_oder_berufsunfaehig_partner", Bool),
    ("rentner_freibetrag_erstmalig_partner", Bool),
    // DBA, GewSt
    ("dba_mehrere_staaten", Kind::Selten),
    ("dba_auslaendische_einkuenfte", Cent),
    ("dba_gezahlte_auslaendische_steuer", Cent),
    ("gewst_messbetrag", Cent),
    ("gewst_hebesatz", Hebesatz),
    ("gewst_messbetrag_partner", Cent),
    ("gewst_hebesatz_partner", Hebesatz),
    // Kapital
    ("kap_kapitalertraege", Cent),
    ("kap_gewinn_aktien", Cent),
    ("kap_verlust_aktien", Cent),
    ("kap_gewinn_sonstige", Cent),
    ("kap_verlust_sonstige", Cent),
    ("kap_kapitalertraege_partner", Cent),
    ("kap_gewinn_aktien_partner", Cent),
    ("kap_verlust_aktien_partner", Cent),
    ("kap_gewinn_sonstige_partner", Cent),
    ("kap_verlust_sonstige_partner", Cent),
    // Gewinn
    ("einkuenfte_gewinn", Cent),
    ("betriebseinnahmen", Cent),
    ("sonstige_betriebsausgaben", Cent),
    ("afa_jahresbetrag", Cent),
    ("gwg_anschaffungskosten_netto", Cent),
    ("gwg_bewegliches_selbstaendig_nutzbar", Bool),
    ("gwg_netto_ohne_vorsteuer", Bool),
    ("gwg_verzeichnis_ab_250", Bool),
    ("gewinn_betriebsart", Kind::Betriebsart),
    ("gewinnanteil", Cent),
    ("verguetung_taetigkeit", Cent),
    // Betrag-offen
    ("p33a_unterhalt_aufwendungen", Cent),
    ("kist_konfession", Kind::Konfession),
    ("kist_gezahlt", Cent),
    ("kist_erstattet", Cent),
    ("realsplitting_zustimmung", Bool),
    ("realsplitting_unterhaltsleistungen", Cent),
    ("fahrtkosten_pausch_ag_bl_tbl_h", Bool),
    ("fahrtkosten_pausch_gdb80_oder_70g", Bool),
    // Vermietung
    ("vv_einnahmen", Cent),
    ("vv_gebaeude_afa", Cent),
    ("vv_schuldzinsen", Cent),
    ("vv_erhaltungsaufwand", Cent),
    ("vv_sonstige_wk", Cent),
    ("vv_entgelt_quote_prozent", Prozent),
    ("vv_nebenkosten_umgelegt", Cent),
    // Rente
    ("rentner_renten_art", Rentenart),
    ("rentner_jahresrente", Cent),
    ("rentner_renten_beginn_jahr", Jahr),
    ("rentner_alter_bei_rentenbeginn", Anz),
    ("rentner_rentenfreibetrag", Cent),
    ("rentner_renten_art_partner", Rentenart),
    ("rentner_jahresrente_partner", Cent),
    ("rentner_renten_beginn_jahr_partner", Jahr),
    ("rentner_alter_bei_rentenbeginn_partner", Anz),
    ("rentner_rentenfreibetrag_partner", Cent),
    ("basis_kv_partner", Cent),
    ("basis_pv_partner", Cent),
    ("versicherungsart_partner", Text),
    // Versorgung, § 33b
    ("versorgung_jahresrente", Cent),
    ("versorgung_beginn_jahr", Jahr),
    ("versorgung_bemessungsgrundlage", Cent),
    ("rentner_grad_der_behinderung", Gdb),
    ("rentner_grad_der_behinderung_partner", Gdb),
    ("rentner_hilflos_blind_taubblind", Bool),
    ("rentner_hilflos_blind_taubblind_partner", Bool),
    ("behinderungsbedingte_aufwendungen", Cent),
    ("behinderungsbedingte_aufwendungen_partner", Cent),
    ("behinderungsbedingte_aufwendungen_wahlrecht_pb", Bool),
    (
        "behinderungsbedingte_aufwendungen_wahlrecht_pb_partner",
        Bool,
    ),
    ("kind_idnr", Idnr),
    ("kind_behinderten_pb_antrag", Bool),
    ("kind_pb_nicht_selbst_genutzt", Bool),
    ("kinderbetreuungskosten", Cent),
    ("kind_unter_14_haushaltszugehoerig", Bool),
    ("kind_betreuung_reine_betreuung", Bool),
    ("kind_betreuung_rechnung_ueberweisung", Bool),
    // § 35a, § 35c
    ("hh_minijob_betrag", Cent),
    ("hh_dienstleistung_betrag", Cent),
    ("hh_handwerker_betrag", Cent),
    ("hh_minijob_aufwendungen", Cent),
    ("hh_dienstleistungen", Cent),
    ("hh_handwerker_arbeitskosten", Cent),
    ("hh_rechnung_unbar", Bool),
    ("hh_handwerker_keine_foerderung", Bool),
    ("hh_in_eu_ewr", Bool),
    ("p35c_sanierungsaufwendungen", Cent),
    ("p35c_energieberater_aufwendungen", Cent),
    ("p35c_keine_doppelfoerderung", Bool),
    // § 22 Nr. 3, Ausbildung
    ("p22_nr3_einnahmen", Cent),
    ("p22_nr3_einkuenfte", Cent),
    ("berufsausbildung_aufwendungen", Cent),
    // dHf, Verpflegung, Übernachtung, Arbeitsmittel
    ("dhf_unterkunftskosten_monat", Cent),
    ("dhf_im_inland", Bool),
    ("dhf_beruflich_veranlasst", Bool),
    ("dhf_eigener_hausstand", Bool),
    ("dhf_finanzielle_beteiligung", Bool),
    ("tage_24h", Anz),
    ("tage_an_abreise", Anz),
    ("tage_ueber_8h_eintaegig", Anz),
    ("vpf_monate_am_ort", Monate),
    ("vpf_tage_24h_nach_drei_monaten", Anz),
    ("vpf_tage_an_abreise_nach_drei_monaten", Anz),
    ("vpf_tage_ueber_8h_nach_drei_monaten", Anz),
    ("vpf_fruehstuecke_gestellt_anzahl", Anz),
    ("vpf_mittagessen_gestellt_anzahl", Anz),
    ("vpf_abendessen_gestellt_anzahl", Anz),
    ("vpf_keine_mahlzeitengestellung", Bool),
    ("vpf_frist_nicht_unterbrochen", Bool),
    ("vpf_mahlzeiten_gezahltes_entgelt", Cent),
    ("vpf_steuerfreie_erstattung_betrag", Cent),
    ("uebernachtung_kosten_monat", Cent),
    ("uebernachtung_im_inland", Bool),
    ("uebernachtung_auswaerts", Bool),
    ("uebernachtung_alleinnutzung", Bool),
    ("uebernachtung_keine_lange_unterbrechung", Bool),
    ("uebernachtung_monate_bisher", Monate),
    ("uebernachtung_monate", Monate),
    ("am_anschaffungskosten", Cent),
    ("am_gwg_sofortabzug_gewaehlt", Bool),
    ("arbeitsmittel_nutzungsdauer", Anz),
    ("am_anschaffung_monat", Monate),
    ("am_afa_ist_anschaffungsjahr", Bool),
];

const GRENZEN: &[i64] = &[
    0,
    1,
    99,
    100,
    101,
    25_000,
    25_001,
    80_000,
    80_001,
    99_900,
    100_000,
    250_000,
    1_000_000,
    5_000_000,
    20_000_000,
    500_000_100,
];

/// Felder, die `konsistenz` liest (`FLAG_NEGIERT`-Basen und `PARTNER_FELDER`).
fn konsistenz_feld(fid: &str) -> bool {
    konsistenz::FLAG_NEGIERT
        .iter()
        .any(|(_, basen)| basen.contains(&fid))
        || konsistenz::PARTNER_FELDER.contains(&fid)
}

fn wert(c: &mut Cursor, k: Kind, wahr_pct: u64, kein_float: bool) -> Value {
    match k {
        Cent => match c.range(24) {
            0 => Value::Null,
            1 => json!(true),
            2 => json!("12"),
            3 if kein_float => json!(c.range(2_000_000) as i64),
            3 => json!(c.range(2_000_000) as i64 as f64 + 0.5),
            4 => json!(-(c.range(600_000) as i64)),
            5 | 6 => json!(*c.waehle(GRENZEN)),
            7 => json!(-150),
            8 => json!(-*c.waehle(GRENZEN)),
            9 => json!(c.range(30_000_000) as i64),
            _ => json!(c.range(3_000_000) as i64),
        },
        Bool => match c.range(100) {
            0..=3 => Value::Null,
            4 => json!("ja"),
            5 => json!(1),
            _ => json!(c.chance(wahr_pct)),
        },
        // Sperr-Schalter: selten wahr, sonst verdecken sie alle Pruefungen dahinter.
        Kind::Selten => match c.range(100) {
            0..=3 => Value::Null,
            4 => json!("ja"),
            _ => json!(c.chance(8)),
        },
        Gdb => match c.range(8) {
            0 => json!(0),
            1 => Value::Null,
            2 if kein_float => json!(19),
            2 => json!(19.5),
            _ => json!(20 + c.range(90) as i64),
        },
        Jahr => match c.range(10) {
            0 => Value::Null,
            1 => json!(0),
            2 => json!(true),
            3 => json!(-5),
            4 => json!("1990"),
            _ => json!(1930 + c.range(100) as i64),
        },
        Anz => match c.range(14) {
            0 => Value::Null,
            1 => json!("2"),
            2 => json!(true),
            3 => json!(2.5),
            4 => json!(-1),
            5 => json!("x"),
            6 => json!([]),
            _ => json!(c.range(25) as i64),
        },
        Monate => match c.range(12) {
            0 => Value::Null,
            1 => json!("4"),
            2 => json!(true),
            3 => json!(13),
            _ => json!(c.range(15) as i64),
        },
        Prozent => match c.range(6) {
            0 => Value::Null,
            _ => json!(c.range(101) as i64),
        },
        Hebesatz => json!(*c.waehle(&[0, 200, 380, 400, 490, 900, -100])),
        Idnr => match c.range(6) {
            0 => json!("kurz"),
            1 => Value::Null,
            2 => json!(12_345_678_901_i64),
            3 => json!(""),
            _ => json!(format!("{:011}", c.range(99_999_999_999))),
        },
        Veranl => json!(*c.waehle(&["einzel", "zusammen", "zusammen", "getrennt", "", "Zusammen"])),
        Kind::Betriebsart => {
            json!(*c.waehle(&["gewerbe", "gewerbe", "selbstaendig", "land_forst", ""]))
        }
        Rentenart => match c.range(10) {
            0 => Value::Null,
            1 => json!(5),
            2 => json!(["gesetzliche_rente"]),
            _ => json!(*c.waehle(&[
                "gesetzliche_rente",
                "gesetzliche_rente",
                "berufsstaendische_versorgung",
                "private_basisrente",
                "leibrente",
                ""
            ])),
        },
        Kind::Konfession => match c.range(8) {
            0 => Value::Null,
            1 => json!(3),
            _ => json!(*c.waehle(&["keine", "keine", "ev", "rk", ""])),
        },
        Text => json!(*c.waehle(&["gesetzlich_an", "privat", "", "x"])),
    }
}

fn event(i: usize, fid: &str, wert: &Value, bestaetigt: bool) -> Value {
    let mut e = json!({
        "ts": format!("2026-01-01T{:02}:{:02}:{:02}+00:00", i / 3600, (i / 60) % 60, i % 60), "feld_id": fid, "wert": wert,
        "zustand": if bestaetigt { "bestaetigt" } else { "vorlaeufig" },
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": null, "signal_2": if bestaetigt { json!("ok") } else { Value::Null }},
        "ersetzt": null,
    });
    e["event_id"] = json!(EventId::von_json(&e).to_string());
    e
}

/// Präfixe der frühen Sperren; im sauberen Store fehlen diese Felder.
const FRUEH: &[&str] = &[
    "dba_",
    "p32b",
    "rentner_veraeusserungsgewinn",
    "antrag_ermaessigter",
    "kap_",
    "kein_gewinn",
    "kein_kap",
    "kein_vuv",
    "kein_sonstige",
    "kein_p23",
    "fam_alleinstehend",
    "einkuenfte_gewinn",
    "betriebseinnahmen",
    "sonstige_betriebsausgaben",
    "afa_",
    "gewinnanteil",
    "verguetung_",
];

/// Fokusgruppen (Teilstrings der Feld-Id): im Fokus liegt die Feld-Dichte bei 95 %.
const FOKUS: &[&[&str]] = &[
    &[],
    &["dhf_", "veranlagung"],
    &["tage_", "vpf_", "veranlagung"],
    &["uebernachtung_", "am_", "arbeitsmittel_"],
    &[
        "dba_",
        "p32b",
        "rentner_veraeusserungsgewinn",
        "antrag_",
        "gewst_",
        "geburtsjahr",
        "berufsunf",
        "ermaessigung",
    ],
    &[
        "kap_",
        "kein_",
        "gewinn",
        "betriebs",
        "einkuenfte_gewinn",
        "afa_",
        "gwg_",
        "verguetung",
        "veranlagung",
    ],
    &[
        "lohnersatz",
        "verlustvortrag",
        "unterhalt",
        "kist_",
        "realsplitting",
        "fahrtkosten",
        "behinderung",
        "gewst_",
    ],
    &["vv_", "veranlagung", "bruttoarbeitslohn_partner", "vor_"],
    &[
        "rentner_",
        "basis_",
        "versicherungsart",
        "veranlagung",
        "versorgung_",
        "kein_",
    ],
    &[
        "hh_",
        "p35c",
        "kind",
        "p22_",
        "berufsausbildung",
        "gwg_",
        "veranlagung",
    ],
];

fn generiere_store(
    c: &mut Cursor,
    index: &HashMap<String, &'static Bindung>,
) -> (Value, u16, bool) {
    let dichte = *c.waehle(&[15, 35, 60, 90]);
    let bestaetigt_pct = *c.waehle(&[50, 80, 100]);
    let wahr_pct = *c.waehle(&[40, 85, 95]);
    let fokus = FOKUS[c.range(FOKUS.len() as u64) as usize];
    // Jeder zehnte Store darf Floats in `konsistenz`-Feldern tragen (dann ohne die zwei `konsistenz`-Funktionen).
    let float_modus = c.chance(10);
    // Sauberer Store (40 %): die frühen Sperren (DBA, § 32b, § 16 Abs. 4, Flags, Kapital) bleiben weg
    // und Schalter sind selten wahr — sonst erreicht kaum ein Store die späten Prüfungen.
    let sauber = c.chance(40);
    let wahr_pct = if sauber { 12 } else { wahr_pct };
    let mut events: Vec<(String, Value, bool)> = Vec::new();
    let inst_zustand = [
        c.chance(bestaetigt_pct),
        c.chance(bestaetigt_pct),
        c.chance(bestaetigt_pct),
    ];
    for (fid, kind) in FELDER {
        if sauber && FRUEH.iter().any(|p| fid.starts_with(p)) {
            continue;
        }
        let im_fokus = fokus.iter().any(|p| fid.contains(p));
        // Partnerfelder ohne Fokus selten: sonst erzeugen sie fast jeden Store `partner_konsistenz_offen`.
        let dichte_hier = if im_fokus {
            95
        } else if fid.contains("_partner") {
            dichte / 4
        } else {
            dichte
        };
        if !c.chance(dichte_hier) {
            continue;
        }
        let gruppe = index.get(*fid).is_some_and(|b| b.instanz_gruppe.is_some());
        let instanzen: &[u64] = if gruppe {
            &[1, 2, 3][..=(c.range(3) as usize)]
        } else {
            &[1]
        };
        for n in instanzen {
            let id = match n {
                1 if gruppe && c.chance(5) => format!("{fid}__1"),
                1 => (*fid).to_owned(),
                n => format!("{fid}__{n}"),
            };
            let z = if gruppe {
                inst_zustand[(*n - 1) as usize] != c.chance(4)
            } else {
                c.chance(bestaetigt_pct)
            };
            events.push((
                id,
                wert(c, *kind, wahr_pct, !float_modus && konsistenz_feld(fid)),
                z,
            ));
        }
    }
    if !events.is_empty() && c.chance(15) {
        let (fid, _, _) = events[c.range(events.len() as u64) as usize].clone();
        events.push((fid, wert(c, Cent, 50, true), c.chance(50)));
    }
    let evs: Vec<Value> = events
        .iter()
        .enumerate()
        .map(|(i, (f, w, b))| event(i, f, w, *b))
        .collect();
    let vz = *c.waehle(&[2024_u16, 2025, 2026]);
    (
        json!({"version": 1, "veranlagungszeitraum": vz, "events": evs}),
        vz,
        float_modus,
    )
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

#[test]
fn generierte_faelle() {
    if skip() {
        return;
    }
    let b = std::cell::RefCell::new(Bilanz::default());
    let n = std::cell::Cell::new(0_usize);
    let kontexte = std::cell::Cell::new(0_usize);
    let floats = std::cell::Cell::new(0_usize);
    let n_faelle =
        parity::fallzahl::holen_u32("bescheid_deklaration_paritaet generierte_faelle", 1200);
    let ergebnis = runner(n_faelle).run(&prop::collection::vec(any::<u8>(), 1024..4096), |bytes| {
        let mut c = Cursor {
            bytes: &bytes,
            pos: 0,
        };
        let (store, vz, float_modus) = generiere_store(&mut c, index());
        floats.set(floats.get() + usize::from(float_modus));
        n.set(n.get() + 1);
        let vorher = b.borrow().abweichungen();
        // Jeder vierte Fall bekommt ALLE Aufrufformen (alle sechs Scheiben-Kontexte).
        let dicht = n.get().is_multiple_of(4);
        for k in formen(&Quelle::Store(store), vz, None, dicht, float_modus) {
            vergleiche(
                &mut b.borrow_mut(),
                &k,
                &format!("gen#{}", n.get()),
                true,
                false,
            );
            kontexte.set(kontexte.get() + 1);
        }
        prop_assert_eq!(
            b.borrow().abweichungen(),
            vorher,
            "{:?}",
            b.borrow().abweichungen.last()
        );
        Ok(())
    });
    b.borrow().drucke("generierte_faelle", kontexte.get());
    assert_eq!(
        b.borrow().abweichungen(),
        0,
        "generierte_faelle: Abweichungen (Anzahl s. o.)"
    );
    // Pin und Untergrenzen gelten beim Standard; sonst muss der Lauf mindestens die verlangte Zahl
    // rechnen.
    let wachen = parity::fallzahl::wache_gilt(
        "bescheid_deklaration_paritaet generierte_faelle",
        n_faelle as usize,
        1200,
    );
    if wachen {
        b.borrow().wache_rechnet("generierte_faelle", &[]);
    }
    b.borrow().drucke_gruende();
    eprintln!(
        "generierte_faelle: {} Stores ({} im Float-Modus), {} Kontexte",
        n.get(),
        floats.get(),
        kontexte.get()
    );
    ergebnis.unwrap();
    assert!(n.get() >= if wachen { 1000 } else { n_faelle as usize });
    if wachen {
        for f in FUNKTIONEN {
            assert!(
                b.borrow().zeilen[f].python >= 1000,
                "{f}: weniger als 1.000 Aufrufe"
            );
        }
    }
}

// ---------------------------------------------------------------- Negativkontrolle

/// Ein Rust-Ergebnis gestört → GENAU eine Abweichung. Ohne diesen Test könnte `vergleiche` kaputt
/// sein (immer 0) und die Läufe oben trotzdem grün.
#[test]
fn negativkontrolle_erkennt_genau_eine_abweichung() {
    if skip() {
        return;
    }
    let f = |wert: Value| json!({"wert": wert, "zustand": "bestaetigt", "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}});
    // Nicht-leere Ergebnisse: Kap → E1900401/E1901401, vv_einnahmen → Summen, hh-Instanz → Summe;
    // p35c-Betrag ohne Doppelförderungs-Antwort → Sperrgrund.
    let felder = json!({
        "veranlagung": f(json!("einzel")), "kap_kapitalertraege": f(json!(300_000)),
        "vv_einnahmen": f(json!(1_200_000)), "vv_schuldzinsen": f(json!(200_000)),
        "hh_handwerker_betrag": f(json!(500_000)), "p35c_sanierungsaufwendungen": f(json!(700_000)),
    });
    let k = Kontext {
        quelle: Quelle::Felder(felder),
        vz: 2025,
        scheibe: Some("gesamt"),
        store_uebergeben: true,
        bindung_uebergeben: true,
        vz_ohne: false,
        float_modus: false,
    };
    let mut sauber = Bilanz::default();
    vergleiche(&mut sauber, &k, "neg", true, false);
    let mut gestoert = Bilanz::default();
    vergleiche(&mut gestoert, &k, "neg", true, true);
    let nicht_leer_zahl: u64 = sauber.zeilen.values().map(|z| z.nicht_leer).sum();
    gestoert.drucke("negativkontrolle (gestört)", 1);
    eprintln!("negativkontrolle: ungestört {} Abweichungen, gestört {} (erwartet 0 und 1), nicht-leere Ergebnisse: {nicht_leer_zahl}", sauber.abweichungen(), gestoert.abweichungen());
    assert_eq!(sauber.abweichungen(), 0);
    assert_eq!(
        gestoert.abweichungen(),
        1,
        "die Störung muss GENAU eine Abweichung erzeugen"
    );
    assert!(
        nicht_leer_zahl >= 2,
        "der Kontrollfall muss nicht-leere Ergebnisse tragen"
    );

    // Zweite Störung auf der Sperrgrund-Zeichenkette (der Text ist kein i64).
    let mut r = json!("p35c_doppelfoerderung_offen");
    assert!(stoere(&mut r));
    assert_ne!(r, json!("p35c_doppelfoerderung_offen"));
}

// ---------------------------------------------------------------- gezielte Fälle

/// Der Sperrgrund des Wächters bei vorläufigen oder fehlenden Nach-Frist-Tagen und mehr als 3 Monaten am Ort.
const VPF_FRIST_GRUND: &str = "verpflegung_dreimonatsfrist_aufteilung_offen";

/// `mit_ring_werten` liest in der Verpflegung nur BESTÄTIGTE Werte (Entscheid
/// verpflegung-vorschau-liest-nur-bestaetigte-werte): 14 Felder-Gruppen × (bestätigt, fehlt, vorläufig) × 3 Scheiben
/// = 126 Fälle, alles andere bestätigt. Die Gruppen sind die 42 Fälle der Messung von `neunc` (69119b9). Der
/// erwartete Sperrgrund ist der des Wächters: nur fehlende oder vorläufige Nach-Frist-Tage bei 4 Monaten sperren.
fn verpflegung_faelle() -> Vec<(&'static str, &'static str, Vec<(&'static str, Value, bool)>)> {
    type Paare = Vec<(&'static str, i64)>;
    const NACH: [&str; 3] = [
        "vpf_tage_24h_nach_drei_monaten",
        "vpf_tage_an_abreise_nach_drei_monaten",
        "vpf_tage_ueber_8h_nach_drei_monaten",
    ];
    let ohne = |b: &Paare, feld: &str| -> Paare {
        b.iter().filter(|(f, _)| *f != feld).copied().collect()
    };
    let m: Paare = vec![
        ("tage_24h", 100),
        ("vpf_fruehstuecke_gestellt_anzahl", 5),
        ("vpf_mittagessen_gestellt_anzahl", 2),
        ("vpf_abendessen_gestellt_anzahl", 2),
    ];
    let mahlzeiten_p: Paare = vec![
        ("vpf_fruehstuecke_gestellt_anzahl", 10),
        ("vpf_mittagessen_gestellt_anzahl", 10),
        ("vpf_abendessen_gestellt_anzahl", 10),
    ];
    let tage_p: Paare = vec![
        ("tage_24h", 3),
        ("tage_an_abreise", 2),
        ("tage_ueber_8h_eintaegig", 2),
    ];
    let p: Paare = [tage_p.clone(), mahlzeiten_p.clone()].concat();
    let nach: Paare = vec![(NACH[0], 2), (NACH[1], 1), (NACH[2], 1)];
    // (Feld, Wert, bestätigte Basis ohne das Feld, Monate am Ort)
    let mut gruppen: Vec<(&'static str, i64, Paare, i64)> = vec![];
    for &(f, w) in &m[1..] {
        gruppen.push((f, w, ohne(&m, f), 2));
    }
    gruppen.push(("vpf_mahlzeiten_gezahltes_entgelt", 2000, m.clone(), 2));
    for &(f, w) in &tage_p {
        gruppen.push((f, w, ohne(&p, f), 2));
    }
    gruppen.push(("tage_24h", 3, mahlzeiten_p, 2));
    for monate in [2, 4] {
        for &(f, w) in &nach {
            let basis = [
                p.clone(),
                nach.iter()
                    .filter(|(g, _)| *g != f)
                    .map(|(g, _)| (*g, 0))
                    .collect(),
            ]
            .concat();
            gruppen.push((f, w, basis, monate));
        }
    }
    assert_eq!(gruppen.len(), 14);
    let mut out = vec![];
    for scheibe in ["gesamt", "an_gesamt", "n_vor_gwg"] {
        for (feld, wert, basis, monate) in &gruppen {
            for variante in ["bestaetigt", "fehlt", "vorlaeufig"] {
                let mut ev: Vec<(&'static str, Value, bool)> =
                    basis.iter().map(|(f, w)| (*f, json!(*w), true)).collect();
                ev.push(("vpf_monate_am_ort", json!(*monate), true));
                if variante != "fehlt" {
                    ev.push((*feld, json!(*wert), variante == "bestaetigt"));
                }
                let erwartet = if *monate > 3 && NACH.contains(feld) && variante != "bestaetigt" {
                    VPF_FRIST_GRUND
                } else {
                    "(keine Sperre)"
                };
                out.push((erwartet, scheibe, ev));
            }
        }
    }
    out
}

/// § 34 Abs. 3: der Zwilling `p34_abs3_antragsbetrag` (Antrags-Kz G E0801602 / S E0805003 /
/// L E0901704, Vault `p34-antrag-ohne-kennzahl-erreicht-elster-nicht`) entsteht in `mit_ring_werten`
/// nur bei bestaetigtem Antrag UND Berechtigung UND 0 < `netto_vg` <= 5 Mio EUR. Je Fall: Python gleich
/// Rust (alle fuenf Funktionen, `werte`), dazu die Erwartung "Zwilling da / nicht da" an PYTHON — ohne
/// sie waere ein Fall, in dem beide Seiten nichts schreiben, gruen. Die Grenzfaelle des Netto-Gewinns
/// sind Cent genau: 5.000.000,00 EUR (Freibetrag 0) schreibt, 5.000.001,00 EUR nicht.
#[test]
fn p34_antrag_zwilling() {
    if skip() {
        return;
    }
    let eligible = |art: &'static str, vg: i64| -> Vec<(&'static str, Value, bool)> {
        vec![
            ("antrag_ermaessigter_satz", json!(true), true),
            ("geburtsjahr", json!(1960), true),
            ("dauernd_berufsunfaehig", json!(false), true),
            ("ermaessigung_einmal_genutzt", json!(false), true),
            ("rentner_alter_55_oder_berufsunfaehig", json!(true), true),
            ("rentner_freibetrag_erstmalig", json!(true), true),
            ("rentner_veraeusserungsgewinn", json!(vg), true),
            ("rentner_veraeusserungs_betriebsart", json!(art), true),
        ]
    };
    let mit = |f: Vec<(&'static str, Value, bool)>, neu: (&'static str, Value, bool)| {
        let mut f: Vec<_> = f.into_iter().filter(|(n, ..)| *n != neu.0).collect();
        f.push(neu);
        f
    };
    // (Name, Scheibe, Felder, vz ohne Jahr, Zwilling erwartet)
    let faelle: Vec<(
        &str,
        &'static str,
        Vec<(&'static str, Value, bool)>,
        bool,
        bool,
    )> = vec![
        (
            "gewerbe",
            "gesamt",
            eligible("gewerbe", 50_000_000),
            false,
            true,
        ),
        (
            "selbstaendig",
            "gesamt",
            eligible("selbstaendig", 50_000_000),
            false,
            true,
        ),
        (
            "land_forst",
            "gesamt",
            eligible("land_forst", 50_000_000),
            false,
            true,
        ),
        (
            "gewerbe/rentner_gesamt",
            "rentner_gesamt",
            eligible("gewerbe", 50_000_000),
            false,
            true,
        ),
        (
            "netto genau 5 Mio",
            "gesamt",
            eligible("gewerbe", 500_000_000),
            false,
            true,
        ),
        (
            "netto 1 EUR ueber 5 Mio",
            "gesamt",
            eligible("gewerbe", 500_000_100),
            false,
            false,
        ),
        (
            "viel ueber 5 Mio",
            "gesamt",
            eligible("gewerbe", 600_000_000),
            false,
            false,
        ),
        (
            "Antrag nein",
            "gesamt",
            mit(
                eligible("gewerbe", 50_000_000),
                ("antrag_ermaessigter_satz", json!(false), true),
            ),
            false,
            false,
        ),
        (
            "Antrag vorlaeufig",
            "gesamt",
            mit(
                eligible("gewerbe", 50_000_000),
                ("antrag_ermaessigter_satz", json!(true), false),
            ),
            false,
            false,
        ),
        (
            "Antrag fehlt",
            "gesamt",
            eligible("gewerbe", 50_000_000)
                .into_iter()
                .filter(|(n, ..)| *n != "antrag_ermaessigter_satz")
                .collect(),
            false,
            false,
        ),
        (
            "zu jung",
            "gesamt",
            mit(
                eligible("gewerbe", 50_000_000),
                ("geburtsjahr", json!(1990), true),
            ),
            false,
            false,
        ),
        (
            "schon genutzt",
            "gesamt",
            mit(
                eligible("gewerbe", 50_000_000),
                ("ermaessigung_einmal_genutzt", json!(true), true),
            ),
            false,
            false,
        ),
        (
            "netto null (unter Freibetrag)",
            "gesamt",
            eligible("gewerbe", 4_000_000),
            false,
            false,
        ),
        (
            "ohne Veranlagungsjahr",
            "gesamt",
            eligible("gewerbe", 50_000_000),
            true,
            false,
        ),
    ];
    let mut b = Bilanz::default();
    let mut da = 0;
    for (name, scheibe, felder, vz_ohne, erwartet) in &faelle {
        let evs: Vec<Value> = felder
            .iter()
            .enumerate()
            .map(|(i, (f, w, z))| event(i, f, w, *z))
            .collect();
        let k = Kontext {
            quelle: Quelle::Store(
                json!({"version": 1, "veranlagungszeitraum": 2025, "events": evs}),
            ),
            vz: 2025,
            scheibe: Some(scheibe),
            store_uebergeben: true,
            bindung_uebergeben: true,
            vz_ohne: *vz_ohne,
            float_modus: false,
        };
        let py = frage_roh(&k.request(&["mit_ring_werten"]));
        let py_da = py["bescheid.mit_ring_werten"]["ok"]
            .get("p34_abs3_antragsbetrag")
            .is_some();
        assert_eq!(
            py_da,
            *erwartet,
            "Python: Zwilling {} (Fall {name}): {}",
            if py_da { "da" } else { "fehlt" },
            py["bescheid.mit_ring_werten"]
        );
        da += usize::from(py_da);
        vergleiche(&mut b, &k, name, true, false);
    }
    b.drucke("p34_antrag_zwilling", faelle.len());
    assert_eq!(b.abweichungen(), 0);
    assert_eq!(
        da, 5,
        "fuenf Faelle schreiben den Zwilling, alle anderen nicht"
    );
}

/// § 34 Abs. 3 fuer A + Veraeusserungsgewinn beim Ehegatten (`abs3_partner_gewinn_offen`, Vault
/// `p34-antrag-ohne-kennzahl-erreicht-elster-nicht` `AK2b`, Entscheid 2026-10-03): zusammen UND Antrag
/// bestaetigt UND Berechtigung UND 0 < `netto_vg` <= 5 Mio UND ROHER Partner-Gewinn > 0; nur bestaetigte
/// Felder urteilen, ein Betrag <= 0 sperrt nie. Je Fall: Python gleich Rust (alle fuenf Funktionen) UND
/// der erwartete Grund an PYTHON festgenagelt (ein Fall, in dem beide Seiten aus demselben falschen Grund
/// nichts sperren, waere sonst gruen). Der Abweichungsfall roh/netto: Partner 40.000 EUR unter dem
/// Freibetrag sperrt. Rust-hermetisch ohne Python: `bescheid/tests/abs3_partner_gewinn.rs`.
#[test]
fn p34_partner_gewinn_sperre() {
    const GRUND: &str = "abs3_partner_gewinn_offen";
    const KEINE: &str = "(keine Sperre)";
    type Paare = Vec<(&'static str, Value, bool)>;
    if skip() {
        return;
    }
    let basis = |abw: &[(&'static str, Value, bool)]| -> Paare {
        let mut e: Paare = vec![
            ("veranlagung", json!("zusammen"), true),
            ("antrag_ermaessigter_satz", json!(true), true),
            ("geburtsjahr", json!(1960), true),
            ("dauernd_berufsunfaehig", json!(false), true),
            ("ermaessigung_einmal_genutzt", json!(false), true),
            ("rentner_alter_55_oder_berufsunfaehig", json!(true), true),
            ("rentner_freibetrag_erstmalig", json!(true), true),
            ("rentner_veraeusserungsgewinn", json!(50_000_000), true),
            ("rentner_veraeusserungs_betriebsart", json!("gewerbe"), true),
            (
                "rentner_veraeusserungsgewinn_partner",
                json!(30_000_000),
                true,
            ),
            (
                "rentner_alter_55_oder_berufsunfaehig_partner",
                json!(true),
                true,
            ),
            ("rentner_freibetrag_erstmalig_partner", json!(true), true),
            ("kein_gewinn", json!(false), true),
            ("bruttoarbeitslohn_partner", json!(0), true),
            ("kap_kapitalertraege_partner", json!(0), true),
            ("kap_gewinn_aktien_partner", json!(0), true),
            ("kap_gewinn_sonstige_partner", json!(0), true),
            ("kap_verlust_aktien_partner", json!(0), true),
            ("kap_verlust_sonstige_partner", json!(0), true),
        ];
        for (f, w, b) in abw {
            e.retain(|(g, _, _)| g != f);
            if !w.is_null() {
                e.push((f, w.clone(), *b));
            }
        }
        e
    };
    let pvg = "rentner_veraeusserungsgewinn_partner";
    let faelle: Vec<(&str, &str, Paare)> = vec![
        ("voll", GRUND, basis(&[])),
        ("Partner 1 Cent", GRUND, basis(&[(pvg, json!(1), true)])),
        (
            "Partner 40.000 EUR unter Freibetrag (roh/netto)",
            GRUND,
            basis(&[(pvg, json!(4_000_000), true)]),
        ),
        ("Partner-VG 0", KEINE, basis(&[(pvg, json!(0), true)])),
        (
            "Partner-VG negativ",
            KEINE,
            basis(&[(pvg, json!(-1), true)]),
        ),
        (
            "Partner-VG fehlt",
            KEINE,
            basis(&[(pvg, Value::Null, true)]),
        ),
        (
            "Partner-VG vorlaeufig",
            KEINE,
            basis(&[(pvg, json!(30_000_000), false)]),
        ),
        (
            "Antrag vorlaeufig Ja",
            KEINE,
            basis(&[("antrag_ermaessigter_satz", json!(true), false)]),
        ),
        (
            "Antrag vorlaeufig Nein",
            KEINE,
            basis(&[("antrag_ermaessigter_satz", json!(false), false)]),
        ),
        (
            "Veranlagung vorlaeufig",
            KEINE,
            basis(&[("veranlagung", json!("zusammen"), false)]),
        ),
        (
            "Geburtsjahr vorlaeufig",
            KEINE,
            basis(&[("geburtsjahr", json!(1960), false)]),
        ),
        (
            "Gewinn A vorlaeufig",
            KEINE,
            basis(&[("rentner_veraeusserungsgewinn", json!(50_000_000), false)]),
        ),
        (
            "Antrag Nein",
            KEINE,
            basis(&[("antrag_ermaessigter_satz", json!(false), true)]),
        ),
        (
            "Antrag fehlt",
            KEINE,
            basis(&[("antrag_ermaessigter_satz", Value::Null, true)]),
        ),
        (
            "Einzelveranlagung",
            KEINE,
            basis(&[("veranlagung", json!("einzel"), true)]),
        ),
        (
            "A zu jung",
            KEINE,
            basis(&[("geburtsjahr", json!(1990), true)]),
        ),
        (
            "A schon genutzt",
            KEINE,
            basis(&[("ermaessigung_einmal_genutzt", json!(true), true)]),
        ),
        (
            "A netto 0",
            KEINE,
            basis(&[("rentner_veraeusserungsgewinn", json!(4_000_000), true)]),
        ),
        (
            "A ohne Gewinn",
            KEINE,
            basis(&[("rentner_veraeusserungsgewinn", Value::Null, true)]),
        ),
        (
            "A ueber 5 Mio: eigener Grund davor",
            "abs3_ueber_5mio_offen",
            basis(&[("rentner_veraeusserungsgewinn", json!(600_000_000), true)]),
        ),
    ];
    let mut b = Bilanz::default();
    let mut gesperrt = 0;
    for scheibe in ["gesamt", "rentner_gesamt"] {
        for (name, erwartet, felder) in &faelle {
            let evs: Vec<Value> = felder
                .iter()
                .enumerate()
                .map(|(i, (f, w, z))| event(i, f, w, *z))
                .collect();
            let k = Kontext {
                quelle: Quelle::Store(
                    json!({"version": 1, "veranlagungszeitraum": 2025, "events": evs}),
                ),
                vz: 2025,
                scheibe: Some(scheibe),
                store_uebergeben: true,
                bindung_uebergeben: true,
                vz_ohne: false,
                float_modus: false,
            };
            let py = frage_roh(&k.request(&["an_gesamt_sperrgrund"]));
            let py_grund = py["bescheid.an_gesamt_sperrgrund"]["ok"]
                .as_str()
                .unwrap_or(KEINE);
            assert_eq!(
                py_grund, *erwartet,
                "Python: falscher Grund ({scheibe}: {name})"
            );
            gesperrt += usize::from(py_grund == GRUND);
            vergleiche(&mut b, &k, &format!("{scheibe}: {name}"), true, false);
        }
    }
    b.drucke("p34_partner_gewinn_sperre", faelle.len() * 2);
    assert_eq!(b.abweichungen(), 0);
    assert_eq!(gesperrt, 6, "3 Sperrfaelle je Scheibe, alle anderen nicht");
}

/// Die Sperrgründe, die der Zufall nicht zuverlässig erreicht (spät im Guard hinter frühen Sperren).
/// Jeder Fall ist von Hand gebaut; der Test verlangt, dass PYTHON den erwarteten Grund liefert (der
/// Fall trifft die Stelle wirklich) und dass Rust ihn ebenso liefert.
#[test]
fn gezielte_faelle() {
    if skip() {
        return;
    }
    let partner_kegel: Vec<(&str, Value, bool)> =
        std::iter::once(("veranlagung", json!("zusammen"), true))
            .chain(
                [
                    "bruttoarbeitslohn_partner",
                    "kap_kapitalertraege_partner",
                    "kap_gewinn_aktien_partner",
                    "kap_gewinn_sonstige_partner",
                    "kap_verlust_aktien_partner",
                    "kap_verlust_sonstige_partner",
                ]
                .map(|f| (f, json!(0), true)),
            )
            .collect();
    let euer_leer: Vec<(&str, Value, bool)> = vec![
        ("kein_gewinn", json!(false), true),
        ("betriebseinnahmen", json!(0), true),
        ("sonstige_betriebsausgaben", json!(0), true),
        ("afa_jahresbetrag", json!(0), true),
    ];
    let mut faelle: Vec<(&str, &str, Vec<(&str, Value, bool)>)> = vec![
        (
            "abs3_ueber_5mio_offen",
            "gesamt",
            vec![
                ("antrag_ermaessigter_satz", json!(true), true),
                ("geburtsjahr", json!(1960), true),
                ("rentner_veraeusserungsgewinn", json!(600_000_000), true),
            ],
        ),
        // Grenze der 5-Mio-Schwelle (`// 100 > 5_000_000` auf Cent): 5.000.000,00 und 5.000.000,99 EUR
        // liegen NICHT darueber (dann greift erst § 16 Abs. 4), 5.000.001,00 EUR schon.
        (
            "p16_4_gate_offen",
            "gesamt",
            vec![
                ("antrag_ermaessigter_satz", json!(true), true),
                ("geburtsjahr", json!(1960), true),
                ("rentner_veraeusserungsgewinn", json!(500_000_000), true),
            ],
        ),
        (
            "p16_4_gate_offen",
            "gesamt",
            vec![
                ("antrag_ermaessigter_satz", json!(true), true),
                ("geburtsjahr", json!(1960), true),
                ("rentner_veraeusserungsgewinn", json!(500_000_099), true),
            ],
        ),
        (
            "abs3_ueber_5mio_offen",
            "gesamt",
            vec![
                ("antrag_ermaessigter_satz", json!(true), true),
                ("geburtsjahr", json!(1960), true),
                ("rentner_veraeusserungsgewinn", json!(500_000_100), true),
            ],
        ),
        // 3-Monats-Frist: mehr als 3 Monate am Ort, Tage > 0, alle Nach-Frist-Angaben 0 → Rueckfrage
        // zur Unterbrechung; mit beantworteter Rueckfrage rueckt der Guard zur Mahlzeitenfrage vor.
        (
            "verpflegung_dreimonatsfrist_unterbrechung_offen",
            "gesamt",
            vec![
                ("tage_24h", json!(10), true),
                ("vpf_monate_am_ort", json!(4), true),
                ("vpf_tage_24h_nach_drei_monaten", json!(0), true),
            ],
        ),
        (
            "verpflegung_reduktion_offen",
            "gesamt",
            vec![
                ("tage_24h", json!(10), true),
                ("vpf_monate_am_ort", json!(4), true),
                ("vpf_tage_24h_nach_drei_monaten", json!(0), true),
                ("vpf_frist_nicht_unterbrochen", json!(false), true),
            ],
        ),
        (
            "gwg_tatbestand_offen",
            "gesamt",
            [
                euer_leer.clone(),
                vec![("gwg_anschaffungskosten_netto", json!(50_000), true)],
            ]
            .concat(),
        ),
        (
            "kinderbetreuung_reine_betreuung_offen",
            "gesamt",
            vec![
                ("kind_idnr", json!("12345678901"), true),
                ("kind_unter_14_haushaltszugehoerig", json!(true), true),
                ("kinderbetreuungskosten", json!(100_000), true),
            ],
        ),
        (
            "kinderbetreuung_zahlung_offen",
            "gesamt",
            vec![
                ("kind_idnr", json!("12345678901"), true),
                ("kind_unter_14_haushaltszugehoerig", json!(true), true),
                ("kinderbetreuungskosten", json!(100_000), true),
                ("kind_betreuung_reine_betreuung", json!(true), true),
                ("kind_betreuung_rechnung_ueberweisung", json!(false), true),
            ],
        ),
        (
            "behinderungsbedingte_aufwendungen_wahlrecht_partner_offen",
            "gesamt",
            [
                partner_kegel.clone(),
                vec![
                    ("rentner_grad_der_behinderung_partner", json!(50), true),
                    (
                        "behinderungsbedingte_aufwendungen_partner",
                        json!(100_000),
                        true,
                    ),
                ],
            ]
            .concat(),
        ),
        (
            "behinderungsbedingte_aufwendungen_wahlrecht_offen",
            "rentner_gesamt",
            vec![
                ("rentner_grad_der_behinderung", json!(50), true),
                ("behinderungsbedingte_aufwendungen", json!(100_000), true),
            ],
        ),
        // § 22 aa: Beginn nach dem VZ (der Ring hat keinen Zweig dafuer) und, als Gegenstueck derselben
        // Funktion, Beginn vor dem VZ ohne Freibetrag. Der Zufallskorpus trifft beides nicht.
        (
            "rentenbeginn_nach_vz",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("gesetzliche_rente"), true),
                ("rentner_renten_beginn_jahr", json!(2026), true),
            ],
        ),
        (
            "rentenfreibetrag_fixierung_offen",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("gesetzliche_rente"), true),
                ("rentner_renten_beginn_jahr", json!(2024), true),
            ],
        ),
        (
            "rentenbeginn_nach_vz",
            "rentner_gesamt",
            vec![
                ("veranlagung", json!("zusammen"), true),
                ("rentner_renten_art_partner", json!("gesetzliche_rente"), true),
                ("rentner_jahresrente_partner", json!(1_200_000), true),
                ("rentner_renten_beginn_jahr_partner", json!(2026), true),
                ("rentner_alter_bei_rentenbeginn_partner", json!(65), true),
            ],
        ),
        // § 22 bb sperrt nach dem VZ wie aa; vor dem VZ ohne Freibetrag sperrt bb nicht (kein Freibetrag).
        (
            "rentenbeginn_nach_vz",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("private_leibrente"), true),
                ("rentner_renten_beginn_jahr", json!(2026), true),
            ],
        ),
        (
            "rentenbeginn_nach_vz",
            "rentner_gesamt",
            vec![
                ("veranlagung", json!("zusammen"), true),
                ("rentner_renten_art_partner", json!("sonstige_leibrente"), true),
                ("rentner_jahresrente_partner", json!(1_200_000), true),
                ("rentner_renten_beginn_jahr_partner", json!(2026), true),
                ("rentner_alter_bei_rentenbeginn_partner", json!(65), true),
            ],
        ),
        (
            "(keine Sperre)",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("private_leibrente"), true),
                ("rentner_renten_beginn_jahr", json!(2024), true),
            ],
        ),
        // § 22: Jahr <= 0 wird zum Kz-Datum "01.01.0000", ERiC lehnt es ab. Der Grund steht VOR der
        // Freibetrag-Bedingung: aa ohne Freibetrag meldet ihn statt der Fixierung. Person A (Basis und
        // Instanz `__2`), Person B, aa mit und ohne Freibetrag, beide Leibrenten; -1 = Direktweg am
        // Speicher vorbei. Die Kontrollen (Jahr 1) sperren nicht.
        (
            "rentenbeginn_jahr_ungueltig",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("gesetzliche_rente"), true),
                ("rentner_renten_beginn_jahr", json!(0), true),
            ],
        ),
        (
            "rentenbeginn_jahr_ungueltig",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("gesetzliche_rente"), true),
                ("rentner_renten_beginn_jahr", json!(0), true),
                ("rentner_rentenfreibetrag", json!(600_000), true),
            ],
        ),
        (
            "rentenbeginn_jahr_ungueltig",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("gesetzliche_rente"), true),
                ("rentner_renten_beginn_jahr", json!(-1), true),
            ],
        ),
        (
            "rentenbeginn_jahr_ungueltig",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("private_leibrente"), true),
                ("rentner_renten_beginn_jahr", json!(0), true),
            ],
        ),
        (
            "rentenbeginn_jahr_ungueltig",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("sonstige_leibrente"), true),
                ("rentner_renten_beginn_jahr", json!(-1), true),
            ],
        ),
        (
            "rentenbeginn_jahr_ungueltig",
            "rentner_gesamt",
            vec![
                ("kein_sonstige", json!(false), true),
                ("rentner_renten_art__2", json!("private_leibrente"), true),
                ("rentner_jahresrente__2", json!(900_000), true),
                ("rentner_renten_beginn_jahr__2", json!(0), true),
                ("rentner_alter_bei_rentenbeginn__2", json!(65), true),
            ],
        ),
        (
            "rentenbeginn_jahr_ungueltig",
            "rentner_gesamt",
            vec![
                ("veranlagung", json!("zusammen"), true),
                (
                    "rentner_renten_art_partner",
                    json!("gesetzliche_rente"),
                    true,
                ),
                ("rentner_jahresrente_partner", json!(1_200_000), true),
                ("rentner_renten_beginn_jahr_partner", json!(0), true),
                ("rentner_alter_bei_rentenbeginn_partner", json!(65), true),
            ],
        ),
        (
            "rentenbeginn_jahr_ungueltig",
            "rentner_gesamt",
            vec![
                ("veranlagung", json!("zusammen"), true),
                (
                    "rentner_renten_art_partner",
                    json!("private_leibrente"),
                    true,
                ),
                ("rentner_jahresrente_partner", json!(1_200_000), true),
                ("rentner_renten_beginn_jahr_partner", json!(-1), true),
                ("rentner_alter_bei_rentenbeginn_partner", json!(65), true),
            ],
        ),
        (
            "(keine Sperre)",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("gesetzliche_rente"), true),
                ("rentner_renten_beginn_jahr", json!(1), true),
                ("rentner_rentenfreibetrag", json!(600_000), true),
            ],
        ),
        (
            "(keine Sperre)",
            "rentner_gesamt",
            vec![
                ("rentner_renten_art", json!("private_leibrente"), true),
                ("rentner_renten_beginn_jahr", json!(1), true),
            ],
        ),
        // Anlage KAP: `mit_ring_werten` liest nur BESTAETIGTE Werte. Python == Rust auf: vorlaeufiger Topf
        // allein (kein Antrag), vorlaeufiges Aggregat, gemischter Zustand (Antrag, Pauschbetrag nur aus dem
        // bestaetigten Topf), vorlaeufiger Partner-Topf in rentner_gesamt. Der Guard meldet nichts (keine
        // Sperre); verglichen wird die Ausgabe von `mit_ring_werten` in `vergleiche`.
        (
            "(keine Sperre)",
            "gesamt",
            vec![
                ("kein_kap", json!(false), true),
                ("kap_gewinn_sonstige", json!(175_000), false),
            ],
        ),
        (
            "(keine Sperre)",
            "gesamt",
            vec![
                ("kein_kap", json!(false), true),
                ("kap_kapitalertraege", json!(175_000), false),
            ],
        ),
        (
            "(keine Sperre)",
            "gesamt",
            vec![
                ("kein_kap", json!(false), true),
                ("kap_gewinn_aktien", json!(40_000), true),
                ("kap_gewinn_sonstige", json!(30_000), false),
            ],
        ),
        (
            "(keine Sperre)",
            "rentner_gesamt",
            vec![
                ("veranlagung", json!("zusammen"), true),
                ("kap_gewinn_sonstige_partner", json!(175_000), false),
            ],
        ),
        // § 35: der Hebesatz des Partner-Betriebs fehlt wie der von Person A.
        (
            "gewst_hebesatz_offen",
            "gesamt",
            vec![
                ("veranlagung", json!("zusammen"), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
            ],
        ),
        // § 35: Hebesatz 0 bei Messbetrag > 0 ist unmoeglich und sperrt wie ein fehlender, A und B.
        (
            "gewst_hebesatz_offen",
            "gesamt",
            vec![
                ("gewst_messbetrag", json!(100_000), true),
                ("gewst_hebesatz", json!(0), true),
            ],
        ),
        (
            "gewst_hebesatz_offen",
            "gesamt",
            vec![
                ("veranlagung", json!("zusammen"), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
                ("gewst_hebesatz_partner", json!(0), true),
            ],
        ),
        // Ein negativer Hebesatz erst recht (main 2026-10-02), A und B.
        (
            "gewst_hebesatz_offen",
            "gesamt",
            vec![
                ("gewst_messbetrag", json!(100_000), true),
                ("gewst_hebesatz", json!(-1), true),
            ],
        ),
        (
            "gewst_hebesatz_offen",
            "gesamt",
            vec![
                ("veranlagung", json!("zusammen"), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
                ("gewst_hebesatz_partner", json!(-1), true),
            ],
        ),
        (
            "(keine Sperre)",
            "gesamt",
            vec![
                ("gewst_messbetrag", json!(0), true),
                ("gewst_hebesatz", json!(0), true),
            ],
        ),
    ];
    // GWG ohne Sofortabzug (main-Auftrag h8-gwg, 2026-10-03): jede Konstellation, die vorher still 0 abzog und
    // `bestaetigt` blieb, ist offen. Zwei neue Gruende, Reihenfolge je Instanz wie in `_an_gesamt_sperrgrund`:
    // nicht selbstaendig nutzbar -> "netto: nein" -> ueber 800 EUR -> unbeantwortet -> Verzeichnis (ueber 250 EUR).
    let gwg_fall = |betrag: i64, s: Option<bool>, n: Option<bool>, v: Option<bool>| {
        let mut f = euer_leer.clone();
        f.push(("gwg_anschaffungskosten_netto", json!(betrag), true));
        for (id, w) in [
            ("gwg_bewegliches_selbstaendig_nutzbar", s),
            ("gwg_netto_ohne_vorsteuer", n),
            ("gwg_verzeichnis_ab_250", v),
        ] {
            if let Some(w) = w {
                f.push((id, json!(w), true));
            }
        }
        f
    };
    for (erwartet, betrag, s, n, v) in [
        ("gwg_mehrwertsteuer_offen", 79_000, Some(true), Some(false), Some(true)),
        ("gwg_mehrwertsteuer_offen", 85_000, Some(true), Some(false), Some(true)),
        ("gwg_abschreibung_offen", 50_000, Some(true), Some(true), Some(false)),
        ("gwg_abschreibung_offen", 50_000, Some(false), Some(true), Some(true)),
        // nicht nutzbar UND netto=nein: die Mehrwertsteuer ist egal, kein GWG (Reihenfolge)
        ("gwg_abschreibung_offen", 50_000, Some(false), Some(false), Some(true)),
        ("gwg_abschreibung_offen", 100_000, Some(true), Some(true), Some(true)),
        ("gwg_abschreibung_offen", 100_000, None, None, None),
        ("gwg_abschreibung_offen", 80_001, Some(true), Some(true), Some(true)),
        ("gwg_abschreibung_offen", 25_001, Some(true), Some(true), Some(false)),
        // Grenzen und Ausweg: kein Fehlalarm
        ("(keine Sperre)", 80_000, Some(true), Some(true), Some(true)),
        ("(keine Sperre)", 25_000, Some(true), Some(true), Some(false)),
        ("(keine Sperre)", 0, Some(true), Some(false), Some(true)),
    ] {
        faelle.push((erwartet, "gesamt", gwg_fall(betrag, s, n, v)));
    }
    // Ein VORLAEUFIGES "nein" ist keine Antwort: gwg_tatbestand_offen, nicht gwg_mehrwertsteuer_offen.
    let mut gwg_vorlaeufig = gwg_fall(50_000, Some(true), None, Some(true));
    gwg_vorlaeufig.push(("gwg_netto_ohne_vorsteuer", json!(false), false));
    faelle.push(("gwg_tatbestand_offen", "gesamt", gwg_vorlaeufig));
    // Kontrollfall: dieselben Angaben mit beantworteten Fragen sperren NICHT (kein "immer gleicher Grund").
    faelle.push((
        "(keine Sperre)",
        "gesamt",
        [
            euer_leer,
            vec![
                ("gwg_anschaffungskosten_netto", json!(50_000), true),
                ("gwg_bewegliches_selbstaendig_nutzbar", json!(true), true),
                ("gwg_netto_ohne_vorsteuer", json!(true), true),
                ("gwg_verzeichnis_ab_250", json!(true), true),
            ],
        ]
        .concat(),
    ));
    faelle.extend(verpflegung_faelle());
    let mut b = Bilanz::default();
    for (erwartet, scheibe, felder) in &faelle {
        let evs: Vec<Value> = felder
            .iter()
            .enumerate()
            .map(|(i, (f, w, z))| event(i, f, w, *z))
            .collect();
        let k = Kontext {
            quelle: Quelle::Store(
                json!({"version": 1, "veranlagungszeitraum": 2025, "events": evs}),
            ),
            vz: 2025,
            scheibe: Some(scheibe),
            store_uebergeben: true,
            bindung_uebergeben: true,
            vz_ohne: false,
            float_modus: false,
        };
        let py = frage_roh(&k.request(&["an_gesamt_sperrgrund"]));
        let py_grund = py["bescheid.an_gesamt_sperrgrund"]["ok"]
            .as_str()
            .unwrap_or("(keine Sperre)");
        assert_eq!(
            py_grund, *erwartet,
            "Python trifft die Stelle nicht (Fall {erwartet})"
        );
        let c = baue_ctx(&k);
        let rust = rust_run(&c, "an_gesamt_sperrgrund").unwrap();
        assert_eq!(
            rust.as_str().unwrap_or("(keine Sperre)"),
            *erwartet,
            "Rust weicht ab (Fall {erwartet})"
        );
        vergleiche(&mut b, &k, erwartet, true, false);
    }
    b.drucke("gezielte_faelle", faelle.len());
    b.wache_rechnet("gezielte_faelle", LEER_GEZIELTE);
    eprintln!(
        "gezielte_faelle: {} Fälle, jeder trifft in Python den erwarteten Grund",
        faelle.len()
    );
    assert_eq!(b.abweichungen(), 0);
}
