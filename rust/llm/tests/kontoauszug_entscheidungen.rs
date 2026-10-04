//! Entscheidungsstellen des Kontoauszug-Klassifikators (`kontoauszug_writer.py`: `_parse_llm_kategorie`,
//! `llm_klassifikator_factory`), am Aufrufort von `llm::kontoauszug` geprueft (N4, Mutationsmessung
//! `rust/llm`, Teil `kontoauszug.rs`). Jede Erwartung stammt aus dem Python-Aufruf mit denselben Eingaben;
//! sie steht hier als Literal, kein Test ruft Python.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::cell::RefCell;

use llm::kontoauszug::{klassifiziere, parse_kategorie};
use llm::{Chat, Completion, Kategorie, LlmFehler, Nachricht, Rolle};
use serde_json::Value;

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// (Zeichenzahl, FNV-1a-64 ueber die UTF-8-Bytes): die Kennung eines langen Textes aus Python.
fn kennung(s: &str) -> (usize, u64) {
    (s.chars().count(), fnv(s))
}

/// Ein Chat, der die Nachrichten festhaelt und eine feste Antwort gibt.
struct Fang {
    gesehen: RefCell<Vec<(Rolle, String)>>,
    antwort: Result<&'static str, ()>,
}

impl Chat for Fang {
    fn complete(&self, m: &[Nachricht], schema: Option<&Value>) -> Result<Completion, LlmFehler> {
        assert!(schema.is_none(), "der Klassifikator sendet kein Schema");
        *self.gesehen.borrow_mut() = m
            .iter()
            .map(|n| (n.rolle(), n.inhalt().to_owned()))
            .collect();
        match self.antwort {
            Ok(t) => Ok(Completion {
                text: t.into(),
                ..Default::default()
            }),
            Err(()) => Err(LlmFehler::Endgueltig {
                grund: llm::Grund::Sonstig,
                detail: "aus".into(),
            }),
        }
    }
}

fn buchung(
    antwort: Result<&'static str, ()>,
    zweck: &str,
    betrag: i64,
) -> (Option<Kategorie>, Vec<(Rolle, String)>) {
    let f = Fang {
        gesehen: RefCell::new(Vec::new()),
        antwort,
    };
    let k = klassifiziere(&f, &llm::pii::maskiere(zweck), betrag);
    (k, f.gesehen.into_inner())
}

/// Wire-Wort und Kategorie gehoeren paarweise zusammen; alles andere ist keine Kategorie.
#[test]
fn kategorie_wort_und_rueckweg() {
    for (wort, kat) in [
        ("handwerker", Kategorie::Handwerker),
        ("dienstleistung", Kategorie::Dienstleistung),
        ("minijob", Kategorie::Minijob),
        ("spende", Kategorie::Spende),
        ("vorsorge", Kategorie::Vorsorge),
    ] {
        assert_eq!(Kategorie::aus_wort(wort), Some(kat), "aus_wort({wort})");
        assert_eq!(kat.als_str(), wort, "als_str({wort})");
    }
    for fremd in [
        "miete",
        "Spende",
        "",
        "spende ",
        "handwerkerx",
        "null",
        "kategorie",
    ] {
        assert_eq!(Kategorie::aus_wort(fremd), None, "aus_wort({fremd:?})");
    }
}

/// `_parse_llm_kategorie`: erstes `{` bis letztes `}`, als JSON, `kategorie` aus der MVP-Menge.
#[test]
#[rustfmt::skip]
fn parse_kategorie_wie_python() {
    let faelle: &[(&str, Option<&str>)] = &[
        ("{\"kategorie\": \"spende\"}", Some("spende")),
        ("Antwort: {\"kategorie\": \"vorsorge\"} ok", Some("vorsorge")),
        ("a {\"x\":1} {\"kategorie\":\"spende\"}", None),
        ("{\"kategorie\":\"spende\"} x }", None),
        ("x } {\"kategorie\": \"minijob\"} y", Some("minijob")),
        ("}{", None),
        ("{", None),
        ("}", None),
        ("", None),
        ("{}", None),
        ("{\"kategorie\": \"miete\"}", None),
        ("{\"kategorie\": null}", None),
        ("{\"category\": \"spende\"}", None),
        ("{\"kategorie\":\"handwerker\"}", Some("handwerker")),
        ("{\"kategorie\":\"dienstleistung\"}", Some("dienstleistung")),
        ("{\"kategorie\":\"minijob\"}", Some("minijob")),
        ("{\"kategorie\":\"vorsorge\"}", Some("vorsorge")),
        ("{\"kategorie\": \"spende\"", None),
        ("{\"kategorie\": 5}", None),
        ("{\"kategorie\": \"Spende\"}", None),
        ("{\"kategorie\": \"spende \"}", None),
        ("```json\n{\"kategorie\": \"handwerker\"}\n```", Some("handwerker")),
        ("{\"kategorie\": \"spende\"}}", None),
        ("{{\"kategorie\": \"spende\"}", None),
        ("kein json {", None),
        ("x{\"kategorie\":\"spende\"}y{", Some("spende")),
        ("{\"a\": {\"kategorie\": \"spende\"}}", None),
        ("{\"kategorie\": \"spende\", \"x\": [1, 2]}", Some("spende")),
        ("{\"kategorie\"\n: \"vorsorge\"}", Some("vorsorge")),
    ];
    for (text, erwartet) in faelle {
        assert_eq!(parse_kategorie(text).map(Kategorie::als_str), *erwartet, "parse_kategorie({text:?})");
    }
}

/// Der Klassifikator schickt Systemtext und Buchung wie Python; Betrag in Euro mit zwei Stellen.
#[test]
#[rustfmt::skip]
fn klassifiziere_sendet_system_und_buchung_wie_python() {
    let faelle: &[(&str, i64, &str)] = &[
        ("Maler DE89****", -48000, "Zweck: Maler DE89****\nBetrag: -480.00 EUR"),
        ("Spende 12****", -1, "Zweck: Spende 12****\nBetrag: -0.01 EUR"),
        ("", 0, "Zweck: \nBetrag: 0.00 EUR"),
        ("x", 5, "Zweck: x\nBetrag: 0.05 EUR"),
        ("Zweck", 123_456_789, "Zweck: Zweck\nBetrag: 1234567.89 EUR"),
        ("y", -100, "Zweck: y\nBetrag: -1.00 EUR"),
        ("z", 1050, "Zweck: z\nBetrag: 10.50 EUR"),
        ("a", 99, "Zweck: a\nBetrag: 0.99 EUR"),
        ("b", -99999, "Zweck: b\nBetrag: -999.99 EUR"),
        ("Miete", 100_000, "Zweck: Miete\nBetrag: 1000.00 EUR"),
        ("c", 1, "Zweck: c\nBetrag: 0.01 EUR"),
    ];
    for (zweck, betrag, user) in faelle {
        let (_, m) = buchung(Ok("{}"), zweck, *betrag);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].0, Rolle::System);
        assert_eq!(kennung(&m[0].1), (381, 0x53b9_2929_5b13_f3ad), "Systemtext: {}", m[0].1);
        assert_eq!(m[1].0, Rolle::User);
        assert_eq!(m[1].1, *user, "Buchung ({zweck:?}, {betrag})");
    }
}

/// Nur eine lesbare Kategorie zaehlt; unlesbar und Fehler bleiben ohne Kategorie, ein Betrag von 0 aendert nichts.
#[test]
fn klassifiziere_antwort_wird_zur_kategorie_oder_nichts() {
    let spende = r#"{"kategorie": "spende"}"#;
    assert_eq!(
        buchung(Ok(spende), "Zweck", -500).0,
        Some(Kategorie::Spende)
    );
    assert_eq!(
        buchung(Ok(spende), "Zweck", 0).0,
        Some(Kategorie::Spende),
        "Betrag 0 sperrt nicht"
    );
    assert_eq!(
        buchung(Ok("kein json"), "Zweck", -500).0,
        None,
        "Unlesbares wird keine Kategorie"
    );
    assert_eq!(buchung(Ok(r#"{"kategorie": null}"#), "Zweck", -500).0, None);
    assert_eq!(
        buchung(Ok(r#"{"kategorie": "miete"}"#), "Zweck", -500).0,
        None
    );
    assert_eq!(
        buchung(Err(()), "Zweck", -500).0,
        None,
        "ein Fehler des Chats ist keine Kategorie"
    );
}
