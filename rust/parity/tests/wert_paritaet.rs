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
//! Die Lader (`PyWert::deserialize` ueber `serde_json` und `serde_yaml_ng`, gegen `json.loads` und
//! `yaml.safe_load` mit `PyYAML` 6.0.3, dem reinen Python-Lader der Produktpfade). Verglichen wird der
//! geladene Wert im Draht-Format (Art, Reihenfolge, Bits), nicht sein Text. Lehnen beide ab, gilt das
//! als gleich. Jede Abweichung steht als festgehaltener Fall (Text, Rust, `CPython`) in einer Liste; ein
//! neuer Fall und ein verschwundener lassen den Test rot werden, die gemessene Liste steht in der Meldung.
//! - `lader_json_gegen_cpython`: 134 Skalare, 80 Dokumente. 28 Abweichungen: Ganzzahl ausserhalb von
//!   `i64::MIN..=u64::MAX` (D1), `NaN`/`Infinity` und Zahlen ueber `f64::MAX` (D2), `-0` (Rust `-0.0`,
//!   Python `0`), einzelnes Surrogat in einer Zeichenkette.
//! - `lader_yaml_gegen_cpython`: 317 Skalare, 440 Dokumente. 227 Abweichungen, fast alle YAML 1.2
//!   (`serde_yaml_ng`) gegen YAML 1.1 (`PyYAML`): `yes`/`no`/`on`/`off`, Oktal `017`, Unterstriche,
//!   Sexagesimal `1:30`, Gleitkomma ohne Punkt `1e3`, Zeitstempel, Merge-Schluessel `<<`, Tags, doppelte
//!   Anker, rekursive Anker, `int`-Schluessel. Skalare stehen zusaetzlich in `[x]`, `{k: x}` und `{x: v}`.
//! - `lader_gleitkomma_gegen_cpython`: 7.227 Dezimaltexte (kuerzeste Texte, 16 bis 30 Ziffern, exakte
//!   Mittelpunkte zweier Nachbarn, Randfaelle). Beide Rust-Lader treffen jedes Bit; nur Texte ueber
//!   `f64::MAX` weichen ab (`CPython` sagt `inf`).
//! - `lader_tiefe_gegen_cpython`: Schachtelungstiefe. `CPython` nimmt 300 an, `serde_json` 127,
//!   `serde_yaml_ng` 128.
//! - `lader_echte_dateien_gegen_cpython`: alle 527 verfolgten YAML- und 163 JSON-Dateien. JSON: 0
//!   Abweichungen. YAML: 9 (`behinderten_pauschbetrag_p33b` 2024 bis 2026 und sechs Kohortentabellen),
//!   alle nur wegen `int`-Schluesseln (`{20: 384}` wird `{"20": 384}`); mit `int`-Schluesseln als Text
//!   laden alle 527 gleich.
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

/// Werte in `int_werte()` ohne die zwei Zufallsstellen von `int_gleit` (gemessen: 4.279 im Standard).
const INT_WERTE_OHNE_ZUFALL: usize = 2279;

/// Werte in `repr_gleit()` ohne die drei Zufallsstellen (gemessen: 10.086 im Standard).
const REPR_GLEIT_OHNE_ZUFALL: usize = 6586;

/// Die zwei Zufallsstellen von `int_gleit` als (Zahl, Standard); `PARITY_N` setzt beide.
fn int_gleit_stellen() -> [(usize, usize); 2] {
    [
        (
            parity::fallzahl::holen("wert_paritaet int_gleit Bitmuster", 1000),
            1000,
        ),
        (
            parity::fallzahl::holen("wert_paritaet int_gleit Mantisse", 1000),
            1000,
        ),
    ]
}

/// Floats fuer `int(x)`: die Kanten der Ganzzahl-Darstellung, 2^k, und zufaellige Werte (Bitmuster
/// und Mantisse mal Zehnerpotenz, damit viele davon im Bereich bis 1e24 liegen).
#[allow(clippy::cast_precision_loss)]
fn int_gleit() -> Vec<PyWert> {
    let [(n_bits, _), (n_mantisse, _)] = int_gleit_stellen();
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
    for _ in 0..n_bits {
        v.push(f64::from_bits(naechste()));
    }
    for _ in 0..n_mantisse {
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
    // Die Zufallsstellen liefern genau so viele Werte, wie verlangt sind (Standard 2.000, `PARITY_N`
    // je Stelle): eine Stelle, die ihre Zahl wieder im Quelltext traegt, faellt hier auf.
    assert_eq!(
        werte.len(),
        INT_WERTE_OHNE_ZUFALL + int_gleit_stellen().iter().map(|(n, _)| n).sum::<usize>()
    );
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
    // Das Orakel sagt nicht nur "ok" oder nur "Fehler": jede Klasse kommt vor. Die Mindestzahlen
    // sind an den Standard-Pool gebunden (`PARITY_N` kuerzt die zufaelligen Floats).
    if parity::fallzahl::wache_gilt_pool("wert_paritaet int_werte", &int_gleit_stellen()) {
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

/// Die drei Zufallsstellen von `repr_gleit` als (Zahl, Standard); `PARITY_N` setzt alle.
fn repr_gleit_stellen() -> [(usize, usize); 3] {
    [
        (
            parity::fallzahl::holen("wert_paritaet repr_gleit Bitmuster", 1500),
            1500,
        ),
        (
            parity::fallzahl::holen("wert_paritaet repr_gleit Mantisse", 1000),
            1000,
        ),
        (
            parity::fallzahl::holen("wert_paritaet repr_gleit Dezimalbrueche", 1000),
            1000,
        ),
    ]
}

/// Die Gleitkommazahlen fuer `repr`: die Rand- und Gleichstandswerte aus dem Auftrag, jede
/// Zweierpotenz, kleine ungerade Vielfache davon, die Kanten der Festkomma-Schranke (`1e16`, `1e-4`)
/// und zufaellige Werte (Bitmuster, Mantisse mal Zehnerpotenz, kurze Dezimalbrueche, Subnormale).
// `excessive_precision`: der Gleichstand ist als `f64` exakt (`...713.25`), sein kuerzester Text aber
// `...713.2`. Das Literal bleibt, wie es im Auftrag steht.
#[allow(clippy::cast_precision_loss, clippy::excessive_precision)]
fn repr_gleit() -> Vec<f64> {
    let [(n_bits, _), (n_mantisse, _), (n_dezimal, _)] = repr_gleit_stellen();
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
    for _ in 0..n_bits {
        v.push(f64::from_bits(naechste()));
    }
    for _ in 0..n_mantisse {
        let mantisse = (naechste() >> 11) as f64 / 9_007_199_254_740_992.0;
        let zehn = i32::try_from(naechste() % 61).unwrap() - 30;
        let vorzeichen = if naechste() & 1 == 0 { 1.0 } else { -1.0 };
        v.push(vorzeichen * mantisse * 10f64.powi(zehn));
    }
    for _ in 0..n_dezimal {
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
    // Wie bei `int`: die drei Zufallsstellen liefern genau die verlangte Zahl (Standard 3.500).
    assert_eq!(
        pool.len(),
        REPR_GLEIT_OHNE_ZUFALL + repr_gleit_stellen().iter().map(|(n, _)| n).sum::<usize>()
    );
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
    if parity::fallzahl::wache_gilt_pool("wert_paritaet repr_gleit", &repr_gleit_stellen()) {
        assert!(pool.len() > 9000 && exponent > 3000 && gleichstaende > 100);
    }
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
        if parity::fallzahl::wache_gilt_pool("wert_paritaet repr_gleit", &repr_gleit_stellen()) {
            assert!(werte.len() > 17_000 && gesehen.len() > 15_000);
        }
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
// ---------------------------------------------------------------------------------------------
// Block E: die Lader -- `PyWert::deserialize` ueber `serde_json` (gegen `json.loads`) und
// `serde_yaml_ng` (gegen `yaml.safe_load`, PyYAML 6.0.3, reiner Python-Lader wie in den Produktpfaden).

/// JSON-Texte ohne Struktur: Zahlen, Woerter, Zeichenketten.
#[rustfmt::skip]
const JSON_SKALARE: &[&str] = &[
    // Woerter
    "null", "true", "false", "nul", "True", "False", "None", "NULL", "NaN", "-NaN", "Infinity",
    "-Infinity", "+Infinity", "nan", "inf", "-inf", "infinity",
    // Ganzzahlen
    "0", "-0", "00", "01", "-01", "+1", "1", "-1", "123", "-123", "0123", "1_000", "0x10", "1,000",
    "9007199254740991", "9007199254740992", "9007199254740993", "-9007199254740993",
    "9223372036854775807", "9223372036854775808", "-9223372036854775808", "-9223372036854775809",
    "18446744073709551615", "18446744073709551616", "18446744073709551617",
    "340282366920938463463374607431768211455", "340282366920938463463374607431768211456",
    "-340282366920938463463374607431768211456",
    // Gleitkomma
    "0.0", "-0.0", "0.5", "-0.5", "1.0", "1.5", "1.", ".5", "-.5", "1.e3", "1e", "1e+", "1e-",
    "1E3", "1e3", "1E+3", "1e-3", "1e+3", "0e0", "-0e0", "0.0e0", "0e999999", "-0e999999",
    "1.7976931348623157e308", "1.7976931348623159e308", "1e308", "1e309", "-1e309", "1e999999999",
    "-1e999999999", "1e-400", "-1e-400", "1e-324", "5e-324", "2.4703282292062327e-324",
    "2.4703282292062328e-324", "1e-999999999", "0.1", "0.2", "0.30000000000000004", "1e22",
    "1e23", "123456789012345678901234567890", "1.23456789012345678901234567890",
    "0.00000000000000000000000000000000000001", "100000000000000000000.0", "1E400", "1e+400",
    // Zeichenketten
    "\"\"", "\"a\"", "\"a b\"", "\"\\u00e4\"", "\"\\u00E4\"", "\"\u{e4}\"", "\"\u{1f600}\"",
    "\"\\ud83d\\ude00\"", "\"\\ud800\"", "\"\\udc00\"", "\"\\ud800\\u0041\"", "\"\\ud83d\"",
    "\"\\ude00\\ud83d\"", "\"\\u0000\"", "\"\\/\"", "\"\\'\"", "\"\\x41\"", "\"\\u12\"", "\"\\uzzzz\"",
    "\"\\\"\"", "\"\\\\\"", "\"\\b\\f\\n\\r\\t\"", "\"a\tb\"", "\"a\nb\"", "\"\u{7f}\"", "\"\u{2028}\"",
    "\"\u{0}\"", "\"\u{1f}\"", "'a'", "a", "\"a", "a\"", "\"\\", "\"\\u\"", "\"\\U0001F600\"",
    "\"1\"", "\"true\"", "\"null\"", "\"\u{feff}\"", "\"\u{ffff}\"", "\"\u{10ffff}\"",
];

/// JSON-Texte mit Struktur, Leerraum und Rest hinter dem Wert.
#[rustfmt::skip]
const JSON_STRUKTUREN: &[&str] = &[
    "", " ", "\n", "[", "]", "{", "}", "[]", "[ ]", "{}", "{ }", "[1,]", "[,1]", "[1 2]", "[1,,2]",
    "[1]]", "[1] x", "{} {}", "1 2", "1,", "{\"a\":1,}", "{\"a\"}", "{\"a\":}", "{a:1}", "{1:2}",
    "{\"a\":1 \"b\":2}", "{\"a\":1,\"a\":2}", "{\"a\":1,\"b\":2,\"a\":3}", "{\"\":1}", "{\"a\":1,\"\\u0061\":2}",
    "{\"a\":{\"b\":1,\"b\":2}}", "{\"a\":1,\"a\":{\"b\":2}}", "{\"b\":1,\"a\":2}", "{\"a\":NaN}", "[NaN]",
    "[-Infinity, Infinity]", "[1e999]", "[1, 2.0, \"3\", null, true, [], {}]",
    "{\"a\": [1, {\"b\": [2, 3]}], \"c\": null}", "{\"\u{e4}\": 1}", "{\"\\ud800\": 1}", "{\"a\\u0000b\": 1}",
    "{\"a\": 1, \"A\": 2}", "[[[]]]", "[[],[[]],[[],[]]]", " 1 ", "\t1", "\n1\r\n", "\u{b}1", "\u{c}1", "\u{a0}1",
    "\u{feff}1", "1 \u{a0}", "1\u{0}", "1 // c", "/*c*/1", "[1,\n2]", "[1\t,2]", "{\"a\"\n:\n1}",
    "[\"a\",\"a\"]", "[1,1.0,1e0]", "{\"a\":[]}", "{\"a\":{}}", "[{}]", "[{},{}]", "{\"k\":\"\u{e4}\"}",
    "\"\u{e4}\" ", "[\"\\u00e4\",\"\u{e4}\"]", "[true,false,null]", "[True]", "[tru]", "[nul]", "{\"a\":tru}",
    "{\"a\":01}", "{\"a\":-}", "{\"a\":--1}", "{\"a\":1.}", "{\"a\":.1}", "{\"a\":1e}", "{\"a\":+1}",
];

/// YAML-Skalare: Boole, Null, Zahlen in allen Formen, Zeitstempel, Sonderfaelle. Sie stehen als Wurzel
/// und, wenn sie kein Flow-Zeichen tragen, in `[x]`, `{k: x}` und `{x: v}`.
#[rustfmt::skip]
const YAML_SKALARE: &[&str] = &[
    // Boole (YAML 1.1: y/n/yes/no/on/off in jeder Schreibweise)
    "y", "Y", "yes", "Yes", "YES", "yEs", "n", "N", "no", "No", "NO", "nO", "true", "True", "TRUE",
    "tRUE", "false", "False", "FALSE", "fALSE", "on", "On", "ON", "oN", "off", "Off", "OFF", "oFF",
    // Null
    "~", "null", "Null", "NULL", "nULL", "nil", "Nil", "None",
    // Ganzzahlen: Vorzeichen, Oktal (1.1: 017, 1.2: 0o17), Hex, Binaer, Unterstriche
    "0", "1", "-1", "+1", "+0", "-0", "007", "08", "09", "010", "017", "-017", "+017", "0o17", "0O17",
    "-0o17", "0o8", "0x1F", "0X1f", "0xdeadbeef", "-0x1F", "+0x1F", "0x", "0xG", "0x_1", "0b11", "0B11",
    "0b1_1", "-0b11", "+0b11", "0b", "0b2", "0o", "1_000", "1__0", "_1", "1_", "1,000", "0_1", "0_0",
    "1_0_0", "-1_0", "+1_0", "0_8", "1 000", "0b_1", "0x1_F", "0_",
    "9223372036854775807", "9223372036854775808", "-9223372036854775808", "-9223372036854775809",
    "18446744073709551615", "18446744073709551616", "18446744073709551617",
    "340282366920938463463374607431768211455", "340282366920938463463374607431768211456",
    "-170141183460469231731687303715884105728", "-170141183460469231731687303715884105729",
    "0x7FFFFFFFFFFFFFFF", "0x8000000000000000", "0xFFFFFFFFFFFFFFFF", "0x10000000000000000",
    "0b1111111111111111111111111111111111111111111111111111111111111111",
    "0b10000000000000000000000000000000000000000000000000000000000000000",
    "01777777777777777777777", "02000000000000000000000",
    // Sexagesimal (YAML 1.1): Basis 60
    "1:30", "190:20:30", "-1:30", "+1:30", "1:60", "1:59", "0:30", "00:30", "01:30", "1:2:3:4",
    "12:34:56:78", "1:30.5", "190:20:30.15", "1_0:30", "0:30.5", "1:3", "1:03", "1:", ":30", "1::30",
    "1:a", "12:30:45", "-12:30:45", "1:30:00", "1:00", "0:0", "10:10:10:10:10", "1:30:60",
    "1:30:00.5", "-190:20:30.15", "+190:20:30.15",
    // Gleitkomma: PyYAML verlangt Punkt, und ein Exponent braucht ein Vorzeichen
    "1.0", "-1.0", "+1.0", ".5", "-.5", "+.5", "5.", "-5.", "+5.", "1.5e3", "1.5E3", "1.5e+3", "1.5E+3",
    "1.5e-3", "1.5E-3", "1e3", "1E3", "1e+3", "1e-3", "1.e3", "1.e+3", "1.e-3", ".1e+3", ".1e3", "1_0.5",
    "1.5_5", "1.0_", "_1.0", "1_.0", "0.0", "-0.0", "+0.0", "00.5", "0.5.5", "1.2.3", "1.0e", "1.0e+",
    "1.0e+0", "1.0e+00", "1.0E+0", "1.0e-0", "1.0e+400", "-1.0e+400", "1.0e-400", "1e400", "1e-400",
    "1.7976931348623157e+308", "1.7976931348623159e+308", "5e-324", "5.0e-324", "4.9e-324",
    "2.4703282292062327e-324", "2.4703282292062328e-324", "1e22", "1.0e+22", "1.0e+23",
    "123456789012345678901234567890.0", "0.1", "0.2", "0.30000000000000004", "9007199254740993.0",
    ".inf", ".Inf", ".INF", ".iNf", "+.inf", "-.inf", "-.Inf", "-.INF", "+.Inf", "inf", "Inf", "INF", "+inf",
    "-inf", "infinity", ".infinity", ".nan", ".NaN", ".NAN", ".nAn", "nan", "NaN", "NAN", "-.nan", "+.nan",
    "Infinity", "-Infinity", "+Infinity",
    // Zeitstempel: PyYAML macht daraus date und datetime
    "2001-12-14", "2001-12-14t21:59:43.10-05:00", "2001-12-14 21:59:43.10 -5", "2001-12-14 21:59:43.10",
    "2001-12-14T21:59:43Z", "2001-12-14 21:59:43", "2001-13-14", "2001-02-30", "2001-2-3", "2001-12-14T21:59",
    "2001-12-14T21:59:43+05:30", "2001-12-14T21:59:43.123456789Z", "0001-01-01", "0000-01-01", "9999-12-31",
    "10000-01-01", "1-1-1", "2001-12-14Z",
    // Zeichenketten, die wie Sonderfaelle aussehen
    "=", "<<", "<", "<<<", "a", "abc", "a b", "\u{e4}", "1a", "1e", "e3", ".", "..", "...", "-1a", "0x1G", "1.5x",
    "+", "-", "+-1", "--1", "+.", "-.", ".e3", "True1", "yess", "nulll", "~~", "~a", "1/2", "$1.00", "1.000,50",
    "'yes'", "\"yes\"", "'1'", "\"1\"", "'~'", "''", "\"\"", "'null'", "'1.0'", "'0x1F'", "'true'",
    // Tags auf Skalaren
    "!!str 1", "!!str yes", "!!str", "!!int '1'", "!!int 'x'", "!!int 0x10", "!!int 1.5", "!!int ''", "!!float 1",
    "!!float 'x'", "!!float .inf", "!!float 1e3", "!!bool yes", "!!bool 'x'", "!!bool 1", "!!null ''",
    "!!null x", "!!null ~", "!!binary aGVsbG8=", "!!binary '!'", "!!binary ''", "!!timestamp 2001-12-14",
    "!!timestamp x", "!custom x", "!!custom x", "! x", "!<tag:yaml.org,2002:str> 1", "!!python/object:os.system x",
    "!!python/name:os.system ''", "!!python/tuple [1]", "!!int 1_000", "!!float 1_0.5", "!!str 0x10",
];

/// YAML-Dokumente mit Struktur: Folgen, Abbildungen, doppelte Schluessel, Anker, Merge, Tags,
/// Block- und Flow-Skalare, Anfuehrungszeichen, Direktiven, Unicode.
#[rustfmt::skip]
const YAML_STRUKTUREN: &[&str] = &[
    // Dokumente
    "", " ", "\n", "# nur Kommentar", "---", "--- ", "---\n", "...", "---\n...", "--- a", "--- a\n...",
    "a\n...\nb", "---\na\n---\nb", "--- a\n--- b", "a\n---\nb", "%YAML 1.1\n---\na", "%YAML 1.2\n---\na",
    "%YAML 2.0\n---\na", "%YAML 1.1\n%YAML 1.1\n---\na", "%TAG !e! tag:example.com,2000:\n---\n!e!x a",
    "%FOO bar\n---\na", "\u{feff}a: 1", "a: 1\n\u{feff}", "a: 1\n", "a: 1\n\n\n", "\n\na: 1", "  a: 1",
    "a: 1\n  b: 2", " a: 1\nb: 2", "a: 1\r\nb: 2\r\n", "a: 1\rb: 2", "a: 1\u{85}b: 2", "a: 1\u{2028}b: 2",
    // Folgen und Abbildungen
    "- a\n- b", "- a\n-\n- b", "-", "- - a\n  - b\n- c", "- a\n b", "-a", "- a: 1\n  b: 2\n- c: 3",
    "a: [1, 2]", "a:\n  - 1\n  - 2", "a:\n- 1\n- 2", "{a: 1, b: 2}", "{a: 1, b: 2,}", "[1, 2,]", "[,1]",
    "[1,,2]", "{a, b}", "{a: , b: }", "{? a : 1}", "? a\n: 1", "? [a, b]\n: c", "? {a: b}\n: c", "[a: 1]",
    "[a: 1, b]", "a: b: c", "a:\tb", "a : b", "a:b", "a: b # c", "a: b#c", "a: 1\nb: 2\nc:", "a:\nb:",
    "a: ''\nb: \"\"", "a: ~\nb: null", ": a", "? a", "a: |\n  x\n  y\nb: >\n  p\n  q\nc: d", "[]", "{}", "[ ]",
    "[[]]", "[{}]", "{a: []}", "{a: {}}", "a: []\nb: {}", "- []\n- {}", "[1, [2, [3]]]", "{a: {b: {c: d}}}",
    "[a, b, c]", "[a,b]", "[a , b]", "[ a, b ]", "{a: 1,b: 2}", "{a: 1 , b: 2}", "[a b, c]", "[a: b]",
    "[1, 2]\n", "[1,\n 2]", "{a:\n 1}", "- [a, b]\n- {c: d}", "a: [b,\n  c]",
    // doppelte Schluessel
    "a: 1\na: 2", "a: 1\nb: 2\na: 3", "{a: 1, a: 2}", "{a: 1, b: 2, a: 3}", "a: {x: 1}\na: {y: 2}", "1: a\n'1': b",
    "'1': a\n1: b", "true: a\n'true': b", "null: a\n~: b", "~: a\nnull: b", "1: a\n1.0: b", "0x10: a\n16: b",
    "? a\n? a", "a: 1\n'a': 2", "\"a\": 1\n'a': 2", "a: 1\n? a\n: 2", "1: a\n01: b", "1: a\n+1: b", "yes: a\n'yes': b",
    "yes: a\ntrue: b", "y: a\n'y': b", "1: a\n0b1: b", "1.5: a\n'1.5': b", ".5: a\n0.5: b", "1: a\n1_0: b",
    // Schluessel, die keine Zeichenketten sind
    "{1: a}", "{1.5: a}", "{true: a}", "{yes: a}", "{~: a}", "{null: a}", "{.nan: a}", "{.inf: a}", "{0x10: a}",
    "{2001-12-14: a}", "{1:30: a}", "{[a]: b}", "{{a: b}: c}", "{? [a, b] : c}", "{!!binary YQ==: a}",
    "{!!str 1: a}", "{!!int '1': a}", "{'': a}", "{\"\": a}", "{: a}", "? \n: a", "? a b\n: c", "{a b: c}",
    "{a: b, c}", "{ a : b }", "{a : b}", "{a :b}", "{\"a\":b}", "{\"a\": b}", "{'a':b}", "[\"a\":b]",
    // Anker und Aliase
    "a: &x 1\nb: *x", "a: &x hello\nb: *x", "- &a [1, 2]\n- *a", "a: &x {k: v}\nb: *x\nc: *x", "&a a: *a", "&a [*a]",
    "&a {k: *a}", "a: *x", "*x", "a: &x\nb: *x", "a: &x 1\nb: &x 2\nc: *x", "[&x 1, *x]", "[&x a, &y b, *y, *x]",
    "&a &b x", "&a !!str 1", "!!str &a 1", "&a\n- 1\n- *a", "a: &a1 x\nb: &a2 *a1\nc: *a2", "&k key: v\n*k : w",
    "a: &x [1]\nb: *x\nc: *x", "&a x", "&a", "- &a\n- *a", "a: &a ~\nb: *a", "a: &a null\nb: *a", "a: &a 1\nb: [*a, *a]",
    "a: &a 1.5\nb: *a", "a: &a yes\nb: *a", "a: &a '1'\nb: *a", "a: &a\n  - 1\nb: *a", "a: &a\n  k: v\nb:\n  - *a",
    "*a : 1", "a: &a b\n*a : c", "{&a k: v, *a : w}", "a: *a", "a: &a *a", "&a *a", "&1 x", "&a-b x", "&a_b x", "&\u{e4} x",
    "a: &x [1, 2]\nb: &y [*x, *x]\nc: *y", "x: &x {a: 1}\ny: &y {b: *x}\nz: *y",
    "a: &a [&b 1, &c 2]\nd: [*a, *b, *c]", "- &a x\n- &a y\n- *a",
    // Merge-Schluessel
    "base: &b {a: 1, b: 2}\nd:\n  <<: *b\n  b: 3", "base: &b {a: 1, b: 2}\nd:\n  b: 3\n  <<: *b",
    "x: &a {p: 1}\ny: &b {p: 2, q: 3}\nd:\n  <<: [*a, *b]", "d:\n  <<: [{a: 1}, {a: 2, b: 3}]", "d: {<<: {a: 1}}",
    "<<: {a: 1}\nb: 2", "d:\n  <<: 1", "d:\n  <<: [1]", "d:\n  <<: *x", "d:\n  '<<': 1", "d:\n  <<: ~", "{<<: {a: 1}, a: 2}",
    "a: &a {x: 1}\nb: &b\n  <<: *a\n  y: 2\nc:\n  <<: *b", "d:\n  <<: {a: 1}\n  <<: {b: 2}", "d:\n  <<: []",
    "d:\n  <<: {}", "d:\n  <<: [[1]]", "d:\n  <<: [*a]", "d:\n  <<: \"x\"", "<<: 1", "- <<: {a: 1}\n  b: 2",
    "a: {x: 1}\nd:\n  <<: {y: 2}\n  x: 3", "d:\n  ? <<\n  : {a: 1}", "d:\n  \"<<\": {a: 1}", "d:\n  !!merge <<: {a: 1}",
    // Tags
    "!!str", "!!set {a, b}", "!!set [a]", "!!set {a: 1}", "!!omap [a: 1, b: 2]", "!!omap [a, b]", "!!omap [a: 1, a: 2]",
    "!!pairs [a: 1, a: 2]", "!!map {a: 1}", "!!seq [1]", "!!str [1]", "!!seq 1", "!!map []", "!!map 1",
    "!!python/tuple [1]", "--- !!str\n1", "!!binary |\n  aGVs\n  bG8=", "!!binary YQ", "!!set\n? a\n? b", "!!map\na: 1",
    "!!seq\n- 1", "- !!str 1\n- !!int '2'\n- !!float 3", "a: !!str 1\nb: !!int '2'", "!!bool true", "!!str\n- a",
    "!!float [1]", "!!int {a: 1}", "!<!> x", "!a!b!c x", "!!!x y", "!x!y z", "!!python/object/apply:os.system [x]",
    "!!python/object/new:str [x]", "!!python/unicode x", "!!python/bytes x", "!!python/complex 1+2j",
    "!!python/module:os x", "!!python/long 1", "!!python/float 1.5", "!!python/dict {a: 1}", "!!python/list [1]",
    "!!python/str x", "!!python/bool true", "!!python/none ''", "!!js/function x", "!!java.util.Date x",
    "!!merge x", "!!value x", "!!yaml x", "!!null", "!!null 1", "!!bool", "!!int", "!!float", "!!timestamp",
    // Skalare in Anfuehrungszeichen und Bloecken
    "'it''s'", "'a\\b'", "\"a\\tb\"", "\"\\x41\"", "\"\\u00e4\"", "\"\\U0001F600\"", "\"\\N\"", "\"\\_\"", "\"\\L\"",
    "\"\\P\"", "\"\\e\"", "\"\\/\"", "\"\\a\\b\\f\\v\\0\"", "\"\\q\"", "\"\\x4\"", "\"\\u12\"", "\"\\ud800\"", "\"\\udc00\"",
    "\"\\ud83d\\ude00\"", "\"\\U0000D800\"", "\"\\U00110000\"", "\"a\\\n  b\"", "\"a\n  b\"", "'a\n  b'", "'a\n\n  b'",
    "a\n b\n\n c", "a\n  b", "|\n a\n b", "|-\n a\n", "|+\n a\n\n", ">\n a\n b\n\n c\n", ">-\n a\n b", ">+\n a\n\n",
    ">2\n  a\n   b", "|2\n  a", "|\n  a\n b", "|\n\ta", "|\n  a\n\n\n", "|1\n a", "|10\n a", "|0\n a", "|\n", ">\n", "|\n\n",
    "|x\n a", "|-+\n a", "|2-\n  a", "a: |\n  1\nb: |-\n  2", "- |\n  a\n- >\n  b", "a: >\n  x\n\n  y\nb: 1",
    "\"\"", "''", "\"\\\"\"", "'\"'", "\"'\"", "\"a\" \"b\"", "'a' 'b'", "\"a\"b", "'a'b", "'a'#b", "a #b", "a#b",
    "'a' #b", "a\tb", "\ta", "a\t", "a:\n\tb: 1", "[\t1]", "{\ta: 1}", "a: 'b\n  c'", "a: \"b\n  c\"", "a: b\n  c",
    "\"a\\\n\\ b\"", "\"\\ \"", "\"\\\t\"", "\"\\\n\"", "'\\'", "'''", "\"\"\"", "\"a", "'a", "[a", "a]", "{a", "a}",
    // Unicode und Steuerzeichen
    "\u{e4}", "\u{85}", "a\u{85}b", "a\u{2028}b", "a\u{2029}b", "- \u{85}a", "\u{a0}a", "a\u{a0}", "'\u{feff}'",
    "\u{1f600}", "a\u{0}b", "\u{0}", "a\u{7}b", "\u{1}", "\u{7f}", "\u{80}", "\u{9f}", "\u{fffe}", "\u{ffff}",
    "\u{10ffff}", "\u{d7ff}", "\u{e000}", "\u{fffd}", "a\u{200b}b", "\u{feff}", "\u{feff}\u{feff}a", "a: \u{e4}",
    "\u{e4}: a", "{\u{e4}: \u{e5}}", "\"\u{0}\"", "'\u{0}'", "\"\u{7}\"", "\"\u{85}\"", "\"\u{2028}\"", "\"\u{1f}\"",
    "\"\u{ffff}\"", "\"\u{fffe}\"", "'\u{fffe}'", "\u{e000}: a", "a\u{1f}b", "a\u{1b}b", "\"a\u{1b}b\"",
    // Zeichen, die ein Skalar einleiten
    "@a", "`a", "%a", "|", ">", "?", "? ", "-", "- ", "- -", ":", ": ", "[", "]", "{", "}", ",", "# c", "&", "*", "!", "'",
    "\"", "a: @b", "a: `b", "a: %b", "a: |", "a: >", "a: ?", "a: -", "a: - b", "a: [", "a: ,", "a: ,b", "a: b,", "a: &",
    "a: *", "a: !", "a: #", "a: \u{feff}",
];

/// Ein Skalar steht in Flow-Kontexten, wenn er kein Flow- oder Zeilenzeichen traegt.
fn flow_tauglich(x: &str) -> bool {
    !x.contains([
        ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`', '\n', '\t',
    ]) && !x.is_empty()
        && !x.starts_with(['-', '?', ':', ' '])
        && !x.ends_with([' ', ':'])
}

/// Die Antwort von `CPython` auf einen Ladeversuch: (Draht-Wert, `repr`) oder (Klasse, Meldung).
type LadeAnt = Result<(Value, String), (String, String)>;

fn lade_ant(a: &Value) -> LadeAnt {
    match a.get("ok") {
        Some(o) => Ok((o["w"].clone(), o["r"].as_str().unwrap_or("?").to_owned())),
        None => Err((
            a["err"].as_str().unwrap_or("?").to_owned(),
            a["msg"].as_str().unwrap_or("?").to_owned(),
        )),
    }
}

fn lade_antworten(antworten: &Value, erwartet: usize) -> Vec<LadeAnt> {
    let Value::Array(antworten) = antworten else {
        panic!("keine Liste: {antworten}");
    };
    assert_eq!(antworten.len(), erwartet);
    antworten.iter().map(lade_ant).collect()
}

/// `json.loads(text)` oder `yaml.safe_load(text)` in `CPython`, je Text.
fn laden_cpython(art: &str, texte: &[String]) -> Vec<LadeAnt> {
    let a = frage(&json!({ "fn": "wert.laden", "art": art, "texte": texte }));
    lade_antworten(&a, texte.len())
}

/// Der Lader von Rust: `PyWert::deserialize` ueber `serde_json` oder `serde_yaml_ng`.
fn laden_rust(art: &str, text: &str) -> Result<PyWert, String> {
    if art == "json" {
        serde_json::from_str::<PyWert>(text).map_err(|e| e.to_string())
    } else {
        serde_yaml_ng::from_str::<PyWert>(text).map_err(|e| e.to_string())
    }
}

/// (Rust, `CPython`) als `repr`; `FEHLER` = der Lader lehnt ab, bei `CPython` mit der Klasse.
type Abw = (String, String);

/// Jede NaN auf dieselben Bits: welche Bits ein NaN traegt, ist nicht beobachtbar (`repr` ist `nan`), aber
/// `PyYAML` bildet es als `-inf / inf` und setzt auf `x86` das Vorzeichenbit, Rust nicht.
fn nan_gleich(v: &Value) -> Value {
    match v {
        Value::Object(o) if o.len() == 1 && o.contains_key("f") => {
            let bits = o["f"]
                .as_str()
                .and_then(|h| u64::from_str_radix(h, 16).ok());
            match bits {
                Some(b) if f64::from_bits(b).is_nan() => {
                    json!({ "f": format!("{:016x}", f64::NAN.to_bits()) })
                }
                _ => v.clone(),
            }
        }
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, w)| (k.clone(), nan_gleich(w))).collect())
        }
        Value::Array(a) => Value::Array(a.iter().map(nan_gleich).collect()),
        _ => v.clone(),
    }
}

/// Die ersten 80 Zeichen: lange `repr` (Zeitstempel mit Zeitzone) bleiben lesbar in der Liste.
fn kurz(s: &str) -> String {
    s.chars().take(80).collect()
}

/// `None`: beide laden denselben Wert (Draht-Format, also Art, Reihenfolge und Bits gleich) oder
/// beide lehnen ab -- die Fehlermeldungen der beiden Seiten sind nicht vergleichbar.
fn abweichung(rust: &Result<PyWert, String>, py: &LadeAnt) -> Option<Abw> {
    match (rust, py) {
        (Ok(w), Ok((d, _))) if nan_gleich(&draht(w)) == nan_gleich(d) => None,
        (Err(_), Err(_)) => None,
        (Ok(w), Ok((_, r))) => Some((kurz(&w.repr()), kurz(r))),
        (Ok(w), Err((k, _))) => Some((kurz(&w.repr()), format!("FEHLER {k}"))),
        (Err(_), Ok((_, r))) => Some(("FEHLER".to_owned(), kurz(r))),
    }
}

struct Messung {
    text: String,
    abw: Option<Abw>,
    /// `CPython` laedt den Text.
    py_ok: bool,
    /// `CPython` laedt den Text als `str` (nur dann bleibt er als Schluessel ein Text).
    py_str: bool,
}

fn messe(art: &str, texte: &[String]) -> Vec<Messung> {
    let py = laden_cpython(art, texte);
    texte
        .iter()
        .zip(&py)
        .map(|(t, p)| Messung {
            abw: abweichung(&laden_rust(art, t), p),
            py_ok: p.is_ok(),
            py_str: matches!(p, Ok((Value::String(_), _))),
            text: t.clone(),
        })
        .collect()
}

/// Ohne doppelte Eintraege, in der Reihenfolge des ersten Vorkommens.
fn eindeutig(texte: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut gesehen = std::collections::HashSet::new();
    texte
        .into_iter()
        .filter(|t| gesehen.insert(t.clone()))
        .collect()
}

/// Ein festgehaltener Abweichungsfall: Text, Rust, `CPython`.
type Pin = (&'static str, &'static str, &'static str);

/// Die gemessenen Abweichungen muessen genau die festgehaltenen sein, in beiden Richtungen: eine neue
/// Abweichung und eine, die verschwand (der Lader wurde besser), lassen den Test rot werden. Bei
/// Abweichung steht die gemessene Liste als Quelltext in der Meldung.
fn gleiche_pins(name: &str, ms: &[Messung], soll: &[Pin]) -> Option<String> {
    let ist: Vec<(String, String, String)> = ms
        .iter()
        .filter_map(|m| {
            m.abw
                .as_ref()
                .map(|(r, p)| (m.text.clone(), r.clone(), p.clone()))
        })
        .collect();
    let soll: Vec<(String, String, String)> = soll
        .iter()
        .map(|(t, r, p)| ((*t).to_owned(), (*r).to_owned(), (*p).to_owned()))
        .collect();
    if ist == soll {
        return None;
    }
    let neu: Vec<String> = ist
        .iter()
        .filter(|e| !soll.contains(e))
        .map(|(t, r, p)| format!("    ({t:?}, {r:?}, {p:?}),"))
        .collect();
    let weg: Vec<String> = soll
        .iter()
        .filter(|e| !ist.contains(e))
        .map(|(t, r, p)| format!("    ({t:?}, {r:?}, {p:?}),"))
        .collect();
    Some(format!(
        "{name}: {} gemessen, {} festgehalten.\nNEU (nicht festgehalten):\n{}\nWEG (festgehalten, nicht mehr gemessen):\n{}",
        ist.len(),
        soll.len(),
        neu.join("\n"),
        weg.join("\n")
    ))
}

/// Die Texte der Wurzel plus ihre Kontexte; liefert die Messungen der Wurzeln und die Kontexte, in
/// denen sich ein Skalar anders verhaelt als erwartet. Erwartet: ein Skalar weicht im Wert-Kontext
/// (`[x]`, `{k: x}`) ab, wenn er an der Wurzel abweicht, und sonst nicht. Als Schluessel (`{x: v}`)
/// weicht er zusaetzlich ab, wenn `CPython` ihn nicht als Text laedt: `PyWert` kennt nur Text-Schluessel
/// und schreibt `{1: 'v'}` als `{'1': 'v'}`.
fn mit_kontexten(
    art: &str,
    skalare: &[&str],
    tauglich: fn(&str) -> bool,
    kontexte: fn(&str) -> Vec<(String, bool)>,
) -> (Vec<Messung>, Vec<String>) {
    let wurzeln = eindeutig(skalare.iter().map(|s| (*s).to_owned()));
    let wurzel_ms = messe(art, &wurzeln);
    let mut texte = Vec::new();
    let mut zu_wurzel = Vec::new();
    for (x, m) in wurzeln.iter().zip(&wurzel_ms) {
        if !tauglich(x) {
            continue;
        }
        for (k, ist_schluessel) in kontexte(x) {
            texte.push(k);
            zu_wurzel.push(m.abw.is_some() || (ist_schluessel && !m.py_str));
        }
    }
    let ms = messe(art, &texte);
    let ausnahmen = ms
        .iter()
        .zip(&zu_wurzel)
        .filter(|(m, wurzel_weicht_ab)| m.abw.is_some() != **wurzel_weicht_ab)
        .map(|(m, _)| m.text.clone())
        .collect();
    (wurzel_ms, ausnahmen)
}

fn json_kontexte(x: &str) -> Vec<(String, bool)> {
    vec![
        (format!("[{x}]"), false),
        (format!("{{\"k\": {x}}}"), false),
    ]
}

fn json_tauglich(x: &str) -> bool {
    !x.is_empty() && !x.contains(['\n', '\t', ',']) && !x.starts_with(' ')
}

fn yaml_kontexte_liste(x: &str) -> Vec<(String, bool)> {
    vec![
        (format!("[{x}]"), false),
        (format!("{{k: {x}}}"), false),
        (format!("{{{x}: v}}"), true),
    ]
}

/// JSON-Skalare, die `serde_json` anders laedt als `json.loads`: D1 (Ganzzahl ausserhalb von `i64::MIN..=u64::MAX` wird
/// `Gleit`), D2 (`NaN`/`Infinity` und Zahlen ueber `f64::MAX` lehnt `serde_json` ab), `-0` (Rust `-0.0`, Python `0`)
/// und ein einzelnes Surrogat in einer Zeichenkette (`String` kennt keines).
#[rustfmt::skip]
const JSON_SKALARE_ABW: &[Pin] = &[
    ("NaN", "FEHLER", "nan"),
    ("Infinity", "FEHLER", "inf"),
    ("-Infinity", "FEHLER", "-inf"),
    ("-0", "-0.0", "0"),
    ("-9223372036854775809", "-9.223372036854776e+18", "-9223372036854775809"),
    ("18446744073709551616", "1.8446744073709552e+19", "18446744073709551616"),
    ("18446744073709551617", "1.8446744073709552e+19", "18446744073709551617"),
    ("340282366920938463463374607431768211455", "3.402823669209385e+38", "340282366920938463463374607431768211455"),
    ("340282366920938463463374607431768211456", "3.402823669209385e+38", "340282366920938463463374607431768211456"),
    ("-340282366920938463463374607431768211456", "-3.402823669209385e+38", "-340282366920938463463374607431768211456"),
    ("1.7976931348623159e308", "FEHLER", "inf"),
    ("1e309", "FEHLER", "inf"),
    ("-1e309", "FEHLER", "-inf"),
    ("1e999999999", "FEHLER", "inf"),
    ("-1e999999999", "FEHLER", "-inf"),
    ("123456789012345678901234567890", "1.2345678901234568e+29", "123456789012345678901234567890"),
    ("1E400", "FEHLER", "inf"),
    ("1e+400", "FEHLER", "inf"),
    ("\"\\ud800\"", "FEHLER", "'\\ud800'"),
    ("\"\\udc00\"", "FEHLER", "'\\udc00'"),
    ("\"\\ud800\\u0041\"", "FEHLER", "'\\ud800A'"),
    ("\"\\ud83d\"", "FEHLER", "'\\ud83d'"),
    ("\"\\ude00\\ud83d\"", "FEHLER", "'\\ude00\\ud83d'"),
];

/// JSON-Dokumente: dieselben Gruende, in einem Behaelter.
#[rustfmt::skip]
const JSON_STRUKTUREN_ABW: &[Pin] = &[
    ("{\"a\":NaN}", "FEHLER", "{'a': nan}"),
    ("[NaN]", "FEHLER", "[nan]"),
    ("[-Infinity, Infinity]", "FEHLER", "[-inf, inf]"),
    ("[1e999]", "FEHLER", "[inf]"),
    ("{\"\\ud800\": 1}", "FEHLER", "{'\\ud800': 1}"),
];

/// Kontexte `[x]` und `{"k": x}`, in denen ein JSON-Skalar sich anders verhaelt als an der Wurzel.
#[rustfmt::skip]
const JSON_KONTEXT_AUSNAHMEN: &[&str] = &[];

/// YAML-Skalare, die `serde_yaml_ng` (YAML 1.2) anders laedt als `yaml.safe_load` (`PyYAML`, YAML 1.1).
#[rustfmt::skip]
const YAML_SKALARE_ABW: &[Pin] = &[
    ("yes", "'yes'", "True"),
    ("Yes", "'Yes'", "True"),
    ("YES", "'YES'", "True"),
    ("no", "'no'", "False"),
    ("No", "'No'", "False"),
    ("NO", "'NO'", "False"),
    ("on", "'on'", "True"),
    ("On", "'On'", "True"),
    ("ON", "'ON'", "True"),
    ("off", "'off'", "False"),
    ("Off", "'Off'", "False"),
    ("OFF", "'OFF'", "False"),
    ("007", "'007'", "7"),
    ("010", "'010'", "8"),
    ("017", "'017'", "15"),
    ("-017", "'-017'", "-15"),
    ("+017", "'+017'", "15"),
    ("0o17", "15", "'0o17'"),
    ("-0o17", "-15", "'-0o17'"),
    ("0x_1", "'0x_1'", "1"),
    ("0b1_1", "'0b1_1'", "3"),
    ("1_000", "'1_000'", "1000"),
    ("1__0", "'1__0'", "10"),
    ("1_", "'1_'", "1"),
    ("0_1", "'0_1'", "1"),
    ("0_0", "'0_0'", "0"),
    ("1_0_0", "'1_0_0'", "100"),
    ("-1_0", "'-1_0'", "-10"),
    ("+1_0", "'+1_0'", "10"),
    ("0b_1", "'0b_1'", "1"),
    ("0x1_F", "'0x1_F'", "31"),
    ("0_", "'0_'", "0"),
    ("-9223372036854775809", "-9.223372036854776e+18", "-9223372036854775809"),
    ("18446744073709551616", "1.8446744073709552e+19", "18446744073709551616"),
    ("18446744073709551617", "1.8446744073709552e+19", "18446744073709551617"),
    ("340282366920938463463374607431768211455", "3.402823669209385e+38", "340282366920938463463374607431768211455"),
    ("340282366920938463463374607431768211456", "3.402823669209385e+38", "340282366920938463463374607431768211456"),
    ("-170141183460469231731687303715884105728", "-1.7014118346046923e+38", "-170141183460469231731687303715884105728"),
    ("-170141183460469231731687303715884105729", "-1.7014118346046923e+38", "-170141183460469231731687303715884105729"),
    ("0x10000000000000000", "1.8446744073709552e+19", "18446744073709551616"),
    ("0b10000000000000000000000000000000000000000000000000000000000000000", "1.8446744073709552e+19", "18446744073709551616"),
    ("01777777777777777777777", "'01777777777777777777777'", "18446744073709551615"),
    ("02000000000000000000000", "'02000000000000000000000'", "18446744073709551616"),
    ("1:30", "'1:30'", "90"),
    ("190:20:30", "'190:20:30'", "685230"),
    ("-1:30", "'-1:30'", "-90"),
    ("+1:30", "'+1:30'", "90"),
    ("1:59", "'1:59'", "119"),
    ("1:2:3:4", "'1:2:3:4'", "223384"),
    ("1:30.5", "'1:30.5'", "90.5"),
    ("190:20:30.15", "'190:20:30.15'", "685230.15"),
    ("1_0:30", "'1_0:30'", "630"),
    ("0:30.5", "'0:30.5'", "30.5"),
    ("1:3", "'1:3'", "63"),
    ("1:03", "'1:03'", "63"),
    ("1:", "{'1': None}", "{1: None}"),
    ("12:30:45", "'12:30:45'", "45045"),
    ("-12:30:45", "'-12:30:45'", "-45045"),
    ("1:30:00", "'1:30:00'", "5400"),
    ("1:00", "'1:00'", "60"),
    ("10:10:10:10:10", "'10:10:10:10:10'", "131796610"),
    ("1:30:00.5", "'1:30:00.5'", "5400.5"),
    ("-190:20:30.15", "'-190:20:30.15'", "-685230.15"),
    ("+190:20:30.15", "'+190:20:30.15'", "685230.15"),
    ("-.5", "-0.5", "'-.5'"),
    ("+.5", "0.5", "'+.5'"),
    ("1.5e3", "1500.0", "'1.5e3'"),
    ("1.5E3", "1500.0", "'1.5E3'"),
    ("1e3", "1000.0", "'1e3'"),
    ("1E3", "1000.0", "'1E3'"),
    ("1e+3", "1000.0", "'1e+3'"),
    ("1e-3", "0.001", "'1e-3'"),
    ("1.e3", "1000.0", "'1.e3'"),
    (".1e3", "100.0", "'.1e3'"),
    ("1_0.5", "'1_0.5'", "10.5"),
    ("1.5_5", "'1.5_5'", "1.55"),
    ("1.0_", "'1.0_'", "1.0"),
    ("1_.0", "'1_.0'", "1.0"),
    ("1.0e+400", "'1.0e+400'", "inf"),
    ("-1.0e+400", "'-1.0e+400'", "-inf"),
    ("1e-400", "0.0", "'1e-400'"),
    ("1.7976931348623159e+308", "'1.7976931348623159e+308'", "inf"),
    ("5e-324", "5e-324", "'5e-324'"),
    ("1e22", "1e+22", "'1e22'"),
    ("2001-12-14", "'2001-12-14'", "datetime.date(2001, 12, 14)"),
    ("2001-12-14t21:59:43.10-05:00", "'2001-12-14t21:59:43.10-05:00'", "datetime.datetime(2001, 12, 14, 21, 59, 43, 100000, tzinfo=datetime.timezone(dat"),
    ("2001-12-14 21:59:43.10 -5", "'2001-12-14 21:59:43.10 -5'", "datetime.datetime(2001, 12, 14, 21, 59, 43, 100000, tzinfo=datetime.timezone(dat"),
    ("2001-12-14 21:59:43.10", "'2001-12-14 21:59:43.10'", "datetime.datetime(2001, 12, 14, 21, 59, 43, 100000)"),
    ("2001-12-14T21:59:43Z", "'2001-12-14T21:59:43Z'", "datetime.datetime(2001, 12, 14, 21, 59, 43, tzinfo=datetime.timezone.utc)"),
    ("2001-12-14 21:59:43", "'2001-12-14 21:59:43'", "datetime.datetime(2001, 12, 14, 21, 59, 43)"),
    ("2001-13-14", "'2001-13-14'", "FEHLER ValueError"),
    ("2001-02-30", "'2001-02-30'", "FEHLER ValueError"),
    ("2001-12-14T21:59:43+05:30", "'2001-12-14T21:59:43+05:30'", "datetime.datetime(2001, 12, 14, 21, 59, 43, tzinfo=datetime.timezone(datetime.ti"),
    ("2001-12-14T21:59:43.123456789Z", "'2001-12-14T21:59:43.123456789Z'", "datetime.datetime(2001, 12, 14, 21, 59, 43, 123456, tzinfo=datetime.timezone.utc"),
    ("0001-01-01", "'0001-01-01'", "datetime.date(1, 1, 1)"),
    ("0000-01-01", "'0000-01-01'", "FEHLER ValueError"),
    ("9999-12-31", "'9999-12-31'", "datetime.date(9999, 12, 31)"),
    ("=", "'='", "FEHLER ConstructorError"),
    ("<<", "'<<'", "FEHLER ConstructorError"),
    ("!!bool yes", "FEHLER", "True"),
    ("!!null ''", "FEHLER", "None"),
    ("!!null x", "FEHLER", "None"),
    ("!!binary aGVsbG8=", "'aGVsbG8='", "b'hello'"),
    ("!!binary '!'", "'!'", "b''"),
    ("!!binary ''", "''", "b''"),
    ("!!timestamp 2001-12-14", "'2001-12-14'", "datetime.date(2001, 12, 14)"),
    ("!!timestamp x", "'x'", "FEHLER AttributeError"),
    ("!!custom x", "'x'", "FEHLER ConstructorError"),
    ("! x", "FEHLER", "'x'"),
    ("!!python/object:os.system x", "'x'", "FEHLER ConstructorError"),
    ("!!python/name:os.system ''", "''", "FEHLER ConstructorError"),
    ("!!python/tuple [1]", "[1]", "FEHLER ConstructorError"),
    ("!!int 1_000", "FEHLER", "1000"),
    ("!!float 1_0.5", "FEHLER", "10.5"),
];

/// YAML-Dokumente mit Struktur.
#[rustfmt::skip]
const YAML_STRUKTUREN_ABW: &[Pin] = &[
    ("%TAG !e! tag:example.com,2000:\n---\n!e!x a", "'a'", "FEHLER ConstructorError"),
    ("%FOO bar\n---\na", "FEHLER", "'a'"),
    ("a: 1\n\u{feff}", "{'a': 1}", "FEHLER ScannerError"),
    ("a:\tb", "{'a': 'b'}", "FEHLER ScannerError"),
    ("1: a\n'1': b", "{'1': 'b'}", "{1: 'a', '1': 'b'}"),
    ("'1': a\n1: b", "{'1': 'b'}", "{'1': 'a', 1: 'b'}"),
    ("true: a\n'true': b", "{'true': 'b'}", "{True: 'a', 'true': 'b'}"),
    ("null: a\n~: b", "{'null': 'a', '~': 'b'}", "{None: 'b'}"),
    ("~: a\nnull: b", "{'~': 'a', 'null': 'b'}", "{None: 'b'}"),
    ("1: a\n1.0: b", "{'1': 'a', '1.0': 'b'}", "{1: 'b'}"),
    ("0x10: a\n16: b", "{'0x10': 'a', '16': 'b'}", "{16: 'b'}"),
    ("1: a\n01: b", "{'1': 'a', '01': 'b'}", "{1: 'b'}"),
    ("1: a\n+1: b", "{'1': 'a', '+1': 'b'}", "{1: 'b'}"),
    ("yes: a\n'yes': b", "{'yes': 'b'}", "{True: 'a', 'yes': 'b'}"),
    ("yes: a\ntrue: b", "{'yes': 'a', 'true': 'b'}", "{True: 'b'}"),
    ("1: a\n0b1: b", "{'1': 'a', '0b1': 'b'}", "{1: 'b'}"),
    ("1.5: a\n'1.5': b", "{'1.5': 'b'}", "{1.5: 'a', '1.5': 'b'}"),
    (".5: a\n0.5: b", "{'.5': 'a', '0.5': 'b'}", "{0.5: 'b'}"),
    ("1: a\n1_0: b", "{'1': 'a', '1_0': 'b'}", "{1: 'a', 10: 'b'}"),
    ("{1: a}", "{'1': 'a'}", "{1: 'a'}"),
    ("{1.5: a}", "{'1.5': 'a'}", "{1.5: 'a'}"),
    ("{true: a}", "{'true': 'a'}", "{True: 'a'}"),
    ("{yes: a}", "{'yes': 'a'}", "{True: 'a'}"),
    ("{~: a}", "{'~': 'a'}", "{None: 'a'}"),
    ("{null: a}", "{'null': 'a'}", "{None: 'a'}"),
    ("{.nan: a}", "{'.nan': 'a'}", "{nan: 'a'}"),
    ("{.inf: a}", "{'.inf': 'a'}", "{inf: 'a'}"),
    ("{0x10: a}", "{'0x10': 'a'}", "{16: 'a'}"),
    ("{2001-12-14: a}", "{'2001-12-14': 'a'}", "{datetime.date(2001, 12, 14): 'a'}"),
    ("{1:30: a}", "{'1:30': 'a'}", "{90: 'a'}"),
    ("{!!binary YQ==: a}", "{'YQ==': 'a'}", "{b'a': 'a'}"),
    ("{!!int '1': a}", "{'1': 'a'}", "{1: 'a'}"),
    ("? \n: a", "{'': 'a'}", "{None: 'a'}"),
    ("&a [*a]", "FEHLER", "[[...]]"),
    ("&a {k: *a}", "FEHLER", "{'k': {...}}"),
    ("a: &x 1\nb: &x 2\nc: *x", "{'a': 1, 'b': 2, 'c': 2}", "FEHLER ComposerError"),
    ("&a\n- 1\n- *a", "FEHLER", "[1, [...]]"),
    ("a: &a yes\nb: *a", "{'a': 'yes', 'b': 'yes'}", "{'a': True, 'b': True}"),
    ("- &a x\n- &a y\n- *a", "['x', 'y', 'y']", "FEHLER ComposerError"),
    ("base: &b {a: 1, b: 2}\nd:\n  <<: *b\n  b: 3", "{'base': {'a': 1, 'b': 2}, 'd': {'<<': {'a': 1, 'b': 2}, 'b': 3}}", "{'base': {'a': 1, 'b': 2}, 'd': {'a': 1, 'b': 3}}"),
    ("base: &b {a: 1, b: 2}\nd:\n  b: 3\n  <<: *b", "{'base': {'a': 1, 'b': 2}, 'd': {'b': 3, '<<': {'a': 1, 'b': 2}}}", "{'base': {'a': 1, 'b': 2}, 'd': {'a': 1, 'b': 3}}"),
    ("x: &a {p: 1}\ny: &b {p: 2, q: 3}\nd:\n  <<: [*a, *b]", "{'x': {'p': 1}, 'y': {'p': 2, 'q': 3}, 'd': {'<<': [{'p': 1}, {'p': 2, 'q': 3}]}", "{'x': {'p': 1}, 'y': {'p': 2, 'q': 3}, 'd': {'p': 1, 'q': 3}}"),
    ("d:\n  <<: [{a: 1}, {a: 2, b: 3}]", "{'d': {'<<': [{'a': 1}, {'a': 2, 'b': 3}]}}", "{'d': {'a': 1, 'b': 3}}"),
    ("d: {<<: {a: 1}}", "{'d': {'<<': {'a': 1}}}", "{'d': {'a': 1}}"),
    ("<<: {a: 1}\nb: 2", "{'<<': {'a': 1}, 'b': 2}", "{'a': 1, 'b': 2}"),
    ("d:\n  <<: 1", "{'d': {'<<': 1}}", "FEHLER ConstructorError"),
    ("d:\n  <<: [1]", "{'d': {'<<': [1]}}", "FEHLER ConstructorError"),
    ("d:\n  <<: ~", "{'d': {'<<': None}}", "FEHLER ConstructorError"),
    ("{<<: {a: 1}, a: 2}", "{'<<': {'a': 1}, 'a': 2}", "{'a': 2}"),
    ("a: &a {x: 1}\nb: &b\n  <<: *a\n  y: 2\nc:\n  <<: *b", "{'a': {'x': 1}, 'b': {'<<': {'x': 1}, 'y': 2}, 'c': {'<<': {'<<': {'x': 1}, 'y':", "{'a': {'x': 1}, 'b': {'x': 1, 'y': 2}, 'c': {'x': 1, 'y': 2}}"),
    ("d:\n  <<: {a: 1}\n  <<: {b: 2}", "{'d': {'<<': {'b': 2}}}", "{'d': {'a': 1, 'b': 2}}"),
    ("d:\n  <<: []", "{'d': {'<<': []}}", "{'d': {}}"),
    ("d:\n  <<: {}", "{'d': {'<<': {}}}", "{'d': {}}"),
    ("d:\n  <<: [[1]]", "{'d': {'<<': [[1]]}}", "FEHLER ConstructorError"),
    ("d:\n  <<: \"x\"", "{'d': {'<<': 'x'}}", "FEHLER ConstructorError"),
    ("<<: 1", "{'<<': 1}", "FEHLER ConstructorError"),
    ("- <<: {a: 1}\n  b: 2", "[{'<<': {'a': 1}, 'b': 2}]", "[{'a': 1, 'b': 2}]"),
    ("a: {x: 1}\nd:\n  <<: {y: 2}\n  x: 3", "{'a': {'x': 1}, 'd': {'<<': {'y': 2}, 'x': 3}}", "{'a': {'x': 1}, 'd': {'y': 2, 'x': 3}}"),
    ("d:\n  ? <<\n  : {a: 1}", "{'d': {'<<': {'a': 1}}}", "{'d': {'a': 1}}"),
    ("d:\n  !!merge <<: {a: 1}", "{'d': {'<<': {'a': 1}}}", "{'d': {'a': 1}}"),
    ("!!set {a, b}", "{'a': None, 'b': None}", "{'a', 'b'}"),
    ("!!set [a]", "['a']", "FEHLER ConstructorError"),
    ("!!set {a: 1}", "{'a': 1}", "{'a'}"),
    ("!!omap [a: 1, b: 2]", "[{'a': 1}, {'b': 2}]", "[('a', 1), ('b', 2)]"),
    ("!!omap [a, b]", "['a', 'b']", "FEHLER ConstructorError"),
    ("!!omap [a: 1, a: 2]", "[{'a': 1}, {'a': 2}]", "[('a', 1), ('a', 2)]"),
    ("!!pairs [a: 1, a: 2]", "[{'a': 1}, {'a': 2}]", "[('a', 1), ('a', 2)]"),
    ("!!str [1]", "[1]", "FEHLER ConstructorError"),
    ("!!seq 1", "'1'", "FEHLER ConstructorError"),
    ("!!map []", "[]", "FEHLER ConstructorError"),
    ("!!map 1", "'1'", "FEHLER ConstructorError"),
    ("!!python/tuple [1]", "[1]", "FEHLER ConstructorError"),
    ("!!binary |\n  aGVs\n  bG8=", "'aGVs\\nbG8='", "b'hello'"),
    ("!!binary YQ", "'YQ'", "FEHLER ConstructorError"),
    ("!!set\n? a\n? b", "{'a': None, 'b': None}", "{'a', 'b'}"),
    ("!!str\n- a", "['a']", "FEHLER ConstructorError"),
    ("!!float [1]", "[1]", "FEHLER ConstructorError"),
    ("!!int {a: 1}", "{'a': 1}", "FEHLER ConstructorError"),
    ("!<!> x", "FEHLER", "'x'"),
    ("!!!x y", "'y'", "FEHLER ConstructorError"),
    ("!!python/object/apply:os.system [x]", "['x']", "FEHLER ConstructorError"),
    ("!!python/object/new:str [x]", "['x']", "FEHLER ConstructorError"),
    ("!!python/unicode x", "'x'", "FEHLER ConstructorError"),
    ("!!python/bytes x", "'x'", "FEHLER ConstructorError"),
    ("!!python/complex 1+2j", "'1+2j'", "FEHLER ConstructorError"),
    ("!!python/module:os x", "'x'", "FEHLER ConstructorError"),
    ("!!python/long 1", "'1'", "FEHLER ConstructorError"),
    ("!!python/float 1.5", "'1.5'", "FEHLER ConstructorError"),
    ("!!python/dict {a: 1}", "{'a': 1}", "FEHLER ConstructorError"),
    ("!!python/list [1]", "[1]", "FEHLER ConstructorError"),
    ("!!python/str x", "'x'", "FEHLER ConstructorError"),
    ("!!python/bool true", "'true'", "FEHLER ConstructorError"),
    ("!!python/none ''", "''", "FEHLER ConstructorError"),
    ("!!js/function x", "'x'", "FEHLER ConstructorError"),
    ("!!java.util.Date x", "'x'", "FEHLER ConstructorError"),
    ("!!merge x", "'x'", "FEHLER ConstructorError"),
    ("!!value x", "'x'", "FEHLER ConstructorError"),
    ("!!yaml x", "'x'", "FEHLER ConstructorError"),
    ("!!null", "FEHLER", "None"),
    ("!!null 1", "FEHLER", "None"),
    ("!!timestamp", "''", "FEHLER AttributeError"),
    ("\"\\ud800\"", "FEHLER", "'\\ud800'"),
    ("\"\\udc00\"", "FEHLER", "'\\udc00'"),
    ("\"\\ud83d\\ude00\"", "FEHLER", "'\\ud83d\\ude00'"),
    ("\"\\U0000D800\"", "FEHLER", "'\\ud800'"),
    ("a\tb", "'a\\tb'", "FEHLER ScannerError"),
    ("a\t", "'a'", "FEHLER ScannerError"),
    ("[\t1]", "[1]", "FEHLER ScannerError"),
    ("{\ta: 1}", "{'a': 1}", "FEHLER ScannerError"),
    ("?", "{'': None}", "{None: None}"),
    ("? ", "{'': None}", "{None: None}"),
    ("!", "FEHLER", "None"),
    ("a: !", "FEHLER", "{'a': None}"),
];

/// Kontexte, in denen ein YAML-Skalar sich anders verhaelt als erwartet.
#[rustfmt::skip]
const YAML_KONTEXT_AUSNAHMEN: &[&str] = &["{0o17: v}", "{+.5: v}", "{1.5e3: v}", "{1.5E3: v}", "{1e3: v}", "{1E3: v}", "{1e+3: v}", "{1e-3: v}", "{1.e3: v}", "{.1e3: v}", "{1e-400: v}", "{5e-324: v}", "{1e22: v}", "{=: v}", "{...: v}"];

fn zaehle(name: &str, ms: &[Messung]) -> (usize, usize, usize) {
    let (mut gleich, mut beide_ab, mut abw) = (0, 0, 0);
    for m in ms {
        match (&m.abw, m.py_ok) {
            (Some(_), _) => abw += 1,
            (None, true) => gleich += 1,
            (None, false) => beide_ab += 1,
        }
    }
    println!(
        "{name}: {} Texte, gleich {gleich}, beide lehnen ab {beide_ab}, Abweichungen {abw}",
        ms.len()
    );
    (gleich, beide_ab, abw)
}

fn texte_aus(listen: &[&[&str]]) -> Vec<String> {
    eindeutig(
        listen
            .iter()
            .flat_map(|l| l.iter().map(|s| (*s).to_owned())),
    )
}

#[test]
fn lader_json_gegen_cpython() {
    if skip() {
        return;
    }
    let (skalare, kontext) = mit_kontexten("json", JSON_SKALARE, json_tauglich, json_kontexte);
    let (g1, a1, w1) = zaehle("json Skalare", &skalare);
    let mut fehler: Vec<String> = gleiche_pins("json Skalare", &skalare, JSON_SKALARE_ABW)
        .into_iter()
        .collect();
    let strukturen = messe("json", &texte_aus(&[JSON_STRUKTUREN]));
    let (g2, a2, w2) = zaehle("json Strukturen", &strukturen);
    fehler.extend(gleiche_pins(
        "json Strukturen",
        &strukturen,
        JSON_STRUKTUREN_ABW,
    ));
    let kontext: Vec<&str> = kontext.iter().map(String::as_str).collect();
    if kontext != JSON_KONTEXT_AUSNAHMEN {
        fehler.push(format!(
            "json Kontexte: gemessen {kontext:?}, festgehalten {JSON_KONTEXT_AUSNAHMEN:?}"
        ));
    }
    assert!(fehler.is_empty(), "{}", fehler.join("\n"));
    // Das Orakel sagt nicht nur eine Art von Antwort.
    assert!(
        g1 + g2 > 100 && a1 + a2 > 50 && w1 + w2 > 5,
        "{g1}+{g2} {a1}+{a2} {w1}+{w2}"
    );
}

#[test]
fn lader_yaml_gegen_cpython() {
    if skip() {
        return;
    }
    let (skalare, kontext) =
        mit_kontexten("yaml", YAML_SKALARE, flow_tauglich, yaml_kontexte_liste);
    let (g1, a1, w1) = zaehle("yaml Skalare", &skalare);
    let mut fehler: Vec<String> = gleiche_pins("yaml Skalare", &skalare, YAML_SKALARE_ABW)
        .into_iter()
        .collect();
    let strukturen = messe("yaml", &texte_aus(&[YAML_STRUKTUREN]));
    let (g2, a2, w2) = zaehle("yaml Strukturen", &strukturen);
    fehler.extend(gleiche_pins(
        "yaml Strukturen",
        &strukturen,
        YAML_STRUKTUREN_ABW,
    ));
    let kontext: Vec<&str> = kontext.iter().map(String::as_str).collect();
    if kontext != YAML_KONTEXT_AUSNAHMEN {
        fehler.push(format!(
            "yaml Kontexte: gemessen {kontext:?}, festgehalten {YAML_KONTEXT_AUSNAHMEN:?}"
        ));
    }
    assert!(fehler.is_empty(), "{}", fehler.join("\n"));
    assert!(
        g1 + g2 > 200 && a1 + a2 > 50 && w1 + w2 > 100,
        "{g1}+{g2} {a1}+{a2} {w1}+{w2}"
    );
}

/// Die Gleitkomma-Texte des Orakels (`wert.laden_gleit_texte`): kuerzeste Texte von Zufallsmustern,
/// 16 bis 30 Ziffern, exakte Mittelpunkte zweier Nachbarn und bekannte Randfaelle.
fn gleit_texte() -> Vec<String> {
    let a = frage(&json!({ "fn": "wert.laden_gleit_texte" }));
    a.as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_owned())
        .collect()
}

fn bits_aus_draht(d: &Value) -> u64 {
    u64::from_str_radix(d["f"].as_str().unwrap(), 16).unwrap()
}

/// Gleitkommatexte durch beide Lader. `json.loads` und `yaml.safe_load` runden immer richtig
/// (`float(text)`). Gemessen: beide Rust-Lader treffen dieselben Bits, ausser wenn der Text ueber
/// `f64::MAX` liegt: `CPython` liefert `inf`, `serde_json` lehnt ab, `serde_yaml_ng` liest einen Text.
#[test]
fn lader_gleitkomma_gegen_cpython() {
    if skip() {
        return;
    }
    let texte = gleit_texte();
    let inf = f64::INFINITY.to_bits();
    for (art, ueber_max_rust) in [("json", "lehnt ab"), ("yaml", "liest Text")] {
        let py = laden_cpython(art, &texte);
        let (mut gleich, mut daneben, mut ueber_max, mut falsch) = (0, 0, 0, Vec::new());
        for (t, p) in texte.iter().zip(&py) {
            let Ok((d, _)) = p else {
                panic!("{art}: CPython lehnt {t} ab");
            };
            let soll = bits_aus_draht(d);
            match (laden_rust(art, t), soll & !(1 << 63) == inf) {
                (Ok(Gleit(f)), _) if f.to_bits() == soll => gleich += 1,
                (Ok(Gleit(f)), _) => {
                    daneben += 1;
                    falsch.push(format!(
                        "{t}: Rust {f:e}, CPython {:e}",
                        f64::from_bits(soll)
                    ));
                }
                // Nur ein Text ueber `f64::MAX` darf so abweichen: dort sagt CPython `inf`.
                (Err(_) | Ok(Text(_)), true) => ueber_max += 1,
                (andere, _) => falsch.push(format!("{t}: Rust {andere:?}, CPython {soll:016x}")),
            }
        }
        println!(
            "{art} Gleitkomma: {} Texte, gleich {gleich}, eine Stelle oder mehr daneben {daneben}, \
             ueber f64::MAX ({ueber_max_rust}) {ueber_max}",
            texte.len()
        );
        assert!(falsch.is_empty(), "{art}: {falsch:#?}");
        assert!(
            gleich > 7000 && (1..10).contains(&ueber_max),
            "{art}: {gleich} {ueber_max}"
        );
    }
}

/// Die groesste Schachtelungstiefe `[[...]]`, die der Lader noch annimmt: `CPython` bis zur Grenze der
/// Suche (JSON ueber 1000, `PyYAML` etwa 490 -- von der Python-Version abhaengig), `serde_json` bis 127,
/// `serde_yaml_ng` bis 128 (beide: Rekursionsgrenze 128).
#[test]
fn lader_tiefe_gegen_cpython() {
    if skip() {
        return;
    }
    for (art, rust_soll) in [("json", 127), ("yaml", 128)] {
        let bis = 300;
        let py = frage(&json!({ "fn": "wert.laden_tiefe", "art": art, "bis": bis }));
        let py = usize::try_from(py["tiefste"].as_u64().unwrap()).unwrap();
        let rust = (1..=bis)
            .take_while(|d| {
                laden_rust(art, &format!("{}{}", "[".repeat(*d), "]".repeat(*d))).is_ok()
            })
            .count();
        println!("{art} Tiefe: CPython bis {py} (Suche endet bei {bis}), Rust bis {rust}");
        assert_eq!(py, bis, "{art}: CPython nimmt jede Tiefe bis {bis} an");
        assert_eq!(rust, rust_soll, "{art}");
    }
}

fn verfolgte_dateien(endungen: &[&str]) -> Vec<String> {
    let aus = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files");
    assert!(aus.status.success());
    String::from_utf8(aus.stdout)
        .expect("Pfade in UTF-8")
        .split('\0')
        .filter(|p| endungen.iter().any(|e| p.ends_with(e)))
        .map(str::to_owned)
        .collect()
}

/// Die erste Stelle, an der zwei Texte sich unterscheiden, mit Umgebung.
fn erste_stelle(rust: &str, python: &str) -> String {
    let i = rust
        .chars()
        .zip(python.chars())
        .take_while(|(a, b)| a == b)
        .count();
    let umgebung = |s: &str| {
        s.chars()
            .skip(i.saturating_sub(30))
            .take(70)
            .collect::<String>()
    };
    format!(
        "Zeichen {i}: Rust ...{}..., CPython ...{}...",
        umgebung(rust),
        umgebung(python)
    )
}

/// Die verfolgten YAML-Dateien, in denen `CPython` einen `int` als Schluessel laedt (`{20: 384}`): `PyWert`
/// kennt nur Text-Schluessel und legt `{"20": 384}` ab. Alle uebrigen Dateien laden gleich.
const ECHTE_YAML_ABWEICHUNGEN: &[&str] = &[
    "params/2024/behinderten_pauschbetrag_p33b.yaml",
    "params/2025/behinderten_pauschbetrag_p33b.yaml",
    "params/2026/behinderten_pauschbetrag_p33b.yaml",
    "params/kohorten/abzinsungsfaktor_5komma5_p6.yaml",
    "params/kohorten/altersentlastungsbetrag_p24a.yaml",
    "params/kohorten/ekfz_sonderafa_staffel_p7.yaml",
    "params/kohorten/rente_besteuerungsanteil_p22.yaml",
    "params/kohorten/rente_ertragsanteil_p22.yaml",
    "params/kohorten/versorgungsfreibetrag_p19_2.yaml",
];

/// Jede verfolgte `*.yaml`/`*.yml`- und `*.json`-Datei des Repos durch beide Lader. Das sind die Daten,
/// die Produktpfade wirklich laden (Parameter, Bindungen, Kohorten, goldene Faelle, Fixtures).
#[test]
fn lader_echte_dateien_gegen_cpython() {
    if skip() {
        return;
    }
    for (art, endungen, erwartet) in [
        ("yaml", &[".yaml", ".yml"][..], ECHTE_YAML_ABWEICHUNGEN),
        ("json", &[".json"][..], &[][..]),
    ] {
        let (mut pfade, mut texte) = (Vec::new(), Vec::new());
        for p in verfolgte_dateien(endungen) {
            if let Ok(t) = std::fs::read_to_string(repo_root().join(&p)) {
                pfade.push(p);
                texte.push(t);
            }
        }
        let a = frage(&json!({ "fn": "wert.laden_dateien", "art": art, "pfade": pfade }));
        let py = lade_antworten(&a, pfade.len());
        let (mut gleich, mut beide_ab) = (0, Vec::new());
        let (mut abweichend, mut stellen) = (Vec::new(), Vec::new());
        for ((p, t), a) in pfade.iter().zip(&texte).zip(&py) {
            let r = laden_rust(art, t);
            match abweichung(&r, a) {
                None if a.is_ok() => gleich += 1,
                None => beide_ab.push(p.clone()),
                Some(_) => {
                    abweichend.push(p.as_str());
                    stellen.push(match (&r, a) {
                        (Ok(w), Ok((_, pr))) => format!("{p}: {}", erste_stelle(&w.repr(), pr)),
                        (Ok(_), Err((k, _))) => format!("{p}: Rust laedt, CPython {k}"),
                        _ => format!("{p}: Rust lehnt ab, CPython laedt"),
                    });
                }
            }
        }
        println!(
            "{art} echte Dateien: {} gelesen, gleich {gleich}, beide lehnen ab {}, Abweichungen {}",
            pfade.len(),
            beide_ab.len(),
            abweichend.len()
        );
        assert!(beide_ab.is_empty(), "{art}: beide lehnen ab: {beide_ab:?}");
        assert!(
            abweichend == erwartet,
            "{art}: gemessen {} Abweichungen, festgehalten {}:\n{}\n{abweichend:#?}",
            abweichend.len(),
            erwartet.len(),
            stellen.join("\n")
        );
        // Die Abweichung ist nur der Schluessel: mit `int`-Schluesseln als Text laden alle Dateien gleich.
        let b = frage(&json!({
            "fn": "wert.laden_dateien", "art": art, "pfade": pfade, "schluessel_text": true
        }));
        let py_text = lade_antworten(&b, pfade.len());
        let rest: Vec<&str> = pfade
            .iter()
            .zip(&texte)
            .zip(&py_text)
            .filter(|((_, t), a)| abweichung(&laden_rust(art, t), a).is_some())
            .map(|((p, _), _)| p.as_str())
            .collect();
        assert!(
            rest.is_empty(),
            "{art}: auch mit Text-Schluesseln abweichend: {rest:?}"
        );
    }
}
