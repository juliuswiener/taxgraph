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
//!
//! **Grenze (Julius 2026-10-15, Option a): Namen ohne Anrede gehen durch.** Der Filter erkennt einen Namen nur nach
//! "Herr" oder "Frau" (`anrede_name`). "Max Mustermann", "Mustermann, Max" und "Dr. Meier" bleiben unveraendert im Text,
//! der an den Anbieter geht; bei "Herr Dr. Schmidt" faellt nur "Herr Dr" weg, der Nachname bleibt. Der Befund zaehlt, sobald
//! `LLM_API_KEY` gesetzt ist (Ticket-Befund (g), im Code gelesen). Es gibt kein Namenserkennungs-Modell (ein
//! neues Crate braucht Julius' Wort) und keine Heuristik (zwei grosse Woerter hintereinander trafen auch "Einkommensteuer
//! Berlin"). Die Oberflaeche warnt darum neben dem KI-Eingabefeld vor Namen. Festgehalten ist die Grenze im Test
//! `namen_ohne_anrede_gehen_durch_bekannte_grenze`; wer den Filter verbessert, aendert Test, diese Zeilen und die Warnung zusammen.
use std::fmt;
use std::sync::LazyLock;

use crate::py::PyRegex;

const PLATZHALTER: &str = "[PII]";

struct Muster {
    kategorien: Vec<(&'static str, PyRegex)>,
    art9: PyRegex,
    art9_buchungstext: PyRegex,
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
         |berufsunfaehig|erwerbsunfaehig|krankheitskosten|heilbehandlung\
         |gewerkschaft|berufsverband",
    ),
    // Wortschatz fuer den VERWENDUNGSZWECK einer Buchung (Bankimport S2, d-60): Gesundheit, Gewerkschaft, Kirche (Art. 9 Abs. 1
    // DSGVO). Er ist grob mit Absicht: ein Treffer zu viel haelt eine Buchung von der KI fern, ein Treffer zu wenig gibt einen Text
    // aus der besonderen Kategorie weiter. Die Gesundheits- und Kirchen-Woerter sind Wortteile ("arzt" in "Zahnarzt", "kirch" in
    // "Kirchensteuer"); "ver.di" und "verdi" stehen mit Wortgrenze, sonst traefe "Verdienst".
    // ponytail: Wortliste von Hand, keine Quelle im Repo; sie kennt keine Namen von Kassen, Praxen oder Gewerkschaften ohne eines
    // dieser Woerter ("AOK", "Barmer", "GEW"). Ausbau: ein Verzeichnis dieser Namen, sobald eines vorliegt.
    art9_buchungstext: PyRegex::neu(
        "(?i)arzt|ärzt|aerzt|praxis|klinik|krankenhaus|apotheke|physiotherap|psychotherap|heilpraktik|krankenkasse\
         |krankenversicherung|pflegeversicherung|pflegekasse|medizin|arznei\
         |gewerkschaft|\\bver\\.?di\\b|\\big\\s*metall\\b|\\big\\s*bau\\b|\\big\\s*bce\\b|\\bdgb\\b|berufsverband\
         |kirch|bistum|diözese|dioezese|pfarr|evangelisch|katholisch|moschee|synagoge",
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
    // Tragend, s. Moduldoku: IBAN vor Steuer-Id vor Kontonummer.
    debug_assert!(["iban", "steuer_id", "kontonummer"]
        .map(|k| MUSTER.kategorien.iter().position(|(n, _)| *n == k))
        .is_sorted());
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
/// assert!(llm::pii::ist_besondere_kategorie("berufsverband_beitraege", ""));
/// assert!(llm::pii::ist_besondere_kategorie("x", "Beitrag an die Gewerkschaft?"));
/// assert!(!llm::pii::ist_besondere_kategorie("bruttoarbeitslohn", "Brutto?"));
/// ```
#[must_use]
pub fn ist_besondere_kategorie(feld_id: &str, fragetext: &str) -> bool {
    let a = &MUSTER.art9;
    a.sucht(feld_id).unwrap_or(true) || a.sucht(fragetext).unwrap_or(true)
}

/// Nennt der Verwendungszweck einer Buchung etwas aus einer besonderen Kategorie (Gesundheit, Gewerkschaft, Kirche; Art. 9
/// DSGVO)? Ein solcher Text geht nie an das Sprachmodell, in keinem Haken-Zustand (Bankimport S2, d-60). Ein Laufzeitfehler des
/// Musters zaehlt fail-closed als "ja".
///
/// ```
/// assert!(llm::pii::ist_art9_buchungstext("Zahnarzt Dr. Beispiel"));
/// assert!(llm::pii::ist_art9_buchungstext("ver.di Mitgliedsbeitrag"));
/// assert!(llm::pii::ist_art9_buchungstext("Kirchensteuer Erzbistum"));
/// assert!(!llm::pii::ist_art9_buchungstext("Verdienstausfall Muster Einkauf"));
/// ```
#[must_use]
pub fn ist_art9_buchungstext(zweck: &str) -> bool {
    MUSTER.art9_buchungstext.sucht(zweck).unwrap_or(true)
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

    /// Der Wortschatz fuer den Buchungstext (Bankimport S2): Gesundheit, Gewerkschaft und Kirche treffen, Alltagstexte nicht.
    /// Die Grenze steht fest: ein Name ohne eines dieser Woerter ("AOK", "Barmer") trifft nicht (`ponytail` am Muster).
    #[test]
    fn art9_buchungstext_trifft_die_drei_kategorien_und_keinen_alltag() {
        use super::ist_art9_buchungstext;
        for ja in [
            "Zahnarzt Dr. Beispiel",
            "Arztpraxis Muster",
            "Apotheke am Markt",
            "Klinikum Musterstadt",
            "Techniker Krankenkasse Beitrag",
            "Pflegeversicherung Beitrag",
            "ver.di Mitgliedsbeitrag",
            "VERDI Beitrag",
            "IG Metall Beitrag",
            "Gewerkschaft Muster",
            "Kirchensteuer Erzbistum",
            "Kirchgeld 2025",
            "Evangelische Gemeinde Muster",
        ] {
            assert!(ist_art9_buchungstext(ja), "{ja:?} muss treffen");
        }
        for nein in ["Muster Einkauf 4711", "Verdienstausfall", "Miete Januar", "Spende Tierheim", "Maler Huber", "Barmer Beitrag"] {
            assert!(!ist_art9_buchungstext(nein), "{nein:?} darf nicht treffen");
        }
    }

    /// Jede Alternative des Musters hat ein eigenes Beispiel, das keine andere Alternative trifft: faellt eine Alternative aus dem
    /// Muster, wird genau ihr Beispiel rot. Die Beispiele stehen in der Reihenfolge des Musters.
    #[test]
    fn art9_buchungstext_jede_alternative_des_musters_hat_ein_eigenes_beispiel() {
        use super::ist_art9_buchungstext;
        for (alternative, beispiel) in [
            ("arzt", "Hausarzt Muster"),
            ("ärzt", "Ärztehaus Muster"),
            ("aerzt", "Aerztehaus Muster"),
            ("praxis", "Praxisgebuehr Muster"),
            ("klinik", "Klinikum Muster"),
            ("krankenhaus", "Krankenhaus Muster"),
            ("apotheke", "Apotheke am Markt"),
            ("physiotherap", "Physiotherapie Muster"),
            ("psychotherap", "Psychotherapie Muster"),
            ("heilpraktik", "Heilpraktiker Muster"),
            ("krankenkasse", "Muster Krankenkasse"),
            ("krankenversicherung", "Krankenversicherung Beitrag"),
            ("pflegeversicherung", "Pflegeversicherung Beitrag"),
            ("pflegekasse", "Pflegekasse Muster"),
            ("medizin", "Medizintechnik Muster"),
            ("arznei", "Arzneimittel Muster"),
            ("gewerkschaft", "Gewerkschaftsbeitrag Muster"),
            ("ver.di", "ver.di Mitgliedsbeitrag"),
            ("verdi", "VERDI Mitgliedsbeitrag"),
            ("ig metall", "IG Metall Beitrag"),
            ("igmetall", "IGMetall Beitrag"),
            ("ig bau", "IG Bau Beitrag"),
            ("ig bce", "IG BCE Beitrag"),
            ("dgb", "DGB Rechtsschutz"),
            ("berufsverband", "Berufsverband Muster"),
            ("kirch", "Kirchensteuer Muster"),
            ("bistum", "Bistum Muster"),
            ("diözese", "Diözese Muster"),
            ("dioezese", "Dioezese Muster"),
            ("pfarr", "Pfarramt Muster"),
            ("evangelisch", "Evangelische Gemeinde Muster"),
            ("katholisch", "Katholische Gemeinde Muster"),
            ("moschee", "Moscheeverein Muster"),
            ("synagoge", "Synagoge Muster"),
        ] {
            assert!(ist_art9_buchungstext(beispiel), "{alternative:?}: {beispiel:?} muss treffen");
        }
    }

    /// Die Grenzen des Wortschatzes: Wortgrenzen bei `ver.di`, `ig metall`, `ig bau`, `ig bce` und `dgb`, dazu Alltag, der einem
    /// Wortteil gleicht ("Verdienst", "Metallbau") oder einer Kasse ohne eines der Woerter ("AOK", "Barmer", `ponytail` am Muster).
    #[test]
    fn art9_buchungstext_trifft_nicht_bei_fehlender_wortgrenze_und_nicht_bei_alltag() {
        use super::ist_art9_buchungstext;
        for nein in [
            "Verdienstausfall",
            "Zeitig Metall Handel",
            "IG Metallbau Huber",
            "Digital Bau Muster",
            "IG Baumarkt Muster",
            "IG BCEX Muster",
            "DGBX Muster",
            "Adgb Muster",
            "AOK Beitrag",
            "Barmer Beitrag",
            "Tankstelle Muster",
            "Baeckerei Muster",
            "Miete Januar",
            "Stromversorgung Muster",
        ] {
            assert!(!ist_art9_buchungstext(nein), "{nein:?} darf nicht treffen");
        }
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
