//! Ueberlaufstellen der Bescheid-Schicht, die `ueberlauf_hermetisch.rs` und `ueberlauf_zweige_hermetisch.rs` NICHT erreichen, im
//! Standardlauf (ohne `PARITY=1`, ohne Python). Bericht h8-ueberlauf-luecke.
//!
//! Zwei Stellen, beide ueber die `pub`-Schnittstelle erreichbar, ueber die HTTP-Haut nicht (Eingangsgrenzen im Bericht), und eine
//! Gegenprobe der Feld-Naht zur PV-Stelle der Engine (`pv_anzahl_einheiten` hat keine Grenze, die Haut reicht sie durch):
//!
//! * `einreichung.rs`, `u16::try_from(jahr)`: das Jahr der Akte kommt aus der Falldatei. `POST /fall` nimmt nur 2024..=2026; eine
//!   von Hand geschriebene Akte kann jedes Jahr tragen. Die Mutante (`jahr as u16`) wickelt 67561 (= 2025 + 65536) auf 2025 und
//!   reicht die Akte als Jahr 2025 ein. Rust lehnt das Jahr ab (`EinreichFehler::Veranlagungszeitraum`, mit dem Rohwert) -- wie
//!   `deklaration` und `stand` (A7-A13, `api/tests/ueberlauf_klassen_hermetisch.rs`; dort ist die Ablehnung fuer `deklaration` mit
//!   `orakel_a11.py` gegen Python belegt, `ValueError`). Fuer `einreichen` ist der Python-Wert NICHT gemessen: der Test pinnt Rusts
//!   Verhalten (ablehnen, nie wickeln), kein Orakel stuetzt es.
//! * `lib.rs`, `dezimal_plus` (ueber `an_gesamt_sperrgrund`, die Summe der Verpflegungstage): Floats im Store sind aus der HTTP-Haut
//!   nicht erreichbar (Typ `int`), wohl aus einer von Hand geschriebenen Akte. Zwei `Decimal`-Summanden ueber `Decimal::MAX / 2`
//!   laufen ueber; Rust saettigt (D18, `ponytail` an `zahl_dezimal`), die Mutante (`a + b`) PANIKT. Rot ist hier der Absturz.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::deklaration::{an_gesamt_sperrgrund, einreichungs_xml, EinreichFehler};
use bescheid::einkuenfte::laufender_gewinn;
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::{BescheidFehler, Instanzquelle};
use domain::{Sperrgrund, Vz};
use engine::zugriff::teil1::fehler::EngineFehler;
use serde_json::json;
use store::Store;

/// Eine Akte der Scheibe `gesamt` (traegt alle Stammdatenfelder) mit dem gegebenen Jahr, ohne Events.
fn akte(jahr: &str) -> Store {
    let datei = serde_json::from_str(&format!(
        r#"{{"version": 1, "veranlagungszeitraum": {jahr}, "scheibe": "gesamt", "events": []}}"#
    ))
    .unwrap();
    Store::aus_datei(datei)
}

/// Ein Jahr ausserhalb `u16` wird nie auf ein anderes Jahr gewickelt: 67561 = 2025 + 65536 und -63511 = 2025 - 65536 sind die
/// zwei Werte, die `as u16` auf 2025 faltet (ebenso 4294969321 = 2025 + 2^32). 2023 und 2027 passen in `u16`, sind aber kein
/// `Vz` (die Aufzaehlung kennt 2024..=2026): die zweite Stelle der Kette, `Vz::try_from(j)`, darf sie nicht auf 2025 setzen.
/// Gegenprobe: 2025 selbst kommt am Jahres-Check vorbei (die Akte ist leer, also
/// meldet der naechste Schritt etwas anderes als `Veranlagungszeitraum`) -- sonst wuerde der Test nichts messen.
#[test]
fn einreichung_lehnt_ein_jahr_ausserhalb_u16_ab_statt_es_zu_wickeln() {
    let mut falsch = Vec::new();
    for jahr in [
        "67561",
        "-63511",
        "4294969321",
        "9223372036854775807",
        "2023",
        "2027",
    ] {
        let r = einreichungs_xml(&akte(jahr), index(), params(), "BY", None);
        let erwartet = jahr.parse::<i128>().unwrap();
        match r {
            Err(EinreichFehler::Veranlagungszeitraum(j)) if i128::from(j) == erwartet => {}
            anders => falsch.push(format!("Jahr {jahr}: {anders:?}")),
        }
    }
    let gegenprobe = einreichungs_xml(&akte("2025"), index(), params(), "BY", None);
    if matches!(gegenprobe, Err(EinreichFehler::Veranlagungszeitraum(_))) {
        falsch.push(format!("Gegenprobe Jahr 2025: {gegenprobe:?}"));
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// Summe der Verpflegungstage im K2-Guard: zwei Floats ueber `Decimal::MAX / 2` (7,9e28) saettigen, sie wickeln und panieren nicht.
/// Beide Vorzeichen (die Saettigung geht Richtung Vorzeichen des zweiten Summanden). Gegenprobe: zwei Ganzzahlen summieren sich.
#[test]
fn verpflegungstage_summe_ausserhalb_decimal_saettigt_statt_zu_paniken() {
    let q = Instanzquelle {
        store: None,
        bindung: None,
        nur_bestaetigt: true,
    };
    let guard = |a: f64, b: f64| {
        let f = felder(&store(&[
            ("tage_24h", json!(a), true),
            ("tage_an_abreise", json!(b), true),
        ]));
        an_gesamt_sperrgrund(&f, None, Some(Vz::Vz2025), &q)
    };
    // Summe 1,4e29 > Decimal::MAX: saettigt auf MAX, `positiv()`, kein bestaetigter Monat am Ort -> der Ring sperrt.
    assert_eq!(
        guard(7.0e28, 7.0e28).unwrap(),
        Some(Sperrgrund::VerpflegungDreimonatsfristAufteilungOffen)
    );
    // Summe -1,4e29 < Decimal::MIN: saettigt auf MIN, nicht `positiv()`, kein Verpflegungsfall -> keine Sperre.
    assert_eq!(guard(-7.0e28, -7.0e28).unwrap(), None);
    // Gegenprobe: 5 + 5 Tage rechnen exakt, positiv, derselbe Grund.
    assert_eq!(
        guard(5.0, 5.0).unwrap(),
        Some(Sperrgrund::VerpflegungDreimonatsfristAufteilungOffen)
    );
}

/// § 3 Nr. 72 `EStG` (PV-Freistellung) an der Feld-Naht: `pv_anzahl_einheiten` ist ein `int` ohne Bereich, die Haut reicht jeden
/// `i64` durch. `Einheiten * 30` ausserhalb `i64` meldet `Ueberlauf("pv")` (Engine, Mutante `wrapping_mul` liefert 0 EUR Freistellung);
/// bei der Grenze `i64::MAX / 30` rechnet die Stelle (900 EUR Freistellung, der Gewinn 0 bleibt 0). Python rechnet mit beliebig
/// grossen `int` und stellt die 900 EUR frei: ein "Zwischenprodukt ausserhalb i64" (fail-closed-Konvention, keine Python-Stuetze).
#[test]
fn pv_einheiten_mal_30_ausserhalb_i64_meldet_ueberlauf_an_der_feld_naht() {
    let q = Instanzquelle {
        store: None,
        bindung: None,
        nur_bestaetigt: true,
    };
    let rechne = |einheiten: i64| {
        let f = felder(&store(&[
            ("pv_auf_gebaeude", json!(true), true),
            ("pv_einnahmen", json!(90_000), true),
            ("pv_bruttoleistung_kwp", json!(1), true),
            ("pv_anzahl_einheiten", json!(einheiten), true),
        ]));
        laufender_gewinn(&f, &q)
    };
    let sperre = rechne(i64::MAX / 30 + 1);
    assert!(
        matches!(
            sperre,
            Err(BescheidFehler::Engine(EngineFehler::Ueberlauf("pv")))
        ),
        "{sperre:?}"
    );
    let (gewinn, _) = rechne(i64::MAX / 30).unwrap();
    assert_eq!(gewinn.get(), 0, "Gegenprobe i64::MAX / 30 Einheiten");
}
