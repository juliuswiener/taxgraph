//! PII-Filter vor jedem ausgehenden LLM-Aufruf (`produkt/haut/pii_filter.py`) und die
//! Kontoauszug-Maskierung (`produkt/eingang/kontoauszug_writer.py:139-187`).
//!
//! Zwei Ersetzungsregeln, bewusst NICHT zusammengelegt (wie in Python): der Chat-Pfad ersetzt
//! jede Kennung durch `[PII]`, der Kontoauszug-Pfad laesst ein Praefix stehen (`DE89****`),
//! damit die Klassifikation noch etwas greifen kann. Beide Ergebnisse sind eigene Typen
//! ([`Gefiltert`], [`Maskiert`]), und nur diese beiden Typen erreichen den LLM-Client
//! ([`crate::Nachricht`]).
//!
//! Reihenfolge ist tragend: IBAN vor Steuer-Id vor Kontonummer (sonst frisst die Ziffernregel
//! den IBAN-Rumpf bzw. meldet die unspezifische Kategorie).
use std::fmt;
use std::sync::LazyLock;

use crate::py::PyRegex;

const PLATZHALTER: &str = "[PII]";

struct Muster {
    kategorien: Vec<(&'static str, PyRegex)>,
    art9: PyRegex,
    konto_iban: PyRegex,
    konto_stnr: PyRegex,
    konto_kto: PyRegex,
}

const IBAN: &str = r"\b[A-Za-z]{2}\d{2}(?:[A-Za-z0-9]{10,30}|(?:[ -][A-Za-z0-9]{4}){2,7}(?:[ -][A-Za-z0-9]{1,4})?)\b";
const STEUER_ID: &str = r"\b\d(?:[ /]?\d){10,12}\b";

static MUSTER: LazyLock<Muster> = LazyLock::new(|| Muster {
    kategorien: vec![
        ("iban", PyRegex::neu(IBAN)),
        ("steuer_id", PyRegex::neu(STEUER_ID)),
        ("kontonummer", PyRegex::neu(r"\b\d{8,}\b")),
        (
            "datum",
            PyRegex::neu(r"\b(0[1-9]|[12]\d|3[01])\.(0[1-9]|1[0-2])\.\d{4}\b"),
        ),
        (
            "plz_ort",
            PyRegex::neu(
                r"\b\d{5}\s+(?!(?:Euro|EUR|€|Kilometer|km|Meter|m|Stück|Tonnen|kg|g|Liter|Jahre|Tage|Stunden|Mitglieder|Mitarbeiter|Einwohner|Jahr|Tag|Stunde)\b)[A-ZÄÖÜ][a-zäöüß]+\b",
            ),
        ),
        (
            "strasse",
            PyRegex::neu(
                r"\b[A-ZÄÖÜ][a-zäöüß]+(?:straße|strasse|str\.)(?:\s+\d{1,4}[a-z]?)?|\b[A-ZÄÖÜ][a-zäöüß]+(?:weg|allee|platz|gasse|damm|ring|chaussee)(?:\s+\d{1,4}[a-z]?)",
            ),
        ),
        (
            "anrede_name",
            PyRegex::neu(r"\b(?:Herr|Frau)\s+[A-ZÄÖÜ][a-zäöüß]+\b"),
        ),
    ],
    art9: PyRegex::neu(
        "(?i)konfession|kirche|grad_der_behinderung|schwerbehind|gehbehind|behinderungsbedingte\
         |behinderten_pb|behinderten.?pausch|pflegegrad|pflegebeduerftig|gepflegter\
         |pflegst|pflege_durch|pflegt die person|hilflos|blind|taubblind|merkzeichen\
         |berufsunfaehig|erwerbsunfaehig|krankheitskosten|heilbehandlung",
    ),
    konto_iban: PyRegex::neu(
        r"\b([A-Za-z]{2}\d{2})(?:[A-Za-z0-9]{10,30}|(?:[ -][A-Za-z0-9]{4}){2,7}(?:[ -][A-Za-z0-9]{1,4})?)\b",
    ),
    konto_stnr: PyRegex::neu(STEUER_ID),
    konto_kto: PyRegex::neu(r"\b(\d{8,})\b"),
});

/// Nutzertext nach `filtere()` — der einzige Weg, auf dem Nutzertext den LLM-Client erreicht.
/// Nur [`filtere`] konstruiert ihn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gefiltert(String);

impl Gefiltert {
    /// Der gefilterte Text.
    ///
    /// ```
    /// let (g, _) = llm::pii::filtere("IBAN DE89370400440532013000");
    /// assert_eq!(g.as_str(), "IBAN [PII]");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Gefiltert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Kontoauszug-Verwendungszweck nach der Kontoauszug-Maskierung. Nur [`maskiere`] konstruiert
/// ihn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Maskiert(String);

impl Maskiert {
    /// Der maskierte Text.
    ///
    /// ```
    /// assert_eq!(llm::pii::maskiere("Kto 12345678").as_str(), "Kto 12****");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Maskiert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `filtere(text)` (`pii_filter.py:160-178`): Kennungen durch `[PII]` ersetzen; Rueckgabe
/// `(gefiltert, sortierte Kategorien)`.
///
/// FAIL-CLOSED: bricht ein Muster zur Laufzeit ab (Backtracking-Grenze), wird der GANZE Text
/// ersetzt und die Kategorie `filter_abgebrochen` gemeldet — nie geht ungefilterter Text hinaus.
/// ponytail: Ganz-Ersetzung verliert den Satz; eine feinere Rueckfallebene erst, wenn der Fall
/// je gemessen auftritt.
///
/// ```
/// let (g, k) = llm::pii::filtere("Herr Maier, 12345 Berlin, am 01.02.2024");
/// assert_eq!(g.as_str(), "[PII], [PII], am [PII]");
/// assert_eq!(k, vec!["anrede_name", "datum", "plz_ort"]);
/// ```
#[must_use]
pub fn filtere(text: &str) -> (Gefiltert, Vec<&'static str>) {
    let mut text = text.to_owned();
    let mut getroffen = Vec::new();
    if text.is_empty() {
        return (Gefiltert(text), getroffen);
    }
    for (kategorie, muster) in &MUSTER.kategorien {
        match muster.ersetze(&text, |_| PLATZHALTER.to_owned()) {
            Some((neu, n)) => {
                if n > 0 {
                    getroffen.push(*kategorie);
                    text = neu;
                }
            }
            None => {
                return (
                    Gefiltert(PLATZHALTER.to_owned()),
                    vec!["filter_abgebrochen"],
                )
            }
        }
    }
    getroffen.sort_unstable();
    (Gefiltert(text), getroffen)
}

/// `ist_besondere_kategorie(feld_id, fragetext)` (`pii_filter.py:137-147`): offenbart dieses
/// FELD eine Angabe nach Art. 9 DSGVO? Laufzeitfehler zaehlt fail-closed als „ja".
///
/// ```
/// assert!(llm::pii::ist_besondere_kategorie("grad_der_behinderung", ""));
/// assert!(llm::pii::ist_besondere_kategorie("x", "Wen PFLEGST du?"));
/// assert!(!llm::pii::ist_besondere_kategorie("bruttoarbeitslohn", "Brutto?"));
/// ```
#[must_use]
pub fn ist_besondere_kategorie(feld_id: &str, fragetext: &str) -> bool {
    let a = &MUSTER.art9;
    a.sucht(feld_id).unwrap_or(true) || a.sucht(fragetext).unwrap_or(true)
}

/// `maskiere(text)` (`kontoauszug_writer.py:155-187`): IBAN auf Laenderkuerzel+Pruefziffer
/// kuerzen, Steuernummer/IdNr voll verdecken, lange Ziffernlaeufe auf zwei Ziffern kuerzen.
/// Namen werden bewusst NICHT maskiert (s. Python-Doku).
///
/// FAIL-CLOSED wie [`filtere`]: Laufzeitfehler maskiert den ganzen Zweck.
///
/// ```
/// assert_eq!(llm::pii::maskiere("de89 3704 0044 0532 0130 00 Miete").as_str(), "de89**** Miete");
/// assert_eq!(llm::pii::maskiere("StNr 181/815/08155").as_str(), "StNr ****");
/// ```
#[must_use]
pub fn maskiere(text: &str) -> Maskiert {
    if text.is_empty() {
        return Maskiert(String::new());
    }
    let m = &*MUSTER;
    let erste = |g: &[Option<String>]| g.first().cloned().flatten().unwrap_or_default();
    let schritt = m
        .konto_iban
        .ersetze(text, |g| format!("{}****", erste(g)))
        .and_then(|(t, _)| m.konto_stnr.ersetze(&t, |_| "****".to_owned()))
        .and_then(|(t, _)| {
            m.konto_kto
                .ersetze(&t, |g| format!("{}****", crate::py::vorne(&erste(g), 2)))
        });
    Maskiert(schritt.map_or_else(|| "****".to_owned(), |(t, _)| t))
}

#[cfg(test)]
mod tests {
    use super::{filtere, maskiere};

    #[test]
    fn plz_vor_euro_bleibt_stehen() {
        assert_eq!(filtere("12345 Euro Gehalt").0.as_str(), "12345 Euro Gehalt");
        assert_eq!(filtere("12345 Europa").0.as_str(), "[PII]");
    }

    #[test]
    fn iban_vor_steuer_id() {
        let (g, k) = filtere("DE89 3704 0044 0532 0130 00");
        assert_eq!(g.as_str(), "[PII]");
        assert_eq!(k, vec!["iban"]);
    }

    #[test]
    fn leerer_text_bleibt_leer() {
        assert_eq!(filtere("").0.as_str(), "");
        assert_eq!(maskiere("").as_str(), "");
    }
}
