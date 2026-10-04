//! Entscheidungsstellen der Antwort-Parser (`api_llm.py`: `_chat_parse`, `_rueckfragen_parse`,
//! `_antwort_parse`), am Aufrufort von `llm::parse` geprueft (N4, Mutationsmessung `rust/llm`,
//! Teil `parse.rs`). Erwartungen aus dem Python-Aufruf mit denselben Eingaben.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use llm::parse::{antwort_parse, aussagen_parse, chat_parse, rueckfragen_parse};
use llm::Antwort;
use serde_json::json;

/// Kaputtes JSON ist `Unlesbar`, nicht eine leere, tolerant gelesene Antwort: der Aufrufer
/// unterscheidet „das Modell hat nichts vorgeschlagen" von „die Antwort war Muell". Python gibt in
/// beiden Faellen `[]`; die Unterscheidung ist Rust-eigen.
#[test]
fn kaputtes_json_ist_unlesbar() {
    let (gefiltert, _) = llm::pii::filtere("x");
    assert!(matches!(chat_parse("kein json"), Antwort::Unlesbar));
    assert!(matches!(
        rueckfragen_parse("kein json", 3),
        Antwort::Unlesbar
    ));
    assert!(matches!(antwort_parse("kein json"), Antwort::Unlesbar));
    assert!(matches!(
        aussagen_parse("kein json", &gefiltert),
        Antwort::Unlesbar
    ));
}

/// Python `for k in ("vorschlaege", "vorschläge", "suggestions", "felder")`: der erste Schluessel
/// mit einer Liste gilt, die Reihenfolge ist die des Tupels, nicht die der Eingabe.
#[test]
fn wrapper_schluessel_gelten_in_fester_reihenfolge() {
    for (text, soll) in [
        (
            r#"{"felder": [{"feld_id": "b", "wert": 2}], "vorschlaege": [{"feld_id": "a", "wert": 1}]}"#,
            "a",
        ),
        (
            r#"{"felder": [{"feld_id": "b", "wert": 2}], "suggestions": [{"feld_id": "c", "wert": 3}]}"#,
            "c",
        ),
        (
            r#"{"suggestions": [{"feld_id": "c", "wert": 3}], "vorschläge": [{"feld_id": "d", "wert": 4}]}"#,
            "d",
        ),
    ] {
        let v = chat_parse(text).oder_leer_wie_python();
        assert_eq!(v.len(), 1, "{text}");
        assert_eq!(v[0].feld_id, soll, "{text}");
    }
}

/// Python `str(v.get("beleg", ""))[:300]`, `str(v["feld_id"])`: der Beleg bleibt bei 300 Zeichen
/// stehen, laenger wird gekappt, genau 300 bleiben; die `feld_id` wird nicht getrimmt.
#[test]
fn chat_parse_kappt_den_beleg_bei_300_und_trimmt_die_feld_id_nicht() {
    for (laenge, soll) in [(299, 299), (300, 300), (301, 300), (400, 300)] {
        let text = json!([{"feld_id": " km ", "wert": 1, "beleg": "a".repeat(laenge)}]).to_string();
        let v = chat_parse(&text).oder_leer_wie_python();
        assert_eq!(v[0].beleg.chars().count(), soll, "laenge {laenge}");
        assert_eq!(v[0].feld_id, " km ");
    }
}
