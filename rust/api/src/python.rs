//! Pythons Wert-Semantik, soweit sie in Fehlertexten und Eingabeprüfungen sichtbar wird.
//!
//! PARITÄT: `server.py` schickt `f"{type(e).__name__}: {e}"` an den Client (P5), und `api.py` setzt
//! `{wert!r}` in seine 400er. Beides hängt an Pythons `repr`/`int()`/Typnamen; ein Rust-Wert kennt
//! sie nicht. Nur die Werte, die `json.loads` liefert (None, bool, int, float, str, list, dict).
use serde_json::Value;

/// `type(x).__name__` für einen aus JSON gelesenen Wert.
///
/// ```
/// assert_eq!(api::python::typname(&serde_json::json!([1])), "list");
/// assert_eq!(api::python::typname(&serde_json::Value::Null), "NoneType");
/// ```
#[must_use]
pub fn typname(v: &Value) -> &'static str {
    match v {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::Number(n) if n.is_f64() => "float",
        Value::Number(_) => "int",
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "dict",
    }
}

/// Pythons `TypeError`-Text für einen Schlüssel, der nicht hashbar ist (Liste, Dict).
///
/// PARITÄT: der Wortlaut stammt aus Python 3.14 (`python3` dieses Rechners); ≤ 3.13 sagt nur
/// `unhashable type: 'list'`.
///
/// ```
/// let t = api::python::unhashbar(&serde_json::json!([1]), "dict key");
/// assert_eq!(t, "cannot use 'list' as a dict key (unhashable type: 'list')");
/// ```
#[must_use]
pub fn unhashbar(v: &Value, als: &str) -> String {
    let t = typname(v);
    format!("cannot use '{t}' as a {als} (unhashable type: '{t}')")
}

/// `bool(x)` — Pythons Wahrheitswert.
#[must_use]
pub fn wahr(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// `repr(str)`: einfache Anführungszeichen, außer der Text enthält nur `'` und kein `"`.
fn repr_text(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut aus = String::with_capacity(s.len() + 2);
    aus.push(quote);
    for c in s.chars() {
        match c {
            '\\' => aus.push_str("\\\\"),
            '\n' => aus.push_str("\\n"),
            '\r' => aus.push_str("\\r"),
            '\t' => aus.push_str("\\t"),
            c if c == quote => {
                aus.push('\\');
                aus.push(c);
            }
            c if c.is_control() => {
                let n = u32::from(c);
                let form = if n <= 0xff {
                    format!("\\x{n:02x}")
                } else {
                    format!("\\u{n:04x}")
                };
                aus.push_str(&form);
            }
            c => aus.push(c),
        }
    }
    aus.push(quote);
    aus
}

/// `repr(float)`: `1.0`, `1e+16`, `1e-05` (Rust schreibt `1e16`, `1e-5`).
fn repr_float(f: f64) -> String {
    let rust = format!("{f:?}");
    let Some((mantisse, exp)) = rust.split_once('e') else {
        return rust;
    };
    let (vorzeichen, ziffern) = exp.strip_prefix('-').map_or(('+', exp), |z| ('-', z));
    format!("{mantisse}e{vorzeichen}{ziffern:0>2}")
}

/// `repr(x)` für einen aus JSON gelesenen Wert.
///
/// ```
/// use api::python::repr;
/// assert_eq!(repr(&serde_json::json!("a'b")), r#""a'b""#);
/// assert_eq!(repr(&serde_json::json!(null)), "None");
/// assert_eq!(repr(&serde_json::json!([true, 1.5])), "[True, 1.5]");
/// ```
#[must_use]
pub fn repr(v: &Value) -> String {
    match v {
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::Number(n) => n
            .as_f64()
            .filter(|_| n.is_f64())
            .map_or_else(|| n.to_string(), repr_float),
        Value::String(s) => repr_text(s),
        Value::Array(a) => format!("[{}]", a.iter().map(repr).collect::<Vec<_>>().join(", ")),
        Value::Object(o) => {
            let teile: Vec<String> = o
                .iter()
                .map(|(k, w)| format!("{}: {}", repr_text(k), repr(w)))
                .collect();
            format!("{{{}}}", teile.join(", "))
        }
    }
}

/// `str(x)`: ein Text bleibt er selbst, alles andere ist sein `repr`.
#[must_use]
pub fn text(v: &Value) -> String {
    v.as_str().map_or_else(|| repr(v), str::to_owned)
}

/// `int(x)` als normalisierter Dezimaltext (beliebig große Ganzzahl, ohne führende Nullen).
/// `None`, wo Python `TypeError`/`ValueError` wirft.
///
/// PARITÄT-Grenze: Unicode-Ziffern (`int("２０２５")`) kennt diese Funktion nicht.
///
/// ```
/// use api::python::int;
/// assert_eq!(int(&serde_json::json!(" +2_025 ")).as_deref(), Some("2025"));
/// assert_eq!(int(&serde_json::json!(2025.9)).as_deref(), Some("2025"));
/// assert_eq!(int(&serde_json::json!(true)).as_deref(), Some("1"));
/// assert_eq!(int(&serde_json::json!("2025x")), None);
/// assert_eq!(int(&serde_json::json!(null)), None);
/// ```
#[must_use]
pub fn int(v: &Value) -> Option<String> {
    match v {
        Value::Bool(b) => Some(u8::from(*b).to_string()),
        Value::Number(n) if n.is_f64() => n
            .as_f64()
            .filter(|f| f.is_finite())
            .map(|f| normiere(&format!("{:.0}", f.trunc()))),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => int_aus_text(s),
        _ => None,
    }
}

fn int_aus_text(s: &str) -> Option<String> {
    let s = s.trim();
    let (negativ, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let gueltig = !rest.is_empty()
        && !rest.starts_with('_')
        && !rest.ends_with('_')
        && !rest.contains("__")
        && rest.chars().all(|c| c.is_ascii_digit() || c == '_');
    gueltig.then(|| {
        let ziffern: String = rest.chars().filter(char::is_ascii_digit).collect();
        normiere(&format!("{}{ziffern}", if negativ { "-" } else { "" }))
    })
}

fn normiere(roh: &str) -> String {
    let (neg, ziffern) = roh.strip_prefix('-').map_or((false, roh), |r| (true, r));
    let ohne = ziffern.trim_start_matches('0');
    if ohne.is_empty() {
        "0".to_owned()
    } else if neg {
        format!("-{ohne}")
    } else {
        ohne.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{int, repr};
    use serde_json::json;

    #[test]
    fn repr_folgt_python() {
        assert_eq!(repr(&json!("it's")), "\"it's\"");
        assert_eq!(repr(&json!("a\nb")), "'a\\nb'");
        assert_eq!(repr(&json!(1e16)), "1e+16");
        assert_eq!(repr(&json!(0.00001)), "1e-05");
        assert_eq!(repr(&json!({"a": [null]})), "{'a': [None]}");
    }

    #[test]
    fn int_folgt_python() {
        assert_eq!(int(&json!("-0")).as_deref(), Some("0"));
        assert_eq!(int(&json!("1__0")), None);
        assert_eq!(int(&json!("_1")), None);
        assert_eq!(int(&json!(1e20)).as_deref(), Some("100000000000000000000"));
        assert_eq!(
            int(&json!("99999999999999999999999")).as_deref(),
            Some("99999999999999999999999")
        );
    }
}
