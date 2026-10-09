//! Unfallkosten im ERZEUGTEN XML (Abweichung Nr. 48, Punkt 8 der Entscheidungen vom 2026-10-08): der Betrag steht unter
//! `N/Wk/Weitere_Wk/Sonst` (`E0205405` Bezeichnung, `E0205406` Betrag in volle Euro, aufgerundet) und in der Summe `Sum`
//! (`E0204803`). Die Abgabe bleibt gesperrt (`UNFALLKOSTEN_SPERRE`), bis `checkESt` die Zeile einmal angenommen hat;
//! dieser Test belegt nur, was OHNE ELSTER belegbar ist: Betrag, Ort im Dokument, Rundung, Schema.
//!
//! Die Tests lesen das XML, das `erzeuge_xml` aus der Deklaration macht, nicht nur `deklaration`: ein Betrag in der
//! Deklaration, den der Schreiber nicht ins Dokument setzt, bliebe sonst grün. Die Sperre hält das Produkt davon ab, dieses
//! XML zu bauen (`erzeuge_xml` lehnt jede Deklaration mit offenem Eintrag ab); `xml_ohne_sperre` beschreibt den Umweg des
//! Tests. Die Kontrolle am Ende belegt, dass
//! `xmllint` einen falschen Betrag in dieser Zeile wirklich ablehnt, sonst belegte das Schema-Grün nichts.
//!
//! Die Zeilen 62 bis 64 der Anlage N 2025 (Anleitung `anl_n_2025.txt:220-228`, `:284`) sind `Sonst`, `Sonst`, `Sum`;
//! dass `E0205405`/`E0205406` die Zeilen 62/63 sind, folgt aus Schema und Vordruck (`derived`), nicht aus einem Kz-Druck.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::testhilfe::index;
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Vz, Zustand};
use elster::testhilfe::schemas_da;
use elster::{deklariere, erzeuge_xml, validiere_xsd_text, Deklaration, Felder, XmlOptionen};
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

/// Wie in `unfallkosten_einreichung_hermetisch.rs`: Arbeitsweg 30 km, 220 Tage, Kfz; `unfall` in CENT.
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

fn deklaration(unfall: Option<i64>) -> (Deklaration, Felder) {
    let s = akte(unfall);
    let (felder, _) = s.materialisiere(None).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    (d, felder)
}

fn xml_von(d: &Deklaration, felder: &Felder) -> String {
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        snapshot: Some(felder),
        ..XmlOptionen::default()
    };
    erzeuge_xml(d, &opt).unwrap()
}

const UNFALL_KZ: [&str; 3] = ["E0205405", "E0205406", "E0204803"];

/// Das XML, das die Deklaration MIT Unfallkosten ergäbe, wäre die Sperre nicht da. `erzeuge_xml` lehnt jede Deklaration
/// mit offenem Eintrag ab ("kein Submission-XML"), also auch diese: die Sperre hält das Produkt davon ab, das XML
/// überhaupt zu bauen. Darum nimmt der Test die Deklaration der Akte OHNE Unfallkosten (keine Sperre) und setzt genau die
/// drei Kz hinein, die `deklariere` für den Betrag berechnet hat. Zwei Prüfungen halten das ehrlich: außer diesen Kz
/// unterscheiden sich beide Deklarationen in nichts, und der Betrag kommt aus dem Store über `deklariere`, nicht aus dem Test.
fn xml_ohne_sperre(cent: i64) -> String {
    let (mit, felder) = deklaration(Some(cent));
    let (mut sauber, _) = deklaration(None);
    for kz in UNFALL_KZ {
        let wert = mit
            .deklaration
            .get(kz)
            .unwrap_or_else(|| panic!("{cent} Cent: `deklariere` setzt {kz} nicht"));
        sauber.deklaration.insert(kz.to_owned(), wert.clone());
    }
    assert_eq!(
        mit.deklaration, sauber.deklaration,
        "{cent} Cent: die Deklaration unterscheidet sich in mehr als den drei Kz"
    );
    xml_von(&sauber, &felder)
}

/// Der Text zwischen `<tag>` und `</tag>`, das erste Vorkommen.
fn tag_text<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let auf = format!("<{tag}>");
    let zu = format!("</{tag}>");
    let von = xml.find(&auf)? + auf.len();
    let bis = xml[von..].find(&zu)? + von;
    Some(&xml[von..bis])
}

/// Die Zeilen "weitere Werbungskosten" der Anlage N im erzeugten XML.
fn weitere_wk(xml: &str) -> Option<&str> {
    tag_text(xml, "Weitere_Wk")
}

fn pruefe_schema(xml: &str) {
    if !schemas_da(2025) {
        return;
    }
    let (ok, meldung) = validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
    assert!(ok, "xmllint gegen E10-2025.xsd: {meldung}");
}

/// Der Betrag steht im ERZEUGTEN XML unter `Sonst`, aufgerundet auf volle Euro, und in `Sum`; das Schema nimmt ihn.
#[test]
fn unfallkosten_stehen_im_erzeugten_xml_unter_weitere_wk_sonst_und_in_der_summe() {
    // `erzeuge_xml` verlangt das Schema; ohne ERiC-Auslieferung (CI) gibt es kein XML zu lesen.
    if !schemas_da(2025) {
        return;
    }
    for (cent, euro) in [(150_000, "1500"), (150_001, "1501"), (1, "1")] {
        let xml = xml_ohne_sperre(cent);
        let block = weitere_wk(&xml).unwrap_or_else(|| panic!("{cent} Cent: kein <Weitere_Wk> im XML"));
        let sonst = tag_text(block, "Sonst").unwrap_or_else(|| panic!("{cent} Cent: kein <Sonst>: {block}"));
        assert_eq!(
            tag_text(sonst, "E0205406"),
            Some(euro),
            "{cent} Cent: Betrag in E0205406 ({sonst})"
        );
        let text = tag_text(sonst, "E0205405").unwrap_or_default();
        assert!(text.contains("Unfall"), "{cent} Cent: Bezeichnung `{text}`");
        let summe = tag_text(block, "Sum").unwrap_or_else(|| panic!("{cent} Cent: kein <Sum>: {block}"));
        assert_eq!(
            tag_text(summe, "E0204803"),
            Some(euro),
            "{cent} Cent: Summe der weiteren Werbungskosten ({summe})"
        );
        assert!(
            block.find("<Sonst>") < block.find("<Sum>"),
            "{cent} Cent: Schema-Reihenfolge Sonst vor Sum: {block}"
        );
        pruefe_schema(&xml);
    }
}

/// Die Sperre bleibt (Julius, 2026-10-08): der Betrag steht im XML, die Abgabe bleibt gesperrt.
#[test]
fn die_sperre_bleibt_trotz_betrag_im_xml() {
    for cent in [150_000, 1] {
        let (d, felder) = deklaration(Some(cent));
        let fehler = erzeuge_xml(
            &d,
            &XmlOptionen {
                hersteller_id: Some("74931".to_owned()),
                snapshot: Some(&felder),
                ..XmlOptionen::default()
            },
        )
        .expect_err("die Sperre hält das XML zurück");
        assert!(fehler.0.contains(FELD), "{cent} Cent: XmlFehler `{}`", fehler.0);
        let luecken = d.unvollstaendig();
        let grund = luecken
            .iter()
            .find(|e| e.feld_id == FELD)
            .unwrap_or_else(|| panic!("{cent} Cent: die Sperre ist weg"));
        assert!(
            grund.grund.contains("gesperrt"),
            "{cent} Cent: Sperrgrund `{}`",
            grund.grund
        );
    }
}

/// Ohne Unfallkosten und mit 0 Cent bleibt das XML wie vorher: kein `Sonst`, keine Summe, dieselbe Deklaration.
#[test]
fn ohne_unfallkosten_und_mit_null_steht_keine_zeile_im_xml() {
    if !schemas_da(2025) {
        return;
    }
    let (leer, felder) = deklaration(None);
    let (null, _) = deklaration(Some(0));
    for (name, d) in [("nicht gesetzt", &leer), ("0 Cent", &null)] {
        for kz in ["E0205405", "E0205406", "E0204803"] {
            assert!(!d.deklaration.contains_key(kz), "{name}: {kz} steht in der Deklaration");
        }
        assert!(d.unvollstaendig().iter().all(|e| e.feld_id != FELD), "{name}: Sperre ohne Betrag");
    }
    assert_eq!(leer.deklaration, null.deklaration, "0 Cent ändert die Deklaration");
    let xml = xml_von(&leer, &felder);
    assert!(!xml.contains("<Sonst>"), "<Sonst> ohne Unfallkosten im XML");
    assert!(!xml.contains("E0204803"), "Summe E0204803 ohne Unfallkosten im XML");
}

/// KONTROLLE: dieselbe Zeile mit einem Betrag, den das Schema sicher ablehnt ("abc" statt Ganzzahl), muss `xmllint` rot
/// melden. Sonst belegte das Schema-Grün oben nichts.
#[test]
fn kontrolle_xmllint_lehnt_einen_falschen_betrag_in_sonst_ab() {
    if !schemas_da(2025) {
        return;
    }
    let (mut d, felder) = deklaration(None);
    d.deklaration
        .insert("E0205405".to_owned(), json!("Unfallkosten Arbeitsweg"));
    d.deklaration.insert("E0205406".to_owned(), json!("abc"));
    let xml = xml_von(&d, &felder);
    let (ok, meldung) = validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
    assert!(!ok, "xmllint nahm einen Betrag `abc` an");
    assert!(meldung.contains("E0205406"), "falsche Ablehnung: {meldung}");
}
