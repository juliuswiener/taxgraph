//! Paritaet der Teil-2-Accessoren (`engine::zugriff::teil2`) gegen `runner.catala_*` ab
//! `catala_sparer_pb` (`REWRITE_PLAN.md` §5).
//!
//! 1. Korpus: jeder Datensatz aus `rust/fixtures/corpus/runner/catala_<name>.*.jsonl` gegen das
//!    aufgezeichnete Ergebnis UND gegen das Live-Orakel (Arbeitsbaum-Stand).
//! 2. Generiert: 1000 Proptest-dicts je Funktion gegen das Live-Orakel, inkl. Fehlerparitaet.
//! 3. Kohorten exhaustiv: jeder Tabellenschluessel der Renten-/Versorgungstabellen.
//! 4. Negativkontrolle: ein um 1 verschobenes Ergebnis muss als Abweichung auffallen.
//!
//!   `PARITY`=1 `cargo` test -p parity --test `zugriff_teil2_paritaet` -- --nocapture
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

#[path = "zugriff_teil2/adapter.rs"]
mod adapter;
#[path = "zugriff_teil2/gen.rs"]
mod gen;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use adapter::{python_typ, Adapter, Fehl, FUNKTIONEN};
use bindung::Params;
use parity::Oracle;
use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::TestRunner;
use serde_json::{json, Value};

const GENERIERT_JE_FUNKTION: usize = 1000;

/// Ausnahmeklassen des Catala-Laufzeitsystems (`catala_runtime.py`, `CatalaError`-Unterklassen).
const CATALA_KLASSEN: &[&str] = &[
    "CatalaError", "AssertionFailed", "NoValue", "Conflict", "DivisionByZero", "ListEmpty",
    "NotSameLength", "UncomparableValues", "DateError", "Impossible",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn aktiv() -> bool {
    let an = std::env::var("PARITY").as_deref() == Ok("1");
    if !an {
        eprintln!("PARITY!=1 -- uebersprungen (braucht Python-Umfeld)");
    }
    an
}

/// EIN Orakel-Prozess fuer die ganze Testdatei.
fn oracle() -> MutexGuard<'static, Oracle> {
    static O: OnceLock<Mutex<Oracle>> = OnceLock::new();
    O.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn params() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(|| Params::lade(&repo_root()).expect("params laden"))
}

/// Ergebnis einer Seite, vergleichbar gemacht.
#[derive(Debug, Clone, PartialEq)]
enum Ausgang {
    Ok(Value),
    Err(String),
    Catala,
}

fn rust(f: Adapter, args: &[Value]) -> Ausgang {
    match f(args, params()) {
        Ok(v) => Ausgang::Ok(v),
        Err(Fehl::Py(t)) => Ausgang::Err(t.to_string()),
        Err(Fehl::Engine(e)) => python_typ(&e).map_or(Ausgang::Catala, |t| Ausgang::Err(t.to_string())),
    }
}

/// `{"ok": v}` / `{"err": T}` / `{"err": T, "catala": bool}` -- Korpus- und Orakelform.
fn python(antwort: &Value) -> Ausgang {
    if let Some(v) = antwort.get("ok") {
        return Ausgang::Ok(v.clone());
    }
    let typ = antwort["err"].as_str().unwrap_or_else(|| panic!("unlesbar: {antwort}"));
    let catala = antwort.get("catala").and_then(Value::as_bool).unwrap_or(false);
    if catala || CATALA_KLASSEN.contains(&typ) {
        Ausgang::Catala
    } else {
        Ausgang::Err(typ.to_string())
    }
}

fn live(name: &str, args: &[Value]) -> Ausgang {
    python(&oracle().call_runner(&format!("catala_{name}"), args).expect("Orakel antwortet"))
}

#[derive(Default)]
struct Zaehler {
    faelle: usize,
    abweichungen: usize,
    fehlerfaelle: usize,
    beispiele: Vec<String>,
}

impl Zaehler {
    fn pruefe(&mut self, args: &[Value], r: &Ausgang, py: &Ausgang, quelle: &str) {
        self.faelle += 1;
        if matches!(py, Ausgang::Err(_) | Ausgang::Catala) {
            self.fehlerfaelle += 1;
        }
        if r != py {
            self.abweichungen += 1;
            if self.beispiele.len() < 3 {
                self.beispiele.push(format!("{quelle}: {} rust={r:?} python={py:?}", json!(args)));
            }
        }
    }
}

fn korpus(name: &str) -> Vec<Value> {
    let dir = repo_root().join("rust/fixtures/corpus/runner");
    let praefix = format!("catala_{name}.");
    let mut dateien: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&praefix)))
        .collect();
    dateien.sort();
    dateien
        .iter()
        .flat_map(|p| std::fs::read_to_string(p).unwrap().lines().map(str::to_string).collect::<Vec<_>>())
        .map(|z| serde_json::from_str::<Value>(&z).unwrap())
        .filter(|r| r.get("kopf").is_none())
        .collect()
}

fn args_von(r: &Value) -> Vec<Value> {
    assert!(r["kwargs"].as_object().is_none_or(serde_json::Map::is_empty), "kwargs nicht modelliert: {r}");
    r["args"].as_array().unwrap().clone()
}

#[test]
fn korpus_gegen_aufzeichnung_und_orakel() {
    if !aktiv() {
        return;
    }
    let mut summe = 0;
    eprintln!("{:<32} {:>6} {:>9} {:>9} {:>6}", "funktion", "faelle", "diff_rec", "diff_live", "err");
    for (name, f) in FUNKTIONEN {
        let (mut rec, mut liv) = (Zaehler::default(), Zaehler::default());
        for r in korpus(name) {
            let args = args_von(&r);
            let ergebnis = rust(*f, &args);
            rec.pruefe(&args, &ergebnis, &python(&r), "korpus/rec");
            liv.pruefe(&args, &ergebnis, &live(name, &args), "korpus/live");
        }
        eprintln!(
            "{name:<32} {:>6} {:>9} {:>9} {:>6}",
            rec.faelle, rec.abweichungen, liv.abweichungen, liv.fehlerfaelle
        );
        for b in rec.beispiele.iter().chain(&liv.beispiele) {
            eprintln!("    {b}");
        }
        summe += rec.abweichungen + liv.abweichungen;
    }
    assert_eq!(summe, 0, "Korpus-Abweichungen");
}

#[test]
fn generiert_gegen_orakel() {
    if !aktiv() {
        return;
    }
    let mut summe = 0;
    eprintln!("{:<32} {:>6} {:>6} {:>6}", "funktion", "faelle", "diff", "err");
    for (name, f) in FUNKTIONEN {
        let strategie = gen::fuer(name);
        let mut runner = TestRunner::deterministic();
        let mut z = Zaehler::default();
        for _ in 0..GENERIERT_JE_FUNKTION {
            let args = strategie.new_tree(&mut runner).unwrap().current();
            z.pruefe(&args, &rust(*f, &args), &live(name, &args), "generiert");
        }
        eprintln!("{name:<32} {:>6} {:>6} {:>6}", z.faelle, z.abweichungen, z.fehlerfaelle);
        for b in &z.beispiele {
            eprintln!("    {b}");
        }
        summe += z.abweichungen;
    }
    assert_eq!(summe, 0, "Abweichungen bei generierten Faellen");
}

/// Jeder Schluessel der Kohortentabellen (plus Raender ausserhalb) mit mehreren Betraegen:
/// deckt den ganzen Definitionsbereich von `Zehntelprozent` ab -- danach ist die Rechnung
/// exakte Ganzzahlarithmetik auf beiden Seiten.
#[test]
fn kohorten_exhaustiv() {
    if !aktiv() {
        return;
    }
    let renten = [1i64, 12_345, 100_000, 999_999, -5_555];
    let mut z = Zaehler::default();
    let mut fall = |name: &str, f: Adapter, d: Value| {
        let args = vec![d];
        z.pruefe(&args, &rust(f, &args), &live(name, &args), name);
    };
    for alter in -2i64..=100 {
        for r in renten {
            fall("renten_einkuenfte", adapter::renten_einkuenfte, json!({"veranlagungszeitraum": 2025,
                "renten_art": "private_leibrente", "jahresrente": r, "alter_bei_rentenbeginn": alter}));
        }
    }
    for vz in 2024i64..=2026 {
        for r in renten {
            fall("renten_einkuenfte", adapter::renten_einkuenfte, json!({"veranlagungszeitraum": vz,
                "renten_art": "gesetzliche_rente", "jahresrente": r, "renten_beginn_jahr": vz}));
        }
    }
    for beginn in 1990i64..=2070 {
        for bg in renten {
            fall("p19_2_versorgungsfreibetrag", adapter::p19_2_versorgungsfreibetrag,
                json!({"versorgung_bemessungsgrundlage": bg, "versorgung_beginn_jahr": beginn}));
        }
    }
    eprintln!("kohorten_exhaustiv: {} Faelle, {} Abweichungen, {} Fehlerfaelle", z.faelle, z.abweichungen, z.fehlerfaelle);
    for b in &z.beispiele {
        eprintln!("    {b}");
    }
    assert_eq!(z.abweichungen, 0);
}

/// Wirksamkeit: 20 echte `solz`-Faelle, einer davon um 1 Cent verschoben -> genau 1 Abweichung.
#[test]
fn negativkontrolle() {
    if !aktiv() {
        return;
    }
    let mut z = Zaehler::default();
    for (i, r) in korpus("solz").iter().filter(|r| r.get("ok").is_some()).take(20).enumerate() {
        let args = args_von(r);
        let mut ergebnis = rust(adapter::solz, &args);
        if i == 10 {
            if let Ausgang::Ok(v) = &ergebnis {
                ergebnis = Ausgang::Ok(json!(v.as_i64().unwrap() + 1));
            }
        }
        z.pruefe(&args, &ergebnis, &live("solz", &args), "negativkontrolle");
    }
    eprintln!("negativkontrolle: {} Faelle, {} Abweichungen (erwartet 1)", z.faelle, z.abweichungen);
    assert_eq!(z.faelle, 20);
    assert_eq!(z.abweichungen, 1);
}
