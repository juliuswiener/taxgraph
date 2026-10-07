//! Spenden an politische Parteien (`parteispenden_betrag`, Abweichung Nr. 31 in `rust/fixtures/README.md`): die Erklaerung traegt
//! den Betrag. Die Rechnung senkt die Steuer um die Haelfte der Spende (`parteispenden_ermaessigung.rs`); die Erklaerung muss
//! dazu die Spende selbst nennen, in der Zeile der Anlage Sonderausgaben, die den Parteien gehoert (Zeile 7, Kennzahl
//! `E0108701`, Knoten `Sp_MB/Polit_P/Sum_Best`; `E10-2025.xsd`, `E10-2024.xsd`). Fehlte der Betrag dort, fehlte dem Bescheid
//! sein Gegenstueck in der Erklaerung: die bekannte Naht-Luecke, die das Finanzamt anders rechnen liesse als `TaxGraph`.
//!
//! Der Test geht ueber `deklariere` und `einreichungs_xml`, nie ueber `POST /einreichen`: dieser Weg ruft danach `ERiC`. Die
//! Akte ist `rust/fixtures/e2e/gesamt.json` (vollstaendig, mit Stammdaten).
//!
//! Rundung: eine Spende ist ein Aufwand und wird in der Erklaerung auf volle Euro aufgerundet (`ABZUGS_KZ`, "zu Ihren
//! Gunsten"); die Rechnung rundet den Betrag ab (`cent_zu_euro`, wie bei der allgemeinen Spende) und rechnet deshalb hoechstens
//! 1 Euro Spende (0,50 Euro Ermaessigung) unter dem, was das Finanzamt aus dem erklaerten Betrag rechnet.
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

const FELD: &str = "parteispenden_betrag";
const KZ: &str = "E0108701";
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

/// Die vollstaendige Akte; `partei` und `allgemein` sind Betraege in CENT, `None` heisst nie beantwortet.
fn akte(partei: Option<i64>, allgemein: Option<i64>) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    for (feld, cent) in [(FELD, partei), ("spenden_betrag", allgemein)] {
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

/// KONTROLLE zuerst: die Akte ohne das Feld traegt weder `E0108701` noch eine Spende. Sonst belegte "steht im XML" nichts: ein
/// Kz, das schon ohne das Feld in der Deklaration steht, liesse die Tests unten gruen.
#[test]
fn kontrolle_ohne_das_feld_steht_kein_kz_in_der_erklaerung() {
    for vz in [2024, 2025, 2026] {
        let d = deklaration(&akte(None, None), vz);
        assert!(!d.contains_key(KZ), "{vz}: {:?}", d.get(KZ));
    }
}

/// AK1: 500 Euro Parteispende stehen unter `E0108701`, in allen drei Jahren, und die allgemeine Spende bleibt in `E0108105`:
/// die zwei Betraege verwechseln sich nicht.
#[test]
fn die_parteispende_steht_unter_e0108701_die_allgemeine_unter_e0108105() {
    for vz in [2024, 2025, 2026] {
        let nur_partei = deklaration(&akte(Some(50_000), None), vz);
        assert_eq!(nur_partei.get(KZ).map(String::as_str), Some("500"), "{vz}");
        assert!(
            !nur_partei.contains_key(KZ_ALLGEMEIN),
            "{vz}: die Parteispende steht in der Zeile der allgemeinen Spende: {:?}",
            nur_partei.get(KZ_ALLGEMEIN)
        );
        let beide = deklaration(&akte(Some(50_000), Some(120_000)), vz);
        assert_eq!(beide.get(KZ).map(String::as_str), Some("500"), "{vz}");
        assert_eq!(
            beide.get(KZ_ALLGEMEIN).map(String::as_str),
            Some("1200"),
            "{vz}"
        );
    }
}

/// Eine Spende ist ein Aufwand: 500,01 Euro stehen als 501 in der Erklaerung ("zu Ihren Gunsten"). Ein Kz ausserhalb von
/// `ABZUGS_KZ` rundete ab und schriebe 500.
#[test]
fn der_betrag_wird_aufgerundet() {
    let d = deklaration(&akte(Some(50_001), None), 2025);
    assert_eq!(d.get(KZ).map(String::as_str), Some("501"), "{d:?}");
}

/// Das Schema verbietet die 0 in `E0108701` (`GanzzahlPosOhneFuehrNull`): 0 heisst "nichts anzugeben" und bleibt aus der
/// Erklaerung, sonst lehnt `ERiC` die ganze Erklaerung ab (Vault `elster-null-in-kz-ohne-null-weglassen`).
#[test]
fn null_bleibt_aus_der_erklaerung() {
    for vz in [2024, 2025, 2026] {
        let d = deklaration(&akte(Some(0), None), vz);
        assert!(!d.contains_key(KZ), "{vz}: {:?}", d.get(KZ));
    }
}

/// Das XML (nur mit dem amtlichen Schema, das nicht im Repo liegt): `E0108701` steht unter `Sp_MB/Polit_P/Sum_Best`, und die
/// Erklaerung ist unvollstaendig NICHT wegen dieses Feldes. Ohne Schema prueft der Test nur den Weg bis zum Writer.
#[test]
fn das_xml_traegt_den_betrag_im_knoten_der_parteien() {
    let s = akte(Some(50_000), None);
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
    let nach_polit = xml
        .split("<Polit_P>")
        .nth(1)
        .unwrap_or_else(|| panic!("kein Knoten Polit_P im XML"));
    let im_knoten = nach_polit.split("</Polit_P>").next().unwrap();
    assert!(
        im_knoten.contains("<Sum_Best>") && im_knoten.contains("<E0108701>500</E0108701>"),
        "E0108701 steht nicht unter Polit_P/Sum_Best: {im_knoten}"
    );
}
