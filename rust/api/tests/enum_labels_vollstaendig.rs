//! Kein Auswahlwert erreicht den Nutzer als Rohwert: Jeder `enum_wert` jeder askable Enum-Bindung
//! traegt in `api::enum_labels::ENUM_LABELS` einen Anzeigetext, und der Text ist nicht der Rohwert.
//!
//! Warum ein Test und kein Typ: `ENUM_LABELS` ist eine String-Tabelle, die Werte kommen zur
//! Laufzeit aus der Bindungs-YAML. Der Compiler sieht beide nicht zusammen. Ein neuer `enum_wert`
//! ohne Label zeigt dem Nutzer `land_forst` statt „Land- und Forstwirtschaft", ohne dass etwas
//! scheitert. Ersetzt `tests/test_enum_labels.py` (Python-Test, entfaellt mit Python).
//!
//! Die Registry kommt aus `bindung::lade_registry_der_wurzel`: Felder in `produkt/bindung/` UND in
//! `rust/bindung/felder/` (Weg B leicht) brauchen ein Label.
//!
//! Die Auslieferung der Labels an die Oberflaeche (`/fragen`, `/chat`) belegen `lesen_naht.rs` und
//! `chat_llm_attrappe_hermetisch.rs`; hier steht nur die Tabelle.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use api::enum_labels::ENUM_LABELS;
use domain::Feldtyp;

/// Werte, die von Haus aus lesbar sind: Laendernamen stehen schon richtig in der Bindung
/// (`dba_staat`), das Label darf sie wiederholen.
const LESBAR_ROH: [&str; 13] = [
    "Deutschland",
    "Frankreich",
    "Italien",
    "Schweiz",
    "Niederlande",
    "Polen",
    "Tschechien",
    "Dänemark",
    "Luxemburg",
    "Türkei",
    "Spanien",
    "USA",
    "Kanada",
];

/// `produkt/bindung/` und `rust/bindung/felder/` liegen unter der Wurzel, zwei Ebenen ueber dem Crate.
fn wurzel() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `feld_id -> enum_werte` jeder askable Enum-Bindung mit Werten, ueber alle Bindungsdateien beider
/// Verzeichnisse (so laedt der Dienst sie: `bindung::lade_registry_der_wurzel`).
fn askable_enums() -> BTreeMap<String, Vec<String>> {
    let reg = bindung::lade_registry_der_wurzel(&wurzel()).expect("Registry");
    reg.dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .filter(|b| b.askable && b.typ == Feldtyp::Enum)
        .filter_map(|b| {
            b.enum_werte
                .as_ref()
                .filter(|w| !w.is_empty())
                .map(|w| (b.feld_id.clone(), w.clone()))
        })
        .collect()
}

fn label(feld: &str, wert: &str) -> Option<&'static str> {
    ENUM_LABELS
        .iter()
        .find(|(f, _)| *f == feld)
        .and_then(|(_, ws)| ws.iter().find(|(w, _)| *w == wert))
        .map(|(_, t)| *t)
}

/// Der Test misst nur etwas, wenn die Registry Enum-Felder liefert: eine leere Menge wuerde
/// jede Pruefung darunter gruen lassen (Pfad falsch, Filter falsch).
#[test]
fn die_registry_liefert_askable_enum_felder() {
    let felder = askable_enums();
    assert!(
        felder.len() >= 20,
        "nur {} askable Enum-Felder in der Registry; zu wenig, der Test wuerde nichts messen",
        felder.len()
    );
}

/// Jedes askable Enum-Feld hat eine Labeltabelle, und jeder seiner Werte hat darin einen Text.
#[test]
fn jeder_enum_wert_hat_einen_anzeigetext() {
    let mut fehlen = Vec::new();
    for (feld, werte) in askable_enums() {
        if !ENUM_LABELS.iter().any(|(f, _)| *f == feld) {
            fehlen.push(format!("{feld}: ganze Tabelle fehlt"));
            continue;
        }
        for w in werte {
            if label(&feld, &w).is_none() {
                fehlen.push(format!("{feld}:{w}"));
            }
        }
    }
    assert!(
        fehlen.is_empty(),
        "{} Enum-Werte ohne Anzeigetext, der Nutzer saehe den Rohwert: {fehlen:?}",
        fehlen.len()
    );
}

/// Gegen die billige Erfuellung des Tests oben: ein Label, das den Rohwert nur kopiert (oder nur
/// Unterstriche durch Leerzeichen ersetzt), ist keins.
#[test]
fn kein_label_wiederholt_nur_den_rohwert() {
    let mut schlecht = Vec::new();
    for (feld, werte) in askable_enums() {
        for w in werte {
            let Some(t) = label(&feld, &w) else { continue };
            if LESBAR_ROH.contains(&w.as_str()) {
                continue;
            }
            if t == w || (t.contains('_') && t.replace('_', " ") == w.replace('_', " ")) {
                schlecht.push(format!("{feld}:{w}"));
            }
        }
    }
    assert!(
        schlecht.is_empty(),
        "Labels, die nur den Rohwert wiederholen: {schlecht:?}"
    );
}
