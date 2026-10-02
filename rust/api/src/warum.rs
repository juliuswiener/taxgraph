//! `api.warum` (`api.py:548`): wie ein Feld zu seinem Wert kam — das Justification-Objekt aus
//! Event und Bindung (`traverser.justification`, `traverser.py:738`).
use domain::FallId;
use serde_json::json;
use store::Store;

use crate::antwort::Antwort;
use crate::fehler::ApiFehler;
use crate::python::repr;
use crate::zustand::Zustand;

/// `api.warum(fall_id, feld_id)` nach dem Owner-Check: erst Scheibe und Bindung (400/500 wie in
/// Python), dann das Feld; ohne aktives Event 404.
///
/// # Errors
/// 400/500 aus Scheibe und Bindung; 404 ohne aktives Event des Felds.
pub fn warum(
    z: &Zustand,
    fall_id: &FallId,
    store: &Store,
    feld_id: &str,
) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(store)?;
    let Some(j) = interview::justification(store, feld_id, &sb.sicht) else {
        return Err(ApiFehler::status(
            404,
            format!("Feld {} hat (noch) kein Event", repr(&json!(feld_id))),
        ));
    };
    let j =
        serde_json::to_value(&j).map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    Ok(Antwort::neu(
        200,
        json!({"fall_id": fall_id.as_str(), "justification": j}),
    ))
}
