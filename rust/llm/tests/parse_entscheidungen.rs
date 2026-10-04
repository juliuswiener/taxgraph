//! Entscheidungsstellen der Antwort-Parser (`api_llm.py`: `_chat_parse`, `_rueckfragen_parse`,
//! `_antwort_parse`), am Aufrufort von `llm::parse` geprueft (N4, Mutationsmessung `rust/llm`,
//! Teil `parse.rs`). Erwartungen aus dem Python-Aufruf mit denselben Eingaben.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::HashSet;

use llm::parse::{antwort_parse, aussagen_parse, chat_parse, rueckfragen_parse, zuordnung_parse};
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
    let erlaubt: HashSet<String> = HashSet::new();
    assert!(matches!(
        zuordnung_parse("kein json", &erlaubt, 2),
        Antwort::Unlesbar
    ));
}

/// Ein JSON, das kein Objekt ist (Liste, Zahl), enthaelt keine `rueckfragen`/`aussagen`/
/// `zuordnungen`, auch wenn ein Listenelement diesen Schluessel traegt: leer, in jedem Parser.
/// Belegt PA27-PA29 als gleichwertig (`Value::get(&str)` auf Nicht-Objekten ist `None`).
#[test]
fn json_ohne_objekt_hat_keine_schluessel() {
    let (gefiltert, _) = llm::pii::filtere("x");
    let erlaubt: HashSet<String> = ["r1".to_owned()].into_iter().collect();
    for text in [
        r#"[{"rueckfragen": [{"frage": "?"}]}]"#,
        r#"[{"aussagen": [{"text": "x"}]}]"#,
        r#"[{"zuordnungen": [{"aussage": 0, "regeln": ["r1"]}]}]"#,
        "5",
    ] {
        assert!(rueckfragen_parse(text, 3).oder_leer_wie_python().is_empty());
        assert!(aussagen_parse(text, &gefiltert)
            .oder_leer_wie_python()
            .is_empty());
        assert!(zuordnung_parse(text, &erlaubt, 2)
            .oder_leer_wie_python()
            .getroffen
            .is_empty());
    }
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

/// Python `str(r.get("frage", "")).strip()[:300]`: die Frage wird erst gestrippt, dann bei 300
/// Zeichen gekappt; genau 300 bleiben.
#[test]
fn rueckfragen_parse_kappt_die_frage_bei_300() {
    for (laenge, soll) in [(299, 299), (300, 300), (301, 300), (400, 300)] {
        let text =
            json!({"rueckfragen": [{"frage": format!(" {} ", "a".repeat(laenge)), "feld_id": ""}]})
                .to_string();
        let r = rueckfragen_parse(&text, 3).oder_leer_wie_python();
        assert_eq!(r[0].frage.chars().count(), soll, "laenge {laenge}");
    }
}

/// Python `str(v).strip()` fuer `feld_id` der Rueckfrage: Leerraum faellt weg.
#[test]
fn rueckfragen_parse_strippt_die_feld_id() {
    let text = json!({"rueckfragen": [{"frage": "?", "feld_id": "  km \t"}]}).to_string();
    let r = rueckfragen_parse(&text, 3).oder_leer_wie_python();
    assert_eq!(r[0].feld_id, "km");
}

/// Python `_antwort_parse`: die Antwort wird gestrippt und bei 2000 Zeichen gekappt, genau 2000
/// bleiben. Ein JSON, das kein Objekt ist, ergibt `("", False)` und gilt als tolerant gelesen,
/// nicht als schemagerecht.
#[test]
fn antwort_parse_kappt_bei_2000_und_liest_nichtobjekte_tolerant() {
    for (laenge, soll) in [(1999, 1999), (2000, 2000), (2001, 2000), (2500, 2000)] {
        let text =
            json!({"antwort": format!(" {} ", "a".repeat(laenge)), "unsicher": false}).to_string();
        let (antwort, unsicher) = antwort_parse(&text).oder_leer_wie_python();
        assert_eq!(antwort.chars().count(), soll, "laenge {laenge}");
        assert!(!unsicher);
    }
    assert!(matches!(
        antwort_parse("[1]"),
        Antwort::Tolerant((ref a, false)) if a.is_empty()
    ));
}

/// Python `_aussagen_parse`: der Satz wird gestrippt und bei 300 Zeichen gekappt, ebenso der
/// Beleg, der danach noch im gefilterten Freitext stehen muss.
#[test]
fn aussagen_parse_kappt_satz_und_beleg_bei_300() {
    let (gefiltert, _) = llm::pii::filtere(&"a".repeat(300));
    for (laenge, soll) in [(299, 299), (300, 300), (301, 300), (400, 300)] {
        let text = json!({"aussagen": [{
            "text": format!(" {} ", "a".repeat(laenge)),
            "beleg": "a".repeat(laenge),
        }]})
        .to_string();
        let a = aussagen_parse(&text, &gefiltert).oder_leer_wie_python();
        assert_eq!(a[0].text.chars().count(), soll, "satz {laenge}");
        assert_eq!(a[0].beleg.chars().count(), soll, "beleg {laenge}");
    }
}
