//! Die bindungsweiten Tabellen (`traverser.py:26-103`, `lade_*`) und die [`Sicht`] auf eine
//! Teilmenge der Bindung, wie sie `api.py:_scheibe_bindung` je Scheibe uebergibt.
//!
//! WARUM ZWEI EBENEN: `relevanz`/`naechste_fragen` bekommen in Python eine Teil-Bindung (die
//! Felder der Scheibe, in Scheiben-Reihenfolge), lesen aber `regel_bedingungen`,
//! `instanz_gruppen` und `themen_zuerst` IMMER aus allen `bindung_*.yaml`. Der [`Graph`] haelt
//! die globalen Tabellen, die [`Sicht`] die uebergebene Teil-Bindung.
use std::collections::HashMap;

use bindung::{Bindung, InstanzGruppe, RegelBedingung, Registry};

/// Ein `feld_id` der Teil-Bindung steht in keiner Bindungsdatei (`api.py:188-190`: HTTP 500
/// "Bindungstabelle unvollständig für Scheibe").
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Bindungstabelle unvollstaendig: {0:?} nicht gebunden")]
pub struct UnbekanntesFeld(pub String);

/// Eine geordnete Bindung ohne Doppel: `feld_id -> &Bindung` in Aufrufer-Reihenfolge — das
/// Python-`dict`, das `traverser.py` als `bindung` bekommt.
///
/// Die Reihenfolge ist Semantik an genau einer Stelle: `relevanz` bricht die Gate-Schleife beim
/// ersten ausschliessenden Gate ab (`traverser.py:331-333`), `gates_offen` enthaelt dann nur die
/// Gates davor.
#[derive(Debug, Clone, Default)]
pub struct Sicht<'r> {
    eintraege: Vec<&'r Bindung>,
    index: HashMap<&'r str, usize>,
}

impl<'r> Sicht<'r> {
    /// Baut eine Sicht; ein zweites Vorkommen desselben `feld_id` behaelt die erste Position
    /// (Python: `{f: b[f] for f in felder}`).
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// let s = interview::Sicht::aus(g.alle().iter().take(3).chain(g.alle().iter().take(1)));
    /// assert_eq!(s.len(), 3);
    /// ```
    pub fn aus(bindungen: impl IntoIterator<Item = &'r Bindung>) -> Self {
        let mut sicht = Self::default();
        for b in bindungen {
            if !sicht.index.contains_key(b.feld_id.as_str()) {
                sicht
                    .index
                    .insert(b.feld_id.as_str(), sicht.eintraege.len());
                sicht.eintraege.push(b);
            }
        }
        sicht
    }

    /// Der Eintrag zu `feld_id` (`bindung.get(feld_id)`).
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// assert!(g.alle().get("veranlagung").is_some());
    /// assert!(g.alle().get("gibt_es_nicht").is_none());
    /// ```
    #[must_use]
    pub fn get(&self, feld_id: &str) -> Option<&'r Bindung> {
        self.index
            .get(feld_id)
            .and_then(|&i| self.eintraege.get(i))
            .copied()
    }

    /// Alle Eintraege in Sicht-Reihenfolge.
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// assert_eq!(g.alle().iter().count(), g.alle().len());
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &'r Bindung> + '_ {
        self.eintraege.iter().copied()
    }

    /// Die `feld_id`s in Sicht-Reihenfolge.
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// assert_eq!(g.alle().feld_ids().count(), g.alle().len());
    /// ```
    pub fn feld_ids(&self) -> impl Iterator<Item = &'r str> + '_ {
        self.eintraege.iter().map(|b| b.feld_id.as_str())
    }

    /// Zahl der Eintraege.
    ///
    /// ```
    /// assert_eq!(interview::Sicht::default().len(), 0);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.eintraege.len()
    }

    /// `true` ohne Eintraege.
    ///
    /// ```
    /// assert!(interview::Sicht::default().is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.eintraege.is_empty()
    }
}

/// Die globalen Tabellen aller `bindung_*.yaml` plus die volle [`Sicht`].
#[derive(Debug)]
pub struct Graph<'r> {
    alle: Sicht<'r>,
    regel_bedingungen: HashMap<&'r str, Vec<&'r RegelBedingung>>,
    /// `feld -> regel_ids`, deren `regel_bedingungen` dieses Feld nennen (je Regel einmal) —
    /// die Umkehrung, die `gate_gewicht` (`traverser.py:373-375`) je Feld braucht.
    bedingte_regeln: HashMap<&'r str, Vec<&'r str>>,
    instanz_gruppen: HashMap<&'r str, &'r InstanzGruppe>,
    themen_zuerst: Vec<&'r str>,
}

impl<'r> Graph<'r> {
    /// Baut die Tabellen aus der Registry (`lade_bindung`, `lade_regel_bedingungen`,
    /// `lade_instanz_gruppen`, `lade_themen_zuerst`).
    ///
    /// PARITAET: Python liest die Dateien in `glob`-Reihenfolge (Dateisystem), Rust sortiert
    /// (`lade_registry`). Das aendert nur die Reihenfolge der vollen Sicht; jede Produktions-
    /// Lesestelle bekommt in Python eine Scheiben-Bindung in Scheiben-Reihenfolge
    /// ([`Graph::sicht`]). `themen_zuerst` steht nur in einer Datei, `instanz_gruppen` sind
    /// eindeutig (gemessen) — dort ist die Dateireihenfolge ohne Wirkung.
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// assert!(g.alle().len() > 100);
    /// assert!(!g.themen_zuerst().is_empty());
    /// ```
    #[must_use]
    pub fn aus_registry(registry: &'r Registry) -> Self {
        let mut regel_bedingungen: HashMap<&str, Vec<&RegelBedingung>> = HashMap::new();
        let mut bedingte_regeln: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut instanz_gruppen = HashMap::new();
        let mut themen_zuerst: Vec<&str> = Vec::new();
        for (_, datei) in &registry.dateien {
            for rb in &datei.regel_bedingungen {
                regel_bedingungen
                    .entry(rb.regel_id.as_str())
                    .or_default()
                    .push(rb);
                let regeln = bedingte_regeln.entry(rb.feld.as_str()).or_default();
                if !regeln.contains(&rb.regel_id.as_str()) {
                    regeln.push(rb.regel_id.as_str());
                }
            }
            // Python: `out[g["gruppe"]] = g` — der letzte gewinnt.
            for g in &datei.instanz_gruppen {
                instanz_gruppen.insert(g.gruppe.as_str(), g);
            }
            for t in &datei.themen_zuerst {
                if !themen_zuerst.contains(&t.regel_id.as_str()) {
                    themen_zuerst.push(t.regel_id.as_str());
                }
            }
        }
        let alle = Sicht::aus(
            registry
                .dateien
                .iter()
                .flat_map(|(_, d)| d.bindungen.iter()),
        );
        Self {
            alle,
            regel_bedingungen,
            bedingte_regeln,
            instanz_gruppen,
            themen_zuerst,
        }
    }

    /// Die volle Bindung (`lade_bindung()`), Dateien alphabetisch.
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// assert!(g.alle().get("veranlagung").is_some());
    /// ```
    #[must_use]
    pub fn alle(&self) -> &Sicht<'r> {
        &self.alle
    }

    /// Teil-Bindung in der Reihenfolge von `felder` (`api.py:185-191`, `_scheibe_bindung`).
    ///
    /// # Errors
    /// [`UnbekanntesFeld`] beim ersten nicht gebundenen `feld_id`.
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// let s = g.sicht(["veranlagung", "bruttoarbeitslohn"]).unwrap();
    /// assert_eq!(s.feld_ids().collect::<Vec<_>>(), ["veranlagung", "bruttoarbeitslohn"]);
    /// assert!(g.sicht(["gibt_es_nicht"]).is_err());
    /// ```
    pub fn sicht<'f>(
        &self,
        felder: impl IntoIterator<Item = &'f str>,
    ) -> Result<Sicht<'r>, UnbekanntesFeld> {
        let mut out = Vec::new();
        for f in felder {
            out.push(
                self.alle
                    .get(f)
                    .ok_or_else(|| UnbekanntesFeld(f.to_owned()))?,
            );
        }
        Ok(Sicht::aus(out))
    }

    /// Die `regel_bedingungen` einer Regel (leer, wenn keine).
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// assert!(!g.regel_bedingungen("p2_festzusetzung_zusammen").is_empty());
    /// ```
    #[must_use]
    pub fn regel_bedingungen(&self, regel_id: &str) -> &[&'r RegelBedingung] {
        self.regel_bedingungen
            .get(regel_id)
            .map_or(&[], Vec::as_slice)
    }

    /// Die Regeln, deren `regel_bedingungen` `feld` nennen.
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// assert!(g.bedingte_regeln("veranlagung").contains(&"p2_festzusetzung_zusammen"));
    /// ```
    #[must_use]
    pub fn bedingte_regeln(&self, feld: &str) -> &[&'r str] {
        self.bedingte_regeln.get(feld).map_or(&[], Vec::as_slice)
    }

    /// Instanz-Gruppe nach Name (`lade_instanz_gruppen().get(gruppe)`).
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// assert!(g.instanz_gruppe("kind").is_some());
    /// ```
    #[must_use]
    pub fn instanz_gruppe(&self, gruppe: &str) -> Option<&'r InstanzGruppe> {
        self.instanz_gruppen.get(gruppe).copied()
    }

    /// Alle Instanz-Gruppen (`lade_instanz_gruppen().values()`), in unbestimmter Reihenfolge.
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// assert!(g.instanz_gruppen().any(|x| x.gruppe == "kind"));
    /// ```
    pub fn instanz_gruppen(&self) -> impl Iterator<Item = &'r InstanzGruppe> + '_ {
        self.instanz_gruppen.values().copied()
    }

    /// Die Themen, die den Fragebogen eroeffnen, in Deklarations-Reihenfolge ohne Doppel.
    ///
    /// ```
    /// let reg = interview::doctest_registry().unwrap();
    /// let g = interview::Graph::aus_registry(&reg);
    /// assert_eq!(g.themen_zuerst().first(), Some(&"p2_festzusetzung_einzel"));
    /// ```
    #[must_use]
    pub fn themen_zuerst(&self) -> &[&'r str] {
        &self.themen_zuerst
    }

    /// Die Instanz-Gruppe eines Feldes in `sicht` (`_gruppe_von`, `traverser.py:122-125`):
    /// `None` ohne `instanz_gruppe`, bei leerem Namen oder ungepflegter Gruppe.
    pub(crate) fn gruppe_von(&self, sicht: &Sicht<'r>, feld_id: &str) -> Option<&'r InstanzGruppe> {
        let name = sicht
            .get(feld_id)?
            .instanz_gruppe
            .as_deref()
            .filter(|g| !g.is_empty())?;
        self.instanz_gruppe(name)
    }
}
