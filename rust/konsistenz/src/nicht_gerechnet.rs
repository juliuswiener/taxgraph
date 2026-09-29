//! Angaben, die ans Finanzamt gehen, die der Rechenkern aber nicht kennt
//! (`produkt/konsistenz/check_nicht_gerechnet.py`). Die Vorschau liegt dann zu hoch — die
//! ungefährliche Richtung —, aber der Nutzer soll nicht raten müssen, warum.
use crate::lesung::{lies, Felder, Lesung};

/// `feld_id -> Hinweis`. Leer seit 2026-09-26: der einzige Eintrag (`spenden_vermoegensstock`,
/// § 10b Abs. 1a) ist stillgelegt und steht nicht mehr in der Erklärung
/// (`check_nicht_gerechnet.py:17-24`). Wer hier einträgt, hat ein Feld gebunden, das der Ring
/// nicht liest — eine Aussage über eine Lücke, nicht über den Nutzer.
pub const NICHT_GERECHNET: &[(&str, &str)] = &[];

/// Ein Treffer aus [`NICHT_GERECHNET`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NichtGerechnet {
    pub feld_id: &'static str,
    pub hinweis: &'static str,
}

/// Einträge aus [`NICHT_GERECHNET`], die bestätigt und eine Ganzzahl > 0 sind
/// (`check_nicht_gerechnet.py:30-44`). „0" ist der Normalfall, ein Entwurf keine Angabe.
///
/// ```
/// use konsistenz::{nicht_gerechnete_angaben, Felder};
/// assert!(nicht_gerechnete_angaben(&Felder::new()).is_empty());
/// ```
#[must_use]
pub fn nicht_gerechnete_angaben(felder: &Felder) -> Vec<NichtGerechnet> {
    NICHT_GERECHNET
        .iter()
        .filter(|(fid, _)| {
            // `isinstance(wert, int) and not isinstance(wert, bool) and wert > 0`; ein
            // bestätigtes `null` ist kein int.
            matches!(lies(felder, fid), Lesung::Bestaetigt(w) if w.as_i64().is_some_and(|n| n > 0))
        })
        .map(|&(feld_id, hinweis)| NichtGerechnet { feld_id, hinweis })
        .collect()
}
