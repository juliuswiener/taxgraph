//! Kontoauszug-Uebernahme (`produkt/eingang/kontoauszug_writer.py`, JSON-Zweig aus
//! `api.py:973-979`): parsen, deterministisch klassifizieren, LLM nur als gedeckelter Rueckfall,
//! je Ausgabe mit sicherer Kategorie EIN vorlaeufiger Vorschlag. Fail-closed: keine Kategorie
//! oder kein Zielfeld → kein Vorschlag.
use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use llm::pii::{self, Maskiert};
use llm::py::{self, PyInt, PyRegex};
use llm::Kategorie;
use serde_json::{json, Value};
use store::{BindungNachschlag, Katalog, Store};

use crate::csv;
use crate::vorschlag::{Quelle, SchreibFehler, VorschlagEvent};

/// `LLM_AUFRUFE_HOECHSTZAHL`.
pub const LLM_AUFRUFE_HOECHSTZAHL: usize = 50;
/// `LLM_ZEITBUDGET_S`.
pub const LLM_ZEITBUDGET: Duration = Duration::from_secs(300);

/// `KATEGORIE_FELD`: Kategorie → Ziel-`feld_id`.
///
/// ```
/// assert_eq!(eingang::kontoauszug::zielfeld(llm::Kategorie::Spende), "spenden_betrag");
/// ```
#[must_use]
pub fn zielfeld(k: Kategorie) -> &'static str {
    match k {
        Kategorie::Handwerker => "hh_handwerker_betrag",
        Kategorie::Dienstleistung => "hh_dienstleistung_betrag",
        Kategorie::Minijob => "hh_minijob_betrag",
        Kategorie::Spende => "spenden_betrag",
        Kategorie::Vorsorge => "vor_rv_ausserhalb_lstb",
    }
}

const SCHLUESSELWOERTER: [(Kategorie, &[&str]); 5] = [
    (
        Kategorie::Handwerker,
        &[
            "maler",
            "sanitär",
            "elektr",
            "dachdeck",
            "handwerk",
            "installat",
            "klempner",
            "heizung",
            "renovier",
            "modernisier",
        ],
    ),
    (
        Kategorie::Dienstleistung,
        &[
            "reinigung",
            "putzhilfe",
            "gartenpfleg",
            "hausmeister",
            "gebäudereinig",
            "fensterputz",
            "winterdienst",
        ],
    ),
    (
        Kategorie::Minijob,
        &["minijob", "haushaltshilfe", "minijob-zentrale"],
    ),
    (
        Kategorie::Spende,
        &[
            "spende",
            "zuwendung",
            "hilfswerk",
            "stiftung",
            " e.v",
            "e. v.",
            "tierheim",
            "unicef",
            "rotes kreuz",
        ],
    ),
    (
        Kategorie::Vorsorge,
        &[
            "rentenversicherung",
            "rürup",
            "basisrente",
            "altersvorsorge",
            "rürup-rente",
        ],
    ),
];

/// `klassifiziere_det(zweck)`: erste Kategorie, deren Schluesselwort im kleingeschriebenen
/// Zweck steht.
///
/// ```
/// assert_eq!(eingang::kontoauszug::klassifiziere_det("MALERMEISTER Huber"), Some(llm::Kategorie::Handwerker));
/// assert_eq!(eingang::kontoauszug::klassifiziere_det("Miete"), None);
/// ```
#[must_use]
pub fn klassifiziere_det(zweck: &str) -> Option<Kategorie> {
    let z = zweck.to_lowercase();
    SCHLUESSELWOERTER
        .iter()
        .find(|(_, keys)| keys.iter().any(|k| z.contains(k)))
        .map(|(k, _)| *k)
}

/// Eine Buchung. `datum` bleibt roher JSON-Wert: der JSON-Zweig reicht ihn unveraendert in
/// `signal_1` durch.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaktion {
    pub datum: Value,
    pub betrag: i64,
    pub verwendungszweck: String,
}

/// Fehler beim Lesen oder Uebernehmen eines Auszugs.
#[derive(Debug, thiserror::Error)]
pub enum KontoauszugFehler {
    #[error(transparent)]
    Csv(#[from] csv::CsvFehler),
    /// Python: `OverflowError` (`inf`) bzw. grosse Ganzzahl ausserhalb `i64`.
    #[error("Betrag ausserhalb des Wertebereichs: {0}")]
    BetragUeberlauf(String),
    /// JSON-Zweig: `int(betrag)` scheitert (Python: `ValueError`/`TypeError` → 500, Audit B §6 #6).
    #[error("Transaktion {index}: betrag nicht ganzzahlig lesbar")]
    BetragUngueltig { index: usize },
    /// JSON-Zweig: Element ist kein Objekt bzw. der Zweck einer Ausgabe kein Text (Python:
    /// `AttributeError` → 500); `meldung` ist Pythons Text (`'int' object has no attribute 'lower'`).
    #[error("Transaktion {index}: {meldung}")]
    TransaktionUngueltig { index: usize, meldung: String },
    #[error(transparent)]
    Schreiben(#[from] SchreibFehler),
}

/// `float(s)` fuer Text: [`domain::py_float`].
///
/// ```
/// assert_eq!(eingang::kontoauszug::py_float(" 1_000.5 "), Some(1000.5));
/// assert_eq!(eingang::kontoauszug::py_float("1__0"), None);
/// ```
#[must_use]
pub fn py_float(s: &str) -> Option<f64> {
    domain::py_float(s)
}

/// Nur eindeutige Cent-Schreibweisen (Vault `decisions/kontoauszug-betrag-cent-genau-oder-verworfen`):
/// deutsch mit Tausenderpunkten und Komma (`1.234,56`), sonst Komma oder Punkt mit hoechstens
/// zwei Stellen (`480,5`/`480.00`/`480`). Gleiches Muster wie `_BETRAG_RE` in Python.
static BETRAG_CSV: LazyLock<PyRegex> = LazyLock::new(|| {
    PyRegex::neu(r"^(?:(\d{1,3}(?:\.\d{3})+),(\d{1,2})|(\d+)(?:[,.](\d{1,2}))?)$")
});

/// `_eur_cent_signed(s)`: `'-480,00'`/`'1.234,56'` → Cent mit Vorzeichen; unlesbar → `None`
/// (u. a. `1e3`, `-1.234`, `1,2,3`, `inf` und Werte ausserhalb `i64`). Ganzzahlig, ohne `f64`.
///
/// ```
/// assert_eq!(eingang::kontoauszug::eur_cent_signed("-1.234,56 €"), Some(-123_456));
/// assert_eq!(eingang::kontoauszug::eur_cent_signed("-1.234"), None);
/// assert_eq!(eingang::kontoauszug::eur_cent_signed("1e3"), None);
/// ```
#[must_use]
pub fn eur_cent_signed(roh: &str) -> Option<i64> {
    let s = py::strip(roh).replace(['€', ' '], "");
    let neg = s.starts_with('-');
    // Python liest Dezimalziffern jeder Schrift (`\d`, `int`); hier vorher auf ASCII abgebildet.
    let s = py::ascii_ziffern(s.trim_start_matches(['+', '-']));
    let g = BETRAG_CSV.gruppen(&s)??;
    let feld = |i: usize| g.get(i).cloned().flatten();
    let ganz = feld(0).or_else(|| feld(2))?.replace('.', "");
    let bruch = format!("{:0<2}", feld(1).or_else(|| feld(3)).unwrap_or_default());
    let cent = ganz
        .parse::<i64>()
        .ok()?
        .checked_mul(100)?
        .checked_add(bruch.parse().ok()?)?;
    Some(if neg { -cent } else { cent })
}

/// `parse_csv(text)`: `;`-getrennt, Spalten per Alias (case-insensitiv), Zeilen ohne Betrag
/// uebersprungen, Zeilen mit unlesbarem Betrag gezaehlt (zweiter Wert).
///
/// ```
/// let (tx, verworfen) = eingang::kontoauszug::parse_csv("Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler\n02.03.2025;1e3;X\n").unwrap();
/// assert_eq!((tx[0].betrag, verworfen), (-48000, 1));
/// ```
///
/// # Errors
/// [`KontoauszugFehler::Csv`].
pub fn parse_csv(text: &str) -> Result<(Vec<Transaktion>, usize), KontoauszugFehler> {
    let (kopf, zeilen, csv_fehler) = csv::dict_reader(text, ';');
    if kopf.is_empty() {
        return csv_fehler.map_or(Ok((Vec::new(), 0)), |e| Err(e.into()));
    }
    let mut norm: Vec<(String, String)> = Vec::new();
    for f in &kopf {
        let k = py::strip(f).to_lowercase();
        match norm.iter_mut().find(|(n, _)| *n == k) {
            Some(e) => e.1.clone_from(f),
            None => norm.push((k, f.clone())),
        }
    }
    let spalte = |aliase: &[&str]| {
        aliase
            .iter()
            .find_map(|a| norm.iter().find(|(n, _)| n == a).map(|(_, f)| f.clone()))
    };
    let c_dat = spalte(&["datum", "buchungstag", "buchungsdatum", "date"]);
    let c_bet = spalte(&["betrag", "umsatz", "amount", "betrag (eur)"]);
    let c_zwk = spalte(&[
        "verwendungszweck",
        "buchungstext",
        "zweck",
        "beschreibung",
        "description",
    ]);
    let wert = |z: &csv::Zeile, c: &Option<String>| {
        c.as_ref()
            .and_then(|c| z.get(c).cloned().flatten())
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    let mut verworfen = 0;
    for z in &zeilen {
        let betrag_roh = wert(z, &c_bet);
        if c_bet.is_none() || py::strip(&betrag_roh).is_empty() {
            continue;
        }
        let Some(betrag) = eur_cent_signed(&betrag_roh) else {
            verworfen += 1;
            continue;
        };
        out.push(Transaktion {
            datum: Value::String(py::strip(&wert(z, &c_dat)).to_owned()),
            betrag,
            verwendungszweck: py::strip(&wert(z, &c_zwk)).to_owned(),
        });
    }
    // Der `csv.Error` kommt in Python erst beim Lesen der Zeile, in der er steckt — also nach
    // allen frueheren Zeilen.
    csv_fehler.map_or(Ok((out, verworfen)), |e| Err(e.into()))
}

/// JSON-Zweig: die Eingabe von `uebernehme_kontoauszug` (`int(tx.get("betrag", 0))`, `tx.get(...)`):
/// Liste von Objekten. Anders als Python wird die GANZE Liste vor jedem Schreiben geprueft; Python
/// bricht mitten in der Schleife ab (500) und verwirft den Fall ungespeichert — nach aussen gleich.
///
/// Die Route ruft vorher [`verwirf_unlesbare_betraege_json`] (`api.py:1019`): eine Buchung, deren
/// Betrag nicht lesbar ist oder ab 10^10 Cent liegt, fliegt dort einzeln raus und kommt hier nie
/// an. Was hier noch scheitert, scheitert in Python in `uebernehme_kontoauszug` selbst: ein Element
/// ohne Objekt, ein Betrag, den `int()` nicht liest, und ein Nicht-Text als Zweck AUF EINER
/// AUSGABE (`betrag < 0`; bei den anderen Buchungen liest Python den Zweck nie).
///
/// ```
/// let tx = eingang::kontoauszug::aus_json(&serde_json::json!([{"betrag": "-5", "verwendungszweck": "Spende"}])).unwrap();
/// assert_eq!(tx[0].betrag, -5);
/// assert!(eingang::kontoauszug::aus_json(&serde_json::json!([{"betrag": "1,5"}])).is_err());
/// // Ein Zweck, der kein Text ist, wirft nur bei einer Ausgabe.
/// assert!(eingang::kontoauszug::aus_json(&serde_json::json!([{"betrag": 5, "verwendungszweck": 7}])).is_ok());
/// assert!(eingang::kontoauszug::aus_json(&serde_json::json!([{"betrag": -5, "verwendungszweck": 7}])).is_err());
/// ```
///
/// # Errors
/// [`KontoauszugFehler::BetragUngueltig`] / [`KontoauszugFehler::TransaktionUngueltig`].
pub fn aus_json(liste: &Value) -> Result<Vec<Transaktion>, KontoauszugFehler> {
    let leer = Vec::new();
    let liste = liste.as_array().unwrap_or(&leer);
    liste
        .iter()
        .enumerate()
        .map(|(index, tx)| {
            let o = tx
                .as_object()
                .ok_or_else(|| KontoauszugFehler::TransaktionUngueltig {
                    index,
                    meldung: format!("'{}' object has no attribute 'get'", py_typ(tx)),
                })?;
            let betrag = match o.get("betrag").map_or(PyInt::Wert(0), py::py_int) {
                PyInt::Wert(b) => b,
                PyInt::Ueberlauf => {
                    return Err(KontoauszugFehler::BetragUeberlauf(format!(
                        "{:?}",
                        o.get("betrag")
                    )))
                }
                PyInt::WertFehler | PyInt::TypFehler => {
                    return Err(KontoauszugFehler::BetragUngueltig { index })
                }
            };
            let verwendungszweck = match o.get("verwendungszweck") {
                None => String::new(),
                Some(Value::String(s)) => s.clone(),
                // Ein falscher Wert (null, 0, "") ist in Python `(z or "")` und damit leer. Ein
                // wahrer Nicht-Text wirft dort `AttributeError`, aber nur, wo Python den Zweck liest:
                // bei einer Ausgabe (`betrag >= 0` springt vorher weiter).
                Some(v) if !py::wahr(v) || betrag >= 0 => String::new(),
                Some(v) => {
                    return Err(KontoauszugFehler::TransaktionUngueltig {
                        index,
                        meldung: format!("'{}' object has no attribute 'lower'", py_typ(v)),
                    })
                }
            };
            Ok(Transaktion {
                datum: o.get("datum").cloned().unwrap_or_else(|| json!("")),
                betrag,
                verwendungszweck,
            })
        })
        .collect()
}

/// Pythons `type(v).__name__` fuer einen JSON-Wert.
fn py_typ(v: &Value) -> &'static str {
    match v {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::Number(n) if n.is_f64() => "float",
        Value::Number(_) => "int",
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "dict",
    }
}

/// `BETRAG_GRENZE_CENT`: ab hier weist der Store einen Vorschlag ab (Auflage F2/Magnitude).
pub const BETRAG_GRENZE_CENT: u64 = 10_000_000_000;

/// `_betrag_tragbar(tx)`: ob der Betrag in `uebernehme_kontoauszug` ankommt — eine Zahl unter 10^10
/// Cent. NaN, Infinity, Text ohne Zahl, `null`, Liste und ein Element ohne Objekt sind es nicht.
///
/// ```
/// use eingang::kontoauszug::betrag_tragbar;
/// use serde_json::json;
/// assert!(betrag_tragbar(&json!({"betrag": "-1_5"})));
/// assert!(betrag_tragbar(&json!({})));
/// assert!(!betrag_tragbar(&json!({"betrag": 10_000_000_000_i64})));
/// assert!(!betrag_tragbar(&json!({"betrag": null})));
/// assert!(!betrag_tragbar(&json!(5)));
/// ```
#[must_use]
pub fn betrag_tragbar(tx: &Value) -> bool {
    let Some(o) = tx.as_object() else {
        return false;
    };
    match o.get("betrag").map_or(PyInt::Wert(0), py::py_int) {
        PyInt::Wert(b) => b.unsigned_abs() < BETRAG_GRENZE_CENT,
        PyInt::WertFehler | PyInt::TypFehler | PyInt::Ueberlauf => false,
    }
}

/// `verwirf_unlesbare_betraege(transaktionen, n_verworfen)` fuer die Elemente eines JSON-Auszugs
/// (Vault `decisions/kontoauszug-betrag-cent-genau-oder-verworfen`): eine Buchung, deren Betrag
/// [`betrag_tragbar`] nicht trägt, fliegt einzeln raus und zaehlt in `n_verworfen`; die uebrigen
/// bleiben. Den Auszug ganz abzulehnen ist ausdruecklich nicht gewollt.
#[must_use]
pub fn verwirf_unlesbare_betraege_json(liste: &[Value], n_verworfen: usize) -> (Vec<Value>, usize) {
    let ok: Vec<Value> = liste
        .iter()
        .filter(|tx| betrag_tragbar(tx))
        .cloned()
        .collect();
    let weg = liste.len() - ok.len();
    (ok, n_verworfen + weg)
}

/// Wie [`verwirf_unlesbare_betraege_json`], fuer die Buchungen aus CSV und PDF (der Betrag ist dort
/// schon eine Zahl; nur ab 10^10 Cent fliegt eine Buchung raus).
///
/// ```
/// use eingang::kontoauszug::{verwirf_unlesbare_betraege, Transaktion};
/// let t = |betrag| Transaktion { datum: serde_json::json!(""), betrag, verwendungszweck: String::new() };
/// let (ok, n) = verwirf_unlesbare_betraege(vec![t(-500), t(i64::MAX), t(-10_000_000_000), t(9_999_999_999)], 1);
/// assert_eq!((ok.len(), n), (2, 3));
/// ```
#[must_use]
pub fn verwirf_unlesbare_betraege(
    transaktionen: Vec<Transaktion>,
    n_verworfen: usize,
) -> (Vec<Transaktion>, usize) {
    let gesamt = transaktionen.len();
    let ok: Vec<Transaktion> = transaktionen
        .into_iter()
        .filter(|t| t.betrag.unsigned_abs() < BETRAG_GRENZE_CENT)
        .collect();
    let weg = gesamt - ok.len();
    (ok, n_verworfen + weg)
}

/// `hinweis_verworfen(n, fmt)`: der Satz fuer den Nutzer, warum `n` Zeilen eines Auszugs im Format
/// `fmt` nicht uebernommen wurden.
///
/// ```
/// assert_eq!(
///     eingang::kontoauszug::hinweis_verworfen(2, "csv"),
///     "2 Zeile(n) mit unlesbarem Betrag (keine Zahl oder ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen."
/// );
/// ```
#[must_use]
pub fn hinweis_verworfen(n: usize, fmt: &str) -> String {
    let grund = if fmt == "pdf" {
        "unsicher erkannt (Confidence < 60%) oder mit zu großem Betrag (ab 100 Mio. €) verworfen"
    } else {
        "mit unlesbarem Betrag (keine Zahl oder ab 100 Mio. €) verworfen"
    };
    format!("{n} Zeile(n) {grund} — bitte manuell prüfen/nachtragen.")
}

static SALDO: LazyLock<PyRegex> =
    LazyLock::new(|| PyRegex::neu(r"(?i)(kontostand|saldo)\b|\bzwischensumme\b"));
static DATUM_ZEILE: LazyLock<PyRegex> =
    LazyLock::new(|| PyRegex::neu(r"^(\d{2}\.\d{2}\.\d{4})\s+(.*)$"));
static BETRAG_TOKEN: LazyLock<PyRegex> =
    LazyLock::new(|| PyRegex::neu(r"[+-]?\d{1,3}(?:\.\d{3})*,\d{2}"));
static EUR_ENDE: LazyLock<PyRegex> = LazyLock::new(|| PyRegex::neu(r"(?i)\s*(?:EUR|€)\s*$"));
static LEERRAUM: LazyLock<PyRegex> = LazyLock::new(|| PyRegex::neu(r"\s+"));

/// `parse_pdf_zeilen(text, conf_map, schwelle)`: Zeilen mit Datum vorn und GENAU einem Betrag
/// und Confidence ≥ Schwelle werden Buchungen; mehrdeutige/unsichere zaehlen als verworfen.
/// Ein Regex-Laufzeitfehler verwirft die Zeile (fail-closed).
///
/// ```
/// let (tx, verworfen) = eingang::kontoauszug::parse_pdf_zeilen("01.03.2025 Maler Huber -480,00 EUR\nSaldo -1,00", &Default::default(), 0.6);
/// assert_eq!((tx[0].betrag, tx[0].verwendungszweck.as_str(), verworfen), (-48000, "Maler Huber", 1));
/// ```
#[must_use]
pub fn parse_pdf_zeilen(
    text: &str,
    conf: &crate::ocr::ConfMap,
    schwelle: f64,
) -> (Vec<Transaktion>, usize) {
    let mut out = Vec::new();
    let mut verworfen = 0;
    for (i, zeile) in py::splitlines(text).into_iter().enumerate() {
        let z = py::strip(zeile);
        if z.is_empty() {
            continue;
        }
        match SALDO.sucht(z) {
            Some(false) => {}
            Some(true) => {
                if BETRAG_TOKEN
                    .finde_alle(z)
                    .is_none_or(|b| b.iter().any(|t| t.starts_with('-')))
                {
                    verworfen += 1;
                }
                continue;
            }
            None => {
                verworfen += 1;
                continue;
            }
        }
        let Some(Some(gruppen)) = DATUM_ZEILE.gruppen(z) else {
            continue;
        };
        let (datum, rest) = match gruppen.as_slice() {
            [Some(d), Some(r)] => (d.clone(), r.clone()),
            _ => continue,
        };
        let Some(betraege) = BETRAG_TOKEN.finde_alle(&rest) else {
            verworfen += 1;
            continue;
        };
        let [betrag] = betraege.as_slice() else {
            if betraege.len() > 1 {
                verworfen += 1;
            }
            continue;
        };
        if conf.get(&i).copied().unwrap_or(1.0) < schwelle {
            verworfen += 1;
            continue;
        }
        // Nur ausserhalb `i64`; das Token ist sonst lesbar.
        let Some(cent) = eur_cent_signed(betrag) else {
            verworfen += 1;
            continue;
        };
        let zweck = rest.replacen(betrag.as_str(), "", 1);
        let zweck = EUR_ENDE
            .ersetze(&zweck, |_| String::new())
            .map_or(zweck.clone(), |(t, _)| t);
        let zweck = LEERRAUM
            .ersetze(&zweck, |_| " ".to_owned())
            .map_or(zweck.clone(), |(t, _)| t);
        out.push(Transaktion {
            datum: Value::String(datum),
            betrag: cent,
            verwendungszweck: py::strip(&zweck).to_owned(),
        });
    }
    (out, verworfen)
}

/// LLM-Rueckfall: maskierter Zweck und Betrag → Kategorie oder `None` (unklassifiziert).
pub type Klassifikator<'a> = dyn Fn(&Maskiert, i64) -> Option<Kategorie> + 'a;

/// Ergebnis einer Uebernahme: `(uebernommen, llm_uebersprungen)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Uebernahme {
    pub uebernommen: usize,
    /// Buchungen, die wegen Aufruf- oder Zeitdeckel NIE klassifiziert wurden — sonst saehe ein
    /// halb angesehener Auszug aus wie ein ganz geprüfter.
    pub llm_uebersprungen: usize,
}

/// `uebernehme_kontoauszug(store, transaktionen, bindung, llm_klassifikator=, ts=, katalog=)`.
///
/// `bindung` = Bindung der Scheibe (Zielfeld muss darin stehen); `katalog` = globaler Katalog,
/// fehlt er, wird er aus `bindung` gebaut. `aktiv` wird EINMAL vorab gelesen und nur um die
/// eigenen Schreibungen ergaenzt — wie Python; ein zwischenzeitlich abgeleitetes Feld weist der
/// Store dann ab.
///
/// # Errors
/// [`KontoauszugFehler`] (Store-Abweisung, Betragsueberlauf).
///
/// ```
/// use eingang::kontoauszug::{uebernehme, Transaktion};
/// use store::Store;
/// use store::BindungNachschlag;
/// let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
/// let mut store = Store::leer(2025, None);
/// let tx = Transaktion { datum: serde_json::json!("2025-03-01"), betrag: -1500, verwendungszweck: "Miete".into() };
/// let erg = uebernehme(&mut store, &[tx], nachschlag, None, None, None).unwrap();
/// assert_eq!(erg.llm_uebersprungen, 0); // ohne Klassifikator wird nichts uebersprungen
/// ```
pub fn uebernehme(
    store: &mut Store,
    transaktionen: &[Transaktion],
    bindung: BindungNachschlag<'_>,
    llm_klassifikator: Option<&Klassifikator<'_>>,
    ts: Option<&str>,
    katalog: Option<&Katalog>,
) -> Result<Uebernahme, KontoauszugFehler> {
    let eigener;
    let katalog = if let Some(k) = katalog {
        k
    } else {
        eigener = Katalog::aus_bindungen(bindung.alle().map(|(_, b)| b));
        &eigener
    };
    let mut aktiv: HashSet<String> = store.aktive().map(|(f, _)| f.to_owned()).collect();
    let (mut n, mut aufrufe, mut uebersprungen) = (0, 0, 0);
    let beginn = Instant::now();
    for tx in transaktionen {
        if tx.betrag >= 0 {
            continue;
        }
        let zweck = &tx.verwendungszweck;
        let det = klassifiziere_det(zweck);
        let mut kategorie = det;
        if kategorie.is_none() {
            if let Some(k) = llm_klassifikator {
                if aufrufe >= LLM_AUFRUFE_HOECHSTZAHL || beginn.elapsed() > LLM_ZEITBUDGET {
                    uebersprungen += 1;
                    continue;
                }
                aufrufe += 1;
                kategorie = k(&pii::maskiere(zweck), tx.betrag);
            }
        }
        let Some(kategorie) = kategorie else { continue };
        let feld = zielfeld(kategorie);
        if bindung.get(feld).is_none() || aktiv.contains(feld) {
            continue;
        }
        let betrag_abs = tx
            .betrag
            .checked_abs()
            .ok_or_else(|| KontoauszugFehler::BetragUeberlauf(tx.betrag.to_string()))?;
        let signal_1 = json!({"typ": "kontoauszug", "datum": tx.datum, "betrag": tx.betrag,
            "verwendungszweck": pii::maskiere(zweck).as_str(), "kategorie": kategorie.als_str(),
            "quelle": if det.is_some() { "heuristik" } else { "llm" }});
        let v = VorschlagEvent {
            quelle: Quelle::Kontoauszug,
            feld_id: feld.to_owned(),
            wert: json!(betrag_abs),
            signal_1,
        };
        v.schreibe(store, Some(katalog), bindung, ts)?;
        aktiv.insert(feld.to_owned());
        n += 1;
    }
    Ok(Uebernahme {
        uebernommen: n,
        llm_uebersprungen: uebersprungen,
    })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{aus_json, eur_cent_signed, hinweis_verworfen, parse_csv, KontoauszugFehler};

    /// Vault `decisions/kontoauszug-betrag-cent-genau-oder-verworfen`; Python-Gegenstueck
    /// `tests/test_kontoauszug_writer.py::test_eur_cent_signed_tabelle`.
    #[test]
    fn betrag_tabelle() {
        for (roh, cent) in [
            ("1.234,56", Some(123_456)),
            ("-480,00", Some(-48_000)),
            ("480,5", Some(48_050)),
            ("480", Some(48_000)),
            ("480.00", Some(48_000)),
            ("480.5", Some(48_050)),
            ("-1200,00 €", Some(-120_000)),
            ("92233720368547758,07", Some(i64::MAX)),
            ("-1.234", None),
            ("1e3", None),
            ("inf", None),
            ("abc", None),
            ("1,2,3", None),
            ("1.234.567", None),
            ("480,055", None),
            ("92233720368547758,08", None),
        ] {
            assert_eq!(eur_cent_signed(roh), cent, "{roh:?}");
        }
        // Lange Ziffernfolgen (Python: int() liest hoechstens 4300 Ziffern, fuehrende Nullen zaehlen nicht).
        for (roh, cent) in [
            ("1".repeat(4301), None),
            (format!("{}1,00", "0".repeat(5000)), Some(100)),
            (format!("{}1,00", "٠".repeat(5000)), Some(100)),
            (format!("1{},00", ".000".repeat(1500)), None),
        ] {
            assert_eq!(eur_cent_signed(&roh), cent, "{} Zeichen", roh.len());
        }
    }

    /// Der Satz fuer den Nutzer nennt je Format den Grund, der dort wirklich zutrifft.
    #[test]
    fn hinweis_nennt_den_grund_je_format() {
        assert_eq!(
            hinweis_verworfen(3, "pdf"),
            "3 Zeile(n) unsicher erkannt (Confidence < 60%) oder mit zu großem Betrag (ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen."
        );
        for fmt in ["csv", "json"] {
            assert_eq!(
                hinweis_verworfen(1, fmt),
                "1 Zeile(n) mit unlesbarem Betrag (keine Zahl oder ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen."
            );
        }
    }

    /// `aus_json` ohne die Vorauswahl der Route: ein Element ohne Objekt und ein Zweck ohne Text
    /// scheitern mit Pythons Meldung (`tx.get`, `.lower()`); ein Zweck ohne Text bei einer Einnahme nicht.
    #[test]
    fn aus_json_meldungen_wie_python() {
        use serde_json::json;
        let meldung = |liste| match aus_json(&liste) {
            Err(KontoauszugFehler::TransaktionUngueltig { index, meldung }) => {
                Some((index, meldung))
            }
            _ => None,
        };
        assert_eq!(
            meldung(json!([{"betrag": 1}, 5])),
            Some((1, "'int' object has no attribute 'get'".to_owned()))
        );
        assert_eq!(
            meldung(json!([null])),
            Some((0, "'NoneType' object has no attribute 'get'".to_owned()))
        );
        assert_eq!(
            meldung(json!([{"betrag": -1, "verwendungszweck": 1.5}])),
            Some((0, "'float' object has no attribute 'lower'".to_owned()))
        );
        assert_eq!(
            meldung(json!([{"betrag": -1, "verwendungszweck": [1]}])),
            Some((0, "'list' object has no attribute 'lower'".to_owned()))
        );
        assert!(aus_json(&json!([{"betrag": 1, "verwendungszweck": 5}])).is_ok());
        assert!(aus_json(&json!([{"betrag": -1, "verwendungszweck": null}])).is_ok());
        assert!(aus_json(&json!([{"betrag": -1, "verwendungszweck": 0}])).is_ok());
        let tx = aus_json(&json!([{"betrag": -1}])).unwrap();
        assert_eq!(tx[0].datum, json!(""));
    }

    /// AK1: eine Zeile mit unlesbarem Betrag zaehlt in `verworfen`, die lesbare bleibt.
    #[test]
    fn csv_unlesbarer_betrag_zaehlt_in_verworfen() {
        let csv = "datum;betrag;verwendungszweck\n\
            15.03.2025;-1200,00;Ruerup-Rente Jahresbeitrag Basisrente\n\
            16.03.2025;abc;Ruerup-Rente Nachzahlung Basisrente\n\
            17.03.2025;1,2,3;Ruerup-Rente Sonderzahlung Basisrente\n";
        let Ok((tx, verworfen)) = parse_csv(csv) else {
            panic!("parse_csv scheitert")
        };
        let betraege: Vec<i64> = tx.iter().map(|t| t.betrag).collect();
        assert_eq!((betraege, verworfen), (vec![-120_000], 2));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        /// Zwei Nachkommastellen ueber den ganzen `i64`-Bereich: der Ganzzahl-Parser trifft den
        /// Cent genau, mit Punkt, Komma, Tausenderpunkten und Euro-Zeichen.
        #[test]
        fn eur_cent_signed_zwei_stellen_centgenau(
            c in prop_oneof![-10_000_i64..10_000, -i64::MAX..=i64::MAX],
            komma in any::<bool>(),
            tausender in any::<bool>(),
            euro in any::<bool>(),
        ) {
            let z = format!("{:03}", c.unsigned_abs());
            let (ganz, bruch) = z.split_at(z.len() - 2);
            let mut gruppiert = String::new();
            for (i, ziffer) in ganz.chars().enumerate() {
                if komma && tausender && i > 0 && (ganz.len() - i) % 3 == 0 {
                    gruppiert.push('.');
                }
                gruppiert.push(ziffer);
            }
            let text = format!(
                "{}{gruppiert}{}{bruch}{}",
                if c < 0 { "-" } else { "" },
                if komma { ',' } else { '.' },
                if euro { " €" } else { "" }
            );
            prop_assert_eq!(eur_cent_signed(&text), Some(c));
        }
    }
}
