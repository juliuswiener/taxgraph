//! Nutzername und Passwort als validierte Typen (`auth.py:25-26`, `:133-136`).
//!
//! `_USER_RE = ^[a-zA-Z][a-zA-Z0-9_-]{2,31}$` und `_PW_RE = ^.{8,128}$`, beide per `fullmatch`.
//! `fullmatch` laesst das `$` NICHT vor einem abschliessenden `\n` enden (der Treffer muss die
//! ganze Zeichenkette decken), und Pythons `.` trifft jedes Zeichen ausser `\n`. Laenge zaehlt in
//! Codepunkten, nicht Bytes — daher `chars().count()`.
use std::fmt;

use crate::AuthFehler;

/// Ein gueltiger Nutzername: 3–32 ASCII-Zeichen, beginnt mit einem Buchstaben, danach nur
/// `A-Za-z0-9_-`. Nur ueber [`Username::neu`] konstruierbar.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Username(String);

impl Username {
    /// Prueft `roh` gegen `_USER_RE` (`auth.py:25`).
    ///
    /// ```
    /// use auth::Username;
    /// assert!(Username::neu("julius").is_ok());
    /// assert!(Username::neu("1abc").is_err());
    /// assert!(Username::neu("ab").is_err());
    /// ```
    ///
    /// # Errors
    /// [`AuthFehler::UsernameUngueltig`], wenn das Muster nicht passt.
    pub fn neu(roh: &str) -> Result<Self, AuthFehler> {
        if ist_gueltiger_username(roh) {
            Ok(Self(roh.to_owned()))
        } else {
            Err(AuthFehler::UsernameUngueltig)
        }
    }

    /// Der Name als `&str`.
    ///
    /// ```
    /// assert_eq!(auth::Username::neu("julius").unwrap().as_str(), "julius");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `_USER_RE.fullmatch(roh)` als Praedikat.
///
/// ```
/// assert!(auth::ist_gueltiger_username("a_b-c"));
/// assert!(!auth::ist_gueltiger_username("a_b-c\n"));
/// ```
#[must_use]
pub fn ist_gueltiger_username(roh: &str) -> bool {
    let mut zeichen = roh.chars();
    let Some(erstes) = zeichen.next() else { return false };
    let rest: Vec<char> = zeichen.collect();
    erstes.is_ascii_alphabetic()
        && (2..=31).contains(&rest.len())
        && rest.iter().all(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
}

/// `_PW_RE.fullmatch(roh)` als Praedikat: 8–128 Codepunkte, keiner davon `\n`.
///
/// ```
/// assert!(auth::ist_gueltiges_passwort("geheim123"));
/// assert!(!auth::ist_gueltiges_passwort("kurz"));
/// assert!(!auth::ist_gueltiges_passwort("zeile\neins"));
/// ```
#[must_use]
pub fn ist_gueltiges_passwort(roh: &str) -> bool {
    (8..=128).contains(&roh.chars().count()) && !roh.contains('\n')
}

/// `{username, password}` aus dem Request-Body (`auth.py:124-132`, `:150-158`). Pythons
/// Meldung fuer fehlende Felder listet sie sortiert als Python-Liste.
#[derive(Debug, Clone)]
pub struct Anmeldung {
    pub username: String,
    pub password: String,
}

impl Anmeldung {
    /// Liest die zwei Pflichtfelder. Fehlen sie, meldet der Fehler genau Pythons Text
    /// `Pflichtfelder fehlen: ['password', 'username']`.
    ///
    /// ```
    /// let body = serde_json::json!({"username": "julius"});
    /// let fehler = auth::Anmeldung::aus_body(&body).unwrap_err();
    /// assert_eq!(fehler.to_string(), "Pflichtfelder fehlen: ['password']");
    /// ```
    ///
    /// # Errors
    /// [`AuthFehler::PflichtfelderFehlen`] bzw. [`AuthFehler::FeldKeinText`] (PARITAET: Python
    /// wirft dort `TypeError`/`AttributeError`, der Server antwortet 500).
    pub fn aus_body(body: &serde_json::Value) -> Result<Self, AuthFehler> {
        let feld = |name: &str| body.get(name);
        let fehlend: Vec<&'static str> =
            ["password", "username"].into_iter().filter(|n| feld(n).is_none()).collect();
        if !fehlend.is_empty() {
            return Err(AuthFehler::PflichtfelderFehlen(fehlend));
        }
        let text = |name: &'static str| {
            feld(name).and_then(serde_json::Value::as_str).map(str::to_owned).ok_or(AuthFehler::FeldKeinText(name))
        };
        Ok(Self { username: text("username")?, password: text("password")? })
    }
}
