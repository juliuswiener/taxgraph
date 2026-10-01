//! Partnerangaben ↔ Veranlagung (`produkt/konsistenz/partner_check.py`).
//!
//! § 26b `EStG`: Partnerbezogene Angaben (§ 33b Person B, § 20 Kapital B, § 22 Rente B) setzen
//! voraus, dass der Partner Teil DIESER Erklärung ist. § 24b Abs. 1/3: der Entlastungsbetrag für
//! Alleinerziehende setzt „allein stehend" voraus — mit Zusammenveranlagung unvereinbar.
//!
//! Die Handliste bleibt eine Handliste (Parität). `domain::Veranlagung::Einzel { a }` macht
//! Partnerdaten im RECHEN-Eingang unrepräsentierbar; der Store kann sie aber weiter halten
//! (Korrektur auf „einzel" nach bestätigten Partnerwerten) — genau das meldet diese Prüfung.
use domain::Veranlagung;
use serde_json::Value;

use crate::lesung::{lage_veranlagung, lies, Felder};
use crate::zahl::zahl_gt0;

/// Partnerfelder, die eine Zusammenveranlagung voraussetzen (`partner_check.py:17-29`).
/// `rentner_*_partner`: Instanz-Reuse derselben Kz E0109708/E0109706 wie Person A. Für die Rente
/// zählt nur der Betrag — die int-Meta (Beginn, Alter) wären über `> 0` falsch-positiv.
pub const PARTNER_FELDER: [&str; 8] = [
    "rentner_grad_der_behinderung_partner",
    "rentner_hilflos_blind_taubblind_partner",
    "kap_kapitalertraege_partner",
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige_partner",
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige_partner",
    "rentner_jahresrente_partner",
];

fn partner_name(feld_id: &str) -> &str {
    match feld_id {
        "rentner_grad_der_behinderung_partner" => "Grad der Behinderung des Partners",
        "rentner_hilflos_blind_taubblind_partner" => "Behinderten-Merkzeichen des Partners",
        "kap_kapitalertraege_partner" => "Kapitaleinkünfte des Partners",
        "kap_gewinn_aktien_partner" => "Aktiengewinne des Partners",
        "kap_gewinn_sonstige_partner" => "sonstige Kapitalgewinne des Partners",
        "kap_verlust_aktien_partner" => "Aktienverluste des Partners",
        "kap_verlust_sonstige_partner" => "sonstige Kapitalverluste des Partners",
        "rentner_jahresrente_partner" => "Rentenbezüge des Partners",
        andere => andere,
    }
}

/// Ein Partner↔Veranlagung-Widerspruch (beide Richtungen, gleiche Form wie in Python).
#[derive(Debug, Clone, PartialEq)]
pub struct PartnerWiderspruch {
    pub feld_id: &'static str,
    pub wert: Value,
    /// Der bestätigte Veranlagungswert, wie im Snapshot.
    pub veranlagung: Value,
    /// Klartext, byte-gleich zu Python.
    pub grund: String,
}

/// `_ist_gesetzt` (`partner_check.py:35-39`): `GdB` > 0 oder Merkzeichen `true`.
fn ist_gesetzt(wert: &Value) -> bool {
    *wert == Value::Bool(true) || zahl_gt0(wert)
}

/// Partnerfeld bestätigt gesetzt UND Veranlagung bestätigt ≠ „zusammen" (`partner_check.py:42-76`).
/// Unbestätigte Veranlagung ist Unvollständigkeit, kein Widerspruch.
///
/// ```
/// use konsistenz::{partner_ohne_zusammen, Felder};
/// assert!(partner_ohne_zusammen(&Felder::new()).is_empty());
/// ```
#[must_use]
pub fn partner_ohne_zusammen(felder: &Felder) -> Vec<PartnerWiderspruch> {
    let Some(veranlagung) = lies(felder, "veranlagung").bestaetigt() else {
        return Vec::new();
    };
    // Typisiert statt `veranlagung.as_str() == Some("zusammen")`: die Entscheidung „ist das
    // zusammen?" faellt jetzt in `Lage::veranlagung` (`domain/src/lage.rs:49`), nicht hier.
    //
    // PARITAET, an `partner_check.py:58` gemessen: Python bricht NUR bei exakt `"zusammen"` ab.
    // Ein abweichender Wert (`"Zusammen"`, `5`, `true`) ist dort nicht `"zusammen"` und laeuft
    // WEITER — deshalb steht hier `Some(Veranlagung::Zusammen)` und nicht „nicht abweichend".
    // `Lage::Abweichend` verhaelt sich damit wie in Python; das Feld `veranlagung` unten traegt
    // weiter den ROHEN Wert, weil Python ihn roh in das dict schreibt.
    if matches!(lage_veranlagung(felder).0, Some(Veranlagung::Zusammen)) {
        return Vec::new();
    }
    PARTNER_FELDER
        .iter()
        .filter_map(|&feld_id| {
            let wert = lies(felder, feld_id)
                .bestaetigt()
                .filter(|w| ist_gesetzt(w))?;
            // Feldname in Anführungszeichen statt im Satzfluss: Singular und Plural gemischt,
            // jede feste Präposition-Artikel-Kombination beugte die Hälfte falsch.
            Some(PartnerWiderspruch {
                feld_id,
                wert: wert.clone(),
                veranlagung: veranlagung.clone(),
                grund: format!(
                    "Du hast etwas bei „{}“ eingetragen, aber keine Zusammenveranlagung gewählt. \
                     Partnerbezogene Angaben sind nur bei gemeinsamer Veranlagung möglich. Bitte \
                     prüfe deine Angaben.",
                    partner_name(feld_id)
                ),
            })
        })
        .collect()
}

/// § 24b: `fam_alleinstehend` bestätigt `true` UND Veranlagung bestätigt „zusammen"
/// (`partner_check.py:79-97`). Sonst würde der Abzug still gewährt — Unterbesteuerung.
///
/// ```
/// use konsistenz::{alleinerziehend_mit_zusammen, Felder};
/// assert!(alleinerziehend_mit_zusammen(&Felder::new()).is_empty());
/// ```
#[must_use]
pub fn alleinerziehend_mit_zusammen(felder: &Felder) -> Vec<PartnerWiderspruch> {
    let alleinstehend = lies(felder, "fam_alleinstehend").bestaetigt();
    // Dieselbe Typisierung wie oben, umgekehrte Richtung. PARITAET, an `partner_check.py:91`
    // gemessen: Python laeuft NUR bei exakt `"zusammen"` weiter. Ein abweichender Wert ist dort
    // `!= "zusammen"` und bricht ab; hier ist er `None` und bricht ebenso ab.
    //
    // Der rohe Wert wird nicht mehr gebraucht: der Guard laesst nur exakt "zusammen" durch,
    // deshalb ist das Literal im `veranlagung`-Feld unten (wie in Python, das `veranlagung`
    // dort roh einsetzt) beweisbar derselbe Text.
    if !matches!(lage_veranlagung(felder).0, Some(Veranlagung::Zusammen))
        || alleinstehend != Some(&Value::Bool(true))
    {
        return Vec::new();
    }
    vec![PartnerWiderspruch {
        feld_id: "fam_alleinstehend",
        wert: Value::Bool(true),
        veranlagung: Value::String("zusammen".to_owned()),
        grund: "Du hast angegeben, alleinstehend zu sein — aber auch eine gemeinsame Veranlagung \
                mit deinem Partner gewählt. Der Entlastungsbetrag für Alleinerziehende setzt \
                voraus, dass du nicht zusammenveranlagt bist. Bitte prüfe deine Angaben."
            .to_owned(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lesung::test_snap as snap;
    use domain::Zustand::{Bestaetigt, Vorlaeufig};
    use serde_json::json;

    #[test]
    fn gdb_partner_einzel_widerspruch() {
        let s = snap(&[
            ("veranlagung", json!("einzel"), Bestaetigt),
            (
                "rentner_grad_der_behinderung_partner",
                json!(50),
                Bestaetigt,
            ),
            (
                "rentner_hilflos_blind_taubblind_partner",
                json!(false),
                Bestaetigt,
            ),
        ]);
        let w = partner_ohne_zusammen(&s);
        assert_eq!(w.len(), 1);
        assert!(w[0]
            .grund
            .starts_with("Du hast etwas bei „Grad der Behinderung des Partners“"));
    }

    #[test]
    fn unbestaetigt_kein_widerspruch() {
        let s = snap(&[
            ("veranlagung", json!("einzel"), Vorlaeufig),
            (
                "rentner_grad_der_behinderung_partner",
                json!(50),
                Bestaetigt,
            ),
        ]);
        assert!(partner_ohne_zusammen(&s).is_empty());
    }

    #[test]
    fn alleinerziehend_nur_bei_zusammen() {
        let mut s = snap(&[
            ("veranlagung", json!("zusammen"), Bestaetigt),
            ("fam_alleinstehend", json!(true), Bestaetigt),
        ]);
        assert_eq!(alleinerziehend_mit_zusammen(&s).len(), 1);
        s = snap(&[
            ("veranlagung", json!("einzel"), Bestaetigt),
            ("fam_alleinstehend", json!(true), Bestaetigt),
        ]);
        assert!(alleinerziehend_mit_zusammen(&s).is_empty());
    }
}
