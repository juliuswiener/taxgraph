//! Ueberlaufpfade der Rechenkern-Zugriffe (`engine::zugriff::teil1`): `p7_linear_afa`, `werbungskosten_n` mit doppelter
//! Haushaltsfuehrung, Verpflegung und Uebernachtung, und `ep_ab_21km` melden `EngineFehler::Ueberlauf(<Stelle>)` statt eine
//! falsche Zahl zu liefern, im Standardlauf (ohne `PARITY=1`, ohne Python).
//!
//! Python rechnet mit beliebig grossen Ganzzahlen; Rust rechnet in `i64` mit `checked_*` und meldet `Ueberlauf` (fail-closed
//! statt Wrap-Around, `zugriff/teil1/fehler.rs`: "kein Gegenstueck" in Python). Ein Wrap-Around lieferte eine falsche Zahl in
//! den Bescheid, keinen Fehler. Kein Bestandstest prueft einen dieser Pfade. Gemessen am 2026-10-03 auf 4a2f6ea4 (Bericht
//! h8-hermetisch4): neun Mutationen am Aufrufort, je ein `checked_*` durch `wrapping_*` ersetzt (der Fehler entfaellt still):
//! U1 `AfA` pro rata, U2 dHf Miete mal Monate, U3 Verpflegung Tage mal Pauschale mal 100, U4 Verpflegung Tage minus Tage nach der
//! Frist, U5 Verpflegung Fruehstuecke mal Kuerzung, U6 Uebernachtung Kosten mal Monate, U7 Uebernachtung `48 - Monate bisher`, U8
//! Summe der Zweige in `werbungskosten_n`, U9 `ep_ab_21km` Arbeitstage mal Kilometer. Alle neun lassen `cargo test -p engine`
//! gruen (154 passed, 0 failed). Mit diesen Tests werden alle neun rot, jede mit dem Test der Funktion (Tabelle im Bericht).
//! Die Fehlerart ist Teil der Erwartung: der Test verlangt `Ueberlauf` MIT der Marke der Stelle, nicht irgendeinen Fehler und
//! keine Panik.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: der Python-Wert jedes Falls ist die Ausgabe des Python-Orakels (`runner._dhf_abzug`,
//! `_verpflegung_abzug`, `_uebernachtung_abzug`, `catala_p7_linear_afa`, `catala_ep_ab_21km`, `catala_werbungskosten_n`, leere
//! Wegwerf-Datenwurzel, VZ 2025); Orakel-Skript und Lauf: Anlagen zum Bericht. Jeder Fall traegt eine von drei Marken im Namen:
//! "Python-Wert ausserhalb i64": es gibt keinen `i64`-Wert, Rust MUSS `Ueberlauf` melden; das Orakel stuetzt das. "nur ein
//! Zwischenprodukt ausserhalb i64": Python liefert einen gueltigen Wert (z. B. `p7_linear_afa` mit 9.223.372.036.854.775.807 EUR
//! gibt 9.223.372.036.854.775.807), Rust meldet `Ueberlauf`, weil der Zwischenschritt nicht passt. Das ist die dokumentierte
//! fail-closed-Konvention von Rust und KEINE Python-Stuetze: Rust ist hier strenger als Python (Eingaben ab etwa 9 Trillionen EUR
//! oder Tagen, kein realer Fall, nie eine falsche Zahl). "passt gerade noch": alles passt, Rust MUSS den Python-Wert liefern;
//! diese Gegenproben zeigen, dass nicht jede grosse Zahl fehlschlaegt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::path::Path;
use std::str::FromStr;

use bindung::Params;
use domain::{Cent, Euro, Km, Vz};
use engine::zugriff::teil1::afa::{p7_linear_afa, P7LinearAfaEingabe};
use engine::zugriff::teil1::fehler::EngineFehler;
use engine::zugriff::teil1::reisekosten::{DhfEingabe, UebernachtungEingabe, VerpflegungEingabe};
use engine::zugriff::teil1::werbungskosten::{
    ep_ab_21km, werbungskosten_n, EntfernungspauschaleEingabe, WerbungskostenNEingabe,
};
use rust_decimal::Decimal;

/// `Some(marke)`: Rust MUSS `EngineFehler::Ueberlauf(marke)` melden. `None`: Rust MUSS den Wert `python` liefern.
type Marke = Option<&'static str>;

/// `(Name, Anschaffungskosten EUR, Nutzungsdauer, Monat, Anschaffungsjahr, Python-Wert, Marke)`.
const AFA: &[(&str, i64, i64, i64, bool, i128, Marke)] = &[
    ("Anschaffungskosten 9223372036854775807 EUR, Nutzungsdauer 1, Monat 1, Anschaffungsjahr [nur ein Zwischenprodukt ausserhalb i64]", i64::MAX, 1, 1, true, 9_223_372_036_854_775_807, Some("afa pro rata")),
    ("Anschaffungskosten 768614336404564650 EUR, Nutzungsdauer 1, Monat 1, Anschaffungsjahr [passt gerade noch]", 768_614_336_404_564_650, 1, 1, true, 768_614_336_404_564_650, None),
    ("Anschaffungskosten 9223372036854775807 EUR, Nutzungsdauer 1, Monat 1, Folgejahr [passt gerade noch]", i64::MAX, 1, 1, false, 9_223_372_036_854_775_807, None),
    ("Anschaffungskosten 9223372036854775807 EUR, Nutzungsdauer 1, Monat 12, Anschaffungsjahr [passt gerade noch]", i64::MAX, 1, 12, true, 768_614_336_404_564_650, None),
];

/// `(Name, Miete je Monat EUR, Monate, Inland, Python-Wert, Marke)`, VZ 2025.
const DHF: &[(&str, i64, i64, bool, i128, Marke)] = &[
    (
        "Miete 1000 EUR, 9223372036854775807 Monate, Inland [Python-Wert ausserhalb i64]",
        1000,
        i64::MAX,
        true,
        9_223_372_036_854_775_807_000,
        Some("dhf"),
    ),
    (
        "Miete 1000 EUR, 9223372036854775 Monate, Inland [passt gerade noch]",
        1000,
        9_223_372_036_854_775,
        true,
        9_223_372_036_854_775_000,
        None,
    ),
    (
        "Miete 1000 EUR, -9223372036854775808 Monate, Inland [Python-Wert ausserhalb i64]",
        1000,
        i64::MIN,
        true,
        -9_223_372_036_854_775_808_000,
        Some("dhf"),
    ),
];

/// `(Name, [24-h-Tage, An-/Abreisetage, Tage ueber 8 h, je Kategorie nach der Dreimonatsfrist (3x), Fruehstuecke, Mittagessen,
/// Abendessen], Python-Wert, Marke)`, VZ 2025.
const VPF: &[(&str, [i64; 9], i128, Marke)] = &[
    ("24-h-Tage = i64::MAX [Python-Wert ausserhalb i64]", [i64::MAX, 0, 0, 0, 0, 0, 0, 0, 0], 258_254_417_031_933_722_596, Some("vpf s24")),
    ("24-h-Tage = i64::MAX / 2800 (28 EUR * 100 passt gerade) [passt gerade noch]", [3_294_061_441_733_848, 0, 0, 0, 0, 0, 0, 0, 0], 92_233_720_368_547_744, None),
    ("An-/Abreisetage = i64::MAX [Python-Wert ausserhalb i64]", [0, i64::MAX, 0, 0, 0, 0, 0, 0, 0], 129_127_208_515_966_861_298, Some("vpf sa")),
    ("Tage ueber 8 h = i64::MAX [Python-Wert ausserhalb i64]", [0, 0, i64::MAX, 0, 0, 0, 0, 0, 0], 129_127_208_515_966_861_298, Some("vpf s8")),
    ("24-h-Tage = i64::MIN, Tage nach der Frist = i64::MAX [nur ein Zwischenprodukt ausserhalb i64]", [i64::MIN, 0, 0, i64::MAX, 0, 0, 0, 0, 0], 0, Some("vpf tage")),
    ("Fruehstuecke = i64::MAX [nur ein Zwischenprodukt ausserhalb i64]", [0, 0, 0, 0, 0, 0, i64::MAX, 0, 0], 0, Some("vpf k28")),
    ("Fruehstuecke = i64::MAX / 560 (560 ct Kuerzung je Fruehstueck passen gerade) [passt gerade noch]", [0, 0, 0, 0, 0, 0, 16_470_307_208_669_242, 0, 0], 0, None),
    ("Mittagessen = i64::MAX [nur ein Zwischenprodukt ausserhalb i64]", [0, 0, 0, 0, 0, 0, 0, i64::MAX, 0], 0, Some("vpf k28")),
    ("Abendessen = i64::MAX [nur ein Zwischenprodukt ausserhalb i64]", [0, 0, 0, 0, 0, 0, 0, 0, i64::MAX], 0, Some("vpf k28")),
];

/// `(Name, Kosten je Monat EUR, Monate, Monate bisher, Inland, Python-Wert, Marke)`, VZ 2025.
const UEN: &[(&str, i64, i64, i64, bool, i128, Marke)] = &[
    ("Kosten 9223372036854775807 EUR, 48 Monate, bisher 0, Inland [Python-Wert ausserhalb i64]", i64::MAX, 48, 0, true, 442_721_857_769_029_238_736, Some("uebernachtung")),
    ("Kosten 192153584101141162 EUR, 48 Monate, bisher 0, Inland [passt gerade noch]", 192_153_584_101_141_162, 48, 0, true, 9_223_372_036_854_775_776, None),
    ("Kosten 600 EUR, 1 Monate, bisher -9223372036854775808, Inland [nur ein Zwischenprodukt ausserhalb i64]", 600, 1, i64::MIN, true, 600, Some("uen 48")),
    ("Kosten 600 EUR, 1 Monate, bisher 9223372036854775807, Inland [passt gerade noch]", 600, 1, i64::MAX, true, 600, None),
    ("Kosten 9223372036854775807 EUR, 49 Monate, bisher 0, Ausland [Python-Wert ausserhalb i64]", i64::MAX, 49, 0, false, 451_945_229_805_884_014_543, Some("uebernachtung")),
];

/// `(Name, dHf-Miete EUR, dHf-Monate, Uebernachtung EUR, Uebernachtung-Monate, Python-Wert, Marke)`, VZ 2025.
const WK: &[(&str, i64, i64, i64, i64, i128, Marke)] = &[
    ("dHf 1000 EUR x 9223372036854775 Monate plus Uebernachtung 1000 EUR x 1 Monat [Python-Wert ausserhalb i64]", 1000, 9_223_372_036_854_775, 1000, 1, 9_223_372_036_854_776_000, Some("wk uen")),
    ("dHf 1000 EUR x 9223372036854775 Monate plus Uebernachtung 807 EUR x 1 Monat [passt gerade noch]", 1000, 9_223_372_036_854_775, 807, 1, 9_223_372_036_854_775_807, None),
];

/// `(Name, Arbeitstage, Python-Wert, Marke)`: 30 km, ohne Kfz, ohne `OePNV`, VZ 2025.
const AB21: &[(&str, i64, i128, Marke)] = &[
    (
        "30 km, 9223372036854775807 Arbeitstage, ohne Kfz [nur ein Zwischenprodukt ausserhalb i64]",
        i64::MAX,
        0,
        Some("ab21_roh"),
    ),
    (
        "30 km, 15372286728091293 Arbeitstage, ohne Kfz [passt gerade noch]",
        15_372_286_728_091_293,
        0,
        None,
    ),
];

fn params() -> Params {
    Params::lade(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// Nur ein Zweig von `werbungskosten_n`; alle anderen Schluessel fehlen.
fn nur() -> WerbungskostenNEingabe {
    WerbungskostenNEingabe {
        entfernung: None,
        doppelte_haushaltsfuehrung: None,
        verpflegung: None,
        uebernachtung: None,
        am_anschaffungskosten: None,
    }
}

fn dhf_eingabe(miete: i64, monate: i64, im_inland: bool) -> DhfEingabe {
    DhfEingabe {
        veranlagungszeitraum: Vz::Vz2025,
        unterkunftskosten_monat: Euro::new(miete),
        monate,
        im_inland,
    }
}

fn uen_eingabe(kosten: i64, monate: i64, bisher: i64, im_inland: bool) -> UebernachtungEingabe {
    UebernachtungEingabe {
        veranlagungszeitraum: Vz::Vz2025,
        uebernachtung_kosten_monat: Euro::new(kosten),
        uebernachtung_monate: monate,
        uebernachtung_monate_bisher: bisher,
        uebernachtung_im_inland: im_inland,
    }
}

/// Meldet den Fall, wenn Rust nicht tut, was das Orakel verlangt: bei `None` genau den Python-Wert, bei `Some(marke)` genau
/// `Ueberlauf(marke)` -- weder einen Wert (Wrap-Around) noch eine andere Fehlerart.
fn abweichung(
    name: &str,
    got: &Result<Euro, EngineFehler>,
    python: i128,
    marke: Marke,
) -> Option<String> {
    let passt = match (got, marke) {
        (Ok(e), None) => i128::from(e.get()) == python,
        (Err(EngineFehler::Ueberlauf(m)), Some(soll)) => *m == soll,
        _ => false,
    };
    (!passt).then(|| {
        let soll = marke.map_or_else(
            || "den Python-Wert".to_owned(),
            |m| format!("Ueberlauf({m:?})"),
        );
        format!("{name}: Rust {got:?}, Python {python}, erwartet {soll}")
    })
}

/// Alle Faelle durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn melde(abweichend: &[String], faelle: usize) {
    assert!(
        abweichend.is_empty(),
        "{} von {faelle} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len()
    );
}

/// § 7 Abs. 1 S. 4 (Scheibe `n_vor_gwg`): Jahresbetrag mal (13 - Monat) im Anschaffungsjahr. Das Zwischenprodukt passt nicht
/// in `i64`, auch wenn das Ergebnis noch passt.
#[test]
fn afa_pro_rata_meldet_ueberlauf_statt_zu_wrappen() {
    let abweichend: Vec<String> = AFA
        .iter()
        .filter_map(
            |&(name, euro, nutzungsdauer, monat, anschaffungsjahr, python, marke)| {
                let e = P7LinearAfaEingabe {
                    anschaffungskosten_cent: Cent::new(0),
                    anschaffungskosten: Euro::new(euro),
                    nutzungsdauer,
                    anschaffung_monat: monat,
                    ist_anschaffungsjahr: anschaffungsjahr,
                };
                abweichung(name, &p7_linear_afa(&e), python, marke)
            },
        )
        .collect();
    melde(&abweichend, AFA.len());
}

/// § 9 Abs. 1 S. 3 Nr. 5 (doppelte Haushaltsfuehrung): gekappte Miete mal Monate.
#[test]
fn dhf_miete_mal_monate_meldet_ueberlauf() {
    let p = params();
    let abweichend: Vec<String> = DHF
        .iter()
        .filter_map(|&(name, miete, monate, im_inland, python, marke)| {
            let e = WerbungskostenNEingabe {
                doppelte_haushaltsfuehrung: Some(dhf_eingabe(miete, monate, im_inland)),
                ..nur()
            };
            abweichung(name, &werbungskosten_n(&e, &p), python, marke)
        })
        .collect();
    melde(&abweichend, DHF.len());
}

/// § 9 Abs. 4a (Verpflegung): jede Rechenstufe (Tage mal Pauschale mal 100, Tage minus Tage nach der Frist, Mahlzeiten mal
/// Kuerzung je Mahlzeit) meldet ihren eigenen `Ueberlauf`; die Marke nennt die Stufe.
#[test]
fn verpflegung_meldet_ueberlauf_je_rechenstufe() {
    let p = params();
    let abweichend: Vec<String> = VPF
        .iter()
        .filter_map(|&(name, t, python, marke)| {
            let e = WerbungskostenNEingabe {
                verpflegung: Some(VerpflegungEingabe {
                    veranlagungszeitraum: Vz::Vz2025,
                    tage_24h: t[0],
                    tage_an_abreise: t[1],
                    tage_ueber_8h_eintaegig: t[2],
                    vpf_tage_24h_nach_drei_monaten: t[3],
                    vpf_tage_an_abreise_nach_drei_monaten: t[4],
                    vpf_tage_ueber_8h_nach_drei_monaten: t[5],
                    vpf_fruehstuecke_gestellt_anzahl: t[6],
                    vpf_mittagessen_gestellt_anzahl: t[7],
                    vpf_abendessen_gestellt_anzahl: t[8],
                    vpf_mahlzeiten_gezahltes_entgelt: Cent::new(0),
                    vpf_steuerfreie_erstattung_betrag: Cent::new(0),
                }),
                ..nur()
            };
            abweichung(name, &werbungskosten_n(&e, &p), python, marke)
        })
        .collect();
    melde(&abweichend, VPF.len());
}

/// § 9 Abs. 1 S. 3 Nr. 5a (Uebernachtung): `48 - Monate bisher` und Kosten mal Monate melden je ihren `Ueberlauf`.
#[test]
fn uebernachtung_meldet_ueberlauf_je_rechenstufe() {
    let p = params();
    let abweichend: Vec<String> = UEN
        .iter()
        .filter_map(
            |&(name, kosten, monate, bisher, im_inland, python, marke)| {
                let e = WerbungskostenNEingabe {
                    uebernachtung: Some(uen_eingabe(kosten, monate, bisher, im_inland)),
                    ..nur()
                };
                abweichung(name, &werbungskosten_n(&e, &p), python, marke)
            },
        )
        .collect();
    melde(&abweichend, UEN.len());
}

/// § 9 (Scheibe `an_gesamt`): die Summe der Zweige in `werbungskosten_n` meldet `Ueberlauf`, wenn zwei Zweige zusammen nicht
/// mehr in `i64` passen; zwei Zweige, die gerade noch passen, summieren sich richtig.
#[test]
fn werbungskosten_summe_meldet_ueberlauf() {
    let p = params();
    let abweichend: Vec<String> = WK
        .iter()
        .filter_map(|&(name, miete, dmonate, kosten, umonate, python, marke)| {
            let e = WerbungskostenNEingabe {
                doppelte_haushaltsfuehrung: Some(dhf_eingabe(miete, dmonate, true)),
                uebernachtung: Some(uen_eingabe(kosten, umonate, 0, true)),
                ..nur()
            };
            abweichung(name, &werbungskosten_n(&e, &p), python, marke)
        })
        .collect();
    melde(&abweichend, WK.len());
}

/// § 9 Abs. 1 S. 3 Nr. 4 (Scheibe `ep`): der erhoehte Teil ab dem 21. km, Arbeitstage mal Kilometer mal Satz.
#[test]
fn ep_ab_21km_meldet_ueberlauf() {
    let p = params();
    let abweichend: Vec<String> = AB21
        .iter()
        .filter_map(|&(name, tage, python, marke)| {
            let e = EntfernungspauschaleEingabe {
                veranlagungszeitraum: Vz::Vz2025,
                entfernung_km_roh: Km::new(Decimal::from_str("30").unwrap()),
                arbeitstage: tage,
                eigenes_oder_ueberlassenes_kfz: false,
                oepnv_kosten_jahr: Euro::new(0),
            };
            abweichung(name, &ep_ab_21km(&e, &p), python, marke)
        })
        .collect();
    melde(&abweichend, AB21.len());
}
