//! `POST /fall` und `DELETE /fall/{id}` (`api.fall_anlegen`, `api.fall_loeschen`).
use std::collections::BTreeSet;

use auth::Username;
use axum::extract::State;
use domain::FallId;
use serde_json::{json, Map, Value};
use store::audit::{anhaengen, AuditAktion};
use store::Store;

use crate::antwort::Antwort;
use crate::eigener_fall::{auth_uid_oder_401, fall_kennung, fall_pfad, ist_fall_id, EigenerFall};
use crate::fehler::ApiFehler;
use crate::konfig::Konfig;
use crate::python;
use crate::zustand::{Koerper, Nutzer, Zustand};

/// Die Schlüssel von `SCHEIBEN` (`api_constants.py:794`). Der Differenz-Harness legt je Scheibe
/// einen Fall an und vergleicht mit Python.
pub const SCHEIBEN: [&str; 5] = ["ep", "n_vor_gwg", "an_gesamt", "gesamt", "rentner_gesamt"];

fn scheibe_pruefen(roh: &Value) -> Result<&str, ApiFehler> {
    if matches!(roh, Value::Array(_) | Value::Object(_)) {
        return Err(ApiFehler::unerwartet(
            "TypeError",
            python::unhashbar(roh, "dict key"),
        ));
    }
    roh.as_str()
        .filter(|s| SCHEIBEN.contains(s))
        .ok_or_else(|| ApiFehler::status(400, format!("unbekannte Scheibe {}", python::repr(roh))))
}

/// Jahre mit Verzeichnis unter `params/` (`os.listdir(ROOT/params)`, Name nur aus Ziffern).
fn verfuegbare_jahre(konfig: &Konfig) -> Result<BTreeSet<i64>, ApiFehler> {
    let mut jahre = BTreeSet::new();
    for eintrag in std::fs::read_dir(konfig.wurzel.join("params"))? {
        let name = eintrag?.file_name().to_string_lossy().into_owned();
        if !name.is_empty() && name.bytes().all(|b| b.is_ascii_digit()) {
            jahre.extend(name.parse::<i64>());
        }
    }
    Ok(jahre)
}

fn vz_pruefen(konfig: &Konfig, obj: &Map<String, Value>) -> Result<i64, ApiFehler> {
    let roh = obj
        .get("veranlagungszeitraum")
        .cloned()
        .unwrap_or_else(|| json!(2025));
    let Some(text) = python::int(&roh) else {
        return Err(ApiFehler::status(
            400,
            format!("veranlagungszeitraum {} ist keine Zahl", python::repr(&roh)),
        ));
    };
    let jahre = verfuegbare_jahre(konfig)?;
    if let Some(j) = text.parse::<i64>().ok().filter(|j| jahre.contains(j)) {
        return Ok(j);
    }
    let liste = jahre
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    Err(ApiFehler::status(
        400,
        format!("veranlagungszeitraum {text} hat keine Parameter — verfügbar: [{liste}]"),
    ))
}

fn fall_id_pruefen(obj: &Map<String, Value>) -> Result<FallId, ApiFehler> {
    let roh = obj.get("fall_id").unwrap_or(&Value::Null);
    if !python::wahr(roh) || !ist_fall_id(&python::text(roh)) {
        return Err(ApiFehler::status(
            400,
            "fall_id fehlt oder ungültig (nur [A-Za-z0-9_-]{1,64})",
        ));
    }
    // PARITÄT: `str(fall_id)` besteht das Muster auch für `123` oder `true`; erst `_fall_pfad`
    // wirft dann `TypeError` (500).
    let text = roh.as_str().ok_or_else(|| {
        ApiFehler::unerwartet(
            "TypeError",
            format!(
                "expected string or bytes-like object, got '{}'",
                python::typname(roh)
            ),
        )
    })?;
    fall_kennung(text)
}

/// `fall_anlegen(body)` (`api.py:277`): Auth, Eingaben, Datei, Audit.
///
/// # Errors
/// 401 ohne Nutzer, 400 bei ungültigen Eingaben, 409 bei vorhandenem Fall, 500 bei I/O.
pub fn fall_anlegen(z: &Zustand, nutzer: &Nutzer, body: &Value) -> Result<Antwort, ApiFehler> {
    let uid = auth_uid_oder_401(nutzer)?;
    let Value::Object(obj) = body else {
        return Err(ApiFehler::unerwartet(
            "AttributeError",
            format!("'{}' object has no attribute 'get'", python::typname(body)),
        ));
    };
    let standard = Value::String("ep".into());
    let scheibe = scheibe_pruefen(obj.get("scheibe").unwrap_or(&standard))?.to_owned();
    let vz = vz_pruefen(&z.konfig, obj)?;
    let fall_id = fall_id_pruefen(obj)?;
    let pfad = fall_pfad(&z.konfig, &fall_id);
    if pfad.exists() {
        return Err(ApiFehler::status(
            409,
            format!(
                "Fall {} existiert bereits",
                python::repr(&fall_id.as_str().into())
            ),
        ));
    }
    let mut datei = Store::leer(vz, Some(fall_id.to_string())).into_datei();
    datei.scheibe = Some(scheibe.clone());
    datei.user_id = uid.as_ref().map(Username::to_string);
    store::speichere(&pfad, &datei)?;
    if let Some(uid) = &uid {
        anhaengen(
            &z.konfig.audit_pfad(),
            Some(uid.as_str()),
            AuditAktion::FallAngelegt,
            Some(fall_id.as_str()),
            Some(&format!("scheibe={scheibe}")),
        )?;
    }
    Ok(Antwort::neu(
        201,
        json!({ "fall_id": fall_id.as_str(), "scheibe": scheibe, "veranlagungszeitraum": vz }),
    ))
}

/// `POST /fall` — `api.fall_anlegen`.
///
/// # Errors
/// Wie [`fall_anlegen`].
#[utoipa::path(
    post,
    path = "/fall",
    request_body = crate::schema::FallAnlegen,
    responses(
        (status = 201, description = "Fall angelegt", body = crate::schema::FallAngelegt),
        (status = 400, description = "Eingabe ungültig", body = crate::schema::Fehler),
        (status = 401, description = "Nicht angemeldet", body = crate::schema::Fehler),
        (status = 409, description = "Fall existiert bereits", body = crate::schema::Fehler)
    )
)]
pub async fn anlegen(
    State(z): State<Zustand>,
    nutzer: Nutzer,
    Koerper(body): Koerper,
) -> Result<Antwort, ApiFehler> {
    fall_anlegen(&z, &nutzer, &body)
}

/// `DELETE /fall/{id}` — `api.fall_loeschen` (`api.py:228`): Datei entfernen, Audit behalten.
///
/// # Errors
/// 401/403/404 aus dem Owner-Check, 500 bei I/O.
#[utoipa::path(
    delete,
    path = "/fall/{id}",
    params(("id" = String, Path, description = "Fall-Kennung")),
    responses(
        (status = 200, description = "Fall gelöscht", body = crate::schema::FallGeloescht),
        (status = 401, description = "Nicht angemeldet", body = crate::schema::Fehler),
        (status = 403, description = "Fremder Fall", body = crate::schema::Fehler),
        (status = 404, description = "Fall existiert nicht", body = crate::schema::Fehler)
    )
)]
pub async fn loeschen(State(z): State<Zustand>, fall: EigenerFall) -> Result<Antwort, ApiFehler> {
    let datei = fall.store().datei();
    let scheibe = datei.scheibe.clone();
    let vz = datei.veranlagungszeitraum.0;
    std::fs::remove_file(fall.pfad())?;
    let detail = format!("scheibe={}, vz={vz}", scheibe.as_deref().unwrap_or("None"));
    anhaengen(
        &z.konfig.audit_pfad(),
        Some(fall.nutzer().map_or("unbekannt", Username::as_str)),
        AuditAktion::FallGeloescht,
        Some(fall.id().as_str()),
        Some(&detail),
    )?;
    // PARITÄT-Grenze: ein Jahr außerhalb von i64 (gemessen: eine Fall-Datei mit 10^38) trägt
    // `serde_json` ohne `arbitrary_precision` nicht als Ganzzahl; dort kommt ein Float zurück.
    #[allow(clippy::cast_precision_loss)] // bewusst: die Grenze steht im Kommentar darüber
    let vz_json = i64::try_from(vz).map_or_else(|_| json!(vz as f64), |j| json!(j));
    Ok(Antwort::neu(
        200,
        json!({ "geloescht": true, "fall_id": fall.id().as_str(), "scheibe": scheibe, "veranlagungszeitraum": vz_json }),
    ))
}
