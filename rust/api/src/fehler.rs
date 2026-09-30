//! `ApiError`/`AuthError` und die nicht abgefangene Ausnahme, wie `server.py:_dispatch` sie
//! abbildet: Status + `{"fehler": …}`.
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::antwort::json_antwort;

/// Was ein Handler statt einer [`crate::Antwort`] liefern kann.
#[derive(Debug)]
pub enum ApiFehler {
    /// `ApiError(status, msg)` oder `AuthError(status, msg)`: `{"fehler": msg}` mit diesem Status.
    // PARITÄT: P6 — der Status hängt an der Route, nicht an der Ursache: `/event` meldet eine
    // Store-Abweisung als `Status(422, …)`, `/kontoauszug` (`api.py:1010` gegen `:534`) lässt sie als
    // `Unerwartet` zu 500 durch. Jeder Handler wählt die Variante wie sein Python-Gegenstück.
    Status(u16, String),
    /// Eine Python-Ausnahme, die `_dispatch` nicht kennt (`except Exception`): 500 und der
    /// Body `{"fehler": "Typ: Meldung"}`; der Dispatcher protokolliert sie im Fehlerlog.
    // PARITÄT: P5 — `server.py:252` schickt `str(e)` an den Client, auch wenn die Meldung einen
    // Wert aus dem Store trägt (`store.py:230`). Rust baut das nach; die Korrektur ist ein eigener
    // Commit.
    Unerwartet { typ: String, meldung: String },
}

/// Marker an der Antwort: der Dispatcher braucht zu wissen, WIE sie entstand (Audit-Status,
/// Fehlerlog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ausgang {
    /// `ApiError`/`AuthError`.
    Fehler,
    /// Nicht abgefangene Ausnahme.
    Unerwartet,
}

impl ApiFehler {
    #[must_use]
    pub fn status(status: u16, meldung: impl Into<String>) -> Self {
        Self::Status(status, meldung.into())
    }

    #[must_use]
    pub fn unerwartet(typ: impl Into<String>, meldung: impl Into<String>) -> Self {
        Self::Unerwartet {
            typ: typ.into(),
            meldung: meldung.into(),
        }
    }
}

impl IntoResponse for ApiFehler {
    fn into_response(self) -> Response {
        let (status, text, ausgang) = match self {
            Self::Status(s, m) => (s, m, Ausgang::Fehler),
            Self::Unerwartet { typ, meldung } => {
                (500, format!("{typ}: {meldung}"), Ausgang::Unerwartet)
            }
        };
        let mut r = json_antwort(status, &json!({ "fehler": text }));
        r.extensions_mut().insert(ausgang);
        r
    }
}

impl From<auth::AuthFehler> for ApiFehler {
    /// 400/401/409 sind Pythons `AuthError`; alles andere dort eine nicht abgefangene Ausnahme
    /// (`ValueError` von bcrypt, `OSError`, `JSONDecodeError` …).
    fn from(e: auth::AuthFehler) -> Self {
        use auth::AuthFehler as A;
        let typ = match &e {
            A::PasswortUeber72Bytes | A::Bcrypt(_) => "ValueError",
            A::FeldKeinText(_) => "TypeError",
            A::Speicher(_) | A::Zufall(_) => "OSError",
            A::NutzerdateiKaputt(_) => "JSONDecodeError",
            A::NutzerdateiOhneUsers => "KeyError",
            A::Jwt(_) => "PyJWTError",
            _ => return Self::Status(e.status(), e.to_string()),
        };
        Self::unerwartet(typ, e.to_string())
    }
}

impl From<store::PersistenzFehler> for ApiFehler {
    fn from(e: store::PersistenzFehler) -> Self {
        let typ = match &e {
            store::PersistenzFehler::Lesen(..) | store::PersistenzFehler::Schreiben(_) => "OSError",
            store::PersistenzFehler::Format(..) => "JSONDecodeError",
            store::PersistenzFehler::Serialisieren(_) => "TypeError",
        };
        Self::unerwartet(typ, e.to_string())
    }
}

impl From<store::audit::AuditFehler> for ApiFehler {
    fn from(e: store::audit::AuditFehler) -> Self {
        Self::unerwartet("OSError", e.to_string())
    }
}

impl From<std::io::Error> for ApiFehler {
    fn from(e: std::io::Error) -> Self {
        Self::unerwartet("OSError", e.to_string())
    }
}
