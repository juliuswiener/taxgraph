//! `api.graph` (`api.py:833`): die Abhaengigkeits-Uebersicht der Scheibe. Knoten sind die Regeln mit
//! ihrem Relevanz-Status, Kanten die Felder mit der Regel, die sie speisen, und ihrem Zustand.
//! Reine Ableitung: ein `relevanz`-Aufruf, kein Bescheid, kein Schreibpfad.
use bindung::Bindungspunkt;
use domain::FallId;
use serde_json::{json, Value};
use store::Store;

use crate::antwort::Antwort;
use crate::fehler::ApiFehler;
use crate::zustand::Zustand;

/// `api.graph(fall_id)` nach dem Owner-Check.
///
/// # Errors
/// 400/500 aus Scheibe und Bindung; 500 `ValueError`, wenn der Snapshot nicht entsteht.
pub fn graph(z: &Zustand, fall_id: &FallId, store: &Store) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(store)?;
    let (felder, sid) = store
        .materialisiere(None)
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    let rel = interview::relevanz(store, &sb.sicht, &sb.graph);
    // `sorted(rel.items())`: die Map ist nach `regel_id` geordnet.
    let knoten: Vec<Value> = rel
        .iter()
        .map(|(rid, s)| {
            json!({
                "regel_id": rid, "status": s.status,
                "gates_offen": s.gates_offen, "annahmen_offen": s.annahmen_offen,
            })
        })
        .collect();
    // `sorted(bindung)`: Pythons Text-Ordnung ist die der Codepunkte, die von `str` in Rust die der
    // UTF-8-Bytes — dieselbe.
    let mut eintraege: Vec<_> = sb.index.iter().collect();
    eintraege.sort_by(|a, b| a.0.cmp(b.0));
    let kanten: Vec<Value> = eintraege
        .into_iter()
        .map(|(fid, b)| {
            let rolle = match b.quelle.bindungspunkt {
                Bindungspunkt::SignaturSlot(_) => "slot",
                Bindungspunkt::Geltungsbedingung(_) => "gate",
            };
            let zustand = felder
                .get(fid)
                .map_or_else(|| json!("offen"), |v| json!(v.zustand));
            json!({
                "feld_id": fid, "regel_id": b.quelle.regel_id, "rolle": rolle,
                "zustand": zustand, "fragetext_laie": b.fragetext_laie,
            })
        })
        .collect();
    Ok(Antwort::neu(
        200,
        json!({
            "fall_id": fall_id.as_str(), "snapshot_id": sid.to_string(),
            "knoten": knoten, "kanten": kanten,
        }),
    ))
}
