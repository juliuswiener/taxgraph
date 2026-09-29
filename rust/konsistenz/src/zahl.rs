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
