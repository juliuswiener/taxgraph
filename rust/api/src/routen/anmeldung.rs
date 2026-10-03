//! `/auth/*` — Registrierung, Anmeldung, Abmeldung, Sitzung (`auth.py:127-182`,
//! `server.py:_session_check`). Die Logik liegt in `rust/auth`; hier steht, was Python im HTTP-Rand
//! tut: Body lesen, Pflichtfelder, und die Ausnahmen, die ein Nicht-Objekt als Body auslöst.
use auth::{Anmeldung, AuthFehler};
use axum::extract::State;
use serde_json::{json, Value};

use crate::antwort::Antwort;
use crate::fehler::ApiFehler;
use crate::python::{typname, unhashbar, wahr};
use crate::zustand::{Koerper, Nutzer, Zustand};

fn typ_fehler(typ: &str, meldung: String) -> ApiFehler {
    ApiFehler::unerwartet(typ, meldung)
}

/// `_REGISTER_FELDER - set(body)` und danach `body["username"]`, `body["password"]`.
///
/// PARITÄT: Pythons `set(body)` nimmt jedes Iterable. Ein Text liefert seine Zeichen (beide
/// Felder fehlen), eine Liste ihre Elemente, eine Zahl/`null` wirft `TypeError` (500). Nachgebildet
/// werden genau diese Fälle, weil `json.loads` nichts anderes liefert.
fn pflichtfelder(body: &Value) -> Result<(&Value, &Value), ApiFehler> {
    let fehlend = |vorhanden: &dyn Fn(&str) -> bool| -> Vec<&'static str> {
        ["password", "username"]
            .into_iter()
            .filter(|n| !vorhanden(n))
            .collect()
    };
    match body {
        Value::Object(m) => {
            let f = fehlend(&|n| m.contains_key(n));
            match (m.get("username"), m.get("password")) {
                (Some(u), Some(p)) if f.is_empty() => Ok((u, p)),
                _ => Err(AuthFehler::PflichtfelderFehlen(f).into()),
            }
        }
        Value::String(_) => Err(AuthFehler::PflichtfelderFehlen(fehlend(&|_| false)).into()),
        Value::Array(a) => {
            if let Some(unhashbar_wert) = a
                .iter()
                .find(|w| matches!(w, Value::Array(_) | Value::Object(_)))
            {
                return Err(typ_fehler(
                    "TypeError",
                    unhashbar(unhashbar_wert, "set element"),
                ));
            }
            let f = fehlend(&|n| a.iter().any(|w| w.as_str() == Some(n)));
            if f.is_empty() {
                return Err(typ_fehler(
                    "TypeError",
                    "list indices must be integers or slices, not str".into(),
                ));
            }
            Err(AuthFehler::PflichtfelderFehlen(f).into())
        }
        andere => Err(typ_fehler(
            "TypeError",
            format!("'{}' object is not iterable", typname(andere)),
        )),
    }
}

/// `re.fullmatch(nicht_text)` — Pythons `TypeError`.
fn kein_text(wert: &Value) -> ApiFehler {
    typ_fehler(
        "TypeError",
        format!(
            "expected string or bytes-like object, got '{}'",
            typname(wert)
        ),
    )
}

async fn blockierend<T: Send + 'static>(
    arbeit: impl FnOnce() -> Result<T, AuthFehler> + Send + 'static,
) -> Result<T, ApiFehler> {
    // bcrypt Kosten 12 und Dateizugriff: nicht auf einem Async-Worker.
    tokio::task::spawn_blocking(arbeit)
        .await
        .map_err(|e| ApiFehler::unerwartet("RuntimeError", e.to_string()))?
        .map_err(ApiFehler::from)
}

/// `POST /auth/register` (`auth.register`, `auth.py:127`).
///
/// # Errors
/// 400/409 wie `auth.register`; 500 bei I/O und Text über 72 Byte.
#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = crate::schema::Anmeldedaten,
    responses(
        (status = 201, description = "Nutzer angelegt", body = crate::schema::Registriert),
        (status = 400, description = "Pflichtfelder fehlen oder Muster verletzt", body = crate::schema::Fehler),
        (status = 409, description = "Nutzer existiert", body = crate::schema::Fehler)
    )
)]
pub async fn register(
    State(z): State<Zustand>,
    Koerper(body): Koerper,
) -> Result<Antwort, ApiFehler> {
    let (username, password) = pflichtfelder(&body)?;
    let Some(name) = username.as_str() else {
        return Err(kein_text(username));
    };
    let Some(klartext) = password.as_str() else {
        // Python prüft den Namen (400) vor dem Passwort (TypeError).
        return Err(if auth::ist_gueltiger_username(name) {
            kein_text(password)
        } else {
            AuthFehler::UsernameUngueltig.into()
        });
    };
    let anmeldung = Anmeldung {
        username: name.to_owned(),
        password: klartext.to_owned(),
    };
    let auth = z.auth.clone();
    let angelegt = blockierend(move || auth.registriere(&anmeldung)).await?;
    Ok(Antwort::neu(
        201,
        json!({ "username": angelegt.as_str(), "message": "registriert" }),
    ))
}

/// `POST /auth/login` (`auth.login`, `auth.py:153`).
///
/// # Errors
/// 400 bei fehlenden Feldern, 401 bei falschen Zugangsdaten.
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = crate::schema::Anmeldedaten,
    responses(
        (status = 200, description = "Token ausgestellt", body = crate::schema::Angemeldet),
        (status = 400, description = "Pflichtfelder fehlen", body = crate::schema::Fehler),
        (status = 401, description = "Name oder Passwort falsch", body = crate::schema::Fehler)
    )
)]
pub async fn login(State(z): State<Zustand>, Koerper(body): Koerper) -> Result<Antwort, ApiFehler> {
    let (username, password) = pflichtfelder(&body)?;
    let name = match username {
        Value::String(s) => s.clone(),
        // Ein Nicht-Text ist nie ein Nutzer: 401 wie bei einem falschen Passwort, protokolliert, ohne die
        // Nutzerdatei zu lesen. Auch `null`/`true`/`false`: ihr `repr` ("None", "True", "False") ist ein
        // gültiger Name, den ein Nutzer tragen könnte. Python: `isinstance(username, str)` in `login`
        // (Vault decisions/login-prueft-das-namensmuster-vor-dem-nachschlagen, Punkt 2); eine Liste oder ein
        // Objekt warf dort vorher `TypeError` (unhashbar, 500). Im Protokoll steht `unbekannt` (leerer Name,
        // `anhaengen`), nie der Rohwert und nie sein `repr`: `true` darf nicht als der echte Nutzer "True"
        // dastehen (Vault decisions/login-protokolliert-einen-nicht-text-namen-als-unbekannt).
        _ => return Err(z.auth.weise_ab("").into()),
    };
    // PARITÄT-Grenze: ein Nicht-Text als Passwort wirft in Python nur für einen EXISTIERENDEN
    // Nutzer `AttributeError` (500), sonst 401. Hier ist es immer 401 (leeres Passwort).
    let geheim = password.as_str().unwrap_or_default().to_owned();
    let auth = z.auth.clone();
    let nutzer = name.clone();
    let token = blockierend(move || {
        auth.login(&Anmeldung {
            username: nutzer,
            password: geheim,
        })
    })
    .await?;
    Ok(Antwort::neu(
        200,
        json!({ "token": token, "username": name }),
    ))
}

/// `POST /auth/logout` (`auth.logout`, `auth.py:171`) — antwortet immer 200.
///
/// # Errors
/// 500 nur bei einem Body, den Python mit `AttributeError` beantwortet.
#[utoipa::path(
    post,
    path = "/auth/logout",
    request_body = crate::schema::Abmeldung,
    responses((status = 200, description = "Abgemeldet (auch bei ungültigem Token)", body = crate::schema::Abgemeldet))
)]
pub async fn logout(
    State(z): State<Zustand>,
    Koerper(body): Koerper,
) -> Result<Antwort, ApiFehler> {
    let Value::Object(m) = &body else {
        return Err(typ_fehler(
            "AttributeError",
            format!("'{}' object has no attribute 'get'", typname(&body)),
        ));
    };
    let roh = match m.get("token") {
        Some(Value::String(s)) => s.as_str(),
        Some(t) if wahr(t) => {
            return Err(typ_fehler(
                "AttributeError",
                format!("'{}' object has no attribute 'startswith'", typname(t)),
            ));
        }
        _ => "",
    };
    z.auth.logout(roh);
    Ok(Antwort::neu(200, json!({ "message": "abgemeldet" })))
}

/// `GET /auth/session` (`server._session_check`, `server.py:37`).
///
/// # Errors
/// 401 ohne gültiges Token.
#[utoipa::path(
    get,
    path = "/auth/session",
    responses(
        (status = 200, description = "Token gültig", body = crate::schema::Sitzung),
        (status = 401, description = "Token fehlt, ungültig oder abgemeldet", body = crate::schema::Fehler)
    )
)]
pub async fn session(nutzer: Nutzer) -> Result<Antwort, ApiFehler> {
    match nutzer.0 {
        Some(uid) => Ok(Antwort::neu(
            200,
            json!({ "username": uid, "authenticated": true }),
        )),
        None => Err(ApiFehler::status(401, "ungültiges oder abgelaufenes Token")),
    }
}
