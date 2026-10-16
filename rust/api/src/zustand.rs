//! Geteilter Dienstzustand und die Request-Kontexte, die der Dispatcher an den Handler reicht.
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, OnceLock};

use auth::Auth;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use bescheid::deklaration::{scheibe_bindung, Cfg};
use bescheid::BindungIndex;
use bindung::{Params, Registry};
use interview::{Graph, Sicht};
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
    params: Arc<OnceLock<Params>>,
}

/// `TR.lade_bindung()` und `api._datei_felder`: alle `bindung_*.yaml`, einmal je Prozess geladen.
struct Bindungen {
    registry: Registry,
    /// Dateiname → `feld_id`s in Dateireihenfolge.
    je_datei: HashMap<String, Vec<String>>,
}

/// Was `_cfg(store)` und `_scheibe_bindung(store)` zusammen liefern, in den Formen, die ihre
/// Leser brauchen: `index` fuer `bescheid`/`elster` (Schluesselmenge), `sicht` fuer `interview`
/// (Scheibenreihenfolge, Python-`dict`), `graph` fuer die bindungsweiten Tabellen.
#[derive(Debug)]
pub struct ScheibenBindung<'a> {
    pub cfg: Cfg,
    /// Die Felder der Scheibe in Scheibenreihenfolge (`_scheibe_felder`).
    pub felder: Vec<String>,
    pub index: BindungIndex<'a>,
    pub sicht: Sicht<'a>,
    pub graph: Graph<'a>,
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
            params: Arc::new(OnceLock::new()),
        }
    }

    /// `_cfg(store)` und `_scheibe_bindung(store)` (`api.py:171-195`): die Scheibe der Akte und die
    /// Bindung ihrer Felder. `n_vor_gwg` liest seine Felder aus der YAML (`_datei_felder`).
    ///
    /// # Errors
    /// 400 bei unbekannter Scheibe, 500 bei unvollstaendiger Bindung, beide mit Pythons Wortlaut;
    /// eine unlesbare Bindung ist eine unerwartete Ausnahme.
    pub fn scheibe_bindung(&self, store: &Store) -> Result<ScheibenBindung<'_>, ApiFehler> {
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
            let pfad = self
                .konfig
                .wurzel
                .join(bindung::BINDUNG_VERZEICHNIS)
                .join(datei);
            return Err(ApiFehler::unerwartet(
                "FileNotFoundError",
                format!(
                    "[Errno 2] No such file or directory: {}",
                    repr(&json!(pfad.to_string_lossy()))
                ),
            ));
        }
        let nachschlag: BindungIndex<'_> = b
            .registry
            .dateien
            .iter()
            .flat_map(|(_, d)| &d.bindungen)
            .map(|x| (x.feld_id.clone(), x))
            .collect();
        let index = scheibe_bindung(&felder, &nachschlag)?;
        let graph = Graph::aus_registry(&b.registry);
        let sicht = graph
            .sicht(felder.iter().map(String::as_str))
            .map_err(|e| ApiFehler::unerwartet("KeyError", e.to_string()))?;
        Ok(ScheibenBindung {
            cfg,
            felder,
            index,
            sicht,
            graph,
        })
    }

    fn bindungen(&self) -> Result<&Bindungen, ApiFehler> {
        if let Some(b) = self.bindungen.get() {
            return Ok(b);
        }
        // PARITÄT: Python meldet hier den Typ der YAML- oder OS-Ausnahme; die Registry prueft
        // strenger (doppelte `feld_id`). Beides trifft nur ein kaputtes Repo.
        // Der Dienst liest `rust/bindung/daten` (Rust-Besitz, Weg B voll, 2026-10-05), nicht mehr
        // `produkt/bindung`. Python liest dieses Verzeichnis nicht.
        let registry = bindung::lade_registry_der_wurzel(&self.konfig.wurzel)
            .map_err(|e| ApiFehler::unerwartet("OSError", e.to_string()))?;
        let je_datei = registry
            .dateien
            .iter()
            .map(|(pfad, datei)| {
                let name = pfad.file_name().map(|n| n.to_string_lossy().into_owned());
                let ids = datei.bindungen.iter().map(|x| x.feld_id.clone()).collect();
                (name.unwrap_or_default(), ids)
            })
            .collect();
        Ok(self
            .bindungen
            .get_or_init(|| Bindungen { registry, je_datei }))
    }

    /// Der Feld-Katalog der Vorschlags-Schreiber (`ST.lade_katalog(TR.lade_bindung())`,
    /// `api.py:531`): GLOBAL ueber alle Bindungsdateien, nicht je Scheibe — die Freigabe haengt am
    /// Feld, nicht an der Scheibe.
    ///
    /// # Errors
    /// 500, wenn `rust/bindung/daten` nicht lesbar ist.
    pub fn katalog(&self) -> Result<store::Katalog, ApiFehler> {
        let b = self.bindungen()?;
        Ok(store::Katalog::aus_bindungen(
            b.registry.dateien.iter().flat_map(|(_, d)| &d.bindungen),
        ))
    }

    /// Die Jahresparameter aus `params/`, einmal je Prozess geladen.
    ///
    /// # Errors
    /// Eine unlesbare oder unvollstaendige `params/`-Datei ist eine unerwartete Ausnahme.
    pub fn params(&self) -> Result<&Params, ApiFehler> {
        if let Some(p) = self.params.get() {
            return Ok(p);
        }
        let p = Params::lade(&self.konfig.wurzel)
            .map_err(|e| ApiFehler::unerwartet("OSError", e.to_string()))?;
        Ok(self.params.get_or_init(|| p))
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

/// Der POST-Rumpf als Bytes, so wie er ankam. Fuer die Routen, die den Rumpf mit Pythons
/// Schluesselreihenfolge brauchen (`flow`): [`Koerper`] ist ein `serde_json::Value` und sortiert.
#[derive(Debug, Clone)]
pub struct KoerperRoh(pub axum::body::Bytes);

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

impl FromRequestParts<Zustand> for KoerperRoh {
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
