//! § 35c, Kirchensteuer-Satz und § 36 Abs. 3 `EStG`: die Rechnung im Standardlauf (ohne `PARITY=1`, ohne
//! Python).
//!
//! Bisher stuetzten diese Funktionen nur je ein Doctest und die Parity (`zugriff_teil1_paritaet.rs`,
//! `zugriff_teil2_paritaet`, nur mit `PARITY=1`; die CI faehrt Parity nicht). Gemessen am 2026-10-03 auf
//! 92578c9b (Bericht h8-hermetisch2): vier Mutationen lassen `cargo test -p engine` gruen (146 passed, 0 failed):
//! `p35c_jahresdeckel` mit 14.000 statt 12.000 EUR im uebernaechsten Foerderjahr, `p35c_ermaessigung_cent` mit
//! 7 statt 6 % im uebernaechsten Foerderjahr, `kist` mit 8 statt 9 % ausserhalb Bayerns und Baden-Wuerttembergs,
//! `p36_abschlusszahlung` ohne Aufrunden der `SolZ` auf `KapESt`. Mit diesen Tests wird jede der vier
//! rot (150 passed, 1 failed), und zwar mit genau dem Test, der ihre Funktion prueft.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl ist die Ausgabe des Python-Orakels (`runner.catala_p35c_*`,
//! `runner._p35c_ermaessigung_cent`, `runner.catala_kist`, `runner.catala_p36_abschlusszahlung`) auf denselben
//! Eingaben UND stimmt mit einer Rechnung aus den Quellen im Baum ueberein (§ 35c Abs. 1: 7 %/14.000 EUR, 6 %/12.000 EUR,
//! Energieberater 50 %, `sources/gesetze-im-internet/estg_p35c_2026-07-13.txt`; Kirchensteuer 8 % in Bayern und
//! Baden-Wuerttemberg, 9 % sonst, `sources/kirchensteuer/kist_hebesatz_2026-07-22.txt`; § 36 Abs. 3 S. 1 und 2: jede
//! Abzugsteuer einzeln auf volle Euro aufrunden, `sources/gesetze-im-internet/estg_p36_2026-07-11.txt`). Das Orakel hat
//! beides gegeneinander geprueft und bricht bei einer Abweichung ab. Der gemeinsame Jahreshoechstbetrag fuer Sanierung
//! plus Energieberater (`p35c_jahresdeckel`) ist die Auslegung des Projekts (`rules/estg/p35c/energetische_massnahmen.catala_en`,
//! BMF-Nachtrag Charge 14), nicht Gesetzeswortlaut; dort stuetzt nur das Python-Orakel. Kein Wert ist aus dem Rust-Code
//! abgelesen.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use domain::{Cent, Euro};
use engine::zugriff::teil1::ermaessigungen::{
    kist, p36_abschlusszahlung, KistEingabe, P36AbschlusszahlungEingabe,
};
use engine::zugriff::teil2::p35c::{
    p35c_energieberater, p35c_ermaessigung_cent, p35c_jahresdeckel, p35c_sanierung,
    JahresdeckelEingabe, SanierungEingabe,
};

/// `(Name, Sanierung EUR, Energieberater EUR, uebernaechstes Foerderjahr, erwartet EUR)`
const DECKEL: &[(&str, i64, i64, bool, i64)] = &[
    (
        "Sanierung 13000 + Energieberater 2000 EUR, uebernaechstes Foerderjahr: nein",
        13_000,
        2000,
        false,
        14_000,
    ),
    (
        "Sanierung 13000 + Energieberater 2000 EUR, uebernaechstes Foerderjahr: ja",
        13_000,
        2000,
        true,
        12_000,
    ),
    (
        "Sanierung 11999 + Energieberater 1 EUR, uebernaechstes Foerderjahr: ja",
        11_999,
        1,
        true,
        12_000,
    ),
    (
        "Sanierung 11999 + Energieberater 2 EUR, uebernaechstes Foerderjahr: ja",
        11_999,
        2,
        true,
        12_000,
    ),
    (
        "Sanierung 6000 + Energieberater 5999 EUR, uebernaechstes Foerderjahr: ja",
        6000,
        5999,
        true,
        11_999,
    ),
    (
        "Sanierung 7000 + Energieberater 5000 EUR, uebernaechstes Foerderjahr: nein",
        7000,
        5000,
        false,
        12_000,
    ),
    (
        "Sanierung 7000 + Energieberater 7001 EUR, uebernaechstes Foerderjahr: nein",
        7000,
        7001,
        false,
        14_000,
    ),
    (
        "Sanierung 14000 + Energieberater 0 EUR, uebernaechstes Foerderjahr: nein",
        14_000,
        0,
        false,
        14_000,
    ),
    (
        "Sanierung 12000 + Energieberater 0 EUR, uebernaechstes Foerderjahr: ja",
        12_000,
        0,
        true,
        12_000,
    ),
    (
        "Sanierung 0 + Energieberater 0 EUR, uebernaechstes Foerderjahr: ja",
        0,
        0,
        true,
        0,
    ),
    (
        "Sanierung 500 + Energieberater 300 EUR, uebernaechstes Foerderjahr: ja",
        500,
        300,
        true,
        800,
    ),
    (
        "Sanierung 500 + Energieberater 300 EUR, uebernaechstes Foerderjahr: nein",
        500,
        300,
        false,
        800,
    ),
];

/// `(Name, Aufwand EUR, uebernaechstes Foerderjahr, erwartet Cent)`
const CENT: &[(&str, i64, bool, i64)] = &[
    (
        "Aufwand 20000 EUR, uebernaechstes Foerderjahr: nein",
        20_000,
        false,
        140_000,
    ),
    (
        "Aufwand 20000 EUR, uebernaechstes Foerderjahr: ja",
        20_000,
        true,
        120_000,
    ),
    (
        "Aufwand 300000 EUR, uebernaechstes Foerderjahr: nein",
        300_000,
        false,
        1_400_000,
    ),
    (
        "Aufwand 300000 EUR, uebernaechstes Foerderjahr: ja",
        300_000,
        true,
        1_200_000,
    ),
    (
        "Aufwand 1 EUR, uebernaechstes Foerderjahr: nein",
        1,
        false,
        7,
    ),
    ("Aufwand 1 EUR, uebernaechstes Foerderjahr: ja", 1, true, 6),
    (
        "Aufwand 200000 EUR, uebernaechstes Foerderjahr: ja",
        200_000,
        true,
        1_200_000,
    ),
    (
        "Aufwand 200001 EUR, uebernaechstes Foerderjahr: ja",
        200_001,
        true,
        1_200_000,
    ),
    (
        "Aufwand 233333 EUR, uebernaechstes Foerderjahr: nein",
        233_333,
        false,
        1_400_000,
    ),
];

/// `(Name, Aufwand EUR, uebernaechstes Foerderjahr, erwartet EUR)`
const SANIERUNG: &[(&str, i64, bool, i64)] = &[
    (
        "Aufwand 20000 EUR, uebernaechstes Foerderjahr: nein",
        20_000,
        false,
        1400,
    ),
    (
        "Aufwand 20000 EUR, uebernaechstes Foerderjahr: ja",
        20_000,
        true,
        1200,
    ),
    (
        "Aufwand 300000 EUR, uebernaechstes Foerderjahr: nein",
        300_000,
        false,
        14_000,
    ),
    (
        "Aufwand 300000 EUR, uebernaechstes Foerderjahr: ja",
        300_000,
        true,
        12_000,
    ),
    (
        "Aufwand 99 EUR, uebernaechstes Foerderjahr: nein",
        99,
        false,
        6,
    ),
    (
        "Aufwand 99 EUR, uebernaechstes Foerderjahr: ja",
        99,
        true,
        5,
    ),
    (
        "Aufwand 0 EUR, uebernaechstes Foerderjahr: nein",
        0,
        false,
        0,
    ),
];

/// `(Name, Aufwand EUR, erwartet EUR)`
const ENERGIEBERATER: &[(&str, i64, i64)] = &[
    ("Energieberater 1001 EUR: 50 %", 1001, 500),
    ("Energieberater 1 EUR: 50 %", 1, 0),
    ("Energieberater 3 EUR: 50 %", 3, 1),
    ("Energieberater 0 EUR: 50 %", 0, 0),
    ("Energieberater 100000 EUR: 50 %", 100_000, 50_000),
];

/// `(Name, Konfession, Bundesland, Bemessungsgrundlage EUR, erwartet Cent)`
const KIST: &[(&str, &str, &str, i64, i64)] = &[
    (
        "evangelisch / bayern / 1000 EUR",
        "evangelisch",
        "bayern",
        1000,
        8000,
    ),
    (
        "evangelisch / baden_wuerttemberg / 1000 EUR",
        "evangelisch",
        "baden_wuerttemberg",
        1000,
        8000,
    ),
    (
        "evangelisch / nordrhein_westfalen / 1000 EUR",
        "evangelisch",
        "nordrhein_westfalen",
        1000,
        9000,
    ),
    (
        "roemisch-katholisch / hamburg / 1000 EUR",
        "roemisch-katholisch",
        "hamburg",
        1000,
        9000,
    ),
    (
        "roemisch-katholisch / bayern / 333 EUR",
        "roemisch-katholisch",
        "bayern",
        333,
        2664,
    ),
    ("keine / bayern / 1000 EUR", "keine", "bayern", 1000, 0),
    (
        "andere / nordrhein_westfalen / 1000 EUR",
        "andere",
        "nordrhein_westfalen",
        1000,
        0,
    ),
    (
        "evangelisch / (fehlt) / 1000 EUR",
        "evangelisch",
        "",
        1000,
        9000,
    ),
    (
        "evangelisch / bayern / 0 EUR",
        "evangelisch",
        "bayern",
        0,
        0,
    ),
];

/// `(Name, [festzusetzende ESt, LSt, KapESt, SolZ auf KapESt, KiSt auf KapESt, Vorauszahlungen] in Cent, erwartet Cent)`
const P36: &[(&str, [i64; 6], i64)] = &[
    ("nur LSt 500,01 EUR", [100_000, 50_001, 0, 0, 0, 0], 49_900),
    (
        "nur KapESt 100,01 EUR",
        [100_000, 0, 10_001, 0, 0, 0],
        89_900,
    ),
    (
        "nur SolZ auf KapESt 50,01 EUR",
        [100_000, 0, 0, 5001, 0, 0],
        94_900,
    ),
    (
        "nur KiSt auf KapESt 30,01 EUR",
        [100_000, 0, 0, 0, 3001, 0],
        96_900,
    ),
    (
        "Vorauszahlung wird nicht aufgerundet",
        [100_000, 0, 0, 0, 0, 12_345],
        87_655,
    ),
    (
        "glatte Euro: nichts zu runden",
        [100_000, 50_000, 10_000, 5000, 3000, 1000],
        31_000,
    ),
    (
        "alle mit Cent-Rest",
        [1_000_000, 123_401, 45_001, 2501, 401, 100_050],
        728_250,
    ),
    ("Erstattung", [10_000, 50_001, 0, 0, 0, 0], -40_100),
    (
        "LSt und KapESt je 500,01 EUR: jede einzeln aufgerundet, nicht die Summe",
        [200_000, 50_001, 50_001, 0, 0, 0],
        99_800,
    ),
];

/// Alle Faelle durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn melde(was: &str, abweichend: &[String], von: usize) {
    assert!(
        abweichend.is_empty(),
        "{was}: {} von {von} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len()
    );
}

/// Der gemeinsame Jahresdeckel fuer Sanierung plus Energieberater: 14.000 EUR, im uebernaechsten
/// Foerderjahr 12.000 EUR.
#[test]
fn jahresdeckel_ist_12000_im_uebernaechsten_foerderjahr_sonst_14000() {
    let abweichend: Vec<String> = DECKEL
        .iter()
        .filter_map(|&(name, s, e, ueb, erwartet)| {
            let got = p35c_jahresdeckel(&JahresdeckelEingabe {
                sanierung_ermaessigung: Euro::new(s),
                energieberater_ermaessigung: Euro::new(e),
                ist_uebernaechstes_foerderjahr: ueb,
            })
            .unwrap()
            .get();
            (got != erwartet).then(|| format!("{name}: Rust {got}, Orakel {erwartet}"))
        })
        .collect();
    melde("p35c_jahresdeckel", &abweichend, DECKEL.len());
}

/// Der Cent-Zweig der Sanierung (`catala_est`): 7 % bis 14.000 EUR, im uebernaechsten Foerderjahr 6 % bis
/// 12.000 EUR, exakt in Cent.
#[test]
fn sanierung_cent_zweig_satz_und_hoechstbetrag_je_foerderjahr() {
    let abweichend: Vec<String> = CENT
        .iter()
        .filter_map(|&(name, aufwand, ueb, erwartet)| {
            let got = p35c_ermaessigung_cent(&SanierungEingabe {
                sanierungsaufwendungen: Euro::new(aufwand),
                ist_uebernaechstes_foerderjahr: ueb,
            })
            .unwrap()
            .get();
            (got != erwartet).then(|| format!("{name}: Rust {got}, Orakel {erwartet}"))
        })
        .collect();
    melde("p35c_ermaessigung_cent", &abweichend, CENT.len());
}

/// Die Euro-Zweige: Sanierung (7 %/14.000, 6 %/12.000) und Energieberater (50 %, abgerundet).
#[test]
fn sanierung_und_energieberater_in_euro() {
    let mut abweichend: Vec<String> = SANIERUNG
        .iter()
        .filter_map(|&(name, aufwand, ueb, erwartet)| {
            let got = p35c_sanierung(&SanierungEingabe {
                sanierungsaufwendungen: Euro::new(aufwand),
                ist_uebernaechstes_foerderjahr: ueb,
            })
            .unwrap()
            .get();
            (got != erwartet).then(|| format!("{name}: Rust {got}, Orakel {erwartet}"))
        })
        .collect();
    abweichend.extend(
        ENERGIEBERATER
            .iter()
            .filter_map(|&(name, aufwand, erwartet)| {
                let got = p35c_energieberater(Euro::new(aufwand)).unwrap().get();
                (got != erwartet).then(|| format!("{name}: Rust {got}, Orakel {erwartet}"))
            }),
    );
    melde(
        "p35c_sanierung/p35c_energieberater",
        &abweichend,
        SANIERUNG.len() + ENERGIEBERATER.len(),
    );
}

/// Kirchensteuer: 8 % in Bayern und Baden-Wuerttemberg, sonst 9 %; ohne steuererhebende Konfession und bei
/// fehlendem Bundesland (9 %) wie Python.
#[test]
fn kirchensteuersatz_8_in_bayern_und_bw_sonst_9() {
    let abweichend: Vec<String> = KIST
        .iter()
        .filter_map(|&(name, konfession, bundesland, est, erwartet)| {
            let got = kist(&KistEingabe {
                konfession: konfession.into(),
                bundesland: bundesland.into(),
                est_mit_fb: Euro::new(est),
            })
            .unwrap()
            .get();
            (got != erwartet).then(|| format!("{name}: Rust {got}, Orakel {erwartet}"))
        })
        .collect();
    melde("kist", &abweichend, KIST.len());
}

/// § 36 Abs. 3: `LSt`, `KapESt`, `SolZ` auf `KapESt` und `KiSt` auf `KapESt` werden JEDE FUER SICH auf volle Euro
/// aufgerundet (nicht die Summe, nicht die Vorauszahlungen).
#[test]
fn abschlusszahlung_rundet_jede_abzugsteuer_einzeln_auf() {
    let abweichend: Vec<String> = P36
        .iter()
        .filter_map(|&(name, a, erwartet)| {
            let got = p36_abschlusszahlung(&P36AbschlusszahlungEingabe {
                festzusetzende_est_cent: Cent::new(a[0]),
                lohnsteuer_cent: Cent::new(a[1]),
                kapitalertragsteuer_cent: Cent::new(a[2]),
                kapitalertragsteuer_solz_cent: Cent::new(a[3]),
                kapitalertragsteuer_kist_cent: Cent::new(a[4]),
                vorauszahlungen_cent: Cent::new(a[5]),
            })
            .unwrap()
            .get();
            (got != erwartet).then(|| format!("{name}: Rust {got}, Orakel {erwartet}"))
        })
        .collect();
    melde("p36_abschlusszahlung", &abweichend, P36.len());
}
