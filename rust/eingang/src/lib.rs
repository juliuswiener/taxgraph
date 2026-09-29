//! `eingang` — Importe in den Store (`produkt/eingang/*_writer.py`, `vast_mapping.py`):
//! Kontoauszug, Beleg, Vorjahr als VORLAEUFIGE Vorschlaege ([`vorschlag::VorschlagEvent`]), eDaten
//! als einziger bestaetigter Import ([`vorschlag::EdatenEvent`]). PDF/OCR bleiben Unterprozesse
//! mit Zeitlimit und Seitendeckel ([`ocr`]).
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

pub mod beleg;
pub mod csv;
pub mod edaten;
pub mod kontoauszug;
pub mod ocr;
pub mod vast;
pub mod vorjahr;
pub mod vorschlag;
