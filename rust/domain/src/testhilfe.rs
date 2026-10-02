//! Generatoren und Pruefungen fuer die Aequivalenz-Proptests der Alt-Helfer (D15).
//!
//! Jede Crate mit Python-Helfern auf `serde_json::Value` vergleicht sie in einem
//! `#[cfg(test)] mod aequivalenz` mit [`PyWert`]. Die Generatoren treffen jeden Rand, an dem ein
//! Alt-Helfer von `CPython` abweicht. Der Rand bleibt im Generator; die Ausnahmeliste des Helfers
//! nennt ihn mit D-Nummer. Nur mit Feature `testhilfe` (Dev-Abhaengigkeit), nie im Produkt.
use std::fmt::Debug;

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::select;
use proptest::test_runner::TestCaseError;
use serde::Deserialize as _;
use serde_json::{Number, Value};

use crate::py_text::{druckbar, ist_nd};
use crate::{py_strip, PyFehler, PyWert};

/// Ergebnis einer Python-Operation: Wert, Klasse der Python-Ausnahme oder `None` fuer eine
/// Rust-Grenze (wie [`PyFehler::python_klasse`]).
pub type Ergebnis<T> = Result<T, Option<&'static str>>;

const VE: Ergebnis<i64> = Err(Some("ValueError"));
const OE: Ergebnis<i64> = Err(Some("OverflowError"));

/// `str.isspace()`: die 25 Zeichen von `str::trim` und U+001C..U+001F (D7, D16).
const LEERRAUM: &[char] = &[
    '\t', '\n', '\u{b}', '\u{c}', '\r', '\u{1c}', '\u{1d}', '\u{1e}', '\u{1f}', ' ', '\u{85}',
    '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
    '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}',
    '\u{205f}', '\u{3000}',
];

/// Die Null je Schrift (Nd, je zehn Ziffern am Stueck): arabisch-indisch, erweitert
/// arabisch-indisch, Devanagari, Vollbreite, mathematisch fett (D6).
const ND_NULL: &[char] = &['\u{660}', '\u{6f0}', '\u{966}', '\u{ff10}', '\u{1d7ce}'];

/// Rumpf ohne Vorzeichen an den i64-Grenzen (D12), auch mit `_` und fuehrenden Nullen.
const GRENZ_TEXTE: &[&str] = &[
    "9223372036854775807",
    "9223372036854775808",
    "9223372036854775809",
    "9_223_372_036_854_775_808",
    "0009223372036854775808",
    "18446744073709551615",
    "18446744073709551616",
    "0",
    "00",
    "3",
    "4",
];

/// Formen, die `int()` ablehnt oder nur mit passendem Vorzeichen annimmt.
const UNGUELTIG: &[&str] = &[
    "",
    "_",
    "_1",
    "1_",
    "1__0",
    "x",
    "1.5",
    "+1",
    "0x10",
    "1e3",
    "1 2",
    "\u{663}_\u{664}",
];

const VORZEICHEN: &[&str] = &["", "+", "-"];
const FUELLUNG: &[&str] = &["0", "7"];

/// Zeichen fuer [`text`]: Buchstaben, Anfuehrungszeichen, Backslash, Cc, Zs/Zl, Cf (escaped und
/// nicht escaped), Co, Cn, ein Emoji, Ziffern und U+001C..U+001F (D7, D9).
const ZEICHEN: &[char] = &[
    'a',
    'Z',
    '\u{e4}',
    ' ',
    '\'',
    '"',
    '\\',
    '\n',
    '\t',
    '\r',
    '\0',
    '\u{7}',
    '\u{7f}',
    '\u{85}',
    '\u{a0}',
    '\u{ad}',
    '\u{200b}',
    '\u{2028}',
    '\u{3000}',
    '\u{feff}',
    '\u{600}',
    '\u{2066}',
    '\u{e000}',
    '\u{378}',
    '\u{1f600}',
    '\u{10ffff}',
    '\u{1c}',
    '\u{1f}',
    '1',
    '\u{663}',
    '_',
    '-',
];

/// Float-Raender: -2^63 (D11), 2^63 und Nachbarn, 1,8e19 bis 2^64 (D14), um und ueber
/// `Decimal::MAX` (D18), -0.0, Bruchteile um 0 und 3, Wechsel der `repr`-Form bei 1e16 und 1e-4.
const GRENZ_FLOATS: &[f64] = &[
    -9_223_372_036_854_775_808.0,
    9_223_372_036_854_775_808.0,
    -9_223_372_036_854_777_856.0,
    9_223_372_036_854_774_784.0,
    1.8e19,
    18_446_744_073_709_549_568.0,
    18_446_744_073_709_551_616.0,
    9e20,
    1e21,
    7.9e28,
    8e28,
    -8e28,
    1e29,
    1e300,
    -0.0,
    0.0,
    0.5,
    -0.5,
    2.7,
    -2.7,
    3.0,
    3.5,
    4.0,
    1e16,
    1e-5,
    5e-324,
];

/// Ganzzahl-Raender: i64-Grenzen, 3/4 (`> 3`), 100 (`// 100`).
const GRENZ_GANZ: &[i64] = &[
    i64::MIN,
    i64::MIN + 1,
    i64::MAX,
    -1,
    0,
    1,
    3,
    4,
    100,
    1000,
    1001,
];

/// Ganzzahlen ueber `i64::MAX` (D3): 2^63, `u64::MAX`, 2^64 - 2048 und 1,8e19 (D14).
const GRENZ_GROSS: &[u64] = &[
    1 << 63,
    u64::MAX,
    18_446_744_073_709_549_568,
    18_000_000_000_000_000_000,
];

/// Schluessel, deren Sortierung von der Einfuegereihenfolge abweichen kann (D8), mit
/// Anfuehrungszeichen und escaptem Leerraum (D9).
const SCHLUESSEL: &[&str] = &["b", "a", "z", "A", "\u{e4}", "'", "\"", "1", "", "\u{a0}"];

/// Eine Ziffer in ASCII oder einer Schrift aus [`ND_NULL`].
fn ziffer() -> impl Strategy<Value = char> {
    let ascii = (0u32..10).prop_map(|w| char::from_digit(w, 10).unwrap_or('0'));
    let nd = (select(ND_NULL), 0u32..10)
        .prop_map(|(null, w)| char::from_u32(u32::from(null) + w).unwrap_or(null));
    prop_oneof![5 => ascii, 1 => nd]
}

/// Ganzzahl-Text fuer `int()`: Leerraum am Rand (D7, D16), Vorzeichen, Ziffern jeder Schrift mit
/// `_` (D6), die i64-Grenzen (D12), Texte um die 4300-Ziffern-Grenze (D17) und ungueltige Formen.
///
/// ```
/// use proptest::strategy::{Strategy, ValueTree};
/// let mut lauf = proptest::test_runner::TestRunner::deterministic();
/// let t = domain::testhilfe::ganzzahl_text().new_tree(&mut lauf).unwrap().current();
/// assert!(t.chars().count() < 4320);
/// ```
pub fn ganzzahl_text() -> impl Strategy<Value = String> {
    let rand = || vec(select(LEERRAUM), 0..3).prop_map(String::from_iter);
    let ziffern = vec((ziffer(), 0u8..8), 1..22).prop_map(|zs| {
        zs.into_iter().fold(String::new(), |mut s, (z, trenner)| {
            if trenner == 0 {
                s.push('_');
            }
            s.push(z);
            s
        })
    });
    let rumpf = prop_oneof![
        6 => ziffern,
        2 => select(GRENZ_TEXTE).prop_map(str::to_owned),
        2 => (4295usize..4307, select(FUELLUNG))
            .prop_map(|(n, fuell)| format!("{}5", fuell.repeat(n - 1))),
        1 => select(UNGUELTIG).prop_map(str::to_owned),
    ];
    (rand(), select(VORZEICHEN), rumpf, rand())
        .prop_map(|(links, zeichen, rumpf, rechts)| format!("{links}{zeichen}{rumpf}{rechts}"))
}

/// Text: aus [`ZEICHEN`], ein [`ganzzahl_text`] oder beliebiges Unicode.
///
/// ```
/// use proptest::strategy::{Strategy, ValueTree};
/// let mut lauf = proptest::test_runner::TestRunner::deterministic();
/// let _: String = domain::testhilfe::text().new_tree(&mut lauf).unwrap().current();
/// ```
pub fn text() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => vec(select(ZEICHEN), 0..8).prop_map(String::from_iter),
        3 => ganzzahl_text(),
        1 => any::<String>(),
    ]
}

/// JSON-Zahl: i64, Ganzzahlen ueber `i64::MAX` (D3), endliche Floats mit den Raendern aus
/// [`GRENZ_FLOATS`].
///
/// ```
/// use proptest::strategy::{Strategy, ValueTree};
/// let mut lauf = proptest::test_runner::TestRunner::deterministic();
/// let n = domain::testhilfe::zahl().new_tree(&mut lauf).unwrap().current();
/// assert!(n.as_f64().is_some_and(f64::is_finite));
/// ```
pub fn zahl() -> impl Strategy<Value = Number> {
    prop_oneof![
        3 => (-5i64..=5).prop_map(Number::from),
        2 => any::<i64>().prop_map(Number::from),
        2 => select(GRENZ_GANZ).prop_map(Number::from),
        2 => ((1u64 << 63)..=u64::MAX).prop_map(Number::from),
        1 => select(GRENZ_GROSS).prop_map(Number::from),
        3 => select(GRENZ_FLOATS).prop_filter_map("endlich", Number::from_f64),
        2 => (-10.0f64..10.0).prop_filter_map("endlich", Number::from_f64),
        2 => any::<f64>().prop_filter_map("endlich", Number::from_f64),
    ]
}

/// Endliche `f64` fuer `repr(float)`: beliebige Bitmuster, das Festkomma-Fenster von `CPython`
/// (`1e-4 <= |f| < 1e16`) und die exakten Gleichstaende `n + j/2^m` (`n` ab 2^30, `j` ungerade, `m`
/// 2..=8). Dort liegt der Wert genau zwischen zwei gleich kurzen Dezimalzahlen; `CPython` nimmt die
/// gerade Ziffer, `{:e}` von `std` rundet auf. Zufaellige Bitmuster treffen sie in 0,03 % der Faelle.
///
/// ```
/// use proptest::strategy::{Strategy, ValueTree};
/// let mut lauf = proptest::test_runner::TestRunner::deterministic();
/// assert!(domain::testhilfe::gleitzahl().new_tree(&mut lauf).unwrap().current().is_finite());
/// ```
pub fn gleitzahl() -> impl Strategy<Value = f64> {
    let gleichstand = ((1i64 << 30)..(1i64 << 53), 2u32..=8, any::<u8>(), any::<bool>()).prop_map(
        |(n, m, j, minus)| {
            let j = f64::from(u32::from(j) % (1 << (m - 1)) * 2 + 1);
            // Ganzzahl-Teil `n` (bis 2^53) und Bruch `j/2^m` sind in `f64` exakt, ihre Summe auch.
            #[allow(clippy::cast_precision_loss, reason = "n < 2^53 ist exakt")]
            let x = n as f64 + j / f64::from(1u32 << m);
            if minus { -x } else { x }
        },
    );
    prop_oneof![
        2 => any::<f64>().prop_filter("endlich", |f| f.is_finite()),
        2 => -1e16f64..1e16,
        1 => 1e-6f64..1e-3,
        4 => gleichstand,
    ]
}

/// `neu` ist `repr(f)` nach `CPython`. Das Orakel ist ryu (`serde_json`), das die kuerzesten Ziffern wie
/// `CPython` waehlt, auch beim exakten Gleichstand (gemessen: 0 Abweichungen auf 541 848 Werten):
/// gleiche Ziffern wie ryu, Umlauf `neu` -> `f`, und im Festkomma-Fenster (`1e-4 <= |f| < 1e16`,
/// `0.0`) derselbe Text wie ryu. `nan`, `inf` und `-inf` stehen fest.
///
/// ```
/// assert!(domain::testhilfe::pruefe_repr_float(1e16, "1e+16").is_ok());
/// assert!(domain::testhilfe::pruefe_repr_float(-1_409_149_049_912_713.25, "-1409149049912713.2").is_ok());
/// assert!(domain::testhilfe::pruefe_repr_float(-1_409_149_049_912_713.25, "-1409149049912713.3").is_err());
/// assert!(domain::testhilfe::pruefe_repr_float(f64::NAN, "nan").is_ok());
/// ```
///
/// # Errors
/// Ein Proptest-Fehlschlag mit `f` und beiden Texten.
pub fn pruefe_repr_float(f: f64, neu: &str) -> Result<(), TestCaseError> {
    let Some(ryu) = Number::from_f64(f).map(|n| n.to_string()) else {
        let fest = if f.is_nan() { "nan" } else if f > 0.0 { "inf" } else { "-inf" };
        prop_assert_eq!(neu, fest, "{:?}", f);
        return Ok(());
    };
    let ziffern = |t: &str| {
        let mantisse = t.split('e').next().unwrap_or("");
        mantisse.replace(['-', '.'], "").trim_matches('0').to_owned()
    };
    prop_assert_eq!(ziffern(neu), ziffern(&ryu), "Ziffern von {:?}: neu {:?}, ryu {:?}", f, neu, ryu);
    prop_assert_eq!(neu.parse::<f64>().ok(), Some(f), "Umlauf von {:?}: neu {:?}", f, neu);
    if f == 0.0 || (1e-4..1e16).contains(&f.abs()) {
        prop_assert_eq!(neu, ryu.as_str(), "Festkomma von {:?}", f);
    }
    Ok(())
}

/// JSON-Wert: `null`, `bool`, [`zahl`], [`text`], Listen und Objekte bis Tiefe 3; oben in
/// mindestens der Haelfte der Faelle ein Skalar.
///
/// ```
/// use proptest::strategy::{Strategy, ValueTree};
/// let mut lauf = proptest::test_runner::TestRunner::deterministic();
/// let v = domain::testhilfe::json_wert().new_tree(&mut lauf).unwrap().current();
/// let _ = domain::testhilfe::py(&v);
/// ```
pub fn json_wert() -> impl Strategy<Value = Value> {
    let blatt = || {
        prop_oneof![
            1 => Just(Value::Null),
            1 => any::<bool>().prop_map(Value::Bool),
            4 => zahl().prop_map(Value::Number),
            4 => text().prop_map(Value::String),
        ]
    };
    let baum = blatt().prop_recursive(3, 24, 4, |innen| {
        prop_oneof![
            vec(innen.clone(), 0..4).prop_map(Value::Array),
            vec((select(SCHLUESSEL), innen), 0..4).prop_map(|paare| {
                Value::Object(paare.into_iter().map(|(k, w)| (k.to_owned(), w)).collect())
            }),
        ]
    });
    // `prop_recursive` waehlt oben zu 90 % einen Container, und `int()` sieht dort nur
    // `TypeError`. Der Skalar-Zweig hebt die Laeufe mit -2^63 oben (D11 in llm) von rechnerisch
    // 24 % auf 78 %, gemessen 6 und 16 von 20.
    prop_oneof![1 => blatt(), 1 => baum]
}

/// `v` als `PyWert` ueber dessen `Deserialize`. Objekte haben die sortierte Reihenfolge von `Value`.
///
/// # Panics
/// Nie: `PyWert` nimmt jeden JSON-Wert an.
///
/// ```
/// let w = domain::testhilfe::py(&serde_json::json!({"b": 1, "a": 2}));
/// assert_eq!(w.repr(), "{'a': 2, 'b': 1}");
/// ```
#[must_use]
#[allow(clippy::expect_used, reason = "PyWert nimmt jeden JSON-Wert an")]
pub fn py(v: &Value) -> PyWert {
    PyWert::deserialize(v).expect("PyWert nimmt jeden JSON-Wert an")
}

/// Wie [`py`], aber jedes Objekt in absteigender Schluessel-Reihenfolge: so steht `v` in einer
/// Datei, deren Reihenfolge `json.loads` und `PyWert` behalten und `Value` sortiert (D8).
///
/// ```
/// let w = domain::testhilfe::py_absteigend(&serde_json::json!({"a": [{"x": 1, "y": 2}], "b": 3}));
/// assert_eq!(w.repr(), "{'b': 3, 'a': [{'y': 2, 'x': 1}]}");
/// ```
#[must_use]
pub fn py_absteigend(v: &Value) -> PyWert {
    fn umkehren(w: PyWert) -> PyWert {
        match w {
            PyWert::Liste(l) => PyWert::Liste(l.into_iter().map(umkehren).collect()),
            PyWert::Objekt(o) => {
                PyWert::Objekt(o.into_iter().rev().map(|(k, w)| (k, umkehren(w))).collect())
            }
            w => w,
        }
    }
    umkehren(py(v))
}

/// `r` mit der Python-Klasse statt des [`PyFehler`].
///
/// ```
/// use domain::{testhilfe::klasse, PyWert};
/// assert_eq!(klasse(PyWert::Null.int()), Err(Some("TypeError")));
/// assert_eq!(klasse(PyWert::GrossGanz(u64::MAX).int()), Err(None));
/// ```
///
/// # Errors
/// Die Python-Klasse des Fehlers, `None` fuer eine Rust-Grenze.
pub fn klasse<T>(r: Result<T, PyFehler>) -> Ergebnis<T> {
    r.map_err(|e| e.python_klasse())
}

/// Vergleicht einen Alt-Helfer mit seinem `PyWert`-Ersatz. Gleich, oder jede D-Nummer aus
/// `ausnahmen` steht in `liste`. Eine Abweichung ohne D-Nummer ist ein Befund: melden, nicht
/// eintragen.
///
/// ```
/// use domain::testhilfe::pruefe;
/// assert!(pruefe(&1, &1, &1, Vec::new, &[]).is_ok());
/// assert!(pruefe(&1, &1, &2, || vec!["D3"], &["D3"]).is_ok());
/// assert!(pruefe(&1, &1, &2, || vec!["D3", "D16"], &["D3"]).is_err());
/// assert!(pruefe(&1, &1, &2, Vec::new, &["D3"]).is_err());
/// ```
///
/// # Errors
/// Ein Proptest-Fehlschlag mit Eingabe, beiden Ergebnissen und den D-Nummern.
pub fn pruefe<T: PartialEq + Debug>(
    eingabe: &dyn Debug,
    alt: &T,
    neu: &T,
    ausnahmen: impl FnOnce() -> Vec<&'static str>,
    liste: &[&str],
) -> Result<(), TestCaseError> {
    if alt == neu {
        return Ok(());
    }
    let d = ausnahmen();
    if d.is_empty() {
        Err(TestCaseError::fail(format!(
            "Abweichung ohne D-Nummer (Befund): {eingabe:?}, alt {alt:?}, neu {neu:?}"
        )))
    } else if d.iter().all(|x| liste.contains(x)) {
        Ok(())
    } else {
        Err(TestCaseError::fail(format!(
            "{d:?} nicht alle in der Ausnahmeliste {liste:?}: {eingabe:?}, alt {alt:?}, neu {neu:?}"
        )))
    }
}

/// D16: `s` hat nach `str::trim` U+001C..U+001F am Rand; `str.strip()` entfernt sie, `int()` nicht.
///
/// ```
/// assert!(domain::testhilfe::rand_steuer(" \u{1c}42"));
/// assert!(!domain::testhilfe::rand_steuer("4\u{1c}2"));
/// ```
#[must_use]
pub fn rand_steuer(s: &str) -> bool {
    let steuer = |c: char| ('\u{1c}'..='\u{1f}').contains(&c);
    s.trim().starts_with(steuer) || s.trim().ends_with(steuer)
}

/// D6: `s` enthaelt eine Dezimalziffer (Nd) ausserhalb von ASCII.
///
/// ```
/// assert!(domain::testhilfe::nd_ziffer("1\u{663}"));
/// assert!(!domain::testhilfe::nd_ziffer("13\u{b2}"));
/// ```
#[must_use]
pub fn nd_ziffer(s: &str) -> bool {
    s.chars().any(|c| !c.is_ascii_digit() && ist_nd(c))
}

/// D17: mehr als 4300 Dezimalziffern (`sys.get_int_max_str_digits()`).
///
/// ```
/// assert!(domain::testhilfe::viele_ziffern(&"0".repeat(4301)));
/// assert!(!domain::testhilfe::viele_ziffern(&"0".repeat(4300)));
/// ```
#[must_use]
pub fn viele_ziffern(s: &str) -> bool {
    s.chars()
        .filter(|c| c.is_ascii_digit() || ist_nd(*c))
        .count()
        > 4300
}

/// `s` mit den Ziffern der Schriften aus [`ND_NULL`] als ASCII-Ziffern: `int()` liest beide gleich
/// (D6). Unabhaengig von `PyWert`, weil der Generator den Ziffernwert kennt.
///
/// ```
/// assert_eq!(domain::testhilfe::ascii_fassung("-\u{663}\u{ff17}"), "-37");
/// ```
#[must_use]
pub fn ascii_fassung(s: &str) -> String {
    s.chars()
        .map(|c| {
            ND_NULL
                .iter()
                .find_map(|&null| {
                    let wert = u32::from(c).checked_sub(u32::from(null))?;
                    char::from_digit(wert, 10)
                })
                .unwrap_or(c)
        })
        .collect()
}

/// D11: der Float -2^63.
///
/// ```
/// assert!(domain::testhilfe::minus_2_hoch_63(&serde_json::json!(-9_223_372_036_854_775_808.0)));
/// assert!(!domain::testhilfe::minus_2_hoch_63(&serde_json::json!(i64::MIN)));
/// ```
#[must_use]
pub fn minus_2_hoch_63(v: &Value) -> bool {
    v.is_f64() && v.as_f64().map(f64::to_bits) == Some((-9_223_372_036_854_775_808.0f64).to_bits())
}

/// D3: eine Ganzzahl ueber `i64::MAX`.
///
/// ```
/// assert!(domain::testhilfe::gross_ganz(&serde_json::json!(u64::MAX)));
/// assert!(!domain::testhilfe::gross_ganz(&serde_json::json!(i64::MAX)));
/// ```
#[must_use]
pub fn gross_ganz(v: &Value) -> bool {
    v.is_u64() && !v.is_i64()
}

/// D3 als Ausnahme: `v` ist eine Ganzzahl ueber `i64::MAX`, und `PyWert` meldet die i64-Grenze.
/// Die Alt-Helfer antworten dort verschieden (Saettigung, `None`, `true`).
///
/// ```
/// use domain::testhilfe::{d3, Ergebnis};
/// let grenze: Ergebnis<i64> = Err(None);
/// assert_eq!(d3(&serde_json::json!(u64::MAX), &grenze), ["D3"]);
/// assert!(d3(&serde_json::json!(i64::MAX), &grenze).is_empty());
/// ```
#[must_use]
pub fn d3<T>(v: &Value, neu: &Ergebnis<T>) -> Vec<&'static str> {
    if gross_ganz(v) && matches!(neu, Err(None)) {
        vec!["D3"]
    } else {
        Vec::new()
    }
}

/// D3 und D10 zusammen: `v` ist eine Ganzzahl ueber `i64::MAX` (D3) oder ein Float (D10). Beide
/// D-Nummern stehen fuer dasselbe Muster: der Alt-Helfer sammelt auf `i64` ein und liefert `None`,
/// waehrend `CPython` weiterrechnet (`0 + wert`, `wert // 100`) und damit ein Ergebnis hat.
///
/// ```
/// use domain::testhilfe::d3_d10;
/// assert_eq!(d3_d10(&serde_json::json!(u64::MAX)), ["D3"]);
/// assert_eq!(d3_d10(&serde_json::json!(1500.0)), ["D10"]);
/// assert!(d3_d10(&serde_json::json!(1500)).is_empty());
/// ```
#[must_use]
pub fn d3_d10(v: &Value) -> Vec<&'static str> {
    match (gross_ganz(v), v.is_f64()) {
        (true, _) => vec!["D3"],
        (_, true) => vec!["D10"],
        _ => Vec::new(),
    }
}

/// D9: ein Text oder Schluessel in `v` enthaelt ein Zeichen, das `repr` escapet, obwohl es kein
/// Steuerzeichen (Cc) ist.
///
/// ```
/// assert!(domain::testhilfe::hat_d9(&serde_json::json!({"\u{a0}": 1})));
/// assert!(!domain::testhilfe::hat_d9(&serde_json::json!(["\n", "a"])));
/// ```
#[must_use]
pub fn hat_d9(v: &Value) -> bool {
    let d9 = |s: &str| s.chars().any(|c| !c.is_control() && !druckbar(c));
    match v {
        Value::String(s) => d9(s),
        Value::Array(l) => l.iter().any(hat_d9),
        Value::Object(o) => o.iter().any(|(k, w)| d9(k) || hat_d9(w)),
        _ => false,
    }
}

/// `v` mit `§` statt jedes Zeichens aus [`hat_d9`]: das schreibt jedes `repr` roh, D9 faellt weg.
///
/// ```
/// let v = domain::testhilfe::ohne_d9(&serde_json::json!({"\u{a0}": ["a\u{2028}\n"]}));
/// assert_eq!(v, serde_json::json!({"§": ["a§\n"]}));
/// ```
#[must_use]
pub fn ohne_d9(v: &Value) -> Value {
    let ersetze = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c.is_control() || druckbar(c) {
                    c
                } else {
                    '§'
                }
            })
            .collect()
    };
    match v {
        Value::String(s) => Value::String(ersetze(s)),
        Value::Array(l) => Value::Array(l.iter().map(ohne_d9).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, w)| (ersetze(k), ohne_d9(w))).collect())
        }
        w => w.clone(),
    }
}

/// Die D-Nummern, unter denen ein Alt-Helfer fuer `int(v)` `alt` liefert, wo [`PyWert::int`]
/// `neu` liefert. Jede Regel prueft Eingabe und Ergebnispaar; leer heisst Befund.
///
/// ```
/// use domain::testhilfe::int_ausnahmen;
/// let ve = Err(Some("ValueError"));
/// assert_eq!(int_ausnahmen(&serde_json::json!("\u{1c}42"), &Ok(42), &ve), ["D16"]);
/// assert_eq!(int_ausnahmen(&serde_json::json!("\u{1c}\u{663}"), &ve, &ve), ["D6", "D16"]);
/// assert!(int_ausnahmen(&serde_json::json!("42"), &Ok(41), &Ok(42)).is_empty());
/// ```
#[must_use]
pub fn int_ausnahmen(v: &Value, alt: &Ergebnis<i64>, neu: &Ergebnis<i64>) -> Vec<&'static str> {
    let text = v.as_str();
    if let Some(s) = text.filter(|s| rand_steuer(s)) {
        // D16: der Alt-Helfer streift U+001C..U+001F mit ab und liest dann wie `int(s.strip())`.
        if *neu != VE {
            return Vec::new();
        }
        let ohne = Value::from(py_strip(s));
        let neu_ohne = klasse(py(&ohne).int());
        let mut rest = if *alt == neu_ohne {
            Vec::new()
        } else {
            int_ausnahmen(&ohne, alt, &neu_ohne)
        };
        if *alt == neu_ohne || !rest.is_empty() {
            rest.push("D16");
        }
        return rest;
    }
    let d = if text.is_some_and(viele_ziffern) && *neu == VE {
        "D17"
    } else if text.is_some_and(|s| {
        nd_ziffer(s) && *alt == VE && *neu == klasse(py(&Value::from(ascii_fassung(s))).int())
    }) {
        "D6"
    } else if minus_2_hoch_63(v) && *alt == Err(None) && *neu == Ok(i64::MIN) {
        "D11"
    } else if text.is_some() && *alt == OE && *neu == Ok(i64::MIN) {
        "D12"
    } else if *alt == OE && *neu == Err(None) {
        "D4"
    } else {
        return Vec::new();
    };
    vec![d]
}

#[cfg(test)]
mod tests {
    use super::{ist_nd, LEERRAUM, ND_NULL};

    #[test]
    fn leerraum_ist_isspace() {
        let isspace = |c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c);
        let alle: Vec<char> = (0..=0x10_ffff)
            .filter_map(char::from_u32)
            .filter(|c| isspace(*c))
            .collect();
        assert_eq!(LEERRAUM, alle);
    }

    #[test]
    fn nd_null_beginnt_zehn_ziffern() {
        for null in ND_NULL {
            let start = u32::from(*null);
            assert!((start..start + 10).filter_map(char::from_u32).all(ist_nd));
            assert!(char::from_u32(start - 1).is_some_and(|c| !ist_nd(c)));
        }
    }
}
