//! Geteilter Dienstzustand und die Request-Kontexte, die der Dispatcher an den Handler reicht.
use std::future::Future;
use std::sync::Arc;

use auth::Auth;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use serde_json::Value;

use crate::fehler::ApiFehler;
use crate::konfig::Konfig;

/// Was jeder Handler sieht: Pfade, Auth, und die Sperre, die den Dienst wie Pythons
/// einfädigen `HTTPServer` Anfrage für Anfrage arbeiten lässt.
#[derive(Clone)]
pub struct Zustand {
    pub konfig: Arc<Konfig>,
    pub auth: Arc<Auth>,
    pub(crate) sperre: Arc<tokio::sync::Mutex<()>>,
}

impl Zustand {
    // ponytail: eine globale Sperre wie `make_server` (Catala-Laufzeit ist nicht thread-sicher,
    // `server.py:278-286`); feiner, sobald `catala-sys` unter Last gemessen ist.
    #[must_use]
    pub fn neu(konfig: Konfig, auth: Auth) -> Self {
        Self {
            konfig: Arc::new(konfig),
            auth: Arc::new(auth),
            sperre: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Zustand aus der Umgebung: `TAXGRAPH_JWT_SECRET`, `TAXGRAPH_USER_STORE` wie `auth.py:18-23`.
    ///
    /// # Errors
    /// Wenn die Zufallsquelle für das Geheimnis versagt.
    pub fn aus_env() -> Result<Self, auth::AuthFehler> {
        let konfig = Konfig::aus_env();
        let auth = Auth::aus_env(&konfig.nutzerdatei(), Some(konfig.audit_pfad()))?;
        Ok(Self::neu(konfig, auth))
    }
}

/// Der Nutzer dieser Anfrage (`api_auth._AUTH_USER`): `None`, wenn kein gültiges Token kam.
#[derive(Debug, Clone)]
pub struct Nutzer(pub Option<String>);

/// Der gelesene JSON-Rumpf (`body`); `{}` ohne Rumpf, sonst was immer `json.loads` lieferte.
#[derive(Debug, Clone)]
pub struct Koerper(pub Value);

/// Das Routenmuster, das gegriffen hat, und seine Treffergruppen (`treffer.groupdict()`).
#[derive(Debug, Clone)]
pub struct Treffer {
    /// Python-Muster; Teil des Fehlerlog-Orts `server.dispatch <muster>`.
    pub ort: &'static str,
    pub id: Option<String>,
    pub fid: Option<String>,
}

fn aus_erweiterung<T: Clone + Send + Sync + 'static>(parts: &Parts) -> Result<T, ApiFehler> {
    parts.extensions.get::<T>().cloned().ok_or_else(|| {
        ApiFehler::unerwartet("RuntimeError", "Dispatcher hat den Kontext nicht gesetzt")
    })
}

impl FromRequestParts<Zustand> for Nutzer {
    type Rejection = ApiFehler;

    fn from_request_parts(
        parts: &mut Parts,
        _: &Zustand,
    ) -> impl Future<Output = Result<Self, ApiFehler>> {
        std::future::ready(aus_erweiterung(parts))
    }
}

impl FromRequestParts<Zustand> for Koerper {
    type Rejection = ApiFehler;

    fn from_request_parts(
        parts: &mut Parts,
        _: &Zustand,
    ) -> impl Future<Output = Result<Self, ApiFehler>> {
        std::future::ready(aus_erweiterung(parts))
    }
}

impl FromRequestParts<Zustand> for Treffer {
    type Rejection = ApiFehler;

    fn from_request_parts(
        parts: &mut Parts,
        _: &Zustand,
    ) -> impl Future<Output = Result<Self, ApiFehler>> {
        std::future::ready(aus_erweiterung(parts))
    }
}
