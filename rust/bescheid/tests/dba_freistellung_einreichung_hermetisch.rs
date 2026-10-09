//! DBA-Freistellung (Abweichung Nr. 40): die Abgabe bleibt gesperrt, auch wenn der Bescheid rechnet. Standardlauf, ohne
//! `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Der Bescheid wendet den Progressionsvorbehalt an (`dba_freistellung_progression.rs`). Die Erklaerung
//! kann freigestellte Auslandseinkuenfte aber noch nicht an der richtigen Stelle melden: das XML schriebe sie unter den Block
//! der Anrechnung (`E0601401` Einkuenfte, `E0601901` Steuer, Anlage AUS). Der Arbeitslohn aus dem Ausland gehoert bei
//! Freistellung in die Anlage N-AUS, andere freigestellte Einkuenfte in Zeilen der Anlage AUS (Anleitung 2025).
//!
//! **Warum es zaehlt.** Ohne Sperre sahe das Finanzamt Einkuenfte mit anzurechnender Steuer, wo das Abkommen sie freistellt;
//! der Bescheid und die Erklaerung liefen auseinander (die bekannte Naht-Luecke, wie bei Unfallkosten und Abzug).
//!
//! Der Test geht ueber `einreichungs_xml` und nicht ueber `POST /einreichen`: der HTTP-Weg ruft danach `ERiC`. Die Sperre
//! liegt VOR dem Writer. Die Akte ist `rust/fixtures/e2e/gesamt.json` (vollstaendig, mit Stammdaten, ohne Auslandsangaben
//! und ohne Kapital), dazu die Auslandsangaben dieses Tests.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::deklaration::{einreichungs_xml, EinreichFehler};
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Zustand};
use elster::testhilfe::schemas_da;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const METHODE: &str = "dba_methode";
/// Seit Abweichung Nr. 51 braucht jede Akte mit Auslandseinkuenften einen Staat. Das Feld `dba_methode` geht dem Staat vor
/// (`dba_methode_der_akte`): Frankreich aendert die Methode nicht.
const STAAT: &str = "dba_staat";

/// Ein bestaetigter Wert (zwei Signale).
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

/// Die vollstaendige Akte plus die gegebenen bestaetigten Auslandsangaben.
fn akte(paare: &[(&str, Value)]) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    for (feld, wert) in paare {
        setze(&mut s, feld, wert.clone());
    }
    s
}

fn lauf(s: &Store) -> Result<String, EinreichFehler> {
    einreichungs_xml(s, index(), params(), "BY", Some("74931".to_owned())).map(|e| e.xml)
}

/// Die Eintraege der Sperre als `(feld_id, grund)`; leer, wenn `lauf` nicht mit `DeklarationUnvollstaendig` endet.
fn sperre(r: &Result<String, EinreichFehler>) -> Vec<(String, String)> {
    match r {
        Err(EinreichFehler::DeklarationUnvollstaendig(e)) => e
            .iter()
            .map(|x| (x.feld_id.clone(), x.grund.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// KONTROLLE zuerst: dieselben Betraege bei Anrechnung sind einreichbar (im XML mit Schema, am Writer ohne) und sperren
/// nicht. Sonst belegte die Sperre unten nichts: ein Fall, der aus anderem Grund sperrt, laesst sie gruen.
#[test]
fn kontrolle_die_anrechnung_ist_einreichbar() {
    for paare in [
        vec![(STEUER, json!(70_000)), (EINKUENFTE, json!(2_000_000)), (STAAT, json!("Frankreich"))],
        vec![
            (STEUER, json!(70_000)),
            (EINKUENFTE, json!(2_000_000)),
            (STAAT, json!("Frankreich")),
            (METHODE, json!("dba_anrechnung")),
        ],
    ] {
        let r = lauf(&akte(&paare));
        assert!(sperre(&r).is_empty(), "die Anrechnung sperrt schon: {r:?}");
        assert_eq!(r.is_ok(), schemas_da(2025), "{r:?}");
    }
}

/// Freigestellte Einkuenfte ueber 0 sperren die Abgabe mit EIGENEM Grund (409 `deklaration_unvollstaendig`): ueber das Feld
/// `dba_methode` und ueber Staat und Einkunftsart. Die Liste nennt genau die Einkuenfte, der Grund Freistellung, Anlage und
/// die Sperre. Ein Cent genuegt.
#[test]
fn freigestellte_einkuenfte_sperren_die_abgabe_mit_eigenem_grund() {
    let faelle: [(&str, Vec<(&str, Value)>); 4] = [
        (
            "Methode",
            vec![
                (EINKUENFTE, json!(2_000_000)),
                (STAAT, json!("Frankreich")),
                (METHODE, json!("dba_freistellung")),
            ],
        ),
        (
            "USA",
            vec![(EINKUENFTE, json!(2_000_000)), (STAAT, json!("USA"))],
        ),
        (
            "Polen, Ruhegehaelter",
            vec![
                (EINKUENFTE, json!(2_000_000)),
                (STAAT, json!("Polen")),
                ("dba_einkunftsart", json!("ruhegehaelter")),
            ],
        ),
        (
            "ein Cent",
            vec![(EINKUENFTE, json!(1)), (STAAT, json!("Frankreich")), (METHODE, json!("dba_freistellung"))],
        ),
    ];
    for (name, paare) in faelle {
        let r = lauf(&akte(&paare));
        let e = sperre(&r);
        assert_eq!(
            e.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
            [EINKUENFTE],
            "{name}: die Sperre nennt genau dieses Feld, erhalten {r:?}"
        );
        let grund = &e.first().unwrap().1;
        for teil in ["Freistellung", "Anlage", "gesperrt"] {
            assert!(grund.contains(teil), "{name}: Grund ohne `{teil}`: {grund}");
        }
    }
}

/// Auch die gezahlte Steuer allein sperrt bei Freistellung: das XML schriebe sie als anzurechnende Steuer (`E0601901`), wo
/// es nichts anzurechnen gibt. Die Liste nennt dann die Steuer.
#[test]
fn bei_freistellung_sperrt_auch_die_gezahlte_steuer_allein() {
    let r = lauf(&akte(&[
        (STEUER, json!(70_000)),
        (METHODE, json!("dba_freistellung")),
    ]));
    let e = sperre(&r);
    // Zwei Gruende zur selben Steuer: die Freistellung (dieser Test) und, seit Abweichung Nr. 51, die Steuer ohne Einkuenfte
    // (`checkESt` lehnt sie auch mit Freistellung ab, rc=610001002). Beide stehen in der Liste, keine andere Angabe.
    assert_eq!(
        e.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
        [STEUER, STEUER],
        "erhalten {r:?}"
    );
    assert!(e.iter().any(|(_, g)| g.contains("Freistellung")), "kein Grund nennt die Freistellung: {r:?}");
    assert!(e.iter().any(|(_, g)| g.contains("Ohne Einkünfte") || g.contains("ohne Einkünfte")), "kein Grund nennt die fehlenden Einkuenfte: {r:?}");
}

/// Ohne Einkuenfte und ohne Steuer ueber 0 sperrt die Freistellung nicht: kein Betrag, der falsch stuende. Der Lauf endet
/// wie der der Anrechnung, im XML oder am Writer, nie in der Sperre.
#[test]
fn freistellung_ohne_betrag_sperrt_nichts() {
    for paare in [
        vec![(METHODE, json!("dba_freistellung"))],
        vec![(EINKUENFTE, json!(0)), (STAAT, json!("Frankreich")), (METHODE, json!("dba_freistellung"))],
        vec![
            (EINKUENFTE, json!(0)),
            (STEUER, json!(0)),
            (STAAT, json!("Frankreich")),
            (METHODE, json!("dba_freistellung")),
        ],
    ] {
        let r = lauf(&akte(&paare));
        assert!(sperre(&r).is_empty(), "{paare:?} sperrt: {r:?}");
        assert_eq!(r.is_ok(), schemas_da(2025), "{paare:?}: {r:?}");
    }
}
