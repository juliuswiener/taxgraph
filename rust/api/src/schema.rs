//! Request-/Response-Typen der in 9a fertigen Routen — nur für das OpenAPI-Dokument
//! ([`crate::openapi::ApiDoc`]); die Handler bauen ihre Antworten als `serde_json::Value`, weil
//! Pythons Dicts Schlüssel tragen, die ein Struct nicht abbilden soll (`body` ist beliebiges JSON).
//! Die Schemas der Stub-Routen liegen in `produkt/haut/api_schema/*.json` und kommen mit ihren
//! Handlern (D2/D3).
use serde::Serialize;
use utoipa::ToSchema;

/// `{"fehler": "<Meldung>"}` — jede Fehlerantwort (`ApiError`, `AuthError`, 500).
#[derive(Debug, Serialize, ToSchema)]
pub struct Fehler {
    pub fehler: String,
}

/// `GET /health`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Gesundheit {
    pub status: String,
    /// Läuft der Fluss-Mitschnitt (`TAXGRAPH_FLOW=1`).
    pub flow: bool,
}

/// `GET /ready` — 200 `{"status": "ok"}` oder 503 mit `detail`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Bereitschaft {
    pub status: String,
    pub detail: Option<String>,
}

/// Rumpf von `POST /auth/register` und `POST /auth/login`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Anmeldedaten {
    pub username: String,
    pub password: String,
}

/// 201 von `POST /auth/register`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Registriert {
    pub username: String,
    pub message: String,
}

/// 200 von `POST /auth/login`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Angemeldet {
    pub token: String,
    pub username: String,
}

/// Rumpf von `POST /auth/logout`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Abmeldung {
    pub token: Option<String>,
}

/// 200 von `POST /auth/logout`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Abgemeldet {
    pub message: String,
}

/// 200 von `GET /auth/session`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Sitzung {
    pub username: String,
    pub authenticated: bool,
}

/// Rumpf von `POST /fall`.
#[derive(Debug, Serialize, ToSchema)]
pub struct FallAnlegen {
    /// `ep` (Vorgabe), `n_vor_gwg`, `an_gesamt`, `gesamt`, `rentner_gesamt`.
    pub scheibe: Option<String>,
    /// Vorgabe 2025; nur Jahre mit Verzeichnis unter `params/`.
    pub veranlagungszeitraum: Option<i64>,
    /// `[A-Za-z0-9_-]{1,64}`.
    pub fall_id: String,
}

/// 201 von `POST /fall`.
#[derive(Debug, Serialize, ToSchema)]
pub struct FallAngelegt {
    pub fall_id: String,
    pub scheibe: String,
    pub veranlagungszeitraum: i64,
}

/// 200 von `DELETE /fall/{id}`.
#[derive(Debug, Serialize, ToSchema)]
pub struct FallGeloescht {
    pub geloescht: bool,
    pub fall_id: String,
    pub scheibe: Option<String>,
    pub veranlagungszeitraum: Option<i64>,
}

/// 503 von `POST /fall/{id}/elster-ampel`.
#[derive(Debug, Serialize, ToSchema)]
pub struct AmpelNichtVerdrahtet {
    pub fehler: String,
    pub grund: String,
    pub regel: String,
}
