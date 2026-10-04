//! Entscheidungen in `eingang`, die das Python-Orakel nicht erreicht, weil Python sie nicht anbietet
//! (kein Orakelwert): hier steht jeweils der Fall und was die Crate dort tut.
//!
//! * `uebernehme_in_reihenfolge` nimmt die Feldreihenfolge als Parameter (die API reicht die Reihenfolge
//!   der Scheibe durch); Pythons `uebernehme_vorjahr` laeuft ueber die Felder der Bindung und kennt kein
//!   Feld doppelt. Nur die fuenf Wertpruefungen (Typ, Vorzeichen, Bereich, Format, Zeichensatz)
//!   ueberspringen das Feld; jede andere Abweisung des Stores bricht die Uebernahme ab
//!   (`vorjahr.rs`, `ist_pruef_abweisung`).
//! * `parse_pdf_zeilen` liest die Saldo-Erkennung (`SALDO`, mit Wortgrenzen) mit `fancy-regex`; dessen
//!   Backtracking-Grenze (1 Mio. Schritte) bricht bei einer Zeile ab etwa 0,5 Mio. Zeichen ab, Pythons
//!   `re` nicht. Die Crate verwirft eine solche Zeile und zaehlt sie (fail-closed, Doku an der
//!   Funktion); Python machte aus ihr eine Buchung mit 0,5 MB Verwendungszweck. Abweichung gewollt.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use eingang::kontoauszug::parse_pdf_zeilen;
use eingang::vorjahr::{uebernehme_in_reihenfolge, uebertragbare_felder, VorjahrFeld};
use eingang::vorschlag::SchreibFehler;
use serde_json::json;
use store::{Abweisung, BindungNachschlag, Store};

#[test]
fn jede_abweisung_ausser_den_fuenf_wertpruefungen_bricht_die_vorjahr_uebernahme_ab() {
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let felder = uebertragbare_felder(nachschlag);
    let vorjahr = |fid: &str| {
        BTreeMap::from([(
            fid.to_owned(),
            VorjahrFeld {
                wert: json!(1),
                zustand: Some("bestaetigt".into()),
            },
        )])
    };
    // Ein Feld, das den Wert 1 annimmt (ein Text- oder Bool-Feld wiese ihn als TypInkonform ab).
    let fid = felder
        .keys()
        .find(|f| {
            let mut store = Store::leer(2025, None);
            uebernehme_in_reihenfolge(
                &mut store,
                &vorjahr(f),
                nachschlag,
                &[(*f).clone()],
                2024,
                None,
            )
            .is_ok_and(|e| e.uebertragen == 1)
        })
        .expect("ein uebertragbares Feld nimmt den Wert 1 an")
        .clone();

    // Einmal in der Reihenfolge: uebertragen.
    let mut store = Store::leer(2025, None);
    let einmal = uebernehme_in_reihenfolge(
        &mut store,
        &vorjahr(&fid),
        nachschlag,
        std::slice::from_ref(&fid),
        2024,
        None,
    )
    .unwrap();
    assert_eq!((einmal.uebertragen, einmal.uebersprungen.len()), (1, 0));

    // Zweimal: der zweite Schreibversuch trifft das aktive Event des ersten (Auflage B). Das ist keine
    // Wertpruefung, also kein Ueberspringen, sondern ein Fehler; das erste Event bleibt im Store.
    let mut store = Store::leer(2025, None);
    let zweimal = uebernehme_in_reihenfolge(
        &mut store,
        &vorjahr(&fid),
        nachschlag,
        &[fid.clone(), fid.clone()],
        2024,
        None,
    );
    assert!(
        matches!(
            zweimal,
            Err(SchreibFehler::Abweisung(
                Abweisung::AktivesEventVorhanden { .. }
            ))
        ),
        "erwartet AktivesEventVorhanden, war {zweimal:?}"
    );
    assert_eq!(store.aktive().count(), 1);
}

/// Pythons `int(...)` meldet keinen Platz; die Crate nennt ihn: der Index zaehlt ab 0 und ist der Platz
/// in der Liste, nicht die Zahl der lesbaren Buchungen davor (`api` gibt den Text als 500 weiter).
#[test]
fn ein_unlesbarer_betrag_im_json_nennt_den_platz_der_buchung() {
    use eingang::kontoauszug::{aus_json, KontoauszugFehler};
    let fehler = aus_json(&json!([{"betrag": 1}, {"betrag": "x"}, {"betrag": "y"}])).unwrap_err();
    assert!(
        matches!(fehler, KontoauszugFehler::BetragUngueltig { index: 1 }),
        "{fehler:?}"
    );
    assert_eq!(
        fehler.to_string(),
        "Transaktion 1: betrag nicht ganzzahlig lesbar"
    );
    let fehler = aus_json(&json!([{"betrag": null}])).unwrap_err();
    assert!(
        matches!(fehler, KontoauszugFehler::BetragUngueltig { index: 0 }),
        "{fehler:?}"
    );
}

#[test]
fn eine_zeile_ueber_der_backtracking_grenze_wird_verworfen_und_gezaehlt() {
    let kurz = "01.03.2025 Maler -480,00 EUR";
    let (tx, verworfen) = parse_pdf_zeilen(kurz, &BTreeMap::default(), 0.6);
    assert_eq!(
        (tx.len(), verworfen),
        (1, 0),
        "Gegenprobe: die kurze Zeile ist eine Buchung"
    );

    // 600 000 Zeichen ohne Leerraum: `SALDO.sucht` liefert `None` (gemessen: ab 500 000 Zeichen).
    let lang = format!("01.03.2025 {} -480,00 EUR", "x".repeat(600_000));
    let (tx, verworfen) = parse_pdf_zeilen(&format!("{lang}\n{kurz}"), &BTreeMap::default(), 0.6);
    assert_eq!(verworfen, 1, "die ueberlange Zeile zaehlt als verworfen");
    assert_eq!(tx.len(), 1, "die Zeile danach wird gelesen");
    assert_eq!(tx.first().unwrap().verwendungszweck, "Maler");
}
