//! Kz-Sperre (Weg B voll, Stufe 3, Waechter W10): was ein Kz im ELSTER-XML am Ende wirklich
//! schreibt, und welche Zuordnung `feld_id` → Kz die Bindung traegt.
//!
//! - **Ja-Typen** (`ja_typ_nein_wird_nach_typ_geschrieben`): ein bool-Kz mit `false` wird bei
//!   `JaNein12` als `2` geschrieben (eine echte Antwort), bei einem Ankreuzfeld (`Ja1`/`JaX`/`Ja2`)
//!   weggelassen. Vorbild: `tests/test_kz_bindung_durchgang.py::test_janein12_nein_wird_geschrieben_
//!   nicht_weggelassen`. Messung 2026-10-05: ein Mutant, der den `JaNein12`-Zweig in
//!   `xml.rs::blatt_text` auf `None` setzt, liess alle 2051 Tests gruen. Das Schema liegt nur lokal;
//!   die CI sieht diese Pruefung ueber den hermetischen Gegenstueck-Test in `src/xml.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashMap;
use std::sync::OnceLock;

use bindung::Bindung;
use domain::Feldtyp;
use elster::testhilfe::schemas_da;
use elster::{deklariere, erzeuge_xml, finde_schema, kz_meta, Felder, KzMeta, XmlOptionen};
use serde_json::json;

/// Das Jahr, gegen dessen Schema geprueft wird (wie `bindungs_typ_vs_xsd_typ.rs`).
const VZ: i64 = 2025;

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        // Die Bindung des Dienstes (`rust/bindung/daten`), nie ein fester Pfad im Test.
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel)
            .expect("Bindung laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

/// Die Typ-Angaben der Kz aus E10 des Jahres [`VZ`].
fn e10_meta() -> HashMap<String, KzMeta> {
    let pfad = finde_schema(VZ, "E10-{jahr}.xsd").expect("E10-Schema fehlt");
    kz_meta(&pfad, "E10").unwrap()
}

/// Das XML einer Deklaration, die genau dieses eine Kz mit `false` traegt.
fn xml_mit_nein(kz: &str) -> String {
    let mut d = deklariere(&Felder::new(), index(), 2025, None).unwrap();
    d.deklaration.insert(kz.to_owned(), json!(false));
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        ..XmlOptionen::default()
    };
    erzeuge_xml(&d, &opt).unwrap_or_else(|e| panic!("{kz}: erzeuge_xml scheitert: {e}"))
}

/// Jede bool-Bindung mit einem Ja-Typ im Schema: `false` heisst bei `JaNein12` `<Kz>2</Kz>`, bei
/// `Ja1`/`JaX`/`Ja2` kein Element. Schleife ueber die echte Bindung, damit ein neues Ja-Kz ohne
/// Zutun mitgeprueft wird; beide Familien muessen vorkommen, sonst prueft die Schleife nichts.
#[test]
fn ja_typ_nein_wird_nach_typ_geschrieben() {
    if !schemas_da(VZ) {
        return;
    }
    let meta = e10_meta();
    let (mut janein12, mut ankreuz) = (Vec::new(), Vec::new());
    for b in bindungen().iter().filter(|b| b.typ == Feldtyp::Bool) {
        let Some(kz) = b.elster_kz.as_ref().map(domain::Kz::as_str) else {
            continue;
        };
        let Some(m) = meta.get(kz).filter(|m| m.is_ja) else {
            continue;
        };
        let xml = xml_mit_nein(kz);
        if m.type_name.starts_with("JaNein12") {
            assert!(
                xml.contains(&format!("<{kz}>2</{kz}>")),
                "{} ({kz}, {}): JaNein12-Nein fehlt im XML — die gegebene Antwort ist verschwunden",
                b.feld_id,
                m.type_name
            );
            janein12.push(kz);
        } else {
            assert!(
                !xml.contains(&format!("<{kz}")),
                "{} ({kz}, {}): ein Ankreuzfeld mit Nein darf kein Element ergeben",
                b.feld_id,
                m.type_name
            );
            ankreuz.push(kz);
        }
    }
    assert!(
        !janein12.is_empty() && !ankreuz.is_empty(),
        "die Schleife sah JaNein12 {janein12:?} und Ankreuzfelder {ankreuz:?}; beide Familien muessen vorkommen"
    );
}
