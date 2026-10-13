//! Lesen der benannten Typen des amtlichen Schemas, fuer die Testdateien `nicht_negativ_gegen_schema.rs` und `zeichensatz_gegen_schema.rs`
//! (`mod gemeinsam;`). Das Schema liegt nur lokal; wer diese Funktionen ruft, steht hinter `elster::testhilfe::schemas_da`.
#![allow(dead_code, clippy::unwrap_used, clippy::panic)]

use std::collections::HashMap;

use elster::finde_schema;
use regex::Regex;

/// Je benanntem `complexType` des Schemas: der `base` seines `simpleContent` und die `pattern`-Werte seines Schritts (`restriction`/`extension`).
pub type Typen = HashMap<String, (Option<String>, Vec<String>)>;

pub fn typen(jahr: i64, muster: &str) -> Typen {
    let pfad = finde_schema(jahr, muster).unwrap_or_else(|| panic!("{muster} fehlt"));
    let text = std::fs::read_to_string(&pfad).unwrap();
    let doc = roxmltree::Document::parse(&text).unwrap();
    let mut m = Typen::new();
    for ct in doc.descendants().filter(|n| n.tag_name().name() == "complexType") {
        let Some(name) = ct.attribute("name") else {
            continue;
        };
        for sc in ct.children().filter(|c| c.tag_name().name() == "simpleContent") {
            for schritt in sc.children().filter(|c| matches!(c.tag_name().name(), "restriction" | "extension")) {
                let muster: Vec<String> = schritt
                    .children()
                    .filter(|e| e.tag_name().name() == "pattern")
                    .filter_map(|e| e.attribute("value").map(str::to_owned))
                    .collect();
                m.insert(name.to_owned(), (schritt.attribute("base").map(str::to_owned), muster));
            }
        }
    }
    m
}

/// Passt das ganze Muster auf `wert`? (XSD-Muster sind verankert.)
pub fn voll(muster: &str, wert: &str) -> bool {
    Regex::new(&format!("^(?:{muster})$")).unwrap_or_else(|e| panic!("Muster {muster:?}: {e}")).is_match(wert)
}
