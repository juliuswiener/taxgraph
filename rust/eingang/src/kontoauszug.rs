//! Kontoauszug-Uebernahme (`produkt/eingang/kontoauszug_writer.py`, JSON-Zweig aus
//! `api.py:973-979`): parsen, deterministisch klassifizieren, LLM nur als gedeckelter Rueckfall,
//! je Zielfeld EIN vorlaeufiger Vorschlag: die Summe der Ausgaben des Zahlungsjahrs, abzueglich
//! der Rueckbuchungen derselben Kategorie (Bankimport S1). Fail-closed: keine Kategorie oder kein
//! Zielfeld → kein Vorschlag.
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use domain::{Schreiber, Zustand};
use llm::pii::{self, Maskiert};
use llm::py::{self, PyInt, PyRegex};
use llm::Kategorie;
use serde_json::{json, Value};
use store::{BindungNachschlag, Katalog, Store};

use crate::bank::buchung::{Datum, Posten};
use crate::bank::saldo;
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

/// Parteinamen (kleingeschrieben), bei denen `uebernehme` fuer eine Spende KEINEN Vorschlag schreibt: Eine Parteispende
/// gehoert nicht in `spenden_betrag`, sondern ins eigene Feld `parteispenden_betrag` (Abweichung Nr. 39).
// ponytail: geschlossene Liste. Sie trifft nur diese Namen in genau dieser Schreibweise: keine Beugung ("Gruenen", "Linken"),
// keine Waehlervereinigung, kein Ortsverband mit eigenem Namen; solche Buchungen bekommen weiter den alten Vorschlag. Mehr
// verlangt eine Quelle der Parteinamen (das Repo hat keine), nicht weitere Eintraege von Hand.
const PARTEINAMEN: [&str; 12] = [
    "partei",
    "parteispende",
    "spd",
    "cdu",
    "csu",
    "fdp",
    "afd",
    "bsw",
    "grüne",
    "bündnis 90",
    "die linke",
    "freie wähler",
];

/// Nennt der Zweck einen Parteinamen als GANZES Wort? Ein Treffer mitten in einem Wort ("Schlafdecke" enthaelt "afd") zaehlt
/// nicht; Satzzeichen, Bindestrich und Schraegstrich begrenzen ein Wort ("SPD-Ortsverband", "Bündnis 90/Die Grünen"), Buchstaben
/// und Ziffern nicht.
fn nennt_partei(zweck: &str) -> bool {
    let z = zweck.to_lowercase();
    let wortzeichen = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    PARTEINAMEN.iter().any(|name| {
        z.match_indices(name).any(|(anfang, _)| {
            let davor = z.get(..anfang).and_then(|s| s.chars().next_back());
            let danach = z.get(anfang + name.len()..).and_then(|s| s.chars().next());
            !wortzeichen(davor) && !wortzeichen(danach)
        })
    })
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

/// So viele Datensaetze von oben sucht der Leser die Kopfzeile: Banken stellen ihr Kontostand, Zeitraum
/// und Kontonummer voran (DKB 4 bis 5 Zeilen, ING 13).
const KOPF_SUCHE: usize = 30;

/// Die Trenner, die der Leser nacheinander versucht; `;` zuerst (deutsche Banken, die Dateien vor S0).
const TRENNER: [char; 3] = [';', ',', '\t'];

// Spaltennamen (klein) je Bedeutung, in der Reihenfolge ihres Vorrangs. Alle aus nachgebauten Dateien
// nach Fremdparsern (Vault `research/bankimport-entwurf-version-1-…`, M1): KEIN Bankdokument bestaetigt
// sie, eine echte Datei kann anders heissen.
const ALIASE_DATUM: [&str; 6] = [
    "datum",
    "buchungstag",
    "buchungsdatum",
    "date",
    "buchung",
    "completed date",
];
const ALIASE_BETRAG: [&str; 6] = [
    "betrag",
    "umsatz",
    "amount",
    "betrag (eur)",
    "betrag (€)",
    "umsatz in eur",
];
const ALIASE_ZWECK: [&str; 6] = [
    "verwendungszweck",
    "buchungstext",
    "zweck",
    "beschreibung",
    "description",
    "vorgang/verwendungszweck",
];

/// Kopfnamen klein und ohne Rand; ein doppelter Name behaelt den letzten (wie `DictReader`).
fn normalisiere(kopf: &[String]) -> Vec<(String, String)> {
    let mut norm: Vec<(String, String)> = Vec::new();
    for f in kopf {
        let k = py::strip(f).to_lowercase();
        match norm.iter_mut().find(|(n, _)| *n == k) {
            Some(e) => e.1.clone_from(f),
            None => norm.push((k, f.clone())),
        }
    }
    norm
}

/// Der erste Alias (in Rangfolge), den der Kopf fuehrt, als Name im Kopf.
fn spalte(norm: &[(String, String)], aliase: &[&str]) -> Option<String> {
    aliase
        .iter()
        .find_map(|a| norm.iter().find(|(n, _)| n == a).map(|(_, f)| f.clone()))
}

/// Was `parse_csv_befund` ueber die Datei weiss, damit die Route sagen kann, warum nichts gelesen wurde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvBefund {
    /// Der Trenner, mit dem gelesen wurde (`;`, wenn keiner eine Kopfzeile fand).
    pub trenner: char,
    /// Nr. (ab 0) des Datensatzes, der die Kopfzeile mit Betragsspalte ist; `None` = keine gefunden.
    pub kopf_nr: Option<usize>,
    /// Die Spaltennamen der Kopfzeile; ohne Betragsspalte die des ersten Datensatzes, der nach einer
    /// Kopfzeile aussieht (fuehrt ein Datums- oder Zweck-Alias), sonst leer. Nie eine Datenzeile.
    pub spalten: Vec<String>,
    /// Zeilen unter der Kopfzeile, deren Betragsfeld leer ist (uebersprungen).
    pub ohne_betrag: usize,
}

/// `parse_csv(text)`: Spalten per Alias (case-insensitiv), Zeilen ohne Betrag uebersprungen, Zeilen
/// mit unlesbarem Betrag gezaehlt (zweiter Wert). Wie [`parse_csv_befund`], ohne den Befund.
///
/// ```
/// let (tx, verworfen) = eingang::kontoauszug::parse_csv("Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler\n02.03.2025;1e3;X\n").unwrap();
/// assert_eq!((tx[0].betrag, verworfen), (-48000, 1));
/// ```
///
/// # Errors
/// [`KontoauszugFehler::Csv`].
pub fn parse_csv(text: &str) -> Result<(Vec<Transaktion>, usize), KontoauszugFehler> {
    parse_csv_befund(text).map(|(tx, verworfen, _)| (tx, verworfen))
}

/// Liest einen CSV-Auszug und sagt dazu, was es gefunden hat. Die Kopfzeile steht in den ersten
/// 30 (`KOPF_SUCHE`) Datensaetzen: der erste, der eine Betragsspalte fuehrt, mit dem ersten Trenner
/// (`;`, `,`, Tab), bei dem es einen gibt. Fuehrt keiner eine, liest der Leser wie vor S0: `;`, erster
/// Datensatz als Kopf, keine Buchung. Ein BOM am Anfang gehoert nicht zum ersten Spaltennamen.
///
/// Die Profile (DKB, ING, comdirect, N26, Revolut) stammen aus Fremdparsern, nicht aus einem
/// Bankdokument.
///
/// ```
/// let text = "Konto;DE89\nKontostand;1,00\n\nBuchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler\n";
/// let (tx, verworfen, befund) = eingang::kontoauszug::parse_csv_befund(text).unwrap();
/// assert_eq!((tx.len(), verworfen, befund.kopf_nr), (1, 0, Some(3)));
/// ```
///
/// # Errors
/// [`KontoauszugFehler::Csv`].
pub fn parse_csv_befund(
    text: &str,
) -> Result<(Vec<Transaktion>, usize, CsvBefund), KontoauszugFehler> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut befund = CsvBefund {
        trenner: ';',
        kopf_nr: None,
        spalten: Vec::new(),
        ohne_betrag: 0,
    };
    for trenner in TRENNER {
        let (anfang, _) = csv::lies_anfang(text, trenner, KOPF_SUCHE);
        let fuehrt =
            |satz: &[String], aliase: &[&str]| spalte(&normalisiere(satz), aliase).is_some();
        if let Some((nr, satz)) = anfang
            .iter()
            .enumerate()
            .find(|(_, s)| fuehrt(s, &ALIASE_BETRAG))
        {
            befund.trenner = trenner;
            befund.kopf_nr = Some(nr);
            befund.spalten.clone_from(satz);
            break;
        }
        if befund.spalten.is_empty() {
            if let Some(s) = anfang
                .iter()
                .find(|s| fuehrt(s, &ALIASE_DATUM) || fuehrt(s, &ALIASE_ZWECK))
            {
                befund.spalten.clone_from(s);
            }
        }
    }
    let (saetze, csv_fehler) = csv::lies_datensaetze(text, befund.trenner);
    let (kopf, zeilen) = csv::dict_aus_saetzen(saetze, befund.kopf_nr.unwrap_or(0));
    if kopf.is_empty() {
        return csv_fehler.map_or(Ok((Vec::new(), 0, befund)), |e| Err(e.into()));
    }
    let norm = normalisiere(&kopf);
    let c_dat = spalte(&norm, &ALIASE_DATUM);
    let c_bet = spalte(&norm, &ALIASE_BETRAG);
    let c_zwk = spalte(&norm, &ALIASE_ZWECK);
    let wert = |z: &csv::Zeile, c: &Option<String>| {
        c.as_ref()
            .and_then(|c| z.get(c).cloned().flatten())
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    let mut verworfen = 0;
    for z in &zeilen {
        let betrag_roh = wert(z, &c_bet);
        if c_bet.is_none() {
            continue;
        }
        if py::strip(&betrag_roh).is_empty() {
            befund.ohne_betrag += 1;
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
    csv_fehler.map_or(Ok((out, verworfen, befund)), |e| Err(e.into()))
}

/// JSON-Zweig: die Eingabe von `uebernehme_kontoauszug` (`int(tx.get("betrag", 0))`, `tx.get(...)`):
/// Liste von Objekten. Anders als Python wird die GANZE Liste vor jedem Schreiben geprueft; Python
/// bricht mitten in der Schleife ab (500) und verwirft den Fall ungespeichert — nach aussen gleich.
///
/// Die Route ruft vorher [`verwirf_unlesbare_betraege_json`] (`api.py:1019`): eine Buchung, deren
/// Betrag nicht lesbar ist oder ab 10^10 Cent liegt, fliegt dort einzeln raus und kommt hier nie
/// an. Was hier noch scheitert, scheitert in Python in `uebernehme_kontoauszug` selbst: ein Element
/// ohne Objekt, ein Betrag, den `int()` nicht liest, und ein Nicht-Text als Zweck AUF EINER
/// AUSGABE (`betrag < 0`; bei den anderen Buchungen liest Python den Zweck nie). Die Route ruft
/// zuvor [`pruefe_buchungsfelder`]: den Zweck weist sie dort mit 422 ab, `aus_json` sieht ihn nie.
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

/// Der Typ der Zahl in `wert` (beliebig tief), die die Fallakte nicht exakt haelt: `"float"` fuer eine nicht
/// endliche Kommazahl, `"int"` fuer eine Ganzzahl ausserhalb von `i64`; `"float"` geht vor. Beide kennt
/// `serde_json` nicht und die Rust-Fassung der Akte sperrt sie; die Route kennzeichnet sie im JSON-Text
/// mit einem Text, der mit `marke` beginnt und auf `float` oder `int` endet.
fn nicht_tragbare_zahl(wert: &Value, marke: Option<&str>) -> Option<&'static str> {
    let mut offen = vec![wert];
    let mut gefunden = None;
    while let Some(w) = offen.pop() {
        match w {
            Value::Object(o) => offen.extend(o.values()),
            Value::Array(a) => offen.extend(a),
            Value::String(s) => match markentyp(s, marke) {
                Some("float") => return Some("float"),
                Some(typ) => gefunden = Some(typ),
                None => {}
            },
            _ => {}
        }
    }
    gefunden
}

/// Der Typ, den die Kennzeichnung `marke` im Text `s` nennt.
fn markentyp(s: &str, marke: Option<&str>) -> Option<&'static str> {
    let rest = s.strip_prefix(marke?)?;
    Some(if rest == "float" { "float" } else { "int" })
}

/// `pruefe_buchungsfelder(transaktionen)` (`kontoauszug_writer.py`), nach [`verwirf_unlesbare_betraege_json`]:
/// das `datum` jeder Buchung traegt keine Zahl, die die Akte nicht haelt, und der `verwendungszweck`
/// einer Ausgabe ist Text (falsch heisst leer). Erst alle Daten, dann alle Zwecke, wie Python. Der
/// Text der Meldung nennt Feld und Typ, nie den Wert; die Route macht daraus ein 422.
///
/// `marke`: das Kennzeichen der Zahlen, die die Route im JSON-Text gefunden und durch einen Text ersetzt hat.
///
/// ```
/// use eingang::kontoauszug::pruefe_buchungsfelder;
/// use serde_json::json;
/// let tx = json!([{"datum": "d", "betrag": -5, "verwendungszweck": "Spende"}]);
/// assert_eq!(pruefe_buchungsfelder(tx.as_array().unwrap(), None), Ok(()));
/// let kaputt = json!([{"datum": ["#float"], "betrag": -5}]);
/// assert!(pruefe_buchungsfelder(kaputt.as_array().unwrap(), Some("#")).unwrap_err().contains("(float)"));
/// let zweck = json!([{"betrag": 5, "verwendungszweck": 7}, {"betrag": -5, "verwendungszweck": [1]}]);
/// assert!(pruefe_buchungsfelder(zweck.as_array().unwrap(), None).unwrap_err().ends_with("nicht list."));
/// ```
///
/// # Errors
/// Der Text der Meldung, ohne das Praefix `Kontoauszug nicht lesbar: `.
pub fn pruefe_buchungsfelder(liste: &[Value], marke: Option<&str>) -> Result<(), String> {
    for tx in liste {
        if let Some(typ) = tx.get("datum").and_then(|d| nicht_tragbare_zahl(d, marke)) {
            return Err(format!(
                "datum einer Buchung enthält eine Zahl ({typ}), die die Akte nicht halten kann."
            ));
        }
    }
    for tx in liste {
        let Some(o) = tx.as_object() else { continue };
        let ausgabe =
            matches!(o.get("betrag").map_or(PyInt::Wert(0), py::py_int), PyInt::Wert(b) if b < 0);
        let Some(zweck) = o.get("verwendungszweck").filter(|z| ausgabe && py::wahr(z)) else {
            continue;
        };
        let typ = match zweck {
            Value::String(s) => markentyp(s, marke),
            andere => Some(py_typ(andere)),
        };
        if let Some(typ) = typ {
            return Err(format!(
                "verwendungszweck einer Ausgabe muss Text sein, nicht {typ}."
            ));
        }
    }
    Ok(())
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

/// Hoechstens so viele Spaltennamen und so viele Zeichen je Name nennt der Hinweis: Er zeigt die
/// Kopfzeile, nie eine Datenzeile, und bleibt kurz.
const HINWEIS_SPALTEN: usize = 20;
const HINWEIS_NAME_ZEICHEN: usize = 40;

/// Der Satz fuer den Nutzer, wenn ein CSV-Auszug KEINE Buchung lieferte (und keine Zeile verworfen
/// wurde): fehlt die Kopfzeile mit Betragsspalte, nennt er die erwarteten und die gefundenen
/// Spaltennamen, sonst, warum unter der Kopfzeile nichts steht.
///
/// ```
/// use eingang::kontoauszug::{hinweis_nichts_gelesen, parse_csv_befund};
/// let (_, _, b) = parse_csv_befund("Buchungstag;Wert;Verwendungszweck\n01.03.2025;5;X\n").unwrap();
/// let h = hinweis_nichts_gelesen(&b);
/// assert!(h.contains("keine Kopfzeile mit Betragsspalte") && h.contains("Buchungstag, Wert, Verwendungszweck"));
/// ```
#[must_use]
pub fn hinweis_nichts_gelesen(befund: &CsvBefund) -> String {
    if befund.kopf_nr.is_some() {
        return if befund.ohne_betrag > 0 {
            format!(
                "Keine Buchung gelesen: {} Zeile(n) unter der Kopfzeile haben keinen Betrag.",
                befund.ohne_betrag
            )
        } else {
            "Keine Buchung gelesen: Unter der Kopfzeile folgt keine Datenzeile.".to_owned()
        };
    }
    let gefunden = if befund.spalten.is_empty() {
        "keine Kopfzeile erkannt".to_owned()
    } else {
        let namen: Vec<String> = befund
            .spalten
            .iter()
            .filter(|s| !py::strip(s).is_empty())
            .take(HINWEIS_SPALTEN)
            .map(|s| py::strip(s).chars().take(HINWEIS_NAME_ZEICHEN).collect())
            .collect();
        format!("gefundene Spalten: {}", namen.join(", "))
    };
    format!(
        "Keine Buchung gelesen: In den ersten {KOPF_SUCHE} Zeilen steht keine Kopfzeile mit Betragsspalte \
         (erwartet: {}); {gefunden}.",
        ALIASE_BETRAG.join(", ")
    )
}

/// Der Satz fuer den Nutzer, wenn ein CSV-Auszug `gelesen` Buchungen lieferte und keine davon ein
/// Vorschlag wurde: die Zahl je Grund, nie ein Zweck oder eine Gegenpartei. Die Buchungen, die der
/// Deckel der KI-Aufrufe uebersprang, meldet die Route mit eigenem Satz.
///
/// ```
/// use eingang::kontoauszug::{hinweis_nichts_uebernommen, Uebernahme};
/// let u = Uebernahme {
///     uebernommen: 0,
///     llm_uebersprungen: 0,
///     einnahmen: 1,
///     ohne_stichwort: 2,
///     partei: 0,
///     feld_fehlt: 0,
///     feld_belegt: 0,
///     anderes_jahr: 0,
///     datum_unlesbar: 0,
///     saldo_nicht_positiv: 0,
/// };
/// assert_eq!(
///     hinweis_nichts_uebernommen(3, &u).as_deref(),
///     Some("3 Buchung(en) gelesen, aber kein Vorschlag: 1 Buchung(en) ohne Minuszeichen (nur Beträge mit Minus gelten als Ausgabe); 2 Buchung(en) ohne erkanntes Stichwort.")
/// );
/// let nur_deckel = Uebernahme { llm_uebersprungen: 3, einnahmen: 0, ohne_stichwort: 0, ..u };
/// assert_eq!(hinweis_nichts_uebernommen(3, &nur_deckel), None);
/// ```
#[must_use]
pub fn hinweis_nichts_uebernommen(gelesen: usize, u: &Uebernahme) -> Option<String> {
    let gruende = [
        (
            u.einnahmen,
            "ohne Minuszeichen (nur Beträge mit Minus gelten als Ausgabe)",
        ),
        (u.ohne_stichwort, "ohne erkanntes Stichwort"),
        (
            u.partei,
            "als Spende mit Parteinamen (dafür gibt es ein eigenes Feld)",
        ),
        (u.feld_fehlt, "für ein Feld, das dieser Fall nicht führt"),
        (u.feld_belegt, "für ein Feld, das schon einen Wert hat"),
        (u.anderes_jahr, GRUND_ANDERES_JAHR),
        (u.datum_unlesbar, GRUND_DATUM_UNLESBAR),
        (u.saldo_nicht_positiv, GRUND_SALDO),
    ];
    let teile: Vec<String> = gruende
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, text)| format!("{n} Buchung(en) {text}"))
        .collect();
    (!teile.is_empty()).then(|| {
        format!(
            "{gelesen} Buchung(en) gelesen, aber kein Vorschlag: {}.",
            teile.join("; ")
        )
    })
}

const GRUND_ANDERES_JAHR: &str = "außerhalb des Veranlagungsjahrs";
const GRUND_DATUM_UNLESBAR: &str =
    "mit einem Datum, das der Leser nicht erkennt (erwartet: TT.MM.JJJJ, TT.MM.JJ, JJJJ-MM-TT)";
const GRUND_SALDO: &str = "in einem Feld, dessen Saldo nach Rückbuchungen null oder negativ ist";

/// Der Satz neben einem Vorschlag oder bei JSON und PDF: so viele Buchungen zaehlten wegen Jahr,
/// Datum oder Saldo nicht mit. Nur Zahlen, nie ein Zweck oder eine Gegenpartei. `None`, wenn keine
/// Buchung aus diesen drei Gruenden wegfiel. Bei CSV ohne Vorschlag nennt
/// [`hinweis_nichts_uebernommen`] dieselben Gruende; die Route haengt beide nie zugleich an.
///
/// ```
/// use eingang::kontoauszug::{hinweis_nicht_gezaehlt, Uebernahme};
/// let u = Uebernahme { uebernommen: 3, llm_uebersprungen: 0, einnahmen: 0, ohne_stichwort: 0, partei: 0, feld_fehlt: 0, feld_belegt: 0, anderes_jahr: 2, datum_unlesbar: 0, saldo_nicht_positiv: 0 };
/// assert_eq!(
///     hinweis_nicht_gezaehlt(&u).as_deref(),
///     Some("Nicht gezählt: 2 Buchung(en) außerhalb des Veranlagungsjahrs.")
/// );
/// assert_eq!(hinweis_nicht_gezaehlt(&Uebernahme { anderes_jahr: 0, ..u }), None);
/// ```
#[must_use]
pub fn hinweis_nicht_gezaehlt(u: &Uebernahme) -> Option<String> {
    let teile: Vec<String> = [
        (u.anderes_jahr, GRUND_ANDERES_JAHR),
        (u.datum_unlesbar, GRUND_DATUM_UNLESBAR),
        (u.saldo_nicht_positiv, GRUND_SALDO),
    ]
    .iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, text)| format!("{n} Buchung(en) {text}"))
    .collect();
    (!teile.is_empty()).then(|| format!("Nicht gezählt: {}.", teile.join("; ")))
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
    /// Buchungen, die in die Summe eines geschriebenen Vorschlags eingingen (nicht die Zahl der
    /// Vorschlaege: zwei Maler-Rechnungen ergeben EINEN Vorschlag und zaehlen zweimal).
    pub uebernommen: usize,
    /// Buchungen, die wegen Aufruf- oder Zeitdeckel NIE klassifiziert wurden — sonst saehe ein
    /// halb angesehener Auszug aus wie ein ganz geprüfter.
    pub llm_uebersprungen: usize,
    /// Warum die uebrigen Buchungen keinen Vorschlag wurden. Zusammen mit `uebernommen` und
    /// `llm_uebersprungen` ergeben diese acht Zaehler genau die Zahl der Buchungen.
    /// Betrag 0 oder positiv: keine Ausgabe.
    pub einnahmen: usize,
    /// Weder Stichwort noch Klassifikator ordnete die Ausgabe einer Kategorie zu.
    pub ohne_stichwort: usize,
    /// Eine Spende, deren Zweck eine Partei nennt (Abweichung Nr. 39).
    pub partei: usize,
    /// Das Zielfeld fehlt in der Bindung der Scheibe.
    pub feld_fehlt: usize,
    /// Das Zielfeld hatte schon einen Wert im Store. Ein Vorschlag ersetzt ihn nie (Auflage A);
    /// auch den eigenen vorlaeufigen Vorschlag aus einem frueheren Import nicht (offen: d-63).
    pub feld_belegt: usize,
    /// Das Datum der Buchung lag ausserhalb des Veranlagungsjahrs des Stores (Bankimport S1).
    pub anderes_jahr: usize,
    /// Das Datum hatte keine der vier gelesenen Formen und wird nicht geraten (Bankimport S1).
    pub datum_unlesbar: usize,
    /// Die Rueckbuchungen glichen die Ausgaben des Felds aus oder uebertrafen sie: kein
    /// Vorschlag. Gezaehlt werden alle Buchungen dieses Felds (Bankimport S1).
    pub saldo_nicht_positiv: usize,
}

/// Alle Buchungen eines Zielfelds, die in EINE Summe eingehen. Eine Rueckbuchung (Betrag > 0)
/// derselben Kategorie steht darin mit ihrem positiven Betrag und mindert den Saldo.
struct Gruppe {
    feld: &'static str,
    kategorie: Kategorie,
    /// Alle Buchungen kamen ueber ein Stichwort, keine ueber die KI.
    nur_stichwort: bool,
    posten: Vec<Posten>,
}

/// Die Posten eines vorlaeufigen Kontoauszug-Ereignisses auf `feld`, wie `uebernehme` sie in
/// `signal_1` ablegte. `None`, wenn das Feld kein aktives Ereignis hat, das Ereignis bestaetigt
/// ist oder von einem andern Schreiber stammt (Beleg, Mensch, KI), oder wenn `signal_1` die
/// Posten nicht ganz traegt (ein Ereignis vor S1). Eine unlesbare Menge wird nie vereinigt.
///
/// Das ist der Lese-Pfad der Vereinigung: `bank::saldo::vereinige(&bestehende, &neue)` ergibt
/// den Saldo, der entstuende, wenn der neue Import das Ereignis ersetzen duerfte. Heute darf er
/// es nicht (Auflage A verbietet `ersetzt` fuer `import:kontoauszug`, Entscheidung d-63 offen);
/// dann kommt nur das Schreiben dazu, nicht die Rechnung.
///
/// ```
/// use eingang::kontoauszug::{bestehende_posten, uebernehme, Transaktion};
/// use store::{BindungNachschlag, Store};
/// let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
/// let mut store = Store::leer(2025, None);
/// assert!(bestehende_posten(&store, "spenden_betrag").is_none());
/// let tx = Transaktion { datum: serde_json::json!("2025-03-01"), betrag: -4_00, verwendungszweck: "Spende Rotes Kreuz".into() };
/// uebernehme(&mut store, &[tx], nachschlag, None, None, None).unwrap();
/// assert_eq!(bestehende_posten(&store, "spenden_betrag").unwrap().len(), 1);
/// ```
#[must_use]
pub fn bestehende_posten(store: &Store, feld: &str) -> Option<Vec<Posten>> {
    let ereignis = store.aktives(feld)?;
    if ereignis.zustand != Zustand::Vorlaeufig || ereignis.schreiber != Schreiber::ImportKontoauszug
    {
        return None;
    }
    let signal_1 = ereignis
        .signal
        .as_ref()?
        .signal_1
        .as_ref()?
        .as_ref()?
        .zu_json()
        .ok()?;
    if signal_1.get("typ")? != "kontoauszug" {
        return None;
    }
    signal_1
        .get("posten")?
        .as_array()?
        .iter()
        .map(|p| {
            Some(Posten {
                datum: Datum::aus_json(p.get("datum")?)?,
                betrag_cent: p.get("betrag_cent")?.as_i64()?,
                zweck_maskiert: p.get("zweck_maskiert")?.as_str()?.to_owned(),
            })
        })
        .collect()
}

/// Bildet den Saldo je Gruppe und schreibt, wo er positiv ist, EINEN Vorschlag. Erst alle Salden,
/// dann die Schreibungen: ein Ueberlauf der Summe bricht ab, bevor ein Vorschlag steht.
fn schreibe_gruppen(
    store: &mut Store,
    gruppen: Vec<Gruppe>,
    jahr: i64,
    (bindung, katalog, ts): (BindungNachschlag<'_>, &Katalog, Option<&str>),
    u: &mut Uebernahme,
) -> Result<(), KontoauszugFehler> {
    let mut zu_schreiben = Vec::new();
    for g in gruppen {
        let saldo = saldo::saldo(&g.posten)
            .ok_or_else(|| KontoauszugFehler::BetragUeberlauf(format!("Summe {}", g.feld)))?;
        if saldo <= 0 {
            u.saldo_nicht_positiv += g.posten.len();
        } else {
            zu_schreiben.push((g, saldo));
        }
    }
    for (g, saldo) in zu_schreiben {
        let posten: Vec<Value> = g
            .posten
            .iter()
            .map(|p| {
                json!({"datum": p.datum.als_text(), "betrag_cent": p.betrag_cent,
                    "zweck_maskiert": p.zweck_maskiert})
            })
            .collect();
        let signal_1 = json!({"typ": "kontoauszug", "jahr": jahr, "posten": posten,
            "summe_cent": saldo, "kategorie": g.kategorie.als_str(),
            "quelle": if g.nur_stichwort { "heuristik" } else { "llm" }});
        let v = VorschlagEvent {
            quelle: Quelle::Kontoauszug,
            feld_id: g.feld.to_owned(),
            wert: json!(saldo),
            signal_1,
        };
        v.schreibe(store, Some(katalog), bindung, ts)?;
        u.uebernommen += g.posten.len();
    }
    Ok(())
}

/// `uebernehme_kontoauszug(store, transaktionen, bindung, llm_klassifikator=, ts=, katalog=)`.
///
/// `bindung` = Bindung der Scheibe (Zielfeld muss darin stehen); `katalog` = globaler Katalog,
/// fehlt er, wird er aus `bindung` gebaut.
///
/// Je Zielfeld entsteht EIN vorlaeufiger Vorschlag (Bankimport S1, Abweichung von Python, das
/// die erste Buchung nimmt und kein Jahr prueft): der Wert ist die Summe der Ausgaben des
/// Veranlagungsjahrs in Cent, abzueglich der Rueckbuchungen derselben Kategorie (Betrag > 0 mit
/// Stichwort); `signal_1` traegt die Einzelposten. Ein Datum ausserhalb des Jahres oder ohne eine
/// der vier gelesenen Formen laesst die Buchung weg und zaehlt sie. Ist der Saldo null oder
/// negativ, entsteht kein Vorschlag. Hatte das Feld schon einen Wert, auch den eines frueheren
/// Imports, schreibt dieser Aufruf nichts (Auflage A, `feld_belegt`).
///
/// Eine Buchung der Kategorie Spende, deren Zweck einen Parteinamen nennt (`PARTEINAMEN`), bekommt KEINEN Vorschlag
/// (Abweichung Nr. 39, `rust/fixtures/README.md`): Python schriebe sie nach `spenden_betrag`. Das gilt auch fuer
/// ihre Rueckbuchung, sonst minderte sie die Summe fremder Spenden.
///
/// # Errors
/// [`KontoauszugFehler`] (Store-Abweisung, Betragsueberlauf, auch der der Summe).
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
    let jahr = store.veranlagungszeitraum();
    let mut aufrufe = 0;
    let mut u = Uebernahme {
        uebernommen: 0,
        llm_uebersprungen: 0,
        einnahmen: 0,
        ohne_stichwort: 0,
        partei: 0,
        feld_fehlt: 0,
        feld_belegt: 0,
        anderes_jahr: 0,
        datum_unlesbar: 0,
        saldo_nicht_positiv: 0,
    };
    let beginn = Instant::now();
    // Zuerst ordnen, dann schreiben: ein Ueberlauf der Summe bricht ab, bevor ein Vorschlag steht.
    let mut gruppen: Vec<Gruppe> = Vec::new();
    for tx in transaktionen {
        // Betrag 0 ist weder Ausgabe noch Rueckbuchung.
        if tx.betrag == 0 {
            u.einnahmen += 1;
            continue;
        }
        let zweck = &tx.verwendungszweck;
        let det = klassifiziere_det(zweck);
        // Ein positiver Betrag zaehlt nur mit Stichwort als Rueckbuchung; sonst ist er eine Einnahme.
        // Die KI sieht ihn nie.
        if tx.betrag > 0 && det.is_none() {
            u.einnahmen += 1;
            continue;
        }
        // Das Datum vor der KI: eine Buchung ausserhalb des Jahres kostet keinen Aufruf.
        let datum = match Datum::aus_json(&tx.datum) {
            None => {
                u.datum_unlesbar += 1;
                continue;
            }
            Some(d) if d.jahr != jahr => {
                u.anderes_jahr += 1;
                continue;
            }
            Some(d) => d,
        };
        let mut kategorie = det;
        if kategorie.is_none() {
            if let Some(k) = llm_klassifikator {
                if aufrufe >= LLM_AUFRUFE_HOECHSTZAHL || beginn.elapsed() > LLM_ZEITBUDGET {
                    u.llm_uebersprungen += 1;
                    continue;
                }
                aufrufe += 1;
                kategorie = k(&pii::maskiere(zweck), tx.betrag);
            }
        }
        let Some(kategorie) = kategorie else {
            u.ohne_stichwort += 1;
            continue;
        };
        // GEWOLLT ABWEICHEND von Python (Abweichung Nr. 39): Eine Spende, deren Zweck eine Partei nennt, gehoert nicht
        // in `spenden_betrag`. Die Pruefung steht hinter beiden Wegen der Kategorisierung (Schluesselwort und LLM).
        if kategorie == Kategorie::Spende && nennt_partei(zweck) {
            u.partei += 1;
            continue;
        }
        let feld = zielfeld(kategorie);
        if bindung.get(feld).is_none() {
            u.feld_fehlt += 1;
            continue;
        }
        if store.aktives(feld).is_some() {
            u.feld_belegt += 1;
            continue;
        }
        let posten = Posten {
            datum,
            betrag_cent: tx.betrag,
            zweck_maskiert: pii::maskiere(zweck).as_str().to_owned(),
        };
        if let Some(g) = gruppen.iter_mut().find(|g| g.feld == feld) {
            g.nur_stichwort &= det.is_some();
            g.posten.push(posten);
        } else {
            gruppen.push(Gruppe {
                feld,
                kategorie,
                nur_stichwort: det.is_some(),
                posten: vec![posten],
            });
        }
    }
    schreibe_gruppen(store, gruppen, jahr, (bindung, katalog, ts), &mut u)?;
    Ok(u)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{
        aus_json, eur_cent_signed, hinweis_nichts_gelesen, hinweis_nichts_uebernommen,
        hinweis_verworfen, parse_csv, parse_csv_befund, uebernehme, KontoauszugFehler, Transaktion,
        Uebernahme,
    };

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

    /// Bankimport S0: der Befund nennt, wo die Kopfzeile steht, mit welchem Trenner gelesen wurde und
    /// wie viele Zeilen keinen Betrag trugen.
    #[test]
    fn befund_nennt_kopfzeile_trenner_und_zeilen_ohne_betrag() {
        let befund = |text: &str| parse_csv_befund(text).unwrap().2;
        let b = befund("Vorspann\n\nBuchungstag;Betrag;Zweck\n01.03.2025;-1,00;A\n02.03.2025;;B\n");
        assert_eq!((b.trenner, b.kopf_nr, b.ohne_betrag), (';', Some(2), 1));
        assert_eq!(b.spalten, ["Buchungstag", "Betrag", "Zweck"]);
        let b = befund("Datum,Betrag (EUR),Verwendungszweck\n2025-03-15,\"-4.99\",A\n");
        assert_eq!((b.trenner, b.kopf_nr), (',', Some(0)));
        let b = befund("Datum\tAmount\n2025-03-15\t-4.99\n");
        assert_eq!((b.trenner, b.kopf_nr), ('\t', Some(0)));
        // Ohne Betragsspalte: Rueckfall auf `;` und den ersten Datensatz, die Namen kommen vom Kopf-Kandidaten.
        let b = befund("Konto;1\nBuchungstag;Wert\n01.03.2025;5\n");
        assert_eq!((b.trenner, b.kopf_nr, b.ohne_betrag), (';', None, 0));
        assert_eq!(b.spalten, ["Buchungstag", "Wert"]);
        // Gar nichts, das nach einer Kopfzeile aussieht: keine Namen (kein Inhalt einer Zeile in die Antwort).
        let b = befund("a;b\n1;2\n");
        assert!(b.spalten.is_empty(), "{:?}", b.spalten);
    }

    /// Bankimport S0: die Saetze fuer den Nutzer. Der Hinweis ohne Betragsspalte kuerzt auf 20 Namen
    /// zu je 40 Zeichen; mit Kopfzeile nennt er, was darunter fehlt.
    #[test]
    fn hinweis_nichts_gelesen_nennt_namen_gekuerzt_und_den_grund_darunter() {
        let lang = "x".repeat(60);
        let namen: Vec<String> = std::iter::once("Datum".to_owned())
            .chain((1..=24).map(|i| format!("S{i}")))
            .chain(std::iter::once(lang.clone()))
            .collect();
        let (_, _, b) = parse_csv_befund(&format!("{}\n1;2\n", namen.join(";"))).unwrap();
        let h = hinweis_nichts_gelesen(&b);
        let genannt = h.rsplit_once("gefundene Spalten: ").unwrap().1;
        assert_eq!(genannt.trim_end_matches('.').split(", ").count(), 20, "{h}");
        assert!(!h.contains(&lang), "{h}");
        let (_, _, b) = parse_csv_befund(&format!("Datum;{lang};Betrag\n")).unwrap();
        assert_eq!(
            hinweis_nichts_gelesen(&b),
            "Keine Buchung gelesen: Unter der Kopfzeile folgt keine Datenzeile."
        );
        let (_, _, b) = parse_csv_befund("Datum;Betrag\n01.03.2025;\n").unwrap();
        assert_eq!(
            hinweis_nichts_gelesen(&b),
            "Keine Buchung gelesen: 1 Zeile(n) unter der Kopfzeile haben keinen Betrag."
        );
    }

    /// Bankimport S0/S1: jede Buchung faellt in genau einen Zaehler. Ohne diese Summe waere der Hinweis
    /// „kein Vorschlag, weil …“ eine Behauptung ueber eine Teilmenge. Jeder der zehn Gruende kommt
    /// mindestens einmal vor (`llm_uebersprungen` im Deckeltest der Naht).
    #[test]
    fn jede_buchung_zaehlt_in_genau_einem_grund() {
        let tx = |datum: &str, betrag: i64, zweck: &str| Transaktion {
            datum: serde_json::json!(datum),
            betrag,
            verwendungszweck: zweck.to_owned(),
        };
        // Eine Bindung ohne `spenden_betrag`: die Spende an das Tierheim hat kein Zielfeld.
        let mut karte = crate::doctest_bindung().unwrap().clone();
        karte.remove("spenden_betrag");
        let bindung = store::BindungNachschlag::neu(&karte);
        let mut store = store::Store::leer(2025, None);
        // Das Feld der Reinigung hat schon einen Wert aus einem frueheren Import.
        uebernehme(
            &mut store,
            &[tx("01.02.2025", -4_00, "Reinigung Treppenhaus")],
            bindung,
            None,
            None,
            None,
        )
        .unwrap();
        let buchungen = [
            tx("01.03.2025", 10_000, "Gehalt"),
            tx("01.03.2025", -999, "Einkauf"),
            tx("01.03.2025", -5_000, "Spende SPD Ortsverband"),
            tx("01.03.2025", -5_000, "Spende Tierheim"),
            tx("01.03.2025", -10_000, "Maler Huber"),
            tx("02.03.2025", -20_000, "Maler Meier"),
            tx("31.12.2024", -3_000, "Maler Vorjahr"),
            tx("15.03", -3_000, "Maler ohne Jahr"),
            tx("01.03.2025", -5_000, "Minijob Anna"),
            tx("02.03.2025", 5_000, "Rueckbuchung Minijob Anna"),
            tx("01.03.2025", -6_000, "Reinigung Treppenhaus"),
        ];
        let u = uebernehme(&mut store, &buchungen, bindung, None, None, None).unwrap();
        let soll = Uebernahme {
            uebernommen: 2,
            llm_uebersprungen: 0,
            einnahmen: 1,
            ohne_stichwort: 1,
            partei: 1,
            feld_fehlt: 1,
            feld_belegt: 1,
            anderes_jahr: 1,
            datum_unlesbar: 1,
            saldo_nicht_positiv: 2,
        };
        assert_eq!(u, soll);
        let summe = u.uebernommen
            + u.llm_uebersprungen
            + u.einnahmen
            + u.ohne_stichwort
            + u.partei
            + u.feld_fehlt
            + u.feld_belegt
            + u.anderes_jahr
            + u.datum_unlesbar
            + u.saldo_nicht_positiv;
        assert_eq!(summe, buchungen.len());
        assert_eq!(
            store
                .aktives("hh_handwerker_betrag")
                .and_then(|e| e.wert.zu_json().ok())
                .and_then(|v| v.as_i64()),
            Some(30_000),
            "Maler Huber und Maler Meier ergeben EINEN Vorschlag"
        );
        assert!(
            store.aktives("hh_minijob_betrag").is_none(),
            "Ausgabe und Rueckbuchung gleichen sich aus: kein Vorschlag"
        );
        // Ein Auszug, der nur den Deckel trifft, hat keinen Grund zu nennen (die Route meldet den Deckel selbst).
        let nur_deckel = Uebernahme {
            llm_uebersprungen: 2,
            uebernommen: 0,
            einnahmen: 0,
            ohne_stichwort: 0,
            partei: 0,
            feld_fehlt: 0,
            feld_belegt: 0,
            anderes_jahr: 0,
            datum_unlesbar: 0,
            saldo_nicht_positiv: 0,
        };
        assert_eq!(hinweis_nichts_uebernommen(2, &nur_deckel), None);
    }

    // ---------------------------------------------------------------------------
    // Bankimport S1: Zahlungsjahr, Summe je Feld, Gutschrift mindert, Posten im Ereignis.
    // Diese Tests nutzen nur die heute vorhandene API; auf der Basis `d045d5be` sind sie ROT,
    // weil die erste Buchung gewinnt, kein Jahr gepraueft wird und das Ereignis nur einen
    // Einzelposten traegt.
    // ---------------------------------------------------------------------------

    fn s1_tx(ja: &[(&str, i64, &str)]) -> Vec<Transaktion> {
        ja.iter()
            .map(|(d, b, z)| Transaktion {
                datum: serde_json::json!(d),
                betrag: *b,
                verwendungszweck: (*z).to_owned(),
            })
            .collect()
    }

    /// Uebernahme in einen frischen Fall 2025: Feldwert, Zahl uebernommener Posten, Zahl der
    /// Ereignisse im Log.
    fn s1_lauf(ja: &[(&str, i64, &str)], feld: &str) -> (Option<i64>, usize, usize) {
        let tx = s1_tx(ja);
        let mut store = store::Store::leer(2025, None);
        let erg = uebernehme(
            &mut store,
            &tx,
            store::BindungNachschlag::neu(crate::doctest_bindung().unwrap()),
            None,
            None,
            None,
        )
        .unwrap();
        let wert = store
            .aktives(feld)
            .and_then(|e| e.wert.zu_json().ok())
            .and_then(|v| v.as_i64());
        (wert, erg.uebernommen, store.events().len())
    }

    /// Das Pruefkommando des Backlogeintrags `bankimport-s1`: Summe zweier Spenden,
    /// Zahlungsjahr, Rueckbuchung.
    #[test]
    fn p1_rot_summe_jahr_gutschrift() {
        let (a, _, _) = s1_lauf(
            &[
                ("2025-02-01", -10_000, "Spende A"),
                ("2025-03-01", -25_000, "Spende B"),
            ],
            "spenden_betrag",
        );
        let (c, _, _) = s1_lauf(
            &[
                ("2024-12-30", -5_000, "Spende A"),
                ("2025-02-01", -7_000, "Spende B"),
            ],
            "spenden_betrag",
        );
        let (d, _, _) = s1_lauf(
            &[
                ("2025-02-01", -10_000, "Spende A"),
                ("2025-02-10", 10_000, "Rueckbuchung Spende A"),
            ],
            "spenden_betrag",
        );
        let mut rot = Vec::new();
        if a != Some(35_000) {
            rot.push(format!("A Summe: {a:?}, erwartet Some(35000)"));
        }
        if c != Some(7_000) {
            rot.push(format!("C Zahlungsjahr: {c:?}, erwartet Some(7000)"));
        }
        if d.is_some() {
            rot.push(format!("D Rueckbuchung: {d:?}, erwartet None (Saldo 0)"));
        }
        assert!(rot.is_empty(), "P1 ROT: {rot:?}");
    }

    /// Zwei Ausgaben derselben Kategorie ergeben EINE Summe in EINEM Ereignis; beide Posten
    /// zaehlen als uebernommen. Heute: die erste Buchung schreibt, die zweite gilt als belegt.
    #[test]
    fn s1_rot_zwei_ausgaben_eines_felds_ergeben_eine_summe() {
        let (wert, uebernommen, ereignisse) = s1_lauf(
            &[
                ("01.03.2025", -10_000, "Maler Huber"),
                ("02.03.2025", -20_000, "Maler Meier"),
            ],
            "hh_handwerker_betrag",
        );
        assert_eq!((wert, uebernommen, ereignisse), (Some(30_000), 2, 1));
    }

    /// Eine Gutschrift MIT Stichwort mindert die Summe desselben Felds: 100 + 250 - 50 = 300 EUR.
    /// Heute gilt jede Gutschrift als Einnahme und aendert nichts.
    #[test]
    fn s1_rot_eine_rueckerstattung_mindert_die_summe() {
        let (wert, _, _) = s1_lauf(
            &[
                ("2025-02-01", -10_000, "Spende Rotes Kreuz"),
                ("2025-02-05", -25_000, "Spende Tierheim"),
                ("2025-02-10", 5_000, "Rueckbuchung Spende Tierheim"),
            ],
            "spenden_betrag",
        );
        assert_eq!(wert, Some(30_000));
    }

    /// Nur Zahlungen des Veranlagungsjahrs zaehlen: ein Posten aus 2024 vor einer Spende von
    /// 2025 schreibt heute 5,00 EUR, nach dem Bau 7,00 EUR. Ein Datum ohne Jahr (`15.03`) wird
    /// nicht geraten und schreibt heute eine Ausgabe, die keinem Jahr zuzuordnen ist.
    #[test]
    fn s1_rot_zahlungsjahr_und_datum_ohne_jahr() {
        let (c, _, _) = s1_lauf(
            &[
                ("2024-12-30", -5_000, "Spende Rotes Kreuz"),
                ("2025-02-01", -7_000, "Spende Tierheim"),
            ],
            "spenden_betrag",
        );
        assert_eq!(c, Some(7_000), "nur der Posten von 2025 zaehlt");
        let (_, _, ohne_jahr) = s1_lauf(&[("15.03", -6_000, "Spende Rotes Kreuz")], "spenden_betrag");
        assert_eq!(ohne_jahr, 0, "ein Datum ohne Jahr wird nicht geraten");
    }

    /// Gelesene Datumsformen: `15.03.25` (zweistelliges Jahr am Steuerjahr) und ein Datum mit
    /// Uhrzeit zaehlen beide als 2025 und werden nicht verworfen.
    #[test]
    fn s1_rot_zweistelliges_jahr_und_uhrzeit_werden_gelesen() {
        let tx = s1_tx(&[
            ("15.03.25", -20_00, "Maler Huber"),
            ("2025-03-15 10:00:05", -25_00, "Basisrente Rürup"),
        ]);
        let mut store = store::Store::leer(2025, None);
        let erg = uebernehme(
            &mut store,
            &tx,
            store::BindungNachschlag::neu(crate::doctest_bindung().unwrap()),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(erg.feld_belegt, 0, "{erg:?}");
        assert_eq!(erg.einnahmen, 0, "keine der beiden Buchungen ist eine Einnahme");
        assert_eq!(
            store.aktives("hh_handwerker_betrag").and_then(|e| e.wert.zu_json().ok()).and_then(|v| v.as_i64()),
            Some(20_00),
            "dd.mm.yy wird 2000 + yy"
        );
        assert_eq!(
            store.aktives("vor_rv_ausserhalb_lstb").and_then(|e| e.wert.zu_json().ok()).and_then(|v| v.as_i64()),
            Some(25_00),
            "ein Datum mit Uhrzeit wird gelesen"
        );
    }

    /// Das Zahlungsjahr ist das Veranlagungsjahr des STORES, nicht ein festes 2025: im Fall 2024
    /// zaehlt der Posten von 2024, und der von 2025 ist `anderes_jahr`. Mutante N11 von main
    /// (`let jahr = 2025;`) ueberlebte, weil jeder Test einen Fall 2025 anlegte.
    #[test]
    fn s1_zahlungsjahr_ist_das_veranlagungsjahr_des_stores() {
        let tx = s1_tx(&[
            ("2024-12-30", -5_000, "Spende Rotes Kreuz"),
            ("2025-02-01", -7_000, "Spende Tierheim"),
        ]);
        let mut store = store::Store::leer(2024, None);
        let erg = uebernehme(
            &mut store,
            &tx,
            store::BindungNachschlag::neu(crate::doctest_bindung().unwrap()),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!((erg.uebernommen, erg.anderes_jahr), (1, 1), "{erg:?}");
        assert_eq!(
            store.aktives("spenden_betrag").and_then(|e| e.wert.zu_json().ok()).and_then(|v| v.as_i64()),
            Some(5_000),
            "im Fall 2024 zaehlt nur der Posten von 2024"
        );
    }

    /// Zwei gleiche Buchungen in EINEM Auszug sind zwei Buchungen und zaehlen zweimal: eine
    /// Bank bucht nie dieselbe Zeile doppelt. Die Mehrmenge kuerzt erst beim Vereinigen zweier
    /// Auszuege (`bank::saldo::vereinige`, Maximum je Schluessel), nicht innerhalb eines Auszugs.
    /// Die Erwartung dieses Tests lautete im ersten Commit 10,00 EUR ("die Dublette zaehlt einmal");
    /// das hiess, 4,00 EUR einer echten zweiten Buchung still zu verlieren.
    #[test]
    fn s1_rot_gleiche_buchungen_in_einem_auszug_zaehlen_je_buchung() {
        let (wert, uebernommen, _) = s1_lauf(
            &[
                ("2025-02-01", -4_00, "Spende Rotes Kreuz"),
                ("2025-02-01", -4_00, "Spende Rotes Kreuz"),
                ("2025-02-05", -6_00, "Spende Tierheim"),
            ],
            "spenden_betrag",
        );
        assert_eq!(wert, Some(14_00), "4 plus 4 plus 6");
        assert_eq!(uebernommen, 3);
    }

    /// Das Ereignis traegt die Einzelposten in `signal_1`: `datum`, `betrag_cent` mit
    /// Vorzeichen, `zweck_maskiert`; dazu `jahr` und `summe_cent`. Der Feldwert ist der Saldo
    /// in Cent, ohne Vorzeichen. Heute traegt das Ereignis nur die ERSTE Buchung.
    #[test]
    fn s1_rot_das_ereignis_traegt_posten_und_saldo() {
        let tx = s1_tx(&[
            ("01.03.2025", -4_00, "Spende Rotes Kreuz"),
            ("02.03.2025", -7_00, "Spende Tierheim"),
        ]);
        let mut store = store::Store::leer(2025, None);
        uebernehme(
            &mut store,
            &tx,
            store::BindungNachschlag::neu(crate::doctest_bindung().unwrap()),
            None,
            None,
            None,
        )
        .unwrap();
        let ev = store.aktives("spenden_betrag").expect("kein Ereignis auf spenden_betrag");
        let s1 = ev
            .signal
            .as_ref()
            .and_then(|s| s.signal_1.as_ref())
            .and_then(|innen| innen.as_ref())
            .and_then(|v| v.zu_json().ok())
            .unwrap_or_default();
        let posten: Vec<&serde_json::Value> = s1
            .get("posten")
            .and_then(serde_json::Value::as_array)
            .map_or(vec![], |p| p.iter().collect());
        assert_eq!(s1["typ"], "kontoauszug");
        assert_eq!(
            s1.get("jahr").and_then(serde_json::Value::as_i64),
            Some(2025),
            "{s1}"
        );
        assert_eq!(
            s1.get("summe_cent").and_then(serde_json::Value::as_i64),
            Some(11_00),
            "{s1}"
        );
        assert_eq!(posten.len(), 2, "beide Posten im Ereignis: {s1}");
        assert_eq!(
            posten
                .first()
                .and_then(|p| p.get("betrag_cent"))
                .and_then(serde_json::Value::as_i64),
            Some(-4_00),
            "{s1}"
        );
        assert_eq!(ev.wert.zu_json().ok().and_then(|v| v.as_i64()), Some(11_00));
    }

    /// Waechter: jedes Zielfeld von `zielfeld` laesst den Kontoauszug als Schreiber zu. Eine
    /// Store-Abweisung bricht die ganze Uebernahme ab (das `?` in `uebernehme`), und nur diese
    /// Bindungszeile entscheidet, ob sie ueberhaupt erreichbar ist.
    #[test]
    fn jedes_zielfeld_ist_fuer_den_kontoauszug_vorschlagbar() {
        let nachschlag = store::BindungNachschlag::neu(crate::doctest_bindung().unwrap());
        let mut falsch = Vec::new();
        for k in [
            llm::Kategorie::Handwerker,
            llm::Kategorie::Dienstleistung,
            llm::Kategorie::Minijob,
            llm::Kategorie::Spende,
            llm::Kategorie::Vorsorge,
        ] {
            let feld = super::zielfeld(k);
            match nachschlag.get(feld) {
                Some(b) => {
                    if !b.vorschlagbar_von.contains(&bindung::VorschlagsSchreiber::Kontoauszug) {
                        falsch.push(format!("{feld}: vorschlagbar_von = {:?}", b.vorschlagbar_von));
                    }
                }
                None => falsch.push(format!("{feld}: fehlt in der Bindung")),
            }
        }
        assert!(falsch.is_empty(), "{falsch:?}");
    }

    // ---------------------------------------------------------------------------
    // Bankimport S1, nach dem Bau: Lese-Pfad der Vereinigung, Gutschriften, Ueberlauf der Summe,
    // wer die KI sieht, und die Saetze fuer den Nutzer.
    // ---------------------------------------------------------------------------

    use serde_json::json;

    use super::{bestehende_posten, hinweis_nicht_gezaehlt};
    use crate::bank::{buchung::Posten, saldo};

    fn s1_store_mit(ja: &[(&str, i64, &str)]) -> store::Store {
        let mut store = store::Store::leer(2025, None);
        uebernehme(
            &mut store,
            &s1_tx(ja),
            store::BindungNachschlag::neu(crate::doctest_bindung().unwrap()),
            None,
            Some("2025-03-01T00:00:00+00:00"),
            None,
        )
        .unwrap();
        store
    }

    fn s1_wert(store: &store::Store, feld: &str) -> Option<i64> {
        store
            .aktives(feld)
            .and_then(|e| e.wert.zu_json().ok())
            .and_then(|v| v.as_i64())
    }

    /// Der Lese-Pfad der Vereinigung (Auflage A verbietet das Ersetzen, d-63): die Posten des
    /// bestehenden Ereignisses plus die eines zweiten Auszugs ergeben EINE Summe, und dieselbe
    /// Buchung in beiden zaehlt einmal. Der Import selbst schreibt nichts und zaehlt `feld_belegt`.
    #[test]
    fn s1_vereinigung_ueber_den_lese_pfad() {
        let erster = [
            ("2025-02-01", -4_00, "Spende Rotes Kreuz"),
            ("2025-02-02", -7_00, "Spende Tierheim"),
        ];
        let zweiter = [
            ("2025-02-02", -7_00, "Spende Tierheim"),
            ("2025-02-03", -5_00, "Spende Unicef"),
        ];
        let mut store = s1_store_mit(&erster);
        let bestehend = bestehende_posten(&store, "spenden_betrag").unwrap();
        assert_eq!(bestehend.len(), 2);
        assert_eq!(saldo::saldo(&bestehend), Some(11_00));
        // Die Posten des zweiten Auszugs, vom selben Code erzeugt wie die des ersten.
        let neu = bestehende_posten(&s1_store_mit(&zweiter), "spenden_betrag").unwrap();
        let zusammen = saldo::vereinige(&bestehend, &neu);
        assert_eq!(zusammen.len(), 3, "der gemeinsame Posten zaehlt einmal");
        assert_eq!(saldo::saldo(&zusammen), Some(16_00), "4 + 7 + 5");
        // Dieselbe Datei noch einmal: derselbe Saldo, nicht der doppelte.
        assert_eq!(saldo::saldo(&saldo::vereinige(&bestehend, &bestehend)), Some(11_00));
        // Der Import selbst: Auflage A haelt ihn auf, der Zaehler nennt es, der Wert bleibt.
        let erg = uebernehme(
            &mut store,
            &s1_tx(&zweiter),
            store::BindungNachschlag::neu(crate::doctest_bindung().unwrap()),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!((erg.uebernommen, erg.feld_belegt), (0, 2), "{erg:?}");
        assert_eq!(store.events().len(), 1);
        assert_eq!(s1_wert(&store, "spenden_betrag"), Some(11_00));
    }

    /// Die Vereinigung ueberliest kein bestaetigtes Ereignis und keines eines andern Schreibers.
    #[test]
    fn s1_bestehende_posten_nur_vorlaeufig_aus_dem_kontoauszug_in_lesbarer_form() {
        let bindung = store::BindungNachschlag::neu(crate::doctest_bindung().unwrap());
        let katalog = store::Katalog::aus_bindungen(bindung.alle().map(|(_, b)| b));
        // Ein Ereignis aus dem Kontoauszug im Format VOR S1 (ohne `posten`): nicht lesbar.
        let mut alt = store::Store::leer(2025, None);
        crate::vorschlag::VorschlagEvent {
            quelle: crate::vorschlag::Quelle::Kontoauszug,
            feld_id: "spenden_betrag".to_owned(),
            wert: json!(500),
            signal_1: json!({"typ": "kontoauszug", "datum": "01.03.2025", "betrag": -500,
                "verwendungszweck": "Spende", "kategorie": "spende", "quelle": "heuristik"}),
        }
        .schreibe(&mut alt, Some(&katalog), bindung, None)
        .unwrap();
        assert!(bestehende_posten(&alt, "spenden_betrag").is_none());
        // Ein Posten mit einem Datum, das der Leser nicht kennt: die Menge wird nie halb vereinigt.
        let mut halb = store::Store::leer(2025, None);
        crate::vorschlag::VorschlagEvent {
            quelle: crate::vorschlag::Quelle::Kontoauszug,
            feld_id: "spenden_betrag".to_owned(),
            wert: json!(500),
            signal_1: json!({"typ": "kontoauszug", "posten": [
                {"datum": "2025-03-01", "betrag_cent": -250, "zweck_maskiert": "Spende"},
                {"datum": "kein Datum", "betrag_cent": -250, "zweck_maskiert": "Spende"}]}),
        }
        .schreibe(&mut halb, Some(&katalog), bindung, None)
        .unwrap();
        assert!(bestehende_posten(&halb, "spenden_betrag").is_none());
        // Dasselbe Ereignis mit lesbaren Posten, aber vom BELEG geschrieben: nicht vereinigt.
        let mut beleg = store::Store::leer(2025, None);
        crate::vorschlag::VorschlagEvent {
            quelle: crate::vorschlag::Quelle::Beleg,
            feld_id: "spenden_betrag".to_owned(),
            wert: json!(250),
            signal_1: json!({"typ": "kontoauszug", "posten": [
                {"datum": "2025-03-01", "betrag_cent": -250, "zweck_maskiert": "Spende"}]}),
        }
        .schreibe(&mut beleg, Some(&katalog), bindung, None)
        .unwrap();
        assert!(bestehende_posten(&beleg, "spenden_betrag").is_none());
        // Ein BESTAETIGTES Ereignis eines Menschen mit denselben Posten: nicht vereinigt.
        let mut mensch = store::Store::leer(2025, None);
        let bestaetigt = store::NeuesEvent {
            feld_id: "spenden_betrag".to_owned(),
            wert: json!(250).into(),
            feldzustand: domain::Feldzustand::Bestaetigt {
                signal_2: domain::Signal2::new("klick").unwrap(),
            },
            herkunft: domain::Herkunft {
                herkunft: domain::Achsenwert::new("nutzer").unwrap(),
                pruef_tiefe: domain::PruefTiefe::Ungeprueft,
                haftung: domain::Achsenwert::new("nutzer").unwrap(),
            },
            schreiber: domain::Schreiber::Mensch("julius".to_owned()),
            signal_1: Some(
                json!({"typ": "kontoauszug", "posten": [
                    {"datum": "2025-03-01", "betrag_cent": -250, "zweck_maskiert": "Spende"}]})
                .into(),
            ),
            ersetzt: None,
            ts: Some("2025-03-01T00:00:00+00:00".to_owned()),
        };
        mensch.append(&bestaetigt, Some(&katalog), bindung).unwrap();
        assert!(bestehende_posten(&mensch, "spenden_betrag").is_none());
        // Ein BESTAETIGTES Ereignis des Schreibers `import:kontoauszug`: `append` weist es ab
        // (Auflage A), eine geladene Akte haelt es trotzdem. Es wird nicht vereinigt.
        let mut datei = {
            let mut vorlaeufig = store::Store::leer(2025, None);
            crate::vorschlag::VorschlagEvent {
                quelle: crate::vorschlag::Quelle::Kontoauszug,
                feld_id: "spenden_betrag".to_owned(),
                wert: json!(250),
                signal_1: json!({"typ": "kontoauszug", "posten": [
                    {"datum": "2025-03-01", "betrag_cent": -250, "zweck_maskiert": "Spende"}]}),
            }
            .schreibe(&mut vorlaeufig, Some(&katalog), bindung, None)
            .unwrap();
            vorlaeufig.into_datei()
        };
        let geladen = store::Store::aus_datei(datei.clone());
        assert_eq!(bestehende_posten(&geladen, "spenden_betrag").map(|p| p.len()), Some(1));
        datei.events[0].zustand = domain::Zustand::Bestaetigt;
        let bestaetigt_geladen = store::Store::aus_datei(datei);
        assert!(bestehende_posten(&bestaetigt_geladen, "spenden_betrag").is_none());
        // Kein aktives Ereignis: nichts zu vereinigen.
        assert!(bestehende_posten(&store::Store::leer(2025, None), "spenden_betrag").is_none());
    }

    /// Rueckbuchungen: ein Saldo von null oder weniger schreibt nichts, und die Rueckbuchung einer
    /// Parteispende mindert die Summe fremder Spenden nicht.
    #[test]
    fn s1_gutschriften_mindern_nur_ihr_eigenes_feld() {
        let lauf = |ja: &[(&str, i64, &str)]| {
            let mut store = store::Store::leer(2025, None);
            let erg = uebernehme(
                &mut store,
                &s1_tx(ja),
                store::BindungNachschlag::neu(crate::doctest_bindung().unwrap()),
                None,
                None,
                None,
            )
            .unwrap();
            (erg, s1_wert(&store, "spenden_betrag"))
        };
        // Nur eine Rueckbuchung: kein Vorschlag, gezaehlt als Saldo.
        let (erg, wert) = lauf(&[("2025-02-10", 10_000, "Erstattung Spende")]);
        assert_eq!((wert, erg.saldo_nicht_positiv, erg.einnahmen), (None, 1, 0), "{erg:?}");
        // Die Rueckbuchung uebertrifft die Ausgabe.
        let (erg, wert) = lauf(&[
            ("2025-02-01", -10_000, "Spende Rotes Kreuz"),
            ("2025-02-10", 15_000, "Erstattung Spende"),
        ]);
        assert_eq!((wert, erg.saldo_nicht_positiv, erg.uebernommen), (None, 2, 0), "{erg:?}");
        // Eine Parteispende und ihre Rueckbuchung: beide `partei`, die anderen Spenden bleiben unberuehrt.
        let (erg, wert) = lauf(&[
            ("2025-02-01", -5_000, "Spende Tierheim"),
            ("2025-02-02", -9_000, "Spende SPD Ortsverband"),
            ("2025-02-03", 9_000, "Rueckbuchung Spende SPD Ortsverband"),
        ]);
        assert_eq!((wert, erg.partei, erg.uebernommen), (Some(5_000), 2, 1), "{erg:?}");
        // Betrag 0 mit Stichwort ist weder Ausgabe noch Rueckbuchung.
        let (erg, wert) = lauf(&[("2025-02-01", 0, "Spende Rotes Kreuz")]);
        assert_eq!((wert, erg.einnahmen), (None, 1), "{erg:?}");
        // Eine Rueckbuchung in einem ANDERN Jahr zaehlt als dieses Jahr und mindert nichts.
        let (erg, wert) = lauf(&[
            ("2025-02-01", -10_000, "Spende Rotes Kreuz"),
            ("2024-12-31", 10_000, "Rueckbuchung Spende"),
        ]);
        assert_eq!((wert, erg.anderes_jahr), (Some(10_000), 1), "{erg:?}");
    }

    /// Der Ueberlauf der SUMME ist ein Fehler, kein Panic und kein Wert, und er bricht ab, BEVOR
    /// ein Vorschlag steht (auch der des andern Felds, der allein in Ordnung gewesen waere).
    /// Ueber die Route ist er nicht zu erreichen (jeder Betrag liegt dort unter 10^10 Cent).
    #[test]
    fn s1_ueberlauf_der_summe_bricht_ab_bevor_ein_vorschlag_steht() {
        let mut store = store::Store::leer(2025, None);
        let erg = uebernehme(
            &mut store,
            &s1_tx(&[
                ("2025-02-01", -100, "Maler Huber"),
                ("2025-02-02", -(i64::MAX - 100), "Spende Rotes Kreuz"),
                ("2025-02-03", -200, "Spende Tierheim"),
            ]),
            store::BindungNachschlag::neu(crate::doctest_bindung().unwrap()),
            None,
            None,
            None,
        );
        assert!(
            matches!(erg, Err(KontoauszugFehler::BetragUeberlauf(_))),
            "BetragUeberlauf erwartet, gekommen {erg:?}"
        );
        assert_eq!(store.events().len(), 0, "auch der Maler steht nicht in der Akte");
    }

    /// Die KI sieht keine Rueckbuchung und keine Buchung ausserhalb des Jahres (das Datum steht
    /// vor der KI); eine Summe aus Stichwort UND KI traegt die Quelle `llm`.
    #[test]
    fn s1_ki_sieht_nur_ausgaben_des_jahres_und_die_quelle_nennt_sie() {
        let aufrufe = std::cell::Cell::new(0_usize);
        let k = |_: &llm::pii::Maskiert, _: i64| {
            aufrufe.set(aufrufe.get() + 1);
            Some(llm::Kategorie::Spende)
        };
        let mut store = store::Store::leer(2025, None);
        let erg = uebernehme(
            &mut store,
            &s1_tx(&[
                ("2025-02-01", 5_000, "Zahlungseingang unklar"),
                ("2024-12-30", -3_000, "Ueberweisung unklar"),
                ("2025-02-02", -2_000, "Ueberweisung unklar"),
                ("2025-02-03", -1_000, "Spende Tierheim"),
            ]),
            store::BindungNachschlag::neu(crate::doctest_bindung().unwrap()),
            Some(&k),
            None,
            None,
        )
        .unwrap();
        assert_eq!(aufrufe.get(), 1, "nur die eine unklare Ausgabe des Jahres");
        assert_eq!((erg.einnahmen, erg.anderes_jahr, erg.uebernommen), (1, 1, 2), "{erg:?}");
        assert_eq!(s1_wert(&store, "spenden_betrag"), Some(3_000));
        let s1 = store
            .aktives("spenden_betrag")
            .and_then(|e| e.signal.as_ref())
            .and_then(|s| s.signal_1.as_ref())
            .and_then(|i| i.as_ref())
            .and_then(|v| v.zu_json().ok())
            .unwrap();
        assert_eq!(s1["quelle"], "llm", "ein Posten kam ueber die KI: {s1}");
        assert_eq!(s1["kategorie"], "spende");
    }

    /// Die Saetze fuer den Nutzer: Reihenfolge der Gruende, nur Zahlen.
    #[test]
    fn s1_saetze_nennen_jahr_datum_und_saldo() {
        let u = Uebernahme {
            uebernommen: 0,
            llm_uebersprungen: 0,
            einnahmen: 1,
            ohne_stichwort: 2,
            partei: 0,
            feld_fehlt: 0,
            feld_belegt: 0,
            anderes_jahr: 3,
            datum_unlesbar: 1,
            saldo_nicht_positiv: 2,
        };
        let jahr = "3 Buchung(en) außerhalb des Veranlagungsjahrs";
        let datum = "1 Buchung(en) mit einem Datum, das der Leser nicht erkennt (erwartet: TT.MM.JJJJ, TT.MM.JJ, JJJJ-MM-TT)";
        let saldo_satz = "2 Buchung(en) in einem Feld, dessen Saldo nach Rückbuchungen null oder negativ ist";
        assert_eq!(
            hinweis_nichts_uebernommen(9, &u),
            Some(format!("9 Buchung(en) gelesen, aber kein Vorschlag: 1 Buchung(en) ohne Minuszeichen (nur Beträge mit Minus gelten als Ausgabe); 2 Buchung(en) ohne erkanntes Stichwort; {jahr}; {datum}; {saldo_satz}."))
        );
        assert_eq!(
            hinweis_nicht_gezaehlt(&u),
            Some(format!("Nicht gezählt: {jahr}; {datum}; {saldo_satz}."))
        );
        let nur_saldo = Uebernahme { anderes_jahr: 0, datum_unlesbar: 0, ..u };
        assert_eq!(
            hinweis_nicht_gezaehlt(&nur_saldo),
            Some(format!("Nicht gezählt: {saldo_satz}."))
        );
        let nichts = Uebernahme { saldo_nicht_positiv: 0, ..nur_saldo };
        assert_eq!(hinweis_nicht_gezaehlt(&nichts), None);
    }

    /// Die Saldo- und Vereinigungsarithmetik an den Raendern.
    #[test]
    fn s1_saldo_und_vereinigung_an_den_raendern() {
        let p = |tag: u32, betrag: i64, zweck: &str| Posten {
            datum: crate::bank::buchung::Datum { jahr: 2025, monat: 3, tag },
            betrag_cent: betrag,
            zweck_maskiert: zweck.to_owned(),
        };
        // Maximum der Haeufigkeit, nicht die Summe.
        let a = [p(1, -100, "x"), p(1, -100, "x")];
        let b = [p(1, -100, "x")];
        assert_eq!(saldo::vereinige(&a, &b).len(), 2);
        assert_eq!(saldo::vereinige(&b, &a).len(), 2);
        assert_eq!(saldo::vereinige(&b, &b).len(), 1);
        // Ein anderer Zweck, ein anderer Betrag, ein anderer Tag sind andere Posten.
        let c = [p(1, -100, "y"), p(1, -101, "x"), p(2, -100, "x")];
        assert_eq!(saldo::vereinige(&b, &c).len(), 4);
        // Die Reihenfolge bleibt: erst `alt`, dann die neuen.
        let ergebnis = saldo::vereinige(&b, &c);
        assert_eq!(ergebnis.first().map(|x| x.zweck_maskiert.as_str()), Some("x"));
        assert_eq!(ergebnis.get(1).map(|x| x.betrag_cent), Some(-100));
        // Ein Betrag, der als Summe das Maximum von i64 fassen darf, aber nicht darueber.
        assert_eq!(saldo::saldo(&[p(1, -i64::MAX, "x")]), Some(i64::MAX));
        assert_eq!(saldo::saldo(&[p(1, i64::MIN, "x")]), None);
        assert_eq!(saldo::saldo(&[p(1, i64::MIN, "x"), p(1, 1, "x")]), Some(i64::MAX));
        assert_eq!(saldo::saldo(&[p(1, i64::MAX, "x"), p(1, 1, "x")]), None);
        assert_eq!(saldo::saldo(&[]), Some(0));
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
