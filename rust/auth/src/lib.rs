//! `auth` — Registrierung, Login, Logout (`produkt/auth/auth.py`): bcrypt, JWT HS256 24 h mit
//! `jti`, Logout-Sperrliste im Speicher, Nutzerdatei 0600 atomar.
//!
//! Kein Modul-Global: Geheimnis, Nutzerdatei und Sperrliste gehoeren zu einem [`Auth`]-Wert,
//! den der Server einmal je Prozess baut. Die Sperrliste geht mit dem Prozess verloren — wie in
//! Python (`auth.py:110`).
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

mod datei;
mod eingabe;
mod token;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;
use store::audit::AuditAktion;

pub use datei::py_json;
pub use eingabe::{ist_gueltiger_username, ist_gueltiges_passwort, Anmeldung, Username};
pub use token::{Claims, JWT_TTL_S};

/// bcrypt-Kostenfaktor; `bcrypt.gensalt()` nutzt 12.
const BCRYPT_KOSTEN: u32 = 12;

/// Ein Auth-Fehler mit HTTP-Status (`AuthError(status, msg)`, `auth.py:29-32`). `Display` ist
/// Pythons Meldung wortgleich; die 500er-Varianten stehen fuer Pythons nicht abgefangene
/// Ausnahmen.
#[derive(Debug, thiserror::Error)]
pub enum AuthFehler {
    #[error("Pflichtfelder fehlen: [{}]", .0.iter().map(|f| format!("'{f}'")).collect::<Vec<_>>().join(", "))]
    PflichtfelderFehlen(Vec<&'static str>),
    #[error("username: 3-32 Zeichen, beginnt mit Buchstabe, nur A-Za-z0-9_-")]
    UsernameUngueltig,
    #[error("password: 8-128 Zeichen")]
    PasswortUngueltig,
    #[error("User '{0}' existiert bereits")]
    Existiert(Username),
    #[error("username oder password falsch")]
    Falsch,
    /// bcrypt 5 (Python) lehnt Passwoerter ueber 72 Byte mit `ValueError` ab — 500.
    #[error("password cannot be longer than 72 bytes, truncate manually if necessary (e.g. my_password[:72])")]
    PasswortUeber72Bytes,
    #[error("Feld `{0}` ist kein Text")]
    FeldKeinText(&'static str),
    #[error("Nutzerdatei: {0}")]
    Speicher(#[from] std::io::Error),
    #[error("Nutzerdatei ist kein JSON: {0}")]
    NutzerdateiKaputt(serde_json::Error),
    #[error("Nutzerdatei ohne `users`-Objekt")]
    NutzerdateiOhneUsers,
    #[error("bcrypt: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error("JWT: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("Zufallsquelle: {0}")]
    Zufall(String),
}

impl AuthFehler {
    /// Der HTTP-Status, den `server.py` fuer diesen Fehler sendet.
    ///
    /// ```
    /// assert_eq!(auth::AuthFehler::Falsch.status(), 401);
    /// assert_eq!(auth::AuthFehler::PasswortUeber72Bytes.status(), 500);
    /// ```
    #[must_use]
    pub fn status(&self) -> u16 {
        match self {
            Self::PflichtfelderFehlen(_) | Self::UsernameUngueltig | Self::PasswortUngueltig => 400,
            Self::Existiert(_) => 409,
            Self::Falsch => 401,
            _ => 500,
        }
    }
}

/// Registrierung/Login/Logout/Token-Pruefung ueber einer Nutzerdatei und einem Geheimnis.
pub struct Auth {
    geheimnis: String,
    nutzerdatei: PathBuf,
    audit: Option<PathBuf>,
    gesperrt: Mutex<HashSet<String>>,
}

impl Auth {
    /// Ein Auth mit festem Geheimnis (Tests, Paritaet).
    ///
    /// ```
    /// let a = auth::Auth::neu("geheim".into(), "/nie/geschrieben/users.json".into(), None);
    /// assert!(a.pruefe_token("kein.token.da").is_none());
    /// ```
    #[must_use]
    pub fn neu(geheimnis: String, nutzerdatei: PathBuf, audit: Option<PathBuf>) -> Self {
        Self {
            geheimnis,
            nutzerdatei,
            audit,
            gesperrt: Mutex::new(HashSet::new()),
        }
    }

    /// Wie `auth.py:18-23`: Geheimnis aus `TAXGRAPH_JWT_SECRET`, sonst zufaellig je Start (64
    /// Hex-Zeichen wie `secrets.token_hex(32)` — jeder Neustart entwertet alle Tokens);
    /// Nutzerdatei aus `TAXGRAPH_USER_STORE`, sonst `standard_datei`.
    ///
    /// ```
    /// let a = auth::Auth::aus_env("/tmp/nie-geschrieben.json".as_ref(), None).unwrap();
    /// assert!(a.pruefe_token("x").is_none());
    /// ```
    ///
    /// # Errors
    /// [`AuthFehler::Zufall`], wenn die Zufallsquelle versagt.
    pub fn aus_env(standard_datei: &Path, audit: Option<PathBuf>) -> Result<Self, AuthFehler> {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let geheimnis = match env("TAXGRAPH_JWT_SECRET") {
            Some(g) => g,
            None => token_hex(32)?,
        };
        let nutzerdatei =
            env("TAXGRAPH_USER_STORE").map_or_else(|| standard_datei.to_path_buf(), PathBuf::from);
        Ok(Self::neu(geheimnis, nutzerdatei, audit))
    }

    fn protokolliere(&self, nutzer: &str, aktion: AuditAktion) {
        if let Some(pfad) = &self.audit {
            // Audit ist Nebenkanal: ein Schreibfehler darf die Anmeldung nicht kippen. Python
            // hat hier keinen `try` — dort bricht er sie ab (PARITAET-Abweichung, Bericht).
            let _ = store::audit::anhaengen(pfad, Some(nutzer), aktion, None, None);
        }
    }

    /// `register` (`auth.py:127-147`): pruefen, hashen, speichern. Rueckgabe = der Name.
    ///
    /// # Errors
    /// 400 bei ungueltigem Namen/Passwort, 409 bei vorhandenem Namen, 500 bei I/O oder
    /// Passwoertern ueber 72 Byte (Python-bcrypt 5 wirft dort `ValueError`).
    pub fn registriere(&self, a: &Anmeldung) -> Result<Username, AuthFehler> {
        let name = Username::neu(&a.username)?;
        if !ist_gueltiges_passwort(&a.password) {
            return Err(AuthFehler::PasswortUngueltig);
        }
        let mut bestand = datei::lade(&self.nutzerdatei)?;
        let nutzer = bestand
            .get_mut("users")
            .and_then(Value::as_object_mut)
            .ok_or(AuthFehler::NutzerdateiOhneUsers)?;
        if nutzer.contains_key(name.as_str()) {
            return Err(AuthFehler::Existiert(name));
        }
        let hash =
            bcrypt::non_truncating_hash(&a.password, BCRYPT_KOSTEN).map_err(|e| match e {
                bcrypt::BcryptError::Truncation(_) => AuthFehler::PasswortUeber72Bytes,
                andere => AuthFehler::Bcrypt(andere),
            })?;
        nutzer.insert(
            name.as_str().to_owned(),
            serde_json::json!({"password_hash": hash, "created_at": iso_jetzt()}),
        );
        datei::speichere(&self.nutzerdatei, &bestand)?;
        self.protokolliere(name.as_str(), AuditAktion::Register);
        Ok(name)
    }

    /// `login` (`auth.py:153-168`): Passwort gegen den gespeicherten Hash, dann ein Token.
    /// Kein Namens-Muster — Python prueft es beim Login nicht.
    ///
    /// # Errors
    /// 401 bei unbekanntem Namen oder falschem Passwort; 500 bei I/O, kaputtem Hash oder
    /// Passwoertern ueber 72 Byte fuer einen EXISTIERENDEN Nutzer (Python prueft erst den Namen).
    pub fn login(&self, a: &Anmeldung) -> Result<String, AuthFehler> {
        let bestand = datei::lade(&self.nutzerdatei)?;
        let hash = bestand
            .get("users")
            .and_then(|u| u.get(&a.username))
            .and_then(|u| u.get("password_hash"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let Some(hash) = hash else {
            self.protokolliere(&a.username, AuditAktion::LoginFehlgeschlagen);
            return Err(AuthFehler::Falsch);
        };
        if a.password.len() > 72 {
            return Err(AuthFehler::PasswortUeber72Bytes);
        }
        if !bcrypt::verify(&a.password, &hash)? {
            self.protokolliere(&a.username, AuditAktion::LoginFehlgeschlagen);
            return Err(AuthFehler::Falsch);
        }
        let token = self.stelle_aus(&a.username)?;
        self.protokolliere(&a.username, AuditAktion::Login);
        Ok(token)
    }

    /// `_create_token` (`auth.py:81-89`): `{sub, iat, exp = iat + 24 h, jti}`.
    ///
    /// ```
    /// let a = auth::Auth::neu("g".into(), "/x".into(), None);
    /// let t = a.stelle_aus("julius").unwrap();
    /// assert_eq!(a.pruefe_token(&t).as_deref(), Some("julius"));
    /// ```
    ///
    /// # Errors
    /// [`AuthFehler::Zufall`] oder [`AuthFehler::Jwt`].
    pub fn stelle_aus(&self, sub: &str) -> Result<String, AuthFehler> {
        let iat = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: sub.to_owned(),
            iat,
            exp: iat + JWT_TTL_S,
            jti: token_hex(16)?,
        };
        Ok(token::signiere(&claims, &self.geheimnis)?)
    }

    /// `verify_token` (`auth.py:92-100`): `sub` eines gueltigen, nicht abgemeldeten Tokens.
    ///
    /// ```
    /// let a = auth::Auth::neu("g".into(), "/x".into(), None);
    /// let t = a.stelle_aus("julius").unwrap();
    /// a.logout(&format!("Bearer {t}"));
    /// assert!(a.pruefe_token(&t).is_none());
    /// ```
    #[must_use]
    pub fn pruefe_token(&self, token: &str) -> Option<String> {
        let payload = token::pruefe(token, &self.geheimnis, jetzt())?;
        let jti = payload.get("jti").and_then(Value::as_str).unwrap_or("");
        if self.gesperrt.lock().is_ok_and(|g| g.contains(jti)) {
            return None;
        }
        payload
            .get("sub")
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    /// `logout` (`auth.py:171-182`): ein gueltiges Token (optional mit `Bearer `-Praefix) auf
    /// die Sperrliste setzen. Antwortet in Python immer 200; hier ist die Rueckgabe das `sub`
    /// des abgemeldeten Tokens, falls es eines gab.
    pub fn logout(&self, roh: &str) -> Option<String> {
        let token = roh.strip_prefix("Bearer ").unwrap_or(roh);
        if token.is_empty() {
            return None;
        }
        let payload = token::pruefe(token, &self.geheimnis, jetzt())?;
        let jti = payload
            .get("jti")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if let Ok(mut g) = self.gesperrt.lock() {
            g.insert(jti);
        }
        let sub = payload
            .get("sub")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        if let Some(s) = &sub {
            self.protokolliere(s, AuditAktion::Logout);
        }
        sub
    }
}

fn jetzt() -> i64 {
    chrono::Utc::now().timestamp()
}

/// `datetime.now(timezone.utc).isoformat()`: Mikrosekunden nur, wenn nicht 0.
fn iso_jetzt() -> String {
    let t = chrono::Utc::now();
    if t.timestamp_subsec_micros() == 0 {
        t.format("%Y-%m-%dT%H:%M:%S+00:00").to_string()
    } else {
        t.format("%Y-%m-%dT%H:%M:%S%.6f+00:00").to_string()
    }
}

/// `secrets.token_hex(n)`: `n` Zufallsbytes als Kleinbuchstaben-Hex.
fn token_hex(n: usize) -> Result<String, AuthFehler> {
    let mut puffer = vec![0u8; n];
    getrandom::fill(&mut puffer).map_err(|e| AuthFehler::Zufall(e.to_string()))?;
    Ok(puffer
        .iter()
        .fold(String::with_capacity(2 * n), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        }))
}

#[cfg(test)]
mod tests {
    use super::{Anmeldung, Auth, AuthFehler};

    fn anmeldung(u: &str, p: &str) -> Anmeldung {
        Anmeldung {
            username: u.into(),
            password: p.into(),
        }
    }

    #[test]
    fn register_login_logout_und_datei_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let pfad = dir.path().join("users.json");
        let a = Auth::neu("s".into(), pfad.clone(), None);
        a.registriere(&anmeldung("julius", "geheim123")).unwrap();
        assert_eq!(
            std::fs::metadata(&pfad).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(matches!(
            a.registriere(&anmeldung("julius", "geheim123")),
            Err(AuthFehler::Existiert(_))
        ));
        assert!(matches!(
            a.login(&anmeldung("julius", "falsch123")),
            Err(AuthFehler::Falsch)
        ));
        let t = a.login(&anmeldung("julius", "geheim123")).unwrap();
        assert_eq!(a.pruefe_token(&t).as_deref(), Some("julius"));
        assert_eq!(a.logout(&t).as_deref(), Some("julius"));
        assert!(a.pruefe_token(&t).is_none());
    }

    #[test]
    fn passwort_ueber_72_byte_wie_bcrypt5() {
        let dir = tempfile::tempdir().unwrap();
        let a = Auth::neu("s".into(), dir.path().join("u.json"), None);
        let lang = "x".repeat(73);
        assert!(matches!(
            a.registriere(&anmeldung("julius", &lang)),
            Err(AuthFehler::PasswortUeber72Bytes)
        ));
    }

    #[test]
    fn fremdes_geheimnis_wird_abgelehnt() {
        let a = Auth::neu("eins".into(), "/x".into(), None);
        let b = Auth::neu("zwei".into(), "/x".into(), None);
        let t = a.stelle_aus("julius").unwrap();
        assert!(b.pruefe_token(&t).is_none());
    }
}
