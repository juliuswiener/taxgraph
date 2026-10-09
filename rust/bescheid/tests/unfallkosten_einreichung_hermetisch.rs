//! Unfallkosten auf dem Arbeitsweg (`ep_unfallkosten`, Abweichung Nr. 28 in `rust/fixtures/README.md`): die Rechnung zieht den
//! Betrag ab. Seit Abweichung Nr. 48 traegt die Deklaration ihn in der Zeile "Sonstiges" der Anlage N (`unfallkosten_weitere_wk_xml.rs`),
//! aber der Kz ist UNGEPRUEFT (`kz_status: offen`, `checkESt` lief nie). Ein Bescheid, dessen Abzug eine ungepruefte
//! Zeile traegt, bleibt gesperrt. Darum sperrt `deklariere` die Abgabe bei einem Betrag ueber 0
//! (`einreichungs_xml` -> `DeklarationUnvollstaendig` -> 409 `deklaration_unvollstaendig`, `api/src/einreichen.rs`), und bei
//! leerem Feld oder 0 aendert sich nichts.
//!
//! Der Test geht ueber `einreichungs_xml` und nicht ueber `POST /einreichen`: der HTTP-Weg ruft danach `ERiC`. Die Sperre
//! liegt VOR dem Writer, ihr Ergebnis braucht kein ERiC-Schema. Die Akte ist `rust/fixtures/e2e/gesamt.json` (vollstaendig,
//! mit Stammdaten), mit einem Arbeitsweg von 30 km und 220 Tagen.
//!
//! Fehlt das ERiC-Schema, endet der Lauf OHNE Betrag am Writer mit `XmlNichtBaubar`, nachdem Guard und Deklaration liefen
//! (wie in `dienstwohnung_einreichung_hermetisch.rs`); mit Schema ist es ein XML.
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
use elster::deklariere;
use elster::testhilfe::schemas_da;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const FELD: &str = "ep_unfallkosten";

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

/// Die Akte mit einem Arbeitsweg (30 km, 220 Tage, Kfz); `unfall` ist der Betrag in CENT, `None` heisst nie beantwortet.
/// Die drei Felder des Wegs stehen schon in der Akte (mit 0 km): ihr Wert wird im JSON getauscht, wie in
/// `api/tests/einreichen_attrappe_hermetisch.rs::setze` (die Akte laedt ungeprueft, `append_roh` verlangte `ersetzt`).
fn akte(unfall: Option<i64>) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let mut roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    for e in roh["events"].as_array_mut().unwrap() {
        match e["feld_id"].as_str() {
            Some("ep_entfernung_km") => e["wert"] = json!(30),
            Some("ep_arbeitstage") => e["wert"] = json!(220),
            Some("ep_eigenes_kfz") => e["wert"] = json!(true),
            _ => {}
        }
    }
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    if let Some(cent) = unfall {
        setze(&mut s, FELD, json!(cent));
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

/// KONTROLLE zuerst: derselbe Arbeitsweg OHNE Unfallkosten kommt durch `deklariere`; er endet im XML (mit Schema) oder
/// am Writer (ohne). Sonst belegte die Sperre unten nichts: ein Fall, der aus anderem Grund sperrt, laesst sie gruen.
#[test]
fn kontrolle_der_arbeitsweg_ohne_unfallkosten_ist_einreichbar() {
    let ohne = lauf(&akte(None));
    assert_eq!(ohne.is_ok(), schemas_da(2025), "{ohne:?}");
    assert!(
        sperre(&ohne).is_empty(),
        "der Arbeitsweg allein sperrt schon: {ohne:?}"
    );
}

/// AK5: ein Betrag ueber 0 sperrt die Abgabe mit EIGENEM Grund (409 `deklaration_unvollstaendig`), und dieser Grund nennt
/// den Abzug und die Sperre; kein anderes Feld steht in der Liste.
#[test]
fn unfallkosten_ueber_null_sperren_die_abgabe_mit_eigenem_grund() {
    for cent in [150_000, 1] {
        let r = lauf(&akte(Some(cent)));
        let e = sperre(&r);
        assert_eq!(
            e.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
            [FELD],
            "{cent} Cent: die Sperre nennt genau dieses Feld, erhalten {r:?}"
        );
        let grund = &e.first().unwrap().1;
        for teil in ["Unfallkosten", "Kennzeichen", "gesperrt", "auf 0"] {
            assert!(grund.contains(teil), "Grund ohne `{teil}`: {grund}");
        }
    }
}

/// AK4: leer, 0 und nie beantwortet geben DIESELBE Antwort wie die Akte ohne das Feld: dasselbe XML, oder (ohne Schema)
/// derselbe Fehler am Writer. Die Sperre greift erst ueber 0.
#[test]
fn null_und_leer_aendern_die_abgabe_nicht() {
    let ohne = lauf(&akte(None));
    let null = lauf(&akte(Some(0)));
    assert!(sperre(&null).is_empty(), "0 sperrt: {null:?}");
    match (ohne, null) {
        (Ok(a), Ok(b)) => assert_eq!(a, b, "das XML aendert sich durch 0 Cent"),
        (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string()),
        (a, b) => panic!("ohne Feld {a:?}, mit 0 {b:?}"),
    }
}

/// Abweichung Nr. 48: bei 0 steht nichts in der Deklaration, der Betrag kommt mit Grund in `nicht_deklariert` (kein Kz,
/// kein erfundener Wert) und die Abgabe ist nicht gesperrt. Ueber 0 steht der Betrag unter `E0205406` (Zeile "Sonstiges"
/// der Anlage N, aufgerundet) mit Bezeichnung `E0205405` und Summe `E0204803`; er steht dann NICHT mehr in
/// `nicht_deklariert`, aber weiter unter `unvollstaendig`: die Zeile ist UNGEPRUEFT (`checkESt` lief nie), die Sperre bleibt.
/// Das XML selbst liest `unfallkosten_weitere_wk_xml.rs`.
#[test]
fn der_betrag_steht_ueber_null_in_der_zeile_sonstiges_und_die_sperre_bleibt() {
    for (cent, euro) in [(0_i64, None), (150_000, Some("1500")), (150_001, Some("1501"))] {
        let s = akte(Some(cent));
        let (felder, _) = s.materialisiere(None).unwrap();
        let d = deklariere(&felder, index(), 2025, None).unwrap();
        let nicht: Vec<_> = d
            .nicht_deklariert
            .iter()
            .filter(|e| e.feld_id == FELD)
            .collect();
        let kz = |k: &str| d.deklaration.get(k).map(|v| v.to_string().trim_matches('"').to_owned());
        match euro {
            None => {
                assert_eq!(nicht.len(), 1, "{cent} Cent: {:?}", d.nicht_deklariert);
                assert!(
                    nicht.first().unwrap().grund.contains("kz_status offen"),
                    "der Grund nennt den offenen Kz nicht: {:?}",
                    nicht.first()
                );
                for k in ["E0205405", "E0205406", "E0204803"] {
                    assert_eq!(kz(k), None, "{cent} Cent: {k} ohne Betrag");
                }
                assert!(d.unvollstaendig().iter().all(|e| e.feld_id != FELD));
            }
            Some(euro) => {
                assert!(nicht.is_empty(), "{cent} Cent: {:?}", d.nicht_deklariert);
                assert_eq!(kz("E0205406").as_deref(), Some(euro), "{cent} Cent: Betrag");
                assert_eq!(kz("E0204803").as_deref(), Some(euro), "{cent} Cent: Summe");
                assert!(
                    kz("E0205405").is_some_and(|t| t.contains("Unfall")),
                    "{cent} Cent: Bezeichnung {:?}",
                    kz("E0205405")
                );
                assert!(
                    d.unvollstaendig().iter().any(|e| e.feld_id == FELD),
                    "{cent} Cent: die Sperre ist weg: {:?}",
                    d.unvollstaendig()
                );
            }
        }
    }
}
