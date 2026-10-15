//! Konfiguration über dieselben Umgebungsvariablen wie Python.
//!
//! | Variable | Python | wirkt auf |
//! |---|---|---|
//! | `TAXGRAPH_DATEN`, `XDG_DATA_HOME` | `api_constants._daten_wurzel` | Fall-Verzeichnis `…/faelle` |
//! | `TAXGRAPH_AUDIT_DIR` | `audit.AUDIT_DIR` | Verzeichnis von `audit.jsonl` und `fehler.log` |
//! | `TAXGRAPH_USER_STORE`, `TAXGRAPH_JWT_SECRET` | `auth.py:18-23` | Nutzerdatei, Signaturschlüssel (in `auth::Auth::aus_env`) |
//! | `TAXGRAPH_NO_AUTH` | `api._auth_uid_oder_401` | Einzelnutzer-Opt-out, bei JEDEM Aufruf gelesen |
//! | `TAXGRAPH_FLOW`, `TAXGRAPH_KI_DEBUG` | `flow.an` | Flag in `/health`, bei JEDEM Aufruf gelesen |
//! | `TAXGRAPH_ROOT` (neu) | `ROOT = dirname(PRODUKT)` | Repo-Wurzel für `params/` und `rust/api/static/` |
use std::path::{Path, PathBuf};

use store::fehler_log::{protokolliere, Meta, Stufe};

/// Pfade des Dienstes; alles, was Python beim Import festlegt.
#[derive(Debug, Clone)]
pub struct Konfig {
    /// Repo-Wurzel (`params/<VZ>/`, `rust/api/static/`, bis L3 der Rückgriff `produkt/auth/users.json`).
    pub wurzel: PathBuf,
    pub faelle: PathBuf,
    pub audit_dir: PathBuf,
}

/// `os.path.expanduser`: nur `~` und `~/…` (über `$HOME`); `~nutzer` bleibt unverändert.
fn expanduser(pfad: &str) -> PathBuf {
    let rest = pfad
        .strip_prefix('~')
        .filter(|r| r.is_empty() || r.starts_with('/'));
    match (rest, std::env::var_os("HOME")) {
        (Some(r), Some(home)) => {
            let mut p = PathBuf::from(home);
            if let Some(unter) = r.strip_prefix('/') {
                p.push(unter);
            }
            p
        }
        _ => PathBuf::from(pfad),
    }
}

/// `os.environ.get(name, "").strip()`. `py_strip` statt `str::trim`: Pythons `strip()` nimmt auch
/// U+001C..U+001F weg (`TAXGRAPH_FLOW=$'\x1c1\x1f'` ist dort `"1"`, gemessen 2026-10-02).
fn env_text(name: &str) -> String {
    domain::py_strip(&std::env::var(name).unwrap_or_default()).to_owned()
}

impl Konfig {
    /// Liest die Umgebung. Die Repo-Wurzel folgt dem Quelltext-Ort dieses Crates, außer
    /// `TAXGRAPH_ROOT` ist gesetzt.
    // ponytail: die Wurzel ist zur Bauzeit festgelegt; ein ausgeliefertes Binary setzt `TAXGRAPH_ROOT`.
    #[must_use]
    pub fn aus_env() -> Self {
        let wurzel = std::env::var("TAXGRAPH_ROOT")
            .ok()
            .filter(|v| !v.is_empty())
            .map_or_else(
                || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
                PathBuf::from,
            );
        let eigen = env_text("TAXGRAPH_DATEN");
        let xdg = env_text("XDG_DATA_HOME");
        let daten = if !eigen.is_empty() {
            expanduser(&eigen)
        } else if !xdg.is_empty() {
            expanduser(&xdg).join("taxgraph")
        } else {
            expanduser("~")
                .join(".local")
                .join("share")
                .join("taxgraph")
        };
        let faelle = daten.join("faelle");
        let audit_dir = std::env::var("TAXGRAPH_AUDIT_DIR")
            .ok()
            .filter(|v| !v.is_empty())
            .map_or_else(|| faelle.clone(), PathBuf::from);
        Self {
            wurzel,
            faelle,
            audit_dir,
        }
    }

    #[must_use]
    pub fn audit_pfad(&self) -> PathBuf {
        self.audit_dir.join("audit.jsonl")
    }

    #[must_use]
    pub fn fehler_pfad(&self) -> PathBuf {
        self.audit_dir.join("fehler.log")
    }

    /// Die Oberfläche liegt in `rust/api/static` (E2 des Löschplans; Python hatte sie in
    /// `produkt/haut/static`, die Dateien sind byte-gleich, geprüft in `umzug_aus_produkt.rs`; Ausnahme `index.html` seit
    /// 2026-10-15: der Namen-Hinweis neben dem KI-Eingabefeld, S8).
    #[must_use]
    pub fn static_dir(&self) -> PathBuf {
        self.wurzel.join("rust").join("api").join("static")
    }

    /// Standard-Nutzerdatei, wenn `TAXGRAPH_USER_STORE` fehlt: `users.json` im Datenverzeichnis, neben
    /// `faelle/` (E3 des Löschplans; wie `scripts/starte-api.sh`). Python legt sie noch nach
    /// `produkt/auth/users.json` (`auth.py:18`).
    ///
    /// Gibt es die neue Datei nicht, aber die alte, gilt die alte: sonst verschwänden die Konten
    /// still, sobald dieser Pfad greift (Julius kopiert die Datei erst von Hand). [`Self::nutzerdatei_ist_veraltet`]
    /// und [`Self::melde_veraltete_nutzerdatei`] machen das sichtbar.
    // ponytail: der Rückgriff auf `produkt/auth/users.json` fällt mit L3 (dann gibt es die alte Datei nicht mehr).
    #[must_use]
    pub fn nutzerdatei(&self) -> PathBuf {
        self.nutzerdatei_wahl().0
    }

    /// `true`, wenn [`Self::nutzerdatei`] auf die alte Datei in `produkt/auth` zurückgreift.
    #[must_use]
    pub fn nutzerdatei_ist_veraltet(&self) -> bool {
        self.nutzerdatei_wahl().1
    }

    /// Schreibt die Warnung `server.nutzerdatei_veraltet` ins Fehlerlog (ohne Pfad), wenn die Standarddatei
    /// die alte ist und `TAXGRAPH_USER_STORE` nicht gesetzt ist (`user_store_gesetzt`: gesetzt und nicht leer).
    pub fn melde_veraltete_nutzerdatei(&self, user_store_gesetzt: bool) {
        if user_store_gesetzt || !self.nutzerdatei_ist_veraltet() {
            return;
        }
        // Das Protokoll darf den Start nie abbrechen.
        let _ = protokolliere(
            &self.fehler_pfad(),
            "server.nutzerdatei_veraltet",
            &(),
            Stufe::Warnung,
            None,
            Meta::default(),
        );
    }

    /// (Pfad, ist die alte Datei).
    fn nutzerdatei_wahl(&self) -> (PathBuf, bool) {
        let neu = self
            .faelle
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("users.json");
        let alt = self.wurzel.join("produkt").join("auth").join("users.json");
        if !neu.is_file() && alt.is_file() {
            (alt, true)
        } else {
            (neu, false)
        }
    }
}

/// `TAXGRAPH_NO_AUTH == "1"` (exakt, ohne Trim — `api.py:109`), bei jedem Aufruf gelesen.
#[must_use]
pub fn no_auth() -> bool {
    std::env::var("TAXGRAPH_NO_AUTH").is_ok_and(|v| v == "1")
}

/// `flow.an()` (`flow.py:46`): `TAXGRAPH_FLOW` oder `TAXGRAPH_KI_DEBUG` getrimmt gleich `"1"`.
#[must_use]
pub fn flow_an() -> bool {
    env_text("TAXGRAPH_FLOW") == "1" || env_text("TAXGRAPH_KI_DEBUG") == "1"
}

/// `_lade_env_dateien` (`server.py:331`): `.env.maps`, `.env.llm`, `.env` aus `wurzel`; nur
/// Schlüssel, die noch nicht gesetzt sind. Werte gehen nie in ein Protokoll.
///
/// Eine fehlende Datei (oder ein Verzeichnis dieses Namens) bleibt still, wie Pythons `os.path.isfile`.
/// Eine vorhandene Datei, die sich nicht lesen lässt (Rechte, kein UTF-8), kommt als Warnung
/// `server.env_datei_lesen` ins Fehlerlog, ohne Pfad und ohne Inhalt, und wird übersprungen. Das
/// Fehlerlog liegt dort, wo `Konfig::aus_env` es VOR dem ersten `set_var` dieser Funktion findet
/// (Python: `AUDIT_DIR` steht beim Import fest).
pub fn lade_env_dateien(wurzel: &Path) {
    let fehler_pfad = Konfig::aus_env().fehler_pfad();
    for name in [".env.maps", ".env.llm", ".env"] {
        let pfad = wurzel.join(name);
        if !pfad.is_file() {
            continue;
        }
        let text = match std::fs::read_to_string(&pfad) {
            Ok(text) => text,
            Err(e) => {
                // Das Protokoll darf den Start nie abbrechen: ein Fehler beim Schreiben ist hier
                // verschluckt. Das Verzeichnis legt `protokolliere` bei Bedarf selbst an.
                let _ = protokolliere(
                    &fehler_pfad,
                    "server.env_datei_lesen",
                    &e,
                    Stufe::Warnung,
                    None,
                    Meta::default(),
                );
                continue;
            }
        };
        for zeile in text.lines().map(str::trim) {
            if zeile.is_empty() || zeile.starts_with('#') {
                continue;
            }
            let Some((schluessel, wert)) = zeile.split_once('=') else {
                continue;
            };
            let (schluessel, wert) = (
                schluessel.trim(),
                wert.trim().trim_matches('"').trim_matches('\''),
            );
            if !schluessel.is_empty() && std::env::var_os(schluessel).is_none() {
                std::env::set_var(schluessel, wert);
            }
        }
    }
}
