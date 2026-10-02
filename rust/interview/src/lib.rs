//! `interview` — der Regel-Graph in zwei Leserichtungen (`produkt/traverser/traverser.py`) und
//! die zwei Rollen der Bindung an der Spannen-Naht (`produkt/haut/bindung_rollen.py`).
//!
//! Rueckwaerts = Interview ([`relevanz`], [`naechste_fragen`]), vorwaerts = Beweis
//! ([`justification`], [`trace_ergebnis`]). Reine Ableitung ueber Bindung + Store; READ-ONLY,
//! kein LLM, keine Catala-Introspektion.
//!
//! Die tragenden Invarianten stehen als Typen, nicht als Konvention:
//! - vorlaeufig = unbeantwortet: [`Antwort::aus`] ist der einzige Weg an einen Wert, und er
//!   liefert fuer `vorlaeufig` [`Antwort::Offen`] — ein Vorschlag kann keine Bedingung
//!   entscheiden.
//! - Instanz-Zahl nur aus einem BESTAETIGTEN Zaehlfeld, gekappt auf `max`: [`InstanzAnzahl`].
//! - Instanz-Id `basis` / `basis__n` (n >= 2): `domain::FeldId`, hier nur benutzt.
//! - Bedingung schliesst erst aus, wenn JEDE Instanz bestaetigt abweicht: [`Bedingungsstand`].
//! - Regelstatus als Enum statt drei Strings: [`Regelstatus`].
//! - Bindung eindeutig je `feld_id`: `bindung::lade_registry` weist Duplikate ab; [`Sicht`]
//!   haelt die Aufrufer-Reihenfolge (Python-`dict`) ohne Doppel.
//! - Aufbau- und Achsen-Bindung sind verschiedene Typen: [`AufbauBindung`], [`AchsenBindung`].
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::disallowed_types
    )
)]

mod antwort;
mod beweis;
mod fragen;
mod graph;
mod instanz;
mod relevanz;
mod rollen;

pub use antwort::{Antwort, Eintrag, InstanzAnzahl};
pub use beweis::{justification, trace_ergebnis, AnkerRefSicht, Justification, Trace};
pub use fragen::naechste_fragen;
pub use graph::{Graph, Sicht, UnbekanntesFeld};
pub use instanz::{fehlende_instanzen, instanz_anzahl, instanz_feld_id, FehlendeInstanz};
pub use relevanz::{gate_gewicht, relevanz, Bedingungsstand, RegelRelevanz, Regelstatus};
pub use rollen::{relevante_kegel_felder, ring_bindung, rollen, AchsenBindung, AufbauBindung};

/// Laedt die echte Registry fuer Doctests (`produkt/bindung`). Nicht Teil der API.
///
/// ```
/// assert!(interview::doctest_registry().is_some());
/// ```
#[doc(hidden)]
#[must_use]
pub fn doctest_registry() -> Option<bindung::Registry> {
    let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
    bindung::lade_registry(&pfad).ok()
}
