//! Vorjahres-Uebernahme (`produkt/eingang/vorjahr_writer.py`): nur Felder mit Flag
//! `vorjahr ∈ {uebernehmbar, vorschlag}`, nur BESTAETIGTE Vorjahreswerte, nie ueberschreiben.
use std::collections::{BTreeMap, HashSet};

use bindung::Vorjahr;
use serde_json::{json, Value};
use store::{BindungNachschlag, Store};

use crate::vorschlag::{Quelle, SchreibFehler, VorschlagEvent};

/// Ein materialisiertes Vorjahresfeld (`{wert, zustand, herkunft}`); nur `wert`/`zustand`
/// zaehlen.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct VorjahrFeld {
    pub wert: Value,
    #[serde(default)]
    pub zustand: Option<String>,
}

/// Ergebnis: Zahl der Uebernahmen und die Vergleichsgroesse `verlustvortrag_bestand`, die der
/// Aufrufer im Fall ablegt (Python schreibt sie direkt als `store["vorjahr_referenz"]`; die
/// Rust-`StoreDatei` kennt das Feld noch nicht — offener Punkt fuer `api`/`store`).
#[derive(Debug, Clone, PartialEq)]
pub struct VorjahrErgebnis {
    pub uebertragen: usize,
    pub referenz: Option<Value>,
}

/// `uebertragbare_felder(bindung)`: `feld_id → Kategorie`, sortiert.
///
/// ```
/// use eingang::vorjahr::uebertragbare_felder;
/// use store::BindungNachschlag;
/// let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
/// let felder = uebertragbare_felder(nachschlag);
/// assert!(felder.values().all(|k| *k == "uebernehmbar" || *k == "vorschlag"));
/// ```
#[must_use]
pub fn uebertragbare_felder(bindung: BindungNachschlag<'_>) -> BTreeMap<String, &'static str> {
    bindung
        .alle()
        .filter_map(|(fid, b)| {
            b.vorjahr.map(|v| {
                (fid.to_owned(), match v {
                    Vorjahr::Uebernehmbar => "uebernehmbar",
                    Vorjahr::Vorschlag => "vorschlag",
                })
            })
        })
        .collect()
}

/// `uebernehme_vorjahr(neuer_store, vorjahr_felder, bindung, vorjahr_vz=, ts=)`.
///
/// Reihenfolge: sortiert nach `feld_id`. Python folgt der `glob`-Reihenfolge der Bindungsdateien;
/// das Ergebnis ist davon unabhaengig, weil vorlaeufige Events keine Ableitung ausloesen
/// (`store::Store::append`, `leite_ab`/`rechne_ab` nur fuer `bestaetigt`).
///
/// # Errors
/// [`SchreibFehler`] beim ersten abgewiesenen Feld.
///
/// ```
/// use std::collections::BTreeMap;
/// use eingang::vorjahr::{uebernehme, uebertragbare_felder, VorjahrFeld};
/// use store::Store;
/// use store::BindungNachschlag;
/// let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
/// let mut store = Store::leer(2026, None);
/// // Ein Vorjahrsfeld ohne bestaetigten Zustand wird nie uebertragen.
/// let offen: BTreeMap<String, VorjahrFeld> = uebertragbare_felder(nachschlag)
///     .into_keys()
///     .map(|f| (f, VorjahrFeld { wert: serde_json::json!(1), zustand: Some("vorlaeufig".into()) }))
///     .collect();
/// let erg = uebernehme(&mut store, &offen, nachschlag, 2025, None).unwrap();
/// assert_eq!(erg.uebertragen, 0);
/// ```
pub fn uebernehme(
    store: &mut Store,
    vorjahr_felder: &BTreeMap<String, VorjahrFeld>,
    bindung: BindungNachschlag<'_>,
    vorjahr_vz: i64,
    ts: Option<&str>,
) -> Result<VorjahrErgebnis, SchreibFehler> {
    let aktiv: HashSet<String> = store.aktive().map(|(f, _)| f.to_owned()).collect();
    let mut n = 0;
    for (fid, kat) in uebertragbare_felder(bindung) {
        let Some(vf) = vorjahr_felder.get(&fid).filter(|v| v.zustand.as_deref() == Some("bestaetigt")) else { continue };
        if aktiv.contains(&fid) {
            continue;
        }
        let signal_1 = json!({"typ": "vorjahr", "vz": vorjahr_vz, "quell_feld_id": fid, "quell_wert": vf.wert, "kategorie": kat});
        VorschlagEvent { quelle: Quelle::Vorjahr, feld_id: fid.clone(), wert: vf.wert.clone(), signal_1 }.schreibe(store, None, bindung, ts)?;
        n += 1;
    }
    Ok(VorjahrErgebnis { uebertragen: n, referenz: referenzwert_verlustvortrag(vorjahr_felder) })
}

/// `referenzwert_verlustvortrag`: `{"verlustvortrag_bestand": {"wert": …}}`, nur bei
/// bestaetigtem Vorjahreswert.
///
/// ```
/// let mut f = std::collections::BTreeMap::new();
/// f.insert("verlustvortrag_bestand".to_string(), eingang::vorjahr::VorjahrFeld { wert: 5.into(), zustand: Some("bestaetigt".into()) });
/// assert_eq!(eingang::vorjahr::referenzwert_verlustvortrag(&f), Some(serde_json::json!({"verlustvortrag_bestand": {"wert": 5}})));
/// ```
#[must_use]
pub fn referenzwert_verlustvortrag(vorjahr_felder: &BTreeMap<String, VorjahrFeld>) -> Option<Value> {
    let vf = vorjahr_felder.get("verlustvortrag_bestand").filter(|v| v.zustand.as_deref() == Some("bestaetigt"))?;
    Some(json!({"verlustvortrag_bestand": {"wert": vf.wert}}))
}
