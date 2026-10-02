//! `api.preflight_check` (`api.py:641`): die Konsistenz-Pruefungen und die vergessenen Pauschalen
//! als Liste, die die Oberflaeche zeigt. Kein LLM, kein Schreibpfad.
use std::collections::HashSet;

use domain::{FallId, PyWert};
use serde_json::{json, Value};
use store::Store;

use crate::antwort::Antwort;
use crate::fehler::ApiFehler;
use crate::zustand::Zustand;

/// `(store.get("vorjahr_referenz") or {}).get("verlustvortrag_bestand")` und davon `"wert"`, nur
/// als Ganzzahl (`preflight.py:349-353`).
///
/// ponytail: Python nimmt auch einen `float`, Rust nur eine Ganzzahl bis `i64`, und ein
/// `vorjahr_referenz`, das kein Objekt ist, zaehlt als fehlend (Python wirft `AttributeError`). Die
/// Referenz schreibt nur `api.vorjahr` als Objekt mit Ganzzahl-Betraegen; ein groesserer Bestand
/// als `i64` (92 Billiarden Euro) kommt nicht vor. Upgrade: Gleitkomma-Zweig in
/// `konsistenz::plausibilitaets_widersprueche`.
fn vorjahr_verlustvortrag(store: &Store) -> Option<i64> {
    let feld = |obj: &PyWert, name: &str| match obj {
        PyWert::Objekt(paare) => paare
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone()),
        _ => None,
    };
    let referenz = store.datei().vorjahr_referenz.as_ref()?;
    match feld(&feld(referenz, "verlustvortrag_bestand")?, "wert")? {
        PyWert::Ganz(n) => Some(n),
        _ => None,
    }
}

/// `api.preflight_check(fall_id)` nach dem Owner-Check.
///
/// # Errors
/// 400/500 aus Scheibe und Bindung; 500 `ValueError`, wenn der Snapshot nicht entsteht.
pub fn preflight(z: &Zustand, fall_id: &FallId, store: &Store) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(store)?;
    let (felder, _) = store
        .materialisiere(None)
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    let scheibe: HashSet<String> = sb.index.keys().cloned().collect();
    let e = konsistenz::preflight(
        &felder,
        Some(&scheibe),
        vorjahr_verlustvortrag(store),
        &sb.graph,
    );
    // Nur ausliefern, was etwas zu sagen hat; die Reihenfolge der sieben Listen ist die von Python.
    // `nicht_gerechnet` ist ein eigener Bereich und laeuft NICHT unter `pauschale` mit.
    let mut items: Vec<Value> = Vec::new();
    let mut nimm = |typ: &str, bereich: &str, texte: Vec<&str>| {
        items.extend(
            texte
                .into_iter()
                .map(|text| json!({"typ": typ, "bereich": bereich, "text": text})),
        );
    };
    nimm(
        "widerspruch",
        "flag",
        e.widersprueche_flag
            .iter()
            .map(|w| w.grund.as_str())
            .collect(),
    );
    nimm(
        "widerspruch",
        "partner",
        e.widersprueche_partner
            .iter()
            .map(|w| w.grund.as_str())
            .collect(),
    );
    nimm(
        "widerspruch",
        "alleinerziehend",
        e.widersprueche_alleinerziehend
            .iter()
            .map(|w| w.grund.as_str())
            .collect(),
    );
    nimm(
        "widerspruch",
        "plausibilitaet",
        e.widersprueche_plausibilitaet
            .iter()
            .map(|w| w.grund.as_str())
            .collect(),
    );
    nimm(
        "hinweis",
        "pauschale",
        e.hinweise_pauschalen.iter().map(|h| h.hinweis).collect(),
    );
    nimm(
        "hinweis",
        "nicht_gerechnet",
        e.hinweise_nicht_gerechnet
            .iter()
            .map(|h| h.hinweis)
            .collect(),
    );
    nimm(
        "hinweis",
        "betrag_vorlaeufig",
        e.hinweise_betrag_vorlaeufig
            .iter()
            .map(|h| h.hinweis.as_str())
            .collect(),
    );
    Ok(Antwort::neu(
        200,
        json!({"fall_id": fall_id.as_str(), "status": e.status.als_str(), "items": items}),
    ))
}
