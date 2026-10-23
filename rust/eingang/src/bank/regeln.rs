//! Der Regelkatalog des Bankimports: Stichwoerter im Verwendungszweck ordnen eine Buchung einem Zielfeld zu. Die Regeln stehen in
//! `daten/bankregeln.yaml`, nicht im Code; ein Waechter (`bank::tests`) verlangt je Regel eine vorhandene Fundstelle unter
//! `sources/` oder `params/` oder ein `ponytail:`.
use std::sync::OnceLock;

use llm::Kategorie;
use serde::Deserialize;

/// Was der Import mit einem Treffer tut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Art {
    /// Die Summe der Buchungen wird als vorlaeufiger Vorschlag ins Feld geschrieben.
    #[default]
    Summe,
    /// § 35a (d-61): kein Betrag ins Feld. Der Rechnungsbetrag ist nicht der Arbeitskostenanteil; die Antwort nennt die Zahlung
    /// als Nachfrage.
    ArbeitskostenNachfragen,
}

/// Eine Regel des Katalogs.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regel {
    pub id: String,
    pub feld: String,
    #[serde(default)]
    pub art: Art,
    #[serde(default)]
    pub kategorie: Option<Kategorie>,
    /// Die Regel gilt nur mit dem Haken `art9` (Gesundheit, Gewerkschaft, Kirche; Art. 9 DSGVO, d-60).
    #[serde(default)]
    pub art9: bool,
    pub stichworte: Vec<String>,
    #[serde(default)]
    pub quelle: Option<String>,
    #[serde(default)]
    pub ponytail: Option<String>,
}

static KATALOG: OnceLock<Vec<Regel>> = OnceLock::new();

/// Alle Regeln in der Reihenfolge der Datei. Eine Datei, die nicht zu lesen ist, ergibt keine Regel (nichts wird vorgeschlagen,
/// nie etwas Falsches); der Waechter `bankregeln_lader_liest_alle_regeln_der_datei` schlaegt dann an.
///
/// ```
/// assert!(eingang::bank::regeln::katalog().iter().any(|r| r.feld == "spenden_betrag"));
/// ```
#[must_use]
pub fn katalog() -> &'static [Regel] {
    KATALOG.get_or_init(|| {
        serde_yaml_ng::from_str(include_str!("../../daten/bankregeln.yaml")).unwrap_or_default()
    })
}

/// Die erste Regel, deren Stichwort im kleingeschriebenen Zweck steht. Regeln mit `art9` zaehlen nur, wenn `art9_erlaubt`.
///
/// ```
/// use eingang::bank::regeln::treffer;
/// assert_eq!(treffer("MALERMEISTER Huber", false).map(|r| r.id.as_str()), Some("handwerker"));
/// assert!(treffer("Miete", false).is_none());
/// ```
#[must_use]
pub fn treffer(zweck: &str, art9_erlaubt: bool) -> Option<&'static Regel> {
    let z = zweck.to_lowercase();
    katalog()
        .iter()
        .filter(|r| art9_erlaubt || !r.art9)
        .find(|r| r.stichworte.iter().any(|s| z.contains(s.as_str())))
}

/// Die Regel einer Kategorie der KI-Antwort.
#[must_use]
pub fn regel_fuer(kategorie: Kategorie) -> Option<&'static Regel> {
    katalog().iter().find(|r| r.kategorie == Some(kategorie))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::{katalog, regel_fuer, treffer};
    use llm::Kategorie;

    #[test]
    fn bankregeln_lader_liest_alle_regeln_der_datei() {
        let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("daten/bankregeln.yaml");
        let roh: Vec<serde_yaml_ng::Value> = serde_yaml_ng::from_str(&std::fs::read_to_string(pfad).unwrap()).unwrap();
        assert_eq!(katalog().len(), roh.len(), "der Lader liest nicht jede Regel der Datei");
    }

    #[test]
    fn bankregeln_jede_kategorie_hat_genau_eine_regel() {
        for k in [
            Kategorie::Handwerker,
            Kategorie::Dienstleistung,
            Kategorie::Minijob,
            Kategorie::Spende,
            Kategorie::Vorsorge,
        ] {
            let n = katalog().iter().filter(|r| r.kategorie == Some(k)).count();
            assert_eq!(n, 1, "{k:?}");
            assert_eq!(regel_fuer(k).unwrap().kategorie, Some(k));
        }
    }

    #[test]
    fn bankregeln_die_erste_regel_gewinnt() {
        // "rentenversicherung" (Vorsorge) und "spende" stehen beide im Zweck; die Spende steht im Katalog zuerst.
        assert_eq!(treffer("Spende Rentenversicherung", false).unwrap().id, "spende");
        assert_eq!(treffer("Rentenversicherung Spende", false).unwrap().id, "spende");
    }
}
