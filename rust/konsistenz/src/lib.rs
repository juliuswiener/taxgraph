//! `konsistenz` — Widersprüche und Hinweise über dem materialisierten Snapshot, vor der Abgabe
//! (`produkt/konsistenz/*`). Rein deterministisch, kein LLM, keine Seiteneffekte.
//!
//! - [`flag_widersprueche`]: Abwesenheits-Flag `kein_*` ↔ belegtes Betragsfeld (`flag_check.py`)
//! - [`partner_ohne_zusammen`], [`alleinerziehend_mit_zusammen`]: Partnerangaben ↔ Veranlagung
//!   (`partner_check.py`)
//! - [`pauschal_hinweise`]: vergessene Pauschalen (`check_pauschalen.py`)
//! - [`nicht_gerechnete_angaben`]: deklariert, aber nicht gerechnet (`check_nicht_gerechnet.py`)
//! - [`preflight`]: sammelt alles und bildet die [`Ampel`] (`preflight.py`)
//!
//! Wertebereich der Parität: Snapshot-Werte sind JSON ohne Floats und mit Ganzzahlen in `i64`
//! (Auflage T des Stores lässt auf `cent`/`int`-Feldern nur Ganzzahlen zu; gemessen an 192 realen
//! Fällen: 0 Floats). Wo Python einen Float als Betrag formatieren würde, zählt er in Rust nicht
//! als Betrag (`// PARITÄT:` an der Stelle).
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

mod flag;
mod lesung;
mod nicht_gerechnet;
mod partner;
mod pauschalen;
mod preflight;
mod zahl;

pub use flag::{
    flag_stand, flag_widersprueche, instanz_feld_ids, FlagStand, FlagWiderspruch, FLAG_NEGIERT,
};
pub use lesung::{lies, Felder, Lesung};
pub use nicht_gerechnet::{nicht_gerechnete_angaben, NichtGerechnet, NICHT_GERECHNET};
pub use partner::{
    alleinerziehend_mit_zusammen, partner_ohne_zusammen, PartnerWiderspruch, PARTNER_FELDER,
};
pub use pauschalen::{pauschal_hinweise, PauschalCheck, PauschalHinweis, PAUSCHAL_CHECKS};
pub use preflight::{
    kist_ueber_brutto_anteil, plausibilitaets_widersprueche, preflight, unvollstaendige_instanzen,
    vorlaeufige_ring_betraege, Ampel, PlausiWiderspruch, PreflightErgebnis,
    VorlaeufigerBetrag, BRUTTO_FLOAT_EXAKT_MAX, RING_BETRAGSFELDER, SCHULGELD_SCHWELLE_CENT,
};
pub use zahl::eur;
