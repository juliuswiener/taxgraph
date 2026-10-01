//! Python-Wertsemantik über JSON-Werten, soweit die Prüfungen sie brauchen.
use serde_json::Value;

/// `isinstance(w, int) and not isinstance(w, bool)`, beschränkt auf `i64`. JSON trennt `bool`
/// von Zahlen, deshalb liefert `as_i64` für `true` schon `None`. Floats sind `None`
/// (Wertebereich, s. Crate-Doku).
pub(crate) fn ganzzahl(v: &Value) -> Option<i64> {
    v.as_i64()
}

/// `isinstance(w, (int, float)) and not isinstance(w, bool) and w > 0` — exakt über alle
/// JSON-Zahlen, auch Floats und Ganzzahlen über `i64::MAX`.
pub(crate) fn zahl_gt0(v: &Value) -> bool {
    let Value::Number(n) = v else { return false };
    if let Some(i) = n.as_i64() {
        i > 0
    } else if n.as_u64().is_some() {
        true
    } else {
        n.as_f64().is_some_and(|f| f > 0.0)
    }
}

/// `w == 0` für eine JSON-Zahl (Python: `0 == 0.0 == -0.0`). `false == 0` behandelt der Aufrufer.
pub(crate) fn zahl_gleich_null(v: &Value) -> bool {
    matches!(v, Value::Number(n) if n.as_f64() == Some(0.0))
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

#[cfg(test)]
mod tests {
    use super::{eur, leer_nach_strip, zahl_gleich_null, zahl_gt0};
    use serde_json::json;

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
        assert!(!zahl_gt0(&json!(true)));
        assert!(zahl_gt0(&json!(0.5)));
        assert!(zahl_gt0(&json!(u64::MAX)));
        assert!(zahl_gleich_null(&json!(-0.0)));
        assert!(!zahl_gleich_null(&json!(false)));
        assert!(leer_nach_strip(" \u{1f}\u{a0}"));
        assert!(!leer_nach_strip(" x "));
    }
}

/// Aequivalenz mit `domain::PyWert` (D15): Abweichungen nur mit D-Nummer aus der Liste des
/// Helfers, je D-Nummer ein Test mit dem `CPython`-Verhalten.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{d3, json_wert, klasse, pruefe, py, text};
    use domain::{py_strip, PyWert};
    use proptest::prelude::*;
    use serde_json::{json, Value};

    use super::{ganzzahl, leer_nach_strip, zahl_gleich_null, zahl_gt0};

    /// Ausnahmen von `ganzzahl`.
    const GANZZAHL: &[&str] = &["D3"];
    /// Ausnahmen von `zahl_gleich_null`.
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
        fn ganzzahl_wie_pywert(v in json_wert()) {
            let (alt, neu) = (Ok(ganzzahl(&v)), klasse(py(&v).int_ohne_bool()));
            pruefe(&v, &alt, &neu, || d3(&v, &neu), GANZZAHL)?;
        }

        #[test]
        fn zahl_gt0_wie_pywert(v in json_wert()) {
            let neu = py(&v).zahl_ohne_bool().is_some_and(|z| z.gt_null() == Ok(true));
            pruefe(&v, &zahl_gt0(&v), &neu, Vec::new, &[])?;
        }

        #[test]
        fn zahl_gleich_null_wie_pywert(v in json_wert()) {
            let neu = py(&v).py_eq(&PyWert::Ganz(0));
            pruefe(&v, &zahl_gleich_null(&v), &neu, || d13(&v), GLEICH_NULL)?;
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
        assert_eq!(ganzzahl(&v), None);
        assert_eq!(klasse(py(&v).int_ohne_bool()), Err(None));
    }

    /// D13: `False == 0` ist in `CPython` wahr. Der Alt-Helfer liefert `false`.
    #[test]
    fn d13_false_gleich_null() {
        let v = json!(false);
        assert!(!zahl_gleich_null(&v));
        assert!(py(&v).py_eq(&PyWert::Ganz(0)));
    }
}
