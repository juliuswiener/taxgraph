//! Versorgungsbezuege und die Abgabe-Sperre (Abweichung Nr. 42, gehoben fuer die getragenen Faelle mit Nr. 64): die Naht
//! `einreichungs_xml` und `deklariere`. Ohne `PARITY=1`, ohne Python, ohne Netz, ohne `ERiC`.
//!
//! **Worum es geht.** Wer einen Versorgungsbezug (Pension) angibt, bekommt ihn im Bescheid verrechnet (Versorgungsfreibetrag,
//! Zuschlag, Pauschbetrag). Die ELSTER-Erklaerung traegt ihn seit Abweichung Nr. 50 in den Zeilen 11 bis 13 der Anlage N. Bis
//! Nr. 64 sperrte die Abgabe jeden bestaetigten Bezug ueber 0; Julius hat die Sperre am 2026-10-12 fuer die getragenen Faelle
//! gehoben.
//!
//! **Warum es zaehlt.** Ein Fall, den die Erklaerung nicht voll traegt, darf nicht still durchgehen: das Finanzamt saehe einen
//! Bezug ohne die Angaben, die der Vordruck dafuer verlangt. Es sperrt darum weiter, wo keine Zeile steht (Altersgrenze nicht
//! erreicht, Betrag oder Bemessungsgrundlage unter 1 Euro) und wo der Bezug im Veranlagungsjahr oder danach beginnt (Zeile 14).
//!
//! **Wo es sitzt.** `rust/elster/src/deklaration.rs::Bau::versorgung_sperre`. Der Ehegatte zaehlt nur bei bestaetigter
//! Zusammenveranlagung: nur dann zaehlt sein Bezug im Bescheid (Abweichung Nr. 33). Ein vorlaeufiger Betrag behaelt seinen
//! Grund "Pflicht-Bestaetigung fehlt". Die Faelle der Tabelle stehen in `versorgung_sperre_faelle.rs`; die HTTP-Naht
//! (`GET /deklaration`, `POST /einreichen`) prueft `rust/api/tests/versorgung_abgabe_sperre_hermetisch.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::deklaration::{einreichungs_xml, mit_ring_werten, EinreichFehler};
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Vz, Zustand};
use elster::deklariere;
use elster::testhilfe::schemas_da;
use serde_json::{json, Value};
use store::{Abweisung, AbweisungRoh, BindungNachschlag, NeuesEventRoh, Signal, Store};

const A: &str = "versorgung_jahresrente";
const B: &str = "versorgung_jahresrente_partner";

fn setze(s: &mut Store, feld: &str, wert: Value, zustand: Zustand) {
    let leer = HashMap::new();
    let bestaetigt = zustand == Zustand::Bestaetigt;
    let mut neu = NeuesEventRoh {
        feld_id: feld.to_owned(),
        wert: wert.into(),
        zustand,
        herkunft: HerkunftVektor::Voll(Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        }),
        schreiber: "ui:laie".to_owned(),
        signal: Signal {
            signal_1: Some(None),
            signal_2: bestaetigt.then(|| format!("ok@{feld}")),
            signal_2_fehlt: !bestaetigt,
        },
        signal_2_fremd: None,
        ersetzt: None,
        ts: Some("2026-09-01T00:00:00+00:00".to_owned()),
    };
    match s.append_roh(&neu, None, BindungNachschlag::neu(&leer)) {
        Ok(_) => {}
        Err(AbweisungRoh::Store(Abweisung::AktivesEventVorhanden { aktives_event, .. })) => {
            neu.ersetzt = Some(aktives_event.to_string());
            s.append_roh(&neu, None, BindungNachschlag::neu(&leer)).unwrap();
        }
        Err(e) => panic!("{feld}: {e:?}"),
    }
}

/// Die vollstaendige Akte `gesamt.json` (Stammdaten, Kegel) plus `mehr`, alles bestaetigt; `vorlaeufig` kommt als
/// vorlaeufige Events obendrauf.
fn akte(mehr: &[(&str, Value)], vorlaeufig: &[(&str, Value)]) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    for (f, w) in mehr {
        setze(&mut s, f, w.clone(), Zustand::Bestaetigt);
    }
    for (f, w) in vorlaeufig {
        setze(&mut s, f, w.clone(), Zustand::Vorlaeufig);
    }
    s
}

/// Kurzform fuer die Ausgabe: das XML nicht abdrucken.
fn kurz(r: &Result<String, EinreichFehler>) -> String {
    match r {
        Ok(x) => format!("Ok(XML, {} Byte)", x.len()),
        Err(e) => format!("Err({e:?})"),
    }
}

fn lauf(s: &Store) -> Result<String, EinreichFehler> {
    einreichungs_xml(s, index(), params(), "BY", Some("74931".to_owned())).map(|e| e.xml)
}

/// `(feld_id, grund)` je Eintrag der Sperrliste, wenn die Abgabe an `DeklarationUnvollstaendig` scheitert.
fn sperre(r: &Result<String, EinreichFehler>) -> Vec<(String, String)> {
    match r {
        Err(EinreichFehler::DeklarationUnvollstaendig(e)) => e
            .iter()
            .map(|x| (x.feld_id.clone(), x.grund.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// `(unvollstaendig, nicht_deklariert)` der Deklaration, je als `(feld_id, grund)`; nur Felder `versorgung_*`. Wie die
/// Produktion (`einreichungs_xml`): Ring, dann `deklariere`. Ohne den Ring stuende kein Ring-Wert im Snapshot, und die Sperre
/// "keine Zeile" griffe fuer jeden Bezug (Abweichung Nr. 64).
fn deklaration_der(s: &Store) -> (Vec<(String, String)>, Vec<String>) {
    let (mut felder, _) = s.materialisiere(None).unwrap();
    mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    let offen = d
        .unvollstaendig()
        .iter()
        .filter(|e| e.feld_id.starts_with("versorgung_"))
        .map(|e| (e.feld_id.clone(), e.grund.clone()))
        .collect();
    let nicht = d
        .nicht_deklariert
        .iter()
        .filter(|e| e.feld_id.starts_with("versorgung_"))
        .map(|e| e.feld_id.clone())
        .collect();
    (offen, nicht)
}

/// Person A: Bezug von `euro`, Bemessungsgrundlage gleich dem Bezug, Beginn 2020 (vor dem Veranlagungsjahr 2025),
/// beamtenrechtlich: ein Fall, den die Zeilen 11 bis 13 voll tragen.
fn versorgung_a(euro: i64) -> Vec<(&'static str, Value)> {
    vec![
        (A, json!(euro * 100)),
        ("versorgung_bemessungsgrundlage", json!(euro * 100)),
        ("versorgung_beginn_jahr", json!(2020)),
        ("versorgung_art", json!("beamtenrechtlich")),
    ]
}

fn versorgung_b(euro: i64) -> Vec<(&'static str, Value)> {
    vec![
        (B, json!(euro * 100)),
        ("versorgung_bemessungsgrundlage_partner", json!(euro * 100)),
        ("versorgung_beginn_jahr_partner", json!(2020)),
        ("versorgung_art_partner", json!("beamtenrechtlich")),
    ]
}

/// Dieselben Felder mit Beginn im Veranlagungsjahr 2025: die Erklaerung traegt Zeile 14 nicht, die Abgabe sperrt.
fn im_veranlagungsjahr(mut p: Vec<(&'static str, Value)>) -> Vec<(&'static str, Value)> {
    for (f, w) in &mut p {
        if f.starts_with("versorgung_beginn_jahr") {
            *w = json!(2025);
        }
    }
    p
}

fn mit(mut a: Vec<(&'static str, Value)>, b: Vec<(&'static str, Value)>) -> Vec<(&'static str, Value)> {
    a.extend(b);
    a
}

/// KONTROLLE: ohne Bezug sperrt nichts, und die Akte ist abgabefaehig (soweit das Schema lokal liegt). Ohne sie belegte das
/// Rot unten nichts: die Akte ist ohne Bezug einreichbar, mit Bezug nicht.
#[test]
fn kontrolle_ohne_bezug_sperrt_nichts() {
    let r = lauf(&akte(&[], &[]));
    assert!(sperre(&r).is_empty(), "{}", kurz(&r));
    assert_eq!(r.is_ok(), schemas_da(2025), "{}", kurz(&r));
}

/// KONTROLLE: ein Bezug von 0 sperrt nicht, fuer Person A nicht und, bei Zusammenveranlagung, fuer den Ehegatten nicht.
/// Faengt die Grenze `> 0` gegen `>= 0`.
#[test]
fn kontrolle_bezug_null_sperrt_nichts() {
    let a = akte(&versorgung_a(0), &[]);
    let r = lauf(&a);
    assert!(sperre(&r).is_empty(), "A mit 0: {}", kurz(&r));
    let beide = mit(
        mit(vec![("veranlagung", json!("zusammen"))], versorgung_a(0)),
        versorgung_b(0),
    );
    let (offen, nicht) = deklaration_der(&akte(&beide, &[]));
    assert!(offen.is_empty(), "A und B mit 0: {offen:?}");
    assert!(nicht.contains(&A.to_owned()) && nicht.contains(&B.to_owned()), "{nicht:?}");
}

/// AK1: Person A, bestaetigter Bezug ueber 0, den die Zeilen 11 bis 13 voll tragen (Beginn 2020, Alters-Gate erfuellt): die
/// Abgabe ist frei (kein Eintrag in `unvollstaendig`), der Betrag steht in Zeile 11 und nicht in `nicht_deklariert`. Ohne
/// Schema (CI) endet `einreichungs_xml` erst beim XML, nie bei der Sperre.
#[test]
fn ein_getragener_bezug_von_person_a_sperrt_die_abgabe_nicht() {
    let s = akte(&versorgung_a(30_000), &[]);
    let r = lauf(&s);
    assert!(sperre(&r).is_empty(), "der getragene Bezug sperrt: {}", kurz(&r));
    assert_eq!(r.is_ok(), schemas_da(2025), "{}", kurz(&r));
    let (offen, nicht) = deklaration_der(&s);
    assert!(offen.is_empty(), "{offen:?}");
    assert!(!nicht.contains(&A.to_owned()), "der Betrag steht in Zeile 11, nicht in nicht_deklariert: {nicht:?}");
}

/// AK1, Rest: Person A, Beginn im Veranlagungsjahr 2025 -> die Abgabe sperrt mit genau einem Eintrag fuer
/// `versorgung_jahresrente`, und der Grund nennt Zeile 14, die Sperre und das Formular. Die Zeilen 11 bis 13 stehen trotzdem in
/// der Deklaration (der Betrag ist nicht in `nicht_deklariert`): die Sperre haelt nur die Abgabe zurueck.
#[test]
fn ein_bezug_von_person_a_mit_beginn_im_veranlagungsjahr_sperrt_die_abgabe() {
    let s = akte(&im_veranlagungsjahr(versorgung_a(30_000)), &[]);
    let r = lauf(&s);
    let e = sperre(&r);
    assert_eq!(
        e.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
        [A],
        "genau ein Eintrag fuer den Bezug, erhalten {}",
        kurz(&r)
    );
    let grund = &e[0].1;
    assert!(
        grund.starts_with("Versorgungsbezüge über 0 Euro")
            && grund.contains("Zeile 14")
            && grund.contains("gesperrt")
            && grund.contains("Formular"),
        "der Grund nennt Bezug, Zeile 14, Sperre und Formular nicht: {grund}"
    );
    let (offen, nicht) = deklaration_der(&s);
    assert_eq!(offen.len(), 1, "{offen:?}");
    assert!(!nicht.contains(&A.to_owned()), "der Betrag steht in Zeile 11: {nicht:?}");
}

/// AK1, Rest: Person A, Alters-Gate nicht erfuellt (`altersgrenze_sonstige`, 62 Jahre bei Beginn): der Bezug steckt nur als
/// Arbeitslohn in Zeile 5, keine Zeile 11 bis 13 -> die Abgabe sperrt weiter, mit dem Grund "nicht in die Zeilen 11 bis 13", und
/// der Betrag bleibt mit Grund in `nicht_deklariert`. Die Kontrolle mit 63 Jahren ist frei.
#[test]
fn ohne_zeile_sperrt_die_abgabe_weiter() {
    let gate = |alter: i64| {
        let mut p = versorgung_a(30_000);
        for (f, w) in &mut p {
            if *f == "versorgung_art" {
                *w = json!("altersgrenze_sonstige");
            }
        }
        p.push(("versorgung_alter_bei_beginn", json!(alter)));
        akte(&p, &[])
    };
    let (offen, nicht) = deklaration_der(&gate(63));
    assert!(offen.is_empty(), "Kontrolle, 63 Jahre: Gate erfuellt, die Zeilen stehen: {offen:?}");
    assert!(!nicht.contains(&A.to_owned()), "{nicht:?}");

    let s = gate(62);
    let r = lauf(&s);
    let e = sperre(&r);
    assert_eq!(e.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(), [A], "{}", kurz(&r));
    assert!(
        e[0].1.contains("nicht in die Zeilen 11 bis 13") && e[0].1.contains("gesperrt") && e[0].1.contains("Formular"),
        "{}",
        e[0].1
    );
    let (_, nicht) = deklaration_der(&s);
    assert!(nicht.contains(&A.to_owned()), "ohne Zeile steht der Betrag in nicht_deklariert: {nicht:?}");
}

/// Faelle, die der Bescheid schon vor der Erklaerung sperrt, bleiben so (kein Fall der Abgabe-Sperre): ein Bezug ueber dem
/// Bruttoarbeitslohn (`VersorgungUeberLohn`), ein Bezug ohne Bemessungsgrundlage (`VersorgungsfreibetragOffen`) und ein Beginn
/// nach dem Veranlagungsjahr (`VersorgungsbeginnNachVz`). `einreichungs_xml` endet bei ihnen in `Gesperrt`, nicht in
/// `DeklarationUnvollstaendig`.
#[test]
fn der_bescheid_sperrt_bezug_ueber_lohn_ohne_bemessungsgrundlage_und_beginn_nach_dem_jahr_vor_der_erklaerung() {
    let gesperrt = |p: &[(&str, Value)]| match lauf(&akte(p, &[])) {
        Err(EinreichFehler::Gesperrt(g)) => format!("{g:?}"),
        r => panic!("erwartet Gesperrt, erhalten {}", kurz(&r)),
    };
    assert_eq!(gesperrt(&versorgung_a(9_000_000)), "VersorgungUeberLohn");
    let ohne_bmg: Vec<_> = versorgung_a(30_000)
        .into_iter()
        .filter(|(f, _)| *f != "versorgung_bemessungsgrundlage")
        .collect();
    assert_eq!(gesperrt(&ohne_bmg), "VersorgungsfreibetragOffen");
    let mut spaet = versorgung_a(30_000);
    for (f, w) in &mut spaet {
        if *f == "versorgung_beginn_jahr" {
            *w = json!(2030);
        }
    }
    assert_eq!(gesperrt(&spaet), "VersorgungsbeginnNachVz");
}

/// AK2: der Ehegatte zaehlt nur bei BESTAETIGTER Zusammenveranlagung. Ein getragener Bezug sperrt nicht, einer mit Beginn im
/// Veranlagungsjahr sperrt. Einzelveranlagung und eine vorlaeufige Antwort auf die Veranlagung sperren nie (sein Bezug zaehlt im
/// Bescheid dann nicht).
#[test]
fn der_bezug_des_ehegatten_zaehlt_nur_bei_bestaetigter_zusammenveranlagung() {
    let lohn_b = vec![("bruttoarbeitslohn_partner", json!(4_000_000))];
    let bezug = mit(lohn_b.clone(), versorgung_b(30_000));
    let zusammen = mit(vec![("veranlagung", json!("zusammen"))], bezug.clone());
    let (offen, nicht) = deklaration_der(&akte(&zusammen, &[]));
    assert!(offen.is_empty(), "zusammen, getragen: {offen:?}");
    assert!(!nicht.contains(&B.to_owned()), "der Betrag steht in Zeile 11 der zweiten Anlage N: {nicht:?}");

    let im_jahr = mit(vec![("veranlagung", json!("zusammen"))], mit(lohn_b.clone(), im_veranlagungsjahr(versorgung_b(30_000))));
    let (offen, _) = deklaration_der(&akte(&im_jahr, &[]));
    assert_eq!(
        offen.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
        [B],
        "zusammen, Beginn im Veranlagungsjahr: genau der Ehegatte-Betrag sperrt"
    );
    assert!(offen[0].1.contains("Zeile 14"), "{}", offen[0].1);

    let einzel = mit(
        vec![("veranlagung", json!("einzel"))],
        mit(lohn_b.clone(), im_veranlagungsjahr(versorgung_b(30_000))),
    );
    let (offen, nicht) = deklaration_der(&akte(&einzel, &[]));
    assert!(offen.is_empty(), "einzel: der Bezug des Ehegatten zaehlt nicht, also sperrt er nicht: {offen:?}");
    assert!(nicht.contains(&B.to_owned()), "{nicht:?}");

    let vorlaeufig = akte(&mit(lohn_b, im_veranlagungsjahr(versorgung_b(30_000))), &[("veranlagung", json!("zusammen"))]);
    let (offen, _) = deklaration_der(&vorlaeufig);
    assert!(offen.is_empty(), "zusammen nur vorlaeufig: {offen:?}");
}

/// AK3: ein vorlaeufiger Betrag behaelt seinen bisherigen Grund "Pflicht-Bestaetigung (Zwei-Signal) fehlt"; die neue Sperre
/// gilt nur fuer den bestaetigten Betrag. Genau ein Eintrag je Feld, kein zweiter mit dem neuen Grund.
#[test]
fn ein_vorlaeufiger_betrag_behaelt_den_alten_grund() {
    let s = akte(&[], &[(A, json!(3_000_000))]);
    let (offen, _) = deklaration_der(&s);
    assert_eq!(offen.len(), 1, "{offen:?}");
    assert_eq!(offen[0].0, A);
    assert!(offen[0].1.contains("Pflicht-Bestätigung"), "{}", offen[0].1);
    assert!(!offen[0].1.contains("Versorgungsbezüge"), "{}", offen[0].1);

    let zusammen = akte(&[("veranlagung", json!("zusammen"))], &[(B, json!(3_000_000))]);
    let (offen, _) = deklaration_der(&zusammen);
    assert_eq!(offen.len(), 1, "{offen:?}");
    assert_eq!(offen[0].0, B);
    assert!(offen[0].1.contains("Pflicht-Bestätigung"), "{}", offen[0].1);
}
