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
