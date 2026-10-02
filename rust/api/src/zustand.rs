//! Geteilter Dienstzustand und die Request-Kontexte, die der Dispatcher an den Handler reicht.
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, OnceLock};

use auth::Auth;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use bescheid::deklaration::{scheibe_bindung, Cfg};
use bescheid::BindungIndex;
use bindung::Bindung;
use serde_json::{json, Value};
use store::Store;

use crate::fehler::ApiFehler;
use crate::konfig::Konfig;
use crate::python::repr;

/// Was jeder Handler sieht: Pfade, Auth, und die Sperre, die den Dienst wie Pythons
/// einfädigen `HTTPServer` Anfrage für Anfrage arbeiten lässt.
#[derive(Clone)]
pub struct Zustand {
    pub konfig: Arc<Konfig>,
    pub auth: Arc<Auth>,
    pub(crate) sperre: Arc<tokio::sync::Mutex<()>>,
    bindungen: Arc<OnceLock<Bindungen>>,
}

/// `TR.lade_bindung()` und `api._datei_felder`: alle `bindung_*.yaml`, einmal je Prozess geladen.
struct Bindungen {
    alle: Vec<Bindung>,
    /// Dateiname → `feld_id`s in Dateireihenfolge.
    je_datei: HashMap<String, Vec<String>>,
}

impl Zustand {
    // ponytail: eine globale Sperre wie `make_server` (Catala-Laufzeit ist nicht thread-sicher,
    // `server.py:278-286`); feiner, sobald `catala-sys` unter Last gemessen ist.
    #[must_use]
    pub fn neu(konfig: Konfig, auth: Auth) -> Self {
        Self {
            konfig: Arc::new(konfig),
            auth: Arc::new(auth),
            sperre: Arc::new(tokio::sync::Mutex::new(())),
            bindungen: Arc::new(OnceLock::new()),
        }
    }

    /// `_cfg(store)` und `_scheibe_bindung(store)` (`api.py:171-195`): die Scheibe der Akte und die
    /// Bindung ihrer Felder. `n_vor_gwg` liest seine Felder aus der YAML (`_datei_felder`).
    ///
    /// # Errors
    /// 400 bei unbekannter Scheibe, 500 bei unvollstaendiger Bindung, beide mit Pythons Wortlaut;
    /// eine unlesbare Bindung ist eine unerwartete Ausnahme.
    pub fn scheibe_bindung(&self, store: &Store) -> Result<(Cfg, BindungIndex<'_>), ApiFehler> {
        let cfg = Cfg::der_akte(store)?;
        let b = self.bindungen()?;
        // Eine fehlende Datei ist `FileNotFoundError` wie bei Pythons `open`, nie eine leere
        // Feldliste: die gaebe eine Scheibe ohne Felder statt eines Fehlers.
        let mut fehlt = None;
        let felder = cfg
            .felder(|d| {
                b.je_datei.get(d).cloned().unwrap_or_else(|| {
                    fehlt = Some(d.to_owned());
                    Vec::new()
                })
            })
            .map_err(|e| ApiFehler::unerwartet("KeyError", e.to_string()))?;
        if let Some(datei) = fehlt {
            let pfad = self.konfig.wurzel.join("produkt").join("bindung").join(datei);
            return Err(ApiFehler::unerwartet(
                "FileNotFoundError",
                format!(
                    "[Errno 2] No such file or directory: {}",
                    repr(&json!(pfad.to_string_lossy()))
                ),
            ));
        }
        let bindung = scheibe_bindung(&felder, &store::baue_nachschlag(&b.alle))?;
        Ok((cfg, bindung))
    }

    fn bindungen(&self) -> Result<&Bindungen, ApiFehler> {
        if let Some(b) = self.bindungen.get() {
            return Ok(b);
        }
        // PARITÄT: Python meldet hier den Typ der YAML- oder OS-Ausnahme; die Registry prueft
        // strenger (doppelte `feld_id`). Beides trifft nur ein kaputtes Repo.
        let reg = bindung::lade_registry(&self.konfig.wurzel.join("produkt").join("bindung"))
            .map_err(|e| ApiFehler::unerwartet("OSError", e.to_string()))?;
        let mut b = Bindungen {
            alle: Vec::new(),
            je_datei: HashMap::new(),
        };
        for (pfad, datei) in reg.dateien {
            let name = pfad.file_name().map(|n| n.to_string_lossy().into_owned());
            let ids = datei.bindungen.iter().map(|x| x.feld_id.clone()).collect();
            b.je_datei.insert(name.unwrap_or_default(), ids);
            b.alle.extend(datei.bindungen);
        }
        Ok(self.bindungen.get_or_init(|| b))
    }

    /// Zustand aus der Umgebung: `TAXGRAPH_JWT_SECRET`, `TAXGRAPH_USER_STORE` wie `auth.py:18-23`.
    ///
    /// # Errors
    /// Wenn die Zufallsquelle für das Geheimnis versagt.
    pub fn aus_env() -> Result<Self, auth::AuthFehler> {
        let konfig = Konfig::aus_env();
        let auth = Auth::aus_env(&konfig.nutzerdatei(), Some(konfig.audit_pfad()))?;
        Ok(Self::neu(konfig, auth))
    }
}

/// Der Nutzer dieser Anfrage (`api_auth._AUTH_USER`): `None`, wenn kein gültiges Token kam.
#[derive(Debug, Clone)]
pub struct Nutzer(pub Option<String>);

/// Der gelesene JSON-Rumpf (`body`); `{}` ohne Rumpf, sonst was immer `json.loads` lieferte.
#[derive(Debug, Clone)]
pub struct Koerper(pub Value);

/// Das Routenmuster, das gegriffen hat, und seine Treffergruppen (`treffer.groupdict()`).
#[derive(Debug, Clone)]
pub struct Treffer {
    /// Python-Muster; Teil des Fehlerlog-Orts `server.dispatch <muster>`.
    pub ort: &'static str,
    pub id: Option<String>,
    pub fid: Option<String>,
}

fn aus_erweiterung<T: Clone + Send + Sync + 'static>(parts: &Parts) -> Result<T, ApiFehler> {
    parts.extensions.get::<T>().cloned().ok_or_else(|| {
        ApiFehler::unerwartet("RuntimeError", "Dispatcher hat den Kontext nicht gesetzt")
    })
}

impl FromRequestParts<Zustand> for Nutzer {
    type Rejection = ApiFehler;

    fn from_request_parts(
        parts: &mut Parts,
        _: &Zustand,
    ) -> impl Future<Output = Result<Self, ApiFehler>> {
        std::future::ready(aus_erweiterung(parts))
    }
}

impl FromRequestParts<Zustand> for Koerper {
    type Rejection = ApiFehler;

    fn from_request_parts(
        parts: &mut Parts,
        _: &Zustand,
    ) -> impl Future<Output = Result<Self, ApiFehler>> {
        std::future::ready(aus_erweiterung(parts))
    }
}

impl FromRequestParts<Zustand> for Treffer {
    type Rejection = ApiFehler;

    fn from_request_parts(
        parts: &mut Parts,
        _: &Zustand,
    ) -> impl Future<Output = Result<Self, ApiFehler>> {
        std::future::ready(aus_erweiterung(parts))
    }
}
