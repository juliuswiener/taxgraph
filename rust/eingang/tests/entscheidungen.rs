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
use eingang::vorjahr::{uebernehme, uebernehme_in_reihenfolge, uebertragbare_felder, VorjahrFeld};
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

/// Python ruft den LLM-Rueckfall als `llm_klassifikator(maskiere(zweck), betrag)` mit dem Betrag der
/// Buchung, also negativ bei einer Ausgabe (`uebernehme_kontoauszug`, `kontoauszug_writer.py`). Die
/// Orakel-Faelle lesen die Argumente nicht; hier nimmt der Klassifikator sie auf. Positive Buchungen
/// und Treffer der Schluesselwoerter erreichen ihn nicht.
#[test]
fn der_llm_rueckfall_bekommt_den_zweck_und_den_betrag_mit_vorzeichen() {
    use std::cell::RefCell;

    use eingang::kontoauszug::{uebernehme as uebernehme_konto, Transaktion};
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let tx = |zweck: &str, betrag: i64| Transaktion {
        datum: json!("01.03.2025"),
        betrag,
        verwendungszweck: zweck.to_owned(),
    };
    let gesehen = RefCell::new(Vec::new());
    let klassifikator = |z: &llm::pii::Maskiert, b: i64| {
        gesehen.borrow_mut().push((z.as_str().to_owned(), b));
        None
    };
    let mut store = Store::leer(2025, None);
    uebernehme_konto(
        &mut store,
        &[
            tx("Posten Quelle unbekannt", -4800),
            tx("Gutschrift Posten", 100),
            tx("Maler Huber", -7),
            tx("Zweiter Posten ohne Stichwort", -1),
        ],
        nachschlag,
        Some(&klassifikator),
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        *gesehen.borrow(),
        [
            ("Posten Quelle unbekannt".to_owned(), -4800),
            ("Zweiter Posten ohne Stichwort".to_owned(), -1)
        ]
    );
}

/// `beleg_felder` ordnet ein Cent-Feld dem Beleg-Typ nach dem ANFANG seines `herkunft_slots` zu
/// (`hs.lower().startswith(hs_prefix)`); der Praefix des Minijobs ist `minijob-bescheinigung`, nicht
/// `minijob`. Die ausgelieferte Bindung hat nur den Slot `Minijob-Bescheinigung: Aufwendungen`, darum
/// bekommt das Feld hier einen Zwilling, dessen Slot nur mit `Minijob` beginnt.
#[test]
fn ein_slot_der_nur_mit_minijob_beginnt_gehoert_nicht_zum_minijob_beleg() {
    use eingang::beleg::{beleg_felder, BelegTyp};
    let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut alle: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&wurzel)
        .unwrap()
        .dateien
        .into_iter()
        .flat_map(|(_, d)| d.bindungen)
        .collect();
    let mut zwilling = alle
        .iter()
        .find(|b| b.feld_id == "hh_minijob_betrag")
        .unwrap()
        .clone();
    zwilling.feld_id = "zz_minijob_ohne_bescheinigung".into();
    zwilling.herkunft_slots = Some(vec!["Minijob ohne Bescheinigung: Betrag".into()]);
    alle.push(zwilling);
    let map = store::baue_nachschlag(&alle);
    let felder = beleg_felder(BindungNachschlag::neu(&map), BelegTyp::Minijob);
    assert!(
        felder.contains_key("hh_minijob_betrag"),
        "Gegenprobe: das echte Feld gehoert dazu"
    );
    assert!(
        !felder.contains_key("zz_minijob_ohne_bescheinigung"),
        "{felder:?}"
    );
}

/// `vorjahr::uebernehme` ohne Reihenfolge haengt die Events nach `feld_id` sortiert an (Doku an der
/// Funktion); die API nimmt `uebernehme_in_reihenfolge` und braucht diese Reihenfolge nicht. Die
/// Orakel-Faelle sortieren die Events vor dem Vergleich, sehen die Reihenfolge also nicht.
#[test]
fn die_vorjahr_uebernahme_ohne_reihenfolge_haengt_nach_feld_id_sortiert_an() {
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let feld = || VorjahrFeld {
        wert: json!(1),
        zustand: Some("bestaetigt".into()),
    };
    // Felder, die den Wert 1 annehmen (ein Text- oder Bool-Feld wiese ihn ab), in Schluesselreihenfolge.
    let felder: Vec<String> = uebertragbare_felder(nachschlag)
        .into_keys()
        .filter(|f| {
            let mut store = Store::leer(2025, None);
            let eins = BTreeMap::from([(f.clone(), feld())]);
            uebernehme_in_reihenfolge(
                &mut store,
                &eins,
                nachschlag,
                std::slice::from_ref(f),
                2024,
                None,
            )
            .is_ok_and(|e| e.uebertragen == 1)
        })
        .take(3)
        .collect();
    assert!(felder.len() >= 2, "{felder:?}");
    let vorjahr: BTreeMap<String, VorjahrFeld> =
        felder.iter().map(|f| (f.clone(), feld())).collect();
    let mut store = Store::leer(2025, None);
    uebernehme(&mut store, &vorjahr, nachschlag, 2024, None).unwrap();
    let angehaengt: Vec<String> = serde_json::to_value(store.events())
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["feld_id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(angehaengt, felder);
}
