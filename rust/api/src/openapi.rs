//! OpenAPI-Dokument der 9 Routen, die bisher beschrieben sind (`/health`, `/ready`, `/auth/*`,
//! `POST /fall`, `DELETE /fall/{id}`, `elster-ampel`); die 15 übrigen Routen sind portiert, aber
//! noch nicht im Dokument. Welche, hält `UNDOKUMENTIERT` in `tests/http.rs` fest. Kein Endpunkt:
//! Python kennt `/openapi.json` nicht (dort wäre es ein 404), also liefert es nur
//! `taxgraph-api --openapi`.
use utoipa::OpenApi;

use crate::routen::{anmeldung, betrieb, fall};
use crate::schema::{
    Abgemeldet, Abmeldung, AmpelNichtVerdrahtet, Angemeldet, Anmeldedaten, Bereitschaft,
    FallAngelegt, FallAnlegen, FallGeloescht, Fehler, Gesundheit, Registriert, Sitzung,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        betrieb::health,
        betrieb::ready,
        betrieb::elster_ampel,
        anmeldung::register,
        anmeldung::login,
        anmeldung::logout,
        anmeldung::session,
        fall::anlegen,
        fall::loeschen
    ),
    components(schemas(
        Abgemeldet,
        Abmeldung,
        AmpelNichtVerdrahtet,
        Angemeldet,
        Anmeldedaten,
        Bereitschaft,
        FallAngelegt,
        FallAnlegen,
        FallGeloescht,
        Fehler,
        Gesundheit,
        Registriert,
        Sitzung
    ))
)]
pub struct ApiDoc;
