//! Die Routentabelle: `_routes()` (`server.py:62`), EINE Quelle für Regex-Dispatch und axum-Router.
//!
//! Ein Handler gehört in das Modul seiner Gruppe — `lesen` (GET auf einen Fall), `schreiben` (POST
//! auf einen Fall), `betrieb`, `anmeldung`, `fall`. Diese Tabelle bleibt unberührt, solange der
//! Funktionsname gleich bleibt (die Argumente des Handlers sind axum-Extractor und dürfen sich
//! ändern).
use axum::routing::{delete, get, post};
use axum::Router;

use crate::zustand::Zustand;

pub mod anmeldung;
pub mod betrieb;
pub mod fall;
pub mod lesen;
pub mod schreiben;

/// Eine Zeile aus `_routes()`.
#[derive(Debug)]
pub struct Eintrag {
    pub methode: &'static str,
    /// Das Python-Regex, Zeichen für Zeichen (`^…$`, `_ID`, `_FID`).
    pub muster: &'static str,
    /// Fehlerlog-Ort `server.dispatch <muster>` (`server.py:246`).
    pub ort: &'static str,
    /// Derselbe Pfad in axum-Schreibweise.
    pub axum_pfad: &'static str,
}

macro_rules! id {
    () => {
        "(?P<id>[A-Za-z0-9_-]{1,64})"
    };
}
// Die Obergrenze kommt aus `domain` (`max_feld_id_laenge!`), damit Route und Registry-Test
// (`rust/bindung/tests/registry_quer.rs`) dieselbe Zahl lesen.
macro_rules! fid {
    () => {
        concat!(
            "(?P<fid>[A-Za-z0-9_]{1,",
            domain::max_feld_id_laenge!(),
            "})"
        )
    };
}

macro_rules! tabelle {
    ($( ($axum_fn:ident, $methode:literal, $muster:expr, $pfad:literal, $handler:path) ),+ $(,)?) => {
        /// Alle 24 Routen in Pythons Reihenfolge.
        pub const EINTRAEGE: &[Eintrag] = &[
            $( Eintrag {
                methode: $methode,
                muster: $muster,
                ort: concat!("server.dispatch ", $muster),
                axum_pfad: $pfad,
            } ),+
        ];

        /// Der axum-Router mit allen 24 Routen; Methode und Pfad stehen in [`EINTRAEGE`].
        pub fn router() -> Router<Zustand> {
            Router::new()$( .route($pfad, $axum_fn($handler)) )+
        }
    };
}

tabelle![
    (get, "GET", "^/health$", "/health", betrieb::health),
    (get, "GET", "^/ready$", "/ready", betrieb::ready),
    (
        post,
        "POST",
        "^/auth/register$",
        "/auth/register",
        anmeldung::register
    ),
    (
        post,
        "POST",
        "^/auth/login$",
        "/auth/login",
        anmeldung::login
    ),
    (
        post,
        "POST",
        "^/auth/logout$",
        "/auth/logout",
        anmeldung::logout
    ),
    (
        get,
        "GET",
        "^/auth/session$",
        "/auth/session",
        anmeldung::session
    ),
    (post, "POST", "^/fall$", "/fall", fall::anlegen),
    (
        get,
        "GET",
        concat!("^/fall/", id!(), "/fragen$"),
        "/fall/{id}/fragen",
        lesen::fragen
    ),
    (
        get,
        "GET",
        concat!("^/fall/", id!(), "/stand$"),
        "/fall/{id}/stand",
        lesen::stand
    ),
    (
        post,
        "POST",
        concat!("^/fall/", id!(), "/event$"),
        "/fall/{id}/event",
        schreiben::event
    ),
    (
        get,
        "GET",
        concat!("^/fall/", id!(), "/feld/", fid!(), "/warum$"),
        "/fall/{id}/feld/{fid}/warum",
        lesen::warum
    ),
    (
        get,
        "GET",
        concat!("^/fall/", id!(), "/feld/", fid!(), "/frage$"),
        "/fall/{id}/feld/{fid}/frage",
        lesen::frage_einzeln
    ),
    (
        get,
        "GET",
        concat!("^/fall/", id!(), "/ergebnis$"),
        "/fall/{id}/ergebnis",
        lesen::ergebnis
    ),
    (
        get,
        "GET",
        concat!("^/fall/", id!(), "/preflight$"),
        "/fall/{id}/preflight",
        lesen::preflight
    ),
    (
        get,
        "GET",
        concat!("^/fall/", id!(), "/deklaration$"),
        "/fall/{id}/deklaration",
        lesen::deklaration
    ),
    (
        post,
        "POST",
        concat!("^/fall/", id!(), "/einreichen$"),
        "/fall/{id}/einreichen",
        schreiben::einreichen
    ),
    (
        get,
        "GET",
        concat!("^/fall/", id!(), "/graph$"),
        "/fall/{id}/graph",
        lesen::graph
    ),
    (
        post,
        "POST",
        concat!("^/fall/", id!(), "/elster-ampel$"),
        "/fall/{id}/elster-ampel",
        betrieb::elster_ampel
    ),
    (
        post,
        "POST",
        concat!("^/fall/", id!(), "/chat$"),
        "/fall/{id}/chat",
        schreiben::chat
    ),
    (
        post,
        "POST",
        concat!("^/fall/", id!(), "/flow$"),
        "/fall/{id}/flow",
        schreiben::flow_melden
    ),
    (
        post,
        "POST",
        concat!("^/fall/", id!(), "/entfernung$"),
        "/fall/{id}/entfernung",
        schreiben::entfernung
    ),
    (
        post,
        "POST",
        concat!("^/fall/", id!(), "/vorjahr$"),
        "/fall/{id}/vorjahr",
        schreiben::vorjahr
    ),
    (
        post,
        "POST",
        concat!("^/fall/", id!(), "/kontoauszug$"),
        "/fall/{id}/kontoauszug",
        schreiben::kontoauszug
    ),
    (
        delete,
        "DELETE",
        concat!("^/fall/", id!(), "$"),
        "/fall/{id}",
        fall::loeschen
    ),
];

#[cfg(test)]
mod tests {
    use super::EINTRAEGE;

    /// Das URL-Muster der Feld-Kennung liest die Obergrenze aus `domain`, dieselbe Zahl wie der
    /// Registry-Test `rust/bindung/tests/registry_quer.rs`. Ein Wortlaut `{1,64}` im Muster waere
    /// eine zweite Quelle, die beim naechsten Aendern der Zahl still abweicht.
    #[test]
    fn feld_kennung_im_muster_hat_die_obergrenze_aus_domain() {
        let erwartet = format!("(?P<fid>[A-Za-z0-9_]{{1,{}}})", domain::MAX_FELD_ID_LAENGE);
        let mit_fid: Vec<_> = EINTRAEGE
            .iter()
            .filter(|e| e.muster.contains("(?P<fid>"))
            .collect();
        assert!(mit_fid.len() >= 2, "nur {} Routen mit fid", mit_fid.len());
        for e in mit_fid {
            assert!(e.muster.contains(&erwartet), "{}: {}", e.ort, e.muster);
        }
    }
}
