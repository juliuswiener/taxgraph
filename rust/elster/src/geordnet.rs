//! Map mit Einfuege-Reihenfolge — Pythons `dict` fuer die Stellen, an denen die Reihenfolge
//! beobachtbar ist (Gruppen und Instanz-Indizes in `deklariere`, Kz-Reihenfolge im XSD-Walk).

use std::collections::BTreeMap;

/// Schluessel in Einfuege-Reihenfolge, Werte per `BTreeMap` nachschlagbar.
#[derive(Debug, Clone)]
pub(crate) struct Geordnet<K: Ord + Clone, V> {
    reihenfolge: Vec<K>,
    werte: BTreeMap<K, V>,
}

impl<K: Ord + Clone, V> Default for Geordnet<K, V> {
    fn default() -> Self {
        Self {
            reihenfolge: Vec::new(),
            werte: BTreeMap::new(),
        }
    }
}

impl<K: Ord + Clone, V> Geordnet<K, V> {
    /// Python `d.setdefault(k, neu())`.
    pub(crate) fn eintrag(&mut self, k: K, neu: impl FnOnce() -> V) -> &mut V {
        if !self.werte.contains_key(&k) {
            self.reihenfolge.push(k.clone());
        }
        self.werte.entry(k).or_insert_with(neu)
    }

    pub(crate) fn get(&self, k: &K) -> Option<&V> {
        self.werte.get(k)
    }

    pub(crate) fn schluessel(&self) -> &[K] {
        &self.reihenfolge
    }

    /// Paare in Einfuege-Reihenfolge.
    pub(crate) fn in_reihenfolge(self) -> Vec<(K, V)> {
        let mut werte = self.werte;
        self.reihenfolge
            .into_iter()
            .filter_map(|k| werte.remove(&k).map(|v| (k, v)))
            .collect()
    }
}
