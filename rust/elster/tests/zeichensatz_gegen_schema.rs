//! Jedes Textfeld der Bindung gegen den Zeichensatz des amtlichen Schemas (Python: `tests/test_store_zeichensatz.py::
//! test_jedes_textfeld_der_bindung_endet_im_zeichensatz`).
//!
//! **Worum es geht.** `domain::zeichensatz::elster_zeichen` weist beim Speichern jedes Zeichen ab, das ELSTER in einem Textfeld nicht
//! nimmt (`ł`, Gedankenstrich, geschuetztes Leerzeichen). Die Regel haengt am Feldtyp `text` der Bindung, nicht am Kz. Das stimmt nur,
//! solange JEDES `typ: text`-Feld im Schema auf `StringBaseCType` endet (479 Kz tun das) oder ein engeres Bindungsmuster hat. Kommt ein
//! Textfeld dazu, das beides nicht hat, nimmt der Store dort Zeichen an oder weist sie ab, die das Schema anders sieht: der Test wird rot,
//! und jemand entscheidet, ob die Regel fuer dieses Feld gilt.
//!
//! Die MENGE selbst (jedes Zeichen gegen `StringZUBaseCType` und `StringBaseCType`) prueft `eigenschaften.rs::zeichensatz_gleicht_dem_xsd`;
//! hier steht nur die zweite Haelfte des Python-Tests.
//!
//! **Nur lokal**: das Schema liegt nicht in der CI. Ohne Schema rot, ausser `TAXGRAPH_OHNE_XSD=1` (`elster::testhilfe::schemas_da`); die
//! CI setzt die Variable und prueft die Zuordnung damit NICHT.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use bindung::Bindung;
use domain::Feldtyp;
use elster::testhilfe::schemas_da;
use elster::{ernte_est_mapping_kz, finde_schema, kz_meta};
use std::collections::HashMap;
use std::sync::OnceLock;

mod gemeinsam;
use gemeinsam::{typen, Typen};

const VZ: i64 = 2025;

/// Textfelder ohne eigenes `elster_kz`: ihr Ziel steht in der Verzweigung (Gewinnbezeichnung) bzw. im IBAN-Pfad der Stammdaten.
const ZIELE_OHNE_KZ: [(&str, [&str; 2]); 4] = [
    ("gewinn_bezeichnung", ["E0800301", "E0803101"]),
    ("gewinn_bezeichnung_partner", ["E0800301", "E0803101"]),
    ("stammdaten_iban", ["E0102102", "E0102603"]),
    // Rust-eigen (Python kannte das Feld nicht): die Bezeichnung des verkauften Grundstuecks bzw. Wirtschaftsguts (`P23_ARTFELDER`).
    ("p23_bezeichnung", ["E0306111", "E0307101"]),
];

/// Vorsatz-Steuernummer und `IdNr`: ein Ziffernmuster im Schema, kein `StringBaseCType`.
const NUR_ZIFFERN: [&str; 2] = ["stammdaten_steuernummer", "person_b_idnr"];

/// Rust-eigene Ring-Werte: das System setzt eine Marke (`lohn`/`darueber`), der Nutzer tippt nichts, und kein Kz nimmt sie ins XML auf
/// (`elster_kz: null`, `askable: false`). Der Zeichensatz gilt fuer sie nicht, weil nie ein Nutzertext hineinkommt.
const RING_WERT_OHNE_KZ: [&str; 2] = ["versorgung_altersgrenze", "versorgung_altersgrenze_partner"];

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

/// Folgt `base` von `name` aus, bis `ziel` erreicht ist (Vererbungskette des Typs, wie im Python-Original).
fn endet_auf(typen: &Typen, name: &str, ziel: &str) -> bool {
    let (mut name, mut gesehen) = (name.to_owned(), Vec::new());
    while let Some((basis, _)) = typen.get(&name) {
        if name == ziel {
            return true;
        }
        gesehen.push(name.clone());
        match basis {
            Some(b) if !gesehen.contains(b) => name = b.clone(),
            _ => return false,
        }
    }
    name == ziel
}

#[test]
fn jedes_textfeld_der_bindung_endet_im_zeichensatz() {
    if !schemas_da(VZ) {
        return;
    }
    let pfad = finde_schema(VZ, "E10-{jahr}.xsd").unwrap();
    let meta = kz_meta(&pfad, "E10").unwrap();
    let t = typen(VZ, "E10-{jahr}.xsd");
    let auf_string_base = |kz: &str| meta.get(kz).is_some_and(|m| endet_auf(&t, &m.type_name, "StringBaseCType"));
    // Ein Textfeld mit Verzweigung (Kz haengt am Art-Feld) gilt, wenn JEDER Zweig im Zeichensatz endet.
    let ernte = ernte_est_mapping_kz(index()).expect("Tabellen-Kz haben ein Feld in der Bindung");
    let (mut schlecht, mut geprueft) = (Vec::new(), 0_usize);
    for b in bindungen().iter().filter(|b| b.typ == Feldtyp::Text) {
        let fid = b.feld_id.as_str();
        let ok = if let Some(kz) = &b.elster_kz {
            // Ein Kz, das das Schema nicht kennt (Partner-Spiegel, Rechenfeld), hat kein Urteil: das Bindungsmuster entscheidet.
            geprueft += 1;
            auf_string_base(kz.as_str()) || b.muster.is_some() || !meta.contains_key(kz.as_str())
        } else if let Some((_, ziele)) = ZIELE_OHNE_KZ.iter().find(|(f, _)| *f == fid) {
            geprueft += 1;
            ziele.iter().all(|k| auf_string_base(k))
        } else {
            let praefix = format!("verzweigung:{fid}:");
            let zweige: Vec<&str> = ernte.iter().filter(|k| k.feld_id.starts_with(&praefix)).map(|k| k.elster_kz.as_str()).collect();
            if zweige.is_empty() {
                NUR_ZIFFERN.contains(&fid) || RING_WERT_OHNE_KZ.contains(&fid)
            } else {
                geprueft += 1;
                zweige.iter().all(|k| auf_string_base(k))
            }
        };
        if !ok {
            schlecht.push(fid);
        }
    }
    assert!(schlecht.is_empty(), "Textfelder ausserhalb des ELSTER-Zeichensatzes: {schlecht:?}");
    // KONTROLLE: die Pruefung sah viele Felder (gemessen 2026-10-13: 53 Textfelder mit Schema-Urteil).
    println!("zeichensatz_gegen_schema: {geprueft} Textfelder mit Schema-Urteil");
    assert!(geprueft >= 45, "nur {geprueft} Textfelder mit Urteil: die Kontrolle sah zu wenig");
    // Gegenprobe der Kette: ein Typ ausserhalb der Familie (ein Ja-Typ) endet nicht auf StringBaseCType.
    assert!(endet_auf(&t, "StringBaseCType", "StringBaseCType"));
    assert!(!endet_auf(&t, "JaBaseCType", "StringBaseCType"));
}
