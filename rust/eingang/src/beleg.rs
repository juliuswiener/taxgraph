//! Beleg-Import (`produkt/eingang/beleg_writer.py`): Lohnsteuerbescheinigung, Zuwendungs-
//! bestaetigung, Handwerker-/Dienstleistungsrechnung, Minijob-Bescheinigung → Kandidatenwerte →
//! vorlaeufige Vorschlaege. Deterministisch, ohne LLM; nicht gefunden = Luecke, nie geraten.
use std::collections::BTreeMap;
use std::sync::LazyLock;

use llm::py::{self, PyInt, PyRegex};
use serde::Serialize;
use serde_json::json;
use store::{BindungNachschlag, EventId, Katalog, Store};

use crate::vorschlag::{Quelle, SchreibFehler, VorschlagEvent};

/// Beleg-Typ (`BELEG_TYPEN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BelegTyp {
    Lstb,
    Spende,
    Handwerker,
    Dienstleistung,
    Minijob,
}

impl BelegTyp {
    /// `hs_prefix`: Typ-Tag in `herkunft_slots`.
    fn praefix(self) -> &'static str {
        match self {
            Self::Lstb => "lohnsteuerbescheinigung",
            Self::Spende => "zuwendungsbestätigung",
            Self::Handwerker => "handwerkerrechnung",
            Self::Dienstleistung => "dienstleistungsrechnung",
            Self::Minijob => "minijob-bescheinigung",
        }
    }

    /// Das Wire-Wort.
    ///
    /// ```
    /// assert_eq!(eingang::beleg::BelegTyp::Lstb.als_str(), "lstb");
    /// ```
    #[must_use]
    pub fn als_str(self) -> &'static str {
        match self {
            Self::Lstb => "lstb",
            Self::Spende => "spende",
            Self::Handwerker => "handwerker",
            Self::Dienstleistung => "dienstleistung",
            Self::Minijob => "minijob",
        }
    }
}

/// `erkenne_beleg_typ(text)`: fail-closed bei Handwerker/Dienstleistung-Mehrdeutigkeit
/// (§ 35a Abs. 2 und 3 haben verschiedene Hoechstbetraege).
///
/// ```
/// use eingang::beleg::{erkenne_beleg_typ, BelegTyp};
/// assert_eq!(erkenne_beleg_typ("Lohnsteuerbescheinigung 2025"), Some(BelegTyp::Lstb));
/// assert_eq!(erkenne_beleg_typ("Handwerker und haushaltsnahe Dienstleistung"), None);
/// ```
#[must_use]
pub fn erkenne_beleg_typ(text: &str) -> Option<BelegTyp> {
    let t = text.to_lowercase();
    if t.contains("lohnsteuerbescheinigung") {
        return Some(BelegTyp::Lstb);
    }
    if t.contains("zuwendungsbestätigung") || t.contains("geldzuwendung") {
        return Some(BelegTyp::Spende);
    }
    if t.contains("minijob") || t.contains("haushaltsscheck") {
        return Some(BelegTyp::Minijob);
    }
    let hw = t.contains("handwerker");
    let dl = t.contains("dienstleistung") || t.contains("haushaltsnah");
    match (hw, dl) {
        (true, false) => Some(BelegTyp::Handwerker),
        (false, true) => Some(BelegTyp::Dienstleistung),
        _ => None,
    }
}

/// Anker eines Felds: Formular-Position (`Nr. 3`) oder Fliesstext-Label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anker {
    Nr(String),
    Label(String),
}

impl Anker {
    fn wert(&self) -> &str {
        match self {
            Self::Nr(s) | Self::Label(s) => s,
        }
    }
}

static NR: LazyLock<PyRegex> = LazyLock::new(|| PyRegex::neu(r"Nr\.?\s*(\d+)"));
static EUR: LazyLock<PyRegex> = LazyLock::new(|| PyRegex::neu(r"\d{1,3}(?:\.\d{3})*,\d{2}"));

/// `_anker(hs)`.
fn anker(hs: &str) -> Anker {
    if let Some(Some(g)) = NR.gruppen(hs) {
        if let Some(Some(nr)) = g.first() {
            return Anker::Nr(nr.clone());
        }
    }
    Anker::Label(py::strip(hs.split_once(':').map_or(hs, |(_, r)| r)).to_owned())
}

/// `beleg_felder(bindung, typ)`: `feld_id → Anker` fuer `cent`-Felder, deren `herkunft_slots`
/// mit dem Typ-Praefix beginnen (der letzte passende Eintrag gewinnt).
#[must_use]
pub fn beleg_felder(bindung: BindungNachschlag<'_>, typ: BelegTyp) -> BTreeMap<String, Anker> {
    let mut out = BTreeMap::new();
    for (fid, b) in bindung.alle() {
        if b.typ != domain::Feldtyp::Cent {
            continue;
        }
        for hs in b.herkunft_slots.iter().flatten() {
            if hs.to_lowercase().starts_with(typ.praefix()) {
                out.insert(fid.to_owned(), anker(hs));
            }
        }
    }
    out
}

/// `_parse_eur_cent("45.000,00")` → 4 500 000 (`int()`, Dezimalziffern jeder Schrift).
fn eur_cent(betrag: &str) -> Option<i64> {
    match py::py_int_text(&betrag.replace(['.', ','], "")) {
        PyInt::Wert(c) => Some(c),
        _ => None,
    }
}

/// Zeile, in der `passt` greift → `(cent, zeile.strip())` des LETZTEN EUR-Betrags darauf.
fn finde(text: &str, passt: impl Fn(&str) -> bool) -> Option<(i64, String)> {
    py::splitlines(text).into_iter().filter(|z| passt(z)).find_map(|z| {
        let letzter = EUR.finde_alle(z)?.pop()?;
        Some((eur_cent(&letzter)?, py::strip(z).to_owned()))
    })
}

/// Ein Kandidat (`extrahiere`-Eintrag).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Kandidat {
    pub feld_id: String,
    pub wert: i64,
    pub confidence: f64,
    pub roh_text: String,
    pub beleg_typ: BelegTyp,
    pub anker: String,
}

/// `extrahiere(text, bindung, confidence_map=)`: Kandidaten sortiert nach `feld_id`; kein Typ →
/// leer; nicht gefundenes Feld → weggelassen.
#[must_use]
pub fn extrahiere(text: &str, bindung: BindungNachschlag<'_>, conf: &BTreeMap<String, f64>) -> Vec<Kandidat> {
    let Some(typ) = erkenne_beleg_typ(text) else { return Vec::new() };
    beleg_felder(bindung, typ)
        .into_iter()
        .filter_map(|(fid, a)| {
            let treffer = match &a {
                Anker::Nr(nr) => {
                    let e = regex::escape(nr);
                    let muster = PyRegex::neu(&format!(r"(?:Nr\.?\s*{e}\b|(?:^|\s){e}\.(?!\d))"));
                    // Laufzeitfehler: Zeile gilt als nicht passend (Luecke statt Rate-Wert).
                    finde(text, |z| muster.sucht(z).unwrap_or(false))
                }
                Anker::Label(l) => {
                    let lab = l.to_lowercase();
                    finde(text, |z| z.to_lowercase().contains(&lab))
                }
            }?;
            Some(Kandidat {
                feld_id: fid,
                wert: treffer.0,
                confidence: conf.get(a.wert()).copied().unwrap_or(1.0),
                roh_text: treffer.1,
                beleg_typ: typ,
                anker: a.wert().to_owned(),
            })
        })
        .collect()
}

/// `schreibe_kandidaten(store, kandidaten, beleg_ref=, bindung=, ts=)`: je Kandidat ein
/// vorlaeufiges Event, Katalog aus der Bindung (K1: nur beleg-freigegebene Felder).
///
/// # Errors
/// [`SchreibFehler`] beim ersten abgewiesenen Kandidaten (vorherige bleiben geschrieben, wie in
/// Python).
pub fn schreibe_kandidaten(
    store: &mut Store,
    kandidaten: &[Kandidat],
    beleg_ref: &str,
    bindung: BindungNachschlag<'_>,
    ts: Option<&str>,
) -> Result<Vec<EventId>, SchreibFehler> {
    let katalog = Katalog::aus_bindungen(bindung.alle().map(|(_, b)| b));
    kandidaten
        .iter()
        .map(|k| {
            let signal_1 = json!({"typ": "beleg", "ref": format!("{beleg_ref}#{}:{}", k.beleg_typ.als_str(), k.anker),
                "confidence": k.confidence, "roh_text": k.roh_text});
            VorschlagEvent { quelle: Quelle::Beleg, feld_id: k.feld_id.clone(), wert: json!(k.wert), signal_1 }
                .schreibe(store, Some(&katalog), bindung, ts)
        })
        .collect()
}
