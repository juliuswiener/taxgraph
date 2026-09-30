//! Betrieb: `/health`, `/ready` und die `elster-ampel`, die in Python nichts prüft.
#![allow(clippy::unused_async)] // axum-Handler sind `async`, auch wo sie nichts erwarten.

use axum::extract::State;
use serde_json::json;

use crate::antwort::Antwort;
use crate::konfig::flow_an;
use crate::zustand::Zustand;

/// `GET /health` — Lebendtest, keine Abhängigkeiten (`api.health`, `api.py:1263`).
#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Dienst läuft", body = crate::schema::Gesundheit))
)]
pub async fn health() -> Antwort {
    Antwort::neu(200, json!({ "status": "ok", "flow": flow_an() }))
}

/// `GET /ready` — erreichbares Fall-Verzeichnis (`api.ready`, `api.py:1277`).
#[utoipa::path(
    get,
    path = "/ready",
    responses(
        (status = 200, description = "Fall-Verzeichnis vorhanden", body = crate::schema::Bereitschaft),
        (status = 503, description = "Fall-Verzeichnis fehlt", body = crate::schema::Bereitschaft)
    )
)]
pub async fn ready(State(z): State<Zustand>) -> Antwort {
    if z.konfig.faelle.is_dir() {
        Antwort::neu(200, json!({ "status": "ok" }))
    } else {
        Antwort::neu(
            503,
            json!({ "status": "not_ready", "detail": "faelle-verzeichnis fehlt" }),
        )
    }
}

/// `POST /fall/{id}/elster-ampel` — immer 503 (`api.AMPEL_503`, `api.py:866`).
///
/// PARITÄT: P4 — diese Route hat in Python weder Owner-Check noch Auth-Pflicht (`server.py:92`:
/// `lambda m, b: (503, api.AMPEL_503)`). Sie antwortet auch für fremde und nicht vorhandene Fälle
/// mit 503, ohne `EigenerFall`. Die Korrektur ist ein eigener Commit: Extractor ergänzen.
#[utoipa::path(
    post,
    path = "/fall/{id}/elster-ampel",
    params(("id" = String, Path, description = "Fall-Kennung (ohne Prüfung, PARITÄT P4)")),
    responses((status = 503, description = "Ampel nicht verdrahtet", body = crate::schema::AmpelNichtVerdrahtet))
)]
pub async fn elster_ampel() -> Antwort {
    // PARITÄT: P4 — bewusst KEIN `EigenerFall`.
    Antwort::neu(
        503,
        json!({
            "fehler": "unavailable",
            "grund": "ELSTER-Ampel (warmer checkESt-Daemon) ist für diese Scheibe noch nicht verdrahtet \
        — ein gültiger ESt-Fall entsteht erst mit der Gesamtsteuer-Integration. Kein Fake-Grün.",
            "regel": "gekappt_verdacht=true ist nie grün (API.md-Garantie 5).",
        }),
    )
}
