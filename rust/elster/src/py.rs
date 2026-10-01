//! Python-Semantik auf `serde_json::Value` fuer die Stellen, an denen `est_mapping.py` und
//! `elster_xml.py` rohe Store-Werte anfassen (`int(wert)`, `bool(wert)`, `str(wert)`, `wert == 0`,
//! `repr(...)` in Fehlermeldungen). Die Store-Werte sind JSON; der Port bildet genau die Python-
//! Operationen nach, die das Original auf ihnen ausfuehrt — nicht mehr.
//!
//! ponytail: `py_int` kennt nur ASCII-Ziffern und `i64`; Python akzeptiert jede Unicode-Ziffer
//! (Kategorie Nd) und rechnet unbeschraenkt. Beides erreicht ein Store-Wert nur ausserhalb von
//! Auflage T (typ cent/int ist dort `i64`). Upgrade: Nd-Tabelle und `i128`/Bignum, falls je noetig.

use std::fmt::Write;

use serde_json::Value;

/// Eine Python-Ausnahme, die der Originalcode an dieser Stelle werfen wuerde. Der Klassenname ist
/// das Paritaetskriterium (`tools/parity/elster_oracle.py` meldet `type(exc).__name__`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{klasse}: {nachricht}")]
pub struct PyFehler {
    /// Python-Ausnahmeklasse (`TypeError`, `ValueError`, `OverflowError`).
    pub klasse: &'static str,
    /// Menschlich lesbarer Grund (kein Paritaetskriterium).
    pub nachricht: String,
}

impl PyFehler {
    pub(crate) fn typ(nachricht: impl Into<String>) -> Self {
        Self {
            klasse: "TypeError",
            nachricht: nachricht.into(),
        }
    }
    pub(crate) fn wert(nachricht: impl Into<String>) -> Self {
        Self {
            klasse: "ValueError",
            nachricht: nachricht.into(),
        }
    }
    pub(crate) fn ueberlauf(nachricht: impl Into<String>) -> Self {
        Self {
            klasse: "OverflowError",
            nachricht: nachricht.into(),
        }
    }
}

/// Python `bool(wert)`.
pub(crate) fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Python `wert == 0` (`False == 0` und `0.0 == 0` sind in Python wahr).
pub(crate) fn gleich_null(v: &Value) -> bool {
    match v {
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f == 0.0),
        _ => false,
    }
}

/// Python `str.isspace()` fuer ein Zeichen: Unicode-`White_Space` plus die vier
/// Informationstrenner U+001C..U+001F, die Python ebenfalls als Leerraum fuehrt.
pub(crate) fn ist_leerraum(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{1c}'..='\u{1f}')
}

/// Python `str.strip()` ohne Argument.
pub(crate) fn strip(s: &str) -> &str {
    s.trim_matches(ist_leerraum)
}

/// Python `int(wert)`.
pub(crate) fn int(v: &Value) -> Result<i64, PyFehler> {
    match v {
        Value::Bool(b) => Ok(i64::from(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Ok(i);
            }
            if n.is_u64() {
                return Err(PyFehler::ueberlauf("int() jenseits i64"));
            }
            let f = n.as_f64().unwrap_or(f64::NAN);
            if f.is_nan() {
                return Err(PyFehler::wert("cannot convert float NaN to integer"));
            }
            if f.is_infinite() {
                return Err(PyFehler::ueberlauf(
                    "cannot convert float infinity to integer",
                ));
            }
            let t = f.trunc();
            // i64-Grenzen als f64 sind exakt darstellbar (Zweierpotenzen).
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            if t >= -(2f64.powi(63)) && t < 2f64.powi(63) {
                Ok(t as i64)
            } else {
                Err(PyFehler::ueberlauf("int() jenseits i64"))
            }
        }
        Value::String(s) => int_aus_text(s),
        Value::Null => Err(PyFehler::typ("int() argument must be ... not 'NoneType'")),
        Value::Array(_) => Err(PyFehler::typ("int() argument must be ... not 'list'")),
        Value::Object(_) => Err(PyFehler::typ("int() argument must be ... not 'dict'")),
    }
}

fn int_aus_text(s: &str) -> Result<i64, PyFehler> {
    let fehler = || {
        PyFehler::wert(format!(
            "invalid literal for int() with base 10: {}",
            repr_str(s)
        ))
    };
    let t = strip(s);
    let (negativ, rest) = match t.as_bytes().first() {
        Some(b'-') => (true, t.get(1..).unwrap_or("")),
        Some(b'+') => (false, t.get(1..).unwrap_or("")),
        _ => (false, t),
    };
    // Unterstriche nur ZWISCHEN Ziffern (PEP 515), nie am Rand, nie doppelt.
    if rest.is_empty() || rest.starts_with('_') || rest.ends_with('_') || rest.contains("__") {
        return Err(fehler());
    }
    let mut n: i64 = 0;
    for c in rest.chars().filter(|c| *c != '_') {
        let d = c.to_digit(10).ok_or_else(fehler)?;
        n = n
            .checked_mul(10)
            .and_then(|x| x.checked_add(i64::from(d)))
            .ok_or_else(|| PyFehler::ueberlauf("int() jenseits i64"))?;
    }
    Ok(if negativ { -n } else { n })
}

/// Python `str(wert)` fuer JSON-Werte.
pub(crate) fn str_von(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        _ => repr(v),
    }
}

/// Python `repr(wert)` fuer JSON-Werte (`None`, `True`, `'text'`, `[..]`, `{..}`).
pub(crate) fn repr(v: &Value) -> String {
    match v {
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                n.to_string()
            } else {
                float_repr(n.as_f64().unwrap_or(f64::NAN))
            }
        }
        Value::String(s) => repr_str(s),
        Value::Array(a) => format!("[{}]", a.iter().map(repr).collect::<Vec<_>>().join(", ")),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, w)| format!("{}: {}", repr_str(k), repr(w)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Python `repr(float)`: kuerzeste Ziffernfolge, Exponentialform bei Exponent < -4 oder >= 16.
fn float_repr(f: f64) -> String {
    if f.is_nan() {
        return "nan".to_owned();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    let vorzeichen = if f.is_sign_negative() { "-" } else { "" };
    let e_form = format!("{:e}", f.abs());
    let (mantisse, exp) = e_form.split_once('e').unwrap_or((e_form.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let ziffern: String = mantisse.chars().filter(char::is_ascii_digit).collect();
    let (erste, rest) = ziffern.split_at(1.min(ziffern.len()));
    let rumpf = if (-4..16).contains(&exp) {
        let exp_u = usize::try_from(exp.unsigned_abs()).unwrap_or(0);
        if exp >= 0 {
            if ziffern.len() <= exp_u + 1 {
                format!("{ziffern}{}.0", "0".repeat(exp_u + 1 - ziffern.len()))
            } else {
                let (ganz, bruch) = ziffern.split_at(exp_u + 1);
                format!("{ganz}.{bruch}")
            }
        } else {
            format!("0.{}{ziffern}", "0".repeat(exp_u - 1))
        }
    } else {
        let mant = if rest.is_empty() {
            erste.to_owned()
        } else {
            format!("{erste}.{rest}")
        };
        let zeichen = if exp < 0 { '-' } else { '+' };
        format!("{mant}e{zeichen}{:02}", exp.unsigned_abs())
    };
    format!("{vorzeichen}{rumpf}")
}

/// Python `repr(str)`: einfache Anfuehrungszeichen, ausser der Text enthaelt `'` und kein `"`.
///
/// ponytail: „druckbar" (Python `str.isprintable`) ist hier angenaehert — Steuerzeichen und die
/// gaengigen unsichtbaren Zeichen (NBSP, Soft-Hyphen, Zero-Width, Zeilen-/Absatztrenner, BOM)
/// werden escaped, alles Uebrige bleibt. Upgrade: Unicode-Kategorien-Tabelle.
pub(crate) fn repr_str(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if !druckbar(c) => {
                let n = u32::from(c);
                if n <= 0xff {
                    let _ = write!(out, "\\x{n:02x}");
                } else if n <= 0xffff {
                    let _ = write!(out, "\\u{n:04x}");
                } else {
                    let _ = write!(out, "\\U{n:08x}");
                }
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn druckbar(c: char) -> bool {
    if c == ' ' {
        return true;
    }
    !(c.is_control()
        || matches!(
            c,
            '\u{a0}'
                | '\u{ad}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200f}'
                | '\u{2028}'..='\u{202f}'
                | '\u{205f}'..='\u{2064}'
                | '\u{3000}'
                | '\u{feff}'
        ))
}

#[cfg(test)]
mod tests {
    use super::{float_repr, int, repr, repr_str};
    use serde_json::json;

    #[test]
    fn int_wie_python() {
        assert_eq!(int(&json!(" -1_000 ")).unwrap(), -1000);
        assert_eq!(int(&json!(true)).unwrap(), 1);
        assert_eq!(int(&json!(3.9)).unwrap(), 3);
        assert_eq!(int(&json!(-3.9)).unwrap(), -3);
        assert_eq!(int(&json!("1_")).unwrap_err().klasse, "ValueError");
        assert_eq!(int(&json!(null)).unwrap_err().klasse, "TypeError");
    }

    #[test]
    fn repr_wie_python() {
        assert_eq!(repr_str("a'b"), "\"a'b\"");
        assert_eq!(repr_str("a'b\""), "'a\\'b\"'");
        assert_eq!(
            repr(&json!([{"a": 1}, null, true])),
            "[{'a': 1}, None, True]"
        );
        assert_eq!(float_repr(1e16), "1e+16");
        assert_eq!(float_repr(1.5e-5), "1.5e-05");
        assert_eq!(float_repr(0.0001), "0.0001");
        assert_eq!(float_repr(123.0), "123.0");
        assert_eq!(float_repr(-0.0), "-0.0");
    }
}

/// Aequivalenz mit `domain::PyWert` (D15): Abweichungen nur mit D-Nummer aus der Liste des
/// Helfers, je D-Nummer ein Test mit dem `CPython`-Verhalten.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{
        ganzzahl_text, int_ausnahmen, json_wert, klasse, pruefe, py, py_absteigend, text, Ergebnis,
    };
    use domain::{py_strip, PyWert};
    use proptest::prelude::*;
    use serde_json::{json, Value};

    use super::{gleich_null, int, repr, repr_str, str_von, strip, truthy};

    /// Ausnahmen von `int`.
    const INT: &[&str] = &["D4", "D6", "D12", "D16", "D17"];
    /// Ausnahmen von `repr` und `str_von`.
    const REPR: &[&str] = &["D8"];

    fn alt_int(v: &Value) -> Ergebnis<i64> {
        int(v).map_err(|e| Some(e.klasse))
    }

    fn int_wie(v: &Value) -> Result<(), TestCaseError> {
        let (alt, neu) = (alt_int(v), klasse(py(v).int()));
        pruefe(v, &alt, &neu, || int_ausnahmen(v, &alt, &neu), INT)
    }

    /// D8: gleich, sobald `PyWert` die sortierte Reihenfolge von `Value` hat.
    fn d8(alt: &str, sortiert: &str) -> Vec<&'static str> {
        if alt == sortiert {
            vec!["D8"]
        } else {
            Vec::new()
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn truthy_wie_pywert(v in json_wert()) {
            pruefe(&v, &truthy(&v), &py(&v).truthy(), Vec::new, &[])?;
        }

        #[test]
        fn gleich_null_wie_pywert(v in json_wert()) {
            pruefe(&v, &gleich_null(&v), &py(&v).py_eq(&PyWert::Ganz(0)), Vec::new, &[])?;
        }

        #[test]
        fn strip_wie_pywert(s in text()) {
            pruefe(&s, &strip(&s), &py_strip(&s), Vec::new, &[])?;
        }

        #[test]
        fn int_wie_pywert(v in json_wert()) {
            int_wie(&v)?;
        }

        #[test]
        fn int_text_wie_pywert(s in ganzzahl_text()) {
            int_wie(&Value::String(s))?;
        }

        #[test]
        fn repr_wie_pywert(v in json_wert()) {
            let alt = repr(&v);
            pruefe(&v, &alt, &py_absteigend(&v).repr(), || d8(&alt, &py(&v).repr()), REPR)?;
        }

        #[test]
        fn str_von_wie_pywert(v in json_wert()) {
            let alt = str_von(&v);
            pruefe(&v, &alt, &py_absteigend(&v).py_str(), || d8(&alt, &py(&v).py_str()), REPR)?;
        }

        #[test]
        fn repr_str_wie_pywert(s in text()) {
            pruefe(&s, &repr_str(&s), &PyWert::Text(s.clone()).repr(), Vec::new, &[])?;
        }
    }

    /// D4: `int(2**63)` ist in `CPython` 9223372036854775808. Der Alt-Helfer wirft
    /// `OverflowError`, `PyWert` meldet die i64-Grenze ohne Python-Klasse.
    #[test]
    fn d4_i64_grenze() {
        let v = json!(9_223_372_036_854_775_808_u64);
        assert_eq!(alt_int(&v), Err(Some("OverflowError")));
        assert_eq!(klasse(py(&v).int()), Err(None));
        assert_eq!(py(&v).int_dezimal().unwrap(), "9223372036854775808");
    }

    /// D6: `int("٣")` ist in `CPython` 3.
    #[test]
    fn d6_nd_ziffer() {
        let v = json!("\u{663}");
        assert_eq!(alt_int(&v), Err(Some("ValueError")));
        assert_eq!(klasse(py(&v).int()), Ok(3));
    }

    /// D8: `repr(dict)` folgt in `CPython` der Reihenfolge der Datei.
    #[test]
    fn d8_reihenfolge_der_datei() {
        let datei = r#"{"b": 1, "a": 2}"#;
        let v: Value = serde_json::from_str(datei).unwrap();
        assert_eq!(repr(&v), "{'a': 2, 'b': 1}");
        let w: PyWert = serde_json::from_str(datei).unwrap();
        assert_eq!(w.repr(), "{'b': 1, 'a': 2}");
    }

    /// D12: `int("-9223372036854775808")` ist in `CPython` `i64::MIN`.
    #[test]
    fn d12_i64_min_als_text() {
        let v = json!("-9223372036854775808");
        assert_eq!(alt_int(&v), Err(Some("OverflowError")));
        assert_eq!(klasse(py(&v).int()), Ok(i64::MIN));
    }

    /// D16: `int("\x1c42")` wirft in `CPython` `ValueError`, obwohl `str.strip()` U+001C entfernt.
    #[test]
    fn d16_steuerzeichen_am_rand() {
        let v = json!("\u{1c}42");
        assert_eq!(alt_int(&v), Ok(42));
        assert_eq!(klasse(py(&v).int()), Err(Some("ValueError")));
    }

    /// D17: `int()` mit mehr als 4300 Ziffern wirft in `CPython` `ValueError`.
    #[test]
    fn d17_mehr_als_4300_ziffern() {
        let nullen = json!(format!("{}5", "0".repeat(4300)));
        let sieben = json!("7".repeat(4301));
        assert_eq!(alt_int(&nullen), Ok(5));
        assert_eq!(alt_int(&sieben), Err(Some("OverflowError")));
        for v in [nullen, sieben] {
            assert_eq!(klasse(py(&v).int()), Err(Some("ValueError")));
        }
    }
}
