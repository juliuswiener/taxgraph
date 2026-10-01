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

/// Aequivalenz mit `domain::PyWert` (D15): Abweichungen nur mit D-Nummer aus der Liste des
/// Helfers, je D-Nummer ein Test mit dem `CPython`-Verhalten.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{
        ascii_fassung, ganzzahl_text, hat_d9, json_wert, klasse, nd_ziffer, ohne_d9, pruefe, py,
        py_absteigend, text as zeichenkette, viele_ziffern,
    };
    use domain::PyWert;
    use proptest::prelude::*;
    use serde_json::{json, Value};

    use super::{int, repr, repr_float, text, typname, wahr};

    /// Ausnahmen von `int`.
    const INT: &[&str] = &["D6", "D17"];
    /// Ausnahmen von `repr` und `text`.
    const REPR: &[&str] = &["D8", "D9"];

    /// `int_dezimal()`, jeder Fehler als `None` wie in `int`.
    fn int_neu(v: &Value) -> Option<String> {
        py(v).int_dezimal().ok()
    }

    /// Die D-Nummern, unter denen `int(v)` `alt` liefert, wo [`int_neu`] `neu` liefert.
    fn int_ausnahmen(v: &Value, alt: Option<&str>, neu: Option<&str>) -> Vec<&'static str> {
        let Some(s) = v.as_str() else {
            return Vec::new();
        };
        if viele_ziffern(s) && neu.is_none() {
            vec!["D17"]
        } else if nd_ziffer(s)
            && alt.is_none()
            && neu == int_neu(&Value::from(ascii_fassung(s))).as_deref()
        {
            vec!["D6"]
        } else {
            Vec::new()
        }
    }

    fn int_wie(v: &Value) -> Result<(), TestCaseError> {
        let (alt, neu) = (int(v), int_neu(v));
        pruefe(
            v,
            &alt,
            &neu,
            || int_ausnahmen(v, alt.as_deref(), neu.as_deref()),
            INT,
        )
    }

    /// D8 und D9: `alt_von` gleicht `neu_von` mit sortierten Objekten (D8), sobald jedes Zeichen
    /// aus D9 druckbar ist (D9).
    fn repr_ausnahmen(
        v: &Value,
        alt_von: fn(&Value) -> String,
        neu_von: fn(&PyWert) -> String,
    ) -> Vec<&'static str> {
        let d9 = hat_d9(v);
        let v = if d9 { ohne_d9(v) } else { v.clone() };
        let alt = alt_von(&v);
        let mut d = if alt == neu_von(&py_absteigend(&v)) {
            Vec::new()
        } else if alt == neu_von(&py(&v)) {
            vec!["D8"]
        } else {
            return Vec::new();
        };
        if d9 {
            d.push("D9");
        }
        d
    }

    fn repr_wie(
        v: &Value,
        alt_von: fn(&Value) -> String,
        neu_von: fn(&PyWert) -> String,
    ) -> Result<(), TestCaseError> {
        let (alt, neu) = (alt_von(v), neu_von(&py_absteigend(v)));
        pruefe(v, &alt, &neu, || repr_ausnahmen(v, alt_von, neu_von), REPR)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn typname_wie_pywert(v in json_wert()) {
            pruefe(&v, &typname(&v), &py(&v).typname(), Vec::new, &[])?;
        }

        #[test]
        fn wahr_wie_pywert(v in json_wert()) {
            pruefe(&v, &wahr(&v), &py(&v).truthy(), Vec::new, &[])?;
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
            repr_wie(&v, repr, PyWert::repr)?;
        }

        #[test]
        fn text_wie_pywert(v in json_wert()) {
            repr_wie(&v, text, PyWert::py_str)?;
        }

        /// `repr_text` ueber `repr` eines Texts.
        #[test]
        fn repr_text_wie_pywert(s in zeichenkette()) {
            repr_wie(&Value::String(s), repr, PyWert::repr)?;
        }

        /// Nur endliche Floats: `repr_float` sieht nur Zahlen aus `serde_json`.
        #[test]
        fn repr_float_wie_pywert(f in any::<f64>().prop_filter("endlich", |f| f.is_finite())) {
            pruefe(&f, &repr_float(f), &PyWert::Gleit(f).repr(), Vec::new, &[])?;
        }
    }

    /// D6: `int("٣")` ist in `CPython` 3.
    #[test]
    fn d6_nd_ziffer() {
        let v = json!("\u{663}");
        assert_eq!(int(&v), None);
        assert_eq!(int_neu(&v).as_deref(), Some("3"));
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

    /// D9: `repr("\xa0")` escapet in `CPython` das geschuetzte Leerzeichen. Der Alt-Helfer escapet
    /// nur Steuerzeichen (Cc).
    #[test]
    fn d9_nicht_druckbar() {
        let v = json!("\u{a0}");
        assert_eq!(repr(&v), "'\u{a0}'");
        assert_eq!(py(&v).repr(), "'\\xa0'");
    }

    /// D17: `int()` mit mehr als 4300 Ziffern wirft in `CPython` `ValueError`.
    #[test]
    fn d17_mehr_als_4300_ziffern() {
        let nullen = json!(format!("{}5", "0".repeat(4300)));
        let sieben = "7".repeat(4301);
        assert_eq!(int(&nullen).as_deref(), Some("5"));
        assert_eq!(int(&json!(sieben)).as_deref(), Some(sieben.as_str()));
        for v in [nullen, json!(sieben)] {
            assert_eq!(klasse(py(&v).int_dezimal()), Err(Some("ValueError")));
        }
    }
}
