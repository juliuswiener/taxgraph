//! Paritaet `rust/llm` gegen `produkt/haut/{pii_filter,api_llm,llm_client}.py` und die
//! Kontoauszug-Maskierung (`tools/parity/schritt8_oracle.py`). EIN Orakel-Prozess je Binary.
//!
//! - `pii`: alle String-Konstanten aus fuenf Python-Testdateien + 1000 proptest-Texte aus
//!   Fragmenten → `filtere` (Text + Kategorien), `maskiere`, Art.-9-Entscheidung byte-gleich.
//! - `parser_und_beleg`: kanonische Modellausgaben + 1000 generierte → alle fuenf Parser;
//!   Beleg-Gate auf 1000 Paaren.
//! - `dialog`: 20 feste + 1000 generierte Drei-Stufen-Laeufe mit Fixture-Antworten →
//!   Ergebnis, jeder Prompt (SHA-256), Schema je Aufruf.
//! - `client`: lokaler Fake-Server, dieselben Skripte fuer beide Seiten → Klasse, Grund,
//!   Versuche, Zahl der Anfragen, Anfrage-Koerper. Nie ein echter Anbieter.
//! - `klassifikator_und_schemas`: Kontoauszug-Kategorie-Parser, Klassifikator-Nachricht, Schemas.
//!
//! Jeder Test hat eine Negativkontrolle (ein gestoertes Rust-Ergebnis muss als Abweichung zaehlen).
//!
//!   `PARITY`=1 `cargo` test -p parity --test `llm_paritaet` -- --nocapture
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines)]

use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use llm::gates::KatalogFeld;
use llm::{Chat, Completion, LlmFehler, Nachricht};
use parity::Oracle;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use serde_json::{json, Value};

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn oracle() -> &'static Mutex<Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
}

fn frage(anfrage: &Value) -> Value {
    let a = oracle().lock().unwrap_or_else(std::sync::PoisonError::into_inner).call_json(anfrage).expect("orakel antwortet");
    a.get("ok").cloned().unwrap_or_else(|| panic!("Orakel-Fehler: {a}"))
}

/// Deterministischer Zufall fuer strukturierte Eingaben (xorshift64*).
struct Rng(u64);
impl Rng {
    fn n(&mut self, bis: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        usize::try_from(self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33).unwrap() % bis.max(1)
    }
    fn wahl<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.n(v.len())]
    }
    fn p(&mut self, prozent: usize) -> bool {
        self.n(100) < prozent
    }
}

/// Zaehlt Faelle und Abweichungen; zeigt die ersten fuenf.
#[derive(Default)]
struct Zaehler {
    faelle: usize,
    abw: usize,
    /// Im Bericht als PARITAET-Abweichung gefuehrte Faelle (Python stuerzt, Rust meldet).
    dokumentiert: usize,
}
impl Zaehler {
    fn pruefe(&mut self, was: &str, rust: &Value, py: &Value) {
        self.faelle += 1;
        if rust != py {
            self.abw += 1;
            if self.abw <= 5 {
                eprintln!("ABWEICHUNG {was}\n  bei {}", pfad(rust, py, "$"));
            }
        }
    }
}

/// Erster abweichender JSON-Pfad.
fn pfad(links: &Value, rechts: &Value, ort: &str) -> String {
    match (links, rechts) {
        (Value::Object(lo), Value::Object(ro)) => {
            for k in lo.keys().chain(ro.keys()) {
                if lo.get(k) != ro.get(k) {
                    return pfad(lo.get(k).unwrap_or(&Value::Null), ro.get(k).unwrap_or(&Value::Null), &format!("{ort}.{k}"));
                }
            }
            ort.to_owned()
        }
        (Value::Array(la), Value::Array(ra)) if la.len() == ra.len() => la
            .iter()
            .zip(ra)
            .enumerate()
            .find(|(_, (l, r))| l != r)
            .map_or(ort.to_owned(), |(i, (l, r))| pfad(l, r, &format!("{ort}[{i}]"))),
        _ => format!("{ort}: rust={} py={}", kurz(links), kurz(rechts)),
    }
}

fn kurz(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 600 { format!("{}…", &s[..s.char_indices().nth(600).map_or(s.len(), |(i, _)| i)]) } else { s }
}

// ------------------------------------------------------------------ PII

const FRAGMENTE: &[&str] = &[
    "DE89370400440532013000", "de89 3704 0044 0532 0130 00", "NL91ABNA0417164300", "DE89-3704-0044-0532-0130-00",
    "DE89 3704 0044 0532 01", "12345678901", "181/815/08155", "2181081508155", "12 345 678 901", "0532013000",
    "4111111111111111", "1234567", "01.02.2024", "31.12.2025", "32.01.2024", "1.2.2024", "12345 Berlin",
    "80331 München", "12345 Euro", "12345 Europa", "12345  Tage", "12345\tÖsterreich", "12345 km", "123456 Berlin",
    "Hauptstraße 5", "Bahnhofstr. 12a", "Lindenweg 7", "Parkallee", "Arbeitsplatz", "Am Ring 3", "Gartenweg",
    "Herr Maier", "Frau Özdemir", "Herr müller", "Frau", "Kirchensteuer", "Grad der Behinderung 80", "Pflegegrad 3",
    "blind", "schwerbehindert", "Heilbehandlung", "Konfession", "1.234,56 €", "62000 Euro", "15000", "20 km",
    "§ 35a", "Riester", "Rürup", "Ehegatte", "ich", "fahre", "zur", "Arbeit", "²", "½", "\u{301}", "‿", "ß",
    "Ärger", "x", "_", "5", "0", "IBAN:", "StNr", "Kto-Nr.", "", "Nr. 3", "E.V.", "e. V.",
];
const TRENNER: &[&str] = &[" ", ", ", "\n", "\t", "", "-", "/", ".", "\x1c", "  ", ": ", "\r\n"];

fn pii_texte(n: usize) -> Vec<String> {
    let strat = prop::collection::vec((prop::sample::select(FRAGMENTE), prop::sample::select(TRENNER)), 1..9)
        .prop_map(|v| v.into_iter().flat_map(|(f, t)| [f, t]).collect::<String>());
    let mut runner = TestRunner::deterministic();
    (0..n).map(|_| strat.new_tree(&mut runner).unwrap().current()).collect()
}

fn rust_filtere(t: &str) -> Value {
    let (g, k) = llm::pii::filtere(t);
    json!([g.as_str(), k])
}

#[test]
fn pii() {
    if skip() {
        return;
    }
    let dateien = ["tests/test_pii_filter.py", "tests/test_art9_nicht_an_llm.py", "tests/test_kontoauszug_maskierung.py",
        "tests/test_chat_beleg_gate.py", "tests/test_dialog_drei_stufen.py"];
    let korpus: Vec<String> = serde_json::from_value(frage(&json!({"fn": "schritt8.korpus", "dateien": dateien}))).unwrap();
    let n_korpus = korpus.len();
    let mut texte = korpus;
    texte.extend(pii_texte(1000));
    let py_f = frage(&json!({"fn": "schritt8.llm.filtere", "texte": texte}));
    let py_m = frage(&json!({"fn": "schritt8.llm.maskiere", "texte": texte}));
    let paare: Vec<[&str; 2]> = texte.iter().flat_map(|t| [[t.as_str(), ""], ["", t.as_str()]]).collect();
    let py_a = frage(&json!({"fn": "schritt8.llm.art9", "paare": paare}));
    let mut z = Zaehler::default();
    for (i, t) in texte.iter().enumerate() {
        z.pruefe(&format!("filtere {t:?}"), &rust_filtere(t), &py_f[i]);
        z.pruefe(&format!("maskiere {t:?}"), &json!(llm::pii::maskiere(t).as_str()), &py_m[i]);
    }
    for (i, [f, q]) in paare.iter().enumerate() {
        z.pruefe(&format!("art9 {f:?}/{q:?}"), &json!(llm::pii::ist_besondere_kategorie(f, q)), &py_a[i]);
    }
    let mut neg = Zaehler::default();
    neg.pruefe("negativ", &json!([format!("{}x", llm::pii::filtere(&texte[0]).0), []]), &py_f[0]);
    let getroffen = py_f.as_array().unwrap().iter().filter(|r| !r[1].as_array().unwrap().is_empty()).count();
    println!("pii: {} Texte ({n_korpus} Korpus + 1000 proptest), {} Vergleiche, davon {getroffen} mit Treffer; Abweichungen {}; Negativkontrolle {}",
        texte.len(), z.faelle, z.abw, neg.abw);
    assert_eq!(neg.abw, 1, "Negativkontrolle muss rot sein");
    assert_eq!(z.abw, 0);
}

// ------------------------------------------------------------------ Parser + Beleg-Gate

const FREITEXT: &str = "Ich bin verheiratet, habe 2 Kinder und fahre an 220 Tagen 20km zur Arbeit. Brutto 62000 Euro, IBAN DE89370400440532013000, 5 Tage krank.";

fn wert(r: &mut Rng) -> Value {
    match r.n(9) {
        0 => json!(r.n(100_000)),
        1 => json!(-(i64::try_from(r.n(500)).unwrap())),
        2 => json!(f64::from(u32::try_from(r.n(10_000)).unwrap()) / 7.0),
        3 => json!("verheiratet"),
        4 => json!(true),
        5 => Value::Null,
        6 => json!(1e16),
        7 => json!([1, "a"]),
        _ => json!({"a": 1}),
    }
}

fn beleg(r: &mut Rng) -> Value {
    let quellen = ["20km", "2 Kinder", "5", "220 Tagen", "verheiratet", "erfunden", "", "  BRUTTO   62000  ", "[PII]", "e", "Tagen 20", "62000 Euro, IBAN"];
    if r.p(5) { wert(r) } else { json!(r.wahl(&quellen)) }
}

fn aussage_nr(r: &mut Rng) -> Value {
    r.wahl(&[json!(0), json!(1), json!(2), json!(-1), json!(7), json!("1"), json!(1.9), Value::Null, json!(true), json!("x")]).clone()
}

fn vorschlag(r: &mut Rng, felder: &[String]) -> Value {
    let mut o = serde_json::Map::new();
    if !r.p(5) {
        o.insert("feld_id".into(), if r.p(90) { json!(r.wahl(felder)) } else { wert(r) });
    }
    if !r.p(5) {
        o.insert("wert".into(), wert(r));
    }
    for (k, p) in [("beleg", 90), ("begruendung", 70), ("aussage", 80), ("rechenweg", 50)] {
        if r.p(p) {
            let v = match k {
                "beleg" => beleg(r),
                "begruendung" => json!("x".repeat(r.n(250))),
                "aussage" => aussage_nr(r),
                _ => if r.p(50) { Value::Null } else { json!({"basis": 5_000_000, "faktor": 0.5, "erklaerung": "50.000 € ÷ 12 × 6"}) },
            };
            o.insert(k.into(), v);
        }
    }
    if r.p(5) {
        o.insert("fremd".into(), json!(1));
    }
    Value::Object(o)
}

fn rueckfrage(r: &mut Rng, felder: &[String]) -> Value {
    let fragen = ["Wie viel Euro?", "Wie viele Kinder hast du?", " ", "Wann?", "Anzahl der Renten?", "Wie hoch in €?", "EURO-Betrag?", "Europa?"];
    let fid = match r.n(5) {
        0 => json!(""),
        1 => json!("unbekannt_feld"),
        2 => Value::Null,
        _ => json!(r.wahl(felder)),
    };
    if r.p(5) {
        return json!("keine Rueckfrage");
    }
    json!({"frage": r.wahl(&fragen), "feld_id": fid, "aussage": aussage_nr(r)})
}

fn modell_ausgabe(r: &mut Rng, felder: &[String], regeln: &[String]) -> String {
    let n = r.n(5);
    let vs: Vec<Value> = (0..n).map(|_| vorschlag(r, felder)).collect();
    let rf: Vec<Value> = (0..r.n(12)).map(|_| rueckfrage(r, felder)).collect();
    let aussagen: Vec<Value> = (0..r.n(4)).map(|_| {
        if r.p(10) { json!(5) } else { json!({"text": r.wahl(&["Der Nutzer ist verheiratet", "  ", "IBAN DE89370400440532013000", "hat 2 Kinder"]), "beleg": beleg(r)}) }
    }).collect();
    let zu: Vec<Value> = (0..r.n(4)).map(|_| {
        let mut rs: Vec<Value> = (0..r.n(3)).map(|_| json!(r.wahl(regeln))).collect();
        if r.p(20) { rs.push(json!("erfunden")); }
        if r.p(10) { rs.push(json!(7)); }
        json!({"aussage": aussage_nr(r), "regeln": if r.p(5) { json!("r") } else { json!(rs) }})
    }).collect();
    let v = match r.n(12) {
        0 => json!({"vorschläge": vs}),
        1 => json!({"suggestions": vs}),
        2 => json!({"felder": vs}),
        3 => json!(vs),
        4 => vs.first().cloned().unwrap_or(json!({})),
        5 => json!({"aussagen": aussagen}),
        6 => json!({"zuordnungen": zu}),
        7 => json!("nur text"),
        8 => json!(42),
        _ => json!({"vorschlaege": vs, "rueckfragen": rf, "antwort": if r.p(80) { json!(" Du kannst das absetzen. ") } else { wert(r) },
            "unsicher": if r.p(80) { json!(r.p(50)) } else { wert(r) }, "aussagen": aussagen, "zuordnungen": zu}),
    };
    let s = v.to_string();
    match r.n(10) {
        0 => s[..s.char_indices().nth(r.n(s.chars().count().max(1))).map_or(s.len(), |(i, _)| i)].to_owned(),
        1 => format!("```json\n{s}\n```"),
        2 => format!("{s} trailing"),
        _ => s,
    }
}

fn rust_parse(t: &str, freitext: &str, felder: usize, erlaubt: &HashSet<String>, anzahl: usize) -> Value {
    let (g, _) = llm::pii::filtere(freitext);
    let chat = llm::parse::chat_parse(t).oder_leer_wie_python();
    let rueck = llm::parse::rueckfragen_parse(t, felder).oder_leer_wie_python();
    let (antwort, unsicher) = llm::parse::antwort_parse(t).oder_leer_wie_python();
    let aussagen = llm::parse::aussagen_parse(t, &g).oder_leer_wie_python();
    let z = llm::parse::zuordnung_parse(t, erlaubt, anzahl).oder_leer_wie_python();
    json!({"chat": {"ok": chat}, "rueck": {"ok": rueck}, "antwort": [antwort, unsicher], "aussagen": {"ok": aussagen}, "zuordnung": z})
}

#[test]
fn parser_und_beleg() {
    if skip() {
        return;
    }
    let felder: Vec<String> = ["bruttoarbeitslohn", "fam_anzahl_kinder", "ep_arbeitstage", "ep_entfernung_km", "veranlagung"].map(String::from).to_vec();
    let regeln: Vec<String> = ["p19_1", "p32_6", "p9_ep"].map(String::from).to_vec();
    let erlaubt: HashSet<String> = regeln.iter().cloned().collect();
    let mut texte: Vec<String> = [
        r#"{"vorschlaege": [{"feld_id": "ep_entfernung_km", "wert": 20, "beleg": "20km", "begruendung": "x", "aussage": 0, "rechenweg": null}], "rueckfragen": [], "antwort": "", "unsicher": false}"#,
        r#"[{"feld_id": "a", "wert": 1}]"#, r#"{"feld_id": "a", "wert": null}"#, "", "kaputt", "{", "[]", "null", r#"{"vorschlaege": "x"}"#,
        r#"{"aussagen": [{"text": "Der Nutzer ist ledig", "beleg": "bin ledig"}]}"#, r#"{"zuordnungen": [{"aussage": 0, "regeln": ["p19_1", "p19_1"]}]}"#,
        r#"{"antwort": null, "unsicher": "false"}"#, r#"{"rueckfragen": [{"frage": "Wie viel?", "feld_id": "x"}]}"#,
    ].map(String::from).to_vec();
    let mut r = Rng(0x5eed_1234);
    texte.extend((0..1000).map(|_| modell_ausgabe(&mut r, &felder, &regeln)));
    let mut z = Zaehler::default();
    for (felder_n, anzahl) in [(5usize, 3usize), (0, 1)] {
        let py = frage(&json!({"fn": "schritt8.llm.parse", "texte": texte, "freitext": FREITEXT, "felder": felder_n,
            "erlaubt": regeln, "anzahl": anzahl}));
        for (i, t) in texte.iter().enumerate() {
            z.pruefe(&format!("parse {t:?}"), &rust_parse(t, FREITEXT, felder_n, &erlaubt, anzahl), &py[i]);
        }
    }
    // Beleg-Gate: 1000 Paare aus Belegen und Texten.
    let texte_frei = ["ich fahre 20km mit dem auto", "15000 Euro", "5 Tage", "Ärger ² 5²", "a\u{301}b 5", "IBAN DE89370400440532013000 5"];
    let belege = ["5", "20km", "fahre 20", "²", "a", "5²", "", " ", "EURO", "[PII]", "b", "Tage"];
    let paare: Vec<[&str; 2]> = (0..1000).map(|_| [*r.wahl(&belege), *r.wahl(&texte_frei)]).collect();
    let py_b = frage(&json!({"fn": "schritt8.llm.beleg", "paare": paare}));
    for (i, [b, f]) in paare.iter().enumerate() {
        let v = llm::parse::chat_parse(&json!([{"feld_id": "x", "wert": 1, "beleg": b}]).to_string()).oder_leer_wie_python();
        let (ok, _) = llm::gates::beleg_geprueft(v, &llm::pii::filtere(f).0);
        z.pruefe(&format!("beleg {b:?} in {f:?}"), &json!(!ok.is_empty()), &py_b[i]);
    }
    let mut neg = Zaehler::default();
    let mut gestoert = rust_parse(&texte[0], FREITEXT, 5, &erlaubt, 3);
    gestoert["antwort"][1] = json!(true);
    let py0 = frage(&json!({"fn": "schritt8.llm.parse", "texte": [texte[0]], "freitext": FREITEXT, "felder": 5, "erlaubt": regeln, "anzahl": 3}));
    neg.pruefe("negativ", &gestoert, &py0[0]);
    let schemagerecht = texte.iter().filter(|t| matches!(llm::parse::chat_parse(t), llm::Antwort::Schemagerecht(_))).count();
    let unlesbar = texte.iter().filter(|t| matches!(llm::parse::chat_parse(t), llm::Antwort::Unlesbar)).count();
    println!("parser: {} Modelltexte × 2 Parameter + 1000 Beleg-Paare = {} Vergleiche; schemagerecht {schemagerecht}, unlesbar {unlesbar}; Abweichungen {}; Negativkontrolle {}",
        texte.len(), z.faelle, z.abw, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
}

// ------------------------------------------------------------------ Dialog

/// Fixture-Chat: gibt Antworten der Reihe nach aus und zeichnet jeden Aufruf auf.
struct FixChat {
    antworten: RefCell<VecDeque<Value>>,
    aufrufe: RefCell<Vec<Value>>,
}

impl Chat for FixChat {
    fn complete(&self, nachrichten: &[Nachricht], schema: Option<&Value>) -> Result<Completion, LlmFehler> {
        let msgs: Vec<Value> = nachrichten.iter().map(|m| json!({"role": m.rolle(), "sha256": store::sha256_hex(m.inhalt())})).collect();
        self.aufrufe.borrow_mut().push(json!({"messages": msgs, "schema": schema.and_then(|s| s.get("name")).cloned()}));
        let a = self.antworten.borrow_mut().pop_front().expect("genug Fixture-Antworten");
        if let Some(g) = a.get("fehler").and_then(Value::as_str) {
            let grund = if g == "abgeschnitten" { llm::Grund::Abgeschnitten } else { llm::Grund::Leer };
            return Err(LlmFehler::Voruebergehend { versuche: 3, grund, detail: String::new() });
        }
        Ok(Completion { text: a["text"].as_str().unwrap().to_owned(), provider: String::new(), finish: String::new() })
    }
}

struct Szenario {
    freitext: String,
    kontext: String,
    indizes: Vec<usize>,
    antworten: Vec<Value>,
}

fn szenario(r: &mut Rng, katalog: &[KatalogFeld]) -> Szenario {
    let frag = ["Ich bin verheiratet", "habe 2 Kinder", "fahre 20km zur Arbeit", "bis Juni 100k p.a.", "bekomme 1850 euro im monat",
        "Herr Maier wohnt in 12345 Berlin", "was ist die Pendlerpauschale?", "seit Juli arbeitslos", "IBAN DE89370400440532013000"];
    let freitext = (0..=r.n(4)).map(|_| *r.wahl(&frag)).collect::<Vec<_>>().join(", ");
    let freitext = if r.p(3) { "   ".to_owned() } else { freitext };
    let kontext = if r.p(40) { format!("Offenes Feld: {} (geb. 01.02.1980)", r.wahl(katalog).feld_id) } else { String::new() };
    let indizes: Vec<usize> = if r.p(10) {
        (0..katalog.len()).collect()
    } else {
        let mut v: Vec<usize> = (0..=r.n(40)).map(|_| r.n(katalog.len())).collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let felder: Vec<String> = indizes.iter().map(|&i| katalog[i].feld_id.clone()).collect();
    let mut regeln: Vec<String> = indizes.iter().filter_map(|&i| katalog[i].regel_id.clone()).collect();
    regeln.push("erfunden".into());
    let gefiltert = llm::pii::filtere(&freitext).0.as_str().to_owned();
    let stueck = |r: &mut Rng| -> String {
        let w: Vec<&str> = gefiltert.split(' ').collect();
        let a = r.n(w.len());
        w[a..(a + r.n(3) + 1).min(w.len())].join(" ")
    };
    let aussagen: Vec<Value> = (0..r.n(5)).map(|_| json!({"text": format!("Der Nutzer: {}", stueck(r)), "beleg": if r.p(80) { stueck(r) } else { "erfunden".into() }})).collect();
    let c1 = if r.p(10) { "kaputt".to_owned() } else { json!({"aussagen": aussagen}).to_string() };
    let zu: Vec<Value> = (0..r.n(5)).map(|_| json!({"aussage": i64::try_from(r.n(6)).unwrap() - 1, "regeln": (0..=r.n(3)).map(|_| r.wahl(&regeln).clone()).collect::<Vec<_>>()})).collect();
    let c2 = if r.p(10) { "{".to_owned() } else { json!({"zuordnungen": zu}).to_string() };
    let vs: Vec<Value> = (0..r.n(6)).map(|_| json!({"feld_id": r.wahl(&felder), "wert": r.n(100_000), "beleg": if r.p(75) { stueck(r) } else { "erfunden".into() },
        "begruendung": "b", "aussage": i64::try_from(r.n(7)).unwrap() - 1, "rechenweg": Value::Null})).collect();
    let rf: Vec<Value> = (0..r.n(12)).map(|_| json!({"frage": r.wahl(&["Wie viel Euro im Monat?", "Wie viele Renten?", "Wann genau?", "Anzahl Tage?"]),
        "feld_id": if r.p(80) { json!(r.wahl(&felder)) } else { json!(r.wahl(&["", "unbekannt"])) }, "aussage": i64::try_from(r.n(6)).unwrap() - 1})).collect();
    let c3 = if r.p(8) { "[]".to_owned() } else { json!({"vorschlaege": vs, "rueckfragen": rf, "antwort": "Das zählt.", "unsicher": r.p(30)}).to_string() };
    let mut antworten = vec![json!({"text": c1})];
    let hat_regeln = indizes.iter().any(|&i| katalog[i].regel_id.as_deref().is_some_and(|s| !s.is_empty()));
    if hat_regeln {
        antworten.push(if r.p(8) { json!({"fehler": "abgeschnitten"}) } else { json!({"text": c2}) });
    }
    antworten.push(if r.p(8) { json!({"fehler": "leere_antwort"}) } else { json!({"text": c3}) });
    if r.p(5) {
        antworten[0] = json!({"fehler": "abgeschnitten"});
    }
    Szenario { freitext, kontext, indizes, antworten }
}

fn rust_dialog(s: &Szenario, katalog: &[KatalogFeld], gruppen: &[(String, String)]) -> Value {
    let chat = FixChat { antworten: RefCell::new(s.antworten.iter().cloned().collect()), aufrufe: RefCell::new(Vec::new()) };
    let kat: Vec<KatalogFeld> = s.indizes.iter().map(|&i| katalog[i].clone()).collect();
    let erg = llm::dialog::llm_dialog(&chat, &s.freitext, &kat, &s.kontext, gruppen, &llm::dialog::KeinProtokoll);
    let erg = match erg {
        Ok(e) => json!({"ok": e}),
        Err(_) => json!({"err": "LlmNichtVerfuegbar"}),
    };
    json!({"ergebnis": erg, "aufrufe": chat.aufrufe.into_inner()})
}

/// Diagnose: beim ersten abweichenden Lauf beide Prompts im Klartext vergleichen.
fn zeige_prompt_diff(s: &Szenario, katalog: &[KatalogFeld], gruppen: &[(String, String)]) {
    struct Klar(RefCell<VecDeque<Value>>, RefCell<Vec<String>>);
    impl Chat for Klar {
        fn complete(&self, m: &[Nachricht], _: Option<&Value>) -> Result<Completion, LlmFehler> {
            self.1.borrow_mut().push(m[0].inhalt().to_owned());
            let a = self.0.borrow_mut().pop_front().unwrap();
            a["text"].as_str().map_or_else(
                || Err(LlmFehler::Endgueltig { grund: llm::Grund::Sonstig, detail: String::new() }),
                |t| Ok(Completion { text: t.to_owned(), ..Default::default() }),
            )
        }
    }
    let k = Klar(RefCell::new(s.antworten.iter().cloned().collect()), RefCell::new(Vec::new()));
    let kat: Vec<KatalogFeld> = s.indizes.iter().map(|&i| katalog[i].clone()).collect();
    let _ = llm::dialog::llm_dialog(&k, &s.freitext, &kat, &s.kontext, gruppen, &llm::dialog::KeinProtokoll);
    let py = frage(&json!({"fn": "schritt8.llm.dialog", "freitext": s.freitext, "kontext": s.kontext,
        "katalog": {"indizes": s.indizes}, "antworten": s.antworten}));
    for (i, r) in k.1.into_inner().iter().enumerate() {
        let p = py["aufrufe"][i]["messages"][0]["content"].as_str().unwrap_or("");
        if r != p {
            let ab = r.chars().zip(p.chars()).take_while(|(a, b)| a == b).count();
            let ctx = |t: &str| t.chars().skip(ab.saturating_sub(80)).take(240).collect::<String>();
            eprintln!("PROMPT {i} weicht ab bei Zeichen {ab}\n  rust: {:?}\n  py:   {:?}", ctx(r), ctx(p));
        }
    }
}

#[test]
fn dialog() {
    if skip() {
        return;
    }
    let k = frage(&json!({"fn": "schritt8.llm.katalog"}));
    let mut katalog: Vec<KatalogFeld> = serde_json::from_value(k["katalog"].clone()).unwrap();
    for (f, roh) in katalog.iter_mut().zip(k["katalog"].as_array().unwrap()) {
        if let Some(paare) = roh["bereich_paare"].as_array() {
            f.bereich = Some(llm::py::GeordneteMap(paare.iter().map(|p| (p[0].as_str().unwrap().to_owned(), p[1].clone())).collect()));
        }
    }
    let gruppen: Vec<(String, String)> = serde_json::from_value(k["gruppen"].clone()).unwrap();
    let mut r = Rng(0xd1a1_0600);
    let mut z = Zaehler::default();
    let mut stufen = [0usize; 4];
    let mut erste = None;
    for _ in 0..1020 {
        let s = szenario(&mut r, &katalog);
        let rust = rust_dialog(&s, &katalog, &gruppen);
        let mut py = frage(&json!({"fn": "schritt8.llm.dialog", "freitext": s.freitext, "kontext": s.kontext,
            "katalog": {"indizes": s.indizes}, "antworten": s.antworten, "nur_hash": true}));
        if let Some(e) = py["ergebnis"].get("err") {
            py["ergebnis"] = json!({"err": e});
        }
        stufen[rust["aufrufe"].as_array().unwrap().len()] += 1;
        if rust != py && z.abw == 0 {
            zeige_prompt_diff(&s, &katalog, &gruppen);
        }
        z.pruefe(&format!("dialog {:?}", s.freitext), &rust, &py);
        erste.get_or_insert((rust, py));
    }
    let (mut gestoert, py0) = erste.unwrap();
    gestoert["aufrufe"][0]["messages"][0]["sha256"] = json!("0");
    let mut neg = Zaehler::default();
    neg.pruefe("negativ", &gestoert, &py0);
    println!("dialog: {} Laeufe (Katalog {} Felder, {} Instanz-Gruppen); Aufrufe je Lauf 0/1/2/3 = {stufen:?}; Abweichungen {}; Negativkontrolle {}",
        z.faelle, katalog.len(), gruppen.len(), z.abw, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
}

// ------------------------------------------------------------------ Client gegen Fake-Server

#[derive(Clone)]
enum Aktion {
    Antwort { status: u16, body: String, chunked: bool },
    Schliessen,
    Stille(Duration),
    Tropfen { vorne: usize, body: String, pause: Duration },
}

struct FakeServer {
    port: u16,
    anfragen: Arc<AtomicUsize>,
    koerper: Arc<Mutex<Vec<(String, Value)>>>,
    skript: Arc<Mutex<VecDeque<Aktion>>>,
}

fn lies_anfrage(s: &mut std::net::TcpStream) -> Option<(String, Value)> {
    let mut buf = Vec::new();
    let mut b = [0u8; 4096];
    let kopf_ende = loop {
        let n = s.read(&mut b).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&b[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let kopf = String::from_utf8_lossy(&buf[..kopf_ende]).to_string();
    let laenge: usize = kopf.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap())).unwrap_or(0);
    while buf.len() < kopf_ende + laenge {
        let n = s.read(&mut b).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&b[..n]);
    }
    let auth = kopf.lines().find_map(|l| l.strip_prefix("Authorization: ").map(str::to_owned)).unwrap_or_default();
    Some((auth, serde_json::from_slice(&buf[kopf_ende..]).unwrap_or(Value::Null)))
}

fn antworte(mut s: std::net::TcpStream, a: Aktion) {
    let _ = match a {
        Aktion::Antwort { status, body, chunked: false } => {
            write!(s, "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
        }
        Aktion::Antwort { status, body, chunked: true } => {
            let (a, b) = body.split_at(body.len() / 2);
            write!(s, "HTTP/1.1 {status} X\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{a}\r\n{:x}\r\n{b}\r\n0\r\n\r\n", a.len(), b.len())
        }
        Aktion::Schliessen => Ok(()),
        Aktion::Stille(d) => {
            std::thread::sleep(d);
            Ok(())
        }
        Aktion::Tropfen { vorne, body, pause } => {
            let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let (a, b) = body.as_bytes().split_at(vorne);
            let _ = s.write_all(a);
            for c in b {
                std::thread::sleep(pause);
                if s.write_all(&[*c]).is_err() {
                    break;
                }
            }
            Ok(())
        }
    };
}

impl FakeServer {
    fn starte() -> Self {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let anfragen = Arc::new(AtomicUsize::new(0));
        let koerper = Arc::new(Mutex::new(Vec::new()));
        let skript: Arc<Mutex<VecDeque<Aktion>>> = Arc::new(Mutex::new(VecDeque::new()));
        let (a, k, sk) = (anfragen.clone(), koerper.clone(), skript.clone());
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let (a, k, sk) = (a.clone(), k.clone(), sk.clone());
                std::thread::spawn(move || {
                    let mut s = s;
                    let Some(anfrage) = lies_anfrage(&mut s) else { return };
                    a.fetch_add(1, Ordering::SeqCst);
                    k.lock().unwrap().push(anfrage);
                    let aktion = sk.lock().unwrap().pop_front().unwrap_or(Aktion::Schliessen);
                    antworte(s, aktion);
                });
            }
        });
        Self { port, anfragen, koerper, skript }
    }

    fn lade(&self, skript: &[Aktion]) {
        *self.skript.lock().unwrap() = skript.iter().cloned().collect();
        self.anfragen.store(0, Ordering::SeqCst);
        self.koerper.lock().unwrap().clear();
    }
}

fn ok_body(inhalt: &Value, finish: &str) -> String {
    json!({"choices": [{"message": {"content": inhalt}, "finish_reason": finish}], "provider": "fake"}).to_string()
}

fn szenarien() -> Vec<(&'static str, Vec<Aktion>)> {
    let a = |status: u16, body: String| Aktion::Antwort { status, body, chunked: false };
    let gut = ok_body(&json!("{\"aussagen\": []}"), "stop");
    let lang = ok_body(&json!("{\"aus"), "length");
    let leer = ok_body(&json!(""), "stop");
    let gross = ok_body(&json!("x".repeat(70_000)), "stop");
    vec![
        ("200", vec![a(200, gut.clone())]),
        ("429,500,200", vec![a(429, "{}".into()), a(500, "{}".into()), a(200, gut.clone())]),
        ("503x3", vec![a(503, "{}".into()), a(503, "{}".into()), a(503, "{}".into())]),
        ("length,length", vec![a(200, lang.clone()), a(200, lang.clone())]),
        ("length,200", vec![a(200, lang.clone()), a(200, gut.clone())]),
        ("503,length,length", vec![a(503, "{}".into()), a(200, lang.clone()), a(200, lang.clone())]),
        ("length,503,503", vec![a(200, lang.clone()), a(503, "{}".into()), a(503, "{}".into())]),
        ("503,503,length", vec![a(503, "{}".into()), a(503, "{}".into()), a(200, lang.clone())]),
        ("leer x3", vec![a(200, leer.clone()), a(200, leer.clone()), a(200, leer.clone())]),
        ("leer,200", vec![a(200, leer), a(200, gut.clone())]),
        ("content 0", vec![a(200, ok_body(&json!(0), "stop")); 3]),
        ("content zahl", vec![a(200, ok_body(&json!(5), "stop"))]),
        ("400 mit Key", vec![a(400, "invalid key sk-test-GEHEIM-123 given".into())]),
        ("401", vec![a(401, "{}".into())]),
        ("200 kein JSON", vec![a(200, "<html>".into())]),
        ("200 ohne choices", vec![a(200, "{\"x\": 1}".into())]),
        ("200 choices leer", vec![a(200, "{\"choices\": []}".into())]),
        ("200 chunked", vec![Aktion::Antwort { status: 200, body: gut.clone(), chunked: true }]),
        ("200 gross", vec![a(200, gross.clone())]),
        ("schliessen x3", vec![Aktion::Schliessen, Aktion::Schliessen, Aktion::Schliessen]),
        ("stille,200", vec![Aktion::Stille(Duration::from_millis(900)), a(200, gut.clone())]),
        ("stille x3", vec![Aktion::Stille(Duration::from_millis(900)); 3]),
        ("tropfen ueber frist", vec![Aktion::Tropfen { vorne: 70_000, body: gross, pause: Duration::from_millis(30) }]),
        ("tropfen klein", vec![Aktion::Tropfen { vorne: 10, body: gut, pause: Duration::from_millis(20) }]),
    ]
}

fn rust_klasse(r: &Result<Completion, LlmFehler>) -> Value {
    match r {
        Ok(c) => json!({"ok": {"text": c.text, "provider": c.provider, "finish": c.finish}}),
        Err(e) => {
            let klasse = match e {
                LlmFehler::Voruebergehend { .. } => "voruebergehend",
                LlmFehler::Abgeschnitten { .. } => "abgeschnitten",
                LlmFehler::Endgueltig { .. } => "endgueltig",
            };
            json!({"err": {"klasse": klasse, "grund": e.grund(), "versuche": e.versuche(), "key_in_msg": e.to_string().contains("sk-test-GEHEIM-123")}})
        }
    }
}

#[test]
fn client() {
    if skip() {
        return;
    }
    let server = FakeServer::starte();
    let basis = format!("http://127.0.0.1:{}/v1", server.port);
    let key = "sk-test-GEHEIM-123";
    let mut k = llm::Konfiguration::neu(basis.clone(), "m".into(), key.into());
    k.socket = Duration::from_millis(400);
    k.frist = Duration::from_millis(1500);
    k.frist_wiederholung = Duration::from_millis(1000);
    k.backoff = [Duration::ZERO; 2];
    let chat = llm::HttpChat { konfiguration: k };
    let nachrichten = llm::prompt::aussagen_prompt(&llm::pii::filtere("ich bin ledig").0);
    let mut z = Zaehler::default();
    for (name, skript) in szenarien() {
        for schema in [false, true] {
            server.lade(&skript);
            let rust = rust_klasse(&chat.complete(&nachrichten, schema.then_some(&*llm::schema::DIALOG_SCHEMA)));
            let (n_rust, k_rust) = (server.anfragen.load(Ordering::SeqCst), server.koerper.lock().unwrap().first().cloned());
            server.lade(&skript);
            let py = frage(&json!({"fn": "schritt8.llm.client", "base": basis, "model": "m", "key": key, "socket_s": 0.4,
                "frist_s": 1.5, "frist_wdh_s": 1.0, "schema": schema, "messages": nachrichten}));
            let (n_py, k_py) = (server.anfragen.load(Ordering::SeqCst), server.koerper.lock().unwrap().first().cloned());
            // PARITAET-Abweichung (Bericht): Python ruft `.strip()` auf einem Zahl-Inhalt und stuerzt
            // mit `AttributeError`; Rust meldet endgueltig.
            if py["err"]["klasse"] == "absturz:AttributeError" && rust["err"]["klasse"] == "endgueltig" {
                z.dokumentiert += 1;
            } else {
                z.pruefe(&format!("{name} schema={schema}: Ergebnis"), &rust, &py);
            }
            z.pruefe(&format!("{name} schema={schema}: Anfragen"), &json!(n_rust), &json!(n_py));
            z.pruefe(&format!("{name} schema={schema}: Anfrage-Koerper"), &json!(k_rust), &json!(k_py));
            println!("  client {name:<22} schema={schema:<5} rust={} anfragen={n_rust}", rust.get("err").map_or("ok".to_string(), |e| format!("{}/{}/{}", e["klasse"], e["grund"], e["versuche"])));
        }
    }
    // Kein Server: Verbindung verweigert.
    let tot = TcpListener::bind("127.0.0.1:0").unwrap();
    let tot_basis = format!("http://127.0.0.1:{}/v1", tot.local_addr().unwrap().port());
    drop(tot);
    let mut k2 = chat.konfiguration.clone();
    k2.basis.clone_from(&tot_basis);
    let rust = rust_klasse(&llm::HttpChat { konfiguration: k2 }.complete(&nachrichten, None));
    let py = frage(&json!({"fn": "schritt8.llm.client", "base": tot_basis, "model": "m", "key": key, "socket_s": 0.4,
        "frist_s": 1.5, "frist_wdh_s": 1.0, "schema": false, "messages": nachrichten}));
    z.pruefe("verbindung verweigert", &rust, &py);
    let mut neg = Zaehler::default();
    neg.pruefe("negativ", &json!({"err": {"klasse": "endgueltig"}}), &py);
    println!("client: {} Vergleiche ueber {} Szenarien × 2 (mit/ohne Schema) + Verbindungsabbruch; Abweichungen {}; dokumentiert {}; Negativkontrolle {}",
        z.faelle, szenarien().len(), z.abw, z.dokumentiert, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
}

// ------------------------------------------------------------------ Klassifikator, Schemas

#[test]
fn klassifikator_und_schemas() {
    if skip() {
        return;
    }
    let mut z = Zaehler::default();
    let schemas = frage(&json!({"fn": "schritt8.llm.schemas"}));
    z.pruefe("DIALOG_SCHEMA", &llm::schema::DIALOG_SCHEMA, &schemas["dialog"]);
    z.pruefe("AUSSAGEN_SCHEMA", &llm::schema::AUSSAGEN_SCHEMA, &schemas["aussagen"]);
    z.pruefe("ZUORDNUNG_SCHEMA", &llm::schema::ZUORDNUNG_SCHEMA, &schemas["zuordnung"]);
    let mut r = Rng(0xca7e);
    let teile = ["{", "}", "\"kategorie\"", ":", "\"spende\"", "\"miete\"", "null", " ", "text ", "\"handwerker\"", "[1]", ",", "\"x\": 1", "\n"];
    let mut texte: Vec<String> = vec![r#"{"kategorie": "spende"}"#.into(), String::new(), "} {".into(), r#"x {"kategorie": "vorsorge"} y {"#.into()];
    texte.extend((0..1000).map(|_| (0..=r.n(9)).map(|_| *r.wahl(&teile)).collect::<String>()));
    let py = frage(&json!({"fn": "schritt8.llm.kategorie", "texte": texte}));
    for (i, t) in texte.iter().enumerate() {
        let rust = json!({"ok": llm::kontoauszug::parse_kategorie(t).map(llm::Kategorie::als_str)});
        // PARITAET-Abweichung (Bericht): Liste/Objekt als `kategorie` → Python `TypeError`, Rust None.
        if py[i].get("err").is_some_and(|e| e == "TypeError") && rust == json!({"ok": null}) {
            z.dokumentiert += 1;
        } else {
            z.pruefe(&format!("kategorie {t:?}"), &rust, &py[i]);
        }
    }
    for (zweck, betrag) in [("Maler DE89370400440532013000", -48000i64), ("Spende 12345678", -1), ("", -123_456_789)] {
        struct Fang(RefCell<Vec<Value>>);
        impl Chat for Fang {
            fn complete(&self, m: &[Nachricht], _: Option<&Value>) -> Result<Completion, LlmFehler> {
                *self.0.borrow_mut() = m.iter().map(|n| json!({"role": n.rolle(), "content": n.inhalt()})).collect();
                Ok(Completion::default())
            }
        }
        let f = Fang(RefCell::new(Vec::new()));
        let _ = llm::kontoauszug::klassifiziere(&f, &llm::pii::maskiere(zweck), betrag);
        let py = frage(&json!({"fn": "schritt8.llm.klassifikator", "zweck": llm::pii::maskiere(zweck).as_str(), "betrag": betrag}));
        z.pruefe(&format!("klassifikator {zweck:?}"), &json!(f.0.into_inner()), &py);
    }
    let mut neg = Zaehler::default();
    neg.pruefe("negativ", &json!({"ok": "spende"}), &json!({"ok": null}));
    println!("klassifikator+schemas: {} Vergleiche (3 Schemas, {} Kategorie-Texte, 3 Klassifikator-Nachrichten); Abweichungen {}; dokumentiert {}; Negativkontrolle {}",
        z.faelle, texte.len(), z.abw, z.dokumentiert, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
}
