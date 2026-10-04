//! Entscheidungsstellen des Chat-Clients ohne Netz (`llm_client.py`: Zeitgrenzen, `_MAX_TOKENS`, `GRUND_*`,
//! `_aufgegeben`), am Aufrufort von `llm::client` geprueft (N4, Mutationsmessung `rust/llm`, Teil `client.rs`).
//! Jede Erwartung stammt aus dem Python-Modul (`LC._FRIST_S` usw.) bzw. aus `str(e)` der Python-Ausnahme mit
//! denselben Eingaben; sie steht hier als Literal, kein Test ruft Python.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::time::Duration;

use llm::{Grund, Konfiguration, LlmFehler, MAX_TOKENS};

/// `_FRIST_S` 150, `_FRIST_WIEDERHOLUNG_S` 90, `_TIMEOUT` 30, `_BACKOFF_S` (1, 2), `_MAX_TOKENS` 8192.
#[test]
fn zeitgrenzen_und_token_grenze_wie_python() {
    let k = Konfiguration::neu("http://x".into(), "m".into(), "k".into());
    assert_eq!(k.frist, Duration::from_secs(150));
    assert_eq!(k.frist_wiederholung, Duration::from_secs(90));
    assert_eq!(k.socket, Duration::from_secs(30));
    assert_eq!(k.backoff, [Duration::from_secs(1), Duration::from_secs(2)]);
    assert_eq!(MAX_TOKENS, 8192);
}

/// `GRUND_LEER`, `GRUND_ABGESCHNITTEN`, `GRUND_FRIST`; ohne Grund ist das Wort leer (`getattr(e, "grund", "")`).
#[test]
fn grund_woerter_wie_python() {
    assert_eq!(Grund::Leer.als_str(), "leere_antwort");
    assert_eq!(Grund::Abgeschnitten.als_str(), "abgeschnitten");
    assert_eq!(Grund::Frist.als_str(), "frist_ueberschritten");
    assert_eq!(Grund::Sonstig.als_str(), "");
}

/// Text, Versuchszahl und Grund jeder Fehlerklasse wie `str(e)`, `e.versuche`, `e.grund` in Python.
#[test]
fn fehlertexte_versuche_und_grund_wie_python() {
    let faelle: Vec<(LlmFehler, &str, u32, &str)> = vec![
        (
            LlmFehler::Abgeschnitten { versuche: 2 },
            "LLM-Aufruf nach 2 Versuch(en) fehlgeschlagen: LLM-Antwort bei 8192 Tokens abgeschnitten — unvollständiges JSON.",
            2,
            "abgeschnitten",
        ),
        (
            LlmFehler::Voruebergehend { versuche: 3, grund: Grund::Sonstig, detail: "HTTP 503 {}".into() },
            "LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 503 {}",
            3,
            "",
        ),
        (
            LlmFehler::Voruebergehend { versuche: 3, grund: Grund::Leer, detail: "leerer Inhalt vom Anbieter".into() },
            "LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: leerer Inhalt vom Anbieter",
            3,
            "leere_antwort",
        ),
        (
            LlmFehler::Endgueltig { grund: Grund::Sonstig, detail: "LLM-Aufruf fehlgeschlagen: HTTP 400 x".into() },
            "LLM-Aufruf fehlgeschlagen: HTTP 400 x",
            1,
            "",
        ),
        (
            LlmFehler::Endgueltig { grund: Grund::Frist, detail: "LLM-Antwort überschritt die Frist".into() },
            "LLM-Antwort überschritt die Frist",
            1,
            "frist_ueberschritten",
        ),
    ];
    for (fehler, text, versuche, grund) in faelle {
        assert_eq!(fehler.to_string(), text);
        assert_eq!(fehler.versuche(), versuche, "{text}");
        assert_eq!(fehler.grund(), grund, "{text}");
    }
}

/// Der Schluessel steht in keinem `Debug` (Rust-eigene Zusage des Typs `Schluessel`).
#[test]
fn schluessel_steht_nie_im_debug() {
    let k = Konfiguration::neu("http://x".into(), "m".into(), "sk-geheim-123".into());
    let s = format!("{k:?}");
    assert!(!s.contains("sk-geheim-123"), "{s}");
    assert!(s.contains("Schluessel(<KEY>)"), "{s}");
}
