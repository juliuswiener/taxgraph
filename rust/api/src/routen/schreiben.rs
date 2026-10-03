//! POST-Routen auf einen Fall — STUBS für D3. Wie [`super::lesen`]: der Fall kommt nur über
//! [`EigenerFall`], der Rumpf über [`Koerper`], die Antwort ist `501 nicht_portiert`.
//!
//! D3: Funktionsnamen und Modul lassen, Rumpf ersetzen. `vorjahr` prüft seine zweite Fall-Kennung
//! (`body["vorjahr_fall_id"]`, `api.py:947`) mit `EigenerFall::pruefe`.
#![allow(clippy::unused_async)] // die Stubs warten nicht; die echten Handler tun es.

use axum::extract::State;
use domain::PyWert;

use crate::antwort::Antwort;
use crate::eigener_fall::{EigenerFall, FallBesitz};
use crate::fehler::ApiFehler;
use crate::zustand::{Koerper, KoerperRoh, Nutzer, Zustand};

/// `POST /fall/{id}/event` — `api.event` (`api.py:495`), Rumpf in [`crate::event::event`].
///
/// # Errors
/// Die Fehler des Owner-Checks (401/403/404) und die von `api.event`.
pub async fn event(
    State(z): State<Zustand>,
    mut fall: EigenerFall,
    Koerper(wert): Koerper,
    KoerperRoh(roh): KoerperRoh,
) -> Result<Antwort, ApiFehler> {
    crate::event::event(&z, &mut fall, &wert, &roh)
}

/// `POST /fall/{id}/einreichen` — `api.einreichen` (`api.py:675`).
///
/// # Errors
/// Wie [`event`].
pub async fn einreichen(_fall: EigenerFall, _body: Koerper) -> Result<Antwort, ApiFehler> {
    Ok(Antwort::nicht_portiert("POST /fall/{id}/einreichen"))
}

/// `POST /fall/{id}/chat` — `api.chat` (`api.py:1060`).
///
/// # Errors
/// Wie [`event`].
pub async fn chat(_fall: EigenerFall, _body: Koerper) -> Result<Antwort, ApiFehler> {
    Ok(Antwort::nicht_portiert("POST /fall/{id}/chat"))
}

/// `POST /fall/{id}/flow` — `api.flow_melden` (`api.py:1298`). Nur `FallBesitz`: Python prüft den
/// Besitz und lädt den Fall nicht, ein fehlender Fall ist dort 200, kein 404. Der Rumpf kommt als
/// [`KoerperRoh`], damit `flow.jsonl` die Schlüssel in der Reihenfolge des Clients behält.
///
/// # Errors
/// Die Fehler des Owner-Checks (401/403) und 400 aus `flow.melde_ui`.
pub async fn flow_melden(
    State(z): State<Zustand>,
    fall: FallBesitz,
    Koerper(wert): Koerper,
    KoerperRoh(roh): KoerperRoh,
) -> Result<Antwort, ApiFehler> {
    // Dieselben Bytes hat der Dispatcher schon als `Value` gelesen; `PyWert` liest sie noch einmal,
    // mit Einfuegereihenfolge. Das Fallback ist nie noetig, haelt aber jede Abweichung der beiden
    // Leser als Wert statt als Absturz.
    let body = serde_json::from_slice::<PyWert>(&roh).unwrap_or_else(|_| PyWert::from(wert));
    crate::flow::melde_ui(&z.konfig.audit_dir, fall.id().as_str(), &body)
}

/// `POST /fall/{id}/entfernung` — `api.entfernung` (`api.py:883`).
///
/// # Errors
/// Wie [`event`].
pub async fn entfernung(_fall: EigenerFall, _body: Koerper) -> Result<Antwort, ApiFehler> {
    Ok(Antwort::nicht_portiert("POST /fall/{id}/entfernung"))
}

/// `POST /fall/{id}/vorjahr` — `api.vorjahr` (`api.py:928`), Rumpf in [`crate::vorjahr`]. Die zweite
/// Fall-Kennung des Rumpfs (`vorjahr_fall_id`) geht durch `EigenerFall::pruefe`, dafür der [`Nutzer`].
///
/// # Errors
/// Wie [`event`], dazu 400 für die Kennung der Quelle und 422 für eine Abweisung des Stores.
pub async fn vorjahr(
    State(z): State<Zustand>,
    nutzer: Nutzer,
    mut fall: EigenerFall,
    Koerper(wert): Koerper,
) -> Result<Antwort, ApiFehler> {
    crate::vorjahr::vorjahr(&z, &nutzer, &mut fall, &wert)
}

/// `POST /fall/{id}/kontoauszug` — `api.kontoauszug` (`api.py:956`), Rumpf in [`crate::kontoauszug`].
///
/// # Errors
/// Wie [`event`].
pub async fn kontoauszug(
    State(z): State<Zustand>,
    mut fall: EigenerFall,
    Koerper(wert): Koerper,
) -> Result<Antwort, ApiFehler> {
    crate::kontoauszug::kontoauszug(&z, &mut fall, &wert)
}
