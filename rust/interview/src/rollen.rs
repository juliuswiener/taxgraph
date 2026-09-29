//! Die Rollen der Bindung an der Spannen-/Estimate-Naht (`produkt/haut/bindung_rollen.py`).
//!
//! DREI Rollen, bis 2026-09-26 auf EINER Variable — daraus kam der Widerspruch Kopfzeile ↔
//! `/ergebnis` (240 EUR, Fall § 35a):
//! 1. ENUMERIEREN (Instanz-Gruppen finden) braucht die VOLLE Bindung: mit dem Kegel findet der
//!    § 35a-Topf keine Instanz und summiert 0.
//! 2. ACHSEN (Wertemenge der Spanne) braucht den KEGEL: mit der vollen Bindung werden 341 statt
//!    21 Felder zu Achsen, und die Kopfzeile zeigt "Noch keine Zahl".
//! 3. SLOT-UEBERSETZUNG braucht eine Obermenge der Achsen — die volle Bindung.
//!
//! Rolle 1 und 3 sind [`AufbauBindung`], Rolle 2 ist [`AchsenBindung`]: zwei Typen, damit sie
//! nicht wieder zu einer Variable verschmelzen koennen.
use store::Store;

use crate::graph::{Graph, Sicht};
use crate::relevanz::{relevanz, Regelstatus};

/// Die VOLLE Bindung (Enumerieren, Slot-Uebersetzung).
#[derive(Debug, Clone)]
pub struct AufbauBindung<'r>(pub Sicht<'r>);

/// Der KEGEL ohne abbestellte Felder (Spannen-Achsen).
#[derive(Debug, Clone)]
pub struct AchsenBindung<'r>(pub Sicht<'r>);

/// Der Pflicht-Kegel ohne die Felder, deren Regel der Nutzer selbst abbestellt hat
/// (`relevante_kegel_felder`, `bindung_rollen.py:41-66`).
///
/// Fail-closed an zwei Stellen: [`relevanz`] schliesst NUR bei bestaetigtem `false` aus, und ohne
/// `store` (Alt-Aufrufer, Teil-Ringe) bleibt der volle Kegel. Ein Kegel-Feld ohne Bindung bleibt
/// stehen (Python: `rel.get(None, {})`). Reihenfolge und Doppel wie `kegel`.
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// let s = store::Store::leer(2025, None);
/// let k = ["veranlagung", "gibt_es_nicht"];
/// assert_eq!(interview::relevante_kegel_felder(&k, g.alle(), Some(&s), &g), k);
/// ```
#[must_use]
pub fn relevante_kegel_felder<'k>(
    kegel: &[&'k str],
    sicht: &Sicht<'_>,
    store: Option<&Store>,
    graph: &Graph<'_>,
) -> Vec<&'k str> {
    let Some(store) = store else { return kegel.to_vec() };
    let rel = relevanz(store, sicht, graph);
    kegel
        .iter()
        .copied()
        .filter(|f| {
            sicht
                .get(f)
                .and_then(|b| rel.get(b.quelle.regel_id.as_str()))
                .is_none_or(|r| r.status != Regelstatus::Ausgeschlossen)
        })
        .collect()
}

/// Bindung fuer die Spannen-Rechnung: nur die (relevanten) Kegel-Felder, die in `sicht` stehen;
/// ohne oder mit leerem Kegel die ganze Sicht (`ring_bindung`, `bindung_rollen.py:69-73`).
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// let a = interview::ring_bindung(Some(&["veranlagung", "gibt_es_nicht"]), g.alle(), None, &g);
/// assert_eq!(a.0.feld_ids().collect::<Vec<_>>(), ["veranlagung"]);
/// assert_eq!(interview::ring_bindung(None, g.alle(), None, &g).0.len(), g.alle().len());
/// ```
#[must_use]
pub fn ring_bindung<'r>(
    kegel: Option<&[&str]>,
    sicht: &Sicht<'r>,
    store: Option<&Store>,
    graph: &Graph<'r>,
) -> AchsenBindung<'r> {
    match kegel.filter(|k| !k.is_empty()) {
        None => AchsenBindung(sicht.clone()),
        Some(k) => AchsenBindung(Sicht::aus(
            relevante_kegel_felder(k, sicht, store, graph).into_iter().filter_map(|f| sicht.get(f)),
        )),
    }
}

/// Die ZWEI Bindungen des Estimate-Pfads, einmal benannt: `(aufbau, achsen)`
/// (`rollen`, `bindung_rollen.py:76-82`).
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// let (aufbau, achsen) = interview::rollen(Some(&["veranlagung"]), g.alle(), None, &g);
/// assert_eq!(aufbau.0.len(), g.alle().len());
/// assert_eq!(achsen.0.len(), 1);
/// ```
#[must_use]
pub fn rollen<'r>(
    kegel: Option<&[&str]>,
    sicht: &Sicht<'r>,
    store: Option<&Store>,
    graph: &Graph<'r>,
) -> (AufbauBindung<'r>, AchsenBindung<'r>) {
    (AufbauBindung(sicht.clone()), ring_bindung(kegel, sicht, store, graph))
}
