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
//! `int(x)` (`PyWert::int`, `int_dezimal`, ueber `Text` auch `int_aus_text`):
//! - `int_gegen_cpython`: 4.279 Werte (Skalare, Behaelter, 2.106 Floats von 5e-324 bis `f64::MAX`,
//!   2.099 Texte mit Vorzeichen, Leerraum, `_`, Ziffern anderer Schriften, der 4300-Ziffern-Grenze).
//!   Wert, Klasse und `str(e)` muessen stimmen. `int` meldet ueber `i64` hinaus die Rust-Grenze
//!   (D3, D4), wo `CPython` exakt weiterrechnet. Festgehaltene Abweichung: die Meldung von
//!   `int(text)` traegt ein `repr`, das Cf/Co/Cn nicht escapet (ponytail an `repr_str`).
//! - `int_text_sweep_gegen_cpython`: `int(c + "7" + c)` fuer jeden der 1.112.064 Skalarwerte.
//!
//! Die Typfragen (`truthy`, `typname`, `gt_null`, `int_mit_bool`, `int_ohne_bool`,
//! `zahl_ohne_bool`, `oder_null`, `PyFehler::python_klasse`): je ein Test, alle auf denselben
//! 4.291 Werten (die von `int` und zwoelf Zusatzwerte). Antwort, Klasse und `str(e)` (der
//! `TypeError` von `x > 0`) muessen stimmen; `int_mit_bool` und `int_ohne_bool` melden wie `int`
//! die Rust-Grenze ueber `i64`.
//!
//! `repr(x)` und `str(x)` (`PyWert::repr`, `py_str`, darunter `domain::repr_float` und `repr_str`):
//! - `repr_float_gegen_cpython`: zwoelf Anker mit festgehaltenem Text (`1e16`, `1e-5`, `5e-324`,
//!   `f64::MAX`, `-0.0`, die Gleichstaende `-1409149049912713.25`, `562949953421312.25`) auf beiden
//!   Seiten, dann 10.086 Gleitkommazahlen: jede Zweierpotenz, kleine ungerade Vielfache davon, die
//!   Kanten bei `1e16` und `1e-4`, zufaellige Bitmuster, Subnormale. Darunter 121 exakte
//!   Gleichstaende, die `{:e}` von `std` anders schreibt.
//! - `repr_und_py_str_gegen_cpython`: dieselben Gleitkommazahlen, jede Folge bis Laenge 4 aus `'`, `"`,
//!   `\`, `a`, Zeilenumbruch, und 6.352 Behaelter (jedes Paar aus Skalaren als Liste und als
//!   `dict`) -- 17.243 Werte, `repr` und `str` je einmal.
//! - `repr_text_sweep_gegen_cpython`: `repr(c)` fuer jeden der 1.112.064 Skalarwerte. Festgehaltene
//!   Abweichung: `repr_str` escapet Cf (153 von 170), Co (137.468) und Cn nicht, `CPython` schon
//!   (ponytail an `repr_str`). Mehr nicht: jedes andere Zeichen, das Rust aendert, stimmt mit
//!   `CPython` ueberein, und Rust escapet nie ein druckbares Zeichen.
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

use domain::PyWert::{Bool, Ganz, Gleit, GrossGanz, Null, Text};
use domain::{PyFehler, PyWert};
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

/// Eine Anfrage an das Orakel; die Antwort ist der Inhalt von `ok`.
fn frage(anfrage: &Value) -> Value {
    let a = oracle()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .call_json(anfrage)
        .expect("orakel antwortet");
    match a.get("ok") {
        Some(v) => v.clone(),
        None => panic!("Orakel-Fehler: {a}"),
    }
}

/// Die Antworten des Orakels, eine je Paar (`bool`, oder `{"err": ..}` wenn ein JSON-Text nicht laedt).
fn cpython(funktion: &str, paare: &[Value]) -> Vec<Value> {
    match frage(&json!({ "fn": funktion, "paare": paare })) {
        Value::Array(antworten) => antworten,
        andere => panic!("keine Liste: {andere}"),
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

// ---------------------------------------------------------------------------------------------
// Block B: `int(x)` -- `PyWert::int`, `PyWert::int_dezimal` und ueber `Text` auch `int_aus_text`.

/// Die Antwort auf eine Frage an `CPython` oder an `PyWert`: der Wert, oder (Klasse, `str(e)`).
/// Eine Rust-Grenze ohne Python-Klasse (`I64Grenze`, `DezimalGrenze`) traegt die Klasse `-`.
type Ant = Result<Value, (String, String)>;

fn ant_py(a: &Value) -> Ant {
    match a.get("ok") {
        Some(v) => Ok(v.clone()),
        None => Err((
            a["err"].as_str().unwrap_or("?").to_owned(),
            a["msg"].as_str().unwrap_or("?").to_owned(),
        )),
    }
}

fn ant_rust<T>(r: Result<T, PyFehler>, wert: impl FnOnce(T) -> Value) -> Ant {
    r.map(wert)
        .map_err(|e| (e.python_klasse().unwrap_or("-").to_owned(), e.to_string()))
}

/// Die Antworten von `CPython` auf `op(x)` fuer jedes `x` in `werte`.
fn fragen(funktion: &str, werte: &[PyWert]) -> Vec<Ant> {
    let wire: Vec<Value> = werte.iter().map(draht).collect();
    let antworten = frage(&json!({ "fn": funktion, "werte": wire }));
    let Value::Array(antworten) = antworten else {
        panic!("keine Liste: {antworten}");
    };
    assert_eq!(antworten.len(), werte.len());
    antworten.iter().map(ant_py).collect()
}

/// Gleich, wenn Wert oder (Klasse, Text) gleich sind. Eine Rust-Grenze (`-`) gilt als gleich, wenn
/// beide Seiten sie melden; ihr Text ist die Meldung der Rust-Grenze, nicht von `CPython`.
fn gleich(a: &Ant, b: &Ant) -> bool {
    match (a, b) {
        (Err((ka, _)), Err((kb, _))) if ka == "-" && kb == "-" => true,
        _ => a == b,
    }
}

fn klasse(a: &Ant) -> Option<&str> {
    a.as_ref().err().map(|(k, _)| k.as_str())
}

/// Zeichen, die `repr` in `CPython` escapet, `repr_str` aber stehen laesst (Cf, Co, Cn): das ponytail
/// an `repr_str`. Sie stehen im Text einer `ValueError`-Meldung von `int(text)` (`%.200R`).
const REPR_LUECKE: [char; 4] = ['\u{600}', '\u{e000}', '\u{378}', '\u{10ffff}'];

fn hat_luecke(w: &PyWert) -> bool {
    matches!(w, Text(t) if t.contains(REPR_LUECKE))
}

/// Floats fuer `int(x)`: die Kanten der Ganzzahl-Darstellung, 2^k, und zufaellige Werte (Bitmuster
/// und Mantisse mal Zehnerpotenz, damit viele davon im Bereich bis 1e24 liegen).
#[allow(clippy::cast_precision_loss)]
fn int_gleit() -> Vec<PyWert> {
    let mut v: Vec<f64> = vec![
        0.5,
        -0.5,
        1.5,
        -1.5,
        2.5,
        2.7,
        -2.7,
        0.999_999_999_999_999_9,
        -0.999_999_999_999_999_9,
        1e15,
        1e16,
        1e17,
        1e22,
        1e23,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        5e-324,
        f64::MIN_POSITIVE,
        123_456_789.987_654_32,
        -9.99e21,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
    ];
    for k in 50..70 {
        let p = 2f64.powi(k);
        v.extend([p, -p, p - p / 2f64.powi(52), p + p / 2f64.powi(52)]);
    }
    let mut z: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut naechste = || {
        z ^= z << 13;
        z ^= z >> 7;
        z ^= z << 17;
        z
    };
    for _ in 0..1000 {
        v.push(f64::from_bits(naechste()));
    }
    for _ in 0..1000 {
        let mantisse = (naechste() >> 11) as f64 / 9_007_199_254_740_992.0;
        let zehn = i32::try_from(naechste() % 27).unwrap() - 3;
        let vorzeichen = if naechste() & 1 == 0 { 1.0 } else { -1.0 };
        v.push(vorzeichen * mantisse * 10f64.powi(zehn));
    }
    v.into_iter().map(Gleit).collect()
}

/// Texte fuer `int(text)`: Vorzeichen, Leerraum, Unterstriche, Ziffern anderer Schriften, die
/// Grenze von 4300 Ziffern und Zeichen, die `repr` escapet.
fn int_texte() -> Vec<PyWert> {
    let vor = [
        "",
        " ",
        "+",
        "-",
        "+-",
        " +",
        "\u{3000}-",
        "\t- ",
        "\u{1c}",
        "\u{85}",
    ];
    let rumpf = [
        "0",
        "7",
        "007",
        "-",
        "1_0",
        "1__0",
        "_1",
        "1_",
        "0_0",
        "12",
        "\u{663}",
        "1\u{663}",
        "\u{ff11}\u{ff12}",
        "\u{b2}",
        "0x10",
        "1.0",
        "1e3",
        "",
        "9223372036854775807",
        "9223372036854775808",
        "18446744073709551615",
        "18446744073709551616",
        "1 1",
    ];
    let nach = ["", " ", "\n", "\u{1c}", "\u{a0}", "L", ".", "j", "\u{200b}"];
    let mut v = Vec::new();
    for a in vor {
        for b in rumpf {
            for c in nach {
                v.push(format!("{a}{b}{c}"));
            }
        }
    }
    // Die Grenze zaehlt Ziffern samt fuehrender Nullen, ohne `_` und Leerraum, erst nach der Syntax.
    v.extend([
        "1".repeat(4300),
        "1".repeat(4301),
        "-".to_owned() + &"1".repeat(4301),
        "0".repeat(4301),
        format!(" {} ", "1".repeat(4301)),
        format!("{}1", "1_".repeat(2150)),
        format!("{}1", "1_".repeat(2151)),
        format!("{}x", "1".repeat(4301)),
        "x".repeat(300),
        "9".repeat(300) + "x",
    ]);
    // Zeichen, die `repr` anders schreibt: Anfuehrungszeichen, Backslash, Steuerzeichen, Emoji.
    v.extend(
        [
            "'",
            "\"",
            "'\"",
            "a'b\"c",
            "\\",
            "\t",
            "\r\n",
            "\0",
            "\u{7f}",
            "\u{ad}",
            "\u{200b}",
            "\u{2028}",
            "\u{1f600}",
            "\u{e9}",
            "\u{600}",
            "\u{e000}",
            "\u{378}",
            "7\u{600}",
            "\u{10ffff}",
        ]
        .map(str::to_owned),
    );
    v.into_iter().map(Text).collect()
}

fn int_werte() -> Vec<PyWert> {
    let mut v = skalare();
    v.extend(behaelter());
    v.extend(int_gleit());
    v.extend(int_texte());
    v
}

/// `int_dezimal` und `int` gegen `int(x)` in `CPython`. `int_dezimal` muss Wert, Klasse und Text
/// treffen. `int` auch, ausser dass es ueber `i64` hinaus die Rust-Grenze meldet (D3, D4), wo
/// `CPython` exakt weiterrechnet. Eine Abweichung im Text hat nur ein Grund: `repr_str` escapet
/// Cf/Co/Cn nicht (ponytail), und das steht in der Meldung von `int(text)`.
#[test]
fn int_gegen_cpython() {
    if skip() {
        return;
    }
    let werte = int_werte();
    let py = fragen("wert.int", &werte);
    let (mut unerwartet, mut ueber_i64) = (Vec::new(), 0);
    let mut luecke = std::collections::BTreeSet::new();
    let mut je_klasse = std::collections::BTreeMap::<String, usize>::new();
    for (w, p) in werte.iter().zip(&py) {
        *je_klasse
            .entry(klasse(p).unwrap_or("ok").to_owned())
            .or_default() += 1;
        let soll_int = mit_rust_grenze(p);
        ueber_i64 += usize::from(&soll_int != p);
        let dezimal = ant_rust(w.int_dezimal(), Value::String);
        let ganz = ant_rust(w.int(), |n| Value::String(n.to_string()));
        for (was, ist, soll) in [("int_dezimal", &dezimal, p), ("int", &ganz, &soll_int)] {
            if gleich(ist, soll) {
                continue;
            }
            if hat_luecke(w) && klasse(ist) == klasse(soll) {
                if let Text(t) = w {
                    luecke.insert(t.clone());
                }
                continue;
            }
            unerwartet.push(format!("{was}: {w:?}: Rust {ist:?}, CPython {soll:?}"));
        }
    }
    println!(
        "int: {} Werte, davon ueber i64 {ueber_i64}, je Klasse {je_klasse:?}, Meldungsluecke {} Texte",
        werte.len(),
        luecke.len()
    );
    assert!(
        unerwartet.is_empty(),
        "{} unerwartet, erste: {:#?}",
        unerwartet.len(),
        &unerwartet[..unerwartet.len().min(20)]
    );
    // Die Luecke ist genau die der Texte mit einem Zeichen aus `REPR_LUECKE`: jeder weicht ab.
    let erwartet: std::collections::BTreeSet<String> = werte
        .iter()
        .filter(|w| hat_luecke(w))
        .filter_map(|w| match w {
            Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(luecke, erwartet);
    // Das Orakel sagt nicht nur "ok" oder nur "Fehler": jede Klasse kommt vor.
    for (klasse, mindestens) in [
        ("ok", 1000),
        ("TypeError", 20),
        ("ValueError", 1000),
        ("OverflowError", 2),
    ] {
        assert!(
            je_klasse.get(klasse).copied().unwrap_or(0) >= mindestens,
            "{je_klasse:?}"
        );
    }
    assert!(ueber_i64 > 100, "{ueber_i64}");
}

/// `int(c + "7" + c)` fuer jeden Skalarwert `c` (1.112.064 Texte): welche `CPython` annimmt, mit
/// welchem Wert, und der Text der Meldung bei jedem druckbaren `c`. Das Orakel schickt keine 1,1
/// Mio. Antworten, sondern die Annahmen, die Bereiche der nicht druckbaren `c` und die wenigen
/// Meldungen, die von der Vorlage abweichen (`'` und `\`).
///
/// Unicode 16.0 (`CPython` 3.14): Rust und `CPython` nehmen dieselben Texte an. Aelterer Stand (3.12,
/// Unicode 15.0): Rust nimmt genau die 80 Ziffern der acht Bloecke an, die dort noch fehlen (D6).
#[test]
fn int_text_sweep_gegen_cpython() {
    if skip() {
        return;
    }
    let a = frage(&json!({ "fn": "wert.int_text_sweep" }));
    let zahl = |v: &Value| u32::try_from(v.as_u64().unwrap()).unwrap();
    let ok: std::collections::BTreeMap<u32, String> = a["ok"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (zahl(&e[0]), e[1].as_str().unwrap().to_owned()))
        .collect();
    let mut druckbar = vec![true; 0x11_0000];
    for bereich in a["nicht_druckbar"].as_array().unwrap() {
        for cp in zahl(&bereich[0])..=zahl(&bereich[1]) {
            druckbar[usize::try_from(cp).unwrap()] = false;
        }
    }
    let ausnahmen: std::collections::BTreeMap<u32, String> = a["vorlage_ausnahmen"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (zahl(&e[0]), e[1].as_str().unwrap().to_owned()))
        .collect();
    assert_eq!(
        a["klassen"].as_object().unwrap().keys().collect::<Vec<_>>(),
        ["ValueError"]
    );
    let unicode = a["unicode"].as_str().unwrap();

    let (mut abweichend, mut zu_viel, mut druckbare_fehler) = (Vec::new(), Vec::new(), 0_usize);
    for c in (0..=0x10_ffff_u32).filter_map(char::from_u32) {
        let cp = u32::from(c);
        let rust = PyWert::Text(format!("{c}7{c}")).int_dezimal();
        match (ok.get(&cp), rust) {
            (Some(soll), Ok(ist)) if &ist == soll => {}
            (None, Ok(_)) => zu_viel.push(cp),
            (None, Err(PyFehler::WertFehler(m))) => {
                let vorlage = format!("invalid literal for int() with base 10: '{c}7{c}'");
                let soll = ausnahmen.get(&cp).unwrap_or(&vorlage);
                if druckbar[usize::try_from(cp).unwrap()] {
                    druckbare_fehler += 1;
                    if &m != soll {
                        abweichend.push(format!("U+{cp:04X}: Rust {m:?}, CPython {soll:?}"));
                    }
                } else if !m.starts_with("invalid literal for int() with base 10: ") {
                    abweichend.push(format!("U+{cp:04X}: Rust {m:?}"));
                }
            }
            (soll, ist) => abweichend.push(format!("U+{cp:04X}: Rust {ist:?}, CPython {soll:?}")),
        }
    }
    println!(
        "int-Sweep: Unicode {unicode}, {} angenommen, {druckbare_fehler} Meldungen druckbarer Zeichen, \
         Rust nimmt zusaetzlich {} an",
        ok.len(),
        zu_viel.len()
    );
    assert!(
        abweichend.is_empty(),
        "{} Abweichungen, erste: {:#?}",
        abweichend.len(),
        &abweichend[..abweichend.len().min(20)]
    );
    assert!(ok.len() > 700 && druckbare_fehler > 100_000);
    if unicode.starts_with("16.") {
        assert!(zu_viel.is_empty(), "{zu_viel:?}");
    } else {
        assert_eq!(zu_viel.len(), 80, "{zu_viel:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// Block C: die Pruefungen und Typfragen auf `PyWert` -- `truthy`, `typname`, `gt_null`,
// `int_mit_bool`, `int_ohne_bool`, `zahl_ohne_bool`, `oder_null` und `PyFehler::python_klasse`.

/// Zusaetzliche Werte fuer die Typfragen: Texte, die wie Zahlen oder Wahrheitswerte aussehen, und
/// Behaelter, die leer aussehen.
fn typ_extra() -> Vec<PyWert> {
    let text = |t: &str| Text(t.to_owned());
    vec![
        text(" "),
        text("0"),
        text("False"),
        text("None"),
        text("0.0"),
        Gleit(1e-300),
        Gleit(-5e-324),
        Gleit(-f64::MIN_POSITIVE),
        liste(vec![liste(vec![])]),
        liste(vec![Null, Null]),
        objekt(vec![("", Null)]),
        objekt(vec![("a", liste(vec![]))]),
    ]
}

/// Alle Werte der Typfragen: die Werte von `int` (Skalare, Behaelter, Floats, Texte) und `typ_extra`.
fn typ_werte() -> Vec<PyWert> {
    let mut v = int_werte();
    v.extend(typ_extra());
    v
}

/// `CPython`s Antwort, wenn `int` oder `int_mit_bool` eine Ganzzahl ausserhalb von `i64` meldet:
/// die Rust-Grenze (D3, D4) statt des exakten Werts.
fn mit_rust_grenze(p: &Ant) -> Ant {
    match p {
        Ok(Value::String(s)) if s.parse::<i64>().is_err() => Err(("-".to_owned(), String::new())),
        _ => p.clone(),
    }
}

/// Fragt `CPython` nach `op(x)` fuer jedes `x` in `werte` und haelt `rust(x)` daneben. Gibt die
/// Abweichungen und die Zahl der Antworten je Art (`ok` oder Klasse) zurueck. `ganz`: die Antwort
/// ist eine Ganzzahl als Dezimaltext, und ausserhalb von `i64` gilt die Rust-Grenze.
fn gegen_cpython(
    op: &str,
    ganz: bool,
    werte: &[PyWert],
    rust: impl Fn(&PyWert) -> Ant,
) -> (Vec<String>, std::collections::BTreeMap<String, usize>) {
    let py = fragen(op, werte);
    let mut abw = Vec::new();
    let mut je_art = std::collections::BTreeMap::new();
    for (w, p) in werte.iter().zip(&py) {
        *je_art
            .entry(klasse(p).unwrap_or("ok").to_owned())
            .or_default() += 1;
        let soll = if ganz { mit_rust_grenze(p) } else { p.clone() };
        let ist = rust(w);
        if !gleich(&ist, &soll) {
            abw.push(format!("{w:?}: Rust {ist:?}, CPython {soll:?}"));
        }
    }
    (abw, je_art)
}

fn pruefe_op(
    op: &str,
    ganz: bool,
    rust: impl Fn(&PyWert) -> Ant,
    erwartete_arten: &[(&str, usize)],
) {
    if skip() {
        return;
    }
    let werte = typ_werte();
    let (abw, je_art) = gegen_cpython(op, ganz, &werte, rust);
    println!(
        "{op}: {} Werte, Antworten {je_art:?}, {} Abweichungen",
        werte.len(),
        abw.len()
    );
    assert!(
        abw.is_empty(),
        "{op}: {} Abweichungen, erste: {:#?}",
        abw.len(),
        &abw[..abw.len().min(20)]
    );
    // Das Orakel sagt nicht nur eine Art von Antwort (sonst waere "alles gleich" wertlos).
    for (art, mindestens) in erwartete_arten {
        assert!(
            je_art.get(*art).copied().unwrap_or(0) >= *mindestens,
            "{op}: {je_art:?}"
        );
    }
}

#[test]
fn truthy_gegen_cpython() {
    let wahr = |v: bool| Ok(Value::Bool(v));
    pruefe_op("wert.truthy", false, |w| wahr(w.truthy()), &[("ok", 1000)]);
    if skip() {
        return;
    }
    // Beide Antworten kommen vor.
    let werte = typ_werte();
    let py = fragen("wert.truthy", &werte);
    assert!(py.contains(&Ok(Value::Bool(true))) && py.contains(&Ok(Value::Bool(false))));
}

#[test]
fn typname_gegen_cpython() {
    pruefe_op(
        "wert.typname",
        false,
        |w| Ok(Value::String(w.typname().to_owned())),
        &[("ok", 1000)],
    );
    if skip() {
        return;
    }
    // Alle sieben Typnamen kommen vor.
    let py = fragen("wert.typname", &typ_werte());
    for name in ["NoneType", "bool", "int", "float", "str", "list", "dict"] {
        assert!(py.contains(&Ok(Value::String(name.to_owned()))), "{name}");
    }
}

/// `x > 0`: der Wahrheitswert, oder der `TypeError` mit dem Wortlaut von `CPython`.
#[test]
fn gt_null_gegen_cpython() {
    pruefe_op(
        "wert.gt_null",
        false,
        |w| ant_rust(w.gt_null(), Value::Bool),
        &[("ok", 100), ("TypeError", 1000)],
    );
}

/// `x if isinstance(x, int) else None`: `bool` zaehlt als `int` (D10).
#[test]
fn int_mit_bool_gegen_cpython() {
    pruefe_op(
        "wert.int_mit_bool",
        true,
        |w| {
            ant_rust(w.int_mit_bool(), |o| {
                o.map_or(Value::Null, |n| Value::String(n.to_string()))
            })
        },
        &[("ok", 1000)],
    );
}

/// `x if isinstance(x, int) and not isinstance(x, bool) else None` (D10).
#[test]
fn int_ohne_bool_gegen_cpython() {
    pruefe_op(
        "wert.int_ohne_bool",
        true,
        |w| {
            ant_rust(w.int_ohne_bool(), |o| {
                o.map_or(Value::Null, |n| Value::String(n.to_string()))
            })
        },
        &[("ok", 1000)],
    );
}

/// `x if isinstance(x, (int, float)) and not isinstance(x, bool) else None`: `CPython` gibt `x`
/// selbst zurueck, `zahl_ohne_bool` einen Verweis auf `self`.
#[test]
fn zahl_ohne_bool_gegen_cpython() {
    pruefe_op(
        "wert.zahl_ohne_bool",
        false,
        |w| Ok(Value::Bool(w.zahl_ohne_bool().is_some())),
        &[("ok", 1000)],
    );
    if skip() {
        return;
    }
    for w in typ_werte() {
        if let Some(z) = w.zahl_ohne_bool() {
            assert!(std::ptr::eq(z, std::ptr::from_ref(&w)), "{w:?}");
        }
    }
}

/// `x or 0`: `x` selbst, wenn es wahr ist, sonst die Ganzzahl 0 -- im Draht-Format verglichen.
#[test]
fn oder_null_gegen_cpython() {
    pruefe_op(
        "wert.oder_null",
        false,
        |w| Ok(draht(w.oder_null())),
        &[("ok", 1000)],
    );
}

/// `PyFehler::python_klasse` gegen `type(e).__name__`: `TypeError`, `ValueError` und `OverflowError`
/// kommen von `CPython`; `I64Grenze` und `DezimalGrenze` sind Rust-Grenzen und tragen keine Klasse.
#[test]
fn python_klasse_gegen_cpython() {
    if skip() {
        return;
    }
    let werte = vec![
        Null,
        Text("x".to_owned()),
        Gleit(f64::INFINITY),
        Gleit(f64::NAN),
    ];
    let py = fragen("wert.int", &werte);
    let rust: Vec<Option<&str>> = werte
        .iter()
        .map(|w| w.int_dezimal().err().and_then(|e| e.python_klasse()))
        .collect();
    let python: Vec<String> = py
        .iter()
        .map(|p| klasse(p).unwrap_or("ok").to_owned())
        .collect();
    assert_eq!(
        python,
        ["TypeError", "ValueError", "OverflowError", "ValueError"]
    );
    assert_eq!(
        rust,
        [
            Some("TypeError"),
            Some("ValueError"),
            Some("OverflowError"),
            Some("ValueError")
        ]
    );
    // Ueber `i64` hinaus rechnet `CPython` weiter: dort gibt es keine Ausnahme und keine Klasse.
    assert_eq!(
        fragen("wert.int", &[GrossGanz(u64::MAX)])[0],
        Ok(json!("18446744073709551615"))
    );
    let grenze = GrossGanz(u64::MAX).int().unwrap_err();
    assert_eq!(grenze.python_klasse(), None);
    assert!(matches!(grenze, PyFehler::I64Grenze(_)));
}

// ---------------------------------------------------------------------------------------------
// Block A: `repr`, `py_str` und darunter `repr_float` (ueber `Gleit`) und `repr_str` (ueber `Text`).

/// 2^k als `f64`, exakt, fuer `-1074 <= k <= 1023`.
fn zwei_hoch(k: i32) -> f64 {
    if k >= -1022 {
        f64::from_bits(u64::try_from(1023 + k).unwrap() << 52)
    } else {
        f64::from_bits(1_u64 << (k + 1074))
    }
}

/// Die Gleitkommazahlen fuer `repr`: die Rand- und Gleichstandswerte aus dem Auftrag, jede
/// Zweierpotenz, kleine ungerade Vielfache davon, die Kanten der Festkomma-Schranke (`1e16`, `1e-4`)
/// und zufaellige Werte (Bitmuster, Mantisse mal Zehnerpotenz, kurze Dezimalbrueche, Subnormale).
// `excessive_precision`: der Gleichstand ist als `f64` exakt (`...713.25`), sein kuerzester Text aber
// `...713.2`. Das Literal bleibt, wie es im Auftrag steht.
#[allow(clippy::cast_precision_loss, clippy::excessive_precision)]
fn repr_gleit() -> Vec<f64> {
    let mut v: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        100.0,
        0.1,
        0.1 + 0.2,
        1.0 / 3.0,
        2.0 / 3.0,
        1e15,
        1e16,
        -1e16,
        9_999_999_999_999_998.0,
        1.5e16,
        123_456_789_012_345_680.0,
        1e22,
        1e23,
        1e-4,
        1e-5,
        0.000_099_999_999_999_999_99,
        0.001,
        5e-324,
        -5e-324,
        f64::MIN_POSITIVE,
        2.225_073_858_507_201e-308,
        f64::MAX,
        f64::MIN,
        f64::EPSILON,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        // exakte Gleichstaende zweier kuerzester Kandidaten
        -1_409_149_049_912_713.25,
        562_949_953_421_312.25,
        351_843_720_888_320.312_5,
        zwei_hoch(-25),
    ];
    for k in -1074..=1023 {
        v.push(zwei_hoch(k));
        if k % 5 == 0 {
            v.push(-zwei_hoch(k));
        }
    }
    // kleine ungerade Vielfache von 2^-k: dezimal enden sie auf 5, viele davon sind Gleichstaende
    for k in 1..=70 {
        for m in (3..32).step_by(2) {
            v.push(f64::from(m) * zwei_hoch(-k));
        }
    }
    // Zehnerpotenzen und ihre Nachbarn, die Kanten bei 1e16 und 1e-4
    for e in -8..=24 {
        let zehn: f64 = format!("1e{e}").parse().unwrap();
        for d in -20..=20_i64 {
            v.push(f64::from_bits(
                zehn.to_bits().checked_add_signed(d).unwrap(),
            ));
        }
    }
    for m in 0..40 {
        v.push(1e16 + 2.0 * f64::from(m));
        v.push(9.007_199_254_740_992e15 + f64::from(m));
    }
    let mut z: u64 = 0x2545_f491_4f6c_dd1d;
    let mut naechste = || {
        z ^= z << 13;
        z ^= z >> 7;
        z ^= z << 17;
        z
    };
    // Ganzzahl in [2^k, 2^(k+1)) plus ein Bruch in Achteln: der Abstand der Floats ist dort 1/4 bis 1/8
    for k in 46..53 {
        for _ in 0..150 {
            let n = (1_u64 << k) | (naechste() & ((1_u64 << k) - 1));
            let x = n as f64 + (naechste() % 8) as f64 / 8.0;
            v.push(if naechste() & 1 == 0 { x } else { -x });
        }
    }
    for _ in 0..1500 {
        v.push(f64::from_bits(naechste()));
    }
    for _ in 0..1000 {
        let mantisse = (naechste() >> 11) as f64 / 9_007_199_254_740_992.0;
        let zehn = i32::try_from(naechste() % 61).unwrap() - 30;
        let vorzeichen = if naechste() & 1 == 0 { 1.0 } else { -1.0 };
        v.push(vorzeichen * mantisse * 10f64.powi(zehn));
    }
    for _ in 0..1000 {
        let stellen = u32::try_from(naechste() % 15).unwrap() + 1;
        let nenner = i32::try_from(naechste() % 21).unwrap();
        v.push((naechste() % 10_u64.pow(stellen)) as f64 / 10f64.powi(nenner));
    }
    for _ in 0..500 {
        v.push(f64::from_bits(naechste() & ((1_u64 << 52) - 1)));
    }
    v
}

/// Die Ziffern einer Zahlschreibung: ohne Vorzeichen, Punkt, Exponent, fuehrende und folgende Nullen.
fn nur_ziffern(t: &str) -> String {
    let mantisse = t.trim_start_matches('-').split('e').next().unwrap_or("");
    mantisse.replace('.', "").trim_matches('0').to_owned()
}

/// Paare, deren `CPython`-Text von Hand gemessen und hier festgehalten ist (3.12.14 und 3.14.7 gleich),
/// damit das Orakel nicht nur "gleich" sagt: die Randwerte und die Gleichstandswerte aus dem Auftrag.
#[allow(clippy::excessive_precision)] // wie bei `repr_gleit`: exakter Gleichstand als Literal
const REPR_FLOAT_ANKER: [(f64, &str); 12] = [
    (1e16, "1e+16"),
    (1e15, "1000000000000000.0"),
    (9_999_999_999_999_998.0, "9999999999999998.0"),
    (1e-5, "1e-05"),
    (1e-4, "0.0001"),
    (5e-324, "5e-324"),
    (f64::MAX, "1.7976931348623157e+308"),
    (-0.0, "-0.0"),
    (0.1 + 0.2, "0.30000000000000004"),
    (-1_409_149_049_912_713.25, "-1409149049912713.2"),
    (562_949_953_421_312.25, "562949953421312.2"),
    (0.000_099_999_999_999_999_99, "9.999999999999999e-05"),
];

/// `repr(float)` gegen `CPython`: die Anker auf BEIDEN Seiten, dann der ganze Pool. Der Pool muss die
/// Gleichstaende enthalten, die `{:e}` von `std` anders schreibt (sonst belegte er den Fix nicht).
#[test]
fn repr_float_gegen_cpython() {
    if skip() {
        return;
    }
    let anker: Vec<PyWert> = REPR_FLOAT_ANKER.iter().map(|(f, _)| Gleit(*f)).collect();
    for ((f, soll), p) in REPR_FLOAT_ANKER.iter().zip(fragen("wert.repr", &anker)) {
        assert_eq!(p, Ok(json!(soll)), "CPython {f:e}");
        assert_eq!(domain::repr_float(*f), *soll, "Rust {f:e}");
    }
    // Zusaetzlich die drei Sonderwerte, deren Bits im Pool nicht als Zahl lesbar sind.
    let sonder = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    let sonder_w: Vec<PyWert> = sonder.iter().map(|f| Gleit(*f)).collect();
    assert_eq!(
        fragen("wert.repr", &sonder_w),
        [Ok(json!("nan")), Ok(json!("inf")), Ok(json!("-inf"))]
    );
    assert_eq!(
        sonder.map(domain::repr_float),
        ["nan", "inf", "-inf"].map(str::to_owned)
    );

    let pool = repr_gleit();
    let werte: Vec<PyWert> = pool.iter().map(|f| Gleit(*f)).collect();
    let py = fragen("wert.repr", &werte);
    let (mut abw, mut gleichstaende, mut exponent) = (Vec::new(), 0_usize, 0_usize);
    for (f, p) in pool.iter().zip(&py) {
        let Ok(Value::String(soll)) = p else {
            panic!("keine Antwort: {f:e}: {p:?}");
        };
        let ist = domain::repr_float(*f);
        if &ist != soll || &Gleit(*f).repr() != soll || &Gleit(*f).py_str() != soll {
            abw.push(format!(
                "{:016x} {f:e}: Rust {ist}, CPython {soll}",
                f.to_bits()
            ));
        }
        exponent += usize::from(soll.contains('e'));
        // `{:e}` rundet den exakten Gleichstand auf, `CPython` auf die gerade Ziffer.
        if f.is_finite() && nur_ziffern(&format!("{f:e}")) != nur_ziffern(soll) {
            gleichstaende += 1;
        }
    }
    println!(
        "repr_float: {} Werte, davon mit Exponent {exponent}, Gleichstaende (std anders) {gleichstaende}, \
         {} Abweichungen",
        pool.len(),
        abw.len()
    );
    assert!(
        abw.is_empty(),
        "{} Abweichungen, erste: {:#?}",
        abw.len(),
        &abw[..abw.len().min(20)]
    );
    assert!(pool.len() > 9000 && exponent > 3000 && gleichstaende > 100);
}

/// Texte fuer `repr`: jede Folge bis Laenge 4 aus `'`, `"`, `\`, `a` und Zeilenumbruch (781, damit
/// ist die Wahl der Anfuehrungszeichen erschoepft), dazu Einzelfaelle. Ohne Zeichen aus der
/// festgehaltenen Luecke (`repr_text_sweep_gegen_cpython`).
fn repr_texte() -> Vec<String> {
    let alphabet = ['\'', '"', '\\', 'a', '\n'];
    let mut v = vec![String::new()];
    let mut stufe = vec![String::new()];
    for _ in 0..4 {
        stufe = stufe
            .iter()
            .flat_map(|s| alphabet.iter().map(move |c| format!("{s}{c}")))
            .collect();
        v.extend(stufe.iter().cloned());
    }
    v.extend(
        [
            "\t",
            "\r",
            "\0",
            "\x1f",
            "\x7f",
            "\u{80}",
            "\u{9f}",
            "\u{a0}",
            "\u{ad}",
            "\u{e4}",
            "\u{20ac}",
            "\u{1f600}",
            "\u{2028}",
            "\u{2029}",
            "\u{200b}",
            "\u{feff}",
            "\u{3000}",
            "a\u{300}",
            "\\n",
            "\\x41",
            "'\"'\"",
            "it's \"quoted\"",
            "\u{1f600}'",
            "tab\there",
        ]
        .map(str::to_owned),
    );
    v
}

/// Container fuer `repr`: jeder Skalar in `[x]`, `[[x]]`, `{"k": x}`, mit Schluesseln, die Anfuehrungs-
/// zeichen tragen, dazu jedes Paar zweier Skalare als Liste und als `dict` (Trenner, Reihenfolge).
fn repr_behaelter() -> Vec<PyWert> {
    let mut klein = skalare();
    klein.extend(["'", "\"", "\\", "a'b\"c", "\n"].map(|t| Text(t.to_owned())));
    let mut v = behaelter();
    for a in &klein {
        v.extend((0..3).map(|art| huelle(a, art)));
        v.push(objekt(vec![("'", a.clone())]));
        v.push(objekt(vec![
            ("\"'", a.clone()),
            ("z", liste(vec![a.clone(), a.clone()])),
        ]));
        for b in &klein {
            v.push(liste(vec![a.clone(), b.clone()]));
            v.push(objekt(vec![("x", a.clone()), ("y", b.clone())]));
        }
    }
    v.extend([
        objekt(vec![
            ("b", Ganz(1)),
            (
                "a",
                liste(vec![Null, Bool(true), Gleit(1e16), Text("it's".to_owned())]),
            ),
        ]),
        liste(vec![liste(vec![]), objekt(vec![])]),
        objekt(vec![("", objekt(vec![("", liste(vec![]))]))]),
    ]);
    v
}

/// `repr(x)` und `str(x)` gegen `CPython` fuer Gleitkommazahlen, Texte und Container.
#[test]
fn repr_und_py_str_gegen_cpython() {
    if skip() {
        return;
    }
    let mut werte: Vec<PyWert> = repr_gleit().into_iter().map(Gleit).collect();
    werte.extend(repr_texte().into_iter().map(Text));
    werte.extend(repr_behaelter());
    let rust = |w: &PyWert, op: &str| match op {
        "wert.repr" => w.repr(),
        _ => w.py_str(),
    };
    for op in ["wert.repr", "wert.py_str"] {
        let py = fragen(op, &werte);
        let mut abw = Vec::new();
        let mut gesehen = std::collections::BTreeSet::new();
        for (w, p) in werte.iter().zip(&py) {
            let Ok(Value::String(soll)) = p else {
                panic!("{op}: keine Antwort: {w:?}: {p:?}");
            };
            let ist = rust(w, op);
            if &ist != soll {
                abw.push(format!("{w:?}: Rust {ist:?}, CPython {soll:?}"));
            }
            gesehen.insert(soll.clone());
        }
        println!(
            "{op}: {} Werte, {} verschiedene Antworten, {} Abweichungen",
            werte.len(),
            gesehen.len(),
            abw.len()
        );
        assert!(
            abw.is_empty(),
            "{op}: {} Abweichungen, erste: {:#?}",
            abw.len(),
            &abw[..abw.len().min(20)]
        );
        assert!(werte.len() > 17_000 && gesehen.len() > 15_000);
        // Das Orakel sagt nicht nur eine Art von Antwort.
        for muster in [
            "-0.0", "nan", "-inf", "1e+16", "5e-324", "None", "True", "\"'\"", "'\\\\'", "{}", "[]",
        ] {
            assert!(gesehen.iter().any(|s| s.contains(muster)), "{op}: {muster}");
        }
    }
    // `str` laesst einen Text, wie er ist; `repr` setzt ihn in Anfuehrungszeichen.
    let text = [Text("a'b".to_owned())];
    assert_eq!(fragen("wert.py_str", &text), [Ok(json!("a'b"))]);
    assert_eq!(fragen("wert.repr", &text), [Ok(json!("\"a'b\""))]);
}

/// `repr(c)` fuer jeden der 1.112.064 Skalarwerte `c`. Rust hat keine Tabelle der Kategorien: `druckbar`
/// kennt Cc, Zs/Zl/Zp und 17 Cf. Der Test legt fest, was daraus folgt:
///
/// - Jedes `c`, das Rust NICHT unveraendert laesst (Escape, Anfuehrungszeichen), geht an `CPython`
///   und muss denselben Text ergeben: Rust escapet nie ein druckbares `c` und nie anders.
/// - Jedes `c`, das Rust unveraendert laesst, ist in `CPython` druckbar, oder es gehoert zur
///   festgehaltenen Abweichung: `repr_str` escapet Cf, Co und Cn nicht (ponytail an `repr_str`).
///   Die Menge der Kategorien und die Zahlen sind festgelegt.
///
/// Folge der Abweichung: ein Text mit einem Zeichen der Luecke steht in `repr`, `py_str` von Liste und
/// `dict` und in den Fehlermeldungen von `int(text)` unescapet, `CPython` schreibt `؀`. Das ist
/// Text, kein Betrag; der Wert selbst bleibt gleich.
#[test]
fn repr_text_sweep_gegen_cpython() {
    if skip() {
        return;
    }
    let a = frage(&json!({ "fn": "wert.repr_text_sweep" }));
    let zahl = |v: &Value| u32::try_from(v.as_u64().unwrap()).unwrap();
    let unicode = a["unicode"].as_str().unwrap();
    let mut kategorie: Vec<Option<&str>> = vec![None; 0x11_0000];
    for lauf in a["nicht_druckbar"].as_array().unwrap() {
        let kat = lauf[2].as_str().unwrap();
        for cp in zahl(&lauf[0])..=zahl(&lauf[1]) {
            kategorie[usize::try_from(cp).unwrap()] = Some(kat);
        }
    }
    assert!(
        a["nicht_escaped"].as_array().unwrap().is_empty(),
        "CPython laesst ein nicht druckbares Zeichen stehen: {}",
        a["nicht_escaped"]
    );
    let ausnahmen: Vec<u32> = a["ausnahmen"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| zahl(&e[0]))
        .collect();
    assert_eq!(ausnahmen, [u32::from('\''), u32::from('\\')]);

    let (mut zu_fragen, mut luecke) = (Vec::new(), Vec::new());
    let mut je_kategorie = std::collections::BTreeMap::<&str, usize>::new();
    for c in (0..=0x10_ffff_u32).filter_map(char::from_u32) {
        let cp = u32::from(c);
        let rust = Text(c.to_string()).repr();
        if rust != format!("'{c}'") {
            zu_fragen.push(c);
        } else if let Some(kat) = kategorie[usize::try_from(cp).unwrap()] {
            *je_kategorie.entry(kat).or_default() += 1;
            luecke.push(cp);
        }
    }
    let texte: Vec<PyWert> = zu_fragen.iter().map(|c| Text(c.to_string())).collect();
    let py = fragen("wert.repr", &texte);
    let abw: Vec<String> = texte
        .iter()
        .zip(&py)
        .filter_map(|(w, p)| {
            let ist: Ant = Ok(Value::String(w.repr()));
            (ist != *p).then(|| format!("{w:?}: Rust {ist:?}, CPython {p:?}"))
        })
        .collect();
    println!(
        "repr-Sweep: Unicode {unicode}, {} Zeichen aendert Rust, Luecke je Kategorie {je_kategorie:?}, \
         {} Abweichungen",
        zu_fragen.len(),
        abw.len()
    );
    assert!(
        abw.is_empty(),
        "{} Abweichungen, erste: {:#?}",
        abw.len(),
        &abw[..abw.len().min(20)]
    );
    assert!(zu_fragen.len() > 80, "{}", zu_fragen.len());
    // Die festgehaltene Abweichung: nur Cf, Co, Cn, und genau diese Zahlen.
    assert_eq!(
        je_kategorie.keys().copied().collect::<Vec<_>>(),
        ["Cf", "Cn", "Co"]
    );
    assert_eq!(je_kategorie["Cf"], 153);
    assert_eq!(je_kategorie["Co"], 137_468);
    match unicode {
        "16.0.0" => assert_eq!(je_kategorie["Cn"], 819_533),
        "15.0.0" => assert_eq!(je_kategorie["Cn"], 825_345),
        _ => assert!(je_kategorie["Cn"] > 800_000, "{je_kategorie:?}"),
    }
    for c in REPR_LUECKE {
        assert!(luecke.contains(&u32::from(c)), "{c:?}");
    }
}
