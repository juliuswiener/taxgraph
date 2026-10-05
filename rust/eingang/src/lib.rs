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

/// Laedt die echte Registry als Nachschlag fuer Doctests (`rust/bindung/daten`). Nicht Teil der API.
///
/// ```
/// assert!(eingang::doctest_bindung().is_some());
/// ```
#[doc(hidden)]
#[must_use]
pub fn doctest_bindung(
) -> Option<&'static std::collections::HashMap<String, &'static bindung::Bindung>> {
    static N: std::sync::OnceLock<
        Option<std::collections::HashMap<String, &'static bindung::Bindung>>,
    > = std::sync::OnceLock::new();
    N.get_or_init(|| {
        let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let reg = bindung::lade_registry_der_wurzel(&pfad).ok()?;
        let alle: Vec<bindung::Bindung> = reg
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let alle: &'static [bindung::Bindung] = Box::leak(alle.into_boxed_slice());
        Some(store::baue_nachschlag(alle))
    })
    .as_ref()
}
