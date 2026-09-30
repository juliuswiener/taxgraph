//! Audit-Log: append-only JSON-Lines, niemals delete/update (`produkt/store/audit.py`).
//! `user_id` bleibt system-interner Username, keine PII in Detail-Feldern.
//!
//! PARITAET (Ablageort): Python bestimmt ihn selbst (`_standard_dir`: verzoegerter Import von
//! `api_constants.FAELLE`, sonst `$XDG_DATA_HOME/taxgraph/faelle`) und liest `$TAXGRAPH_AUDIT_DIR`
//! als Override — ein Import-Zeit-Global (`AUDIT_DIR`). Dieses Crate darf `produkt/` weder lesen
//! noch spiegeln; der Pfad kommt hier deshalb als PFLICHT-PARAMETER (`pfad: &Path`) — der
//! Aufrufer (die kuenftige `api`-Schicht) uebergibt denselben Ort, den Python ueber
//! `AUDIT_DIR`/`$TAXGRAPH_AUDIT_DIR` waehlt ("configuration via parameters, no import-time
//! globals", Auftrag).
//!
//! PARITAET (Aktionsmenge): `audit.py`s Docstring nennt sieben Aktionen als geschlossene Liste,
//! aber der tatsaechliche Aufrufbestand (`grep audit.append( produkt/`, Stand 2026-09-29) ist
//! GROESSER und teils DYNAMISCH: `server.py:262` baut `f"fall_{act}"` aus einem URL-Pfadsegment
//! (`act = pfad.split("/")[-1]`), also keine Literal-Menge im Aufruf selbst. [`AuditAktion`]
//! deckt alle gefundenen Literale (`login`/`logout`/`login_fehlgeschlagen`/`register`/
//! `fall_angelegt`/`zugriff_verweigert`/`llm_call`/`fall_create`/`fall_geloescht`/
//! `fall_validiert`) als eigene Varianten ab und traegt zusaetzlich `Andere(String)` fuer den
//! dynamischen Rest — strenger als Python nirgends, aber ohne eine Vollstaendigkeit zu
//! behaupten, die das Original nicht hat.
use serde::{Deserialize, Serialize};

use crate::anhaenge_datei::haenge_zeile_an;
use crate::zeit::jetzt_iso;

/// Eine Audit-Aktion. `Andere` haelt jeden Wert, der keiner der bekannten Varianten entspricht
/// (s. Moduldoku, "PARITAET (Aktionsmenge)"). Wire-Form ist ein blanker String (wie Pythons
/// `"action": action`) — dafuer eine manuelle `Serialize`/`Deserialize`-Implementierung statt
/// eines Derives, das `Andere` als `{"Andere":"..."}` schriebe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditAktion {
    Login,
    Logout,
    LoginFehlgeschlagen,
    Register,
    FallAngelegt,
    FallCreate,
    FallGeloescht,
    FallValidiert,
    ZugriffVerweigert,
    LlmCall,
    Andere(String),
}

impl AuditAktion {
    #[must_use]
    pub fn als_str(&self) -> &str {
        match self {
            Self::Login => "login",
            Self::Logout => "logout",
            Self::LoginFehlgeschlagen => "login_fehlgeschlagen",
            Self::Register => "register",
            Self::FallAngelegt => "fall_angelegt",
            Self::FallCreate => "fall_create",
            Self::FallGeloescht => "fall_geloescht",
            Self::FallValidiert => "fall_validiert",
            Self::ZugriffVerweigert => "zugriff_verweigert",
            Self::LlmCall => "llm_call",
            Self::Andere(s) => s,
        }
    }
}

impl From<&str> for AuditAktion {
    fn from(s: &str) -> Self {
        match s {
            "login" => Self::Login,
            "logout" => Self::Logout,
            "login_fehlgeschlagen" => Self::LoginFehlgeschlagen,
            "register" => Self::Register,
            "fall_angelegt" => Self::FallAngelegt,
            "fall_create" => Self::FallCreate,
            "fall_geloescht" => Self::FallGeloescht,
            "fall_validiert" => Self::FallValidiert,
            "zugriff_verweigert" => Self::ZugriffVerweigert,
            "llm_call" => Self::LlmCall,
            andere => Self::Andere(andere.to_string()),
        }
    }
}

impl Serialize for AuditAktion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.als_str())
    }
}

impl<'de> Deserialize<'de> for AuditAktion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Ein Audit-Eintrag, wie im Log gespeichert (`audit.py:53-59`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEintrag {
    pub ts: String,
    pub user_id: String,
    pub action: AuditAktion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fall_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Fehler beim Anhaengen/Lesen des Audit-Logs.
#[derive(Debug, thiserror::Error)]
pub enum AuditFehler {
    #[error("audit-log konnte nicht geschrieben/gelesen werden: {0}")]
    Ein(#[from] std::io::Error),
    #[error("audit-log enthaelt eine nicht lesbare Zeile: {0}")]
    Format(#[from] serde_json::Error),
}

/// Haengt EINEN Audit-Eintrag an (`audit.py:44-74`, `append`). `user_id: None` -> `"unbekannt"`
/// (`audit.py:55`).
///
/// ```
/// use store::audit::{anhaengen, lies, AuditAktion};
/// let dir = std::env::temp_dir().join(format!("taxgraph-doctest-audit-{}", std::process::id()));
/// let pfad = dir.join("audit.jsonl");
/// anhaengen(&pfad, Some("julius"), AuditAktion::Login, None, None).unwrap();
/// assert_eq!(lies(&pfad).unwrap().len(), 1);
/// std::fs::remove_dir_all(&dir).ok();
/// ```
///
/// # Errors
/// [`AuditFehler::Ein`], wenn die Datei nicht angelegt/geschrieben werden kann.
pub fn anhaengen(
    pfad: &std::path::Path,
    user_id: Option<&str>,
    action: AuditAktion,
    fall_id: Option<&str>,
    detail: Option<&str>,
) -> Result<(), AuditFehler> {
    let eintrag = AuditEintrag {
        ts: jetzt_iso(),
        // Python: `user_id or "unbekannt"` — auch ein leerer Name wird `unbekannt`.
        user_id: user_id.filter(|u| !u.is_empty()).unwrap_or("unbekannt").to_string(),
        action,
        fall_id: fall_id.map(str::to_string),
        detail: detail.map(str::to_string),
    };
    // Kann nur an einem nicht-endlichen Float scheitern -- `AuditEintrag` traegt keinen; s.
    // `canonical.rs`-Moduldoku fuer denselben, unerreichbaren Zweig.
    let zeile = serde_json::to_string(&eintrag).unwrap_or_default();
    haenge_zeile_an(pfad, &zeile)?;
    Ok(())
}

/// Liest alle Eintraege (`audit.py:77-83`, `lies`).
///
/// # Errors
/// [`AuditFehler::Format`], wenn eine Zeile kein gueltiges JSON ist.
pub fn lies(pfad: &std::path::Path) -> Result<Vec<AuditEintrag>, AuditFehler> {
    if !pfad.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(pfad)?;
    text.lines()
        .filter(|z| !z.trim().is_empty())
        .map(|z| serde_json::from_str(z).map_err(AuditFehler::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{anhaengen, lies, AuditAktion};

    #[test]
    fn anhaengen_und_lies_roundtrip() {
        let dir = std::env::temp_dir().join(format!("taxgraph-store-test-audit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("audit.jsonl");
        let _ = std::fs::remove_file(&pfad);
        anhaengen(&pfad, Some("julius"), AuditAktion::Login, None, None).unwrap();
        anhaengen(&pfad, None, AuditAktion::Andere("fall_einreichen".to_string()), Some("f1"), Some("status=200"))
            .unwrap();
        let eintraege = lies(&pfad).unwrap();
        assert_eq!(eintraege.len(), 2);
        assert_eq!(eintraege[0].user_id, "julius");
        assert_eq!(eintraege[0].action, AuditAktion::Login);
        assert_eq!(eintraege[1].user_id, "unbekannt");
        assert_eq!(eintraege[1].action, AuditAktion::Andere("fall_einreichen".to_string()));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fehlende_datei_liest_als_leer() {
        let pfad = std::env::temp_dir().join("taxgraph-store-test-audit-nie-angelegt.jsonl");
        let _ = std::fs::remove_file(&pfad);
        assert!(lies(&pfad).unwrap().is_empty());
    }
}
