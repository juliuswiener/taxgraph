//! Bindungs-Nachschlag fuer Auflage T/F und die Ableitungen: `feld_id -> &Bindung`, mit
//! Repeated-Instance-Aufloesung (`base__n` -> `base`, `produkt/mapping/est_mapping.py:543-552`,
//! `parse_instanz` — "die EINE Enumerations-Wahrheit").
use std::collections::HashMap;

use bindung::Bindung;

/// `base__<n>` (n>=2) -> `base`; eine Basis-`feld_id` ohne Suffix -> `None` (= Instanz 1), ebenso `base__1`.
/// Pendant zu `_INSTANZ_RE = re.compile(r"^(?P<base>[a-z][a-z0-9_]*)__(?P<idx>[2-9]|[1-9][0-9]+)$")`. Pythons
/// `$` passt auch vor EINEM abschliessenden `\n` (`a__2\n`): das gilt hier ebenso, denn `store.py`
/// (`_pruefe_typ_konformitaet`) liest die Basis so, und ein Wert zu `schulgeld__2\n` wird dort auf den Typ von
/// `schulgeld` geprueft. Gleich mit Python bei jeder Eingabe (`feld_kennung_paritaet`).
///
/// ```
/// use store::instanz_basis;
/// assert_eq!(instanz_basis("kind_idnr__2"), Some("kind_idnr"));
/// assert_eq!(instanz_basis("kind_idnr__2\n"), Some("kind_idnr"));
/// assert_eq!(instanz_basis("kind_idnr__2\n\n"), None);
/// assert_eq!(instanz_basis("kind_idnr__1"), None);
/// assert_eq!(instanz_basis("kind_idnr"), None);
/// assert_eq!(instanz_basis("kind_idnr__02"), None);
/// ```
#[must_use]
pub fn instanz_basis(feld_id: &str) -> Option<&str> {
    let (basis, idx) = feld_id.rsplit_once("__")?;
    let idx = idx.strip_suffix('\n').unwrap_or(idx);
    if basis.is_empty()
        || idx.is_empty()
        || !idx.bytes().all(|b| b.is_ascii_digit())
        || idx.starts_with('0')
        || idx == "1"
    {
        return None;
    }
    let mut chars = basis.chars();
    let erstes_ok = matches!(chars.next(), Some(c) if c.is_ascii_lowercase());
    let rest_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    (erstes_ok && rest_ok).then_some(basis)
}

/// `feld_id -> &Bindung` ueber alle geladenen Bindungsdateien (`registry::lade_registry`).
#[derive(Debug, Clone, Copy)]
pub struct BindungNachschlag<'a> {
    by_feld_id: &'a HashMap<String, &'a Bindung>,
}

impl<'a> BindungNachschlag<'a> {
    /// ```
    /// use std::collections::HashMap;
    /// let leer: HashMap<String, &bindung::Bindung> = HashMap::new();
    /// let nachschlag = store::BindungNachschlag::neu(&leer);
    /// assert!(nachschlag.basis_eintrag("kind_idnr").is_none());
    /// ```
    #[must_use]
    pub fn neu(by_feld_id: &'a HashMap<String, &'a Bindung>) -> Self {
        Self { by_feld_id }
    }

    /// Direkter Nachschlag ohne Instanz-Aufloesung.
    ///
    /// ```
    /// # let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
    /// # let bindungen: Vec<bindung::Bindung> = bindung::lade_registry(&pfad).unwrap()
    /// #     .dateien.into_iter().flat_map(|(_, d)| d.bindungen).collect();
    /// let map = store::baue_nachschlag(&bindungen);
    /// let nachschlag = store::BindungNachschlag::neu(&map);
    /// assert_eq!(nachschlag.get("kind_idnr").unwrap().feld_id, "kind_idnr");
    /// assert!(nachschlag.get("kind_idnr__2").is_none());
    /// ```
    #[must_use]
    pub fn get(&self, feld_id: &str) -> Option<&'a Bindung> {
        self.by_feld_id.get(feld_id).copied()
    }

    /// `feld_id` direkt, sonst dessen Instanz-Basis (`store.py:213-224`:
    /// `_pruefe_typ_konformitaet`, "unbekanntes `feld_id`: durchlassen, nicht raten" bleibt beim
    /// Aufrufer — hier nur der Nachschlag selbst).
    ///
    /// ```
    /// # let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
    /// # let bindungen: Vec<bindung::Bindung> = bindung::lade_registry(&pfad).unwrap()
    /// #     .dateien.into_iter().flat_map(|(_, d)| d.bindungen).collect();
    /// let map = store::baue_nachschlag(&bindungen);
    /// let nachschlag = store::BindungNachschlag::neu(&map);
    /// assert_eq!(nachschlag.basis_eintrag("kind_idnr__2").unwrap().feld_id, "kind_idnr");
    /// assert!(nachschlag.basis_eintrag("gibt_es_nicht__2").is_none());
    /// ```
    #[must_use]
    pub fn basis_eintrag(&self, feld_id: &str) -> Option<&'a Bindung> {
        self.get(feld_id)
            .or_else(|| instanz_basis(feld_id).and_then(|b| self.get(b)))
    }

    /// Alle Bindungen mit ihrem `feld_id` (`store.py:529`, `_rechne_ab`: `for ziel, eintrag in
    /// bindung.items()` durchsucht JEDEN Eintrag nach einer `ableitung`-Regel, nicht nur den
    /// eines einzelnen `feld_id`).
    ///
    /// ```
    /// # let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
    /// # let bindungen: Vec<bindung::Bindung> = bindung::lade_registry(&pfad).unwrap()
    /// #     .dateien.into_iter().flat_map(|(_, d)| d.bindungen).collect();
    /// let map = store::baue_nachschlag(&bindungen);
    /// let nachschlag = store::BindungNachschlag::neu(&map);
    /// assert_eq!(nachschlag.alle().count(), bindungen.len());
    /// assert!(nachschlag.alle().all(|(feld_id, b)| feld_id == b.feld_id));
    /// ```
    pub fn alle(&self) -> impl Iterator<Item = (&'a str, &'a Bindung)> + '_ {
        self.by_feld_id.iter().map(|(k, v)| (k.as_str(), *v))
    }
}

/// Baut die flache `feld_id -> &Bindung`-Map aus allen geladenen `BindungDatei`s
/// (`registry::Registry`).
///
/// ```
/// let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
/// let bindungen: Vec<bindung::Bindung> = bindung::lade_registry(&pfad)
///     .unwrap()
///     .dateien
///     .into_iter()
///     .flat_map(|(_, d)| d.bindungen)
///     .collect();
/// let map = store::baue_nachschlag(&bindungen);
/// // `lade_registry` weist eine doppelte `feld_id` ab, die Map verliert also keinen Eintrag.
/// assert_eq!(map.len(), bindungen.len());
/// assert_eq!(map["kind_idnr"].feld_id, "kind_idnr");
/// ```
#[must_use]
pub fn baue_nachschlag(bindungen: &[Bindung]) -> HashMap<String, &Bindung> {
    bindungen.iter().map(|b| (b.feld_id.clone(), b)).collect()
}

#[cfg(test)]
mod tests {
    use super::instanz_basis;

    #[test]
    fn instanz_basis_matcht_python_regex() {
        assert_eq!(instanz_basis("vv_einnahmen__2"), Some("vv_einnahmen"));
        assert_eq!(instanz_basis("vv_einnahmen__12"), Some("vv_einnahmen"));
        assert_eq!(instanz_basis("vv_einnahmen__10"), Some("vv_einnahmen"));
        assert_eq!(instanz_basis("vv_einnahmen__11"), Some("vv_einnahmen"));
        assert_eq!(instanz_basis("vv_einnahmen"), None);
        assert_eq!(instanz_basis("vv_einnahmen__1"), None); // Instanz 1 ist die Basis, nie `__1`
        assert_eq!(instanz_basis("vv_einnahmen__0"), None); // [2-9]|[1-9][0-9]+ -- keine 0, keine fuehrende 0
        assert_eq!(instanz_basis("vv_einnahmen__"), None);
        assert_eq!(instanz_basis("__2"), None); // Basis darf nicht leer sein

        // Pythons `$` passt vor EINEM abschliessenden `\n`, nicht vor zweien und nicht ohne Zaehler.
        assert_eq!(instanz_basis("vv_einnahmen__2\n"), Some("vv_einnahmen"));
        assert_eq!(instanz_basis("vv_einnahmen__10\n"), Some("vv_einnahmen"));
        assert_eq!(instanz_basis("vv_einnahmen__2\n\n"), None);
        assert_eq!(instanz_basis("vv_einnahmen__1\n"), None);
        assert_eq!(instanz_basis("vv_einnahmen__\n"), None);
    }
}
