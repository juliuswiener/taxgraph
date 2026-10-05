//! `rust/eingang` gegen die Antworten des PYTHON-Orakels, hermetisch: `rust/fixtures/eingang_orakel.json`
//! (eingefroren; der Erzeuger ist geloescht, Verlauf: `git show 2dd056a6:tools/parity/extract_eingang_orakel.py`;
//! Python-Seite: `produkt/eingang/*` ueber `tools/parity/schritt8_oracle.py`) haelt Eingaben und die Antwort
//! von Python; dieser Test spielt
//! dieselben Eingaben gegen die Crate und vergleicht. Ohne `PARITY=1`, ohne Python zur Laufzeit.
//!
//! Der feste Anteil dessen, was `rust/parity/tests/eingang_paritaet.rs` zufaellig zieht, plus
//! systematische Reihen an den Entscheidungsstellen: CSV-Zeilenmaschine und Feldgrenze, Betragsparser
//! (`_eur_cent_signed`, VaSt-`_cent`: Ziffernzahl, Rundung, Exponent), jedes Schluesselwort der
//! Kategorien, LLM-Deckel, PDF-Zeilen, tsv, Vorjahr, eDaten, Beleg. Die Vergleichsform je Abschnitt ist
//! die der Paritaetstests (Fehler auf Klassen normiert, Events byte-gleich samt `event_id`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::OnceLock;

use bindung::Bindung;
use eingang::kontoauszug::{self as ka, KontoauszugFehler, Transaktion};
use serde_json::{json, Value};
use store::{BindungNachschlag, Store};

const FIXTURE: &str = include_str!("../../fixtures/eingang_orakel.json");

fn fixture() -> &'static Value {
    static F: OnceLock<Value> = OnceLock::new();
    F.get_or_init(|| serde_json::from_str(FIXTURE).unwrap())
}

fn abschnitt(name: &str) -> &'static Vec<Value> {
    fixture()[name]
        .as_array()
        .unwrap_or_else(|| panic!("Abschnitt {name} fehlt"))
}

fn ts() -> &'static str {
    fixture()["ts"].as_str().unwrap()
}

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
        let reg = bindung::lade_registry(&wurzel).expect("registry");
        reg.dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn nachschlag_map() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

fn nachschlag() -> BindungNachschlag<'static> {
    BindungNachschlag::neu(nachschlag_map())
}

/// Sammelt Abweichungen; `fertig` verlangt keine und eine Mindestzahl an Vergleichen (ein Lauf ohne
/// Faelle bewiese nichts).
#[derive(Default)]
struct Abgleich {
    faelle: usize,
    abweichungen: Vec<String>,
}

impl Abgleich {
    fn pruefe(&mut self, was: &str, rust: &Value, py: &Value) {
        self.faelle += 1;
        if rust != py {
            let (r, p) = (rust.to_string(), py.to_string());
            let ab = r.chars().zip(p.chars()).take_while(|(a, b)| a == b).count();
            let ctx = |t: &str| {
                t.chars()
                    .skip(ab.saturating_sub(60))
                    .take(240)
                    .collect::<String>()
            };
            self.abweichungen.push(format!(
                "{}\n  rust: ...{}\n  py:   ...{}",
                was.chars().take(200).collect::<String>(),
                ctx(&r),
                ctx(&p)
            ));
        }
    }

    fn fertig(self, mindestens: usize) {
        assert!(
            self.faelle >= mindestens,
            "nur {} Vergleiche, erwartet >= {mindestens}",
            self.faelle
        );
        assert!(
            self.abweichungen.is_empty(),
            "{} von {} weichen ab, die ersten:\n{}",
            self.abweichungen.len(),
            self.faelle,
            self.abweichungen
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

// ------------------------------------------------------------------ Adapter (wie eingang_paritaet.rs)

fn tx_json(t: &Transaktion) -> Value {
    json!({"datum": t.datum, "betrag": t.betrag, "verwendungszweck": t.verwendungszweck})
}

fn events(s: &Store) -> Value {
    serde_json::to_value(s.events()).unwrap()
}

/// Rust-Fehler in Pythons Fehlerform.
fn konto_fehler(e: &KontoauszugFehler) -> Value {
    match e {
        KontoauszugFehler::Csv(c) => json!({"err": "Error", "msg": c.to_string()}),
        KontoauszugFehler::BetragUeberlauf(_) => json!({"err": "OverflowError"}),
        _ => json!({"err": true}),
    }
}

fn py_fehler_normiert(v: &Value) -> Value {
    match v.get("err").and_then(Value::as_str) {
        Some("OverflowError") => json!({"err": "OverflowError"}),
        Some(e) if e != "Error" => json!({"err": true}),
        _ => v.clone(),
    }
}

fn kurz(v: &Value) -> Value {
    match v {
        Value::String(s) if s.chars().count() > 100 => {
            json!({"len": s.chars().count(), "kopf": s.chars().take(20).collect::<String>()})
        }
        Value::Array(a) => Value::Array(a.iter().map(kurz).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), kurz(x))).collect()),
        andere => andere.clone(),
    }
}

fn csv_ergebnis(text: &str) -> Value {
    match ka::parse_csv(text) {
        Ok((v, n)) => json!({"ok": [v.iter().map(tx_json).collect::<Vec<_>>(), n]}),
        Err(e) => konto_fehler(&e),
    }
}

fn rust_konto(tx: Result<Vec<Transaktion>, KontoauszugFehler>, llm: Option<&[String]>) -> Value {
    let tx = match tx {
        Ok(t) => t,
        Err(e) => return json!({"r": konto_fehler(&e), "events": []}),
    };
    let mut s = Store::leer(2025, None);
    let katalog = store::Katalog::aus_bindungen(bindungen());
    let texte = RefCell::new(
        llm.map(|l| l.iter().cloned().collect::<VecDeque<_>>())
            .unwrap_or_default(),
    );
    let k = |z: &llm::pii::Maskiert, b: i64| {
        let _ = (z, b);
        llm::kontoauszug::parse_kategorie(&texte.borrow_mut().pop_front().unwrap_or_default())
    };
    let r = ka::uebernehme(
        &mut s,
        &tx,
        nachschlag(),
        llm.map(|_| &k as &ka::Klassifikator<'_>),
        Some(ts()),
        Some(&katalog),
    );
    let r = match r {
        Ok(u) => json!({"ok": [u.uebernommen, u.llm_uebersprungen]}),
        Err(e) => konto_fehler(&e),
    };
    json!({"r": r, "events": events(&s)})
}

fn transaktionen(liste: &Value) -> Vec<Transaktion> {
    liste
        .as_array()
        .unwrap()
        .iter()
        .map(|t| Transaktion {
            datum: t["datum"].clone(),
            betrag: t["betrag"].as_i64().unwrap(),
            verwendungszweck: t["verwendungszweck"].as_str().unwrap().to_owned(),
        })
        .collect()
}

fn llm_texte(v: &Value) -> Option<Vec<String>> {
    v.as_array()
        .map(|a| a.iter().map(|t| t.as_str().unwrap().to_owned()).collect())
}

// ------------------------------------------------------------------ kontoauszug

#[test]
fn csv_wie_orakel() {
    let mut z = Abgleich::default();
    for f in abschnitt("csv") {
        let t = f["text"].as_str().unwrap();
        z.pruefe(
            &format!("parse_csv {t:?}"),
            &csv_ergebnis(t),
            &py_fehler_normiert(&f["py"]),
        );
    }
    z.fertig(600);
}

fn csv_fehler_text(f: Option<&eingang::csv::CsvFehler>) -> Value {
    f.map_or(Value::Null, |e| json!(e.to_string()))
}

/// Die Zeilenmaschine direkt gegen `csv.reader`: alle Folgen bis Laenge 5 ueber {a ; " CR LF} und
/// Handfaelle, Datensaetze samt Leerzeilen (`[]`) und der Fehler, der sie beendet.
#[test]
fn csv_zeilenmaschine_wie_orakel() {
    let mut z = Abgleich::default();
    for f in abschnitt("csv_roh") {
        let t = f[0].as_str().unwrap();
        let (rows, err) = eingang::csv::lies_datensaetze(t, ';');
        z.pruefe(
            &format!("lies_datensaetze {t:?}"),
            &json!([rows, csv_fehler_text(err.as_ref())]),
            &json!([f[1], f[2]]),
        );
    }
    for f in abschnitt("csv_roh_tab") {
        let t = f[0].as_str().unwrap();
        let (rows, err) = eingang::csv::lies_datensaetze(t, '\t');
        z.pruefe(
            &format!("lies_datensaetze tab {t:?}"),
            &json!([rows, csv_fehler_text(err.as_ref())]),
            &json!([f[1], f[2]]),
        );
    }
    z.fertig(3900);
}

#[test]
fn csv_dict_reader_wie_orakel() {
    let mut z = Abgleich::default();
    for f in abschnitt("csv_dict") {
        let t = f[0].as_str().unwrap();
        let (kopf, zeilen, err) = eingang::csv::dict_reader(t, ';');
        let zeilen: Vec<Value> = zeilen.iter().map(|r| json!(r)).collect();
        z.pruefe(
            &format!("dict_reader {t:?}"),
            &json!([kopf, zeilen, csv_fehler_text(err.as_ref())]),
            &json!([f[1], f[2], f[3]]),
        );
    }
    z.fertig(20);
}

#[test]
fn csv_feldgrenze_wie_orakel() {
    let mut z = Abgleich::default();
    for f in abschnitt("csv_gross") {
        let text = format!(
            "{}{}{}",
            f["praefix"].as_str().unwrap(),
            f["fuell"]
                .as_str()
                .unwrap()
                .repeat(usize::try_from(f["n"].as_u64().unwrap()).unwrap()),
            f["suffix"].as_str().unwrap()
        );
        z.pruefe(
            &format!("csv_gross n={}", f["n"]),
            &kurz(&csv_ergebnis(&text)),
            &kurz(&py_fehler_normiert(&f["py"])),
        );
    }
    z.fertig(12);
}

#[test]
fn betragsparser_wie_orakel() {
    let mut z = Abgleich::default();
    for f in abschnitt("cent") {
        let b = f["wert"].as_str().unwrap();
        z.pruefe(
            &format!("eur_cent {b:?}"),
            &json!({"ok": ka::eur_cent_signed(b)}),
            &py_fehler_normiert(&f["py"]),
        );
    }
    z.fertig(200);
}

#[test]
fn uebernahme_aus_csv_wie_orakel() {
    let mut z = Abgleich::default();
    let mut events_n = 0;
    for f in abschnitt("konto") {
        let llm = llm_texte(&f["llm"]);
        let rust = rust_konto(Ok(transaktionen(&f["tx"])), llm.as_deref());
        events_n += rust["events"].as_array().unwrap().len();
        z.pruefe(
            &format!("uebernehme {}", f["tx"]),
            &rust,
            &json!({"r": py_fehler_normiert(&f["py"]["r"]), "events": f["py"]["events"]}),
        );
    }
    assert!(events_n >= 50, "nur {events_n} Events verglichen");
    z.fertig(200);
}

#[test]
fn uebernahme_aus_json_wie_orakel() {
    let mut z = Abgleich::default();
    for f in abschnitt("konto_json") {
        let roh = &f["roh"];
        let rust = rust_konto(ka::aus_json(roh), None);
        let py_r = py_fehler_normiert(&f["py"]["r"]);
        // Bei einem Fehler verwirft die API den Fall ungespeichert; nur die Klasse zaehlt.
        let (rust, py) = if py_r.get("err").is_some() {
            (json!({"r": rust["r"]}), json!({"r": py_r}))
        } else {
            (rust, json!({"r": py_r, "events": f["py"]["events"]}))
        };
        z.pruefe(&format!("uebernehme json {roh}"), &rust, &py);
    }
    z.fertig(150);
}

#[test]
fn schluesselwoerter_deckel_und_llm_wie_orakel() {
    let mut z = Abgleich::default();
    let mut mit_event = 0;
    for f in abschnitt("konto_stichwort") {
        let llm = llm_texte(&f["llm"]);
        let rust = rust_konto(Ok(transaktionen(&f["tx"])), llm.as_deref());
        mit_event += usize::from(!rust["events"].as_array().unwrap().is_empty());
        z.pruefe(
            &format!("stichwort {}", f["tx"]),
            &rust,
            &json!({"r": py_fehler_normiert(&f["py"]["r"]), "events": f["py"]["events"]}),
        );
    }
    assert!(mit_event >= 100, "nur {mit_event} Faelle mit Event");
    z.fertig(150);
}

/// `verwirf_unlesbare_betraege` auf JSON-Elementen und auf Transaktionen, `pruefe_buchungsfelder` und
/// `hinweis_verworfen` gegen die Python-Funktionen.
#[test]
fn verwerfen_und_pruefen_der_buchungen_wie_orakel() {
    let mut z = Abgleich::default();
    let mut getypt = 0;
    for f in abschnitt("verwirf") {
        let liste = f["liste"].as_array().unwrap();
        let n = usize::try_from(f["n"].as_u64().unwrap()).unwrap();
        let (ok, n2) = ka::verwirf_unlesbare_betraege_json(liste, n);
        z.pruefe(
            &format!("verwirf {liste:?} {n}"),
            &json!({"ok": [ok, n2]}),
            &f["py"],
        );
        // Dieselbe Frage fuer Buchungen aus CSV/PDF (der Betrag ist dort schon eine Zahl in i64).
        let tx: Option<Vec<Transaktion>> = liste
            .iter()
            .map(|t| {
                Some(Transaktion {
                    datum: Value::Null,
                    betrag: t.get("betrag")?.as_i64()?,
                    verwendungszweck: String::new(),
                })
            })
            .collect();
        if let Some(tx) = tx.filter(|t| !t.is_empty()) {
            let (ok, n2) = ka::verwirf_unlesbare_betraege(tx, n);
            let betraege: Vec<i64> = ok.iter().map(|t| t.betrag).collect();
            let py_betraege: Vec<Value> = f["py"]["ok"][0].as_array().unwrap().clone();
            let py_betraege: Vec<i64> = py_betraege
                .iter()
                .map(|t| t["betrag"].as_i64().unwrap())
                .collect();
            z.pruefe(
                &format!("verwirf typisiert {liste:?}"),
                &json!([betraege, n2]),
                &json!([py_betraege, f["py"]["ok"][1]]),
            );
            getypt += 1;
        }
    }
    assert!(getypt >= 6, "nur {getypt} typisierte Faelle");
    for f in abschnitt("buchungsfelder") {
        // Python wirft bei einem Betrag, den `int()` nicht liest, schon beim Lesen (TypeError):
        // diesen Fall faengt die Route vorher mit `verwirf_unlesbare_betraege` ab.
        if f["py"].get("err").is_some_and(|e| e != "ValueError") {
            continue;
        }
        let rust = ka::pruefe_buchungsfelder(f["liste"].as_array().unwrap(), None).map_or_else(
            |m| json!({"err": "ValueError", "msg": m}),
            |()| json!({"ok": null}),
        );
        z.pruefe(
            &format!("pruefe_buchungsfelder {}", f["liste"]),
            &rust,
            &f["py"],
        );
    }
    for f in abschnitt("buchungsfelder_daten") {
        let rust = ka::pruefe_buchungsfelder(f["rust"].as_array().unwrap(), Some("#")).map_or_else(
            |m| json!({"err": "ValueError", "msg": m}),
            |()| json!({"ok": null}),
        );
        z.pruefe(
            &format!("pruefe_buchungsfelder daten {}", f["rust"]),
            &rust,
            &f["py"],
        );
    }
    for f in abschnitt("hinweis") {
        let rust = ka::hinweis_verworfen(
            usize::try_from(f["n"].as_u64().unwrap()).unwrap(),
            f["fmt"].as_str().unwrap(),
        );
        z.pruefe(
            &format!("hinweis {} {}", f["n"], f["fmt"]),
            &json!(rust),
            &f["py"],
        );
    }
    z.fertig(150);
}

/// Python liest den Betrag eines JSON-Elements als `int(tx.get("betrag", 0))` (`uebernehme_kontoauszug`):
/// ohne Schluessel gilt 0. Die Listen aus `aus_json` sind nur hier sichtbar, die Uebernahme laesst
/// Buchungen ab 0 aus.
#[test]
fn aus_json_liest_den_betrag_wie_int_von_get_mit_null_als_vorgabe() {
    let tx = ka::aus_json(&json!([
        {"verwendungszweck": "ohne Betrag"},
        {"betrag": 0, "verwendungszweck": 5},
        {"betrag": "-7", "verwendungszweck": "Spende"},
        {"betrag": true},
        {"betrag": -0.9}
    ]))
    .unwrap();
    let betraege: Vec<i64> = tx.iter().map(|t| t.betrag).collect();
    assert_eq!(betraege, [0, 0, -7, 1, 0]);
}

#[test]
fn pdf_zeilen_wie_orakel() {
    let mut z = Abgleich::default();
    for f in abschnitt("pdf_zeilen") {
        let conf: eingang::ocr::ConfMap = f["conf"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), v.as_f64().unwrap()))
            .collect();
        let (t, n) = ka::parse_pdf_zeilen(f["text"].as_str().unwrap(), &conf, 0.6);
        let rust = json!({"ok": [t.iter().map(tx_json).collect::<Vec<_>>(), n]});
        z.pruefe(
            &format!("pdf_zeilen {}", f["text"]),
            &rust,
            &py_fehler_normiert(&f["py"]),
        );
    }
    z.fertig(300);
}

#[test]
fn tsv_wie_orakel() {
    let mut z = Abgleich::default();
    for f in abschnitt("tsv") {
        let t = f["text"].as_str().unwrap();
        let rust = eingang::ocr::tsv_zu_zeilen(t).map_or_else(
            |e| json!({"err": "Error", "msg": e.to_string()}),
            |v| json!({"ok": v}),
        );
        z.pruefe(&format!("tsv {t:?}"), &rust, &f["py"]);
    }
    z.fertig(150);
}

// ------------------------------------------------------------------ VaSt, Vorjahr, eDaten

fn vast_fehler(e: &eingang::vast::VastFehler) -> Value {
    match e {
        eingang::vast::VastFehler::NichtLesbar(_) => {
            json!({"err": "ValueError", "msg": e.to_string()})
        }
        eingang::vast::VastFehler::Ueberlauf(_) => json!({"err": "OverflowError"}),
    }
}

fn vast_py_norm(v: &Value) -> Value {
    if v.get("err").is_some_and(|e| e != "ValueError") {
        json!({"err": "OverflowError"})
    } else {
        v.clone()
    }
}

/// BEKANNTE ABWEICHUNG (Befund aus h8-hermetisch5, `gueltig` in `vast.rs`): Pythons `Decimal`
/// streicht Unterstriche an jeder Stelle (`"1__2"` -> 12, `"1_"` -> 1, `"_1"` -> 1, `"1e1_0"` -> 1e10);
/// Rust lehnt diese vier Formen als `NichtLesbar` ab und nimmt nur `"1_000"` (ein Unterstrich
/// zwischen Ziffern). VaSt-XML traegt keine Unterstriche; die Abweichung ist ohne Folge im Betrieb. Die
/// Faelle stehen hier mit dem heutigen Rust-Verhalten fest, statt aus dem Vergleich zu fallen: wer
/// sie angleicht, aendert diese Liste. Sie gelten nur, solange Rust von Python abweicht.
const UNTERSTRICH_ABWEICHUNG: [&str; 4] = ["1__2", "1_", "_1", "1e1_0"];

#[test]
fn vast_betraege_wie_orakel() {
    let v = &fixture()["vast"];
    let py = &v["py"];
    let mut z = Abgleich::default();
    let mut abweichend = 0;
    for (i, b) in v["werte"].as_array().unwrap().iter().enumerate() {
        let rust =
            eingang::vast::cent(b.as_str()).map_or_else(|e| vast_fehler(&e), |c| json!({"ok": c}));
        if b.as_str()
            .is_some_and(|t| UNTERSTRICH_ABWEICHUNG.contains(&t))
        {
            let schlecht = json!({"err": "ValueError", "msg": format!("VaSt-Betrag nicht lesbar: '{}'", b.as_str().unwrap())});
            z.pruefe(
                &format!("vast cent {b} (bekannte Abweichung)"),
                &rust,
                &schlecht,
            );
            assert_ne!(
                rust,
                vast_py_norm(&py["cent"][i]),
                "{b}: Rust stimmt jetzt mit Python ueberein, Liste kuerzen"
            );
            abweichend += 1;
            continue;
        }
        z.pruefe(
            &format!("vast cent {b}"),
            &rust,
            &vast_py_norm(&py["cent"][i]),
        );
    }
    assert_eq!(abweichend, UNTERSTRICH_ABWEICHUNG.len());
    for (i, w) in v["lstb"].as_array().unwrap().iter().enumerate() {
        let werte: BTreeMap<String, String> = w
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, x)| (k.clone(), x.as_str().unwrap_or_default().to_owned()))
            .collect();
        let rust =
            eingang::vast::aus_lstb(&werte).map_or_else(|e| vast_fehler(&e), |s| json!({"ok": s}));
        z.pruefe(
            &format!("aus_lstb {w}"),
            &rust,
            &vast_py_norm(&py["lstb"][i]),
        );
    }
    for (i, l) in v["lersl"].as_array().unwrap().iter().enumerate() {
        let ls: Vec<eingang::vast::Leistung> = serde_json::from_value(l.clone()).unwrap();
        let rust =
            eingang::vast::aus_lersl(&ls).map_or_else(|e| vast_fehler(&e), |s| json!({"ok": s}));
        z.pruefe(
            &format!("aus_lersl {l}"),
            &rust,
            &vast_py_norm(&py["lersl"][i]),
        );
    }
    z.fertig(500);
}

fn sortiert(v: &Value) -> Value {
    let mut a = v.as_array().cloned().unwrap_or_default();
    a.sort_by_key(|e| format!("{}{}", e["feld_id"], e["event_id"]));
    Value::Array(a)
}

fn vorbelegen(s: &mut Store, fid: &str) {
    let leer = HashMap::new();
    let neu = store::NeuesEvent {
        feld_id: fid.to_owned(),
        wert: domain::PyWert::Ganz(1),
        feldzustand: domain::Feldzustand::Vorlaeufig,
        herkunft: domain::Herkunft {
            herkunft: domain::Achsenwert::new("laie").unwrap(),
            pruef_tiefe: domain::PruefTiefe::Ungeprueft,
            haftung: domain::Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: "ui:laie".parse().unwrap(),
        signal_1: None,
        ersetzt: None,
        ts: Some(ts().into()),
    };
    s.append(&neu, None, BindungNachschlag::neu(&leer)).unwrap();
}

#[test]
fn vorjahr_wie_orakel() {
    let mut z = Abgleich::default();
    let mut mit_uebersprungen = 0;
    let mut mit_referenz = 0;
    for f in abschnitt("vorjahr") {
        let req = &f["req"];
        let py = &f["py"];
        let mut s = Store::leer(2026, None);
        for fid in req["vorbelegt"].as_array().unwrap() {
            vorbelegen(&mut s, fid.as_str().unwrap());
        }
        let vf: BTreeMap<String, eingang::vorjahr::VorjahrFeld> =
            serde_json::from_value(req["felder"].clone()).unwrap();
        let vz = req["vz"].as_i64().unwrap();
        let rust = match eingang::vorjahr::uebernehme(&mut s, &vf, nachschlag(), vz, Some(ts())) {
            Ok(e) => {
                mit_uebersprungen += usize::from(!e.uebersprungen.is_empty());
                mit_referenz += usize::from(e.referenz.is_some());
                json!({"r": {"ok": [e.uebertragen, e.uebersprungen]}, "referenz": e.referenz})
            }
            Err(_) => json!({"r": {"err": true}}),
        };
        let py_r = if py["r"].get("err").is_some() {
            json!({"err": true})
        } else {
            py["r"].clone()
        };
        // Jede andere Abweisung bricht in Python mitten in der Schleife ab (andere Reihenfolge als
        // Rust); dann zaehlt nur die Klasse.
        if py_r.get("err").is_some() {
            z.pruefe(&format!("vorjahr {}", req["felder"]), &rust["r"], &py_r);
        } else {
            z.pruefe(
                &format!("vorjahr {}", req["felder"]),
                &json!({"r": rust["r"], "referenz": rust["referenz"], "events": sortiert(&events(&s))}),
                &json!({"r": py_r, "referenz": py["referenz"], "events": sortiert(&py["events"])}),
            );
        }
    }
    assert!(
        mit_uebersprungen > 0,
        "kein Fall mit uebersprungenem Altwert"
    );
    assert!(mit_referenz > 0, "kein Fall mit Referenzwert");
    z.fertig(120);
}

#[test]
fn edaten_wie_orakel() {
    let mut z = Abgleich::default();
    let mut mit_event = 0;
    for f in abschnitt("edaten") {
        let req = &f["req"];
        let py = &f["py"];
        let mit_bindung = req["mit_bindung"].as_bool().unwrap();
        let mut s = Store::leer(2025, None);
        // Eigene Angaben (vorbelegt) haben Vorrang vor eDaten.
        for fid in req["vorbelegt"].as_array().into_iter().flatten() {
            vorbelegen(&mut s, fid.as_str().unwrap());
        }
        let sz: Vec<eingang::edaten::Satz> = serde_json::from_value(req["saetze"].clone()).unwrap();
        let rust = match eingang::edaten::uebernehme(
            &mut s,
            &sz,
            Some(ts()),
            mit_bindung.then(nachschlag),
        ) {
            Ok(n) => json!({"ok": n}),
            Err(_) => json!({"err": true}),
        };
        let py_r = if py["r"].get("err").is_some() {
            json!({"err": true})
        } else {
            py["r"].clone()
        };
        mit_event += usize::from(!events(&s).as_array().unwrap().is_empty());
        z.pruefe(
            &format!("edaten {} bindung={mit_bindung}", req["saetze"]),
            &json!({"r": rust, "events": events(&s)}),
            &json!({"r": py_r, "events": py["events"]}),
        );
    }
    assert!(mit_event >= 30, "nur {mit_event} Faelle mit Event");
    z.fertig(150);
}

// ------------------------------------------------------------------ Beleg

fn rust_beleg(text: &str, conf: &BTreeMap<String, f64>, schreibe: bool) -> Value {
    let k = eingang::beleg::extrahiere(text, nachschlag(), conf);
    let mut out = json!({"kandidaten": {"ok": k}});
    if schreibe {
        let mut s = Store::leer(2025, None);
        out["schreibe"] = match eingang::beleg::schreibe_kandidaten(
            &mut s,
            &k,
            "upload-1",
            nachschlag(),
            Some(ts()),
        ) {
            Ok(v) => json!({"ok": v.len()}),
            Err(_) => json!({"err": true}),
        };
        out["events"] = events(&s);
    }
    out
}

#[test]
fn beleg_wie_orakel() {
    let mut z = Abgleich::default();
    let mut kandidaten_n = 0;
    for f in abschnitt("beleg") {
        let req = &f["req"];
        let conf: BTreeMap<String, f64> = req["conf"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_f64().unwrap()))
            .collect();
        // `text_aus` = Rezept [Anfang, Einheit, n, Ende] fuer eine ueberlange Zeile (spart 400 KB im Fixture).
        let text = match req.get("text_aus").and_then(Value::as_array) {
            Some(r) => format!(
                "{}{}{}",
                r[0].as_str().unwrap(),
                r[1].as_str()
                    .unwrap()
                    .repeat(usize::try_from(r[2].as_u64().unwrap()).unwrap()),
                r[3].as_str().unwrap()
            ),
            None => req["text"].as_str().unwrap().to_owned(),
        };
        let rust = rust_beleg(&text, &conf, req["schreibe"].as_bool().unwrap());
        kandidaten_n += rust["kandidaten"]["ok"].as_array().unwrap().len();
        let mut py = f["py"].clone();
        if py.get("schreibe").and_then(|s| s.get("err")).is_some() {
            py["schreibe"] = json!({"err": true});
        }
        z.pruefe(
            &format!("beleg {:?}", text.chars().take(80).collect::<String>()),
            &rust,
            &py,
        );
    }
    assert!(kandidaten_n >= 100, "nur {kandidaten_n} Kandidaten");
    z.fertig(150);
}
