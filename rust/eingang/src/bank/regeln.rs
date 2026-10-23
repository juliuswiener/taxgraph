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

impl Art {
    /// Das Wort in der Antwort der Route und in der Regeldatei.
    #[must_use]
    pub fn als_str(self) -> &'static str {
        match self {
            Self::Summe => "summe",
            Self::ArbeitskostenNachfragen => "arbeitskosten_nachfragen",
        }
    }
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
    /// Satz fuer die Nutzerin, wenn die Regel einen Vorschlag schreibt.
    #[serde(default)]
    pub hinweis: Option<String>,
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

    /// Jedes Stichwort einer Art.-9-Regel steht im Wortschatz der KI-Sperre. Fehlte eines, ginge ein Text mit diesem Wort ohne den
    /// Haken `art9` an das Sprachmodell: die Regel greift nicht (ohne Haken), und die Sperre sieht den Text nicht.
    #[test]
    fn bankregeln_jedes_art9_stichwort_steht_im_wortschatz() {
        let art9: Vec<_> = katalog().iter().filter(|r| r.art9).collect();
        assert!(art9.len() >= 2, "{art9:?}");
        for r in art9 {
            for s in &r.stichworte {
                assert!(llm::pii::ist_art9_buchungstext(s), "{}: Stichwort {s:?} fehlt im Wortschatz art9_buchungstext", r.id);
            }
        }
    }

    /// Mit und ohne Haken: beide Art.-9-Regeln schlagen ohne den Haken nie an; mit dem Haken gewinnt bei beiden Woertern die erste
    /// Regel des Katalogs (die Reihenfolge der Datei, vorlaeufig und vom Nutzer zu bestaetigen).
    #[test]
    fn bankregeln_art9_regeln_gelten_nur_mit_haken_und_die_erste_gewinnt() {
        for zweck in ["Techniker Krankenversicherung Beitrag", "AOK Krankenkasse", "Pflegeversicherung Beitrag"] {
            assert!(treffer(zweck, false).is_none(), "{zweck:?} ohne Haken");
        }
        assert_eq!(treffer("Techniker Krankenversicherung Beitrag", true).unwrap().feld, "basis_kv");
        assert_eq!(treffer("AOK Krankenkasse", true).unwrap().feld, "basis_kv");
        assert_eq!(treffer("Pflegeversicherung Beitrag", true).unwrap().feld, "basis_pv");
        assert_eq!(treffer("Krankenversicherung und Pflegeversicherung", true).unwrap().feld, "basis_kv");
        assert_eq!(treffer("Pflegeversicherung und Krankenversicherung", true).unwrap().feld, "basis_kv");
    }

    /// § 35a (d-61): die drei Felder tragen die Art "nachfragen", alle anderen Regeln schreiben die Summe.
    #[test]
    fn bankregeln_nur_die_35a_felder_fragen_nach() {
        use super::Art;
        for r in katalog() {
            let nachfragen = ["hh_handwerker_betrag", "hh_dienstleistung_betrag", "hh_minijob_betrag"].contains(&r.feld.as_str());
            assert_eq!(r.art == Art::ArbeitskostenNachfragen, nachfragen, "{}", r.id);
        }
    }

    #[test]
    fn bankregeln_die_erste_regel_gewinnt() {
        // "rentenversicherung" (Vorsorge) und "spende" stehen beide im Zweck; die Spende steht im Katalog zuerst.
        assert_eq!(treffer("Spende Rentenversicherung", false).unwrap().id, "spende");
        assert_eq!(treffer("Rentenversicherung Spende", false).unwrap().id, "spende");
    }
}
