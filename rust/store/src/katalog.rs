//! Feld-Katalog fuer Auflage K1 (`store.py:120-162`, `lade_katalog`): welches `feld_id` welcher
//! Vorschlags-Typ (`llm`/`beleg`/`kontoauszug`/`maps`) vorschlagen darf. DEFAULT human-only,
//! fail-closed — ein Feld ohne Eintrag ist fuer keinen Vorschlags-Typ freigegeben.
use std::collections::HashSet;

use bindung::{Bindung, VorschlagsSchreiber};

/// `store.py:128`: `LLM_NICHT_VORSCHLAGBAR: frozenset[str] = frozenset()` — heute leer
/// (deliberate). PARITAET: sollte Python diese Liste je fuellen, muss hier eine gleichwertige
/// Ausschlussmenge ergaenzt werden; aktuell gibt es nichts auszuschliessen.
const LLM_NICHT_VORSCHLAGBAR: &[&str] = &[];

/// `feld_id -> welche Vorschlags-Typen duerfen es setzen` (`store.py:120-162`).
#[derive(Debug, Clone, Default)]
pub struct Katalog {
    llm: HashSet<String>,
    beleg: HashSet<String>,
    kontoauszug: HashSet<String>,
    maps: HashSet<String>,
}

impl Katalog {
    /// Baut den Katalog aus allen Bindungen einer Registry (`store.py:139-162`: je Feld
    /// `vorschlagbar_von` fuer beleg/kontoauszug/maps direkt uebernehmen, `llm` zusaetzlich aus
    /// JEDEM `askable`-Feld ausser `LLM_NICHT_VORSCHLAGBAR`).
    ///
    /// ```
    /// # let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    /// # let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad).unwrap()
    /// #     .dateien.into_iter().flat_map(|(_, d)| d.bindungen).collect();
    /// let katalog = store::Katalog::aus_bindungen(&bindungen);
    /// // kap_kapitalertraege: `vorschlagbar_von: [beleg]`, `askable: true`
    /// assert!(katalog.erlaubt("beleg", "kap_kapitalertraege"));
    /// assert!(katalog.erlaubt("llm", "kap_kapitalertraege"));
    /// assert!(!katalog.erlaubt("kontoauszug", "kap_kapitalertraege"));
    /// ```
    #[must_use]
    pub fn aus_bindungen<'a>(bindungen: impl IntoIterator<Item = &'a Bindung>) -> Self {
        let mut k = Self::default();
        for b in bindungen {
            for vs in &b.vorschlagbar_von {
                let ziel = match vs {
                    VorschlagsSchreiber::Kontoauszug => &mut k.kontoauszug,
                    VorschlagsSchreiber::Beleg => &mut k.beleg,
                    VorschlagsSchreiber::Maps => &mut k.maps,
                    VorschlagsSchreiber::Llm => &mut k.llm,
                };
                ziel.insert(b.feld_id.clone());
            }
            if b.askable && !LLM_NICHT_VORSCHLAGBAR.contains(&b.feld_id.as_str()) {
                k.llm.insert(b.feld_id.clone());
            }
        }
        k
    }

    /// Darf `schreiber_typ` (`"llm"`/`"beleg"`/`"kontoauszug"`/`"maps"`) `feld_id` vorschlagen?
    /// Ein unbekannter Typ ist fail-closed nie erlaubt (`store.py:336`:
    /// `katalog.get(typ, frozenset())`).
    ///
    /// ```
    /// assert!(!store::Katalog::default().erlaubt("beleg", "kap_kapitalertraege"));
    /// # let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    /// # let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad).unwrap()
    /// #     .dateien.into_iter().flat_map(|(_, d)| d.bindungen).collect();
    /// let katalog = store::Katalog::aus_bindungen(&bindungen);
    /// assert!(katalog.erlaubt("beleg", "kap_kapitalertraege"));
    /// assert!(!katalog.erlaubt("Beleg", "kap_kapitalertraege")); // unbekannter Typ
    /// ```
    #[must_use]
    pub fn erlaubt(&self, schreiber_typ: &str, feld_id: &str) -> bool {
        match schreiber_typ {
            "llm" => self.llm.contains(feld_id),
            "beleg" => self.beleg.contains(feld_id),
            "kontoauszug" => self.kontoauszug.contains(feld_id),
            "maps" => self.maps.contains(feld_id),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Katalog;

    #[test]
    fn unbekannter_typ_ist_nie_erlaubt() {
        let k = Katalog::default();
        assert!(!k.erlaubt("does-not-exist", "irgendein_feld"));
    }
}
