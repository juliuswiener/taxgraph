//! Bindungs-Nachschlag fuer Auflage T/F und die Ableitungen: `feld_id -> &Bindung`, mit
//! Repeated-Instance-Aufloesung (`base__n` -> `base`, `produkt/mapping/est_mapping.py:543-552`,
//! `parse_instanz` — "die EINE Enumerations-Wahrheit").
use std::collections::HashMap;

use bindung::Bindung;

/// `base__<n>` (n>=1) -> `base`; eine Basis-`feld_id` ohne Suffix -> `None` (= Instanz 1).
/// Byte-identisches Pendant zu `_INSTANZ_RE = re.compile(r"^(?P<base>[a-z][a-z0-9_]*)__(?P<idx>[1-9][0-9]*)$")`.
#[must_use]
pub fn instanz_basis(feld_id: &str) -> Option<&str> {
    let (basis, idx) = feld_id.rsplit_once("__")?;
    if basis.is_empty()
        || idx.is_empty()
        || !idx.bytes().all(|b| b.is_ascii_digit())
        || idx.starts_with('0')
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
    #[must_use]
    pub fn neu(by_feld_id: &'a HashMap<String, &'a Bindung>) -> Self {
        Self { by_feld_id }
    }

    /// Direkter Nachschlag ohne Instanz-Aufloesung.
    #[must_use]
    pub fn get(&self, feld_id: &str) -> Option<&'a Bindung> {
        self.by_feld_id.get(feld_id).copied()
    }

    /// `feld_id` direkt, sonst dessen Instanz-Basis (`store.py:213-224`:
    /// `_pruefe_typ_konformitaet`, "unbekanntes `feld_id`: durchlassen, nicht raten" bleibt beim
    /// Aufrufer — hier nur der Nachschlag selbst).
    #[must_use]
    pub fn basis_eintrag(&self, feld_id: &str) -> Option<&'a Bindung> {
        self.get(feld_id)
            .or_else(|| instanz_basis(feld_id).and_then(|b| self.get(b)))
    }

    /// Alle Bindungen mit ihrem `feld_id` (`store.py:529`, `_rechne_ab`: `for ziel, eintrag in
    /// bindung.items()` durchsucht JEDEN Eintrag nach einer `ableitung`-Regel, nicht nur den
    /// eines einzelnen `feld_id`).
    pub fn alle(&self) -> impl Iterator<Item = (&'a str, &'a Bindung)> + '_ {
        self.by_feld_id.iter().map(|(k, v)| (k.as_str(), *v))
    }
}

/// Baut die flache `feld_id -> &Bindung`-Map aus allen geladenen `BindungDatei`s
/// (`registry::Registry`).
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
        assert_eq!(instanz_basis("vv_einnahmen"), None);
        assert_eq!(instanz_basis("vv_einnahmen__0"), None); // [1-9][0-9]* -- keine fuehrende 0
        assert_eq!(instanz_basis("vv_einnahmen__"), None);
        assert_eq!(instanz_basis("__2"), None); // Basis darf nicht leer sein
    }
}
