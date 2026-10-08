//! Versorgungsbezuege sperren die Abgabe (Abweichung Nr. 42): die Naht `einreichungs_xml` und `deklariere`. Ohne `PARITY=1`,
//! ohne Python, ohne Netz, ohne `ERiC`.
//!
//! **Worum es geht.** Wer einen Versorgungsbezug (Pension) angibt, bekommt ihn im Bescheid verrechnet (Versorgungsfreibetrag,
//! Zuschlag, Pauschbetrag). Die ELSTER-Erklaerung traegt ihn nicht: `versorgung_jahresrente` und `versorgung_jahresrente_partner`
//! haben kein Kennzeichen (Kz). Vorher stand der Betrag nur in `nicht_deklariert`, und die Abgabe ging durch.
//!
//! **Warum es zaehlt.** Das Finanzamt haette den Bezug nie gesehen und die Steuer aus einer Erklaerung ohne ihn gerechnet. Jetzt
//! sperrt die Abgabe bei einem bestaetigten Betrag ueber 0, die Zahl im Bescheid bleibt. Wer den Bezug hat, traegt ihn im
//! amtlichen Formular selbst ein, bis das Kz gebaut ist.
//!
//! **Wo es sitzt.** `rust/elster/src/deklaration.rs::Bau::versorgung`. Der Ehegatte sperrt nur bei bestaetigter
//! Zusammenveranlagung: nur dann zaehlt sein Bezug im Bescheid (Abweichung Nr. 33). Ein vorlaeufiger Betrag behaelt seinen
//! Grund "Pflicht-Bestaetigung fehlt". Die HTTP-Naht (`GET /deklaration`, `POST /einreichen`) prueft
//! `rust/api/tests/versorgung_abgabe_sperre_hermetisch.rs`.
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

/// `(unvollstaendig, nicht_deklariert)` der Deklaration, je als `(feld_id, grund)`; nur Felder `versorgung_*`.
fn deklaration_der(s: &Store) -> (Vec<(String, String)>, Vec<String>) {
    let (felder, _) = s.materialisiere(None).unwrap();
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

/// Person A: Bezug von `euro`, Bemessungsgrundlage gleich dem Bezug, Beginn 2025, beamtenrechtlich.
fn versorgung_a(euro: i64) -> Vec<(&'static str, Value)> {
    vec![
        (A, json!(euro * 100)),
        ("versorgung_bemessungsgrundlage", json!(euro * 100)),
        ("versorgung_beginn_jahr", json!(2025)),
        ("versorgung_art", json!("beamtenrechtlich")),
    ]
}

fn versorgung_b(euro: i64) -> Vec<(&'static str, Value)> {
    vec![
        (B, json!(euro * 100)),
        ("versorgung_bemessungsgrundlage_partner", json!(euro * 100)),
        ("versorgung_beginn_jahr_partner", json!(2025)),
        ("versorgung_art_partner", json!("beamtenrechtlich")),
    ]
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

/// AK1: Person A, bestaetigter Bezug ueber 0 -> die Abgabe sperrt mit einem Eintrag fuer `versorgung_jahresrente`, und der
/// Grund sagt, was fehlt und was der Nutzer tut. Der Betrag steht weiter mit Grund in `nicht_deklariert`.
#[test]
fn bezug_von_person_a_sperrt_die_abgabe() {
    let s = akte(&versorgung_a(30_000), &[]);
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
        grund.starts_with("Versorgungsbezüge über 0 Euro") && grund.contains("gesperrt") && grund.contains("Formular"),
        "der Grund nennt Bezug, Sperre und Formular nicht: {grund}"
    );
    let (offen, nicht) = deklaration_der(&s);
    assert_eq!(offen.len(), 1, "{offen:?}");
    assert!(nicht.contains(&A.to_owned()), "der Betrag verschwindet aus nicht_deklariert: {nicht:?}");
}

/// AK2: der Ehegatte sperrt nur bei BESTAETIGTER Zusammenveranlagung. Einzelveranlagung und eine vorlaeufige Antwort auf die
/// Veranlagung sperren nicht (sein Bezug zaehlt im Bescheid dann nicht).
#[test]
fn der_bezug_des_ehegatten_sperrt_nur_bei_bestaetigter_zusammenveranlagung() {
    let bezug = versorgung_b(30_000);
    let zusammen = mit(vec![("veranlagung", json!("zusammen"))], bezug.clone());
    let (offen, nicht) = deklaration_der(&akte(&zusammen, &[]));
    assert_eq!(
        offen.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
        [B],
        "zusammen: genau der Ehegatte-Betrag sperrt"
    );
    assert!(nicht.contains(&B.to_owned()), "{nicht:?}");

    let einzel = mit(vec![("veranlagung", json!("einzel"))], bezug.clone());
    let (offen, nicht) = deklaration_der(&akte(&einzel, &[]));
    assert!(offen.is_empty(), "einzel: der Bezug des Ehegatten zaehlt nicht, also sperrt er nicht: {offen:?}");
    assert!(nicht.contains(&B.to_owned()), "{nicht:?}");

    let vorlaeufig = akte(&bezug, &[("veranlagung", json!("zusammen"))]);
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
