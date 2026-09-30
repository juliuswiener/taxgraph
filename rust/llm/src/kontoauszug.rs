//! Kontoauszug-Klassifikator als LLM-Rueckfall (`kontoauszug_writer.py:450-475`,
//! `api_llm.py:1179-1194`). Das Modell sieht nur den maskierten Zweck und den Betrag.
use serde::{Deserialize, Serialize};

use crate::client::{Chat, Nachricht};
use crate::pii::Maskiert;
use crate::texte;

/// Die MVP-Kategorien mit Zielfeld (`KATEGORIE_FELD`-Schluessel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kategorie {
    Handwerker,
    Dienstleistung,
    Minijob,
    Spende,
    Vorsorge,
}

impl Kategorie {
    /// Aus dem Wire-Wort.
    ///
    /// ```
    /// assert_eq!(llm::Kategorie::aus_wort("spende"), Some(llm::Kategorie::Spende));
    /// assert_eq!(llm::Kategorie::aus_wort("miete"), None);
    /// ```
    #[must_use]
    pub fn aus_wort(w: &str) -> Option<Self> {
        Some(match w {
            "handwerker" => Self::Handwerker,
            "dienstleistung" => Self::Dienstleistung,
            "minijob" => Self::Minijob,
            "spende" => Self::Spende,
            "vorsorge" => Self::Vorsorge,
            _ => return None,
        })
    }

    /// Das Wire-Wort.
    ///
    /// ```
    /// assert_eq!(llm::Kategorie::Minijob.als_str(), "minijob");
    /// ```
    #[must_use]
    pub fn als_str(self) -> &'static str {
        match self {
            Self::Handwerker => "handwerker",
            Self::Dienstleistung => "dienstleistung",
            Self::Minijob => "minijob",
            Self::Spende => "spende",
            Self::Vorsorge => "vorsorge",
        }
    }
}

/// `_parse_llm_kategorie(text)`: erstes `{` bis letztes `}`, als JSON, `kategorie` aus der
/// MVP-Menge; sonst `None`. PARITAET-Abweichung: eine Liste/ein Objekt als `kategorie` wirft in
/// Python `TypeError` (unhashable) ungefangen; hier `None`.
///
/// ```
/// assert_eq!(llm::kontoauszug::parse_kategorie("Antwort: {\"kategorie\": \"spende\"} ok"), Some(llm::Kategorie::Spende));
/// assert_eq!(llm::kontoauszug::parse_kategorie("{\"kategorie\": null}"), None);
/// ```
#[must_use]
pub fn parse_kategorie(text: &str) -> Option<Kategorie> {
    let start = text.find('{')?;
    let ende = text.rfind('}').filter(|e| *e > start)?;
    let j: serde_json::Value = serde_json::from_str(text.get(start..=ende)?).ok()?;
    j.get("kategorie")?.as_str().and_then(Kategorie::aus_wort)
}

/// Eine Buchung klassifizieren. Jeder [`crate::LlmFehler`] wird zu `None` (unklassifiziert) —
/// wie `_kontoauszug_llm_klassifikator`, der nur `LlmNichtVerfuegbar` faengt.
///
/// ```
/// struct Fix;
/// impl llm::Chat for Fix {
///     fn complete(&self, m: &[llm::Nachricht], _: Option<&serde_json::Value>) -> Result<llm::Completion, llm::LlmFehler> {
///         assert!(m[1].inhalt().starts_with("Zweck: Maler DE89****\nBetrag: -480.00 EUR"));
///         Ok(llm::Completion { text: r#"{"kategorie": "handwerker"}"#.into(), ..Default::default() })
///     }
/// }
/// let z = llm::pii::maskiere("Maler DE89370400440532013000");
/// assert_eq!(llm::kontoauszug::klassifiziere(&Fix, &z, -48000), Some(llm::Kategorie::Handwerker));
/// ```
#[must_use]
pub fn klassifiziere(chat: &dyn Chat, zweck: &Maskiert, betrag_cent: i64) -> Option<Kategorie> {
    let nachrichten = [
        Nachricht::system(texte::KONTOAUSZUG_SYSTEM.to_owned()),
        Nachricht::buchung(zweck, betrag_cent),
    ];
    chat.complete(&nachrichten, None)
        .ok()
        .and_then(|c| parse_kategorie(&c.text))
}
