//! Die drei strikten JSON-Schemas (`DIALOG_SCHEMA`, `AUSSAGEN_SCHEMA`, `ZUORDNUNG_SCHEMA`,
//! `api_llm.py:239-357, 620-647, 716-743`) — einmal als Wert fuer den Anbieter, einmal als
//! serde-Struktur mit `deny_unknown_fields`, die eine Antwort genau dann annimmt, wenn sie dem
//! Schema entspricht.
// Die Strukturfelder spiegeln das Schema; gelesen wird nur, OB eine Antwort hineinpasst.
#![allow(dead_code)]
use std::sync::LazyLock;

use serde::Deserialize;
use serde_json::Value;

use crate::texte;

fn lade(json: &str) -> Value {
    // Generierter, im Test gepruefter JSON-Text; `Null` waere ein Generat-Fehler, den
    // `schemas_sind_objekte` sofort meldet.
    serde_json::from_str(json).unwrap_or(Value::Null)
}

/// `DIALOG_SCHEMA` (Stufe 3).
pub static DIALOG_SCHEMA: LazyLock<Value> = LazyLock::new(|| lade(texte::DIALOG_SCHEMA_JSON));
/// `AUSSAGEN_SCHEMA` (Stufe 1).
pub static AUSSAGEN_SCHEMA: LazyLock<Value> = LazyLock::new(|| lade(texte::AUSSAGEN_SCHEMA_JSON));
/// `ZUORDNUNG_SCHEMA` (Stufe 2).
pub static ZUORDNUNG_SCHEMA: LazyLock<Value> = LazyLock::new(|| lade(texte::ZUORDNUNG_SCHEMA_JSON));

/// `rechenweg` ist `required`, aber `null` erlaubt: ohne `deserialize_with` wuerde serde ein
/// FEHLENDES `Option`-Feld still als `None` lesen.
fn pflicht<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}

/// `wert`: `["string", "number", "boolean"]`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum WertStreng {
    Text(String),
    Zahl(serde_json::Number),
    Wahr(bool),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RechenwegStreng {
    pub basis: serde_json::Number,
    pub faktor: serde_json::Number,
    pub erklaerung: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VorschlagStreng {
    pub feld_id: String,
    pub wert: WertStreng,
    pub beleg: String,
    pub begruendung: String,
    pub aussage: i64,
    #[serde(deserialize_with = "pflicht")]
    pub rechenweg: Option<RechenwegStreng>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RueckfrageStreng {
    pub frage: String,
    pub feld_id: String,
    pub aussage: i64,
}

/// Stufe-3-Antwort genau nach `DIALOG_SCHEMA`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DialogStreng {
    pub vorschlaege: Vec<VorschlagStreng>,
    pub rueckfragen: Vec<RueckfrageStreng>,
    pub antwort: String,
    pub unsicher: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AussageStreng {
    pub text: String,
    pub beleg: String,
}

/// Stufe-1-Antwort genau nach `AUSSAGEN_SCHEMA`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AussagenStreng {
    pub aussagen: Vec<AussageStreng>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ZuordnungEintragStreng {
    pub aussage: i64,
    pub regeln: Vec<String>,
}

/// Stufe-2-Antwort genau nach `ZUORDNUNG_SCHEMA`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ZuordnungStreng {
    pub zuordnungen: Vec<ZuordnungEintragStreng>,
}

/// Entspricht `j` dem strikten Schema `S`?
pub(crate) fn passt<S: serde::de::DeserializeOwned>(j: &Value) -> bool {
    S::deserialize(j).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{passt, DialogStreng, AUSSAGEN_SCHEMA, DIALOG_SCHEMA, ZUORDNUNG_SCHEMA};
    use serde_json::json;

    #[test]
    fn schemas_sind_objekte() {
        for s in [&*DIALOG_SCHEMA, &*AUSSAGEN_SCHEMA, &*ZUORDNUNG_SCHEMA] {
            assert_eq!(s["strict"], json!(true));
            assert_eq!(s["schema"]["additionalProperties"], json!(false));
        }
    }

    #[test]
    fn rechenweg_muss_da_sein() {
        let ohne = json!({"vorschlaege": [{"feld_id": "a", "wert": 1, "beleg": "b", "begruendung": "c",
            "aussage": 0}], "rueckfragen": [], "antwort": "", "unsicher": false});
        assert!(!passt::<DialogStreng>(&ohne));
        let mit = json!({"vorschlaege": [{"feld_id": "a", "wert": 1, "beleg": "b", "begruendung": "c",
            "aussage": 0, "rechenweg": null}], "rueckfragen": [], "antwort": "", "unsicher": false});
        assert!(passt::<DialogStreng>(&mit));
    }
}
