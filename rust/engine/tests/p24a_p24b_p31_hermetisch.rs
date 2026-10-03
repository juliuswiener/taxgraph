//! § 24a, § 24b und § 31 `EStG`: die Rechnung der Funktionen in `zugriff/teil1/ermaessigungen.rs` im Standardlauf
//! (ohne `PARITY=1`, ohne Python).
//!
//! `p24a_altersentlastung`, `p24b_entlastung` und `p31_familienleistung` stuetzten bisher je ein Doctest (ein einziger
//! Punkt) und die Parity (`zugriff_teil1_paritaet.rs`, nur mit `PARITY=1`; die CI faehrt Parity nicht). Gemessen am
//! 2026-10-03 auf 57944f48 (Bericht h8-hermetisch3): acht Mutationen am Aufrufort in `ermaessigungen.rs`. Drei lassen
//! `cargo test -p engine` gruen (151 passed, 0 failed): § 24a `Geburtsjahr <= 0` -> `< 0` (ein unbekanntes Geburtsjahr
//! bekaeme einen Abzug), § 24a `positive_andere_einkuenfte` immer 0, § 24b `monate_ohne_voraussetzung` immer 0. Fuenf
//! faengt allein je ein Doctest: § 24a Folgejahr + 64 statt + 65, § 24a Gate `>=` statt `>`, § 24b `alleinstehend` immer
//! wahr, § 31 `ESt` ohne und mit Freibetraegen vertauscht, § 31 Kindergeld ignoriert. Mit diesen Tests werden alle acht
//! rot, jede mit dem Test ihrer Funktion (`p24a_`, `p24b_` bzw. `p31_`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl ist die Ausgabe des Python-Orakels (`runner.catala_p24a_altersentlastung`,
//! `runner.catala_p24b_entlastung`, `runner.catala_p31_familienleistung`) auf denselben Eingaben UND stimmt mit einer
//! Rechnung aus den Quellen im Baum ueberein: § 24a die Kohortentabelle aus
//! `sources/gesetze-im-internet/estg_p24a_2026-07-13.txt` (die Text-Tabelle geparst, nicht aus `params/`), § 24b Abs. 2 und
//! 4 mit 4.260 EUR plus 240 EUR je weiterem Kind und einem Zwoelftel je Monat ohne Voraussetzung
//! (`estg_p24b_2026-07-09.txt`). Fuer § 31 gilt nur die Kernregel (Freibetrag guenstiger -> `ESt` mit Freibetraegen plus
//! Kindergeld, sonst `ESt` ohne Freibetraege); der Quellenausschnitt `estg_p31_2026-07-11.txt` nennt die Rechenregel nicht,
//! dort stuetzt nur das Python-Orakel. Das Orakel hat beides gegeneinander geprueft und bricht bei einer Abweichung ab.
//! Kein Wert ist aus dem Rust-Code abgelesen.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use bindung::Params;
use domain::Euro;
use engine::zugriff::teil1::ermaessigungen::{
    p24a_altersentlastung, p24b_entlastung, p31_familienleistung, P24aAltersentlastungEingabe,
    P24bEntlastungEingabe, P31FamilienleistungEingabe,
};

/// `(Name, VZ, Geburtsjahr, Arbeitslohn EUR, positive andere Einkuenfte EUR, erwartet EUR)`
const P24A: &[(&str, i64, i64, i64, i64, i64)] = &[
    (
        "VZ 2025, Geburtsjahr 1960, Arbeitslohn 10000, andere Einkuenfte 0 EUR",
        2025,
        1960,
        10_000,
        0,
        627,
    ),
    (
        "VZ 2025, Geburtsjahr 1960, Arbeitslohn 1000, andere Einkuenfte 0 EUR",
        2025,
        1960,
        1000,
        0,
        132,
    ),
    (
        "VZ 2025, Geburtsjahr 1960, Arbeitslohn 1000, andere Einkuenfte 2000 EUR",
        2025,
        1960,
        1000,
        2000,
        396,
    ),
    (
        "VZ 2025, Geburtsjahr 1961, Arbeitslohn 10000, andere Einkuenfte 0 EUR",
        2025,
        1961,
        10_000,
        0,
        0,
    ),
    (
        "VZ 2026, Geburtsjahr 1961, Arbeitslohn 10000, andere Einkuenfte 0 EUR",
        2026,
        1961,
        10_000,
        0,
        608,
    ),
    (
        "VZ 2025, Geburtsjahr 1959, Arbeitslohn 10000, andere Einkuenfte 0 EUR",
        2025,
        1959,
        10_000,
        0,
        646,
    ),
    (
        "VZ 2025, Geburtsjahr 0, Arbeitslohn 10000, andere Einkuenfte 0 EUR",
        2025,
        0,
        10_000,
        0,
        0,
    ),
    (
        "VZ 2025, Geburtsjahr -5, Arbeitslohn 10000, andere Einkuenfte 0 EUR",
        2025,
        -5,
        10_000,
        0,
        0,
    ),
    (
        "VZ 0, Geburtsjahr 1990, Arbeitslohn 10000, andere Einkuenfte 0 EUR",
        0,
        1990,
        10_000,
        0,
        57,
    ),
    (
        "VZ 2025, Geburtsjahr 1940, Arbeitslohn 10000, andere Einkuenfte 0 EUR",
        2025,
        1940,
        10_000,
        0,
        1900,
    ),
    (
        "VZ 2025, Geburtsjahr 1930, Arbeitslohn 1000, andere Einkuenfte 0 EUR",
        2025,
        1930,
        1000,
        0,
        400,
    ),
    (
        "VZ 2025, Geburtsjahr 1960, Arbeitslohn 0, andere Einkuenfte 0 EUR",
        2025,
        1960,
        0,
        0,
        0,
    ),
    (
        "VZ 2025, Geburtsjahr 1960, Arbeitslohn 0, andere Einkuenfte 5000 EUR",
        2025,
        1960,
        0,
        5000,
        627,
    ),
];

/// `(Name, alleinstehend, Zahl der Kinder, Monate ohne Voraussetzung, erwartet EUR)`
const P24B: &[(&str, bool, i64, i64, i64)] = &[
    (
        "alleinstehend ja, 1 Kinder, 0 Monate ohne Voraussetzung",
        true,
        1,
        0,
        4260,
    ),
    (
        "alleinstehend ja, 2 Kinder, 0 Monate ohne Voraussetzung",
        true,
        2,
        0,
        4500,
    ),
    (
        "alleinstehend ja, 3 Kinder, 0 Monate ohne Voraussetzung",
        true,
        3,
        0,
        4740,
    ),
    (
        "alleinstehend ja, 0 Kinder, 0 Monate ohne Voraussetzung",
        true,
        0,
        0,
        0,
    ),
    (
        "alleinstehend nein, 1 Kinder, 0 Monate ohne Voraussetzung",
        false,
        1,
        0,
        0,
    ),
    (
        "alleinstehend ja, 1 Kinder, 6 Monate ohne Voraussetzung",
        true,
        1,
        6,
        2130,
    ),
    (
        "alleinstehend ja, 1 Kinder, 12 Monate ohne Voraussetzung",
        true,
        1,
        12,
        0,
    ),
    (
        "alleinstehend ja, 2 Kinder, 3 Monate ohne Voraussetzung",
        true,
        2,
        3,
        3375,
    ),
    (
        "alleinstehend ja, 1 Kinder, 1 Monate ohne Voraussetzung",
        true,
        1,
        1,
        3905,
    ),
    (
        "alleinstehend ja, 2 Kinder, 12 Monate ohne Voraussetzung",
        true,
        2,
        12,
        0,
    ),
    (
        "alleinstehend nein, 2 Kinder, 6 Monate ohne Voraussetzung",
        false,
        2,
        6,
        0,
    ),
];

/// `(Name, ESt ohne Freibetraege EUR, ESt mit Freibetraegen EUR, Kindergeld EUR, erwartet EUR)`
const P31: &[(&str, i64, i64, i64, i64)] = &[
    (
        "ESt ohne 10000, mit 9000, Kindergeld 3000 EUR",
        10_000,
        9000,
        3000,
        10_000,
    ),
    (
        "ESt ohne 10000, mit 5000, Kindergeld 3000 EUR",
        10_000,
        5000,
        3000,
        8000,
    ),
    (
        "ESt ohne 10000, mit 7000, Kindergeld 3000 EUR",
        10_000,
        7000,
        3000,
        10_000,
    ),
    (
        "ESt ohne 10000, mit 6999, Kindergeld 3000 EUR",
        10_000,
        6999,
        3000,
        9999,
    ),
    ("ESt ohne 0, mit 0, Kindergeld 0 EUR", 0, 0, 0, 0),
    (
        "ESt ohne 10000, mit 5000, Kindergeld 0 EUR",
        10_000,
        5000,
        0,
        5000,
    ),
    (
        "ESt ohne 10000, mit 10000, Kindergeld 0 EUR",
        10_000,
        10_000,
        0,
        10_000,
    ),
    (
        "ESt ohne 8000, mit 2000, Kindergeld 6000 EUR",
        8000,
        2000,
        6000,
        8000,
    ),
];

/// Meldet jeden Fall, der vom Orakel abweicht; unter einer Mutation zeigt die Meldung alle roten Faelle.
fn melde(abweichend: &[String], faelle: usize) {
    assert!(
        abweichend.is_empty(),
        "{} von {faelle} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len()
    );
}

/// § 24a: Kohortentabelle (Prozentsatz und Hoechstbetrag nach Geburtsjahr + 65), das 64+-Gate (S. 3), die
/// Gate-Grenze beim VZ, `Geburtsjahr <= 0` und VZ 0 (Gate aus).
#[test]
fn p24a_gate_kohorte_und_grenzen() {
    let p = Params::lade(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    let abweichend: Vec<String> = P24A
        .iter()
        .filter_map(|&(name, vz, geburtsjahr, lohn, andere, erwartet)| {
            let e = P24aAltersentlastungEingabe {
                veranlagungszeitraum: vz,
                geburtsjahr,
                arbeitslohn: Euro::new(lohn),
                positive_andere_einkuenfte: Euro::new(andere),
            };
            let got = p24a_altersentlastung(&e, &p).map(Euro::get);
            (!matches!(got, Ok(v) if v == erwartet))
                .then(|| format!("{name}: Rust {got:?}, Orakel {erwartet}"))
        })
        .collect();
    melde(&abweichend, P24A.len());
}

/// § 24b: Grundbetrag, Erhoehung je weiterem Kind, anteilige Kuerzung nach Monaten, nur Alleinstehende.
#[test]
fn p24b_alleinstehend_kinder_und_monate() {
    let abweichend: Vec<String> = P24B
        .iter()
        .filter_map(|&(name, alleinstehend, anzahl_kinder, monate, erwartet)| {
            let e = P24bEntlastungEingabe {
                alleinstehend,
                anzahl_kinder,
                monate_ohne_voraussetzung: monate,
            };
            let got = p24b_entlastung(&e).map(Euro::get);
            (!matches!(got, Ok(v) if v == erwartet))
                .then(|| format!("{name}: Rust {got:?}, Orakel {erwartet}"))
        })
        .collect();
    melde(&abweichend, P24B.len());
}

/// § 31 S. 4: Guenstigerpruefung. Die drei Eingaben gehen an die richtige Stelle (`ESt` ohne, `ESt` mit, Kindergeld).
#[test]
fn p31_guenstigerpruefung_und_kindergeld() {
    let abweichend: Vec<String> = P31
        .iter()
        .filter_map(|&(name, ohne, mit, kindergeld, erwartet)| {
            let e = P31FamilienleistungEingabe {
                est_ohne_freibetraege: Euro::new(ohne),
                est_mit_freibetraegen: Euro::new(mit),
                kindergeld: Euro::new(kindergeld),
            };
            let got = p31_familienleistung(&e).map(Euro::get);
            (!matches!(got, Ok(v) if v == erwartet))
                .then(|| format!("{name}: Rust {got:?}, Orakel {erwartet}"))
        })
        .collect();
    melde(&abweichend, P31.len());
}
