//! § 35 `EStG` / § 16 Abs. 1 `GewStG`: der WERT der zu zahlenden Gewerbesteuer
//! (`gewst_zu_zahlen`, Partner `gewst_zu_zahlen_partner`), im Standardlauf ohne `PARITY=1` und ohne
//! Python.
//!
//! Gegenstueck zu `tests/test_kz_bindung_durchgang.py::test_gewst_zu_zahlen_wird_berechnet_beide_personen`.
//! Gerechnet wird mit dem auf volle Euro ABGERUNDETEN Messbetrag mal Hebesatz (ELSTER-Regel
//! 100800013); der Messbetrag steht in Cent, der Hebesatz in Prozent, das Ergebnis wieder in Cent.
//!
//! Messung 2026-10-05 (Plan bvoll-waechter, Sonde QX4): ersetzt man in
//! `deklaration/ring_werte.rs::gewerbesteuer` das Produkt durch eine Summe, wird vorher allein
//! `ueberlauf_hermetisch.rs::ring_werte_melden_ueberlauf_und_rechnen_knapp_richtig` rot: ein Test
//! ueber Grenzwerte nahe `i64::MAX`, kein Alltagswert (5.000 EUR mal 400 %). Schrieb die Rechnung
//! das Partner-Ergebnis in das Feld der Person A (Mutant C4), blieb jener Test gruen; nur die Tests
//! dieser Datei werden dann rot.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::deklaration::mit_ring_werten;
use bescheid::testhilfe::{felder, index, params, store};
use domain::{PyWert, Vz};
use serde_json::{json, Value};

type Ev = (&'static str, Value, bool);

/// Die Ring-Werte (VZ 2025) auf einem Store aus den Events; der Wert von `feld_id`, wenn er gesetzt ist.
fn zu_zahlen(events: &[Ev], feld_id: &str) -> Option<PyWert> {
    let mut f = felder(&store(events));
    mit_ring_werten(&mut f, Some(Vz::Vz2025), params()).unwrap();
    f.get(feld_id).map(|x| x.wert.clone())
}

fn ganz(n: i64) -> PyWert {
    PyWert::Ganz(n)
}

/// 5.000 EUR Messbetrag mal 400 % = 20.000 EUR (2.000.000 Cent); Partnerbetrieb 3.000 EUR mal 450 %
/// = 13.500 EUR (1.350.000 Cent).
#[test]
fn beide_personen_rechnen_messbetrag_mal_hebesatz() {
    let ev: [Ev; 4] = [
        ("gewst_messbetrag", json!(500_000), true),
        ("gewst_hebesatz", json!(400), true),
        ("gewst_messbetrag_partner", json!(300_000), true),
        ("gewst_hebesatz_partner", json!(450), true),
    ];
    assert_eq!(zu_zahlen(&ev, "gewst_zu_zahlen"), Some(ganz(2_000_000)));
    assert_eq!(zu_zahlen(&ev, "gewst_zu_zahlen_partner"), Some(ganz(1_350_000)));
}

/// Der Messbetrag wird auf volle Euro ABGERUNDET, nicht gerundet: 5.000,99 EUR zaehlen als 5.000 EUR.
/// Aufgerundet (oder auf 5.001 gerundet) waeren es 20.004 EUR, vier Euro zu viel Steuer.
#[test]
fn der_messbetrag_wird_auf_volle_euro_abgerundet() {
    let ev: [Ev; 4] = [
        ("gewst_messbetrag", json!(500_099), true),
        ("gewst_hebesatz", json!(400), true),
        ("gewst_messbetrag_partner", json!(300_099), true),
        ("gewst_hebesatz_partner", json!(450), true),
    ];
    assert_eq!(zu_zahlen(&ev, "gewst_zu_zahlen"), Some(ganz(2_000_000)));
    assert_eq!(zu_zahlen(&ev, "gewst_zu_zahlen_partner"), Some(ganz(1_350_000)));
}

/// Ein Betrieb allein laesst die andere Seite unberuehrt — sonst entstuende eine Gewerbesteuerschuld
/// fuer einen Partner, der gar keinen Betrieb hat.
#[test]
fn ein_betrieb_allein_laesst_die_andere_seite_unberuehrt() {
    let nur_a: [Ev; 2] = [
        ("gewst_messbetrag", json!(500_000), true),
        ("gewst_hebesatz", json!(400), true),
    ];
    assert_eq!(zu_zahlen(&nur_a, "gewst_zu_zahlen"), Some(ganz(2_000_000)));
    assert_eq!(zu_zahlen(&nur_a, "gewst_zu_zahlen_partner"), None);
    let nur_b: [Ev; 2] = [
        ("gewst_messbetrag_partner", json!(300_000), true),
        ("gewst_hebesatz_partner", json!(450), true),
    ];
    assert_eq!(zu_zahlen(&nur_b, "gewst_zu_zahlen_partner"), Some(ganz(1_350_000)));
    assert_eq!(zu_zahlen(&nur_b, "gewst_zu_zahlen"), None);
}

/// Unbestaetigt bleibt unberechnet: ein vorlaeufiger Messbetrag (oder Hebesatz) darf keine feste
/// Steuerschuld in die Erklaerung schreiben.
#[test]
fn unbestaetigt_bleibt_unberechnet() {
    let vorlaeufiger_messbetrag: [Ev; 2] = [
        ("gewst_messbetrag", json!(500_000), false),
        ("gewst_hebesatz", json!(400), true),
    ];
    assert_eq!(zu_zahlen(&vorlaeufiger_messbetrag, "gewst_zu_zahlen"), None);
    let vorlaeufiger_hebesatz: [Ev; 2] = [
        ("gewst_messbetrag", json!(500_000), true),
        ("gewst_hebesatz", json!(400), false),
    ];
    assert_eq!(zu_zahlen(&vorlaeufiger_hebesatz, "gewst_zu_zahlen"), None);
}

/// Die Felder je Person: (Messbetrag, Hebesatz, Ergebnis).
const PERSONEN: [(&str, &str, &str); 2] = [
    ("gewst_messbetrag", "gewst_hebesatz", "gewst_zu_zahlen"),
    (
        "gewst_messbetrag_partner",
        "gewst_hebesatz_partner",
        "gewst_zu_zahlen_partner",
    ),
];

/// Grenzfaelle 0 und 1 fuer Messbetrag und Hebesatz, je Person. Die Bedingung der Rechnung heisst
/// `m > 0 && s > 0`: eine Null auf einer der beiden Seiten ergibt KEINEN Ring-Wert (sonst stuende
/// 0 EUR Gewerbesteuer als berechnete Tatsache da, obwohl kein Hebesatz vorliegt), eine Eins rechnet.
/// Bei einem Messbetrag unter 100 Cent ist der abgerundete Betrag 0 EUR; das Ergebnis 0 wird gesetzt
/// (das Schema laesst es zu, siehe `deklaration_schreibt_nullen_nur_wo_das_schema_sie_erlaubt`).
///
/// Erwartung = Rust heute = Python (`bescheid_deklaration._mit_ring_werten`, `produkt/`, nur gelesen
/// und mit denselben acht Eingaben ausgefuehrt: Messbetrag 0, Hebesatz 0, 1 Cent, Hebesatz 1,
/// 99 und 100 Cent, negative Werte; alle Ergebnisse gleich).
/// Mutanten, die vorher in allen Tests gruen blieben: `m >= 0 && s > 0` und `m > 0 && s >= 0`.
#[test]
fn nullen_und_einsen_an_der_grenze_je_person() {
    // (Messbetrag in Cent, Hebesatz in Prozent, erwarteter Ring-Wert in Cent)
    let faelle: [(i64, i64, Option<i64>); 9] = [
        (0, 400, None),
        (500_000, 0, None),
        (0, 0, None),
        (1, 400, Some(0)),
        (500_000, 1, Some(5_000)),
        (1, 1, Some(0)),
        (99, 400, Some(0)),
        (100, 1, Some(1)),
        (-1, 400, None),
    ];
    for (messbetrag, hebesatz, erwartet) in faelle {
        for (mfeld, hfeld, ziel) in PERSONEN {
            let ev: [Ev; 2] = [
                (mfeld, json!(messbetrag), true),
                (hfeld, json!(hebesatz), true),
            ];
            assert_eq!(
                zu_zahlen(&ev, ziel),
                erwartet.map(ganz),
                "{ziel}: Messbetrag {messbetrag} Cent, Hebesatz {hebesatz} %"
            );
        }
    }
}

/// D19 an derselben Grenze: das Schema (E10-2025) fuehrt E0801606 (Messbetrag) und E0801704
/// (zu zahlende Gewerbesteuer) als `GanzzahlNichtNeg…` (0 erlaubt), den Hebesatz E0801705 als
/// `GanzzahlPos…` (0 verboten). Ein Messbetrag unter 100 Cent steht deshalb als 0 in der
/// Deklaration, ein Hebesatz 0 entfaellt. Die Menge der verbotenen Nullen prueft gegen das Schema
/// `rust/elster/tests/eigenschaften.rs::kz_mengen_aus_xsd`; hier steht ihre Wirkung auf diese drei Kz.
/// Person B schreibt in `person_b`, dieselben Kz.
#[test]
fn deklaration_schreibt_nullen_nur_wo_das_schema_sie_erlaubt() {
    let deklaration = |ev: &[Ev]| {
        let mut f = felder(&store(ev));
        mit_ring_werten(&mut f, Some(Vz::Vz2025), params()).unwrap();
        elster::deklariere(&f, index(), 2025, None).unwrap()
    };
    let wert = |m: &std::collections::BTreeMap<String, Value>, kz: &str| m.get(kz).and_then(Value::as_i64);
    for (mfeld, hfeld, _) in PERSONEN {
        let partner = mfeld.ends_with("_partner");
        let bucket = |d: &elster::Deklaration| {
            if partner { d.person_b.clone() } else { d.deklaration.clone() }
        };
        // 1 Cent Messbetrag: abgerundet 0 EUR; E0801606 und E0801704 tragen die 0, der Hebesatz seine 400.
        let d = deklaration(&[(mfeld, json!(1), true), (hfeld, json!(400), true)]);
        let m = bucket(&d);
        assert_eq!(wert(&m, "E0801606"), Some(0), "{mfeld}: E0801606 darf 0 tragen");
        assert_eq!(wert(&m, "E0801704"), Some(0), "{mfeld}: E0801704 darf 0 tragen");
        assert_eq!(wert(&m, "E0801705"), Some(400), "{hfeld}");
        // Hebesatz 0: das Schema verbietet die 0, E0801705 entfaellt; kein Ring-Wert, also kein E0801704.
        let d = deklaration(&[(mfeld, json!(500_000), true), (hfeld, json!(0), true)]);
        let m = bucket(&d);
        assert_eq!(wert(&m, "E0801606"), Some(5_000), "{mfeld}");
        assert_eq!(wert(&m, "E0801705"), None, "{hfeld}: Hebesatz 0 muss entfallen (D19)");
        assert_eq!(wert(&m, "E0801704"), None, "{mfeld}: ohne Hebesatz keine berechnete Steuer");
    }
}
