//! Der Beginn der Rente geht als Datum TT.MM.JJJJ in die Erklaerung, nicht als Jahr (Python: `tests/test_rentenbeginn_kein_datumsformat.py`).
//!
//! **Worum es geht.** Der Nutzer beantwortet das JAHR des Rentenbeginns (`typ: int`). Das Schema verlangt fuer E1800501, E1801701 und
//! E1803202 ein Datum (`DatumTTpMMpJJJJ...`); ERiC lehnt das nackte Jahr ab (`Bitte geben Sie ein gueltiges Datum TT.MM.JJJJ ein`,
//! rc=610001002, gemessen 2026-09-26 an allen drei Schreibwegen). `kz_format::kz_wert` macht aus 2015 das "01.01.2015".
//!
//! **Ohne Schema.** `eigenschaften.rs::rente_*` prueft das Datum im fertigen XML, aber nur hinter `schemas_da(2025)`: die CI setzt
//! `TAXGRAPH_OHNE_XSD=1` und liest diese Tests nicht. Diese Datei liest die Deklaration (Kz -> Wert) und laeuft ueberall. Sie deckt
//! alle fuenf Rentenarten ab und die DREI Schreibwege: Person A Instanz 1 (`deklaration`), Instanz 2 (`anlage_instanzen`) und den
//! Partner (`person_b`). Jeder Weg hat ein eigenes Jahr, damit eine Verwechslung auffaellt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Herkunft, PruefTiefe, Zustand};
use elster::{deklariere, Deklaration, Felder, DATUMS_KZ};
use serde_json::{json, Value};
use store::SnapshotFeld;

/// (Rentenart, Kz des Rentenbeginns): die Zuordnung der Art zum Beginn-Slot im Schema (`Leibr_gesetzl`, `Leibr_priv`, `Leibr_sonst`).
const ARTEN: [(&str, &str); 5] = [
    ("gesetzliche_rente", "E1800501"),
    ("berufsstaendische_versorgung", "E1800501"),
    ("private_basisrente", "E1800501"),
    ("private_leibrente", "E1801701"),
    ("sonstige_leibrente", "E1803202"),
];

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

fn lauf(paare: &[(&str, Value)]) -> Deklaration {
    let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
    let felder: Felder = paare
        .iter()
        .map(|(id, w)| {
            let feld = SnapshotFeld {
                wert: w.clone().into(),
                zustand: Zustand::Bestaetigt,
                herkunft: Herkunft {
                    herkunft: a("laie"),
                    pruef_tiefe: PruefTiefe::Ungeprueft,
                    haftung: a("nutzer"),
                }
                .into(),
            };
            ((*id).to_owned(), feld)
        })
        .collect();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    assert!(d.eingaben_konsistent(), "{:?}", d.unvollstaendig());
    d
}

/// Der Rentenbeginn der ersten Instanz von Person A (Basis-Feld ohne Suffix, landet in `deklaration`).
fn beginn_a1(art: &str, kz: &str) -> Option<Value> {
    let d = lauf(&[
        ("veranlagung", json!("einzel")),
        ("rentner_renten_art", json!(art)),
        ("rentner_jahresrente", json!(1_800_000)),
        ("rentner_renten_beginn_jahr", json!(2015)),
    ]);
    d.deklaration.get(kz).cloned()
}

/// Der Rentenbeginn der zweiten Instanz von Person A (`__2`, landet in `anlage_instanzen`).
fn beginn_a2(art: &str, kz: &str) -> Option<Value> {
    let d = lauf(&[
        ("veranlagung", json!("einzel")),
        ("rentner_renten_art__2", json!(art)),
        ("rentner_jahresrente__2", json!(900_000)),
        ("rentner_renten_beginn_jahr__2", json!(2011)),
        ("rentner_alter_bei_rentenbeginn__2", json!(65)),
    ]);
    let felder: BTreeMap<&String, &Value> = d
        .anlage_instanzen
        .iter()
        .flat_map(|(_, ii)| ii)
        .flat_map(|i| &i.felder)
        .collect();
    felder.get(&kz.to_owned()).map(|v| (*v).clone())
}

/// Der Rentenbeginn des Partners (Zusammenveranlagung, landet in `person_b`).
fn beginn_partner(art: &str, kz: &str) -> Option<Value> {
    let d = lauf(&[
        ("veranlagung", json!("zusammen")),
        ("kein_sonstige_partner", json!(false)),
        ("rentner_renten_art_partner", json!(art)),
        ("rentner_jahresrente_partner", json!(700_000)),
        ("rentner_renten_beginn_jahr_partner", json!(2012)),
    ]);
    d.person_b.get(kz).cloned()
}

#[test]
fn der_rentenbeginn_steht_als_datum_mit_dem_jahr_des_nutzers() {
    for (art, kz) in ARTEN {
        assert_eq!(beginn_a1(art, kz), Some(json!("01.01.2015")), "Person A, Instanz 1, {art} ({kz})");
        assert_eq!(beginn_a2(art, kz), Some(json!("01.01.2011")), "Person A, Instanz 2, {art} ({kz})");
        assert_eq!(beginn_partner(art, kz), Some(json!("01.01.2012")), "Partner, {art} ({kz})");
    }
}

/// Jedes Beginn-Kz der Arten steht in `DATUMS_KZ`, und `DATUMS_KZ` hat kein weiteres: faellt ein Zweig heraus, bekaeme er das rohe Jahr.
#[test]
fn datums_kz_sind_genau_die_beginn_kz_der_rentenarten() {
    let mut aus_arten: Vec<&str> = ARTEN.iter().map(|(_, kz)| *kz).collect();
    aus_arten.sort_unstable();
    aus_arten.dedup();
    let mut datums: Vec<&str> = DATUMS_KZ.to_vec();
    datums.sort_unstable();
    assert_eq!(datums, aus_arten);
}

/// Kontrolle: derselbe Schreibweg wandelt den Jahresbetrag der Rente als Betrag, nicht als Datum. Ein Test, der jeden Wert der
/// Rentenzeile als Datum zoege, bliebe sonst gruen.
#[test]
fn der_jahresbetrag_neben_dem_beginn_bleibt_ein_betrag() {
    let d = lauf(&[
        ("veranlagung", json!("einzel")),
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(1_800_000)),
        ("rentner_renten_beginn_jahr", json!(2015)),
    ]);
    assert_eq!(d.deklaration.get("E1800301"), Some(&json!(18_000)));
}
