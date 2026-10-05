//! Instanzen der Deklaration (`deklariere`) im Standardlauf (ohne `PARITY=1`, ohne Python, ohne ERiC-Schema): der
//! § 23-Gewinn je Instanz und die Aggregat-Summe je Anlage-V-Objekt.
//!
//! WARUM ES DIESE DATEI GIBT. Die Mutationsmessung `bescheid-elster-mutation` (2026-10-04) liess zwei Mutanten in
//! `deklaration.rs` in allen Standard-Tests gruen; nur `PARITY=1` (`elster_paritaet`, Zufallsfaelle) faengt sie:
//! - D05 (`p23_gewinn`): `if gewinn == 0 { continue }` -> `<= 0`. Der Mutant ueberspringt jeden VERLUST, der Kz des
//!   Verlusts fehlt. Der Guard sperrt jede § 23-Eingabe vor `deklariere` mit 409, darum erreicht kein HTTP-Fall die Stelle.
//! - D07 (Instanz-Zweig von `instanz_wert`): `checked_add` -> `checked_sub`. Die Summe der Werbungskosten eines weiteren
//!   Vermietungsobjekts (E0703838) wird negativ. `dokumentiert` geht nicht ins XML, sondern in die Rueckgabe.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Ausgabe des Python-Orakels `elster_oracle` (`elster.fall`, `deklariere` mit
//! `est_mapping.deklariere`), gemessen am 2026-10-04 mit `tools/parity/bescheid_erwartung.py` auf denselben Ereignissen.
//! Kein Wert ist aus dem Rust-Code abgelesen.
//!
//! Zahlen: die Felder sind Cent, die Kz der Deklaration EURO.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashMap;
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Herkunft, PruefTiefe, Zustand};
use elster::{deklariere, Deklaration, Felder};
use serde_json::{json, Value};
use store::SnapshotFeld;

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&pfad)
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

/// Alle Felder bestaetigt, vom Laien geschrieben.
fn felder(paare: &[(&str, Value)]) -> Felder {
    let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
    let herkunft = Herkunft {
        herkunft: a("laie"),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: a("nutzer"),
    };
    paare
        .iter()
        .map(|(id, wert)| {
            (
                (*id).to_owned(),
                SnapshotFeld {
                    wert: wert.clone().into(),
                    zustand: Zustand::Bestaetigt,
                    herkunft: herkunft.clone().into(),
                },
            )
        })
        .collect()
}

/// Die Instanzen einer Gruppe als `(index, Kz -> Wert)`.
fn instanzen(d: &Deklaration, gruppe: &str) -> Vec<(u64, Vec<(String, Value)>)> {
    d.anlage_instanzen
        .iter()
        .filter(|(g, _)| g == gruppe)
        .flat_map(|(_, ii)| ii)
        .map(|i| {
            (
                i.index,
                i.felder
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            )
        })
        .collect()
}

/// § 23 Abs. 3 `EStG`: der Gewinn je Instanz ist Preis - Anschaffungskosten - Werbungskosten, auch wenn er NEGATIV ist
/// (Verlust, E0306801); nur ein Gewinn von genau 0 erzeugt keinen Kz. Python: Instanz 1 Grundstueck -5.500 EUR,
/// Instanz 2 anderes Wirtschaftsgut +4.500 EUR, Instanz 3 (Gewinn 0) fehlt.
#[test]
fn p23_verlust_bekommt_seinen_kz_und_gewinn_null_keinen() {
    let f = felder(&[
        ("p23_veraeusserungspreis", json!(1_000_000)),
        ("p23_anschaffung_herstellungskosten", json!(1_500_000)),
        ("p23_werbungskosten", json!(50_000)),
        ("p23_veraeusserungs_typ", json!("grundstueck")),
        ("p23_veraeusserungspreis__2", json!(2_000_000)),
        ("p23_anschaffung_herstellungskosten__2", json!(1_500_000)),
        ("p23_werbungskosten__2", json!(50_000)),
        ("p23_veraeusserungs_typ__2", json!("anderes_wg")),
        ("p23_veraeusserungspreis__3", json!(100_000)),
        ("p23_anschaffung_herstellungskosten__3", json!(100_000)),
        ("p23_werbungskosten__3", json!(0)),
        ("p23_veraeusserungs_typ__3", json!("grundstueck")),
    ]);
    let d = deklariere(&f, index(), 2025, None).unwrap();
    assert_eq!(
        instanzen(&d, "p23_veraeusserung"),
        vec![
            (1, vec![("E0306801".to_owned(), json!(-5_500))]),
            (2, vec![("E0307701".to_owned(), json!(4_500))]),
        ]
    );
}

/// Ein zweites Vermietungsobjekt: die vier Werbungskosten-Felder (270.000 ct = 2.700 EUR) laufen in die Aggregat-Summe
/// E0703838 der Instanz 2, positiv; die Einnahmen (4.800 EUR) stehen in E0700201. Python: Summe 2.700, vier Quellfelder.
#[test]
fn aggregat_summe_eines_weiteren_objekts_ist_die_summe_seiner_werbungskosten() {
    let f = felder(&[
        ("vv_einnahmen__2", json!(480_000)),
        ("vv_gebaeude_afa__2", json!(150_000)),
        ("vv_schuldzinsen__2", json!(90_000)),
        ("vv_erhaltungsaufwand__2", json!(20_000)),
        ("vv_sonstige_wk__2", json!(10_000)),
    ]);
    let d = deklariere(&f, index(), 2025, None).unwrap();
    let (_, objekte) = d
        .anlage_instanzen
        .iter()
        .find(|(g, _)| g == "vv_objekt")
        .expect("Gruppe vv_objekt");
    assert_eq!(objekte.len(), 1, "{objekte:?}");
    let objekt = &objekte[0];
    assert_eq!(objekt.index, 2);
    assert_eq!(objekt.felder.get("E0700201"), Some(&json!(4_800)));
    let agg = &objekt.dokumentiert["E0703838"];
    assert_eq!(agg.summe, 2_700);
    assert_eq!(
        agg.quell_felder,
        [
            "vv_erhaltungsaufwand__2",
            "vv_gebaeude_afa__2",
            "vv_schuldzinsen__2",
            "vv_sonstige_wk__2"
        ]
    );
}
