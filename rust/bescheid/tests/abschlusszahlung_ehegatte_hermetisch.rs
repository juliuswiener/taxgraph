//! Abschlusszahlung (§ 36 Abs. 2+4 `EStG`) bei Zusammenveranlagung: die einbehaltene Lohnsteuer des Ehegatten
//! (`p36_lohnsteuer_partner`) wird mit angerechnet. Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** `abschlusszahlung_cent` zog bisher nur die Lohnsteuer von Person A ab. Hatte der Ehegatte Lohn, fiel die
//! angezeigte Nachzahlung um genau seine Lohnsteuer zu hoch aus (oder die Erstattung zu niedrig). Die Erklaerung an ELSTER war
//! nicht betroffen, nur die Zahl, die der Nutzer vorher sieht. Abweichung Nr. 38, Entscheidung
//! `abschlusszahlung-rechnet-die-lohnsteuer-beider-ehegatten-an`; Python (eingefroren) liest das Feld nie.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: von Hand nach dem Gesetz gerechnet, kein Wert aus dem Rust-Code abgelesen. § 36 Abs. 2 Satz 1
//! Nr. 2 rechnet die durch Steuerabzug erhobene Steuer an, soweit sie auf die bei der Veranlagung erfassten Einkuenfte
//! entfaellt (ohne Person). § 36 Abs. 3 Satz 2: "Bei den durch Steuerabzug erhobenen Steuern ist jeweils die Summe der Betraege
//! einer einzelnen Abzugsteuer aufzurunden." Die Lohnsteuer beider Ehegatten ist EINE Abzugsteuer: erst addieren, dann einmal
//! aufrunden (`sources/gesetze-im-internet/estg_p36_2026-07-11.txt`). Die festgesetzte Steuer ist in allen Faellen 5.000 EUR.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::testhilfe::{felder, store};
use bescheid::zweige::abschlusszahlung_cent;
use bescheid::BescheidFehler;
use domain::Cent;
use serde_json::{json, Value};

/// Die festgesetzte Einkommensteuer der Faelle, CENT.
const FESTGESETZT: i64 = 500_000;

/// `abschlusszahlung_cent` auf einem Store aus `(feld, wert, bestaetigt)`; der Rueckgabewert in CENT oder `None`.
fn rest(paare: &[(&str, Value, bool)]) -> Option<i64> {
    abschluss(paare, FESTGESETZT).unwrap().map(Cent::get)
}

fn abschluss(
    paare: &[(&str, Value, bool)],
    est_cent: i64,
) -> Result<Option<Cent>, BescheidFehler> {
    abschlusszahlung_cent(&felder(&store(paare)), Cent::new(est_cent))
}

/// AK1: Bei Zusammenveranlagung zieht die Abschlusszahlung die Lohnsteuer BEIDER Ehegatten ab, dazu Vorauszahlungen und die drei
/// Kapitalertragsteuer-Posten. Jeder Posten hat einen anderen Betrag: vertauscht oder verliert die Rechnung einen, aendert sich
/// die Zahl. 5.000 - 1.000 (A) - 700 (Ehegatte) - 300 - 200 - 10 - 20 = 2.770 EUR.
#[test]
fn zusammen_zieht_die_lohnsteuer_beider_ehegatten_ab() {
    let r = rest(&[
        ("veranlagung", json!("zusammen"), true),
        ("p36_lohnsteuer", json!(100_000), true),
        ("p36_lohnsteuer_partner", json!(70_000), true),
        ("p36_vorauszahlungen", json!(30_000), true),
        ("p36_kapitalertragsteuer", json!(20_000), true),
        ("p36_kapitalertragsteuer_solz", json!(1_000), true),
        ("p36_kapitalertragsteuer_kist", json!(2_000), true),
    ]);
    assert_eq!(r, Some(277_000));
}

/// AK3: Ohne Zusammenveranlagung zaehlt ein bestaetigter Wert im Feld des Ehegatten nicht, auch nicht bei offener Veranlagung.
/// 5.000 - 1.000 = 4.000 EUR, mit und ohne den Wert des Ehegatten.
#[test]
fn ohne_zusammenveranlagung_zaehlt_die_lohnsteuer_des_ehegatten_nicht() {
    let a = ("p36_lohnsteuer", json!(100_000), true);
    let b = ("p36_lohnsteuer_partner", json!(70_000), true);
    for veranlagung in [Some("einzel"), None] {
        let mut paare = vec![a.clone(), b.clone()];
        if let Some(v) = veranlagung {
            paare.push(("veranlagung", json!(v), true));
        }
        assert_eq!(rest(&paare), Some(400_000), "veranlagung {veranlagung:?}");
    }
    // Gegenprobe: dieselben Felder, nur zusammen veranlagt, rechnen den Wert an.
    let zusammen = [("veranlagung", json!("zusammen"), true), a, b];
    assert_eq!(rest(&zusammen), Some(330_000));
}

/// AK2: Die Regel "ein einziges bestaetigtes Anrechnungsfeld genuegt, jedes fehlende zaehlt 0" gilt auch fuer das Feld des
/// Ehegatten. Ist nur sein Feld bestaetigt, steht eine Zahl da (vorher `None`). Ist keines bestaetigt, bleibt es `None`.
#[test]
fn nur_die_lohnsteuer_des_ehegatten_bestaetigt_gibt_eine_zahl() {
    let zusammen = ("veranlagung", json!("zusammen"), true);
    let nur_partner = [zusammen.clone(), ("p36_lohnsteuer_partner", json!(70_000), true)];
    assert_eq!(rest(&nur_partner), Some(430_000));
    // Person A unbestaetigt, Ehegatte bestaetigt: A zaehlt nicht, der Ehegatte schon.
    let a_offen = [
        zusammen.clone(),
        ("p36_lohnsteuer", json!(100_000), false),
        ("p36_lohnsteuer_partner", json!(70_000), true),
    ];
    assert_eq!(rest(&a_offen), Some(430_000));
    // Nichts bestaetigt: keine Zahl.
    let nichts = [zusammen.clone(), ("p36_lohnsteuer_partner", json!(70_000), false)];
    assert_eq!(rest(&nichts), None);
    // Nur der Ehegatte bestaetigt, aber einzeln veranlagt: sein Feld zaehlt nicht, also gibt es keine Zahl.
    let einzel = [
        ("veranlagung", json!("einzel"), true),
        ("p36_lohnsteuer_partner", json!(70_000), true),
    ];
    assert_eq!(rest(&einzel), None);
}

/// Ein unbestaetigter Wert des Ehegatten zaehlt nicht (wie bei jedem Anrechnungsfeld): 5.000 - 1.000 = 4.000 EUR.
#[test]
fn ein_unbestaetigter_wert_des_ehegatten_zaehlt_nicht() {
    let r = rest(&[
        ("veranlagung", json!("zusammen"), true),
        ("p36_lohnsteuer", json!(100_000), true),
        ("p36_lohnsteuer_partner", json!(70_000), false),
    ]);
    assert_eq!(r, Some(400_000));
}

/// § 36 Abs. 3 Satz 2: Die Lohnsteuer beider Ehegatten ist EINE Abzugsteuer, ihre Summe wird einmal aufgerundet. 100,50 EUR (A)
/// und 100,50 EUR (Ehegatte) ergeben 201,00 EUR und bleiben 201,00 EUR: 1.000 - 201 = 799 EUR. Rundete man jeden Betrag einzeln
/// auf (101 + 101 = 202), kaeme 798 EUR heraus.
#[test]
fn die_lohnsteuer_beider_wird_erst_addiert_dann_einmal_aufgerundet() {
    let r = abschluss(
        &[
            ("veranlagung", json!("zusammen"), true),
            ("p36_lohnsteuer", json!(10_050), true),
            ("p36_lohnsteuer_partner", json!(10_050), true),
        ],
        100_000,
    )
    .unwrap()
    .map(Cent::get);
    assert_eq!(r, Some(79_900));
}

/// Zwei Lohnsteuern, die zusammen nicht in `i64` passen, sind ein Ueberlauf-Fehler, kein stiller Umlauf und kein Abbruch.
#[test]
fn die_summe_der_zwei_lohnsteuern_laeuft_nicht_still_ueber() {
    let r = abschluss(
        &[
            ("veranlagung", json!("zusammen"), true),
            ("p36_lohnsteuer", json!(i64::MAX), true),
            ("p36_lohnsteuer_partner", json!(i64::MAX), true),
        ],
        FESTGESETZT,
    );
    assert!(matches!(r, Err(BescheidFehler::Ueberlauf(_))), "{r:?}");
}
