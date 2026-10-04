//! Repeated-Instance-Konvention und Round-Trip (`est_mapping.py::parse_instanz`, `instanzen`,
//! `zuruecklesen`).
//!
//! Instanz 1 ist die Basis-`feld_id` ohne Suffix, Instanz n ≥ 2 traegt `__n`. Die Instanz lebt
//! nur in der Bindung (`instanz_gruppe`) und hier; der Store kennt sie nicht.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use domain::{meet_zustand, Kz, Zustand};
use serde::Serialize;
use serde_json::Value;
use store::{SnapshotFehler, SnapshotFeld, Store};

use crate::deklaration::{BindungIndex, Deklaration};
use crate::kz_format::jahr_aus_kz_wert;
use crate::tabellen::{NEGATION, PARTNER_INSTANZ, VERZWEIGUNG};

pub(crate) fn instanz_re() -> Option<&'static regex::Regex> {
    static RE: OnceLock<Option<regex::Regex>> = OnceLock::new();
    // Python `$` passt auch vor einem abschliessenden `\n` — daher `\n?\z`.
    RE.get_or_init(|| regex::Regex::new(r"^([a-z][a-z0-9_]*)__([2-9]|[1-9][0-9]+)\n?\z").ok())
        .as_ref()
}

/// `base__<n>` (n ≥ 2) → `(base, n)`; eine Basis-`feld_id` ohne Suffix → `None` (= Instanz 1), ebenso
/// `base__1`, `base__0` und `base__02`.
///
/// PARITÄT: `est_mapping.py:702` (`_INSTANZ_RE`, Zähler `[2-9]|[1-9][0-9]+`). `x__1` ist keine Instanz:
/// die Oberfläche erzeugt es nie, und die Schreib-Route weist es ab (Entscheidung 2026-10-03). Damit
/// liest diese Regel `__1` wie die Traverser-Regel in [`domain::FeldId`] (`REWRITE_PLAN.md` §4, F2). Ein
/// Index jenseits `u64` liefert hier `None`, Python rechnet unbeschraenkt.
///
/// ```
/// use elster::parse_instanz;
/// assert_eq!(parse_instanz("vv_einnahmen__2"), Some(("vv_einnahmen", 2)));
/// assert_eq!(parse_instanz("vv_einnahmen__10"), Some(("vv_einnahmen", 10)));
/// assert_eq!(parse_instanz("vv_einnahmen__1"), None);
/// assert_eq!(parse_instanz("vv_einnahmen"), None);
/// assert_eq!(parse_instanz("vv_einnahmen__02"), None);
/// ```
#[must_use]
pub fn parse_instanz(feld_id: &str) -> Option<(&str, u64)> {
    let caps = instanz_re()?.captures(feld_id)?;
    let basis = caps.get(1)?.as_str();
    let idx = caps.get(2)?.as_str().parse().ok()?;
    Some((basis, idx))
}

/// Eine Instanz einer `instanz_gruppe` mit meet-Zustand (Ring-Naht).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Instanz {
    pub index: u64,
    /// Auf die BASIS-`feld_id` normiert (`vv_einnahmen__2` → `vv_einnahmen`).
    pub felder: BTreeMap<String, SnapshotFeld>,
    /// `bestaetigt` nur, wenn ALLE Felder der Instanz bestaetigt sind (fail-closed fuer den Ring).
    pub zustand: Zustand,
}

/// Alle Instanzen einer Gruppe aus dem Store, index-sortiert, Instanz 1 = Basis-Felder
/// (`est_mapping.py:972-1002`).
///
/// # Errors
/// [`SnapshotFehler`] aus [`Store::materialisiere`].
///
/// ```
/// use std::collections::HashMap;
/// let store = store::Store::leer(2025, None);
/// assert!(elster::instanzen(&store, &HashMap::new(), "rente").unwrap().is_empty());
/// ```
pub fn instanzen(
    store: &Store,
    bindung: &BindungIndex<'_>,
    gruppe: &str,
) -> Result<Vec<Instanz>, SnapshotFehler> {
    let (felder, _) = store.materialisiere(None)?;
    let mut vorkommen: BTreeMap<u64, BTreeMap<String, SnapshotFeld>> = BTreeMap::new();
    for (fid, sfeld) in felder {
        let (basis, idx) = parse_instanz(&fid).map_or((fid.as_str(), 1), |(b, i)| (b, i));
        let ist_basis = bindung
            .get(basis)
            .is_some_and(|b| b.instanz_gruppe.as_deref() == Some(gruppe));
        if ist_basis {
            // `x__1` ist keine Instanz (`parse_instanz`): es faellt hier mit der Basis `x__1` heraus, die es
            // in keiner Bindung gibt. Index 1 kommt nur von der Basis selbst.
            vorkommen
                .entry(idx)
                .or_default()
                .insert(basis.to_owned(), sfeld);
        }
    }
    Ok(vorkommen
        .into_iter()
        .map(|(index, felder)| {
            let zustand = meet_zustand(felder.values().map(|f| f.zustand));
            Instanz {
                index,
                felder,
                zustand,
            }
        })
        .collect())
}

/// Ergebnis von [`zuruecklesen`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rueckgelesen {
    /// `feld_id` → Wert fuer alle invertierbaren Klassen.
    pub felder: BTreeMap<String, Value>,
    /// Dokumentierte Aggregate: nur die Summe ist rekonstruierbar, nie die Details.
    pub aggregat: BTreeMap<String, i64>,
}

/// Round-Trip Deklaration → Store-Felder (`est_mapping.py:926-969`). 1:1, Negation, Person B,
/// `jahr_aus_kz_wert` fuer einen Schluessel der Deklaration. Ein Schluessel, der keine Kz ist, kann
/// in keiner Datums-Kz stehen; der Wert bleibt dann unveraendert, wie vorher.
fn jahr_aus_schluessel(wert: &Value, schluessel: &str) -> Value {
    Kz::new(schluessel).map_or_else(|_| wert.clone(), |kz| jahr_aus_kz_wert(wert, &kz))
}

/// Instanzen und Verzweigungs-Werte sind invertierbar; die Aggregation nur als Summe.
///
/// ```
/// use std::collections::HashMap;
/// use elster::{deklariere, zuruecklesen, Felder};
/// let d = deklariere(&Felder::new(), &HashMap::new(), 2025, None).unwrap();
/// assert!(zuruecklesen(&d, &HashMap::new()).felder.is_empty());
/// ```
#[must_use]
pub fn zuruecklesen(result: &Deklaration, bindung: &BindungIndex<'_>) -> Rueckgelesen {
    let kz_von = |b: &bindung::Bindung| b.elster_kz.as_ref().map(|k| k.as_str().to_owned());
    let e_nach_feld: BTreeMap<String, &str> = bindung
        .iter()
        .filter_map(|(fid, b)| kz_von(b).map(|k| (k, fid.as_str())))
        .collect();
    let inst_kz_nach_feld: BTreeMap<String, &str> = bindung
        .iter()
        .filter(|(_, b)| b.instanz_gruppe.as_deref().is_some_and(|g| !g.is_empty()))
        .filter_map(|(fid, b)| kz_von(b).map(|k| (k, fid.as_str())))
        .collect();
    let mut e_nach_verzweigung: BTreeMap<&str, &str> = BTreeMap::new();
    for v in VERZWEIGUNG {
        for (_, kz) in v.kz.paare() {
            e_nach_verzweigung.insert(kz, v.feld);
        }
    }
    let b_nach_feld: BTreeMap<&str, &str> = PARTNER_INSTANZ.iter().map(|(f, k)| (*k, *f)).collect();
    let e_nach_negation: BTreeMap<&str, &str> = NEGATION.iter().map(|(f, k)| (*k, *f)).collect();

    let mut felder = BTreeMap::new();
    let mut aggregat = BTreeMap::new();
    for (e_nr, info) in &result.dokumentiert {
        aggregat.insert(e_nr.clone(), info.summe);
    }
    for (e_nr, wert) in &result.person_b {
        if let Some(f) = b_nach_feld.get(e_nr.as_str()) {
            felder.insert((*f).to_owned(), wert.clone());
        }
    }
    for (_, instanzen) in &result.anlage_instanzen {
        for inst in instanzen {
            let idx = inst.index;
            for (kz, wert) in &inst.felder {
                if let Some(f) = inst_kz_nach_feld.get(kz) {
                    felder.insert(format!("{f}__{idx}"), wert.clone());
                } else if let Some(f) = e_nach_verzweigung.get(kz.as_str()) {
                    felder.insert(format!("{f}__{idx}"), jahr_aus_schluessel(wert, kz));
                }
            }
            for (ziel, agg) in &inst.dokumentiert {
                aggregat.insert(format!("{ziel}__{idx}"), agg.summe);
            }
        }
    }
    for (e_nr, wert) in &result.deklaration {
        if let Some(f) = e_nach_negation.get(e_nr.as_str()) {
            felder.insert((*f).to_owned(), Value::Bool(!crate::py::truthy(wert)));
        } else if let Some(f) = e_nach_verzweigung.get(e_nr.as_str()) {
            felder.insert((*f).to_owned(), jahr_aus_schluessel(wert, e_nr));
        } else if let Some(f) = e_nach_feld.get(e_nr) {
            felder.insert((*f).to_owned(), wert.clone());
        }
    }
    Rueckgelesen { felder, aggregat }
}
