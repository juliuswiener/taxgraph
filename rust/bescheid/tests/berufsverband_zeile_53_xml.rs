//! Beitraege zu Berufsverbaenden im ERZEUGTEN XML (Abweichung Nr. 93, § 9 Abs. 1 S. 3 Nr. 3 `EStG`, Zeile 53 der Anlage N): der Betrag
//! steht unter `N/Wk/Berufsverb/Einz` (`E0204001` Bezeichnung, `E0204003` Betrag in volle Euro, aufgerundet) und in der Summe `Sum`
//! (`E0204002`). Person A steht im ersten `<N>`, der Ehegatte im zweiten. Eine Sperre gibt es nicht: die Zeile ist ein Betrag, den
//! Bescheid und Erklaerung gleich tragen.
//!
//! Die Tests lesen das XML, das `erzeuge_xml` aus der Deklaration macht, nicht nur `deklaration`: ein Betrag in der Deklaration, den
//! der Schreiber nicht ins Dokument setzt, bliebe sonst gruen. Die Kontrolle am Ende belegt, dass `xmllint` einen Betrag mit sechs
//! Stellen (`GanzzahlOhneFuehrNull_MaxVK5`) wirklich ablehnt: sie ist der Grund fuer die Obergrenze 99.999 Euro in der Bindung, und
//! ohne sie belegte das Schema-Gruen oben nichts. Der Test belegt nur, was OHNE ELSTER belegbar ist: Betrag, Ort, Rundung, Schema.
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
use elster::{deklariere, erzeuge_xml, validiere_xsd_text, Deklaration, Felder, XmlOptionen};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const A: &str = "berufsverband_beitraege";
const B: &str = "berufsverband_beitraege_partner";
const KZ: [&str; 3] = ["E0204001", "E0204002", "E0204003"];

type Paare = Vec<(&'static str, Value)>;

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

/// Die vollstaendige Akte `rentner.json` (einzeln veranlagt, `checkESt` rc=0 ohne Zusatz) mit `mehr` obendrauf. Ein Feld aus `mehr`,
/// das die Akte schon fuehrt (`veranlagung`), ersetzt den Eintrag der Akte: der Store lehnt ein zweites aktives Event ab.
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

/// Der Lohnteil einer Person: Zeile 5, Steuerklasse und Lohnsteuer, dazu die drei Rueckfragen zum Lohn mit Nein (seit Abweichung Nr. 85
/// sperrt eine fehlende Antwort bei einem Lohn ueber 0; diese Datei misst die Zeile im XML, nicht diese Sperre).
fn lohn(euro: i64) -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(euro * 100)),
        ("steuerklasse", json!(1)),
        ("p36_lohnsteuer", json!(300_000)),
        ("lohn_mehrere_jahre", json!(false)),
        ("lohn_auslandstaetigkeit", json!(false)),
        ("lohn_erstattungen", json!(false)),
    ]
}

/// Der Ehegatte als Person B: Zusammenveranlagung, Stammdaten, Kapital-Nullen und Lohnteil.
fn ehegatte(lohn_euro: i64) -> Paare {
    vec![
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
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn_partner", json!(lohn_euro * 100)),
        ("steuerklasse_partner", json!(1)),
        ("p36_lohnsteuer_partner", json!(300_000)),
        ("lohn_mehrere_jahre_partner", json!(false)),
        ("lohn_auslandstaetigkeit_partner", json!(false)),
        ("lohn_erstattungen_partner", json!(false)),
    ]
}

fn mit(mut a: Paare, b: Paare) -> Paare {
    a.extend(b);
    a
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

/// Das XML der Akte auf dem Weg des Produkts; die Deklaration sperrt nicht.
fn xml_der_akte(mehr: &[(&'static str, Value)]) -> String {
    let (d, felder) = deklaration(mehr);
    assert!(d.unvollstaendig().is_empty(), "die Akte sperrt: {:?}", d.unvollstaendig());
    xml_von(&d, &felder)
}

/// Der Text zwischen `<tag>` und `</tag>`, das erste Vorkommen.
fn tag_text<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let auf = format!("<{tag}>");
    let zu = format!("</{tag}>");
    let von = xml.find(&auf)? + auf.len();
    let bis = xml[von..].find(&zu)? + von;
    Some(&xml[von..bis])
}

/// Alle `<tag>...</tag>` in der Reihenfolge des Dokuments.
fn alle_tags<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let mut rest = xml;
    let mut gefunden = Vec::new();
    while let Some(t) = tag_text(rest, tag) {
        gefunden.push(t);
        let ende = rest.find(&format!("</{tag}>")).unwrap() + tag.len() + 3;
        rest = &rest[ende..];
    }
    gefunden
}

fn pruefe_schema(xml: &str, name: &str) {
    let (ok, meldung) = validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
    assert!(ok, "{name}: xmllint gegen E10-2025.xsd: {meldung}");
}

/// Der Block `Berufsverb` einer Anlage N: `(Bezeichnung, Betrag, Summe)`, oder `None`, wenn er fehlt.
fn zeile(n: &str) -> Option<(String, String, String)> {
    let block = tag_text(n, "Berufsverb")?;
    let einz = tag_text(block, "Einz").unwrap_or_else(|| panic!("kein <Einz> im Block: {block}"));
    assert!(block.find("<Einz>") < block.find("<Sum>"), "Schema-Reihenfolge Einz vor Sum: {block}");
    let hole = |t: &str, wo: &str| tag_text(wo, t).unwrap_or_else(|| panic!("{t} fehlt: {block}")).to_owned();
    Some((hole("E0204001", einz), hole("E0204003", einz), hole("E0204002", block)))
}

/// AK2: das ERZEUGTE XML traegt die Zeile 53 in der Anlage N der Person, aufgerundet, mit Bezeichnung und Summe: Person A allein im
/// einen `<N>`, der Ehegatte allein im zweiten, beide zusammen in beiden; das Schema nimmt jedes Dokument. Ohne ERiC-Schema (CI)
/// entfaellt der Test, weil `erzeuge_xml` es verlangt.
#[test]
fn die_zeile_53_steht_je_person_in_ihrer_anlage_n_und_das_schema_nimmt_sie() {
    if !schemas_da(2025) {
        return;
    }
    let nur_a = mit(lohn(50_000), vec![(A, json!(150_001))]);
    let xml = xml_der_akte(&nur_a);
    let n = alle_tags(&xml, "N");
    assert_eq!(n.len(), 1, "Person A allein: ein <N>");
    let (text, betrag, summe) = zeile(n[0]).expect("Person A: Zeile 53 fehlt");
    assert!(text.contains("Berufsverb"), "Bezeichnung `{text}`");
    assert_eq!((betrag.as_str(), summe.as_str()), ("1501", "1501"), "150.001 Cent steigen auf 1501 Euro");
    pruefe_schema(&xml, "Person A");

    let nur_b = mit(mit(lohn(30_000), ehegatte(20_000)), vec![(B, json!(17_800))]);
    let xml = xml_der_akte(&nur_b);
    let n = alle_tags(&xml, "N");
    assert_eq!(n.len(), 2, "Zusammenveranlagung: zwei <N>");
    assert_eq!(zeile(n[0]), None, "Person A hat keinen Beitrag");
    let (_, betrag, summe) = zeile(n[1]).expect("Ehegatte: Zeile 53 fehlt");
    assert_eq!((betrag.as_str(), summe.as_str()), ("178", "178"), "Ehegatte");
    pruefe_schema(&xml, "Ehegatte");

    let beide = mit(mit(lohn(50_000), ehegatte(40_001)), vec![(A, json!(120_000)), (B, json!(60_001))]);
    let xml = xml_der_akte(&beide);
    let n = alle_tags(&xml, "N");
    assert_eq!(n.len(), 2, "beide: zwei <N>");
    assert_eq!(zeile(n[0]).map(|z| z.1), Some("1200".to_owned()), "Person A");
    assert_eq!(zeile(n[1]).map(|z| z.1), Some("601".to_owned()), "Ehegatte, 600,01 Euro steigen auf 601");
    pruefe_schema(&xml, "beide");
}

/// Ohne Beitrag und mit 0 bleibt das XML wie vorher: kein `Berufsverb`, keines der drei Kz, dieselbe Deklaration.
#[test]
fn ohne_beitrag_und_mit_null_steht_keine_zeile_im_xml() {
    if !schemas_da(2025) {
        return;
    }
    let ohne = mit(lohn(50_000), ehegatte(40_001));
    let null = mit(ohne.clone(), vec![(A, json!(0)), (B, json!(0))]);
    let (leer, felder) = deklaration(&ohne);
    let (nichts, _) = deklaration(&null);
    for (name, d) in [("nicht gesetzt", &leer), ("0 Cent", &nichts)] {
        for kz in KZ {
            assert!(!d.deklaration.contains_key(kz), "{name}: {kz} steht in der Deklaration von Person A");
            assert!(!d.person_b.contains_key(kz), "{name}: {kz} steht in der Deklaration des Ehegatten");
        }
    }
    assert_eq!(leer.deklaration, nichts.deklaration, "0 Cent aendert die Deklaration von Person A");
    assert_eq!(leer.person_b, nichts.person_b, "0 Cent aendert die Deklaration des Ehegatten");
    let xml = xml_von(&leer, &felder);
    assert!(!xml.contains("Berufsverb"), "<Berufsverb> ohne Beitrag im XML");
    for kz in KZ {
        assert!(!xml.contains(kz), "{kz} ohne Beitrag im XML");
    }
}

/// KONTROLLE: dieselbe Zeile mit einem Betrag von sechs Stellen (100.000 Euro) lehnt `xmllint` ab, ebenso `abc`. Der Test belegt die
/// Obergrenze 99.999 Euro in der Bindung und dass das Schema-Gruen oben etwas pruefte.
#[test]
fn kontrolle_xmllint_lehnt_einen_betrag_ueber_fuenf_stellen_ab() {
    if !schemas_da(2025) {
        return;
    }
    let (mut d, felder) = deklaration(&mit(lohn(50_000), vec![(A, json!(9_999_900))]));
    let xml = xml_von(&d, &felder);
    pruefe_schema(&xml, "99.999 Euro");
    for falsch in [json!(100_000), json!("abc")] {
        d.deklaration.insert("E0204003".to_owned(), falsch.clone());
        d.deklaration.insert("E0204002".to_owned(), falsch.clone());
        let xml = xml_von(&d, &felder);
        let (ok, meldung) = validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
        assert!(!ok, "xmllint nahm den Betrag {falsch} an");
        assert!(meldung.contains("E0204003"), "falsche Ablehnung fuer {falsch}: {meldung}");
    }
}
