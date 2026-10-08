//! Spenden an unabhaengige Waehlervereinigungen (`waehlervereinigungen_betrag`, Abweichung Nr. 44 in `rust/fixtures/README.md`):
//! die Erklaerung traegt den Betrag. Die Rechnung senkt die Steuer um die Haelfte der Spende (`waehlervereinigungen.rs`); die
//! Erklaerung muss dazu die Spende selbst nennen, in der Zeile der Anlage Sonderausgaben, die den Waehlervereinigungen gehoert
//! (Zeile 8, Kennzahl `E0108801`, Knoten `Sp_MB/Unabh_Waehl_V/Sum_Best`; `E10-2025.xsd`, `E10-2024.xsd`). Fehlte der Betrag dort,
//! fehlte dem Bescheid sein Gegenstueck in der Erklaerung: die bekannte Naht-Luecke.
//!
//! Der Test geht ueber `deklariere` und `einreichungs_xml`, nie ueber `POST /einreichen`: dieser Weg ruft danach `ERiC`. Die
//! Akte ist `rust/fixtures/e2e/gesamt.json` (vollstaendig, mit Stammdaten). Vorlage: `parteispenden_einreichung_hermetisch.rs`.
//!
//! Rundung: wie bei den Parteien wird der Betrag in der Erklaerung auf volle Euro aufgerundet (`ABZUGS_KZ`, "zu Ihren Gunsten").
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::deklaration::einreichungs_xml;
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Zustand};
use elster::deklariere;
use elster::testhilfe::schemas_da;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const FELD: &str = "waehlervereinigungen_betrag";
const KZ: &str = "E0108801";
const KZ_PARTEI: &str = "E0108701";
const KZ_ALLGEMEIN: &str = "E0108105";

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

/// Die vollstaendige Akte; `wv`, `partei` und `allgemein` sind Betraege in CENT, `None` heisst nie beantwortet.
fn akte(wv: Option<i64>, partei: Option<i64>, allgemein: Option<i64>) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    for (feld, cent) in [(FELD, wv), ("parteispenden_betrag", partei), ("spenden_betrag", allgemein)] {
        if let Some(c) = cent {
            setze(&mut s, feld, json!(c));
        }
    }
    s
}

/// Die Deklaration (Kz -> Text) der Akte im Veranlagungsjahr `vz`.
fn deklaration(s: &Store, vz: i64) -> std::collections::BTreeMap<String, String> {
    let (felder, _) = s.materialisiere(None).unwrap();
    let d = deklariere(&felder, index(), vz, None).unwrap();
    d.deklaration
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                v.as_str().map_or_else(|| v.to_string(), str::to_owned),
            )
        })
        .collect()
}

/// KONTROLLE zuerst: die Akte ohne das Feld traegt weder `E0108801` noch eine Spende. Sonst belegte "steht im XML" nichts.
#[test]
fn kontrolle_ohne_das_feld_steht_kein_kz_in_der_erklaerung() {
    for vz in [2024, 2025, 2026] {
        let d = deklaration(&akte(None, None, None), vz);
        assert!(!d.contains_key(KZ), "{vz}: {:?}", d.get(KZ));
    }
}

/// 500 Euro Spende an eine Waehlervereinigung stehen unter `E0108801`, in allen drei Jahren; die Parteispende (`E0108701`) und die
/// allgemeine Spende (`E0108105`) bleiben in ihren Zeilen: die drei Betraege verwechseln sich nicht.
#[test]
fn die_waehlervereinigung_steht_unter_e0108801_die_anderen_in_ihren_zeilen() {
    for vz in [2024, 2025, 2026] {
        let nur_wv = deklaration(&akte(Some(50_000), None, None), vz);
        assert_eq!(nur_wv.get(KZ).map(String::as_str), Some("500"), "{vz}");
        assert!(!nur_wv.contains_key(KZ_PARTEI), "{vz}: {:?}", nur_wv.get(KZ_PARTEI));
        assert!(!nur_wv.contains_key(KZ_ALLGEMEIN), "{vz}: {:?}", nur_wv.get(KZ_ALLGEMEIN));
        let alle = deklaration(&akte(Some(50_000), Some(70_000), Some(120_000)), vz);
        assert_eq!(alle.get(KZ).map(String::as_str), Some("500"), "{vz}");
        assert_eq!(alle.get(KZ_PARTEI).map(String::as_str), Some("700"), "{vz}");
        assert_eq!(alle.get(KZ_ALLGEMEIN).map(String::as_str), Some("1200"), "{vz}");
    }
}

/// Eine Spende ist ein Aufwand: 500,01 Euro stehen als 501 in der Erklaerung ("zu Ihren Gunsten"). Ein Kz ausserhalb von
/// `ABZUGS_KZ` rundete ab und schriebe 500.
#[test]
fn der_betrag_wird_aufgerundet() {
    let d = deklaration(&akte(Some(50_001), None, None), 2025);
    assert_eq!(d.get(KZ).map(String::as_str), Some("501"), "{d:?}");
}

/// Das Schema verbietet die 0 in `E0108801` (`GanzzahlPosOhneFuehrNull`): 0 heisst "nichts anzugeben" und bleibt aus der
/// Erklaerung, sonst lehnt `ERiC` die ganze Erklaerung ab (Vault `elster-null-in-kz-ohne-null-weglassen`).
#[test]
fn null_bleibt_aus_der_erklaerung() {
    for vz in [2024, 2025, 2026] {
        let d = deklaration(&akte(Some(0), None, None), vz);
        assert!(!d.contains_key(KZ), "{vz}: {:?}", d.get(KZ));
    }
}

/// Das XML (nur mit dem amtlichen Schema, das nicht im Repo liegt): `E0108801` steht unter `Sp_MB/Unabh_Waehl_V/Sum_Best`, und die
/// Erklaerung ist unvollstaendig NICHT wegen dieses Feldes. Ohne Schema prueft der Test nur den Weg bis zum Writer.
#[test]
fn das_xml_traegt_den_betrag_im_knoten_der_waehlervereinigungen() {
    let s = akte(Some(50_000), None, None);
    let r = einreichungs_xml(&s, index(), params(), "BY", Some("74931".to_owned()));
    if !schemas_da(2025) {
        // Ohne Schema endet der Lauf am Writer; Guard und Deklaration liefen, und keine Sperre nennt das Feld.
        if let Err(bescheid::deklaration::EinreichFehler::DeklarationUnvollstaendig(e)) = &r {
            assert!(
                !e.iter().any(|x| x.feld_id == FELD),
                "das Feld sperrt die Abgabe: {e:?}"
            );
        }
        return;
    }
    let xml = r.unwrap().xml;
    let nach_knoten = xml
        .split("<Unabh_Waehl_V>")
        .nth(1)
        .unwrap_or_else(|| panic!("kein Knoten Unabh_Waehl_V im XML"));
    let im_knoten = nach_knoten.split("</Unabh_Waehl_V>").next().unwrap();
    assert!(
        im_knoten.contains("<Sum_Best>") && im_knoten.contains("<E0108801>500</E0108801>"),
        "E0108801 steht nicht unter Unabh_Waehl_V/Sum_Best: {im_knoten}"
    );
}
