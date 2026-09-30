//! Die deterministischen Gates hinter dem Modell (`api_llm.py:367-402, 483-593, 749-838,
//! 896-956`): Beleg-Gate, Rueckfrage-Bindung, Buendelung, Verdraengung, Aussagen-Status,
//! Katalog-Verengung. Keines haengt davon ab, dass sich das Modell an eine Prompt-Regel haelt.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::parse::{Aussage, AussageStatus, Rueckfrage, Vorschlag};
use crate::pii::Gefiltert;
use crate::py::{self, GeordneteMap, PyRegex};

/// `RUECKFRAGEN_MAX` (`api_llm.py:480`).
pub const RUECKFRAGEN_MAX: usize = 8;

/// Ein Vorschlag, dessen Beleg das Gate bestanden hat. Nur [`beleg_geprueft`] baut ihn; nur
/// solche Vorschlaege darf der Aufrufer als vorlaeufiges Event schreiben.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct BelegterVorschlag(Vorschlag);

impl BelegterVorschlag {
    /// Der gepruefte Vorschlag.
    ///
    /// ```
    /// let (g, _) = llm::pii::filtere("ich fahre 20 km");
    /// let v = llm::parse::chat_parse(r#"[{"feld_id": "km", "wert": 20, "beleg": "20 km"}]"#).oder_leer_wie_python();
    /// let (ok, _) = llm::gates::beleg_geprueft(v, &g);
    /// assert_eq!(ok[0].vorschlag().feld_id, "km");
    /// ```
    #[must_use]
    pub fn vorschlag(&self) -> &Vorschlag {
        &self.0
    }
}

/// `_beleg_geprueft(vorschlaege, freitext)`: `(behalten, verworfen)`. Ab 3 Zeichen
/// Teilstring im normalisierten gefilterten Text, darunter als eigenes Wort
/// (`(?<!\w)beleg(?!\w)`).
///
/// ```
/// let (g, _) = llm::pii::filtere("15000 Euro, 5 Tage");
/// let v = llm::parse::chat_parse(r#"[{"feld_id": "a", "wert": 5, "beleg": "5"},
///     {"feld_id": "b", "wert": 1, "beleg": "erfunden"}]"#).oder_leer_wie_python();
/// let (ok, weg) = llm::gates::beleg_geprueft(v, &g);
/// assert_eq!((ok.len(), weg.len()), (1, 1));
/// ```
#[must_use]
pub fn beleg_geprueft(vorschlaege: Vec<Vorschlag>, freitext: &Gefiltert) -> (Vec<BelegterVorschlag>, Vec<Vorschlag>) {
    let heuhaufen = py::normalisiert(freitext.as_str());
    let mut behalten = Vec::new();
    let mut verworfen = Vec::new();
    for v in vorschlaege {
        let beleg = py::normalisiert(&v.beleg);
        let ok = !beleg.is_empty()
            && if py::laenge(&beleg) >= 3 { heuhaufen.contains(&beleg) } else { als_wort(&heuhaufen, &beleg) };
        if ok {
            behalten.push(BelegterVorschlag(v));
        } else {
            verworfen.push(v);
        }
    }
    (behalten, verworfen)
}

/// `re.search(rf"(?<!\w){re.escape(nadel)}(?!\w)", text)` — jede Fundstelle, auch
/// ueberlappende, wie der Suchlauf des Regex.
fn als_wort(text: &str, nadel: &str) -> bool {
    let mut start = 0;
    while let Some(i) = text.get(start..).and_then(|t| t.find(nadel)).map(|i| i + start) {
        let davor = text.get(..i).and_then(|t| t.chars().next_back());
        let danach = text.get(i + nadel.len()..).and_then(|t| t.chars().next());
        if !davor.is_some_and(py::ist_wortzeichen) && !danach.is_some_and(py::ist_wortzeichen) {
            return true;
        }
        start = i + text.get(i..).and_then(|t| t.chars().next()).map_or(1, char::len_utf8);
    }
    false
}

/// `_rueckfragen_gebuendelt`: hoechstens eine Rueckfrage je Aussage (`None` ist eine eigene
/// Gruppe), hoechstens [`RUECKFRAGEN_MAX`] insgesamt. Rueckgabe `(behalten, zurueckgestellt)`.
///
/// ```
/// let r = |a| llm::parse::Rueckfrage { frage: "?".into(), feld_id: String::new(), aussage: a };
/// let (b, weg) = llm::gates::rueckfragen_gebuendelt(vec![r(Some(0)), r(Some(0)), r(None)]);
/// assert_eq!((b.len(), weg), (2, 1));
/// ```
#[must_use]
pub fn rueckfragen_gebuendelt(rueckfragen: Vec<Rueckfrage>) -> (Vec<Rueckfrage>, usize) {
    let gesamt = rueckfragen.len();
    let mut gesehen = HashSet::new();
    let mut behalten = Vec::new();
    for r in rueckfragen {
        if gesehen.contains(&r.aussage) || behalten.len() >= RUECKFRAGEN_MAX {
            continue;
        }
        gesehen.insert(r.aussage);
        behalten.push(r);
    }
    let weg = gesamt - behalten.len();
    (behalten, weg)
}

/// Ein Katalogfeld, wie `api.py:1112-1122` es fuer den Chat baut. `Option::None` steht fuer
/// Pythons `None` (erscheint im Prompt als `None`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KatalogFeld {
    pub feld_id: String,
    #[serde(default)]
    pub fragetext_laie: Option<String>,
    #[serde(default)]
    pub hilfe_kurz: Option<String>,
    #[serde(default)]
    pub typ: Option<String>,
    #[serde(default)]
    pub bereich: Option<GeordneteMap>,
    #[serde(default)]
    pub enum_werte: Option<Vec<String>>,
    #[serde(default)]
    pub regel_id: Option<String>,
    #[serde(default)]
    pub instanz_gruppe: Option<String>,
}

/// Grund, aus dem eine Rueckfrage ihr Feld verlor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoeseGrund {
    Unbekannt,
    Zahlenart,
}

/// Protokoll-Eintrag einer geloesten Rueckfrage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Geloest {
    pub frage: String,
    pub feld_id: String,
    pub grund: LoeseGrund,
}

static GELD_WORT: LazyLock<PyRegex> = LazyLock::new(|| PyRegex::neu(r"(?i)(?<!\w)eur(?:o)?(?!\w)|€"));
static ANZAHL_WORT: LazyLock<PyRegex> = LazyLock::new(|| PyRegex::neu(r"(?i)(?<!\w)(?:wie\s*viele|anzahl)(?!\w)"));

/// `_rueckfragen_gebunden(rueckfragen, kat3)`: das Feld wird geloest (die Frage bleibt), wenn
/// es nicht im Katalog stand oder Geld↔`int` bzw. Anzahl↔`cent` nicht passt. Ein
/// Regex-Laufzeitfehler loest fail-closed.
///
/// ```
/// let kat: Vec<llm::gates::KatalogFeld> = serde_json::from_str(r#"[{"feld_id": "n", "typ": "int"}]"#).unwrap();
/// let r = vec![llm::parse::Rueckfrage { frage: "Wie viel Euro?".into(), feld_id: "n".into(), aussage: None }];
/// let (r, geloest) = llm::gates::rueckfragen_gebunden(r, &kat.iter().collect::<Vec<_>>());
/// assert_eq!((r[0].feld_id.as_str(), geloest.len()), ("", 1));
/// ```
#[must_use]
pub fn rueckfragen_gebunden(mut rueckfragen: Vec<Rueckfrage>, kat3: &[&KatalogFeld]) -> (Vec<Rueckfrage>, Vec<Geloest>) {
    let typen: HashMap<&str, Option<&str>> = kat3.iter().map(|f| (f.feld_id.as_str(), f.typ.as_deref())).collect();
    let mut geloest = Vec::new();
    for r in &mut rueckfragen {
        if r.feld_id.is_empty() {
            continue;
        }
        let grund = match typen.get(r.feld_id.as_str()) {
            None => LoeseGrund::Unbekannt,
            Some(typ) => {
                let geld = *typ == Some("int") && GELD_WORT.sucht(&r.frage).unwrap_or(true);
                let anzahl = *typ == Some("cent") && ANZAHL_WORT.sucht(&r.frage).unwrap_or(true);
                if !(geld || anzahl) {
                    continue;
                }
                LoeseGrund::Zahlenart
            }
        };
        geloest.push(Geloest { frage: r.frage.clone(), feld_id: std::mem::take(&mut r.feld_id), grund });
    }
    (rueckfragen, geloest)
}

/// `_rueckfrage_verdraengt`: eine Rueckfrage ersetzt den Vorschlag zum selben Feld.
///
/// ```
/// let (g, _) = llm::pii::filtere("20 km");
/// let v = llm::parse::chat_parse(r#"[{"feld_id": "km", "wert": 20, "beleg": "20 km"}]"#).oder_leer_wie_python();
/// let (ok, _) = llm::gates::beleg_geprueft(v, &g);
/// let r = vec![llm::parse::Rueckfrage { frage: "?".into(), feld_id: "km".into(), aussage: None }];
/// assert!(llm::gates::rueckfrage_verdraengt(ok, &r).is_empty());
/// ```
#[must_use]
pub fn rueckfrage_verdraengt(behalten: Vec<BelegterVorschlag>, rueckfragen: &[Rueckfrage]) -> Vec<BelegterVorschlag> {
    let gefragt: HashSet<&str> = rueckfragen.iter().filter(|r| !r.feld_id.is_empty()).map(|r| r.feld_id.as_str()).collect();
    behalten.into_iter().filter(|v| !gefragt.contains(v.0.feld_id.as_str())).collect()
}

/// `_zugerechnet(eintraege, aussagen)`: Nummer aus der Antwort, sonst Beleg-Abgleich.
fn zugerechnet<'a>(eintraege: impl Iterator<Item = (Option<i64>, &'a str)>, aussagen: &[Aussage]) -> HashSet<usize> {
    let belege: Vec<String> = aussagen.iter().map(|a| py::normalisiert(&a.beleg)).collect();
    let mut treffer = HashSet::new();
    for (nr, beleg) in eintraege {
        if let Some(i) = nr.and_then(|i| usize::try_from(i).ok()).filter(|i| *i < aussagen.len()) {
            treffer.insert(i);
            continue;
        }
        let eigen = py::normalisiert(beleg);
        if py::laenge(&eigen) < 3 {
            continue;
        }
        if let Some(k) = belege.iter().position(|b| py::laenge(b) >= 3 && (eigen.contains(b.as_str()) || b.contains(&eigen))) {
            treffer.insert(k);
        }
    }
    treffer
}

/// `_status_setzen`: jede Aussage bekommt ihr Ergebnis, auch „nichts".
pub(crate) fn status_setzen(
    aussagen: &mut [Aussage],
    zuordnungen: &BTreeMap<i64, Vec<String>>,
    behalten: &[BelegterVorschlag],
    verworfen: &[Vorschlag],
    rueckfragen: &[Rueckfrage],
) {
    let mit_vorschlag = zugerechnet(behalten.iter().map(|v| (v.0.aussage, v.0.beleg.as_str())), aussagen);
    let mit_rueckfrage = zugerechnet(rueckfragen.iter().map(|r| (r.aussage, "")), aussagen);
    let ohne_beleg = zugerechnet(verworfen.iter().map(|v| (v.aussage, v.beleg.as_str())), aussagen);
    for (i, a) in aussagen.iter_mut().enumerate() {
        a.regeln = i64::try_from(i).ok().and_then(|k| zuordnungen.get(&k)).cloned().unwrap_or_default();
        a.status = if mit_vorschlag.contains(&i) {
            AussageStatus::Vorschlag
        } else if mit_rueckfrage.contains(&i) {
            AussageStatus::Rueckfrage
        } else if ohne_beleg.contains(&i) {
            AussageStatus::OhneBeleg
        } else if a.regeln.is_empty() {
            AussageStatus::KeinThema
        } else {
            AussageStatus::KeinFeld
        };
    }
}

/// `_felder_je_regel`: `regel_id → Felder`, in Erst-Auftritts-Reihenfolge; ohne `regel_id` →
/// `""`.
///
/// ```
/// use llm::gates::KatalogFeld;
/// let feld = |id: &str, regel: Option<&str>, gruppe: Option<&str>| -> KatalogFeld {
///     serde_json::from_value(serde_json::json!({"feld_id": id, "regel_id": regel, "instanz_gruppe": gruppe})).unwrap()
/// };
/// use llm::gates::felder_je_regel;
/// let katalog = [feld("a", Some("r1"), None), feld("b", Some("r2"), None), feld("c", Some("r1"), None)];
/// let je = felder_je_regel(&katalog);
/// assert_eq!(je.iter().map(|(r, f)| (r.as_str(), f.len())).collect::<Vec<_>>(), [("r1", 2), ("r2", 1)]);
/// ```
#[must_use]
pub fn felder_je_regel(katalog: &[KatalogFeld]) -> Vec<(String, Vec<&KatalogFeld>)> {
    let mut je: Vec<(String, Vec<&KatalogFeld>)> = Vec::new();
    for f in katalog {
        let r = f.regel_id.clone().unwrap_or_default();
        match je.iter_mut().find(|(k, _)| *k == r) {
            Some((_, v)) => v.push(f),
            None => je.push((r, vec![f])),
        }
    }
    je
}

/// `_mit_zaehlfeldern(kat3, katalog)` in beide Richtungen; `gruppen` = `(gruppe, anzahl_feld)`
/// in `lade_instanz_gruppen()`-Reihenfolge.
///
/// ```
/// use llm::gates::KatalogFeld;
/// let feld = |id: &str, regel: Option<&str>, gruppe: Option<&str>| -> KatalogFeld {
///     serde_json::from_value(serde_json::json!({"feld_id": id, "regel_id": regel, "instanz_gruppe": gruppe})).unwrap()
/// };
/// use llm::gates::mit_zaehlfeldern;
/// let katalog = [feld("kind_name", None, Some("kind")), feld("fam_anzahl_kinder", None, None)];
/// let gruppen = [("kind".to_owned(), "fam_anzahl_kinder".to_owned())];
/// // Ein Instanzfeld ohne sein Zaehlfeld wuerde die Frage ins Leere stellen: das Zaehlfeld kommt dazu.
/// let mit = mit_zaehlfeldern(vec![&katalog[0]], &katalog, &gruppen);
/// assert!(mit.iter().any(|f| f.feld_id == "fam_anzahl_kinder"));
/// assert_eq!(mit_zaehlfeldern(vec![&katalog[0]], &katalog, &[]).len(), 1); // ohne Gruppen unveraendert
/// ```
#[must_use]
pub fn mit_zaehlfeldern<'a>(kat3: Vec<&'a KatalogFeld>, katalog: &'a [KatalogFeld], gruppen: &[(String, String)]) -> Vec<&'a KatalogFeld> {
    if gruppen.is_empty() {
        return kat3;
    }
    let drin: HashSet<&str> = kat3.iter().map(|f| f.feld_id.as_str()).collect();
    let anzahl_von: HashMap<&str, &str> = gruppen.iter().map(|(g, a)| (g.as_str(), a.as_str())).collect();
    let zu_gruppe: HashMap<&str, &str> = gruppen.iter().map(|(g, a)| (a.as_str(), g.as_str())).collect();
    let fehlende_zahl: HashSet<&str> = kat3
        .iter()
        .filter_map(|f| f.instanz_gruppe.as_deref().filter(|g| !g.is_empty()))
        .filter_map(|g| anzahl_von.get(g).copied())
        .filter(|a| !drin.contains(a))
        .collect();
    let offene_gruppen: HashSet<&str> = drin.iter().filter_map(|fid| zu_gruppe.get(fid).copied()).collect();
    if fehlende_zahl.is_empty() && offene_gruppen.is_empty() {
        return kat3;
    }
    let mut out = kat3.clone();
    out.extend(katalog.iter().filter(|f| {
        !drin.contains(f.feld_id.as_str())
            && (fehlende_zahl.contains(f.feld_id.as_str())
                || f.instanz_gruppe.as_deref().is_some_and(|g| offene_gruppen.contains(g)))
    }));
    out
}

#[cfg(test)]
mod tests {
    use super::als_wort;

    #[test]
    fn kurzer_beleg_braucht_wortgrenze() {
        assert!(!als_wort("15000 euro", "5"));
        assert!(als_wort("15000 euro, 5 tage", "5"));
        assert!(als_wort("5", "5"));
        assert!(!als_wort("5²", "5"));
    }
}
