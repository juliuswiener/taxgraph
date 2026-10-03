//! Die Rechenkern-Zugriffe der Scheiben `ep`, `n_vor_gwg` und `an_gesamt` (`engine::zugriff::teil1`) im Standardlauf (ohne
//! `PARITY=1`, ohne Python): `entfernungspauschale`, `ep_ab_21km`, `p7_linear_afa`, `werbungskosten_n`,
//! `einkuenfte_nichtselbststaendig`.
//!
//! Die Zugriffe geben an, was die Scheiben fuer Entfernungspauschale, lineare `AfA` und Werbungskosten der Anlage N rechnen.
//! Ihre Gegenprobe gegen Python lief bisher nur in `rust/parity/tests/zugriff_teil1_paritaet.rs` und damit nur mit
//! `PARITY=1`; die CI faehrt Parity nicht. Gemessen am 2026-10-03 auf 4a2f6ea4 (Bericht h8-hermetisch4): 10 Mutationen am
//! Aufrufort (`werbungskosten.rs` E1-E5 und G1-G2, `afa.rs` F2-F3, `einkuenfte.rs` G3). Sieben lassen `cargo test -p engine`
//! gruen (154 passed, 0 failed): E2 Kfz-Flag immer wahr (Deckel 4.500 EUR entfaellt), E3 `OePNV`-Kosten ignoriert, E4
//! `ep_ab_21km` ohne Kappung am Rest, F2 Monat 12 gilt nicht als Anschaffungsmonat, G1 Uebernachtung nicht summiert, G2
//! Verpflegung nicht summiert, G3 Werbungskosten der Anlage N ignoriert. Zwei (E1 Saetze bis 20 km und ab 21 km vertauscht, F3
//! Monate im Anschaffungsjahr `12 - Monat`) faengt nur ein Doctest -- ein Doctest zaehlt hier nicht --; eine (E5 kein Deckel am
//! Anteil bis 20 km) faengt zusaetzlich der Bestands-Unittest `ep_ab_21km_zaehlt_nur_volle_km`. Mit diesen Tests werden alle
//! zehn rot, jede mit mindestens einem der fuenf neuen Tests (Tabelle im Bericht).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl der Tabellen unten ist die Ausgabe des Python-Orakels (`runner.catala_*`, mit
//! leerer Wegwerf-Datenwurzel) auf denselben Eingaben; Orakel-Skript und Lauf: Anlagen zum Bericht. Kein Wert ist aus dem
//! Rust-Code abgelesen. 50 der 72 Faelle stimmen ausserdem mit einer unabhaengigen Rechnung aus den Quellen im Baum ueberein:
//! Entfernungspauschale 0,30 EUR (bis 20 km) und 0,38 EUR (ab dem 21. km; ab VZ 2026 durchgehend 0,38 EUR), nur volle
//! Kilometer, Deckel 4.500 EUR ohne Kfz, `OePNV` nur soweit er die Pauschale uebersteigt (§ 9 Abs. 1 S. 3 Nr. 4, Abs. 2 `EStG`,
//! `estg_p9_abs1nr4_abs2_2026-07-09.txt`), `AfA` linear mit Zwoelftelung im Anschaffungsjahr (§ 7 Abs. 1 S. 1, 4 `EStG`,
//! `estg_p7_2026-07-11.txt`) und Arbeitnehmer-Pauschbetrag 1.230 EUR (§ 9a S. 1 Nr. 1a `EStG`, `estg_p9a_2026-07-09.txt`). Die
//! 20 Faelle von `werbungskosten_n` stimmen mit der Summe der einzeln vom Orakel gerechneten Zweige ueberein; die Einzelwerte
//! der Zweige doppelte Haushaltsfuehrung, Verpflegung (Pauschalen, Dreimonatsfrist), Uebernachtung und GWG-Sofortabzug
//! (800 EUR) stuetzt nur das Python-Orakel. Nur das Orakel stuetzt auch die zwei Faelle von `einkuenfte_nichtselbststaendig`
//! mit einem Bruttolohn unter dem Pauschbetrag (Ergebnis 0).
//!
//! Abgrenzung: die `Ueberlauf`-Fehler dieser Zugriffe pruefen die Tests in `zugriff_ueberlauf_hermetisch.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::str::FromStr;

use bindung::Params;
use domain::{Euro, Km, Vz};
use engine::zugriff::teil1::afa::{p7_linear_afa, P7LinearAfaEingabe};
use engine::zugriff::teil1::einkuenfte::{
    einkuenfte_nichtselbststaendig, EinkuenfteNichtselbststaendigEingabe,
};
use engine::zugriff::teil1::reisekosten::{DhfEingabe, UebernachtungEingabe, VerpflegungEingabe};
use engine::zugriff::teil1::werbungskosten::{
    entfernungspauschale, ep_ab_21km, werbungskosten_n, EntfernungspauschaleEingabe,
    WerbungskostenNEingabe,
};
use rust_decimal::Decimal;

/// `(Name, VZ, km, Arbeitstage, Kfz, OePNV EUR, erwartet EUR)`
const EP: &[(&str, Vz, &str, i64, bool, i64, i64)] = &[
    (
        "VZ 2025, 15 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "15",
        220,
        false,
        0,
        990,
    ),
    (
        "VZ 2025, 20 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "20",
        220,
        false,
        0,
        1320,
    ),
    (
        "VZ 2025, 21 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "21",
        220,
        false,
        0,
        1403,
    ),
    (
        "VZ 2025, 30 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "30",
        220,
        false,
        0,
        2156,
    ),
    (
        "VZ 2025, 100 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "100",
        220,
        false,
        0,
        4500,
    ),
    (
        "VZ 2025, 100 km, 220 Arbeitstage, Kfz ja, OePNV 0 EUR",
        Vz::Vz2025,
        "100",
        220,
        true,
        0,
        8008,
    ),
    (
        "VZ 2025, 30 km, 220 Arbeitstage, Kfz nein, OePNV 5000 EUR",
        Vz::Vz2025,
        "30",
        220,
        false,
        5000,
        5000,
    ),
    (
        "VZ 2025, 30 km, 220 Arbeitstage, Kfz nein, OePNV 1000 EUR",
        Vz::Vz2025,
        "30",
        220,
        false,
        1000,
        2156,
    ),
    (
        "VZ 2026, 15 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2026,
        "15",
        220,
        false,
        0,
        1254,
    ),
    (
        "VZ 2026, 30 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2026,
        "30",
        220,
        false,
        0,
        2508,
    ),
    (
        "VZ 2026, 100 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2026,
        "100",
        220,
        false,
        0,
        4500,
    ),
    (
        "VZ 2026, 100 km, 220 Arbeitstage, Kfz ja, OePNV 0 EUR",
        Vz::Vz2026,
        "100",
        220,
        true,
        0,
        8360,
    ),
    (
        "VZ 2024, 30 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2024,
        "30",
        220,
        false,
        0,
        2156,
    ),
    (
        "VZ 2025, 101.5 km, 103 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "101.5",
        103,
        false,
        0,
        3788,
    ),
    (
        "VZ 2025, 30 km, 0 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "30",
        0,
        false,
        0,
        0,
    ),
    (
        "VZ 2025, 0 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "0",
        220,
        false,
        0,
        0,
    ),
];

/// `(Name, VZ, km, Arbeitstage, Kfz, OePNV EUR, erwartet EUR)`
const AB21: &[(&str, Vz, &str, i64, bool, i64, i64)] = &[
    (
        "VZ 2025, 15 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "15",
        220,
        false,
        0,
        0,
    ),
    (
        "VZ 2025, 20 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "20",
        220,
        false,
        0,
        0,
    ),
    (
        "VZ 2025, 21 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "21",
        220,
        false,
        0,
        83,
    ),
    (
        "VZ 2025, 30 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "30",
        220,
        false,
        0,
        836,
    ),
    (
        "VZ 2025, 100 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "100",
        220,
        false,
        0,
        3180,
    ),
    (
        "VZ 2025, 100 km, 220 Arbeitstage, Kfz ja, OePNV 0 EUR",
        Vz::Vz2025,
        "100",
        220,
        true,
        0,
        6688,
    ),
    (
        "VZ 2026, 15 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2026,
        "15",
        220,
        false,
        0,
        0,
    ),
    (
        "VZ 2026, 30 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2026,
        "30",
        220,
        false,
        0,
        836,
    ),
    (
        "VZ 2026, 100 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2026,
        "100",
        220,
        false,
        0,
        2828,
    ),
    (
        "VZ 2026, 100 km, 220 Arbeitstage, Kfz ja, OePNV 0 EUR",
        Vz::Vz2026,
        "100",
        220,
        true,
        0,
        6688,
    ),
    (
        "VZ 2024, 30 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2024,
        "30",
        220,
        false,
        0,
        836,
    ),
    (
        "VZ 2025, 101.5 km, 103 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "101.5",
        103,
        false,
        0,
        3170,
    ),
    (
        "VZ 2025, 0 km, 220 Arbeitstage, Kfz nein, OePNV 0 EUR",
        Vz::Vz2025,
        "0",
        220,
        false,
        0,
        0,
    ),
];

/// `(Name, Kosten in Cent, Kosten in EUR, Nutzungsdauer, Monat, Anschaffungsjahr, erwartet EUR)`
const AFA: &[(&str, i64, i64, i64, i64, bool, i64)] = &[
    (
        "Kosten 0 ct / 1200 EUR, Nutzungsdauer 3, Monat 10, Anschaffungsjahr",
        0,
        1200,
        3,
        10,
        true,
        100,
    ),
    (
        "Kosten 0 ct / 1200 EUR, Nutzungsdauer 3, Monat 12, Anschaffungsjahr",
        0,
        1200,
        3,
        12,
        true,
        33,
    ),
    (
        "Kosten 0 ct / 1200 EUR, Nutzungsdauer 3, Monat 1, Anschaffungsjahr",
        0,
        1200,
        3,
        1,
        true,
        400,
    ),
    (
        "Kosten 0 ct / 1200 EUR, Nutzungsdauer 3, Monat 13, Anschaffungsjahr",
        0,
        1200,
        3,
        13,
        true,
        400,
    ),
    (
        "Kosten 0 ct / 1200 EUR, Nutzungsdauer 3, Monat 0, Anschaffungsjahr",
        0,
        1200,
        3,
        0,
        true,
        400,
    ),
    (
        "Kosten 0 ct / 1200 EUR, Nutzungsdauer 3, Monat 10, Folgejahr",
        0,
        1200,
        3,
        10,
        false,
        400,
    ),
    (
        "Kosten 0 ct / 1000 EUR, Nutzungsdauer 3, Monat 6, Anschaffungsjahr",
        0,
        1000,
        3,
        6,
        true,
        194,
    ),
    (
        "Kosten 0 ct / 1200 EUR, Nutzungsdauer 0, Monat 10, Anschaffungsjahr",
        0,
        1200,
        0,
        10,
        true,
        0,
    ),
    (
        "Kosten 0 ct / 1200 EUR, Nutzungsdauer -1, Monat 10, Anschaffungsjahr",
        0,
        1200,
        -1,
        10,
        true,
        0,
    ),
    (
        "Kosten 0 ct / 0 EUR, Nutzungsdauer 3, Monat 10, Anschaffungsjahr",
        0,
        0,
        3,
        10,
        true,
        0,
    ),
    (
        "Kosten 0 ct / -500 EUR, Nutzungsdauer 3, Monat 10, Anschaffungsjahr",
        0,
        -500,
        3,
        10,
        true,
        0,
    ),
    (
        "Kosten 120000 ct / 0 EUR, Nutzungsdauer 3, Monat 10, Anschaffungsjahr",
        120_000,
        0,
        3,
        10,
        true,
        100,
    ),
    (
        "Kosten 120000 ct / 5000 EUR, Nutzungsdauer 3, Monat 10, Anschaffungsjahr",
        120_000,
        5000,
        3,
        10,
        true,
        100,
    ),
    (
        "Kosten 150 ct / 5000 EUR, Nutzungsdauer 3, Monat 10, Folgejahr",
        150,
        5000,
        3,
        10,
        false,
        0,
    ),
];

/// `(Name, VZ, Entfernung (km, Arbeitstage), dHf (Miete, Monate, Inland), Verpflegung (Tage 24h, An-/Abreise, ueber 8h), Uebernachtung (Kosten, Monate, Monate bisher, Inland), Arbeitsmittel EUR, erwartet EUR)`; `None` = der Schluessel fehlt im Sachverhalt
#[allow(clippy::type_complexity)]
const WK_N: &[(
    &str,
    Vz,
    Option<(&str, i64)>,
    Option<(i64, i64, bool)>,
    Option<(i64, i64, i64)>,
    Option<(i64, i64, i64, bool)>,
    Option<i64>,
    i64,
)] = &[
    (
        "VZ 2025: nur Entfernungspauschale",
        Vz::Vz2025,
        Some(("30", 100)),
        None,
        None,
        None,
        None,
        980,
    ),
    (
        "VZ 2025: nur doppelte Haushaltsfuehrung",
        Vz::Vz2025,
        None,
        Some((800, 6, true)),
        None,
        None,
        None,
        4800,
    ),
    (
        "VZ 2025: nur Verpflegung",
        Vz::Vz2025,
        None,
        None,
        Some((10, 4, 5)),
        None,
        None,
        406,
    ),
    (
        "VZ 2025: nur Uebernachtung",
        Vz::Vz2025,
        None,
        None,
        None,
        Some((600, 4, 0, true)),
        None,
        2400,
    ),
    (
        "VZ 2025: nur Arbeitsmittel 500 EUR",
        Vz::Vz2025,
        None,
        None,
        None,
        None,
        Some(500),
        500,
    ),
    (
        "VZ 2025: Arbeitsmittel 900 EUR (ueber 800: kein Sofortabzug)",
        Vz::Vz2025,
        None,
        None,
        None,
        None,
        Some(900),
        0,
    ),
    (
        "VZ 2025: kein Zweig",
        Vz::Vz2025,
        None,
        None,
        None,
        None,
        None,
        0,
    ),
    (
        "VZ 2025: alle fuenf Zweige",
        Vz::Vz2025,
        Some(("30", 100)),
        Some((800, 6, true)),
        Some((10, 4, 5)),
        Some((600, 4, 0, true)),
        Some(500),
        9086,
    ),
    (
        "VZ 2025: Entfernungspauschale und Arbeitsmittel",
        Vz::Vz2025,
        Some(("30", 100)),
        None,
        None,
        None,
        Some(500),
        1480,
    ),
    (
        "VZ 2025: Uebernachtung und Verpflegung",
        Vz::Vz2025,
        None,
        None,
        Some((10, 4, 5)),
        Some((600, 4, 0, true)),
        None,
        2806,
    ),
    (
        "VZ 2026: nur Entfernungspauschale",
        Vz::Vz2026,
        Some(("30", 100)),
        None,
        None,
        None,
        None,
        1140,
    ),
    (
        "VZ 2026: nur doppelte Haushaltsfuehrung",
        Vz::Vz2026,
        None,
        Some((800, 6, true)),
        None,
        None,
        None,
        4800,
    ),
    (
        "VZ 2026: nur Verpflegung",
        Vz::Vz2026,
        None,
        None,
        Some((10, 4, 5)),
        None,
        None,
        406,
    ),
    (
        "VZ 2026: nur Uebernachtung",
        Vz::Vz2026,
        None,
        None,
        None,
        Some((600, 4, 0, true)),
        None,
        2400,
    ),
    (
        "VZ 2026: nur Arbeitsmittel 500 EUR",
        Vz::Vz2026,
        None,
        None,
        None,
        None,
        Some(500),
        500,
    ),
    (
        "VZ 2026: Arbeitsmittel 900 EUR (ueber 800: kein Sofortabzug)",
        Vz::Vz2026,
        None,
        None,
        None,
        None,
        Some(900),
        0,
    ),
    (
        "VZ 2026: kein Zweig",
        Vz::Vz2026,
        None,
        None,
        None,
        None,
        None,
        0,
    ),
    (
        "VZ 2026: alle fuenf Zweige",
        Vz::Vz2026,
        Some(("30", 100)),
        Some((800, 6, true)),
        Some((10, 4, 5)),
        Some((600, 4, 0, true)),
        Some(500),
        9246,
    ),
    (
        "VZ 2026: Entfernungspauschale und Arbeitsmittel",
        Vz::Vz2026,
        Some(("30", 100)),
        None,
        None,
        None,
        Some(500),
        1640,
    ),
    (
        "VZ 2026: Uebernachtung und Verpflegung",
        Vz::Vz2026,
        None,
        None,
        Some((10, 4, 5)),
        Some((600, 4, 0, true)),
        None,
        2806,
    ),
];

/// `(Name, VZ, Bruttoarbeitslohn EUR, Werbungskosten EUR, erwartet EUR)`
const EINK_N: &[(&str, Vz, i64, i64, i64)] = &[
    (
        "VZ 2025, Brutto 40000, Werbungskosten 0",
        Vz::Vz2025,
        40_000,
        0,
        38_770,
    ),
    (
        "VZ 2025, Brutto 40000, Werbungskosten 600",
        Vz::Vz2025,
        40_000,
        600,
        38_770,
    ),
    (
        "VZ 2025, Brutto 40000, Werbungskosten 1230",
        Vz::Vz2025,
        40_000,
        1230,
        38_770,
    ),
    (
        "VZ 2025, Brutto 40000, Werbungskosten 2000",
        Vz::Vz2025,
        40_000,
        2000,
        38_000,
    ),
    (
        "VZ 2025, Brutto 1000, Werbungskosten 0",
        Vz::Vz2025,
        1000,
        0,
        0,
    ),
    ("VZ 2025, Brutto 0, Werbungskosten 0", Vz::Vz2025, 0, 0, 0),
    (
        "VZ 2026, Brutto 40000, Werbungskosten 0",
        Vz::Vz2026,
        40_000,
        0,
        38_770,
    ),
    (
        "VZ 2026, Brutto 40000, Werbungskosten 2000",
        Vz::Vz2026,
        40_000,
        2000,
        38_000,
    ),
    (
        "VZ 2024, Brutto 40000, Werbungskosten 2000",
        Vz::Vz2024,
        40_000,
        2000,
        38_000,
    ),
];

fn params() -> Params {
    Params::lade(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn ep_eingabe(vz: Vz, km: &str, tage: i64, kfz: bool, oepnv: i64) -> EntfernungspauschaleEingabe {
    EntfernungspauschaleEingabe {
        veranlagungszeitraum: vz,
        entfernung_km_roh: Km::new(Decimal::from_str(km).unwrap()),
        arbeitstage: tage,
        eigenes_oder_ueberlassenes_kfz: kfz,
        oepnv_kosten_jahr: Euro::new(oepnv),
    }
}

/// Meldet jeden Fall, der vom Orakel abweicht; unter einer Mutation zeigt die Meldung alle roten Faelle.
fn melde(abweichend: &[String], faelle: usize) {
    assert!(
        abweichend.is_empty(),
        "{} von {faelle} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len()
    );
}

/// § 9 Abs. 1 S. 3 Nr. 4, Abs. 2 (Scheibe `ep`): Staffel 0,30/0,38 EUR (VZ 2024/2025) bzw. 0,38 EUR (ab VZ 2026), nur volle
/// Kilometer, Deckel 4.500 EUR ohne Kfz, `OePNV` nur soweit er die Pauschale uebersteigt.
#[test]
fn entfernungspauschale_staffel_deckel_kfz_und_oepnv() {
    let p = params();
    let abweichend: Vec<String> = EP
        .iter()
        .filter_map(|&(name, vz, km, tage, kfz, oepnv, erwartet)| {
            let got =
                entfernungspauschale(&ep_eingabe(vz, km, tage, kfz, oepnv), &p).map(Euro::get);
            (!matches!(got, Ok(v) if v == erwartet))
                .then(|| format!("{name}: Rust {got:?}, Orakel {erwartet}"))
        })
        .collect();
    melde(&abweichend, EP.len());
}

/// § 9 Abs. 1 S. 3 Nr. 4 S. 2 (Bemessung der Mobilitaetspraemie): der ab dem 21. vollen km erhoehte Teil, am abziehbaren Rest
/// (Deckel) gekappt, nie mehr als der Anteil ueber 20 km.
#[test]
fn ep_ab_21km_ist_der_erhoehte_teil_gekappt_am_rest() {
    let p = params();
    let abweichend: Vec<String> = AB21
        .iter()
        .filter_map(|&(name, vz, km, tage, kfz, oepnv, erwartet)| {
            let got = ep_ab_21km(&ep_eingabe(vz, km, tage, kfz, oepnv), &p).map(Euro::get);
            (!matches!(got, Ok(v) if v == erwartet))
                .then(|| format!("{name}: Rust {got:?}, Orakel {erwartet}"))
        })
        .collect();
    melde(&abweichend, AB21.len());
}

/// § 7 Abs. 1 S. 1 und 4 (Scheibe `n_vor_gwg`): gleiche Jahresbetraege, im Anschaffungsjahr pro rata temporis ((13 - Monat)
/// Zwoelftel), ausserhalb der Monate 1 bis 12 und im Folgejahr der volle Jahresbetrag; Cent-Eingabe geht vor.
#[test]
fn p7_linear_afa_jahresbetrag_und_pro_rata() {
    let abweichend: Vec<String> = AFA
        .iter()
        .filter_map(
            |&(name, cent, euro, nutzungsdauer, monat, anschaffungsjahr, erwartet)| {
                let e = P7LinearAfaEingabe {
                    anschaffungskosten_cent: domain::Cent::new(cent),
                    anschaffungskosten: Euro::new(euro),
                    nutzungsdauer,
                    anschaffung_monat: monat,
                    ist_anschaffungsjahr: anschaffungsjahr,
                };
                let got = p7_linear_afa(&e).map(Euro::get);
                (!matches!(got, Ok(v) if v == erwartet))
                    .then(|| format!("{name}: Rust {got:?}, Orakel {erwartet}"))
            },
        )
        .collect();
    melde(&abweichend, AFA.len());
}

/// § 9 (Scheibe `an_gesamt`): `werbungskosten_n` summiert jeden Zweig, dessen Schluessel im Sachverhalt steht, und nur den;
/// Arbeitsmittel ueber 800 EUR zaehlen nicht als Sofortabzug.
#[test]
fn werbungskosten_n_summiert_jeden_zweig() {
    let p = params();
    let abweichend: Vec<String> = WK_N
        .iter()
        .filter_map(|&(name, vz, ep, dhf, vpf, uen, am, erwartet)| {
            let e = WerbungskostenNEingabe {
                entfernung: ep.map(|(km, tage)| ep_eingabe(vz, km, tage, false, 0)),
                doppelte_haushaltsfuehrung: dhf.map(|(miete, monate, im_inland)| DhfEingabe {
                    veranlagungszeitraum: vz,
                    unterkunftskosten_monat: Euro::new(miete),
                    monate,
                    im_inland,
                }),
                verpflegung: vpf.map(|(t24, tan, t8)| VerpflegungEingabe {
                    veranlagungszeitraum: vz,
                    tage_24h: t24,
                    tage_an_abreise: tan,
                    tage_ueber_8h_eintaegig: t8,
                    vpf_tage_24h_nach_drei_monaten: 0,
                    vpf_tage_an_abreise_nach_drei_monaten: 0,
                    vpf_tage_ueber_8h_nach_drei_monaten: 0,
                    vpf_fruehstuecke_gestellt_anzahl: 0,
                    vpf_mittagessen_gestellt_anzahl: 0,
                    vpf_abendessen_gestellt_anzahl: 0,
                    vpf_mahlzeiten_gezahltes_entgelt: domain::Cent::new(0),
                    vpf_steuerfreie_erstattung_betrag: domain::Cent::new(0),
                }),
                uebernachtung: uen.map(|(kosten, monate, bisher, im_inland)| {
                    UebernachtungEingabe {
                        veranlagungszeitraum: vz,
                        uebernachtung_kosten_monat: Euro::new(kosten),
                        uebernachtung_monate: monate,
                        uebernachtung_monate_bisher: bisher,
                        uebernachtung_im_inland: im_inland,
                    }
                }),
                am_anschaffungskosten: am.map(Euro::new),
            };
            let got = werbungskosten_n(&e, &p).map(Euro::get);
            (!matches!(got, Ok(v) if v == erwartet))
                .then(|| format!("{name}: Rust {got:?}, Orakel {erwartet}"))
        })
        .collect();
    melde(&abweichend, WK_N.len());
}

/// § 19 i.V.m. § 9a S. 1 Nr. 1a (Scheibe `an_gesamt`): Bruttolohn minus Werbungskosten, mindestens der Arbeitnehmer-
/// Pauschbetrag von 1.230 EUR; die Werbungskosten kommen an.
#[test]
fn einkuenfte_nichtselbststaendig_mindestens_pauschbetrag() {
    let abweichend: Vec<String> = EINK_N
        .iter()
        .filter_map(|&(name, vz, brutto, wk, erwartet)| {
            let e = EinkuenfteNichtselbststaendigEingabe {
                bruttoarbeitslohn: Euro::new(brutto),
                werbungskosten: Euro::new(wk),
                veranlagungszeitraum: vz,
            };
            let got = einkuenfte_nichtselbststaendig(&e).map(Euro::get);
            (!matches!(got, Ok(v) if v == erwartet))
                .then(|| format!("{name}: Rust {got:?}, Orakel {erwartet}"))
        })
        .collect();
    melde(&abweichend, EINK_N.len());
}
