//! GET-Routen auf einen Fall — STUBS für D2. Jeder Handler bekommt seinen Fall nur über
//! [`EigenerFall`]; er antwortet `501 nicht_portiert`, bis die Python-Logik (`api.py`) portiert ist.
//!
//! D2: Funktionsnamen und Modul lassen, Rumpf ersetzen. Der Dispatcher, die Tabelle und der
//! Harness bleiben unberührt. `warum`/`frage_einzeln` lesen `Treffer::fid` (`m["fid"]`).
#![allow(clippy::unused_async)] // die Stubs warten nicht; die echten Handler tun es.

use axum::extract::State;

use crate::antwort::Antwort;
use crate::eigener_fall::EigenerFall;
use crate::fehler::ApiFehler;
use crate::zustand::{Treffer, Zustand};

/// `GET /fall/{id}/fragen` — `api.fragen` (`api.py:342`).
///
/// # Errors
/// Die Fehler des Owner-Checks (401/403/404) und, sobald portiert, die von `api.fragen`.
pub async fn fragen(_fall: EigenerFall) -> Result<Antwort, ApiFehler> {
    Ok(Antwort::nicht_portiert("GET /fall/{id}/fragen"))
}

/// `GET /fall/{id}/stand` — `api.stand` (`api.py:447`), Rumpf in [`crate::stand::stand`].
///
/// # Errors
/// Die Fehler des Owner-Checks (401/403/404) und die von `api.stand`.
pub async fn stand(State(z): State<Zustand>, fall: EigenerFall) -> Result<Antwort, ApiFehler> {
    crate::stand::stand(&z, fall.id(), fall.store())
}

/// `GET /fall/{id}/feld/{fid}/warum` — `api.warum` (`api.py:547`).
///
/// # Errors
/// Wie [`fragen`].
pub async fn warum(_fall: EigenerFall, _treffer: Treffer) -> Result<Antwort, ApiFehler> {
    Ok(Antwort::nicht_portiert("GET /fall/{id}/feld/{fid}/warum"))
}

/// `GET /fall/{id}/feld/{fid}/frage` — `api.frage_einzeln` (`api.py:360`).
///
/// # Errors
/// Wie [`fragen`].
pub async fn frage_einzeln(_fall: EigenerFall, _treffer: Treffer) -> Result<Antwort, ApiFehler> {
    Ok(Antwort::nicht_portiert("GET /fall/{id}/feld/{fid}/frage"))
}

/// `GET /fall/{id}/ergebnis` — `api.ergebnis` (`api.py:568`, `_ergebnis_roh`).
///
/// # Errors
/// Wie [`fragen`].
pub async fn ergebnis(_fall: EigenerFall) -> Result<Antwort, ApiFehler> {
    Ok(Antwort::nicht_portiert("GET /fall/{id}/ergebnis"))
}

/// `GET /fall/{id}/preflight` — `api.preflight_check` (`api.py:640`).
///
/// # Errors
/// Wie [`fragen`].
pub async fn preflight(_fall: EigenerFall) -> Result<Antwort, ApiFehler> {
    Ok(Antwort::nicht_portiert("GET /fall/{id}/preflight"))
}

/// `GET /fall/{id}/deklaration` — `api.deklaration` (`api.py:664`).
///
/// # Errors
/// Wie [`fragen`].
pub async fn deklaration(_fall: EigenerFall) -> Result<Antwort, ApiFehler> {
    Ok(Antwort::nicht_portiert("GET /fall/{id}/deklaration"))
}

/// `GET /fall/{id}/graph` — `api.graph` (`api.py:832`).
///
/// PARITÄT: P7 — `static/graph.js` ruft die Route ohne `Authorization`-Kopf (`graph.js:13,70`); der
/// Extractor antwortet 401, wie Python. Die Korrektur liegt im Frontend, nicht hier.
///
/// # Errors
/// Wie [`fragen`].
pub async fn graph(_fall: EigenerFall) -> Result<Antwort, ApiFehler> {
    Ok(Antwort::nicht_portiert("GET /fall/{id}/graph"))
}
