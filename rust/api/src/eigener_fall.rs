//! Der Owner-Check als Typ: Wer einen [`EigenerFall`] hat, ist angemeldet (oder im Opt-out),
//! der Fall existiert, und er gehört dem Nutzer. Ein Handler ohne diesen Extractor bekommt keinen
//! Fall — `api.py:114-127` prüft dagegen je Handler von Hand.
use std::path::PathBuf;

use auth::Username;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use domain::FallId;
use store::audit::AuditAktion;
use store::Store;

use crate::fehler::ApiFehler;
use crate::konfig::{no_auth, Konfig};
use crate::python;
use crate::zustand::{Nutzer, Treffer, Zustand};

/// Ein geprüfter, geladener Fall.
#[derive(Debug)]
pub struct EigenerFall {
    id: FallId,
    pfad: PathBuf,
    nutzer: Option<Username>,
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
/// PARITÄT-Grenze: Ein gültig signiertes Token, dessen `sub` `_USER_RE` verfehlt, zählt wie kein
/// Token (401, im Opt-out `None`). Python nimmt den Namen als uid an. Ein solches Token entsteht
/// nur aus einer von Hand geänderten `users.json`: die Registrierung prüft das Muster, der Login
/// nicht. Gemessen 2026-10-02: 0 von 3 Nutzern, 0 von 39 Akten betroffen. Festgehalten in
/// `api_http_paritaet::dokumentierte_abweichungen` und `tests/http.rs`.
///
/// # Errors
/// 401 `Authentifizierung erforderlich`.
pub fn auth_uid_oder_401(nutzer: &Nutzer) -> Result<Option<Username>, ApiFehler> {
    match nutzer.0.as_deref().map(Username::neu).and_then(Result::ok) {
        None if !no_auth() => Err(ApiFehler::status(401, "Authentifizierung erforderlich")),
        uid => Ok(uid),
    }
}

/// Die Prüfung aus `_fall_pfad` (`api.py:126`).
///
/// # Errors
/// 400 bei einer Kennung außerhalb von `[A-Za-z0-9_-]{1,64}`.
pub fn fall_kennung(roh: &str) -> Result<FallId, ApiFehler> {
    FallId::new(roh).map_err(|_| {
        ApiFehler::status(
            400,
            format!(
                "ungültige fall_id (nur [A-Za-z0-9_-]{{1,64}}): {}",
                python::repr(&roh.into())
            ),
        )
    })
}

/// `_fall_pfad` (`api.py:126`) nach der Prüfung, die [`fall_kennung`] übernimmt.
#[must_use]
pub fn fall_pfad(konfig: &Konfig, fall_id: &FallId) -> PathBuf {
    konfig.faelle.join(format!("{fall_id}.json"))
}

/// Ergebnis des Owner-Checks: Pfad und — falls die Datei existiert — der geladene Fall.
struct Geprueft {
    pfad: PathBuf,
    store: Option<Store>,
}

/// `_fall_owner_check` (`api.py:108`) nach 401 und 400: ein fehlender Fall ist KEIN Fehler (der
/// Aufrufer wirft 404, wenn er den Fall braucht); 500 bei kaputter Datei; 403 bei fremdem Besitzer.
fn besitz_pruefen(
    z: &Zustand,
    uid: Option<&Username>,
    fall_id: &FallId,
) -> Result<Geprueft, ApiFehler> {
    let pfad = fall_pfad(&z.konfig, fall_id);
    if !pfad.exists() {
        return Ok(Geprueft { pfad, store: None });
    }
    let store = Store::aus_datei(store::lade(&pfad)?);
    if let Some(uid) = uid {
        let besitzer = store.datei().user_id.as_deref();
        if besitzer != Some(uid.as_str()) {
            let detail = format!("user={uid}, owner={}", besitzer.unwrap_or("None"));
            store::audit::anhaengen(
                &z.konfig.audit_pfad(),
                Some(uid.as_str()),
                AuditAktion::ZugriffVerweigert,
                Some(fall_id.as_str()),
                Some(&detail),
            )?;
            return Err(ApiFehler::status(
                403,
                format!(
                    "Zugriff auf Fall {} verweigert",
                    python::repr(&fall_id.as_str().into())
                ),
            ));
        }
    }
    Ok(Geprueft {
        pfad,
        store: Some(store),
    })
}

/// Nutzer und Treffergruppe `id` in Pythons Reihenfolge: erst 401, dann 400.
async fn anmeldung_und_kennung(
    parts: &mut Parts,
    z: &Zustand,
) -> Result<(Option<Username>, FallId), ApiFehler> {
    let nutzer = Nutzer::from_request_parts(parts, z).await?;
    let treffer = Treffer::from_request_parts(parts, z).await?;
    let roh = treffer
        .id
        .ok_or_else(|| ApiFehler::unerwartet("KeyError", "'id'"))?;
    let uid = auth_uid_oder_401(&nutzer)?;
    Ok((uid, fall_kennung(&roh)?))
}

impl EigenerFall {
    /// `_fall_owner_check(fall_id)` gefolgt von `lade_fall(fall_id)`. Reihenfolge der Fehler wie
    /// in Python: 401, (400 prüft der Aufrufer mit [`fall_kennung`]), 404, (500 bei kaputter
    /// Datei), 403.
    ///
    /// `pub`, weil ein Handler mit einer zweiten Fall-Kennung im Rumpf (`vorjahr_fall_id`,
    /// `api.py:947`) sie ebenfalls prüfen muss.
    ///
    /// # Errors
    /// 401/404/403 wie oben; 500, wenn die Datei nicht lesbar oder kein gültiger Store ist.
    pub fn pruefe(z: &Zustand, nutzer: &Nutzer, fall_id: &FallId) -> Result<Self, ApiFehler> {
        let uid = auth_uid_oder_401(nutzer)?;
        Self::laden(z, uid, fall_id.clone())
    }

    fn laden(z: &Zustand, nutzer: Option<Username>, id: FallId) -> Result<Self, ApiFehler> {
        let g = besitz_pruefen(z, nutzer.as_ref(), &id)?;
        let store = g.store.ok_or_else(|| {
            ApiFehler::status(
                404,
                format!("Fall {} existiert nicht", python::repr(&id.as_str().into())),
            )
        })?;
        Ok(Self {
            id,
            pfad: g.pfad,
            nutzer,
            store,
        })
    }

    #[must_use]
    pub fn id(&self) -> &FallId {
        &self.id
    }

    #[must_use]
    pub fn pfad(&self) -> &std::path::Path {
        &self.pfad
    }

    /// Der angemeldete Nutzer; `None` nur im Opt-out `TAXGRAPH_NO_AUTH=1`.
    #[must_use]
    pub fn nutzer(&self) -> Option<&Username> {
        self.nutzer.as_ref()
    }

    #[must_use]
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Der Fall zum Schreiben; gespeichert wird danach mit [`store::speichere`] an [`Self::pfad`].
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }
}

impl FromRequestParts<Zustand> for EigenerFall {
    type Rejection = ApiFehler;

    async fn from_request_parts(parts: &mut Parts, z: &Zustand) -> Result<Self, ApiFehler> {
        let (uid, id) = anmeldung_und_kennung(parts, z).await?;
        Self::laden(z, uid, id)
    }
}

/// Nur der Besitz ist geprüft, der Fall selbst nicht geladen — Pythons `_fall_owner_check` ohne
/// anschließendes `lade_fall` (so `api.flow_melden`, `api.py:1298`: ein fehlender Fall ist dort
/// kein 404). Wer einen Fall braucht, nimmt [`EigenerFall`].
#[derive(Debug)]
pub struct FallBesitz {
    id: FallId,
    nutzer: Option<Username>,
}

impl FallBesitz {
    #[must_use]
    pub fn id(&self) -> &FallId {
        &self.id
    }

    /// Der angemeldete Nutzer; `None` nur im Opt-out `TAXGRAPH_NO_AUTH=1`.
    #[must_use]
    pub fn nutzer(&self) -> Option<&Username> {
        self.nutzer.as_ref()
    }
}

impl FromRequestParts<Zustand> for FallBesitz {
    type Rejection = ApiFehler;

    async fn from_request_parts(parts: &mut Parts, z: &Zustand) -> Result<Self, ApiFehler> {
        let (uid, id) = anmeldung_und_kennung(parts, z).await?;
        besitz_pruefen(z, uid.as_ref(), &id)?;
        Ok(Self { id, nutzer: uid })
    }
}
