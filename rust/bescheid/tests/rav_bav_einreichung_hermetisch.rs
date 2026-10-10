//! Rente mit Leistungsmitteilung (Abweichung Nr. 55, Anlage R-AV/bAV): was die Erklaerung traegt. Standardlauf, ohne `PARITY=1`, ohne
//! Python.
//!
//! **Worum es geht.** Der Bescheid rechnet die Betraege der Nummern 1, 4 und 5 (`rav_bav_ring_hermetisch.rs`). Die Erklaerung muss sie in
//! `RAV_bAV/Person/Leist_AV_betr/Einz` tragen, eine Mitteilung je `Einz`, die Kz in der Reihenfolge des Schemas: Zeile 4 `E1803101`,
//! Zeile 10 `E1803301` mit Beginn `E1803401` (Zeile 12), Zeile 15 `E1803501` mit Beginn `E1803601` (Zeile 16).
//!
//! **Warum es zaehlt.** Traegt die Erklaerung die Rente nicht, weicht sie vom Bescheid ab; legt der Schreiber die zweite Mitteilung als
//! zweites `<RAV_bAV>` an, ist das die Anlage der Person B, und `ERiC` lehnt die Einzelveranlagung ab (Spike Nr. 55, Fall K7).
//!
//! **Wo es sitzt.** `elster/src/deklaration.rs` (`rav_bav_feld`: ein Betrag unter einem Euro und ein Beginn ohne Betrag fallen weg),
//! `elster/src/xml.rs` (`INSTANZ_CONTAINER_TIEFER`: die Gruppe `rente_av_bav` steht unter `Einz`). Der Test geht ueber
//! `einreichungs_xml`, den Weg der Abgabe ohne `ERiC`; die Akte ist `rust/fixtures/e2e/gesamt.json` (Scheibe `gesamt`, geboren
//! 12.03.1985, `kein_sonstige` = ja). Die Abnahme durch `ERiC` selbst steht in `checkest_blockmatrix.rs`.
//!
//! Erwartete Werte von Hand: die Betraege sind volle Euro (Cent abgerundet), die Kz stehen im Vordruck
//! (`sources/bfinv/rav_bav_2025.txt`, Zeilen 4, 10, 12, 15, 16). Ohne das Schema (CI) entfallen die XML-Aussagen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use bescheid::deklaration::{einreichungs_xml, EinreichFehler, Einreichung};
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Sperrgrund, Vz, Zustand};
use elster::testhilfe::schemas_da;
use elster::validiere_xsd_text;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

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

fn name(basis: &str, n: u8) -> String {
    if n == 1 {
        basis.to_owned()
    } else {
        format!("{basis}__{n}")
    }
}

/// Eine vollstaendige Mitteilung der Instanz `n`: Betraege in Cent, Beginn der Nr. 4 am 01.04.2022, der Nr. 5 am 01.06.2020, keine
/// weiteren Nummern, keine Basisrente.
fn mitteilung(n: u8, nr1: i64, nr4: i64, nr5: i64) -> Vec<(String, Value)> {
    vec![
        (name("rav_bav_nr1_betrag", n), json!(nr1)),
        (name("rav_bav_nr4_betrag", n), json!(nr4)),
        (name("rav_bav_nr4_beginn", n), json!("01.04.2022")),
        (name("rav_bav_nr5_betrag", n), json!(nr5)),
        (name("rav_bav_nr5_beginn", n), json!("01.06.2020")),
        (name("rav_bav_andere_nummern", n), json!(false)),
        (name("rav_bav_basisrente", n), json!(false)),
    ]
}

/// Die vollstaendige Akte mit dem Kreuz `kein_rav_bav` (`false`: "ich habe eine Rente mit Leistungsmitteilung") und den Mitteilungen.
fn akte(kreuz: bool, felder: Vec<(String, Value)>) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    setze(&mut s, "kein_rav_bav", json!(kreuz));
    for (feld, wert) in felder {
        setze(&mut s, &feld, wert);
    }
    s
}

fn lauf(s: &Store) -> Result<Einreichung, EinreichFehler> {
    einreichungs_xml(s, index(), params(), "BY", Some("74931".to_owned()))
}

/// Die Erklaerung der Akte; ohne Schema (CI) gibt es keine, dann `None`.
fn einreichung(s: &Store) -> Option<Einreichung> {
    let r = lauf(s);
    if !schemas_da(2025) {
        return None;
    }
    Some(r.unwrap_or_else(|e| panic!("KONTROLLE: erwartet eine Erklaerung, bekommen {e:?}")))
}

/// Die Kz der Zeilen 4, 10, 12, 15 und 16 der Anlage R-AV/bAV.
const KZ_RAV_BAV: [&str; 5] = ["E1803101", "E1803301", "E1803401", "E1803501", "E1803601"];

/// Die Kz der Instanz `index` (1-basiert) der Gruppe `rente_av_bav`: Instanz 1 steht in `deklaration`, ab 2 in den Anlage-Instanzen.
fn kz_der_instanz(e: &Einreichung, index: u64) -> BTreeMap<String, Value> {
    if index == 1 {
        return KZ_RAV_BAV
            .iter()
            .filter_map(|kz| {
                e.deklaration
                    .deklaration
                    .get(*kz)
                    .map(|v| ((*kz).to_owned(), v.clone()))
            })
            .collect();
    }
    e.deklaration
        .anlage_instanzen
        .iter()
        .filter(|(g, _)| g == "rente_av_bav")
        .flat_map(|(_, ii)| ii)
        .find(|i| i.index == index)
        .map(|i| i.felder.clone())
        .unwrap_or_default()
}

/// Der Text zwischen `<tag>` und `</tag>` (erstes Vorkommen).
fn block<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let von = xml.find(&format!("<{tag}>"))? + tag.len() + 2;
    let bis = von + xml[von..].find(&format!("</{tag}>"))?;
    Some(&xml[von..bis])
}

fn schema_nimmt_es(xml: &str) {
    let (ok, meldung) = validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
    assert!(ok, "xmllint gegen E10-2025.xsd: {meldung}");
}

/// Eine Mitteilung mit allen drei Nummern steht mit den Kz ihrer Zeilen in der Erklaerung, die Betraege in vollen Euro (Cent
/// abgerundet: 6.000,99 Euro stehen als 6.000), und das Schema nimmt das XML an.
#[test]
fn eine_mitteilung_steht_mit_den_kz_ihrer_zeilen_in_der_erklaerung() {
    let s = akte(false, mitteilung(1, 600_099, 300_000, 1_000_000));
    let Some(e) = einreichung(&s) else { return };
    assert_eq!(
        kz_der_instanz(&e, 1),
        BTreeMap::from([
            ("E1803101".to_owned(), json!(6_000)),
            ("E1803301".to_owned(), json!(3_000)),
            ("E1803401".to_owned(), json!("01.04.2022")),
            ("E1803501".to_owned(), json!(10_000)),
            ("E1803601".to_owned(), json!("01.06.2020")),
        ])
    );
    let rav = block(&e.xml, "RAV_bAV").expect("KONTROLLE: kein <RAV_bAV> im XML");
    assert_eq!(e.xml.matches("<RAV_bAV>").count(), 1);
    assert_eq!(rav.matches("<Einz>").count(), 1);
    assert!(rav.contains("<Person>PersonA</Person>"), "{rav}");
    // Die Reihenfolge im <Einz> ist die des Schemas: Zeile 4, 10, 12, 15, 16.
    let stellen: Vec<usize> = ["E1803101", "E1803301", "E1803401", "E1803501", "E1803601"]
        .iter()
        .map(|kz| {
            rav.find(&format!("<{kz}>"))
                .unwrap_or_else(|| panic!("{kz} fehlt in {rav}"))
        })
        .collect();
    assert!(
        stellen.is_sorted(),
        "Reihenfolge der Kz: {stellen:?} in {rav}"
    );
    schema_nimmt_es(&e.xml);
}

/// Zwei Mitteilungen stehen als zwei `<Einz>` unter EINEM `<Leist_AV_betr>` der Person A. Ohne den Eintrag in
/// `INSTANZ_CONTAINER_TIEFER` legte die zweite ein zweites `<RAV_bAV>` an, das ist die Anlage der Person B.
#[test]
fn zwei_mitteilungen_stehen_als_zwei_einz_in_derselben_anlage() {
    let m = [mitteilung(1, 600_000, 0, 0), mitteilung(2, 0, 200_000, 0)].concat();
    let Some(e) = einreichung(&akte(false, m)) else {
        return;
    };
    assert_eq!(
        kz_der_instanz(&e, 1),
        BTreeMap::from([("E1803101".to_owned(), json!(6_000))])
    );
    assert_eq!(
        kz_der_instanz(&e, 2),
        BTreeMap::from([
            ("E1803301".to_owned(), json!(2_000)),
            ("E1803401".to_owned(), json!("01.04.2022")),
        ])
    );
    assert_eq!(
        e.xml.matches("<RAV_bAV>").count(),
        1,
        "genau eine Anlage R-AV/bAV"
    );
    assert_eq!(e.xml.matches("<Leist_AV_betr>").count(), 1);
    let rav = block(&e.xml, "RAV_bAV").unwrap();
    assert_eq!(rav.matches("<Einz>").count(), 2, "{rav}");
    assert!(
        !e.xml.contains("PersonB"),
        "die zweite Mitteilung ist keine Anlage der Person B"
    );
    schema_nimmt_es(&e.xml);
}

/// Ein Betrag 0, ein Betrag unter einem Euro und ein Beginn ohne Betrag stehen nicht in der Erklaerung: `ERiC` verlangt Betrag und
/// Beginn gemeinsam. Eine Mitteilung ohne jeden Betrag ueber 0 laesst die Anlage ganz weg.
#[test]
fn nullbetraege_und_ein_beginn_ohne_betrag_stehen_nicht_in_der_erklaerung() {
    // Zeile 4 traegt Geld, Zeile 10 und 15 sind 0 beziehungsweise 99 Cent: nur Zeile 4 steht da, ohne Beginn.
    let Some(e) = einreichung(&akte(false, mitteilung(1, 600_000, 0, 99))) else {
        return;
    };
    assert_eq!(
        kz_der_instanz(&e, 1),
        BTreeMap::from([("E1803101".to_owned(), json!(6_000))])
    );
    assert!(
        !e.xml.contains("E1803401") && !e.xml.contains("E1803601"),
        "Beginn ohne Betrag im XML"
    );
    schema_nimmt_es(&e.xml);
    // Nur Nullen: keine Anlage.
    let Some(e) = einreichung(&akte(false, mitteilung(1, 0, 0, 0))) else {
        return;
    };
    assert!(kz_der_instanz(&e, 1).is_empty());
    assert!(
        !e.xml.contains("<RAV_bAV>"),
        "eine Anlage ohne Betrag steht nicht im XML"
    );
}

/// Das Kreuz "keine Rente mit Leistungsmitteilung" mit eingetragenen Betraegen ist ein Widerspruch: die Abgabe sperrt VOR der
/// Erklaerung, die Betraege fallen nicht still weg.
#[test]
fn ein_betrag_trotz_kreuz_keine_rente_sperrt_vor_der_erklaerung() {
    match lauf(&akte(true, mitteilung(1, 600_000, 0, 0))) {
        Err(EinreichFehler::Gesperrt(g)) => assert_eq!(g, Sperrgrund::RavBavAngabenOffen),
        anders => panic!("Kreuz keine Rente mit Betrag: {anders:?}"),
    }
    // Gegenprobe: dieselbe Akte ohne Betrag sperrt nicht aus diesem Grund.
    let r = lauf(&akte(true, mitteilung(1, 0, 0, 0)));
    assert!(
        !matches!(
            r,
            Err(EinreichFehler::Gesperrt(Sperrgrund::RavBavAngabenOffen))
        ),
        "{r:?}"
    );
}
