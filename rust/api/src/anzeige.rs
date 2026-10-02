//! Die Anzeige-Metadaten eines Felds, die `GET /stand` und der Chat an jedes Feld haengen
//! (`api._anzeige_metadaten`, `api._badge`, `api.py:307`, `:1054`).
use bescheid::BindungIndex;
use serde_json::{json, Map, Value};

use crate::enum_labels::ENUM_LABELS;

/// `ENUM_LABELS.get(fid)` als JSON-Objekt (`wert -> Anzeigetext`), sonst `null`.
#[must_use]
pub(crate) fn enum_labels(fid: &str) -> Value {
    ENUM_LABELS
        .iter()
        .find(|(f, _)| *f == fid)
        .map_or(Value::Null, |(_, labels)| {
            let m: Map<String, Value> = labels
                .iter()
                .map(|(w, t)| ((*w).to_owned(), json!(t)))
                .collect();
            Value::Object(m)
        })
}

/// `_anzeige_metadaten(fid, bindung)`: `frage`, `typ`, `frage_invertiert`, `einheit`, `enum_labels`.
///
/// Fehlt das Feld in der Bindung (nach einem Scheiben-Wechsel moeglich), bleiben die Werte `null`
/// und `frage_invertiert` ist `false`; `enum_labels` haengt nur am Feldnamen und bleibt.
pub(crate) fn anzeige_metadaten(
    fid: &str,
    index: &BindungIndex<'_>,
    ziel: &mut Map<String, Value>,
) {
    let b = index.get(fid);
    ziel.insert(
        "frage".into(),
        json!(b.and_then(|b| b.fragetext_laie.as_deref())),
    );
    ziel.insert("typ".into(), json!(b.map(|b| b.typ.als_str())));
    ziel.insert(
        "frage_invertiert".into(),
        json!(b.is_some_and(|b| b.frage_invertiert)),
    );
    ziel.insert(
        "einheit".into(),
        json!(b.and_then(|b| b.einheit.as_deref())),
    );
    ziel.insert("enum_labels".into(), enum_labels(fid));
}

/// `_badge(herkunft)`: `herkunft.get("herkunft", "laie")`.
#[must_use]
pub(crate) fn badge(herkunft: &Value) -> Value {
    herkunft
        .get("herkunft")
        .cloned()
        .unwrap_or_else(|| json!("laie"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ein Feld ausserhalb der Bindung: alles `null`, `frage_invertiert` falsch, die Labels bleiben.
    #[test]
    fn feld_ohne_bindung_hat_nur_labels() {
        let leer = BindungIndex::new();
        let mut m = Map::new();
        anzeige_metadaten("dba_methode", &leer, &mut m);
        assert_eq!(m["frage"], Value::Null);
        assert_eq!(m["typ"], Value::Null);
        assert_eq!(m["frage_invertiert"], json!(false));
        assert_eq!(m["einheit"], Value::Null);
        assert_eq!(
            m["enum_labels"]["kein_dba"],
            "Kein Doppelbesteuerungsabkommen mit diesem Staat"
        );
        let mut n = Map::new();
        anzeige_metadaten("bruttoarbeitslohn", &leer, &mut n);
        assert_eq!(n["enum_labels"], Value::Null);
    }

    /// `_badge`: ohne Schluessel `laie`, sonst der Wert des Schluessels (auch ein `null`).
    #[test]
    fn badge_vorgabe_laie() {
        assert_eq!(badge(&json!({})), "laie");
        assert_eq!(badge(&json!({"herkunft": "beleg_import"})), "beleg_import");
        assert_eq!(badge(&json!({"herkunft": null})), Value::Null);
    }
}
