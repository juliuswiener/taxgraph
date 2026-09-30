//! Der Owner-Check als Typ: Wer einen [`EigenerFall`] hat, ist angemeldet (oder im Opt-out),
//! der Fall existiert, und er gehört dem Nutzer. Ein Handler ohne diesen Extractor bekommt keinen
//! Fall — `api.py:114-127` prüft dagegen je Handler von Hand.
use std::path::PathBuf;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use store::audit::AuditAktion;
use store::Store;

use crate::fehler::ApiFehler;
use crate::konfig::{no_auth, Konfig};
use crate::python;
use crate::zustand::{Nutzer, Treffer, Zustand};

/// Ein geprüfter, geladener Fall.
#[derive(Debug)]
pub struct EigenerFall {
    id: String,
    pfad: PathBuf,
    nutzer: Option<String>,
    store: Store,
}

/// `_FALL_RE.fullmatch` (`api_constants.py:48`): `[A-Za-z0-9_-]{1,64}`.
#[must_use]
pub fn ist_fall_id(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// `_auth_uid_oder_401` (`api.py:93`): kein Nutzer ist 401, außer im Opt-out; dort `None`.
///
/// # Errors
/// 401 `Authentifizierung erforderlich`.
pub fn auth_uid_oder_401(nutzer: &Nutzer) -> Result<Option<String>, ApiFehler> {
    match &nutzer.0 {
        None if !no_auth() => Err(ApiFehler::status(401, "Authentifizierung erforderlich")),
        uid => Ok(uid.clone()),
    }
}

/// `_fall_pfad` (`api.py:126`).
///
/// # Errors
/// 400 bei einer Kennung außerhalb von `[A-Za-z0-9_-]{1,64}`.
pub fn fall_pfad(konfig: &Konfig, fall_id: &str) -> Result<PathBuf, ApiFehler> {
    if !ist_fall_id(fall_id) {
        return Err(ApiFehler::status(
            400,
            format!(
                "ungültige fall_id (nur [A-Za-z0-9_-]{{1,64}}): {}",
                python::repr(&fall_id.into())
            ),
        ));
    }
    Ok(konfig.faelle.join(format!("{fall_id}.json")))
}

/// Ergebnis des Owner-Checks: Nutzer, Pfad und — falls die Datei existiert — der geladene Fall.
struct Geprueft {
    nutzer: Option<String>,
    pfad: PathBuf,
    store: Option<Store>,
}

/// `_fall_owner_check` (`api.py:108`): 401, 400; ein fehlender Fall ist KEIN Fehler (der Aufrufer
/// wirft 404, wenn er den Fall braucht); 500 bei kaputter Datei; 403 bei fremdem Besitzer.
fn besitz_pruefen(z: &Zustand, nutzer: &Nutzer, fall_id: &str) -> Result<Geprueft, ApiFehler> {
    let uid = auth_uid_oder_401(nutzer)?;
    let pfad = fall_pfad(&z.konfig, fall_id)?;
    if !pfad.exists() {
        return Ok(Geprueft {
            nutzer: uid,
            pfad,
            store: None,
        });
    }
    let store = Store::aus_datei(store::lade(&pfad)?);
    if let Some(uid) = &uid {
        let besitzer = store.datei().user_id.as_deref();
        if besitzer != Some(uid.as_str()) {
            let detail = format!("user={uid}, owner={}", besitzer.unwrap_or("None"));
            store::audit::anhaengen(
                &z.konfig.audit_pfad(),
                Some(uid),
                AuditAktion::ZugriffVerweigert,
                Some(fall_id),
                Some(&detail),
            )?;
            return Err(ApiFehler::status(
                403,
                format!(
                    "Zugriff auf Fall {} verweigert",
                    python::repr(&fall_id.into())
                ),
            ));
        }
    }
    Ok(Geprueft {
        nutzer: uid,
        pfad,
        store: Some(store),
    })
}

impl EigenerFall {
    /// `_fall_owner_check(fall_id)` gefolgt von `lade_fall(fall_id)`. Reihenfolge der Fehler wie
    /// in Python: 401, 400, 404, (500 bei kaputter Datei), 403.
    ///
    /// `pub`, weil ein Handler mit einer zweiten Fall-Kennung im Rumpf (`vorjahr_fall_id`,
    /// `api.py:947`) sie ebenfalls prüfen muss.
    ///
    /// # Errors
    /// 401/400/404/403 wie oben; 500, wenn die Datei nicht lesbar oder kein gültiger Store ist.
    pub fn pruefe(z: &Zustand, nutzer: &Nutzer, fall_id: &str) -> Result<Self, ApiFehler> {
        let g = besitz_pruefen(z, nutzer, fall_id)?;
        let store = g.store.ok_or_else(|| {
            ApiFehler::status(
                404,
                format!("Fall {} existiert nicht", python::repr(&fall_id.into())),
            )
        })?;
        Ok(Self {
            id: fall_id.to_owned(),
            pfad: g.pfad,
            nutzer: g.nutzer,
            store,
        })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn pfad(&self) -> &std::path::Path {
        &self.pfad
    }

    /// Der angemeldete Nutzer; `None` nur im Opt-out `TAXGRAPH_NO_AUTH=1`.
    #[must_use]
    pub fn nutzer(&self) -> Option<&str> {
        self.nutzer.as_deref()
    }

    #[must_use]
    pub fn store(&self) -> &Store {
        &self.store
    }
}

impl FromRequestParts<Zustand> for EigenerFall {
    type Rejection = ApiFehler;

    async fn from_request_parts(parts: &mut Parts, z: &Zustand) -> Result<Self, ApiFehler> {
        let Nutzer(nutzer) = Nutzer::from_request_parts(parts, z).await?;
        let treffer = Treffer::from_request_parts(parts, z).await?;
        let id = treffer
            .id
            .ok_or_else(|| ApiFehler::unerwartet("KeyError", "'id'"))?;
        Self::pruefe(z, &Nutzer(nutzer), &id)
    }
}

/// Nur der Besitz ist geprüft, der Fall selbst nicht geladen — Pythons `_fall_owner_check` ohne
/// anschließendes `lade_fall` (so `api.flow_melden`, `api.py:1298`: ein fehlender Fall ist dort
/// kein 404). Wer einen Fall braucht, nimmt [`EigenerFall`].
#[derive(Debug)]
pub struct FallBesitz {
    id: String,
    nutzer: Option<String>,
}

impl FallBesitz {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Der angemeldete Nutzer; `None` nur im Opt-out `TAXGRAPH_NO_AUTH=1`.
    #[must_use]
    pub fn nutzer(&self) -> Option<&str> {
        self.nutzer.as_deref()
    }
}

impl FromRequestParts<Zustand> for FallBesitz {
    type Rejection = ApiFehler;

    async fn from_request_parts(parts: &mut Parts, z: &Zustand) -> Result<Self, ApiFehler> {
        let nutzer = Nutzer::from_request_parts(parts, z).await?;
        let treffer = Treffer::from_request_parts(parts, z).await?;
        let id = treffer
            .id
            .ok_or_else(|| ApiFehler::unerwartet("KeyError", "'id'"))?;
        let g = besitz_pruefen(z, &nutzer, &id)?;
        Ok(Self {
            id,
            nutzer: g.nutzer,
        })
    }
}
