//! Rohe Modelltexte → Werte (`_chat_parse`, `_rueckfragen_parse`, `_antwort_parse`,
//! `_aussagen_parse`, `_zuordnung_parse`, `_index`; `api_llm.py:405-472, 596-606, 675-708,
//! 863-891`).
//!
//! Python liest tolerant und macht aus kaputtem JSON stillschweigend eine leere Liste. Rust
//! liest genauso tolerant (sonst bricht die Paritaet), sagt aber im Ergebnistyp [`Antwort`], WIE
//! gelesen wurde: schemagerecht, tolerant, oder gar nicht — der Aufrufer muss den dritten Fall
//! benennen, statt ihn fuer „nichts gesagt" zu halten.
use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::pii::{self, Gefiltert};
use crate::py::{self, py_int, py_str, vorne, PyInt};
use crate::schema;

/// Wie eine Modellantwort gelesen wurde.
#[derive(Debug, Clone, PartialEq)]
pub enum Antwort<T> {
    /// JSON entspricht exakt dem strikten Schema (`deny_unknown_fields`).
    Schemagerecht(T),
    /// JSON lesbar, aber nicht schemagerecht (Wrapper-Schluessel, nacktes Array, fremde Felder,
    /// falsche Wurzel) — tolerant gelesen wie Python, moeglicherweise leer.
    Tolerant(T),
    /// Kein JSON. Python: leere Liste bzw. `("", False)`, ohne Fehler.
    Unlesbar,
}

impl<T: Default> Antwort<T> {
    /// Der gelesene Wert; `Unlesbar` wird — wie in Python — zum leeren Wert. Ausdruecklich
    /// benannt, damit die Stelle, die „kaputt" wie „leer" behandelt, im Code sichtbar ist.
    ///
    /// ```
    /// let a: llm::Antwort<Vec<u8>> = llm::Antwort::Unlesbar;
    /// assert!(a.oder_leer_wie_python().is_empty());
    /// ```
    pub fn oder_leer_wie_python(self) -> T {
        match self {
            Self::Schemagerecht(t) | Self::Tolerant(t) => t,
            Self::Unlesbar => T::default(),
        }
    }
}

fn lies(text: &str) -> Option<Value> {
    serde_json::from_str(text).ok()
}

fn einordnen<T, S: serde::de::DeserializeOwned>(j: &Value, wert: T) -> Antwort<T> {
    if schema::passt::<S>(j) {
        Antwort::Schemagerecht(wert)
    } else {
        Antwort::Tolerant(wert)
    }
}

/// `_index(w)`: eine Aussage-Nummer ≥ 0 oder `None`. PARITAET-Abweichung: `int(inf)` wirft in
/// Python `OverflowError` ungefangen; hier `None`.
///
/// ```
/// use serde_json::json;
/// assert_eq!(llm::parse::index(&json!(2)), Some(2));
/// assert_eq!(llm::parse::index(&json!(-1)), None);
/// assert_eq!(llm::parse::index(&json!("3")), Some(3));
/// ```
#[must_use]
pub fn index(w: &Value) -> Option<i64> {
    match py_int(w) {
        PyInt::Wert(i) if i >= 0 => Some(i),
        _ => None,
    }
}

/// Ein Wert-Vorschlag aus Stufe 3 (`_chat_parse`-Eintrag).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Vorschlag {
    pub feld_id: String,
    pub wert: Value,
    pub beleg: String,
    pub begruendung: String,
    pub aussage: Option<i64>,
    pub rechenweg: Value,
}

/// `_chat_parse(text)`: Wrapper `vorschlaege`/`vorschläge`/`suggestions`/`felder`, nacktes
/// Array oder Einzelobjekt mit `feld_id`.
///
/// ```
/// let a = llm::parse::chat_parse(r#"{"suggestions": [{"feld_id": "x", "wert": 1}]}"#);
/// assert!(matches!(a, llm::Antwort::Tolerant(ref v) if v.len() == 1));
/// assert_eq!(llm::parse::chat_parse("kaputt"), llm::Antwort::Unlesbar);
/// ```
#[must_use]
pub fn chat_parse(text: &str) -> Antwort<Vec<Vorschlag>> {
    let Some(j) = lies(text) else {
        return Antwort::Unlesbar;
    };
    let liste: Vec<&Value> = match &j {
        Value::Object(o) => {
            let wrapper = ["vorschlaege", "vorschläge", "suggestions", "felder"]
                .iter()
                .find_map(|k| o.get(*k).and_then(Value::as_array));
            match wrapper {
                Some(a) => a.iter().collect(),
                None if o.contains_key("feld_id") => vec![&j],
                None => Vec::new(),
            }
        }
        Value::Array(a) => a.iter().collect(),
        _ => Vec::new(),
    };
    let out = liste
        .into_iter()
        .filter_map(|v| {
            let o = v.as_object()?;
            let (fid, wert) = (o.get("feld_id")?, o.get("wert")?);
            let text_von = |k: &str, n: usize| {
                o.get(k)
                    .map_or_else(String::new, |x| vorne(&py_str(x), n).to_owned())
            };
            Some(Vorschlag {
                feld_id: py_str(fid),
                wert: wert.clone(),
                beleg: text_von("beleg", 300),
                begruendung: text_von("begruendung", 200),
                aussage: o.get("aussage").and_then(index),
                rechenweg: o.get("rechenweg").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    einordnen::<_, schema::DialogStreng>(&j, out)
}

/// Eine Rueckfrage aus Stufe 3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rueckfrage {
    pub frage: String,
    pub feld_id: String,
    pub aussage: Option<i64>,
}

/// `_rueckfragen_parse(text, felder)`: ohne Felder (`felder == 0`) faellt jede feldbezogene
/// Rueckfrage weg.
///
/// ```
/// let t = r#"{"rueckfragen": [{"frage": " Wie viel? ", "feld_id": "x", "aussage": 0}]}"#;
/// let r = llm::parse::rueckfragen_parse(t, 3).oder_leer_wie_python();
/// assert_eq!(r[0].frage, "Wie viel?");
/// assert!(llm::parse::rueckfragen_parse(t, 0).oder_leer_wie_python().is_empty());
/// ```
#[must_use]
pub fn rueckfragen_parse(text: &str, felder: usize) -> Antwort<Vec<Rueckfrage>> {
    let Some(j) = lies(text) else {
        return Antwort::Unlesbar;
    };
    let Some(liste) = j
        .get("rueckfragen")
        .filter(|_| j.is_object())
        .and_then(Value::as_array)
    else {
        return einordnen::<_, schema::DialogStreng>(&j, Vec::new());
    };
    let out = liste
        .iter()
        .filter_map(|r| {
            let o = r.as_object()?;
            let frage = o.get("frage").map_or_else(String::new, |f| {
                vorne(py::strip(&py_str(f)), 300).to_owned()
            });
            if frage.is_empty() {
                return None;
            }
            let fid = o
                .get("feld_id")
                .filter(|v| py::wahr(v))
                .map_or_else(String::new, |v| py::strip(&py_str(v)).to_owned());
            if !fid.is_empty() && felder == 0 {
                return None;
            }
            Some(Rueckfrage {
                frage,
                feld_id: fid,
                aussage: o.get("aussage").and_then(index),
            })
        })
        .collect();
    einordnen::<_, schema::DialogStreng>(&j, out)
}

/// `_antwort_parse(text)`: `(antwort, unsicher)`.
///
/// ```
/// let a = llm::parse::antwort_parse(r#"{"antwort": " Ja. ", "unsicher": 1}"#).oder_leer_wie_python();
/// assert_eq!(a, ("Ja.".to_string(), true));
/// ```
#[must_use]
pub fn antwort_parse(text: &str) -> Antwort<(String, bool)> {
    let Some(j) = lies(text) else {
        return Antwort::Unlesbar;
    };
    let Some(o) = j.as_object() else {
        return Antwort::Tolerant((String::new(), false));
    };
    let antwort = o
        .get("antwort")
        .filter(|v| py::wahr(v))
        .map_or_else(String::new, |v| {
            vorne(py::strip(&py_str(v)), 2000).to_owned()
        });
    let unsicher = o.get("unsicher").is_some_and(py::wahr);
    einordnen::<_, schema::DialogStreng>(&j, (antwort, unsicher))
}

/// Status einer Aussage aus Stufe 1 (`_status_setzen`, `_teilergebnis`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AussageStatus {
    Offen,
    Vorschlag,
    Rueckfrage,
    OhneBeleg,
    KeinThema,
    KeinFeld,
    ThemenAusgefallen,
    WerteAusgefallen,
}

/// Eine Tatsachenaussage aus Stufe 1. `text` ist Modellausgabe, erneut gefiltert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Aussage {
    pub text: String,
    pub beleg: String,
    pub status: AussageStatus,
    pub regeln: Vec<String>,
}

/// `_aussagen_parse(text, freitext)`: Aussage-Text geht erneut durch [`pii::filtere`]; ein
/// Beleg, der nicht (normalisiert, ≥ 3 Zeichen) im gefilterten Nutzertext steht, wird geleert,
/// die Aussage bleibt.
///
/// ```
/// let (g, _) = llm::pii::filtere("ich bin ledig");
/// let t = r#"{"aussagen": [{"text": "Der Nutzer ist ledig", "beleg": "bin ledig"}]}"#;
/// let a = llm::parse::aussagen_parse(t, &g).oder_leer_wie_python();
/// assert_eq!(a[0].beleg, "bin ledig");
/// ```
#[must_use]
pub fn aussagen_parse(text: &str, freitext: &Gefiltert) -> Antwort<Vec<Aussage>> {
    let Some(j) = lies(text) else {
        return Antwort::Unlesbar;
    };
    let Some(liste) = j
        .get("aussagen")
        .filter(|_| j.is_object())
        .and_then(Value::as_array)
    else {
        return einordnen::<_, schema::AussagenStreng>(&j, Vec::new());
    };
    let heuhaufen = py::normalisiert(freitext.as_str());
    let out = liste
        .iter()
        .filter_map(|a| {
            let o = a.as_object()?;
            let roh = o.get("text").map_or_else(String::new, py_str);
            let satz = pii::filtere(vorne(py::strip(&roh), 300))
                .0
                .as_str()
                .to_owned();
            if satz.is_empty() {
                return None;
            }
            let beleg = o
                .get("beleg")
                .map_or_else(String::new, |b| vorne(&py_str(b), 300).to_owned());
            let n = py::normalisiert(&beleg);
            let beleg = if py::laenge(&n) >= 3 && heuhaufen.contains(&n) {
                beleg
            } else {
                String::new()
            };
            Some(Aussage {
                text: satz,
                beleg,
                status: AussageStatus::Offen,
                regeln: Vec::new(),
            })
        })
        .collect();
    einordnen::<_, schema::AussagenStreng>(&j, out)
}

/// Ergebnis von Stufe 2: Regeln je Aussage-Nummer, und alle getroffenen Regeln in
/// Erst-Treffer-Reihenfolge.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Zuordnung {
    pub je_aussage: BTreeMap<i64, Vec<String>>,
    pub getroffen: Vec<String>,
}

/// `_zuordnung_parse(text, erlaubt, anzahl)`: erfundene Regeln fallen weg; getroffene Regeln
/// zaehlen unabhaengig von der Aussage-Nummer.
///
/// ```
/// let erlaubt: std::collections::HashSet<String> = ["r1".to_string()].into_iter().collect();
/// let t = r#"{"zuordnungen": [{"aussage": -1, "regeln": ["r1", "erfunden"]}]}"#;
/// let z = llm::parse::zuordnung_parse(t, &erlaubt, 2).oder_leer_wie_python();
/// assert_eq!(z.getroffen, vec!["r1"]);
/// assert!(z.je_aussage.is_empty());
/// ```
#[must_use]
pub fn zuordnung_parse<H: std::hash::BuildHasher>(
    text: &str,
    erlaubt: &std::collections::HashSet<String, H>,
    anzahl: usize,
) -> Antwort<Zuordnung> {
    let Some(j) = lies(text) else {
        return Antwort::Unlesbar;
    };
    let mut z = Zuordnung::default();
    let Some(liste) = j
        .get("zuordnungen")
        .filter(|_| j.is_object())
        .and_then(Value::as_array)
    else {
        return einordnen::<_, schema::ZuordnungStreng>(&j, z);
    };
    for eintrag in liste {
        let Some(regeln) = eintrag
            .as_object()
            .and_then(|o| o.get("regeln"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        let regeln: Vec<String> = regeln
            .iter()
            .map(py_str)
            .filter(|r| erlaubt.contains(r))
            .collect();
        if regeln.is_empty() {
            continue;
        }
        for r in &regeln {
            if !z.getroffen.contains(r) {
                z.getroffen.push(r.clone());
            }
        }
        let i = eintrag.get("aussage").and_then(index);
        if let Some(i) = i.filter(|i| usize::try_from(*i).is_ok_and(|i| i < anzahl)) {
            let bisher = z.je_aussage.entry(i).or_default();
            // `je[i] += [r for r in regeln if r not in je[i]]` — gegen den Stand VOR dem Anhaengen.
            let neu: Vec<String> = regeln
                .iter()
                .filter(|r| !bisher.contains(r))
                .cloned()
                .collect();
            bisher.extend(neu);
        }
    }
    einordnen::<_, schema::ZuordnungStreng>(&j, z)
}
