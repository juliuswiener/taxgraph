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
use bescheid::testhilfe::{felder, params, store};
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
