//! Vergessene Pauschalen (`produkt/konsistenz/check_pauschalen.py`). Weiche Hinweise, keine
//! Widersprüche: eine Einkunftsquelle ist bestätigt, das Feld für die zugehörige Pauschale fehlt.
use domain::PyWert;
use serde_json::Value;

use crate::lesung::{lies, Felder};
use crate::zahl::{als_json, als_text, leer_nach_strip, zahl_gleich_null, zahl_gt0};

/// Ein Pauschal-Check (`check_pauschalen.py:19-44`).
#[derive(Debug, Clone, Copy)]
pub struct PauschalCheck {
    pub id: &'static str,
    /// Einkunftsfelder, deren bestätigter Wert > 0 die Pauschale relevant macht.
    pub ausloeser_felder: &'static [&'static str],
    /// Felder, die der Nutzer für die Pauschale ausfüllen müsste.
    pub pauschal_felder: &'static [&'static str],
    pub label: &'static str,
    pub hinweis: &'static str,
    /// Nur melden, wenn ALLE Pauschal-Felder leer sind (Python: Sonderfall `id == "vv_wk"`,
    /// `check_pauschalen.py:90-92`) — als Datenfeld statt als Id-Vergleich im Code.
    pub nur_wenn_alle_leer: bool,
}

/// Die drei Checks in Python-Reihenfolge.
pub const PAUSCHAL_CHECKS: [PauschalCheck; 3] = [
    PauschalCheck {
        id: "sparer_pb",
        ausloeser_felder: &["kap_kapitalertraege", "kap_gewinn_aktien"],
        pauschal_felder: &["veranlagung"],
        label: "Sparer-Pauschbetrag (§ 20 Abs. 9)",
        hinweis: "Kapitaleinkünfte vorhanden, aber der Sparer-Pauschbetrag (1.000/2.000 €) ist \
                  nur mit Angabe der Veranlagungsart korrekt bestimmbar.",
        nur_wenn_alle_leer: false,
    },
    PauschalCheck {
        id: "ep_arbeitstage",
        ausloeser_felder: &["bruttoarbeitslohn"],
        pauschal_felder: &["ep_arbeitstage"],
        label: "Entfernungspauschale (EP-Arbeitstage)",
        hinweis: "Arbeitslohn vorhanden, aber keine Anzahl an Arbeitstagen für die \
                  Entfernungspauschale angegeben. Möglicherweise wurde die Pauschale vergessen.",
        nur_wenn_alle_leer: false,
    },
    PauschalCheck {
        id: "vv_wk",
        ausloeser_felder: &["vv_einnahmen"],
        pauschal_felder: &["vv_schuldzinsen", "vv_erhaltungsaufwand", "vv_sonstige_wk"],
        label: "Werbungskosten bei Vermietung und Verpachtung (§ 21)",
        hinweis:
            "Einnahmen aus Vermietung vorhanden, aber keine Werbungskosten erfasst. \
                  Möglicherweise wurden Ausgaben (Schuldzinsen, Erhaltungsaufwand, etc.) vergessen.",
        nur_wenn_alle_leer: true,
    },
];

/// Ein Pauschal-Hinweis (`check_pauschalen.py:94-100`).
#[derive(Debug, Clone, PartialEq)]
pub struct PauschalHinweis {
    pub check_id: &'static str,
    pub label: &'static str,
    pub hinweis: &'static str,
    /// Auslösende Felder mit ihrem bestätigten Wert.
    pub ausloeser_felder: Vec<(&'static str, Value)>,
    pub fehlende_felder: Vec<&'static str>,
}

/// `_pauschal_feld_ist_leer` (`check_pauschalen.py:53-61`): fehlt, unbestätigt, `null`, `false`,
/// `0` oder nur Leerraum.
///
/// Die drei Zweige des Quells in seiner Reihenfolge: `w is None or w is False or w == 0`. Das
/// erste `is` ist Identität auf `False` (strukturell), das zweite `==` ist Pythons Vergleich
/// (`zahl_gleich_null`, das `bool` ausklammert — `True == 0` ist in `CPython` falsch).
fn ist_leer(felder: &Felder, feld_id: &str) -> bool {
    match lies(felder, feld_id).bestaetigt() {
        None | Some(PyWert::Bool(false)) => true,
        Some(w) if zahl_gleich_null(w) => true,
        // `isinstance(w, str) and not w.strip()` — alles, was kein Text ist, ist nicht leer.
        Some(w) => als_text(w).is_some_and(leer_nach_strip),
    }
}

/// Snapshot → Pauschal-Hinweise (`check_pauschalen.py:64-101`).
///
/// ```
/// use konsistenz::{pauschal_hinweise, Felder};
/// assert!(pauschal_hinweise(&Felder::new()).is_empty());
/// ```
#[must_use]
pub fn pauschal_hinweise(felder: &Felder) -> Vec<PauschalHinweis> {
    let mut hinweise = Vec::new();
    for check in &PAUSCHAL_CHECKS {
        let ausloeser: Vec<(&'static str, Value)> = check
            .ausloeser_felder
            .iter()
            .filter_map(|&fid| {
                lies(felder, fid)
                    .bestaetigt()
                    .filter(|w| zahl_gt0(w))
                    .map(|w| (fid, als_json(w)))
            })
            .collect();
        if ausloeser.is_empty() {
            continue;
        }
        let fehlende: Vec<&'static str> = check
            .pauschal_felder
            .iter()
            .copied()
            .filter(|fid| ist_leer(felder, fid))
            .collect();
        if fehlende.is_empty()
            || (check.nur_wenn_alle_leer && fehlende.len() < check.pauschal_felder.len())
        {
            continue;
        }
        hinweise.push(PauschalHinweis {
            check_id: check.id,
            label: check.label,
            hinweis: check.hinweis,
            ausloeser_felder: ausloeser,
            fehlende_felder: fehlende,
        });
    }
    hinweise
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lesung::test_snap as snap;
    use domain::Zustand::{Bestaetigt, Vorlaeufig};

    #[test]
    fn vv_wk_ein_feld_reicht() {
        let s = snap(&[
            ("vv_einnahmen", PyWert::Ganz(100), Bestaetigt),
            ("vv_schuldzinsen", PyWert::Ganz(5), Bestaetigt),
        ]);
        assert!(pauschal_hinweise(&s).is_empty());
        let s = snap(&[
            ("vv_einnahmen", PyWert::Ganz(100), Bestaetigt),
            ("vv_schuldzinsen", PyWert::Ganz(5), Vorlaeufig),
        ]);
        assert_eq!(pauschal_hinweise(&s)[0].fehlende_felder.len(), 3);
    }

    #[test]
    fn bool_ist_kein_ausloeser() {
        let s = snap(&[("bruttoarbeitslohn", PyWert::Bool(true), Bestaetigt)]);
        assert!(pauschal_hinweise(&s).is_empty());
    }
}
