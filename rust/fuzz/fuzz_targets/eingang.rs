#![no_main]
//! Uploads (CSV/PDF-Text/Beleg), VaSt-, eDaten- und Vorjahres-Import: externe Eingabe.
use std::collections::BTreeMap;

use eingang::{beleg, csv, edaten, kontoauszug, ocr, vast, vorjahr};
use libfuzzer_sys::fuzz_target;
use store::{BindungNachschlag, Store};

fuzz_target!(|data: &[u8]| {
    let Some((&n, rest)) = data.split_first() else { return };
    let Ok(text) = std::str::from_utf8(rest) else { return };
    let nachschlag = taxgraph_fuzz::nachschlag();
    let bindung = BindungNachschlag::neu(nachschlag);

    let trenner = [',', ';', '\t', '|'][usize::from(n) % 4];
    let _ = csv::lies_datensaetze(text, trenner);
    let _ = ocr::tsv_zu_zeilen(text);

    let _ = kontoauszug::parse_csv(text);
    let conf: ocr::ConfMap =
        rest.iter().enumerate().map(|(i, b)| (i, f64::from(*b) / 255.0)).collect();
    let _ = kontoauszug::parse_pdf_zeilen(text, &conf, f64::from(n) / 255.0);
    let _ = kontoauszug::parse_pdf_zeilen(text, &BTreeMap::new(), 0.6);
    let _ = kontoauszug::eur_cent_signed(text);
    let _ = kontoauszug::py_float(text);
    let _ = kontoauszug::klassifiziere_det(text);
    if let Ok(j) = serde_json::from_str::<serde_json::Value>(text) {
        let _ = kontoauszug::aus_json(&j);
    }

    let _ = beleg::erkenne_beleg_typ(text);
    let bconf: BTreeMap<String, f64> = BTreeMap::from([("p36_lohnsteuer".to_owned(), 0.5)]);
    let _ = beleg::extrahiere(text, bindung, &bconf);

    let _ = vast::cent(Some(text));
    let werte: BTreeMap<String, String> = text
        .lines()
        .filter_map(|z| z.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    let _ = vast::aus_lstb(&werte);
    if let Ok(l) = serde_json::from_str::<Vec<vast::Leistung>>(text) {
        let _ = vast::aus_lersl(&l);
    }

    // eDaten-/Vorjahres-Import: JSON -> Saetze -> Store (mit echter Bindung, Auflage T).
    if let Ok(saetze) = serde_json::from_str::<Vec<edaten::Satz>>(text) {
        let mut store = Store::leer(2025, None);
        let _ = edaten::uebernehme(&mut store, &saetze, None, Some(bindung));
    }
    if let Ok(felder) = serde_json::from_str::<BTreeMap<String, vorjahr::VorjahrFeld>>(text) {
        let _ = vorjahr::referenzwert_verlustvortrag(&felder);
        let mut store = Store::leer(2025, None);
        let _ = vorjahr::uebernehme(&mut store, &felder, bindung, 2024, None);
    }
});
