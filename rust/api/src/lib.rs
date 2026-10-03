//! `api` — der HTTP-Dienst (`produkt/haut/server.py`, `api.py`, `api_auth.py`): axum statt
//! `http.server`, dieselben 24 Routen, dieselben Grenzen, derselbe Wortlaut.
//!
//! Aufbau: [`dispatch::dispatch`] ist `Handler._dispatch` als Middleware um den Router aus
//! [`routen::router`]; [`eigener_fall::EigenerFall`] ist der Owner-Check als Extractor.
//! Schritt 9a: `/health`, `/ready`, `/auth/*`, `POST /fall`, `DELETE /fall/{id}` und die
//! `elster-ampel` sind fertig, alle übrigen Fall-Routen antworten `501 nicht_portiert`.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

pub mod antwort;
mod anzeige;
pub mod deklaration;
pub mod dispatch;
pub mod eigener_fall;
pub mod entfernung;
pub mod enum_labels;
pub mod ergebnis;
pub mod event;
pub mod fehler;
pub mod flow;
pub mod fragen;
pub mod graph;
pub mod konfig;
pub mod openapi;
pub mod preflight;
pub mod python;
pub mod routen;
pub mod schema;
pub mod stand;
pub mod warum;
pub mod zustand;

pub use antwort::Antwort;
pub use eigener_fall::{EigenerFall, FallBesitz};
pub use fehler::ApiFehler;
pub use zustand::{Koerper, KoerperRoh, Nutzer, Treffer, Zustand};

/// Der fertige Dienst: Pfad-Normalisierung, darunter der Router mit allen Routen, darum der
/// Dispatcher (auch um den Fallback). Für `axum::serve` mit `axum::ServiceExt::into_make_service`.
pub fn app(
    zustand: Zustand,
) -> impl tower::Service<
    axum::extract::Request,
    Response = axum::response::Response,
    Error = std::convert::Infallible,
    Future: Send,
> + Clone
       + Send {
    let router = routen::router()
        .fallback(dispatch::nicht_erreicht)
        .layer(axum::middleware::from_fn_with_state(
            zustand.clone(),
            dispatch::dispatch,
        ))
        .with_state(zustand);
    tower::ServiceExt::map_request(router, dispatch::normalisiere_pfad)
}
