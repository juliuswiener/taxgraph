//! Rueckwaerts, Teil 1: welche Regel gilt noch (`relevanz`, `traverser.py:290-338`) und wie viel
//! ein Gate abschaltet (`gate_gewicht`, `traverser.py:341-377`).
use std::collections::{BTreeMap, HashMap};

use bindung::{Bindung, Bindungspunkt};
use domain::Feldtyp;
use serde::Serialize;
use serde_json::Value;
use store::Store;

use crate::antwort::{py_eq, Aktiv, Antwort};
use crate::graph::{Graph, Sicht};
use crate::instanz::instanz_antworten;

/// Status einer Regel (Python: die Strings `"ausgeschlossen" | "relevant" | "unentschieden"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Regelstatus {
    /// Eine Bedingung ist bestaetigt verneint — die Regel faellt weg.
    Ausgeschlossen,
    /// Alle Gates bestaetigt bejaht (oder keine).
    Relevant,
    /// Mindestens ein Gate offen oder nur vorlaeufig beantwortet.
    Unentschieden,
}

/// Ergebnis einer Bedingung ueber alle Instanzen (`_bedingung_je_instanz`,
/// `traverser.py:268-281`, Python: `"offen" | "ausgeschlossen" | "erfuellt"`).
///
/// Ausgeschlossen erst, wenn JEDE Instanz bestaetigt abweicht. Zwei Kinder, Kind 1 "unter 14"
/// nein, Kind 2 ja: die Kinderbetreuung bleibt. Eine offene Instanz schliesst nie aus
/// (fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bedingungsstand {
    Offen,
    Ausgeschlossen,
    Erfuellt,
}

impl Bedingungsstand {
    /// Aus den Instanz-Antworten.
    ///
    /// ```
    /// use interview::{Antwort, Bedingungsstand};
    /// use serde_json::json;
    /// let (nein, ja) = (json!(false), json!(true));
    /// let weicht_ab = |w: &serde_json::Value| *w == json!(false);
    /// assert_eq!(Bedingungsstand::aus(&[Antwort::Bestaetigt(&nein), Antwort::Offen], weicht_ab), Bedingungsstand::Offen);
    /// assert_eq!(Bedingungsstand::aus(&[Antwort::Bestaetigt(&nein), Antwort::Bestaetigt(&ja)], weicht_ab), Bedingungsstand::Erfuellt);
    /// assert_eq!(Bedingungsstand::aus(&[Antwort::Bestaetigt(&nein)], weicht_ab), Bedingungsstand::Ausgeschlossen);
    /// ```
    pub fn aus(antworten: &[Antwort<'_>], weicht_ab: impl Fn(&Value) -> bool) -> Self {
        let mut alle_weichen_ab = true;
        for a in antworten {
            match a {
                Antwort::Offen => return Self::Offen,
                Antwort::Bestaetigt(w) => alle_weichen_ab &= weicht_ab(w),
            }
        }
        if alle_weichen_ab { Self::Ausgeschlossen } else { Self::Erfuellt }
    }
}

/// Die EINE Nachschlagestelle fuer jede Bedingung, die eine Antwort prueft: Gate und
/// `regel_bedingung` in [`relevanz`], `feld_bedingung` in `naechste_fragen`.
pub(crate) fn bedingung_je_instanz(
    aktiv: &Aktiv<'_>,
    sicht: &Sicht<'_>,
    graph: &Graph<'_>,
    feld: &str,
    weicht_ab: impl Fn(&Value) -> bool,
) -> Bedingungsstand {
    Bedingungsstand::aus(&instanz_antworten(aktiv, sicht, graph, feld), weicht_ab)
}

/// Relevanz einer Regel (`relevanz()[regel_id]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegelRelevanz<'r> {
    pub status: Regelstatus,
    /// Offene Gates, sortiert. Bei Ausschluss durch ein Gate nur die offenen Gates, die in
    /// Sicht-Reihenfolge VOR ihm stehen (Python bricht die Schleife ab).
    pub gates_offen: Vec<&'r str>,
    /// Geltungsbedingungen, die kein Gate sind (nicht askable oder `gate: false`), sortiert —
    /// nie still als erfuellt (Auflage 1-Zusatz).
    pub annahmen_offen: Vec<&'r str>,
}

/// Je Regel der Sicht: Status, offene Gates, offene Annahmen (`relevanz`,
/// `traverser.py:290-338`).
///
/// Gate = askable Geltungsbedingung ohne `gate: false`. Bestaetigt `false` (jede Instanz) ->
/// ausgeschlossen; offen/vorlaeufig -> unentschieden. `gate: false` macht ein askables Feld zur
/// DEKLARATION: das "nein" des Normalfalls nahm dem Vermieter sonst die ganze Anlage V aus dem
/// Dialog (gemessen 2026-08-16). Zusaetzlich `regel_bedingungen`: bestaetigt UND abweichend
/// (Python `!=`) -> ausgeschlossen; unbeantwortet/vorlaeufig schliesst NICHT aus.
///
/// ```
/// use interview::Regelstatus;
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// let rel = interview::relevanz(&store::Store::leer(2025, None), g.alle(), &g);
/// assert!(rel.values().all(|r| r.status != Regelstatus::Ausgeschlossen));
/// ```
#[must_use]
pub fn relevanz<'r>(store: &Store, sicht: &Sicht<'r>, graph: &Graph<'r>) -> BTreeMap<&'r str, RegelRelevanz<'r>> {
    relevanz_mit(&Aktiv::aus(store), sicht, graph)
}

pub(crate) fn relevanz_mit<'r>(
    aktiv: &Aktiv<'_>,
    sicht: &Sicht<'r>,
    graph: &Graph<'r>,
) -> BTreeMap<&'r str, RegelRelevanz<'r>> {
    // Je Regel: Gates in Sicht-Reihenfolge, Annahmen.
    let mut je_regel: BTreeMap<&'r str, (Vec<&'r str>, Vec<&'r str>)> = BTreeMap::new();
    for b in sicht.iter() {
        let eintrag = je_regel.entry(b.quelle.regel_id.as_str()).or_default();
        if let Bindungspunkt::Geltungsbedingung(gb) = &b.quelle.bindungspunkt {
            if b.askable && b.gate.unwrap_or(true) {
                eintrag.0.push(b.feld_id.as_str());
            } else {
                eintrag.1.push(gb.as_str());
            }
        }
    }
    je_regel
        .into_iter()
        .map(|(rid, (gates, mut annahmen))| {
            let mut status = Regelstatus::Relevant;
            let mut offen = Vec::new();
            for cond in graph.regel_bedingungen(rid) {
                let stand = bedingung_je_instanz(aktiv, sicht, graph, &cond.feld, |w| !py_eq(w, &cond.wert));
                if stand == Bedingungsstand::Ausgeschlossen {
                    status = Regelstatus::Ausgeschlossen;
                }
            }
            if status != Regelstatus::Ausgeschlossen {
                for fid in gates {
                    match bedingung_je_instanz(aktiv, sicht, graph, fid, |w| *w == Value::Bool(false)) {
                        Bedingungsstand::Offen => offen.push(fid),
                        Bedingungsstand::Ausgeschlossen => {
                            status = Regelstatus::Ausgeschlossen;
                            break;
                        }
                        Bedingungsstand::Erfuellt => {}
                    }
                }
                if status != Regelstatus::Ausgeschlossen {
                    status = if offen.is_empty() { Regelstatus::Relevant } else { Regelstatus::Unentschieden };
                }
            }
            offen.sort_unstable();
            annahmen.sort_unstable();
            (rid, RegelRelevanz { status, gates_offen: offen, annahmen_offen: annahmen })
        })
        .collect()
}

/// `feld_id -> Zahl der askable Felder`, die die Antwort auf dieses Gate abschalten kann
/// (`gate_gewicht`, `traverser.py:341-377`); nur askable Felder stehen drin.
///
/// Zwei Quellen, beide aus der Bindung: (a) eigenes bool-Gate der Regel — alle UEBRIGEN askable
/// Felder derselben Regel entfallen mit; (b) das Feld steht in `regel_bedingungen` einer Regel —
/// deren askable Felder (der Sicht) zaehlen. Gemessen 2026-08-14: `veranlagung` 38; alphabetisch
/// stand es auf Frage 203 von 243.
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// let gw = interview::gate_gewicht(g.alle(), &g);
/// assert!(gw["veranlagung"] > 0);
/// ```
#[must_use]
pub fn gate_gewicht<'r>(sicht: &Sicht<'r>, graph: &Graph<'r>) -> HashMap<&'r str, usize> {
    let mut askable_je_regel: HashMap<&str, usize> = HashMap::new();
    for b in sicht.iter().filter(|b| b.askable) {
        *askable_je_regel.entry(b.quelle.regel_id.as_str()).or_default() += 1;
    }
    let zahl = |rid: &str| askable_je_regel.get(rid).copied().unwrap_or(0);
    sicht
        .iter()
        .filter(|b| b.askable)
        .map(|b: &'r Bindung| {
            let mut n = 0;
            if matches!(b.quelle.bindungspunkt, Bindungspunkt::Geltungsbedingung(_)) && b.typ == Feldtyp::Bool {
                // alle askable Felder der eigenen Regel ausser diesem
                n += zahl(&b.quelle.regel_id).saturating_sub(1);
            }
            n += graph.bedingte_regeln(&b.feld_id).iter().map(|rid| zahl(rid)).sum::<usize>();
            (b.feld_id.as_str(), n)
        })
        .collect()
}
