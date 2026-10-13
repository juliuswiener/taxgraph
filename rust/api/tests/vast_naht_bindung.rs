//! Die Naht VaSt-Beleg -> Bindung -> Store (Python: `tests/test_vast_mapping.py`, die Naht-Tests `test_alle_ziel_felder_existieren_in_der_bindung`,
//! `test_alle_ziel_felder_sind_cent`, `test_summen_ziele_existieren_und_sind_cent`, `test_mapping_ausgabe_passt_in_den_writer`,
//! `test_gemappte_vast_elemente_existieren_im_schema`, `test_summen_quellen_existieren_im_schema`).
//!
//! **Worum es geht.** Das Mapping `eingang::vast::{LSTB, LSTB_SUMMEN, LERSL_BETRAG_FELD}` schickt Beträge der Lohnsteuerbescheinigung an
//! Felder der Bindung. Ein Ziel, das es dort nicht gibt, schreibt ins Leere: der Store lehnt den Satz ab, der Nutzer sieht nichts. Ein
//! Ziel, das kein Cent-Feld ist, mischt Euro und Cent. Beide Seiten stimmen fuer sich, das Format dazwischen prueft sonst niemand.
//! `rust/eingang/tests/orakel_werte.rs` gleicht die Werte gegen eine eingefrorene Python-Bindung ab (`fixtures/python_bindung_2dd056a6`),
//! nicht gegen die Bindung des Dienstes: ein Ziel, das in der heutigen Bindung fehlt, faellt dort nicht auf.
//!
//! Zwei Teile:
//!
//! - **Ohne Schema** (laeuft in der CI): jedes Ziel steht in der Bindung und ist `cent`; jedes Element des Mappings durchlaeuft
//!   `aus_lstb` und `edaten::uebernehme` gegen die echte Bindung und kommt als bestaetigter Cent-Wert im Store an.
//! - **Mit Schema** (nur lokal): jeder VaSt-Elementname des Mappings steht als `name="..."` in `VaSt_LStB-202501.xsd`. Ohne Schema rot,
//!   ausser `TAXGRAPH_OHNE_XSD=1` (`elster::testhilfe::schemas_da`); die CI setzt die Variable und prueft die Namen damit NICHT.
//!
//! Grenze: Python hatte eine Liste `NICHT_GEMAPPT` (Beleg-Elemente ohne Ziel, jedes mit Begruendung). Rust fuehrt sie nicht; ein
//! Beleg-Element ohne Eintrag in `LSTB` faellt ohne Meldung weg. Benannter Verlust, siehe `REWRITE_PLAN` Nr. 69.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Feldtyp, PyWert, Zustand};
use eingang::edaten::{uebernehme, Satz};
use eingang::vast::{aus_lstb, LERSL_BETRAG_FELD, LSTB, LSTB_SUMMEN};
use elster::testhilfe::schemas_da;
use store::{BindungNachschlag, Store};

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
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

/// Alle Zielfelder des Mappings: die 1:1-Felder, die Summenfelder und das Feld der Lohnersatzleistungen.
fn ziele() -> BTreeSet<&'static str> {
    LSTB.iter()
        .map(|(_, feld, _)| *feld)
        .chain(LSTB_SUMMEN.iter().map(|(feld, _)| *feld))
        .chain([LERSL_BETRAG_FELD])
        .collect()
}

/// Jeder Elementname, den das Mapping liest.
fn vast_elemente() -> BTreeSet<&'static str> {
    LSTB.iter()
        .map(|(vast, _, _)| *vast)
        .chain(LSTB_SUMMEN.iter().flat_map(|(_, quellen)| quellen.iter().map(|(n, _)| *n)))
        .collect()
}

#[test]
fn jedes_ziel_steht_in_der_bindung_und_ist_ein_cent_feld() {
    let z = ziele();
    // KONTROLLE: die Schleife unten sieht Ziele (gemessen 2026-10-13: 9 verschiedene Felder aus 8 + 1 + 1 Eintraegen).
    assert!(z.len() >= 9, "nur {} Ziele gesehen: {z:?}", z.len());
    let fehlend: Vec<&str> = z.iter().copied().filter(|f| !index().contains_key(*f)).collect();
    assert!(fehlend.is_empty(), "Mapping zeigt auf Felder, die es in der Bindung nicht gibt: {fehlend:?}");
    let falsch: Vec<(&str, Feldtyp)> =
        z.iter().copied().filter_map(|f| Some((f, index()[f].typ)).filter(|(_, t)| *t != Feldtyp::Cent)).collect();
    assert!(falsch.is_empty(), "Ziele ohne typ=cent (VaSt liefert Euro, der Store rechnet in Cent): {falsch:?}");
}

/// Jedes Element des Mappings mit einem Betrag belegt, durch `aus_lstb` und `uebernehme` gegen die echte Bindung: jedes Ziel kommt
/// als bestaetigter Wert an (§ 150 Abs. 7 S. 2 AO), und der Wert ist der Betrag in Cent.
#[test]
fn jedes_element_des_mappings_kommt_als_bestaetigter_cent_wert_im_store_an() {
    let werte: BTreeMap<String, String> = vast_elemente().into_iter().map(|e| (e.to_owned(), "1234.56".to_owned())).collect();
    let saetze: Vec<Satz> = aus_lstb(&werte)
        .unwrap()
        .into_iter()
        .map(|s| Satz { feld_id: s.feld_id, wert: s.wert.into(), kategorie: s.kategorie.into() })
        .collect();
    // `LERSL_BETRAG_FELD` ist zugleich das Ziel von `LeistungenProgVorbeh`, also liefert `aus_lstb` einen Satz je Ziel.
    let erwartet = ziele();
    let gesehen: BTreeSet<&str> = saetze.iter().map(|s| s.feld_id.as_str()).collect();
    assert_eq!(gesehen, erwartet, "aus_lstb liefert nicht fuer jedes Ziel einen Satz");

    let mut store = Store::leer(2025, None);
    let n = uebernehme(&mut store, &saetze, Some("2026-01-01T00:00:00Z"), Some(BindungNachschlag::neu(index()))).unwrap();
    assert_eq!(n, saetze.len(), "die Bindung hat Saetze abgewiesen oder uebersprungen");
    let (felder, _) = store.materialisiere(None).unwrap();
    for feld in &erwartet {
        let f = &felder[*feld];
        assert_eq!(f.zustand, Zustand::Bestaetigt, "{feld}");
        assert_eq!(f.wert, PyWert::Ganz(123_456), "{feld}: 1234,56 EUR sind 123456 Cent");
    }
}

#[test]
fn die_vast_elemente_des_mappings_stehen_im_lstb_schema() {
    if !schemas_da(2025) {
        return;
    }
    let pfad = elster::finde_schema(2025, "VaSt_LStB-{jahr}01.xsd").expect("VaSt_LStB-202501.xsd liegt neben den ELSTER-Schemas");
    let xsd = std::fs::read_to_string(&pfad).unwrap();
    let im_schema: BTreeSet<&str> = xsd.split("name=\"").skip(1).filter_map(|s| s.split('"').next()).collect();
    let fehlend: Vec<&str> = vast_elemente().into_iter().filter(|n| !im_schema.contains(n)).collect();
    assert!(fehlend.is_empty(), "Nicht im LStB-Schema 2025: {fehlend:?}");
    // KONTROLLE: die Suche findet Namen (sonst bewiese ein Null-Ergebnis nichts) und weist einen erfundenen ab.
    assert!(im_schema.len() > 50, "nur {} Namen im Schema gelesen", im_schema.len());
    assert!(!im_schema.contains("ArbnKiSteuerX"));
}
