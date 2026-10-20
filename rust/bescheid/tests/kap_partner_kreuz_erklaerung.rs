//! Abweichung Nr. 96 (Julius d-47, 2026-10-20): ein Ehepaar, das „keine Kapitalertraege des Partners“ bestaetigt und die fuenf
//! Kapital-Felder des Ehegatten nicht beantwortet (Fall X), bekommt dieselbe Erklaerung wie das Paar, das die Kapitalertraege ankreuzt
//! und die fuenf Felder mit 0 beantwortet (Fall Y). Die Sperre `partner_kegel_offen` verlangt die fuenf Felder im Fall X nicht mehr
//! (`sperre/gesamt.rs`); diese Datei misst die naechste Stelle der Kette, die Erklaerung: `deklariere` kennt ein fehlendes Kapital-Feld
//! ausdruecklich als 0 (`kap_alle_null`: „jedes Feld fehlt oder ist bestaetigt 0“), und `erzeuge_xml` schreibt dann keine Kapital-Zeile
//! des Ehegatten (Option A). Gemessen wird: gleiche Kz-Werte beider Personen, nichts unvollstaendig, gleiche Pflichtluecken, gleiches
//! XML (mit ERiC-Schema), und dass die Kapital-Zeilen des Ehegatten (`E1900701` und die Toepfe) in keinem der beiden Dokumente stehen.
//! Ohne ERiC-Schema (CI) entfaellt der XML-Teil.
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
use elster::testhilfe::schemas_da;
use elster::{deklariere, erzeuge_xml, Deklaration, Felder, XmlOptionen};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

type Paare = Vec<(&'static str, Value)>;

/// Die fuenf Kapital-Felder des Ehegatten (`GESAMT_PARTNER_KAP`).
const KAP_PARTNER: [&str; 5] = [
    "kap_kapitalertraege_partner",
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige_partner",
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige_partner",
];
/// Die Kz der Kapital-Zeilen: ohne Betrag stehen sie in keinem der beiden Dokumente (Option A).
const KAP_KZ: [&str; 3] = ["E1900701", "E1900901", "E1901301"];

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

/// Die vollstaendige Akte `rentner.json` mit `mehr` obendrauf; ein Feld aus `mehr` ersetzt den Eintrag der Akte.
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

/// Lohn A, Zusammenveranlagung und Lohn B samt Stammdaten (wie `berufsverband_zeile_53_xml.rs`), ohne die fuenf Kapital-Felder.
fn paar() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(5_000_000)),
        ("steuerklasse", json!(1)),
        ("p36_lohnsteuer", json!(300_000)),
        ("lohn_mehrere_jahre", json!(false)),
        ("lohn_auslandstaetigkeit", json!(false)),
        ("lohn_erstattungen", json!(false)),
        ("stammdaten_nachname_partner", json!("Muster")),
        ("stammdaten_vorname_partner", json!("Erika")),
        ("stammdaten_geburtsdatum_partner", json!("01.01.1960")),
        ("geburtsjahr_partner", json!(1960)),
        ("kist_konfession_partner", json!("keine")),
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn_partner", json!(2_000_000)),
        ("steuerklasse_partner", json!(1)),
        ("p36_lohnsteuer_partner", json!(300_000)),
        ("lohn_mehrere_jahre_partner", json!(false)),
        ("lohn_auslandstaetigkeit_partner", json!(false)),
        ("lohn_erstattungen_partner", json!(false)),
    ]
}

/// Fall X: Kreuz „keine Kapitalertraege des Partners“ bestaetigt (`kein_kap_partner = true`), die fuenf Felder fehlen.
fn fall_x() -> Paare {
    let mut p = paar();
    p.push(("kein_kap_partner", json!(true)));
    p
}

/// Fall Y: Kreuz „Kapitalertraege des Partners“ gesetzt (`kein_kap_partner = false`), die fuenf Felder mit 0 beantwortet.
fn fall_y() -> Paare {
    let mut p = paar();
    p.push(("kein_kap_partner", json!(false)));
    p.extend(KAP_PARTNER.map(|f| (f, json!(0))));
    p
}

/// Wie die Produktion (`einreichungs_xml`, `api::deklaration`): Ring, dann `deklariere`.
fn deklaration(mehr: &[(&'static str, Value)]) -> (Deklaration, Felder) {
    let (mut felder, _) = akte(mehr).materialisiere(None).unwrap();
    mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    (d, felder)
}

fn xml_von(d: &Deklaration, felder: &Felder) -> String {
    erzeuge_xml(
        d,
        &XmlOptionen {
            vz: 2025,
            empfaenger_land: "BY".to_owned(),
            hersteller_id: Some("74931".to_owned()),
            abgabefaehig: true,
            snapshot: Some(felder),
            ..XmlOptionen::default()
        },
    )
    .unwrap()
}

/// Die Namen der Felder, die die Erklaerung nicht traegt.
fn nicht_deklariert(d: &Deklaration) -> Vec<&str> {
    d.nicht_deklariert.iter().map(|e| e.feld_id.as_str()).collect()
}

#[test]
fn die_erklaerung_ohne_die_fuenf_felder_ist_die_des_paares_mit_fuenf_nullen() {
    let (x, fx) = deklaration(&fall_x());
    let (y, fy) = deklaration(&fall_y());
    eprintln!("MESSUNG X nicht_deklariert: {:?}", nicht_deklariert(&x));
    eprintln!("MESSUNG Y nicht_deklariert: {:?}", nicht_deklariert(&y));
    eprintln!("MESSUNG X unvollstaendig: {:?}, Y unvollstaendig: {:?}", x.unvollstaendig(), y.unvollstaendig());
    eprintln!("MESSUNG X luecken: {:?}, Y luecken: {:?}", x.pflichtfelder_luecken(), y.pflichtfelder_luecken());

    assert!(x.unvollstaendig().is_empty(), "Fall X ist unvollstaendig: {:?}", x.unvollstaendig());
    assert_eq!(x.pflichtfelder_luecken(), y.pflichtfelder_luecken(), "die Pflichtluecken beider Faelle");
    assert_eq!(x.deklaration, y.deklaration, "Kz-Werte der Person A und der Basis");
    assert_eq!(x.person_b, y.person_b, "Kz-Werte der Person B");
    for kz in KAP_KZ {
        assert!(!x.person_b.contains_key(kz) && !y.person_b.contains_key(kz), "{kz} steht in der Anlage KAP des Ehegatten");
    }

    if !schemas_da(2025) {
        return;
    }
    let (xml_x, xml_y) = (xml_von(&x, &fx), xml_von(&y, &fy));
    assert_eq!(xml_x, xml_y, "das erzeugte XML beider Faelle");
    for kz in KAP_KZ {
        assert!(!xml_x.contains(kz), "{kz} steht im XML");
    }
}
