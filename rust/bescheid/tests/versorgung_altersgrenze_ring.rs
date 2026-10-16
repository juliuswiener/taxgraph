//! Der Ring-Wert `versorgung_altersgrenze` (Abweichung Nr. 65, Backlog `versorgung-alters-gate-sperre-heben` AK2/AK3/AK5). Ohne
//! `PARITY=1`, ohne Python, ohne Netz, ohne `ERiC`: nur `mit_ring_werten` auf einem Store, ohne `deklariere`.
//!
//! **Worum es geht.** Ein Bezug mit nicht erfuelltem Alters-Gate (Alter BEI BEGINN unter 63, unter 60 bei Grad der Behinderung ab
//! 50) ist nach § 19 Abs. 2 Satz 2 Nr. 2 `EStG` erst ein Versorgungsbezug, wenn die Person die Grenze vollendet hat. Der Ring sagt
//! darum, wie das Veranlagungsjahr 2025 zur Grenze steht: `lohn` (die Person erreicht sie erst nach dem Jahr, der Bezug ist das ganze
//! Jahr Arbeitslohn) oder `darueber` (sie hat sie im oder vor dem Jahr erreicht). Der Wert schliesst `versorgung_zeile` aus.
//!
//! Die Sperre selbst, die aus diesem Wert folgt, steht in `versorgung_sperre_faelle.rs` (Tabellen) und in `elster` (Unit-Tests).
//!
//! **Abweichung Nr. 80.** Den Grad der Behinderung von Person A kann ein Feld der Rentner-Scheibe (`rentner_grad_der_behinderung`) oder
//! die Frage des Gesamtlaufs (`versorgung_grad_der_behinderung`) tragen. Beide Felder muessen dieselbe Grenze ergeben; die Rentner-Scheibe
//! geht vor. Der Ehegatte hat nur das Feld der Rentner-Scheibe: `gesamt` fuehrt keinen Bezug des Ehegatten.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use bescheid::deklaration::mit_ring_werten;
use bescheid::testhilfe::{felder, params, store};
use domain::{PyWert, Vz};
use serde_json::{json, Value};

type Ev = (&'static str, Value, bool);

/// Was der Ring zum Bezug der Person setzt: `versorgung_zeile` und der Text von `versorgung_altersgrenze`.
#[derive(Debug, PartialEq, Eq)]
struct Lage {
    zeile: bool,
    altersgrenze: Option<String>,
}

const A: (&str, &str) = ("versorgung_zeile", "versorgung_altersgrenze");
const B: (&str, &str) = ("versorgung_zeile_partner", "versorgung_altersgrenze_partner");

fn lage(ev: &[Ev], vz: Option<Vz>, person: (&str, &str)) -> Lage {
    let mut f = felder(&store(ev));
    mit_ring_werten(&mut f, vz, params()).unwrap();
    let text = f.get(person.1).map(|s| match &s.wert {
        PyWert::Text(t) => t.clone(),
        w => panic!("{} ist kein Text: {w:?}", person.1),
    });
    let zeile = f.get(person.0).is_some_and(|s| matches!(s.wert, PyWert::Bool(true)));
    // Die beiden Werte schliessen sich aus.
    assert!(!(zeile && text.is_some()), "{person:?}: Zeilen UND Altersgrenze gesetzt");
    Lage { zeile, altersgrenze: text }
}

/// Die Felder des Grades der Behinderung von Person A: die Rentner-Scheibe, dann der Gesamtlauf (Abweichung Nr. 80).
const GRAD_A: [&str; 2] = ["rentner_grad_der_behinderung", "versorgung_grad_der_behinderung"];
/// Das Feld des Grades der Behinderung des Ehegatten (nur die Rentner-Scheibe fragt es).
const GRAD_B: &str = "rentner_grad_der_behinderung_partner";

/// Person A mit Altersgrenzen-Bezug in CENT, alles bestaetigt. `gdb` `None`: das Feld fehlt. Der Grad steht im Feld der Rentner-Scheibe.
fn bezug_a(cent: i64, alter: i64, geburtsjahr: i64, gdb: Option<i64>) -> Vec<Ev> {
    bezug_a_im_feld(cent, alter, geburtsjahr, GRAD_A[0], gdb)
}

/// Wie [`bezug_a`], der Grad steht im Feld `grad_feld`.
fn bezug_a_im_feld(cent: i64, alter: i64, geburtsjahr: i64, grad_feld: &'static str, gdb: Option<i64>) -> Vec<Ev> {
    let mut ev: Vec<Ev> = vec![
        ("versorgung_jahresrente", json!(cent), true),
        ("versorgung_bemessungsgrundlage", json!(cent), true),
        ("versorgung_beginn_jahr", json!(2020), true),
        ("versorgung_art", json!("altersgrenze_sonstige"), true),
        ("versorgung_alter_bei_beginn", json!(alter), true),
        ("geburtsjahr", json!(geburtsjahr), true),
    ];
    if let Some(g) = gdb {
        ev.push((grad_feld, json!(g), true));
    }
    ev
}

const VZ: Option<Vz> = Some(Vz::Vz2025);

/// Die Grenze nach dem Veranlagungsjahr 2025 (Geburtsjahr plus 63, bei Grad ab 50 plus 60, ohne Grad vorsichtig plus 60 groesser
/// 2025) ist `lohn`; im Jahr selbst und davor `darueber`. Jede Zeile ist eine Grenze: ein Jahr davor, genau im Jahr, ein Jahr danach.
#[test]
fn die_altersgrenze_im_veranlagungsjahr_entscheidet_lohn_oder_darueber() {
    // (Geburtsjahr, Grad der Behinderung, erwartet)
    let faelle: [(i64, Option<i64>, &str); 15] = [
        (1961, Some(0), "darueber"), // 63 im Jahr 2024
        (1962, Some(0), "darueber"), // 63 genau 2025
        (1963, Some(0), "lohn"),     // 63 erst 2026
        (1964, Some(0), "lohn"),
        (1964, Some(49), "lohn"), // Grad 49 ist keine Schwerbehinderung: 63
        (1964, Some(50), "darueber"), // Grad 50: 60 im Jahr 2024
        (1965, Some(50), "darueber"), // 60 genau 2025
        (1966, Some(50), "lohn"), // 60 erst 2026
        (1965, Some(100), "darueber"),
        (1965, None, "darueber"), // Grad fehlt: vorsichtig 60, also 2025
        (1966, None, "lohn"),
        (1964, None, "darueber"), // Grad fehlt: vorsichtig 60, 2024
        (1957, Some(0), "darueber"),
        (2000, Some(0), "lohn"),
        (2025, Some(0), "lohn"), // geboren im Veranlagungsjahr selbst: 63 erst 2088
    ];
    // Abweichung Nr. 80: das Feld, das der Gesamtlauf fragt, ergibt dieselbe Grenze wie das der Rentner-Scheibe.
    for grad_feld in GRAD_A {
        for (geburtsjahr, gdb, soll) in faelle {
            let ist = lage(&bezug_a_im_feld(3_000_000, 58, geburtsjahr, grad_feld, gdb), VZ, A);
            assert_eq!(
                ist,
                Lage { zeile: false, altersgrenze: Some(soll.to_owned()) },
                "{grad_feld}: geboren {geburtsjahr}, Grad {gdb:?}"
            );
        }
    }
}

/// Die Grenze folgt dem Veranlagungsjahr, nicht dem Jahr 2025: in jedem Jahr, das die Rechnung kennt, ist die Person im Jahr, in dem
/// sie 63 wird, `darueber` und ein Jahr davor `lohn` (Mutante Y14 der Nachmessung von main hielt das Jahr fest auf 2025 und blieb gruen).
#[test]
fn die_grenze_folgt_dem_veranlagungsjahr() {
    for (vz, jahr) in [(Vz::Vz2024, 2024), (Vz::Vz2025, 2025), (Vz::Vz2026, 2026)] {
        let im_jahr = lage(&bezug_a(3_000_000, 58, jahr - 63, Some(0)), Some(vz), A);
        assert_eq!(im_jahr.altersgrenze, Some("darueber".to_owned()), "VZ {jahr}: 63 genau im Jahr");
        let danach = lage(&bezug_a(3_000_000, 58, jahr - 62, Some(0)), Some(vz), A);
        assert_eq!(danach.altersgrenze, Some("lohn".to_owned()), "VZ {jahr}: 63 erst im Jahr danach");
    }
}

/// Ein Geburtsjahr, das fehlt, nicht bestaetigt ist oder nach dem Veranlagungsjahr liegt, ist unlesbar: der Bezug sperrt (`darueber`),
/// er faellt nicht als Arbeitslohn durch.
#[test]
fn ein_unlesbares_geburtsjahr_zaehlt_als_darueber() {
    let ohne_jahr: Vec<Ev> = bezug_a(3_000_000, 58, 1965, Some(0)).into_iter().filter(|(f, _, _)| *f != "geburtsjahr").collect();
    assert_eq!(lage(&ohne_jahr, VZ, A).altersgrenze, Some("darueber".to_owned()), "Feld fehlt");
    let mut vorlaeufig = ohne_jahr.clone();
    vorlaeufig.push(("geburtsjahr", json!(1965), false));
    assert_eq!(lage(&vorlaeufig, VZ, A).altersgrenze, Some("darueber".to_owned()), "nicht bestaetigt");
    for jahr in [0, 2026, 2100, -1965] {
        let ist = lage(&bezug_a(3_000_000, 58, jahr, Some(0)), VZ, A);
        assert_eq!(ist.altersgrenze, Some("darueber".to_owned()), "Geburtsjahr {jahr}");
    }
}

/// Ein nicht bestaetigter Grad der Behinderung gilt als nicht gefragt: vorsichtig die Grenze 60, auch bei einem Grad 0.
#[test]
fn ein_nicht_bestaetigter_grad_der_behinderung_gilt_als_fehlend() {
    for grad_feld in GRAD_A {
        let mut ev = bezug_a(3_000_000, 58, 1965, None);
        ev.push((grad_feld, json!(0), false));
        assert_eq!(lage(&ev, VZ, A).altersgrenze, Some("darueber".to_owned()), "{grad_feld}: Grad 0 nicht bestaetigt: 60 = 2025");
        ev.push((grad_feld, json!(0), true));
        assert_eq!(lage(&ev, VZ, A).altersgrenze, Some("lohn".to_owned()), "{grad_feld}: Grad 0 bestaetigt: 63 = 2028");
    }
}

/// Abweichung Nr. 80: stehen beide Felder in der Akte (die Rentner-Scheibe und der Gesamtlauf fragen je eines, eine Akte kann beide
/// tragen), gilt das der Rentner-Scheibe; ein nicht bestaetigtes Feld der Rentner-Scheibe laesst das des Gesamtlaufs gelten. Jede Person
/// liest nur ihre eigenen Felder.
#[test]
fn das_feld_der_rentner_scheibe_geht_dem_der_gesamtfrage_vor() {
    // Geboren 1965, Alter bei Beginn 58: Grad ab 50 ist 60 = 2025 (`darueber`), darunter 63 = 2028 (`lohn`).
    let beide = |rentner: i64, gesamt: i64| {
        let mut ev = bezug_a_im_feld(3_000_000, 58, 1965, GRAD_A[0], Some(rentner));
        ev.push((GRAD_A[1], json!(gesamt), true));
        lage(&ev, VZ, A).altersgrenze
    };
    assert_eq!(beide(0, 50), Some("lohn".to_owned()), "Rentner 0, Gesamt 50: die Rentner-Scheibe gilt");
    assert_eq!(beide(50, 0), Some("darueber".to_owned()), "Rentner 50, Gesamt 0: die Rentner-Scheibe gilt");
    let mut ev = bezug_a_im_feld(3_000_000, 58, 1965, GRAD_A[0], Some(50));
    let rentner_grad = ev.len() - 1;
    ev[rentner_grad].2 = false;
    ev.push((GRAD_A[1], json!(0), true));
    assert_eq!(lage(&ev, VZ, A).altersgrenze, Some("lohn".to_owned()), "Rentner 50 nicht bestaetigt, Gesamt 0 bestaetigt: Gesamt gilt");
    // Der Grad des Ehegatten ist kein Grad von Person A (und umgekehrt).
    let mut fremd = bezug_a(3_000_000, 58, 1965, None);
    fremd.push((GRAD_B, json!(0), true));
    assert_eq!(lage(&fremd, VZ, A).altersgrenze, Some("darueber".to_owned()), "der Grad des Ehegatten zaehlt nicht fuer Person A");
}

/// Der Wert entsteht nur, wo das Alters-Gate nicht erfuellt ist und beide Betraege mindestens 1 Euro ergeben; erfuellt das Gate, gilt
/// der Weg der Zeilen 11 bis 13 und der Ring setzt `versorgung_zeile`.
#[test]
fn der_wert_entsteht_nur_bei_nicht_erfuelltem_gate_und_betraegen_ab_einem_euro() {
    let leer = Lage { zeile: false, altersgrenze: None };
    assert_eq!(lage(&bezug_a(3_000_000, 63, 1957, Some(0)), VZ, A), Lage { zeile: true, altersgrenze: None }, "Gate erfuellt");
    assert_eq!(lage(&bezug_a(3_000_000, 60, 1957, Some(50)), VZ, A), Lage { zeile: true, altersgrenze: None }, "Gate 60 erfuellt");
    // Abweichung Nr. 80: das Gate (Alter BEI BEGINN) liest den Grad der Gesamtfrage wie den der Rentner-Scheibe. 60 bei Beginn genuegt nur
    // mit Grad ab 50; mit Grad 0 ist das Gate nicht erfuellt, und die Person (geboren 1957) hat die Grenze 63 = 2020 erreicht.
    let gesamtfrage = GRAD_A[1];
    assert_eq!(
        lage(&bezug_a_im_feld(3_000_000, 60, 1957, gesamtfrage, Some(50)), VZ, A),
        Lage { zeile: true, altersgrenze: None },
        "Gesamtfrage, Gate 60 erfuellt"
    );
    assert_eq!(
        lage(&bezug_a_im_feld(3_000_000, 60, 1957, gesamtfrage, Some(0)), VZ, A),
        Lage { zeile: false, altersgrenze: Some("darueber".to_owned()) },
        "Gesamtfrage, Grad 0: das Gate verlangt 63"
    );
    assert_eq!(lage(&bezug_a(99, 58, 1965, Some(0)), VZ, A), leer, "99 Cent");
    assert_eq!(lage(&bezug_a(0, 58, 1965, Some(0)), VZ, A), leer, "Bezug 0");
    let genau_ein_euro = lage(&bezug_a(100, 58, 1965, Some(0)), VZ, A);
    assert_eq!(genau_ein_euro.altersgrenze, Some("lohn".to_owned()), "genau 1 Euro");
    let mut nur_bmg = bezug_a(3_000_000, 58, 1965, Some(0));
    nur_bmg.retain(|(f, _, _)| *f != "versorgung_bemessungsgrundlage");
    assert_eq!(lage(&nur_bmg, VZ, A), leer, "Bemessungsgrundlage fehlt");
}

/// Ohne Veranlagungsjahr (`None`) kennt der Ring die Grenze nicht: kein Wert; ein erfuelltes Gate setzt `versorgung_zeile` wie zuvor.
#[test]
fn ohne_veranlagungsjahr_gibt_es_keinen_altersgrenzen_wert() {
    assert_eq!(lage(&bezug_a(3_000_000, 58, 1965, Some(0)), None, A), Lage { zeile: false, altersgrenze: None });
    assert_eq!(lage(&bezug_a(3_000_000, 63, 1957, Some(0)), None, A), Lage { zeile: true, altersgrenze: None });
}

/// Der Ehegatte kommt aus den Partner-Feldern und nur bei bestaetigter Zusammenveranlagung; Person A und Ehegatte rechnen je mit
/// dem eigenen Geburtsjahr und dem eigenen Grad der Behinderung.
#[test]
fn der_ehegatte_rechnet_mit_den_partner_feldern_und_nur_bei_zusammenveranlagung() {
    let partner = |cent: i64, alter: i64, geburtsjahr: i64, gdb: Option<i64>| -> Vec<Ev> {
        let mut ev: Vec<Ev> = vec![
            ("versorgung_jahresrente_partner", json!(cent), true),
            ("versorgung_bemessungsgrundlage_partner", json!(cent), true),
            ("versorgung_beginn_jahr_partner", json!(2020), true),
            ("versorgung_art_partner", json!("altersgrenze_sonstige"), true),
            ("versorgung_alter_bei_beginn_partner", json!(alter), true),
            ("geburtsjahr_partner", json!(geburtsjahr), true),
        ];
        if let Some(g) = gdb {
            ev.push(("rentner_grad_der_behinderung_partner", json!(g), true));
        }
        ev
    };
    let mut zusammen = partner(3_000_000, 58, 1965, Some(0));
    zusammen.push(("veranlagung", json!("zusammen"), true));
    assert_eq!(lage(&zusammen, VZ, B).altersgrenze, Some("lohn".to_owned()), "Ehegatte 1965, Grad 0");
    assert_eq!(lage(&zusammen, VZ, A).altersgrenze, None, "Person A hat keinen Bezug");
    let mut schwer = partner(3_000_000, 58, 1965, Some(50));
    schwer.push(("veranlagung", json!("zusammen"), true));
    assert_eq!(lage(&schwer, VZ, B).altersgrenze, Some("darueber".to_owned()), "Ehegatte 1965, Grad 50: 60 = 2025");
    // Person A (1965, Grad 0) und Ehegatte (1965, Grad 50) in einer Akte: jede Person mit ihrem eigenen Grad.
    let mut beide = schwer.clone();
    beide.extend(bezug_a(3_000_000, 58, 1965, Some(0)));
    assert_eq!(lage(&beide, VZ, A).altersgrenze, Some("lohn".to_owned()), "A: Grad 0");
    assert_eq!(lage(&beide, VZ, B).altersgrenze, Some("darueber".to_owned()), "B: Grad 50");
    for veranlagung in [json!("einzel"), json!("unbekannt")] {
        let mut einzel = partner(3_000_000, 58, 1965, Some(0));
        einzel.push(("veranlagung", veranlagung.clone(), true));
        assert_eq!(lage(&einzel, VZ, B).altersgrenze, None, "Veranlagung {veranlagung}");
    }
    let mut vorlaeufig = partner(3_000_000, 58, 1965, Some(0));
    vorlaeufig.push(("veranlagung", json!("zusammen"), false));
    assert_eq!(lage(&vorlaeufig, VZ, B).altersgrenze, None, "Zusammenveranlagung nicht bestaetigt");
}
