//! OpenAPI-Dokument der in 9a fertigen Routen. Kein Endpunkt: Python kennt `/openapi.json` nicht
//! (dort wäre es ein 404), also liefert es nur `taxgraph-api --openapi`. Die Schemas der
//! Stub-Routen (`produkt/haut/api_schema/*.json`) tragen D2/D3 nach.
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
