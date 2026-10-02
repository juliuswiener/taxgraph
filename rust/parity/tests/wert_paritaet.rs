//! `domain::PyWert::py_eq` gegen ECHTES `CPython` (`tools/parity/wert_oracle.py`, Praefix `wert.`).
//!
//! `py_wert.rs` belegt `py_eq` (D13, D14) mit Beispieltests: Konstanten, einmal von Hand an
//! `CPython` gemessen. Diese Suite fragt `CPython` bei jedem Lauf selbst:
//!
//! - `anker`: Paare mit bekannter Antwort, auf BEIDEN Seiten geprueft. Sie belegen, dass das Orakel
//!   nicht nur "falsch" sagt, und halten die Randwerte der D14-Schranke fest (2^63, 2^64 - 2048,
//!   1,8e19, `u64::MAX`).
//! - `py_eq_gegen_cpython`: das Kreuzprodukt aus Randwerten (i64/u64-Rand, 2^53 +- 1, -0.0, NaN,
//!   inf, bool/int/float, Text) und Behaeltern, dazu jedes Skalarpaar in `[x]`, `[[x]]` und
//!   `{"k": x}`. Jedes Paar beantwortet `a == b` in `CPython`; `py_eq` muss dasselbe sagen.
//! - `py_eq_json_gegen_cpython`: dieselbe Frage fuer JSON-Texte durch die Lader (`serde_json` gegen
//!   `json.loads`). Die D1-Ausnahme (Ganzzahl ausserhalb `i64::MIN..=u64::MAX` wird `Gleit`) ist
//!   als Liste festgehalten und muss genau so abweichen.
//!
//! Nicht gedeckt: die Identitaetsabkuerzung von `list.__eq__` (`[nan] == [nan]` bei GEMEINSAMEM
//! NaN-Objekt, ponytail an `PyWert::py_eq`); das Orakel baut jedes Objekt neu.
//!
//! Braucht `python3` mit dem Repo-Umfeld (`oracle.py` importiert die Catala-Pakete eager) -- in CI
//! standardmaessig SKIP, lokal erzwingen:
//!
//!   `PARITY`=1 `cargo` test -p parity --test `wert_paritaet` -- --nocapture
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::{Mutex, OnceLock, PoisonError};

use domain::PyWert;
use domain::PyWert::{Bool, Ganz, Gleit, GrossGanz, Null, Text};
use parity::Oracle;
use serde_json::{json, Value};

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn oracle() -> &'static Mutex<Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
}

/// Die Antworten des Orakels, eine je Paar (`bool`, oder `{"err": ..}` wenn ein JSON-Text nicht laedt).
fn cpython(funktion: &str, paare: &[Value]) -> Vec<Value> {
    let a = oracle()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .call_json(&json!({ "fn": funktion, "paare": paare }))
        .expect("orakel antwortet");
    match a.get("ok") {
        Some(Value::Array(antworten)) => antworten.clone(),
        _ => panic!("Orakel-Fehler: {a}"),
    }
}

/// `PyWert` im Draht-Format von `wert_oracle.py`: Zahlen reisen als Dezimaltext und IEEE-Bits,
/// nie als JSON-Zahl (NaN, -0.0 und u64 waeren dort verloren oder gerundet).
fn draht(w: &PyWert) -> Value {
    match w {
        Null => Value::Null,
        Bool(b) => json!(b),
        Ganz(n) => json!({ "i": n.to_string() }),
        GrossGanz(n) => json!({ "i": n.to_string() }),
        Gleit(f) => json!({ "f": format!("{:016x}", f.to_bits()) }),
        Text(t) => json!(t),
        PyWert::Liste(l) => Value::Array(l.iter().map(draht).collect()),
        PyWert::Objekt(o) => {
            json!({ "o": o.iter().map(|(k, v)| json!([k, draht(v)])).collect::<Vec<_>>() })
        }
    }
}

fn liste(v: Vec<PyWert>) -> PyWert {
    PyWert::Liste(v)
}

fn objekt(v: Vec<(&str, PyWert)>) -> PyWert {
    PyWert::Objekt(v.into_iter().map(|(k, w)| (k.to_owned(), w)).collect())
}

const ZWEI63: f64 = 9_223_372_036_854_775_808.0;
const ZWEI64: f64 = 18_446_744_073_709_551_616.0;
const ZWEI53: f64 = 9_007_199_254_740_992.0;
/// Das ganzzahlige Float 1,8e19: die alte Schranke von `ganz_gleich_gleit` (`f.abs() < 1.8e19`).
const ACHTZEHN_E18: u64 = 18_000_000_000_000_000_000;

/// Die Randwerte: `i64`/`u64`-Rand, die Floats rund um 2^63 und 2^64 (dort ist der Abstand 1024
/// bzw. 2048 bzw. 4096), 2^53 +- 1, -0.0, NaN, inf, `bool`, `None`, Text.
fn skalare() -> Vec<PyWert> {
    vec![
        Null,
        Bool(false),
        Bool(true),
        Ganz(0),
        Ganz(1),
        Ganz(-1),
        Ganz(2),
        Ganz((1 << 53) - 1),
        Ganz(1 << 53),
        Ganz((1 << 53) + 1),
        Ganz(-(1 << 53) - 1),
        Ganz(i64::MIN),
        Ganz(i64::MIN + 1),
        Ganz(i64::MAX - 1),
        Ganz(i64::MAX),
        GrossGanz(1 << 63),
        GrossGanz((1 << 63) + 1),
        GrossGanz((1 << 63) + 2048),
        GrossGanz(ACHTZEHN_E18),
        GrossGanz(u64::MAX - 2047),
        GrossGanz(u64::MAX - 1),
        GrossGanz(u64::MAX),
        Gleit(0.0),
        Gleit(-0.0),
        Gleit(1.0),
        Gleit(-1.0),
        Gleit(0.5),
        Gleit(1.5),
        Gleit(2.0),
        Gleit(ZWEI53 - 1.0),
        Gleit(ZWEI53),
        Gleit(ZWEI53 + 2.0),
        Gleit(ZWEI63),
        Gleit(-ZWEI63),
        Gleit(ZWEI63 - 1024.0),
        Gleit(ZWEI63 + 2048.0),
        Gleit(1.8e19),
        Gleit(ZWEI64 - 2048.0),
        Gleit(ZWEI64),
        Gleit(ZWEI64 + 4096.0),
        Gleit(1e30),
        Gleit(-1e30),
        Gleit(f64::MAX),
        Gleit(5e-324),
        Gleit(f64::NAN),
        Gleit(f64::INFINITY),
        Gleit(f64::NEG_INFINITY),
        Text(String::new()),
        Text("1".to_owned()),
        Text("a".to_owned()),
    ]
}

/// `list` und `dict` mit gleicher und verschiedener Laenge, Reihenfolge, Schluessel, Zahlenart.
fn behaelter() -> Vec<PyWert> {
    vec![
        liste(vec![]),
        liste(vec![Ganz(1)]),
        liste(vec![Gleit(1.0)]),
        liste(vec![Bool(true)]),
        liste(vec![Ganz(1), Ganz(2)]),
        liste(vec![Ganz(2), Ganz(1)]),
        liste(vec![liste(vec![Ganz(1)])]),
        liste(vec![Null]),
        liste(vec![Text("1".to_owned())]),
        liste(vec![GrossGanz(ACHTZEHN_E18)]),
        liste(vec![Gleit(1.8e19)]),
        liste(vec![Gleit(f64::NAN)]),
        objekt(vec![]),
        objekt(vec![("a", Ganz(1))]),
        objekt(vec![("a", Gleit(1.0))]),
        objekt(vec![("a", Bool(true))]),
        objekt(vec![("b", Ganz(1))]),
        objekt(vec![("a", Ganz(1)), ("b", Ganz(2))]),
        objekt(vec![("b", Ganz(2)), ("a", Ganz(1))]),
        objekt(vec![("a", Ganz(1)), ("b", Ganz(3))]),
        objekt(vec![("a", GrossGanz(ACHTZEHN_E18))]),
        objekt(vec![("a", Gleit(1.8e19))]),
        objekt(vec![("a", liste(vec![Ganz(1)]))]),
        objekt(vec![("a", Text("1".to_owned()))]),
    ]
}

/// Ein Skalar in `[x]`, `[[x]]` oder `{"k": x}`.
fn huelle(w: &PyWert, art: usize) -> PyWert {
    match art {
        0 => liste(vec![w.clone()]),
        1 => liste(vec![liste(vec![w.clone()])]),
        _ => objekt(vec![("k", w.clone())]),
    }
}

/// Fragt `CPython` nach `paare` und haelt `py_eq` daneben; liefert die Abweichungen
/// (Beschreibung, Rust, `CPython`) und die Zahl der Paare, die `CPython` wahr nennt.
fn vergleiche(paare: &[(PyWert, PyWert)]) -> (Vec<String>, usize) {
    let wire: Vec<Value> = paare
        .iter()
        .map(|(a, b)| json!([draht(a), draht(b)]))
        .collect();
    let py = cpython("wert.py_eq", &wire);
    assert_eq!(py.len(), paare.len());
    let mut abw = Vec::new();
    let mut wahr = 0;
    for ((a, b), p) in paare.iter().zip(&py) {
        let p = p.as_bool().unwrap_or_else(|| panic!("keine Antwort: {p}"));
        wahr += usize::from(p);
        let rust = a.py_eq(b);
        if rust != p {
            abw.push(format!("{a:?} == {b:?}: Rust {rust}, CPython {p}"));
        }
    }
    (abw, wahr)
}

/// Paare mit bekannter Antwort. Sie stehen hier, weil das Orakel sonst auch dann gruen bliebe, wenn
/// es immer `false` sagte (und `py_eq` dazu).
#[test]
fn anker() {
    if skip() {
        return;
    }
    let anker: Vec<(PyWert, PyWert, bool)> = vec![
        // D14 ab 1,8e19: eine Ganzzahl und das Float, das sie exakt trifft, sind gleich.
        (GrossGanz(ACHTZEHN_E18), Gleit(1.8e19), true),
        (GrossGanz(u64::MAX - 2047), Gleit(ZWEI64 - 2048.0), true),
        (GrossGanz(1 << 63), Gleit(ZWEI63), true),
        (GrossGanz((1 << 63) + 2048), Gleit(ZWEI63 + 2048.0), true),
        (Ganz(i64::MIN), Gleit(-ZWEI63), true),
        (Ganz(0), Gleit(-0.0), true),
        (Bool(true), Gleit(1.0), true),
        (Bool(false), Gleit(-0.0), true),
        (
            liste(vec![GrossGanz(ACHTZEHN_E18)]),
            liste(vec![Gleit(1.8e19)]),
            true,
        ),
        (
            objekt(vec![("k", GrossGanz(ACHTZEHN_E18))]),
            objekt(vec![("k", Gleit(1.8e19))]),
            true,
        ),
        (
            objekt(vec![("a", Ganz(1)), ("b", Ganz(2))]),
            objekt(vec![("b", Gleit(2.0)), ("a", Bool(true))]),
            true,
        ),
        // ... und was nicht gleich ist, obwohl `as f64` es zusammenfiele.
        (GrossGanz(u64::MAX), Gleit(ZWEI64), false),
        (Ganz(i64::MAX), Gleit(ZWEI63), false),
        (Ganz((1 << 53) + 1), Gleit(ZWEI53), false),
        (Ganz(1), Gleit(1.5), false),
        (Gleit(f64::NAN), Gleit(f64::NAN), false),
        (Gleit(f64::INFINITY), Ganz(i64::MAX), false),
        (Text("1".to_owned()), Ganz(1), false),
        (Null, Ganz(0), false),
        (liste(vec![Ganz(1)]), liste(vec![Ganz(1), Ganz(2)]), false),
        (
            objekt(vec![("a", Ganz(1))]),
            objekt(vec![("a", Ganz(1)), ("b", Ganz(2))]),
            false,
        ),
    ];
    let paare: Vec<(PyWert, PyWert)> = anker
        .iter()
        .map(|(a, b, _)| (a.clone(), b.clone()))
        .collect();
    let wire: Vec<Value> = paare
        .iter()
        .map(|(a, b)| json!([draht(a), draht(b)]))
        .collect();
    let py = cpython("wert.py_eq", &wire);
    for ((a, b, erwartet), p) in anker.iter().zip(&py) {
        assert_eq!(p, &json!(erwartet), "CPython: {a:?} == {b:?}");
        assert_eq!(a.py_eq(b), *erwartet, "Rust: {a:?} == {b:?}");
    }
}

#[test]
fn py_eq_gegen_cpython() {
    if skip() {
        return;
    }
    let skalare = skalare();
    let mut alle = skalare.clone();
    alle.extend(behaelter());
    let mut paare = Vec::new();
    for a in &alle {
        for b in &alle {
            paare.push((a.clone(), b.clone()));
        }
    }
    for art in 0..3 {
        for a in &skalare {
            for b in &skalare {
                paare.push((huelle(a, art), huelle(b, art)));
            }
        }
    }
    let (abw, wahr) = vergleiche(&paare);
    println!(
        "py_eq: {} Paare, CPython nennt {wahr} gleich, {} Abweichungen",
        paare.len(),
        abw.len()
    );
    assert!(
        paare.len() > 10_000 && wahr > 300,
        "{} Paare, {wahr} gleich",
        paare.len()
    );
    assert!(
        abw.is_empty(),
        "{} Abweichungen, erste: {:#?}",
        abw.len(),
        &abw[..abw.len().min(20)]
    );
}

/// JSON-Texte, die beide Lader gleich laden (`serde_json` gegen `json.loads`).
const JSON_TEXTE: [&str; 56] = [
    "null",
    "true",
    "false",
    "0",
    "-0",
    "1",
    "-1",
    "2",
    "0.0",
    "-0.0",
    "1.0",
    "-1.0",
    "0.5",
    "1.5",
    "9007199254740991",
    "9007199254740992",
    "9007199254740993",
    "9007199254740992.0",
    "9007199254740994.0",
    "9223372036854775807",
    "9223372036854775808",
    "9223372036854775808.0",
    "9223372036854777856",
    "9223372036854777856.0",
    "-9223372036854775808",
    "-9223372036854775808.0",
    "18000000000000000000",
    "1.8e19",
    "1.8E19",
    "1.8e+19",
    "18446744073709549568",
    "18446744073709549568.0",
    "18446744073709551614",
    "18446744073709551615",
    "1.8446744073709552e19",
    "1e19",
    "1e30",
    "\"1\"",
    "\"\"",
    "[]",
    "{}",
    "[1]",
    "[1.0]",
    "[true]",
    "[18000000000000000000]",
    "[1.8e19]",
    "[[1]]",
    "{\"a\":1}",
    "{\"a\":1.0}",
    "{\"a\":true}",
    "{\"a\":1,\"b\":2}",
    "{\"b\":2,\"a\":1}",
    "{\"a\":18000000000000000000}",
    "{\"a\":1.8e19}",
    "{\"a\":1,\"a\":2}",
    "{\"a\":2}",
];

/// D1: `serde_json` rundet diese Ganzzahlen auf 53 Bit (`Gleit`), `CPython` haelt sie exakt.
const D1_TEXTE: [&str; 3] = [
    "18446744073709551616",
    "18446744073709551617",
    "-9223372036854775809",
];

/// Die Paare (ungeordnet), bei denen D1 `py_eq` kippt: `CPython` falsch, Rust wahr. Jedes andere
/// Paar mit einem D1-Text stimmt mit `CPython` ueberein, z. B. `2^64` gegen `1.8446744073709552e19`.
const D1_ABWEICHUNGEN: [(&str, &str); 4] = [
    ("18446744073709551616", "18446744073709551617"),
    ("18446744073709551617", "1.8446744073709552e19"),
    ("-9223372036854775809", "-9223372036854775808"),
    ("-9223372036854775809", "-9223372036854775808.0"),
];

#[test]
fn py_eq_json_gegen_cpython() {
    if skip() {
        return;
    }
    let texte: Vec<&str> = JSON_TEXTE.iter().chain(&D1_TEXTE).copied().collect();
    let mut paare = Vec::new();
    for a in &texte {
        for b in &texte {
            paare.push((*a, *b));
        }
    }
    let wire: Vec<Value> = paare.iter().map(|(a, b)| json!([a, b])).collect();
    let py = cpython("wert.py_eq_json", &wire);
    assert_eq!(py.len(), paare.len());
    let laden = |t: &str| serde_json::from_str::<PyWert>(t).unwrap_or_else(|e| panic!("{t}: {e}"));
    let mut unerwartet = Vec::new();
    let mut d1 = Vec::new();
    let mut wahr = 0;
    for ((a, b), p) in paare.iter().zip(&py) {
        let p = p
            .as_bool()
            .unwrap_or_else(|| panic!("{a} == {b}: keine Antwort: {p}"));
        wahr += usize::from(p);
        let rust = laden(a).py_eq(&laden(b));
        if rust == p {
            continue;
        }
        let in_d1 = D1_TEXTE.contains(a) || D1_TEXTE.contains(b);
        if in_d1 && !p && rust {
            d1.push((*a, *b));
        } else {
            unerwartet.push(format!("{a} == {b}: Rust {rust}, CPython {p}"));
        }
    }
    println!(
        "py_eq_json: {} Paare, CPython nennt {wahr} gleich, D1-Abweichungen {}",
        paare.len(),
        d1.len()
    );
    assert!(wahr > 100, "{wahr} gleich");
    assert!(
        unerwartet.is_empty(),
        "{} unerwartet: {:#?}",
        unerwartet.len(),
        unerwartet
    );
    // Das Kreuzprodukt liefert jede Abweichung in beiden Reihenfolgen, die Liste nennt sie einmal.
    let mut gesehen: Vec<_> = d1.iter().map(|(a, b)| ungeordnet(a, b)).collect();
    gesehen.sort_unstable();
    gesehen.dedup();
    let mut erwartet: Vec<_> = D1_ABWEICHUNGEN
        .iter()
        .map(|(a, b)| ungeordnet(a, b))
        .collect();
    erwartet.sort_unstable();
    assert_eq!(gesehen, erwartet);
}

fn ungeordnet<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}
