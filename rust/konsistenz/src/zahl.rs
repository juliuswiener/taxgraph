//! Python-Wertsemantik über Store-Werten, soweit die Prüfungen sie brauchen.
//!
//! Die Produktion liest [`PyWert`] — dort lebt Pythons Semantik (`int_ohne_bool`, `zahl_ohne_bool`,
//! `gt_null`). Die Vor-K2-Fassung über `serde_json::Value` steht je Helfer als `*_alt` daneben: sie
//! ist die Referenz, gegen die `mod aequivalenz` misst, und sie hält die D-Liste lebendig
//! (Auflage 1 — die Falle für den ersten Fall, der sie trifft).
use domain::PyWert;
use serde_json::Value;

/// `isinstance(w, int) and not isinstance(w, bool)`, beschränkt auf `i64`.
///
/// `int_ohne_bool` bildet beides ab; eine Ganzzahl über `i64::MAX` meldet `I64Grenze` (D3) und
/// zählt hier wie in der Alt-Fassung als „keine Ganzzahl".
pub(crate) fn ganzzahl(w: &PyWert) -> Option<i64> {
    w.int_ohne_bool().ok().flatten()
}

/// `isinstance(w, (int, float)) and not isinstance(w, bool) and w > 0` — über alle Zahlformen,
/// auch Floats und Ganzzahlen über `i64::MAX`.
pub(crate) fn zahl_gt0(w: &PyWert) -> bool {
    w.zahl_ohne_bool().is_some_and(|z| z.gt_null() == Ok(true))
}

/// `w == 0` für eine Zahl (Python: `0 == 0.0 == -0.0`). `false == 0` behandelt der Aufrufer.
pub(crate) fn zahl_gleich_null(w: &PyWert) -> bool {
    w.zahl_ohne_bool()
        .is_some_and(|z| z.py_eq(&PyWert::Ganz(0)))
}

/// `isinstance(w, str)` — Pythons Typtest, nicht `truthy`: der leere Text ist ein `str`.
pub(crate) fn als_text(w: &PyWert) -> Option<&str> {
    match w {
        PyWert::Text(s) => Some(s),
        _ => None,
    }
}

/// Store-Wert → JSON, für die Ausgabetypen dieses Crates.
///
/// Die Ausgabetypen bleiben `serde_json::Value`: `parity` baut ihr JSON von Hand
/// (`json!({"wert": w.wert})`), `bescheid` liest nur `.is_empty()`. Auflage 2 verbietet ein
/// `Serialize` für `PyWert`, also konvertiert jede Prüfung EINMAL an ihrer Konstruktionsstelle.
///
/// Die Konvertierung kann nach Auflage 3 nicht scheitern: `store::Store::append` weist NaN/inf
/// (auch in `Liste`/`Objekt`) an der Append-Grenze ab, und `serde_json` lehnt `1e999` beim Laden
/// als `NumberOutOfRange` ab. Ein Wert aus einer Fallakte ist damit immer darstellbar.
///
/// ponytail: `expect` statt Fehler-Rückgabe, weil kein Aufrufer einen Fehler tragen kann.
/// `Value::Null` wäre an dieser Stelle die verbotene stille Konvertierung — der Widerspruch
/// behauptete dann einen Wert, den er nicht hat. Ein Absturz ist bei einer Steuererklärung das
/// kleinere Übel. Upgrade: `Result` in den Ausgabetypen, falls je ein Pfad entsteht, der
/// ungeprüft in den Store schreibt.
#[allow(
    clippy::expect_used,
    reason = "Auflage 3 an der Append-Grenze macht den Fehler unerreichbar"
)]
pub(crate) fn als_json(w: &PyWert) -> Value {
    w.zu_json()
        .expect("Store-Wert ist darstellbar (Auflage 3 an der Append-Grenze)")
}

/// `str.isspace()` je Zeichen. Rusts `char::is_whitespace` kennt `\x1c`..`\x1f` nicht, Python
/// schon (Unicode-Bidi-Klasse B/S).
fn py_isspace(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `not s.strip()` — leer oder nur Leerraum im Sinn von Python.
pub(crate) fn leer_nach_strip(s: &str) -> bool {
    s.chars().all(py_isspace)
}

/// Cent → „12.123.213 €" (`preflight.py:84-86`: `f"{cent // 100:,}".replace(",", ".") + " €"`).
/// `//` rundet gegen −∞, deshalb `div_euclid` (−150 Cent → „-2 €", wie Python).
///
/// ```
/// assert_eq!(konsistenz::eur(1_212_321_300), "12.123.213 €");
/// assert_eq!(konsistenz::eur(99), "0 €");
/// assert_eq!(konsistenz::eur(-150), "-2 €");
/// ```
#[must_use]
pub fn eur(cent: i64) -> String {
    let euro = cent.div_euclid(100);
    let ziffern = euro.unsigned_abs().to_string();
    let mut gruppiert = String::with_capacity(ziffern.len() + ziffern.len() / 3 + 4);
    if euro < 0 {
        gruppiert.push('-');
    }
    let n = ziffern.len();
    for (i, c) in ziffern.chars().enumerate() {
        if i > 0 && (n - i).is_multiple_of(3) {
            gruppiert.push('.');
        }
        gruppiert.push(c);
    }
    gruppiert.push_str(" €");
    gruppiert
}

// ---------------------------------------------------------------- Vor-K2-Referenz (nur Tests)

/// Die Fassung vor dem K2-Port: `Value::as_i64` liefert für `true` schon `None`, weil JSON `bool`
/// von Zahlen trennt.
#[cfg(test)]
pub(crate) fn ganzzahl_alt(v: &Value) -> Option<i64> {
    v.as_i64()
}

/// Die Fassung vor dem K2-Port: exakt über alle JSON-Zahlen, auch Floats und Ganzzahlen über
/// `i64::MAX`.
#[cfg(test)]
pub(crate) fn zahl_gt0_alt(v: &Value) -> bool {
    let Value::Number(n) = v else { return false };
    if let Some(i) = n.as_i64() {
        i > 0
    } else if n.as_u64().is_some() {
        true
    } else {
        n.as_f64().is_some_and(|f| f > 0.0)
    }
}

/// Die Fassung vor dem K2-Port: nur JSON-Zahlen, `false` überlässt sie dem Aufrufer.
#[cfg(test)]
pub(crate) fn zahl_gleich_null_alt(v: &Value) -> bool {
    matches!(v, Value::Number(n) if n.as_f64() == Some(0.0))
}

#[cfg(test)]
mod tests {
    use super::{als_json, eur, leer_nach_strip, zahl_gleich_null, zahl_gt0};
    use domain::testhilfe::py;
    use serde_json::json;

    /// Die Ausgabegrenze ist total für alles, was aus einer Fallakte kommen kann.
    #[test]
    fn als_json_trifft_die_store_formen() {
        for v in [
            json!(null),
            json!(true),
            json!(-5),
            json!(u64::MAX),
            json!(2.5),
            json!("x"),
        ] {
            assert_eq!(als_json(&py(&v)), v);
        }
    }

    #[test]
    fn eur_gruppiert_wie_python() {
        assert_eq!(eur(0), "0 €");
        assert_eq!(eur(100_000), "1.000 €");
        assert_eq!(eur(-100_000), "-1.000 €");
        assert_eq!(eur(i64::MIN), "-92.233.720.368.547.759 €");
        assert_eq!(eur(i64::MAX), "92.233.720.368.547.758 €");
    }

    #[test]
    fn zahlsemantik() {
        assert!(!zahl_gt0(&py(&json!(true))));
        assert!(zahl_gt0(&py(&json!(0.5))));
        assert!(zahl_gt0(&py(&json!(u64::MAX))));
        assert!(zahl_gleich_null(&py(&json!(-0.0))));
        assert!(!zahl_gleich_null(&py(&json!(false))));
        assert!(leer_nach_strip(" \u{1f}\u{a0}"));
        assert!(!leer_nach_strip(" x "));
    }
}

/// Aequivalenz mit `domain::PyWert` (D15): Abweichungen nur mit D-Nummer aus der Liste des
/// Helfers, je D-Nummer ein Test mit dem `CPython`-Verhalten.
///
/// Zwei Messungen je Helfer: die Alt-Fassung gegen `CPython` (mit D-Liste, Auflage 1) und die
/// Produktions-Fassung gegen die Alt-Fassung (OHNE Ausnahmen — jede Abweichung ist ein Befund).
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{d3, json_wert, klasse, pruefe, py, text};
    use domain::{py_strip, PyWert};
    use proptest::prelude::*;
    use serde_json::{json, Value};

    use super::{
        ganzzahl, ganzzahl_alt, leer_nach_strip, zahl_gleich_null, zahl_gleich_null_alt, zahl_gt0,
        zahl_gt0_alt,
    };

    /// Ausnahmen von `ganzzahl_alt`.
    const GANZZAHL: &[&str] = &["D3"];
    /// Ausnahmen von `zahl_gleich_null_alt`.
    const GLEICH_NULL: &[&str] = &["D13"];

    /// D13: `False == 0`; der Alt-Helfer ueberlaesst `bool` dem Aufrufer.
    fn d13(v: &Value) -> Vec<&'static str> {
        if *v == Value::Bool(false) {
            vec!["D13"]
        } else {
            Vec::new()
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn ganzzahl_alt_wie_pywert(v in json_wert()) {
            let (alt, neu) = (Ok(ganzzahl_alt(&v)), klasse(py(&v).int_ohne_bool()));
            pruefe(&v, &alt, &neu, || d3(&v, &neu), GANZZAHL)?;
        }

        #[test]
        fn ganzzahl_wie_alt(v in json_wert()) {
            pruefe(&v, &ganzzahl_alt(&v), &ganzzahl(&py(&v)), Vec::new, &[])?;
        }

        #[test]
        fn zahl_gt0_alt_wie_pywert(v in json_wert()) {
            let neu = py(&v).zahl_ohne_bool().is_some_and(|z| z.gt_null() == Ok(true));
            pruefe(&v, &zahl_gt0_alt(&v), &neu, Vec::new, &[])?;
        }

        #[test]
        fn zahl_gt0_wie_alt(v in json_wert()) {
            pruefe(&v, &zahl_gt0_alt(&v), &zahl_gt0(&py(&v)), Vec::new, &[])?;
        }

        #[test]
        fn zahl_gleich_null_alt_wie_pywert(v in json_wert()) {
            let neu = py(&v).py_eq(&PyWert::Ganz(0));
            pruefe(&v, &zahl_gleich_null_alt(&v), &neu, || d13(&v), GLEICH_NULL)?;
        }

        #[test]
        fn zahl_gleich_null_wie_alt(v in json_wert()) {
            pruefe(&v, &zahl_gleich_null_alt(&v), &zahl_gleich_null(&py(&v)), Vec::new, &[])?;
        }

        #[test]
        fn leer_nach_strip_wie_py_strip(s in text()) {
            pruefe(&s, &leer_nach_strip(&s), &py_strip(&s).is_empty(), Vec::new, &[])?;
        }
    }

    /// D3: `2**64 - 1` ist in `CPython` ein `int`. Der Alt-Helfer liefert `None`, `PyWert` meldet
    /// die i64-Grenze.
    #[test]
    fn d3_ueber_i64() {
        let v = json!(u64::MAX);
        assert_eq!(ganzzahl_alt(&v), None);
        assert_eq!(klasse(py(&v).int_ohne_bool()), Err(None));
        // Die Produktion trifft die Alt-Fassung: beide `None`, aber aus verschiedenen Gruenden.
        assert_eq!(ganzzahl(&py(&v)), None);
    }

    /// D13: `False == 0` ist in `CPython` wahr. Der Alt-Helfer liefert `false`.
    #[test]
    fn d13_false_gleich_null() {
        let v = json!(false);
        assert!(!zahl_gleich_null_alt(&v));
        assert!(py(&v).py_eq(&PyWert::Ganz(0)));
        assert!(!zahl_gleich_null(&py(&v)));
    }
}
