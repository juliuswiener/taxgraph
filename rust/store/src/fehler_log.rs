//! PII-sicheres Fehler-Protokoll (`produkt/store/fehler_log.py`): ein `except`-Block bleibt
//! rekonstruierbar, ohne dass die Nutzdaten (Betraege, IBAN, Steuer-ID), die ihn ausgeloest
//! haben, mit hineinrutschen.
//!
//! PARITAET — drei bewusste Abweichungen vom Original:
//! 1. Python liest `str(exc)`/`exc.args` NIRGENDS, nur `type(exc).__name__` + den innersten
//!    Traceback-Rahmen (`_ort_aus_traceback`). Rust hat keinen Traceback; die kostenlose
//!    stdlib-Entsprechung ist `#[track_caller]` + `std::panic::Location::caller()` — die
//!    Aufrufstelle von [`protokolliere`] selbst, nicht der tiefste Rahmen einer laufenden
//!    Fehlerkette. Der Typname kommt ueber `std::any::type_name::<E>()`: ein Bezeichner aus dem
//!    Quelltext, kein Nutzdatum — wie `type(exc).__name__`.
//! 2. [`Meta`] ist ein Struct mit GENAU den vier erlaubten Feldern
//!    (`_ERLAUBTE_META = {"anzahl","laenge","versuche","geglueckt"}`) statt einer Positivliste,
//!    die zur Laufzeit gegen freie `**kwargs` prueft — ein fuenftes Feld existiert im Typsystem
//!    schlicht nicht, dieselbe Bauart wie `Feldzustand` bei der Zwei-Signal-Regel.
//! 3. [`FallId::pruefe`] prueft die FORM (`_FALL_RE = ^[A-Za-z0-9_-]{1,64}$`) immer, direkt
//!    portiert. Die zusaetzliche PII-Kategorie-Pruefung (`pii_filter._KATEGORIEN`, verankertes
//!    `fullmatch`) lebt in `produkt/haut/pii_filter.py`, ausserhalb des fuer dieses Crate
//!    erlaubten Baums — sie kommt hier als PARAMETER (`pii_muster: &[Regex]`) statt einer
//!    zweiten, driftgefaehrdeten Kopie des Katalogs ("configuration via parameters, no
//!    import-time globals", Auftrag). Ohne uebergebene Muster gilt fail-closed dieselbe sichtbare
//!    Sperr-Markierung, die Python fuer "Store ohne Haut" zeigt (`_FALL_ID_KEIN_FILTER`).
use serde::{Deserialize, Serialize};

use crate::anhaenge_datei::haenge_zeile_an;
use crate::zeit::jetzt_iso;

/// Protokoll-Stufe, wie Pythons `logging.ERROR/WARNING/DEBUG` (`fehler_log.py:73`). Wire-Form
/// sind dieselben englischen Grossbuchstaben-Namen, die `logging.getLevelName` liefert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stufe {
    #[serde(rename = "ERROR")]
    Fehler,
    #[serde(rename = "WARNING")]
    Warnung,
    #[serde(rename = "DEBUG")]
    Debug,
}

#[cfg(test)]
mod stufe_tests {
    use super::Stufe;

    #[test]
    fn wire_form_ist_python_logging_getlevelname() {
        assert_eq!(serde_json::to_string(&Stufe::Fehler).unwrap(), "\"ERROR\"");
        assert_eq!(
            serde_json::to_string(&Stufe::Warnung).unwrap(),
            "\"WARNING\""
        );
        assert_eq!(serde_json::to_string(&Stufe::Debug).unwrap(), "\"DEBUG\"");
    }
}

/// Positivliste erlaubter Zusatzangaben (`_ERLAUBTE_META`, `fehler_log.py:174`): NUR Anzahlen und
/// Wahrheitswerte. "Text ist die Form, in der Nutzdaten reisen" (Moduldoku) — deshalb gibt es
/// hier gar kein String-Feld; ein fuenfter Name existiert im Typsystem nicht.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anzahl: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laenge: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub versuche: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geglueckt: Option<bool>,
}

/// Eine geprueft Fall-Kennung — oder eine SICHTBARE Sperr-Markierung statt des rohen Werts
/// (`_sicherer_fall_id`, `fehler_log.py:105-129`): ein leeres Feld ohne Hinweis saehe wie ein
/// anderer Fehler aus (kein `fall_id` uebergeben) statt wie das, was es ist (gesperrt).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FallId(String);

/// Sichtbarer Platzhalter, wenn keine PII-Muster uebergeben wurden (Pythons "Store ohne Haut",
/// `_FALL_ID_KEIN_FILTER`).
const GESPERRT_KEIN_FILTER: &str = "<gesperrt:pii_filter_fehlt>";

impl FallId {
    /// Prueft Form (`^[A-Za-z0-9_-]{1,64}$`) und, wenn `pii_muster` nicht leer ist, ob die
    /// Kennung ALS GANZES (nicht als Teilstring) eines der Muster erfuellt — Pythons
    /// `pattern.fullmatch`. `pii_muster` leer -> fail-closed dieselbe Sperre wie bei fehlendem
    /// `pii_filter` in Python.
    ///
    /// ```
    /// use store::fehler_log::FallId;
    /// assert_eq!(FallId::pruefe("fall-mit leerzeichen", &[]).als_str(), "<gesperrt:form>");
    /// assert_eq!(FallId::pruefe("demo-1234567890123", &[]).als_str(), "<gesperrt:pii_filter_fehlt>");
    /// ```
    #[must_use]
    pub fn pruefe(fall_id: &str, pii_muster: &[regex::Regex]) -> Self {
        let form_ok = !fall_id.is_empty()
            && fall_id.len() <= 64
            && fall_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
        if !form_ok {
            return Self("<gesperrt:form>".to_string());
        }
        if pii_muster.is_empty() {
            return Self(GESPERRT_KEIN_FILTER.to_string());
        }
        let ist_pii = pii_muster.iter().any(|re| {
            re.find(fall_id)
                .is_some_and(|m| m.start() == 0 && m.end() == fall_id.len())
        });
        if ist_pii {
            return Self("<gesperrt:pii>".to_string());
        }
        Self(fall_id.to_string())
    }

    /// ```
    /// use store::fehler_log::FallId;
    /// let steuer_id = regex::Regex::new(r"\d{11}").unwrap();
    /// assert_eq!(FallId::pruefe("demo-1", &[steuer_id.clone()]).als_str(), "demo-1");
    /// assert_eq!(FallId::pruefe("12345678901", &[steuer_id]).als_str(), "<gesperrt:pii>");
    /// ```
    #[must_use]
    pub fn als_str(&self) -> &str {
        &self.0
    }
}

/// Ein Fehler-Eintrag, wie im Log gespeichert (`fehler_log.py:204-213`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FehlerEintrag {
    pub ts: String,
    pub stufe: Stufe,
    pub ort: String,
    pub typ: String,
    pub quelle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fall_id: Option<FallId>,
    #[serde(flatten)]
    pub meta: Meta,
}

/// Fehler beim Anhaengen/Lesen des Fehler-Logs.
#[derive(Debug, thiserror::Error)]
pub enum FehlerLogFehler {
    #[error("fehler-log konnte nicht geschrieben/gelesen werden: {0}")]
    Ein(#[from] std::io::Error),
    #[error("fehler-log enthaelt eine nicht lesbare Zeile: {0}")]
    Format(#[from] serde_json::Error),
}

/// Schreibt EINEN Fehlereintrag als eine JSON-Zeile (`fehler_log.py:189-216`, `protokolliere`).
///
/// `ort`: fester Bezeichner der Fangstelle aus dem Quelltext (z.B. `"server.dispatch"`) — ein
/// Literal, kein zusammengesetzter Text, sonst reist Nutzereingabe mit. `exc` wird NUR ueber
/// seinen Typnamen gelesen, nie ueber seinen Inhalt (s. Moduldoku, Punkt 1).
///
/// ```
/// use store::fehler_log::{protokolliere, Meta, Stufe};
/// let dir = std::env::temp_dir().join(format!("taxgraph-doctest-fehlerlog-{}", std::process::id()));
/// let pfad = dir.join("fehler.jsonl");
/// let fehler = std::io::Error::other("IBAN DE89370400440532013000");
/// protokolliere(&pfad, "server.dispatch", &fehler, Stufe::Fehler, None, Meta::default()).unwrap();
/// let zeile = std::fs::read_to_string(&pfad).unwrap();
/// assert!(zeile.contains("server.dispatch"));
/// assert!(!zeile.contains("DE89")); // der Inhalt des Fehlers reist nie mit
/// std::fs::remove_dir_all(&dir).ok();
/// ```
///
/// # Errors
/// [`FehlerLogFehler::Ein`], wenn die Datei nicht angelegt/geschrieben werden kann.
#[track_caller]
pub fn protokolliere<E: ?Sized>(
    pfad: &std::path::Path,
    ort: &'static str,
    _exc: &E,
    stufe: Stufe,
    fall_id: Option<FallId>,
    meta: Meta,
) -> Result<(), FehlerLogFehler> {
    let aufrufstelle = std::panic::Location::caller();
    let eintrag = FehlerEintrag {
        ts: jetzt_iso(),
        stufe,
        ort: ort.to_string(),
        typ: std::any::type_name::<E>().to_string(),
        quelle: format!("{}:{}", aufrufstelle.file(), aufrufstelle.line()),
        fall_id,
        meta,
    };
    // Kann nur an einem nicht-endlichen Float scheitern -- `FehlerEintrag` traegt keinen; s.
    // `canonical.rs`-Moduldoku fuer denselben, unerreichbaren Zweig.
    let zeile = serde_json::to_string(&eintrag).unwrap_or_default();
    haenge_zeile_an(pfad, &zeile)?;
    Ok(())
}

/// Liest alle Eintraege (`fehler_log.py:219-225`, `lies`).
///
/// ```
/// use store::fehler_log::{lies, protokolliere, Meta, Stufe};
/// let dir = std::env::temp_dir().join(format!("taxgraph-doctest-fehlerlog-lies-{}", std::process::id()));
/// let pfad = dir.join("fehler.jsonl");
/// assert!(lies(&pfad).unwrap().is_empty()); // fehlende Datei: leeres Log
/// let meta = Meta { anzahl: Some(3), ..Meta::default() };
/// protokolliere(&pfad, "import.beleg", &std::fmt::Error, Stufe::Warnung, None, meta).unwrap();
/// let eintraege = lies(&pfad).unwrap();
/// assert_eq!(eintraege[0].stufe, Stufe::Warnung);
/// assert_eq!(eintraege[0].meta.anzahl, Some(3));
/// std::fs::remove_dir_all(&dir).ok();
/// ```
///
/// # Errors
/// [`FehlerLogFehler::Format`], wenn eine Zeile kein gueltiges JSON ist.
pub fn lies(pfad: &std::path::Path) -> Result<Vec<FehlerEintrag>, FehlerLogFehler> {
    if !pfad.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(pfad)?;
    text.lines()
        .filter(|z| !z.trim().is_empty())
        .map(|z| serde_json::from_str(z).map_err(FehlerLogFehler::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{protokolliere, FallId, Meta, Stufe};

    #[derive(Debug)]
    struct Testfehler;

    #[test]
    fn form_pruefung_sperrt_leerzeichen_und_ueberlaenge() {
        assert_eq!(FallId::pruefe("", &[]).als_str(), "<gesperrt:form>");
        assert_eq!(FallId::pruefe("a b", &[]).als_str(), "<gesperrt:form>");
        assert_eq!(
            FallId::pruefe(&"a".repeat(65), &[]).als_str(),
            "<gesperrt:form>"
        );
        assert_eq!(
            FallId::pruefe("demo-1758901234567", &[]).als_str(),
            "<gesperrt:pii_filter_fehlt>"
        );
    }

    #[test]
    fn pii_muster_sperrt_nur_bei_vollstaendigem_treffer() {
        let iban_artig = regex::Regex::new(r"^\d{16,22}$").unwrap();
        assert_eq!(
            FallId::pruefe("12345678901234567890", std::slice::from_ref(&iban_artig)).als_str(),
            "<gesperrt:pii>"
        );
        // Teiltreffer (Praefix+Ziffernfolge) ist WEDER Vollstaendig-Ziffer NOCH gesperrt --
        // dieselbe dokumentierte Restluecke wie in Python (Moduldoku Original: `kunde12345...`).
        assert_eq!(
            FallId::pruefe("demo-12345678901234567890", &[iban_artig]).als_str(),
            "demo-12345678901234567890"
        );
    }

    #[test]
    fn protokolliere_und_lies_roundtrip() {
        let dir =
            std::env::temp_dir().join(format!("taxgraph-store-test-fehler-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("fehler.log");
        let _ = std::fs::remove_file(&pfad);
        protokolliere(
            &pfad,
            "server.dispatch",
            &Testfehler,
            Stufe::Fehler,
            Some(FallId::pruefe("demo-1", &[])),
            Meta {
                anzahl: Some(3),
                ..Meta::default()
            },
        )
        .unwrap();
        let eintraege = super::lies(&pfad).unwrap();
        assert_eq!(eintraege.len(), 1);
        assert_eq!(eintraege[0].ort, "server.dispatch");
        assert!(eintraege[0].typ.contains("Testfehler"));
        assert_eq!(eintraege[0].meta.anzahl, Some(3));
        std::fs::remove_dir_all(&dir).ok();
    }
}
