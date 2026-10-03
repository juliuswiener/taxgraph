//! `llm` — LLM-Anbindung (`produkt/haut/llm_client.py`, `api_llm.py`, `pii_filter.py`):
//! PII-Filter, Chat-Completions-Client mit Wiederholung und Frist, der Drei-Stufen-Chat mit
//! strikten Parsern und deterministischen Gates, der Kontoauszug-Klassifikator. Dazu [`ors`], der
//! Transport zu `OpenRouteService` (`produkt/haut/ors_client.py`) auf demselben HTTP-Unterbau.
//!
//! Tragende Typen: [`pii::Gefiltert`]/[`pii::Maskiert`] (nur sie erreichen [`Nachricht::nutzer`]),
//! [`Antwort`] (kaputtes JSON ist eine benannte Variante, kein stilles Leer),
//! [`gates::BelegterVorschlag`] (nur das Beleg-Gate baut ihn), [`LlmFehler`] (drei Klassen, der
//! Schluessel steht nie im `Display`).
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

mod client;
pub mod dialog;
pub mod gates;
mod http;
pub mod kontoauszug;
pub mod ors;
pub mod parse;
pub mod pii;
pub mod prompt;
pub mod py;
pub mod schema;
mod texte;

pub use client::{
    Chat, Completion, Grund, HttpChat, Konfiguration, LlmFehler, Nachricht, Rolle, Schluessel,
    MAX_TOKENS,
};
pub use kontoauszug::Kategorie;
pub use parse::Antwort;
