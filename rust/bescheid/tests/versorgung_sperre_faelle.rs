//! Welche Versorgungsfaelle die Abgabe sperrt und welche nicht (Abweichung Nr. 64, Backlog
//! `renten-mit-leistungsmitteilung-bauen-oder-hart-sperren` AK11 bis AK13). Ohne `PARITY=1`, ohne Python, ohne Netz, ohne `ERiC`.
//!
//! **Worum es geht.** Die Abgabe-Sperre Nr. 42 hielt jeden Versorgungsbezug ueber 0 zurueck, weil die Erklaerung ihn nicht trug.
//! Seit Nr. 50 traegt sie ihn in den Zeilen 11 bis 13 der Anlage N. Julius hat die Sperre am 2026-10-12 gehoben, soweit die
//! Erklaerung den Bezug voll traegt. Was sie nicht traegt, bleibt gesperrt, mit eigenem Text.
//!
//! **Die Tabelle.** `die_tabelle_sagt_fall_fuer_fall_ob_die_abgabe_sperrt` laeuft jeden Fall durch den Weg der Produktion
//! (`mit_ring_werten`, dann `deklariere`; `einreichungs_xml` ruft beide) und druckt je Fall, was in `unvollstaendig` fuer die
//! Versorgungsfelder steht. Mit `--nocapture` ist das die Tabelle "Fall, heute, nachher" des Berichts; der Test selbst prueft die
//! erwartete Spalte "nachher".
//!
//! Die Faelle, die der Bescheid schon vor der Erklaerung sperrt (Bezug ueber dem Lohn, Bemessungsgrundlage fehlt), stehen in
//! `versorgung_abgabe_sperre.rs`: der Ring rechnet sie nicht.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::deklaration::mit_ring_werten;
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Vz, Zustand};
use elster::deklariere;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

type Paare = Vec<(&'static str, Value)>;
/// Ein Fall der Tabelle: Name, Akte, erwartete Sperren nachher als (Feld, Art); eine leere Liste heisst: die Abgabe ist frei.
type Fall = (&'static str, Paare, Vec<(&'static str, &'static str)>);

const A: &str = "versorgung_jahresrente";
const B: &str = "versorgung_jahresrente_partner";

fn setze(s: &mut Store, feld: &str, wert: Value) {
    let leer = HashMap::new();
    s.append_roh(
        &NeuesEventRoh {
            feld_id: feld.to_owned(),
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: HerkunftVektor::Voll(Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }),
            schreiber: "ui:laie".to_owned(),
            signal: Signal {
                signal_1: Some(None),
                signal_2: Some(format!("ok@{feld}")),
                signal_2_fehlt: false,
            },
            signal_2_fremd: None,
            ersetzt: None,
            ts: Some("2026-09-01T00:00:00+00:00".to_owned()),
        },
        None,
        BindungNachschlag::neu(&leer),
    )
    .unwrap();
}

/// Die vollstaendige Akte `rentner.json` (Rentner-Scheibe, einzeln veranlagt) mit `mehr` obendrauf. Ein Feld aus `mehr`, das die
/// Akte schon fuehrt (`veranlagung`), ersetzt den Eintrag der Akte.
fn akte(mehr: &[(&'static str, Value)]) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/rentner.json");
    let mut roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    roh["events"]
        .as_array_mut()
        .unwrap()
        .retain(|e| !e["feld_id"].as_str().is_some_and(|f| mehr.iter().any(|(m, _)| *m == f)));
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    for (feld, wert) in mehr {
        setze(&mut s, feld, wert.clone());
    }
    s
}

/// Der Lohnteil einer Person: Zeile 5 (Nr. 3 der Bescheinigung, enthaelt den Bezug), Steuerklasse und Lohnsteuer.
fn lohn(euro: i64) -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(euro * 100)),
        ("steuerklasse", json!(1)),
        ("p36_lohnsteuer", json!(300_000)),
    ]
}

/// Der Bezug Person A in CENT (Nr. 8, Nr. 29, Nr. 30 der Bescheinigung).
fn bezug(jahresrente: i64, bemessungsgrundlage: i64, beginn: i64, art: &str) -> Paare {
    vec![
        (A, json!(jahresrente)),
        ("versorgung_bemessungsgrundlage", json!(bemessungsgrundlage)),
        ("versorgung_beginn_jahr", json!(beginn)),
        ("versorgung_art", json!(art)),
    ]
}

/// Der Ehegatte als Person B: Zusammenveranlagung, Stammdaten, Kapital-Nullen, Lohnteil und optional ein Bezug
/// (Jahresrente, Bemessungsgrundlage in CENT, Beginnjahr).
fn ehegatte(veranlagung: &'static str, bezug_partner: Option<[i64; 3]>) -> Paare {
    let mut p: Paare = vec![
        ("stammdaten_nachname_partner", json!("Muster")),
        ("stammdaten_vorname_partner", json!("Erika")),
        ("stammdaten_geburtsdatum_partner", json!("01.01.1960")),
        ("geburtsjahr_partner", json!(1960)),
        ("kist_konfession_partner", json!("keine")),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("veranlagung", json!(veranlagung)),
        ("bruttoarbeitslohn_partner", json!(2_000_000)),
        ("steuerklasse_partner", json!(1)),
        ("p36_lohnsteuer_partner", json!(300_000)),
    ];
    if let Some([rente, bmg, beginn]) = bezug_partner {
        p.extend([
            ("versorgung_jahresrente_partner", json!(rente)),
            ("versorgung_bemessungsgrundlage_partner", json!(bmg)),
            ("versorgung_beginn_jahr_partner", json!(beginn)),
            ("versorgung_art_partner", json!("beamtenrechtlich")),
        ]);
    }
    p
}

fn mit(mut a: Paare, b: Paare) -> Paare {
    a.extend(b);
    a
}

/// Wie die Produktion (`einreichungs_xml`): Ring, dann `deklariere`. `(feld_id, grund)` je Sperre der Versorgungsfelder.
fn sperren(mehr: &[(&'static str, Value)]) -> Vec<(String, String)> {
    let (mut felder, _) = akte(mehr).materialisiere(None).unwrap();
    mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    d.unvollstaendig()
        .iter()
        .filter(|e| e.feld_id.starts_with("versorgung_jahresrente"))
        .map(|e| (e.feld_id.clone(), e.grund.clone()))
        .collect()
}

/// Was ein Grund sagt, an einem Merkmal im Text: der Bezug passt nicht in die Zeilen 11 bis 13, oder er beginnt im
/// Veranlagungsjahr oder danach (Zeile 14). Ein Text ohne eindeutiges Merkmal heisst `"?"` und macht die Tabelle rot.
fn art(grund: &str) -> &'static str {
    assert!(grund.starts_with("Versorgungsbezüge über 0 Euro"), "{grund}");
    match (grund.contains("nicht in die Zeilen 11 bis 13"), grund.contains("Zeile 14")) {
        (true, false) => "ohne_zeile",
        (false, true) => "beginn",
        _ => "?",
    }
}

#[test]
fn die_tabelle_sagt_fall_fuer_fall_ob_die_abgabe_sperrt() {
    let tragbar = || mit(lohn(50_000), bezug(3_000_000, 2_800_000, 2020, "beamtenrechtlich"));
    let gate = |alter: i64| {
        let mut b = bezug(3_000_000, 3_000_000, 2020, "altersgrenze_sonstige");
        b.push(("versorgung_alter_bei_beginn", json!(alter)));
        mit(lohn(50_000), b)
    };
    let b_tragbar = [2_000_000, 1_900_000, 2018];
    // (Fall, Akte, erwartete Sperren nachher als (Feld, Art)); eine leere Liste heisst: die Abgabe ist frei.
    let faelle: Vec<Fall> = vec![
        ("A allein, Beginn 2020", tragbar(), vec![]),
        ("A allein, Alters-Gate mit 63 erfuellt", gate(63), vec![]),
        ("A allein, Alters-Gate mit 62 nicht erfuellt", gate(62), vec![(A, "ohne_zeile")]),
        (
            "A allein, Bezug unter 1 Euro",
            mit(lohn(50_000), bezug(99, 99, 2020, "beamtenrechtlich")),
            vec![(A, "ohne_zeile")],
        ),
        // Jede der beiden Schwellen einzeln: das Schema nimmt `E0200801` und `E0200902` erst ab 1 Euro.
        (
            "A allein, Bezug unter 1 Euro, Bemessungsgrundlage gross",
            mit(lohn(50_000), bezug(99, 2_800_000, 2020, "beamtenrechtlich")),
            vec![(A, "ohne_zeile")],
        ),
        (
            "A allein, Bezug gross, Bemessungsgrundlage unter 1 Euro",
            mit(lohn(50_000), bezug(3_000_000, 99, 2020, "beamtenrechtlich")),
            vec![(A, "ohne_zeile")],
        ),
        (
            "A allein, Beginn = Veranlagungsjahr 2025",
            mit(lohn(50_000), bezug(3_000_000, 2_800_000, 2025, "beamtenrechtlich")),
            vec![(A, "beginn")],
        ),
        (
            "A allein, Beginn 2024 (Jahr davor)",
            mit(lohn(50_000), bezug(3_000_000, 2_800_000, 2024, "beamtenrechtlich")),
            vec![],
        ),
        (
            "A allein, Beginn nach dem Veranlagungsjahr (2030)",
            mit(lohn(50_000), bezug(3_000_000, 2_800_000, 2030, "beamtenrechtlich")),
            vec![(A, "beginn")],
        ),
        (
            "A allein, Bezug 0",
            mit(lohn(50_000), bezug(0, 2_800_000, 2020, "beamtenrechtlich")),
            vec![],
        ),
        ("Ehegatte, Zusammenveranlagung, Beginn 2018", mit(lohn(30_000), ehegatte("zusammen", Some(b_tragbar))), vec![]),
        ("Ehegatte, Einzelveranlagung", mit(lohn(30_000), ehegatte("einzel", Some(b_tragbar))), vec![]),
        (
            "Ehegatte, Zusammenveranlagung, Beginn = Veranlagungsjahr",
            mit(lohn(30_000), ehegatte("zusammen", Some([2_000_000, 1_900_000, 2025]))),
            vec![(B, "beginn")],
        ),
        (
            "Ehegatte, Zusammenveranlagung, Bezug unter 1 Euro",
            mit(lohn(30_000), ehegatte("zusammen", Some([99, 99, 2018]))),
            vec![(B, "ohne_zeile")],
        ),
        (
            "Ehegatte, Einzelveranlagung, Beginn = Veranlagungsjahr (zaehlt nicht)",
            mit(lohn(30_000), ehegatte("einzel", Some([2_000_000, 1_900_000, 2025]))),
            vec![],
        ),
        ("A und Ehegatte, beide tragbar", mit(tragbar(), ehegatte("zusammen", Some(b_tragbar))), vec![]),
        (
            "A tragbar, Ehegatte Beginn = Veranlagungsjahr",
            mit(tragbar(), ehegatte("zusammen", Some([2_000_000, 1_900_000, 2025]))),
            vec![(B, "beginn")],
        ),
        (
            "A Beginn = Veranlagungsjahr, Ehegatte tragbar",
            mit(
                mit(lohn(50_000), bezug(3_000_000, 2_800_000, 2025, "beamtenrechtlich")),
                ehegatte("zusammen", Some(b_tragbar)),
            ),
            vec![(A, "beginn")],
        ),
    ];
    eprintln!("TABELLE Fall | Sperren der Versorgungsfelder (Feld: Art)");
    let mut abweichungen = Vec::new();
    for (name, mehr, erwartet) in faelle {
        let ist: Vec<(String, &str)> = sperren(&mehr).into_iter().map(|(f, g)| (f, art(&g))).collect();
        let zeile = if ist.is_empty() {
            "frei".to_owned()
        } else {
            ist.iter().map(|(f, a)| format!("{f}: {a}")).collect::<Vec<_>>().join(", ")
        };
        eprintln!("TABELLE {name} | {zeile}");
        let soll: Vec<(String, &str)> = erwartet.iter().map(|(f, a)| ((*f).to_owned(), *a)).collect();
        if ist != soll {
            abweichungen.push(format!("{name}: erwartet {soll:?}, erhalten {ist:?}"));
        }
    }
    assert!(abweichungen.is_empty(), "{} Fall/Faelle weichen ab:\n{}", abweichungen.len(), abweichungen.join("\n"));
}

/// Die Schwelle selbst (100 Cent) traegt: die Zeilen 11 und 12 nehmen genau 1 Euro, die Sperre faellt. Darunter (99 Cent)
/// sperrt sie, siehe die Tabelle oben. Jede der beiden Schwellen einzeln und beide zusammen.
#[test]
fn genau_ein_euro_in_betrag_und_bemessungsgrundlage_sperrt_nicht() {
    for (rente, bmg) in [(100, 2_800_000), (3_000_000, 100), (100, 100)] {
        let ist = sperren(&mit(lohn(50_000), bezug(rente, bmg, 2020, "beamtenrechtlich")));
        assert!(ist.is_empty(), "Bezug {rente} Cent, Bemessungsgrundlage {bmg} Cent: {ist:?}");
    }
}
