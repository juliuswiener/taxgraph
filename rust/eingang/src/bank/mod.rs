//! Bankimport: die Bausteine, die eine Buchung zu einem Vorschlag machen.
//!
//! [`buchung`] liest das Datum einer Buchung und beschreibt ihren Posten, [`saldo`] vereinigt
//! Posten aus zwei Auszuegen und bildet den Saldo eines Felds, [`regeln`] haelt den Regelkatalog
//! (`daten/bankregeln.yaml`). Die Uebernahme selbst bleibt in [`crate::kontoauszug`].
pub mod buchung;
pub mod regeln;
pub mod saldo;

/// Waechter ueber die Regeldatei `daten/bankregeln.yaml` (Bankimport S2, AK3): jede Regel hat eine Kennung, ein Feld, Stichwoerter
/// und eine Fundstelle in `sources/` oder `params/`, die es gibt, oder ein `ponytail:`. Gelesen wird die Datei selbst, nicht der
/// Lader: ein Lader, der eine Regel ohne Fundstelle durchliesse, bliebe sonst unbemerkt.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use std::path::Path;

    use serde_yaml_ng::Value;

    fn regeln() -> Vec<Value> {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("daten/bankregeln.yaml");
        let text = std::fs::read_to_string(&pfad).unwrap_or_else(|e| panic!("{}: {e}", pfad.display()));
        serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", pfad.display()))
    }

    fn text<'a>(regel: &'a Value, schluessel: &str) -> Option<&'a str> {
        regel.get(schluessel)?.as_str().filter(|t| !t.trim().is_empty())
    }

    #[test]
    fn bankregeln_haben_kennung_feld_und_stichworte() {
        let regeln = regeln();
        // Die fuenf Kategorien der Schluesselwortlisten von vor S2 sind alle da; eine leere Datei waere sonst gruen.
        assert!(regeln.len() >= 5, "nur {} Regeln gelesen", regeln.len());
        let mut kennungen = Vec::new();
        for r in &regeln {
            let id = text(r, "id").unwrap_or_else(|| panic!("Regel ohne id: {r:?}"));
            assert!(text(r, "feld").is_some(), "{id}: kein feld");
            let stichworte = r.get("stichworte").and_then(Value::as_sequence).unwrap_or_else(|| panic!("{id}: keine stichworte"));
            assert!(!stichworte.is_empty(), "{id}: leere stichworte");
            for s in stichworte {
                let s = s.as_str().unwrap_or_else(|| panic!("{id}: Stichwort ist kein Text: {s:?}"));
                assert!(!s.is_empty() && s == s.to_lowercase(), "{id}: Stichwort {s:?} ist leer oder nicht kleingeschrieben");
            }
            assert!(!kennungen.contains(&id), "{id}: Kennung doppelt");
            kennungen.push(id);
        }
    }

    #[test]
    fn bankregeln_jede_regel_hat_eine_vorhandene_quelle_oder_ponytail() {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut ohne = Vec::new();
        for r in regeln() {
            let id = text(&r, "id").unwrap_or("?").to_owned();
            match (text(&r, "quelle"), text(&r, "ponytail")) {
                (Some(q), _) => {
                    let ok = (q.starts_with("sources/") || q.starts_with("params/")) && wurzel.join(q).is_file();
                    if !ok {
                        ohne.push(format!("{id}: quelle {q:?} ist keine Datei unter sources/ oder params/"));
                    }
                }
                (None, Some(_)) => {}
                (None, None) => ohne.push(format!("{id}: weder quelle noch ponytail")),
            }
        }
        assert!(ohne.is_empty(), "{ohne:#?}");
    }
}
