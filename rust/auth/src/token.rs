//! JWT `HS256` wie `auth.py:81-106` mit PyJWT 2.x (`jwt.encode`/`jwt.decode(..., algorithms=
//! ["HS256"])`).
//!
//! Die Signatur prueft `jsonwebtoken`; die Claim-Pruefung danach bildet die von PyJWT
//! `_validate_claims` (`jwt/api_jwt.py`) nach, weil `jsonwebtoken` sie anders macht: ohne `exp`
//! waere das Token dort ungueltig (PyJWT: gueltig), `exp == now` dort gueltig (PyJWT:
//! abgelaufen), und die Standard-Toleranz betraegt 60 s (PyJWT: 0).
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::Serialize;
use serde_json::{Map, Value};

/// `auth.py:20`: eine Sitzung gilt 24 h.
pub const JWT_TTL_S: i64 = 24 * 3600;

/// Die Claims, die dieser Dienst ausstellt (`auth.py:83-88`), in Pythons Einfuege-Reihenfolge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Claims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

/// Signiert `claims` mit HS256.
pub(crate) fn signiere(
    claims: &Claims,
    geheimnis: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(geheimnis.as_bytes()),
    )
}

/// Geprueftes Token-Payload oder `None` — `jwt.decode(token, secret, algorithms=["HS256"])`
/// ohne Ausnahme. Die Faelle, in denen PyJWT statt `PyJWTError` einen `TypeError` wirft
/// (`int(None)` fuer `exp`/`iat`/`nbf`), liefern hier ebenfalls `None`: PARITAET-Abweichung
/// zugunsten fail-closed, Python antwortet dort mit 500.
pub(crate) fn pruefe(token: &str, geheimnis: &str, jetzt: i64) -> Option<Map<String, Value>> {
    let mut validierung = Validation::new(Algorithm::HS256);
    validierung.required_spec_claims.clear();
    validierung.validate_exp = false;
    validierung.validate_nbf = false;
    validierung.validate_aud = false;
    validierung.leeway = 0;
    let daten = jsonwebtoken::decode::<Value>(
        token,
        &DecodingKey::from_secret(geheimnis.as_bytes()),
        &validierung,
    )
    .ok()?;
    let Value::Object(payload) = daten.claims else {
        return None;
    };
    claims_gueltig(&payload, jetzt).then_some(payload)
}

/// PyJWT `_validate_claims` mit `leeway=0`, ohne `audience`/`issuer`/`subject`.
/// `jetzt` ist die abgerundete Sekunde: fuer ganzzahlige Claims gilt `x > now` genau dann, wenn
/// `x > floor(now)` — der Float-Vergleich in PyJWT braucht hier keinen Float.
fn claims_gueltig(p: &Map<String, Value>, jetzt: i64) -> bool {
    let zahl = |k: &str| p.get(k).map(py_int);
    if let Some(iat) = zahl("iat") {
        match iat {
            Some(iat) if iat <= jetzt => {}
            _ => return false,
        }
    }
    if let Some(nbf) = zahl("nbf") {
        match nbf {
            Some(nbf) if nbf <= jetzt => {}
            _ => return false,
        }
    }
    if let Some(exp) = zahl("exp") {
        match exp {
            Some(exp) if exp > jetzt => {}
            _ => return false,
        }
    }
    // `audience is None`: ein wahres `aud` im Token ist ein Fehler (`Invalid audience`).
    if p.get("aud").is_some_and(py_wahr) {
        return false;
    }
    let text_oder_fehlt = |k: &str| p.get(k).is_none_or(Value::is_string);
    text_oder_fehlt("sub") && text_oder_fehlt("jti")
}

/// Pythons `int(x)` fuer einen JSON-Wert; `None` fuer `ValueError`, `TypeError` und
/// `OverflowError` gleichermassen.
#[allow(clippy::cast_possible_truncation)] // `f.trunc()` endlich; `as` saettigt statt zu ueberlaufen
fn py_int(v: &Value) -> Option<i64> {
    match v {
        Value::Bool(b) => Some(i64::from(*b)),
        Value::Number(n) => n.as_i64().or_else(|| {
            n.as_f64()
                .filter(|f| f.is_finite())
                .map(|f| f.trunc() as i64)
        }),
        Value::String(s) => {
            let s = s.trim();
            let rumpf = s.strip_prefix(['+', '-']).unwrap_or(s);
            let form_ok = !rumpf.is_empty()
                && !rumpf.starts_with('_')
                && !rumpf.ends_with('_')
                && !rumpf.contains("__")
                && rumpf.bytes().all(|b| b.is_ascii_digit() || b == b'_');
            form_ok.then(|| s.replace('_', "").parse().ok()).flatten()
        }
        _ => None,
    }
}

/// Pythons Wahrheitswert eines JSON-Werts.
fn py_wahr(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::{claims_gueltig, py_int};
    use serde_json::json;

    #[test]
    fn exp_gleich_jetzt_ist_abgelaufen_wie_pyjwt() {
        let p = json!({"exp": 100}).as_object().cloned().unwrap();
        assert!(!claims_gueltig(&p, 100));
        assert!(claims_gueltig(&p, 99));
    }

    #[test]
    fn iat_und_nbf_gleich_jetzt_gelten_erst_danach_nicht() {
        // PyJWT: `iat > now` und `nbf > now` sind Fehler, Gleichheit gilt.
        for k in ["iat", "nbf"] {
            let p = |w: i64| json!({ k: w }).as_object().cloned().unwrap();
            assert!(claims_gueltig(&p(99), 100), "{k} davor");
            assert!(claims_gueltig(&p(100), 100), "{k} gleich");
            assert!(!claims_gueltig(&p(101), 100), "{k} danach");
        }
    }

    #[test]
    fn ohne_exp_gueltig_wie_pyjwt() {
        assert!(claims_gueltig(&serde_json::Map::new(), 1_000_000_000));
    }

    #[test]
    fn aud_ohne_audience_ist_ungueltig() {
        let p = json!({"aud": "x"}).as_object().cloned().unwrap();
        assert!(!claims_gueltig(&p, 0));
        let leer = json!({"aud": ""}).as_object().cloned().unwrap();
        assert!(claims_gueltig(&leer, 0));
    }

    #[test]
    fn py_int_wie_python() {
        assert_eq!(py_int(&json!("12")), Some(12));
        assert_eq!(py_int(&json!(" -1_0 ")), Some(-10));
        assert_eq!(py_int(&json!(1.9)), Some(1));
        assert_eq!(py_int(&json!("1.5")), None);
        assert_eq!(py_int(&json!(null)), None);
    }
}

/// Aequivalenz mit `domain::PyWert` (D15): Abweichungen nur mit D-Nummer aus der Liste des
/// Helfers, je D-Nummer ein Test mit dem `CPython`-Verhalten.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{
        ascii_fassung, d3, ganzzahl_text, json_wert, klasse, nd_ziffer, pruefe, py, viele_ziffern,
    };
    use proptest::prelude::*;
    use serde_json::{json, Value};

    use super::{py_int, py_wahr};

    /// Ausnahmen von `py_int`. D19 ist fail-closed: `claims_gueltig` lehnt jedes `None` in `exp`,
    /// `nbf` und `iat` ab, nur unser Secret signiert, und wir stellen keine Zeit ab 2^63 aus.
    const INT: &[&str] = &["D3", "D6", "D17", "D19"];

    /// `int(v)`, jeder Fehler als `None` wie in `py_int`.
    fn int(v: &Value) -> Option<i64> {
        py(v).int().ok()
    }

    /// D19: `v` ist ein Float mit Betrag ab 2^63, der Alt-Helfer saettigt ihn auf `i64::MAX` oder
    /// `i64::MIN`, und `PyWert` meldet die i64-Grenze.
    fn d19(v: &Value, alt: Option<i64>) -> bool {
        let Some(f) = v
            .as_f64()
            .filter(|f| v.is_f64() && f.abs() >= 9_223_372_036_854_775_808.0)
        else {
            return false;
        };
        let saettigung = if f > 0.0 { i64::MAX } else { i64::MIN };
        alt == Some(saettigung) && klasse(py(v).int()) == Err(None)
    }

    /// Die D-Nummern, unter denen `py_int(v)` `alt` liefert, wo [`int`] `neu` liefert.
    fn ausnahmen(v: &Value, alt: Option<i64>, neu: Option<i64>) -> Vec<&'static str> {
        let text = v.as_str();
        if text.is_some_and(viele_ziffern) && neu.is_none() {
            vec!["D17"]
        } else if text.is_some_and(|s| {
            nd_ziffer(s) && alt.is_none() && neu == int(&Value::from(ascii_fassung(s)))
        }) {
            vec!["D6"]
        } else if d19(v, alt) {
            vec!["D19"]
        } else {
            d3(v, &klasse(py(v).int()))
        }
    }

    fn int_wie(v: &Value) -> Result<(), TestCaseError> {
        let (alt, neu) = (py_int(v), int(v));
        pruefe(v, &alt, &neu, || ausnahmen(v, alt, neu), INT)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn py_wahr_wie_pywert(v in json_wert()) {
            pruefe(&v, &py_wahr(&v), &py(&v).truthy(), Vec::new, &[])?;
        }

        #[test]
        fn py_int_wie_pywert(v in json_wert()) {
            int_wie(&v)?;
        }

        #[test]
        fn py_int_text_wie_pywert(s in ganzzahl_text()) {
            int_wie(&Value::String(s))?;
        }
    }

    /// D3: `int(2**64 - 1)` ist in `CPython` exakt. Der Alt-Helfer saettigt auf `i64::MAX`,
    /// `PyWert` meldet die i64-Grenze.
    #[test]
    fn d3_saettigung() {
        let v = json!(u64::MAX);
        assert_eq!(py_int(&v), Some(i64::MAX));
        assert_eq!(klasse(py(&v).int()), Err(None));
    }

    /// D6: `int("٣")` ist in `CPython` 3.
    #[test]
    fn d6_nd_ziffer() {
        let v = json!("\u{663}");
        assert_eq!(py_int(&v), None);
        assert_eq!(klasse(py(&v).int()), Ok(3));
    }

    /// D17: `int()` mit mehr als 4300 Ziffern wirft in `CPython` `ValueError`.
    #[test]
    fn d17_mehr_als_4300_ziffern() {
        let v = json!(format!("{}5", "0".repeat(4300)));
        assert_eq!(py_int(&v), Some(5));
        assert_eq!(klasse(py(&v).int()), Err(Some("ValueError")));
    }

    /// D19: `int(1e19)` und `int(-1e19)` sind in `CPython` exakt. Der Alt-Helfer saettigt auf
    /// `i64::MAX` und `i64::MIN`, `PyWert` meldet die i64-Grenze.
    #[test]
    fn d19_float_ab_2_hoch_63() {
        for (f, saettigung) in [
            (9_223_372_036_854_775_808.0, i64::MAX),
            (1e19, i64::MAX),
            (1e300, i64::MAX),
            (-1e19, i64::MIN),
            (-1e300, i64::MIN),
        ] {
            let v = json!(f);
            assert_eq!(py_int(&v), Some(saettigung));
            assert_eq!(klasse(py(&v).int()), Err(None));
        }
    }
}
