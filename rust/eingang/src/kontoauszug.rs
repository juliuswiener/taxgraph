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
    /// JSON-Zweig: Element ist kein Objekt bzw. Zweck kein Text (Python: `AttributeError` → 500).
    #[error("Transaktion {index}: kein Objekt mit Text-Verwendungszweck")]
    TransaktionUngueltig { index: usize },
    #[error(transparent)]
    Schreiben(#[from] SchreibFehler),
}

/// `float(s)` fuer Text: Leerraum aussen, Dezimalziffern jeder Schrift ([`py::ascii_ziffern`]),
/// `_` nur zwischen Ziffern, dann Rusts Parser (gleiche Grammatik inkl. `inf`/`nan`).
///
/// ```
/// assert_eq!(eingang::kontoauszug::py_float(" 1_000.5 "), Some(1000.5));
/// assert_eq!(eingang::kontoauszug::py_float("1__0"), None);
/// ```
#[must_use]
pub fn py_float(s: &str) -> Option<f64> {
    let s = py::ascii_ziffern(py::strip(s));
    let s = s.as_ref();
    let b = s.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if c == b'_' {
            let davor = i
                .checked_sub(1)
                .and_then(|j| b.get(j))
                .is_some_and(u8::is_ascii_digit);
            let danach = b.get(i + 1).is_some_and(u8::is_ascii_digit);
            if !(davor && danach) {
                return None;
            }
        }
    }
    s.replace('_', "").parse().ok()
}

/// `_eur_cent_signed(s)`: `'-480,00'`/`'1.234,56'` → Cent mit Vorzeichen; nicht lesbar → 0.
///
/// ```
/// assert_eq!(eingang::kontoauszug::eur_cent_signed("-1.234,56 €").unwrap(), -123_456);
/// assert_eq!(eingang::kontoauszug::eur_cent_signed("abc").unwrap(), 0);
/// assert!(eingang::kontoauszug::eur_cent_signed("inf").is_err());
/// ```
///
/// # Errors
/// [`KontoauszugFehler::BetragUeberlauf`] fuer `inf` und Werte ausserhalb `i64`.
pub fn eur_cent_signed(roh: &str) -> Result<i64, KontoauszugFehler> {
    let s = py::strip(roh).replace(['€', ' '], "");
    let neg = s.starts_with('-');
    let s = s.trim_start_matches(['+', '-']);
    let s = if s.contains(',') && s.contains('.') {
        s.replace('.', "").replacen(',', ".", usize::MAX)
    } else {
        s.replace(',', ".")
    };
    let Some(f) = py_float(&s) else { return Ok(0) };
    if f.is_nan() {
        return Ok(0);
    }
    let gerundet = (f * 100.0).round_ties_even();
    if !gerundet.is_finite() || gerundet.abs() >= 9.223_372_036_854_776e18 {
        return Err(KontoauszugFehler::BetragUeberlauf(roh.to_owned()));
    }
    #[allow(clippy::cast_possible_truncation)] // ganzzahlig und im Bereich, oben geprueft
    let cent = gerundet as i64;
    Ok(if neg { -cent } else { cent })
}

/// `parse_csv(text)`: `;`-getrennt, Spalten per Alias (case-insensitiv), Zeilen ohne Betrag
/// uebersprungen.
///
/// ```
/// let tx = eingang::kontoauszug::parse_csv("Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler\n").unwrap();
/// assert_eq!(tx[0].betrag, -48000);
/// ```
///
/// # Errors
/// [`KontoauszugFehler::Csv`] / [`KontoauszugFehler::BetragUeberlauf`].
pub fn parse_csv(text: &str) -> Result<Vec<Transaktion>, KontoauszugFehler> {
    let (kopf, zeilen, csv_fehler) = csv::dict_reader(text, ';');
    if kopf.is_empty() {
        return csv_fehler.map_or(Ok(Vec::new()), |e| Err(e.into()));
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
    for z in &zeilen {
        let betrag_roh = wert(z, &c_bet);
        if c_bet.is_none() || py::strip(&betrag_roh).is_empty() {
            continue;
        }
        out.push(Transaktion {
            datum: Value::String(py::strip(&wert(z, &c_dat)).to_owned()),
            betrag: eur_cent_signed(&betrag_roh)?,
            verwendungszweck: py::strip(&wert(z, &c_zwk)).to_owned(),
        });
    }
    // Der `csv.Error` kommt in Python erst beim Lesen der Zeile, in der er steckt — also nach
    // allen frueheren Zeilen (und deren Betragsfehlern).
    csv_fehler.map_or(Ok(out), |e| Err(e.into()))
}

/// JSON-Zweig (`api.py:973-979` + `int(tx.get("betrag", 0))`): Liste von Objekten. Anders als
/// Python wird die GANZE Liste vor jedem Schreiben geprueft; Python bricht mitten in der
/// Schleife ab (500) und verwirft den Fall ungespeichert — nach aussen gleich.
///
/// ```
/// let tx = eingang::kontoauszug::aus_json(&serde_json::json!([{"betrag": "-5", "verwendungszweck": "Spende"}])).unwrap();
/// assert_eq!(tx[0].betrag, -5);
/// assert!(eingang::kontoauszug::aus_json(&serde_json::json!([{"betrag": "1,5"}])).is_err());
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
                .ok_or(KontoauszugFehler::TransaktionUngueltig { index })?;
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
                // PARITAET-Abweichung: ein falscher Wert (null, 0, "") ist in Python `(z or "")`
                // und damit leer; hier ebenso. Ein wahrer Nicht-Text wirft dort `AttributeError`.
                Some(v) if !py::wahr(v) => String::new(),
                Some(_) => return Err(KontoauszugFehler::TransaktionUngueltig { index }),
            };
            Ok(Transaktion {
                datum: o.get("datum").cloned().unwrap_or_else(|| json!("")),
                betrag,
                verwendungszweck,
            })
        })
        .collect()
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
/// let (tx, verworfen) = eingang::kontoauszug::parse_pdf_zeilen("01.03.2025 Maler Huber -480,00 EUR\nSaldo -1,00", &Default::default(), 0.6).unwrap();
/// assert_eq!((tx[0].betrag, tx[0].verwendungszweck.as_str(), verworfen), (-48000, "Maler Huber", 1));
/// ```
///
/// # Errors
/// [`KontoauszugFehler::BetragUeberlauf`].
pub fn parse_pdf_zeilen(
    text: &str,
    conf: &crate::ocr::ConfMap,
    schwelle: f64,
) -> Result<(Vec<Transaktion>, usize), KontoauszugFehler> {
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
        let zweck = rest.replacen(betrag.as_str(), "", 1);
        let zweck = EUR_ENDE
            .ersetze(&zweck, |_| String::new())
            .map_or(zweck.clone(), |(t, _)| t);
        let zweck = LEERRAUM
            .ersetze(&zweck, |_| " ".to_owned())
            .map_or(zweck.clone(), |(t, _)| t);
        out.push(Transaktion {
            datum: Value::String(datum),
            betrag: eur_cent_signed(betrag)?,
            verwendungszweck: py::strip(&zweck).to_owned(),
        });
    }
    Ok((out, verworfen))
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
