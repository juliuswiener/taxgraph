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

const ANTEIL: &str = "kind_schulgeld_aufteilung_prozent";

/// Der Kz-Wert `E0504603` des Kindes `index`, oder `None`. Kind 1 steht auf oberster Ebene der Deklaration (wie jedes
/// Feld ohne `__N`), Kind 2 und folgende in den Instanzen der Gruppe `kind`.
fn anteil_kz(d: &Deklaration, index: u64) -> Option<Value> {
    if index == 1 {
        return d.deklaration.get("E0504603").cloned();
    }
    instanzen(d, "kind")
        .into_iter()
        .find(|(i, _)| *i == index)
        .and_then(|(_, kz)| kz.into_iter().find(|(k, _)| k == "E0504603").map(|(_, v)| v))
}

/// ABWEICHUNG Nr. 26 (`rust/fixtures/README.md`, nur Rust, kein Python-Orakel): der Anteil am Schulgeld-Hoechstbetrag je
/// Kind (`E0504603`) steht im XML nur bei bestaetigter Einzelveranlagung und einem Anteil, der von 50 abweicht. 50 heisst
/// "je zur Haelfte" und ist kein gesonderter Antrag; die 0 erlaubt das Schema (`[1-9].*|0`) und gehoert hinein. Bei
/// Zusammenveranlagung oder offener Veranlagung bleibt die Angabe draussen, und der Grund steht in `nicht_deklariert`.
/// Jedes Kind traegt seinen eigenen Wert (Kind 2 ueber `..__2`).
#[test]
fn schulgeld_anteil_steht_im_kind_nur_bei_einzelveranlagung_und_abweichung_von_50() {
    const ANTEIL_2: &str = "kind_schulgeld_aufteilung_prozent__2";
    // `veranlagung`: Wert und Zustand; `anteile`: je Kind ein Anteil, Kind 1 und Kind 2 tragen immer dasselbe Schulgeld.
    let kz = |veranlagung: Option<(&str, Zustand)>, anteile: &[(&str, i64)]| {
        let mut paare = vec![
            ("schulgeld", json!(2_000_000)),
            ("schulgeld__2", json!(2_000_000)),
        ];
        if let Some((v, _)) = veranlagung {
            paare.push(("veranlagung", json!(v)));
        }
        for (fid, n) in anteile {
            paare.push((*fid, json!(*n)));
        }
        let mut f = felder(&paare);
        if let Some((_, zustand)) = veranlagung {
            f.get_mut("veranlagung").unwrap().zustand = zustand;
        }
        deklariere(&f, index(), 2025, None).unwrap()
    };
    let einzel = Some(("einzel", Zustand::Bestaetigt));
    let d = kz(einzel, &[(ANTEIL, 100), (ANTEIL_2, 30)]);
    assert_eq!(anteil_kz(&d, 1), Some(json!(100)), "Kind 1, Anteil 100");
    assert_eq!(anteil_kz(&d, 2), Some(json!(30)), "Kind 2, Anteil 30");
    assert!(!d.nicht_deklariert.iter().any(|e| e.feld_id.starts_with(ANTEIL)), "{:?}", d.nicht_deklariert);

    let d = kz(einzel, &[(ANTEIL, 0)]);
    assert_eq!(anteil_kz(&d, 1), Some(json!(0)), "die 0 ist erlaubt und kein 'nichts anzugeben'");

    // Jeder Fall nennt beide Kinder: Kind 1 laeuft ueber `feld`, Kind 2 ueber `instanz_feld` (zwei Zweige).
    for (name, d) in [
        ("Einzel, Anteil 50", kz(einzel, &[(ANTEIL, 50), (ANTEIL_2, 50)])),
        (
            "Zusammen, Anteil 30",
            kz(Some(("zusammen", Zustand::Bestaetigt)), &[(ANTEIL, 30), (ANTEIL_2, 30)]),
        ),
        (
            "Einzel nur vorlaeufig, Anteil 30",
            kz(Some(("einzel", Zustand::Vorlaeufig)), &[(ANTEIL, 30), (ANTEIL_2, 30)]),
        ),
        ("Veranlagung nie beantwortet, Anteil 30", kz(None, &[(ANTEIL, 30), (ANTEIL_2, 30)])),
    ] {
        for (kind, feld) in [(1, ANTEIL), (2, ANTEIL_2)] {
            assert_eq!(anteil_kz(&d, kind), None, "{name}, Kind {kind}: E0504603 darf nicht im XML stehen");
            assert!(
                d.nicht_deklariert.iter().any(|e| e.feld_id == feld),
                "{name}, Kind {kind}: das Weglassen steht mit Grund in nicht_deklariert: {:?}",
                d.nicht_deklariert
            );
        }
    }
    // Ohne Antwort schreibt nichts etwas: die Schulgeld-Summe `E0505607` bleibt der einzige Schulgeld-Kz.
    let d = kz(einzel, &[]);
    assert_eq!(anteil_kz(&d, 1), None);
    assert_eq!(anteil_kz(&d, 2), None);
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
