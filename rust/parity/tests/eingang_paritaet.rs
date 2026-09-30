//! Paritaet `rust/eingang` gegen `produkt/eingang/*` (`tools/parity/schritt8_oracle.py`).
//!
//! - `kontoauszug`: CSV-Korpus aus den Python-Tests + 1000 generierte Auszuege (CSV, davon ein
//!   Teil mit LLM-Rueckfall) + 300 JSON-Auszuege → Transaktionen, `(uebernommen, uebersprungen)`,
//!   Store-Events byte-gleich (inkl. `event_id`); dazu Betrag-Parser, PDF-Zeilen, tesseract-TSV.
//! - `pdf_ocr`: selbst erzeugte PDFs (Textlayer, Bild-Scan, gemischt, 41 Bildseiten) durch
//!   beide `lies_*`-Pfade (echte Unterprozesse `pdftotext`/`pdftoppm`/`tesseract`).
//! - `beleg`: Fixture-Texte aus `tests/fixtures/` + 500 generierte → Kandidaten + Events.
//! - `vorjahr_vast_edaten`: 500 Vorjahres-Faelle, Betraege/LStB/LErsL, 500 eDaten-Laeufe.
//!
//!   `PARITY`=1 `cargo` test -p parity --test `eingang_paritaet` -- --nocapture
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt::Write as _;
use std::sync::{Mutex, OnceLock};

use bindung::Bindung;
use eingang::kontoauszug::{self as ka, KontoauszugFehler, Transaktion};
use parity::Oracle;
use serde_json::{json, Value};
use store::{BindungNachschlag, Store};

const TS: &str = "2026-09-29T00:00:00+00:00";

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
    let a = oracle()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .call_json(anfrage)
        .expect("orakel antwortet");
    a.get("ok")
        .cloned()
        .unwrap_or_else(|| panic!("Orakel-Fehler: {a}"))
}

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let reg = bindung::lade_registry(&repo_root().join("produkt/bindung")).expect("registry");
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

#[derive(Default)]
struct Zaehler {
    faelle: usize,
    abw: usize,
    dokumentiert: usize,
}
impl Zaehler {
    fn pruefe(&mut self, was: &str, rust: &Value, py: &Value) {
        self.faelle += 1;
        if rust != py {
            self.abw += 1;
            if self.abw <= 5 {
                let r = rust.to_string();
                let p = py.to_string();
                let ab = r.chars().zip(p.chars()).take_while(|(a, b)| a == b).count();
                let ctx = |t: &str| {
                    t.chars()
                        .skip(ab.saturating_sub(60))
                        .take(300)
                        .collect::<String>()
                };
                eprintln!(
                    "ABWEICHUNG {was}\n  rust: …{}\n  py:   …{}",
                    ctx(&r),
                    ctx(&p)
                );
            }
        }
    }
}

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

// ------------------------------------------------------------------ Generatoren

const ZWECKE: &[&str] = &[
    "Malermeister Huber Rechnung 12",
    "SANITÄR Müller",
    "Spende Rotes Kreuz",
    "Tierheim e.V.",
    "Miete Oktober",
    "Gebäudereinigung GmbH",
    "Minijob-Zentrale",
    "Rürup-Rente Beitrag",
    "Amazon",
    "Unicef IBAN DE89370400440532013000",
    "Stadtwerke Kto 12345678901",
    "Heizung Wartung",
    "Gartenpflege Schmidt",
    "",
    "Supermarkt",
    "Hilfswerk",
    "Saldo alt",
    "e. V. Verein",
    "Winterdienst",
    "Klempner Notdienst \"24h\"",
    "Zahlung; Rest",
    "StNr 181/815/08155",
];
const BETRAEGE: &[&str] = &[
    "-480,00",
    "-1.234,56",
    "1.234,56",
    "-12.5",
    "abc",
    "",
    "  -5,00 € ",
    "+-3",
    "--7",
    "-1e3",
    "inf",
    "nan",
    "-0,005",
    "-0,015",
    "-99",
    "12",
    "-1_000,5",
    "-1.000.000,00",
    "-0,00",
    "−5,00",
    "-١٢,٠٠",
    "-2,675",
    "-𝟏𝟐,𝟓",
    "-١_٠٠٠,٥",
];

fn csv_zelle(r: &mut Rng, s: &str) -> String {
    if s.contains(';') || s.contains('"') || r.p(15) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

fn csv_auszug(r: &mut Rng) -> String {
    let koepfe: &[&[&str]] = &[
        &["Buchungstag", "Betrag", "Verwendungszweck"],
        &["datum", "betrag", "zweck"],
        &[" DATE ", "Amount", "Description"],
        &["Buchungsdatum", "Umsatz", "Buchungstext", "Saldo"],
        &["Datum", "Betrag (EUR)", "Beschreibung"],
        &["x", "y"],
        &["Betrag", "betrag", "Zweck"],
        &["Datum", "Verwendungszweck"],
    ];
    let kopf = r.wahl(koepfe);
    let ende = *r.wahl(&["\n", "\r\n", "\n", "\n"]);
    let mut out = kopf
        .iter()
        .map(|k| csv_zelle(r, k))
        .collect::<Vec<_>>()
        .join(";");
    for _ in 0..r.n(8) {
        out.push_str(ende);
        if r.p(8) {
            continue;
        }
        let mut zellen = vec![
            "01.03.2025".to_owned(),
            (*r.wahl(BETRAEGE)).to_owned(),
            (*r.wahl(ZWECKE)).to_owned(),
        ];
        if r.p(10) {
            zellen.pop();
        }
        if r.p(10) {
            zellen.push("extra".into());
        }
        out.push_str(
            &zellen
                .iter()
                .map(|z| csv_zelle(r, z))
                .collect::<Vec<_>>()
                .join(";"),
        );
    }
    if r.p(3) {
        out.push_str("a\rb");
    }
    if r.p(3) {
        out.push_str("\n\"offen;quote");
    }
    if r.p(50) {
        out.push_str(ende);
    }
    out
}

fn json_auszug(r: &mut Rng) -> Value {
    let werte = [
        json!(-48000),
        json!("-500"),
        json!(-12.7),
        json!(true),
        json!("1,5"),
        Value::Null,
        json!(-5),
        json!(" -7 "),
        json!([1]),
    ];
    Value::Array(
        (0..r.n(6))
            .map(|_| {
                if r.p(3) {
                    return json!("kein objekt");
                }
                let mut o = serde_json::Map::new();
                if r.p(90) {
                    o.insert("betrag".into(), r.wahl(&werte).clone());
                }
                if r.p(90) {
                    o.insert("verwendungszweck".into(), json!(r.wahl(ZWECKE)));
                }
                if r.p(70) {
                    o.insert(
                        "datum".into(),
                        if r.p(80) {
                            json!("2025-03-01")
                        } else {
                            json!(20_250_301)
                        },
                    );
                }
                Value::Object(o)
            })
            .collect(),
    )
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
        Some(TS),
        Some(&katalog),
    );
    let r = match r {
        Ok(u) => json!({"ok": [u.uebernommen, u.llm_uebersprungen]}),
        Err(e) => konto_fehler(&e),
    };
    json!({"r": r, "events": events(&s)})
}

#[test]
fn kontoauszug() {
    if skip() {
        return;
    }
    let mut z = Zaehler::default();
    let mut r = Rng(0x0c5b_0001);
    let korpus: Vec<String> = serde_json::from_value(frage(&json!({"fn": "schritt8.korpus",
        "dateien": ["tests/test_kontoauszug_writer.py", "tests/test_llm_deckel_und_wiederholung.py", "tests/test_kontoauszug_maskierung.py"]}))).unwrap();
    let mut csvs: Vec<String> = korpus.clone();
    csvs.extend((0..1000).map(|_| csv_auszug(&mut r)));
    let py_csv = frage(&json!({"fn": "schritt8.eingang.csv", "texte": csvs}));
    let mut mit_tx = 0;
    for (i, t) in csvs.iter().enumerate() {
        let rust = match ka::parse_csv(t) {
            Ok(v) => {
                mit_tx += usize::from(!v.is_empty());
                json!({"ok": v.iter().map(tx_json).collect::<Vec<_>>()})
            }
            Err(e) => konto_fehler(&e),
        };
        z.pruefe(
            &format!("parse_csv {t:?}"),
            &rust,
            &py_fehler_normiert(&py_csv[i]),
        );
    }
    // Uebernahme: die generierten CSV-Auszuege (Korpus-Strings sind meist keine), jeder dritte mit LLM.
    let llm_texte = [
        r#"{"kategorie": "spende"}"#,
        r#"{"kategorie": "handwerker"}"#,
        "unklar",
        r#"{"kategorie": "miete"}"#,
        "",
        r#"x {"kategorie": "vorsorge"}"#,
    ];
    let mut events_n = 0;
    for (i, t) in csvs.iter().enumerate().skip(korpus.len()) {
        let llm: Option<Vec<String>> = (i % 3 == 0).then(|| {
            (0..r.n(6))
                .map(|_| (*r.wahl(&llm_texte)).to_owned())
                .collect()
        });
        let Ok(tx) = ka::parse_csv(t) else { continue };
        let py = frage(
            &json!({"fn": "schritt8.eingang.konto", "tx": tx.iter().map(tx_json).collect::<Vec<_>>(), "ts": TS, "llm": llm}),
        );
        let rust = rust_konto(Ok(tx), llm.as_deref());
        events_n += rust["events"].as_array().unwrap().len();
        z.pruefe(
            &format!("uebernehme csv {t:?}"),
            &rust,
            &json!({"r": py_fehler_normiert(&py["r"]), "events": py["events"]}),
        );
    }
    for _ in 0..300 {
        let roh = json_auszug(&mut r);
        let py = frage(&json!({"fn": "schritt8.eingang.konto", "tx": roh, "ts": TS, "llm": null}));
        let rust = rust_konto(ka::aus_json(&roh), None);
        let py_r = py_fehler_normiert(&py["r"]);
        // Bei einem Fehler verwirft die API den Fall ungespeichert; nur die Klasse zaehlt.
        let (rust, py) = if py_r.get("err").is_some() {
            (json!({"r": rust["r"]}), json!({"r": py_r}))
        } else {
            (rust, json!({"r": py_r, "events": py["events"]}))
        };
        z.pruefe(&format!("uebernehme json {roh}"), &rust, &py);
    }
    // Betrag-Parser, PDF-Zeilen, TSV.
    let mut betraege: Vec<String> = BETRAEGE.iter().map(|s| (*s).to_owned()).collect();
    betraege.extend((0..1000).map(|_| {
        format!(
            "{}{}{}",
            r.wahl(&["", "-", "+", " -"]),
            r.n(100_000),
            r.wahl(&[",00", ".5", ",5", ",055", "", ".000,99", "e2", "_5"])
        )
    }));
    let py_b = frage(&json!({"fn": "schritt8.eingang.cent", "werte": betraege}));
    for (i, b) in betraege.iter().enumerate() {
        let rust = ka::eur_cent_signed(b).map_or_else(|e| konto_fehler(&e), |c| json!({"ok": c}));
        z.pruefe(
            &format!("eur_cent {b:?}"),
            &rust,
            &py_fehler_normiert(&py_b[i]),
        );
    }
    let zeilen = [
        "01.03.2025 Maler Huber -480,00 EUR",
        "01.03.2025 Saldo -1,00",
        "Tagessaldo 1.234,56",
        "02.03.2025 Miete -800,00 -1.200,00",
        "03.03.2025 Spende 50,00",
        "04.03.2025 ohne Betrag",
        "Zwischensumme -5,00",
        "05.03.2025\tKlempner  -120,00 €",
        "1.3.2025 x -1,00",
        "06.03.2025 Sollzinsen 3,50% -45,00",
        "  ",
        "07.03.2025 Rechnungssumme -99,99 eur",
        "08.03.2025 X -9,99\x0c09.03.2025 Y -1,00",
    ];
    let faelle: Vec<Value> = (0..500)
        .map(|_| {
            let text = (0..r.n(10))
                .map(|_| *r.wahl(&zeilen))
                .collect::<Vec<_>>()
                .join(r.wahl(&["\n", "\r\n", "\x0b", "\u{2028}"]));
            let conf: BTreeMap<String, f64> = (0..r.n(4))
                .map(|_| {
                    (
                        r.n(10).to_string(),
                        f64::from(u32::try_from(r.n(100)).unwrap()) / 100.0,
                    )
                })
                .collect();
            json!({"text": text, "conf": conf})
        })
        .collect();
    let py_p = frage(&json!({"fn": "schritt8.eingang.pdf_zeilen", "faelle": faelle}));
    for (i, f) in faelle.iter().enumerate() {
        let conf: eingang::ocr::ConfMap = f["conf"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), v.as_f64().unwrap()))
            .collect();
        let rust = match ka::parse_pdf_zeilen(f["text"].as_str().unwrap(), &conf, 0.6) {
            Ok((t, n)) => json!({"ok": [t.iter().map(tx_json).collect::<Vec<_>>(), n]}),
            Err(e) => konto_fehler(&e),
        };
        z.pruefe(
            &format!("pdf_zeilen {f}"),
            &rust,
            &py_fehler_normiert(&py_p[i]),
        );
    }
    let tsvs: Vec<String> = (0..200).map(|_| {
        let mut t = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n".to_owned();
        for _ in 0..r.n(12) {
            let _ = writeln!(t, "{}\t1\t{}\t1\t{}\t1\t0\t0\t1\t1\t{}\t{}", r.wahl(&["5", "4", "5"]), r.n(2), r.n(3),
                r.wahl(&["96.5", "-1", "80", "x", "59.9"]), r.wahl(&["01.03.2025", "Maler", "-480,00", " ", "\"q", "EUR"]));
        }
        t
    }).collect();
    let py_t = frage(&json!({"fn": "schritt8.eingang.tsv", "texte": tsvs}));
    for (i, t) in tsvs.iter().enumerate() {
        let rust = eingang::ocr::tsv_zu_zeilen(t).map_or_else(
            |e| json!({"err": "Error", "msg": e.to_string()}),
            |v| json!({"ok": v}),
        );
        z.pruefe(&format!("tsv {t:?}"), &rust, &py_t[i]);
    }
    let mut neg = Zaehler::default();
    let mut gestoert = rust_konto(
        ka::parse_csv("Datum;Betrag;Zweck\n01.03.2025;-480,00;Maler\n"),
        None,
    );
    gestoert["events"][0]["wert"] = json!(48001);
    let py0 = frage(
        &json!({"fn": "schritt8.eingang.konto", "tx": [{"datum": "01.03.2025", "betrag": -48000, "verwendungszweck": "Maler"}], "ts": TS, "llm": null}),
    );
    neg.pruefe(
        "negativ",
        &gestoert,
        &json!({"r": py0["r"], "events": py0["events"]}),
    );
    println!("kontoauszug: {} CSV ({} Korpus + 1000 generiert, {mit_tx} mit Buchungen), 300 JSON, {} Betraege, 500 PDF-Texte, 200 TSV = {} Vergleiche; {events_n} Rust-Events verglichen; Abweichungen {}; dokumentiert {}; Negativkontrolle {}",
        csvs.len(), korpus.len(), betraege.len(), z.faelle, z.abw, z.dokumentiert, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
}

// ------------------------------------------------------------------ PDF/OCR

/// Minimaler PDF-Schreiber: je Seite Textzeilen (Helvetica, `WinAnsiEncoding`) oder ein PBM-Bild.
enum Seite {
    Text(Vec<String>),
    Bild {
        breite: usize,
        hoehe: usize,
        bits: Vec<u8>,
    },
}

fn pdf(seiten: &[Seite]) -> Vec<u8> {
    let mut objekte: Vec<Vec<u8>> = Vec::new();
    let n = seiten.len();
    // 1 Katalog, 2 Seitenbaum, 3 Font, dann je Seite: Seite, Inhalt, (Bild).
    let mut kinder = Vec::new();
    let mut naechste = 4;
    let mut koerper: Vec<(usize, Vec<u8>)> = Vec::new();
    for s in seiten {
        let seite_nr = naechste;
        let inhalt_nr = naechste + 1;
        kinder.push(format!("{seite_nr} 0 R"));
        match s {
            Seite::Text(zeilen) => {
                let mut strom = b"BT /F1 11 Tf 14 TL 40 800 Td ".to_vec();
                for z in zeilen {
                    strom.push(b'(');
                    for c in z.chars() {
                        match c {
                            '(' | ')' | '\\' => strom.extend([b'\\', c as u8]),
                            c if (c as u32) < 128 => strom.push(c as u8),
                            c => strom.extend(format!("\\{:03o}", u32::from(c).min(255)).bytes()),
                        }
                    }
                    strom.extend(b") Tj T* ");
                }
                strom.extend(b"ET");
                koerper.push((seite_nr, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents {inhalt_nr} 0 R >>").into_bytes()));
                koerper.push((
                    inhalt_nr,
                    [
                        format!("<< /Length {} >>\nstream\n", strom.len()).into_bytes(),
                        strom,
                        b"\nendstream".to_vec(),
                    ]
                    .concat(),
                ));
                naechste += 2;
            }
            Seite::Bild {
                breite,
                hoehe,
                bits,
            } => {
                let bild_nr = naechste + 2;
                let strom = b"q 595 0 0 842 0 0 cm /Im1 Do Q".to_vec();
                koerper.push((seite_nr, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /XObject << /Im1 {bild_nr} 0 R >> >> /Contents {inhalt_nr} 0 R >>").into_bytes()));
                koerper.push((
                    inhalt_nr,
                    [
                        format!("<< /Length {} >>\nstream\n", strom.len()).into_bytes(),
                        strom,
                        b"\nendstream".to_vec(),
                    ]
                    .concat(),
                ));
                koerper.push((bild_nr, [format!("<< /Type /XObject /Subtype /Image /Width {breite} /Height {hoehe} /ColorSpace /DeviceGray /BitsPerComponent 1 /Decode [1 0] /Length {} >>\nstream\n", bits.len()).into_bytes(), bits.clone(), b"\nendstream".to_vec()].concat()));
                naechste += 3;
            }
        }
    }
    objekte.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objekte
        .push(format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kinder.join(" ")).into_bytes());
    objekte.push(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    koerper.sort_by_key(|(k, _)| *k);
    objekte.extend(koerper.into_iter().map(|(_, b)| b));
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut versaetze = Vec::new();
    for (i, o) in objekte.iter().enumerate() {
        versaetze.push(out.len());
        out.extend(format!("{} 0 obj\n", i + 1).bytes());
        out.extend(o);
        out.extend(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objekte.len() + 1).bytes());
    for v in versaetze {
        out.extend(format!("{v:010} 00000 n \n").bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objekte.len() + 1
        )
        .bytes(),
    );
    out
}

/// Rendert eine Text-Seite mit `pdftoppm -mono` zu PBM (P4) — die Vorlage fuer Bild-Seiten.
fn als_bild(zeilen: &[String], dir: &std::path::Path) -> Seite {
    let quelle = dir.join("vorlage.pdf");
    std::fs::write(&quelle, pdf(&[Seite::Text(zeilen.to_vec())])).unwrap();
    let ziel = dir.join("vorlage");
    let st = std::process::Command::new("pdftoppm")
        .args(["-mono", "-r", "100", "-singlefile"])
        .arg(&quelle)
        .arg(&ziel)
        .status()
        .unwrap();
    assert!(st.success());
    let pbm = std::fs::read(dir.join("vorlage.pbm")).unwrap();
    // P4\n<w> <h>\n<bits>
    let mut felder = Vec::new();
    let mut pos = 0;
    while felder.len() < 3 {
        let ende = pos + pbm[pos..].iter().position(u8::is_ascii_whitespace).unwrap();
        if ende > pos {
            felder.push(String::from_utf8_lossy(&pbm[pos..ende]).to_string());
        }
        pos = ende + 1;
    }
    Seite::Bild {
        breite: felder[1].parse().unwrap(),
        hoehe: felder[2].parse().unwrap(),
        bits: pbm[pos..].to_vec(),
    }
}

fn ocr_json(r: &Result<(String, eingang::ocr::ConfMap), eingang::ocr::OcrFehler>) -> Value {
    match r {
        Ok((t, c)) => json!({"ok": [t, c]}),
        Err(eingang::ocr::OcrFehler::ZuAufwendig(m)) => json!({"err": "OcrZuAufwendig", "msg": m}),
        Err(e) => json!({"err": format!("{e:?}")}),
    }
}

#[test]
fn pdf_ocr() {
    if skip() {
        return;
    }
    let werkzeuge = ["pdftotext", "pdftoppm", "tesseract"]
        .iter()
        .all(|w| std::process::Command::new(w).arg("-v").output().is_ok());
    if !werkzeuge {
        println!("pdf_ocr: source_unavailable — pdftotext/pdftoppm/tesseract fehlen");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let text = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    let seite1 = text(&[
        "Kontoauszug Nr. 3 / 2025",
        "01.03.2025 Malermeister Huber -480,00 EUR",
        "02.03.2025 Spende Tierheim e.V. -50,00",
        "03.03.2025 Gehalt 3.100,00",
        "Neuer Saldo 2.570,00",
        "04.03.2025 Gartenpflege -120,50 -2.449,50",
    ]);
    let seite2 = text(&[
        "05.03.2025 Rotes Kreuz Spende -25,00",
        "06.03.2025 Heizung Wartung -89,00 EUR",
        "Kontostand 2.335,50",
    ]);
    let bild2 = als_bild(&seite2, dir.path());
    let bild1 = als_bild(&seite1, dir.path());
    let kleines = als_bild(&text(&["x"]), dir.path());
    let faelle: Vec<(&str, Vec<u8>)> = vec![
        (
            "textlayer",
            pdf(&[Seite::Text(seite1.clone()), Seite::Text(seite2.clone())]),
        ),
        ("scan", pdf(&[bild1])),
        ("gemischt", pdf(&[Seite::Text(seite1.clone()), bild2])),
        ("kurze textseite", pdf(&[Seite::Text(text(&["Seite 1"]))])),
        (
            "41 bildseiten",
            pdf(&(0..41)
                .map(|_| match &kleines {
                    Seite::Bild {
                        breite,
                        hoehe,
                        bits,
                    } => Seite::Bild {
                        breite: *breite,
                        hoehe: *hoehe,
                        bits: bits.clone(),
                    },
                    Seite::Text(_) => unreachable!(),
                })
                .collect::<Vec<_>>()),
        ),
    ];
    let mut z = Zaehler::default();
    for (name, bytes) in &faelle {
        let pfad = dir.path().join(format!("{}.pdf", name.replace(' ', "_")));
        std::fs::write(&pfad, bytes).unwrap();
        let p = pfad.to_string_lossy().to_string();
        let py = frage(&json!({"fn": "schritt8.eingang.pdf", "pfad": p}));
        let konto = eingang::ocr::lies_kontoauszug_pdf(&p);
        let beleg = eingang::ocr::lies_beleg_text(&p);
        z.pruefe(
            &format!("{name}: kontoauszug"),
            &ocr_json(&konto),
            &py["konto"],
        );
        z.pruefe(&format!("{name}: beleg"), &ocr_json(&beleg), &py["beleg"]);
        if let Ok((t, c)) = &konto {
            let rust = ka::parse_pdf_zeilen(t, c, 0.6)
                .map(|(tx, n)| json!([tx.iter().map(tx_json).collect::<Vec<_>>(), n]))
                .unwrap();
            let conf: BTreeMap<String, f64> = c.iter().map(|(k, v)| (k.to_string(), *v)).collect();
            let py_z = frage(
                &json!({"fn": "schritt8.eingang.pdf_zeilen", "faelle": [{"text": t, "conf": conf}]}),
            );
            z.pruefe(
                &format!("{name}: buchungen"),
                &json!({"ok": rust}),
                &py_z[0],
            );
            println!(
                "  pdf {name:<16} {} Zeichen, {} OCR-Zeilen, Buchungen {}",
                t.len(),
                c.len(),
                rust[0].as_array().unwrap().len()
            );
        } else {
            println!("  pdf {name:<16} {}", ocr_json(&konto));
        }
    }
    let mut neg = Zaehler::default();
    neg.pruefe(
        "negativ",
        &json!({"err": "OcrZuAufwendig", "msg": "x"}),
        &json!({"err": "OcrZuAufwendig", "msg": "y"}),
    );
    println!("pdf_ocr: {} PDFs, {} Vergleiche (je Datei beide Leser + Buchungen); Abweichungen {}; Negativkontrolle {}", faelle.len(), z.faelle, z.abw, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
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
            Some(TS),
        ) {
            Ok(v) => json!({"ok": v.len()}),
            Err(_) => json!({"err": true}),
        };
        out["events"] = events(&s);
    }
    out
}

#[test]
fn beleg() {
    if skip() {
        return;
    }
    let mut texte: Vec<String> = std::fs::read_dir(repo_root().join("tests/fixtures"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    texte.sort();
    let n_fix = texte.len();
    let mut anker: Vec<String> = Vec::new();
    for typ in [
        eingang::beleg::BelegTyp::Lstb,
        eingang::beleg::BelegTyp::Spende,
        eingang::beleg::BelegTyp::Handwerker,
        eingang::beleg::BelegTyp::Dienstleistung,
        eingang::beleg::BelegTyp::Minijob,
    ] {
        for a in eingang::beleg::beleg_felder(nachschlag(), typ).into_values() {
            anker.push(match a {
                eingang::beleg::Anker::Nr(n) => format!("Nr. {n}"),
                eingang::beleg::Anker::Label(l) => l,
            });
        }
    }
    let koepfe = [
        "Lohnsteuerbescheinigung 2025",
        "Zuwendungsbestätigung",
        "Rechnung Handwerker",
        "Haushaltsnahe Dienstleistung",
        "Minijob Haushaltsscheck",
        "Handwerker Dienstleistung",
        "Rechnung",
    ];
    let mut r = Rng(0xbe1e_9000);
    texte.extend((0..500).map(|_| {
        let mut t = (*r.wahl(&koepfe)).to_owned();
        for _ in 0..r.n(10) {
            let a = r.wahl(&anker).clone();
            let a = if r.p(20) { a.to_uppercase() } else { a };
            let _ = write!(
                t,
                "\n{}{} {}",
                r.wahl(&["", "  ", "3. "]),
                a,
                r.wahl(&[
                    "45.000,00",
                    "1.234,56 EUR",
                    "12,00 3,50",
                    "ohne Betrag",
                    "7,5",
                    "100,00"
                ])
            );
        }
        t
    }));
    let mut z = Zaehler::default();
    let mut kandidaten_n = 0;
    for (i, t) in texte.iter().enumerate() {
        let conf: BTreeMap<String, f64> = if i % 4 == 0 {
            anker
                .iter()
                .take(3)
                .map(|a| (a.trim_start_matches("Nr. ").to_owned(), 0.42))
                .collect()
        } else {
            BTreeMap::new()
        };
        let schreibe = i % 2 == 0;
        let py = frage(
            &json!({"fn": "schritt8.eingang.beleg", "text": t, "conf": conf, "schreibe": schreibe, "ref": "upload-1", "ts": TS}),
        );
        let rust = rust_beleg(t, &conf, schreibe);
        kandidaten_n += rust["kandidaten"]["ok"].as_array().unwrap().len();
        let mut py = py;
        if py.get("schreibe").and_then(|s| s.get("err")).is_some() {
            py["schreibe"] = json!({"err": true});
        }
        z.pruefe(&format!("beleg {t:?}"), &rust, &py);
    }
    let mut neg = Zaehler::default();
    let mut g = rust_beleg(&texte[0], &BTreeMap::new(), false);
    g["kandidaten"]["ok"] = json!([]);
    neg.pruefe("negativ", &g, &frage(&json!({"fn": "schritt8.eingang.beleg", "text": texte[0], "conf": {}, "schreibe": false, "ref": "", "ts": TS})));
    println!("beleg: {} Texte ({n_fix} Fixtures + 500 generiert, {} Anker aus der Bindung), {kandidaten_n} Kandidaten; Abweichungen {}; Negativkontrolle {}",
        texte.len(), anker.len(), z.abw, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
}

// ------------------------------------------------------------------ Vorjahr, VaSt, eDaten

fn sortiert(v: &Value) -> Value {
    let mut a = v.as_array().cloned().unwrap_or_default();
    a.sort_by_key(|e| format!("{}{}", e["feld_id"], e["event_id"]));
    Value::Array(a)
}

fn vorbelegen(s: &mut Store, fid: &str) {
    let leer = HashMap::new();
    let neu = store::NeuesEvent {
        feld_id: fid.to_owned(),
        wert: json!(1),
        feldzustand: domain::Feldzustand::Vorlaeufig,
        herkunft: domain::Herkunft {
            herkunft: domain::Achsenwert::new("laie").unwrap(),
            pruef_tiefe: domain::PruefTiefe::Ungeprueft,
            haftung: domain::Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: "ui:laie".parse().unwrap(),
        signal_1: None,
        ersetzt: None,
        ts: Some(TS.into()),
    };
    s.append(&neu, None, BindungNachschlag::neu(&leer)).unwrap();
}

#[test]
fn vorjahr_vast_edaten() {
    if skip() {
        return;
    }
    let mut z = Zaehler::default();
    let mut r = Rng(0x7e41_2025);
    let flags: Vec<String> = eingang::vorjahr::uebertragbare_felder(nachschlag())
        .into_keys()
        .collect();
    let werte = [
        json!(4_000_000),
        json!("40000"),
        json!(true),
        json!(1.5),
        Value::Null,
        json!(0),
        json!("verheiratet"),
        json!(-5),
    ];
    for _ in 0..500 {
        let mut felder = serde_json::Map::new();
        for _ in 0..r.n(12) {
            let fid = if r.p(90) {
                r.wahl(&flags).clone()
            } else {
                "verlustvortrag_bestand".into()
            };
            felder.insert(fid, json!({"wert": r.wahl(&werte), "zustand": r.wahl(&["bestaetigt", "vorlaeufig", "bestaetigt"])}));
        }
        let vorbelegt: Vec<String> = (0..r.n(3))
            .map(|_| r.wahl(&flags).clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let py = frage(
            &json!({"fn": "schritt8.eingang.vorjahr", "felder": felder, "vz": 2025, "ts": TS, "vorbelegt": vorbelegt}),
        );
        let mut s = Store::leer(2026, None);
        for f in &vorbelegt {
            vorbelegen(&mut s, f);
        }
        let vf: BTreeMap<String, eingang::vorjahr::VorjahrFeld> =
            serde_json::from_value(Value::Object(felder.clone())).unwrap();
        let rust = match eingang::vorjahr::uebernehme(&mut s, &vf, nachschlag(), 2025, Some(TS)) {
            Ok(e) => json!({"r": {"ok": e.uebertragen}, "referenz": e.referenz}),
            Err(_) => json!({"r": {"err": true}}),
        };
        let py_r = if py["r"].get("err").is_some() {
            json!({"err": true})
        } else {
            py["r"].clone()
        };
        // Bei Abweisung bricht Python mitten in der Schleife ab (andere Reihenfolge als Rust); nur die Klasse zaehlt.
        if py_r.get("err").is_some() {
            z.pruefe(&format!("vorjahr {felder:?}"), &rust["r"], &py_r);
        } else {
            z.pruefe(&format!("vorjahr {felder:?}"), &json!({"r": rust["r"], "referenz": rust["referenz"], "events": sortiert(&events(&s))}),
                &json!({"r": py_r, "referenz": py["referenz"], "events": sortiert(&py["events"])}));
        }
    }
    // VaSt
    let mut betraege: Vec<Option<String>> = [
        "45000.00",
        "1,5",
        "-0.005",
        "0,015",
        "1e2",
        " 7 ",
        "",
        "abc",
        "NaN",
        "Infinity",
        "-inf",
        "1_000.5",
        "0.125",
        "12345678901234567890123456.785",
        "١٢٣٫٤",
        "٤٥٠٠٠.٠٠",
        "1E+2",
        ".5",
        "5.",
        "1..2",
        "+3",
        "--3",
        "sNaN",
        "0e999999",
        "1e-999999",
    ]
    .iter()
    .map(|s| Some((*s).to_owned()))
    .collect();
    betraege.push(None);
    betraege.extend((0..1000).map(|_| {
        Some(format!(
            "{}{}{}",
            r.wahl(&["", "-", "+", " "]),
            r.n(10_000_000),
            r.wahl(&[".00", ",5", ".125", ".135", "", ".9999", "e1", ".5e-1"])
        ))
    }));
    let lstb_namen = [
        "BruttoArbLohn",
        "LSteuer",
        "ArbnKiSteuer",
        "ArbnAnteilArblVers",
        "ArbnAnteilKrankVers",
        "Soli",
        "ArbnAnteilPflegVers",
    ];
    let lstb: Vec<BTreeMap<String, String>> = (0..300)
        .map(|_| {
            (0..r.n(7))
                .map(|_| {
                    (
                        (*r.wahl(&lstb_namen)).to_owned(),
                        r.wahl(&betraege).clone().unwrap_or_default(),
                    )
                })
                .collect()
        })
        .collect();
    let lersl: Vec<Value> = (0..300).map(|_| Value::Array((0..r.n(4)).map(|_| json!({"Betrag": r.wahl(&betraege), "Art": r.wahl(&["ALG", " Krankengeld ", "", "ALG"])})).collect())).collect();
    let py = frage(
        &json!({"fn": "schritt8.eingang.vast", "werte": betraege, "lstb": lstb, "lersl": lersl}),
    );
    let vast_fehler = |e: &eingang::vast::VastFehler| match e {
        eingang::vast::VastFehler::NichtLesbar(_) => {
            json!({"err": "ValueError", "msg": e.to_string()})
        }
        eingang::vast::VastFehler::Ueberlauf(_) => json!({"err": "OverflowError"}),
    };
    let py_norm = |v: &Value| {
        if v.get("err").is_some_and(|e| e != "ValueError") {
            json!({"err": "OverflowError"})
        } else {
            v.clone()
        }
    };
    for (i, b) in betraege.iter().enumerate() {
        let rust = eingang::vast::cent(b.as_deref())
            .map_or_else(|e| vast_fehler(&e), |c| json!({"ok": c}));
        let p = py_norm(&py["cent"][i]);
        // PARITAET-Abweichung (Bericht): Python liefert eine grosse Ganzzahl, Rust `Ueberlauf`.
        if rust == json!({"err": "OverflowError"}) && p.get("ok").is_some() {
            z.dokumentiert += 1;
            continue;
        }
        z.pruefe(&format!("vast cent {b:?}"), &rust, &p);
    }
    for (i, w) in lstb.iter().enumerate() {
        let rust =
            eingang::vast::aus_lstb(w).map_or_else(|e| vast_fehler(&e), |v| json!({"ok": v}));
        z.pruefe(&format!("aus_lstb {w:?}"), &rust, &py_norm(&py["lstb"][i]));
    }
    for (i, l) in lersl.iter().enumerate() {
        let ls: Vec<eingang::vast::Leistung> = serde_json::from_value(l.clone()).unwrap();
        let rust =
            eingang::vast::aus_lersl(&ls).map_or_else(|e| vast_fehler(&e), |v| json!({"ok": v}));
        z.pruefe(&format!("aus_lersl {l}"), &rust, &py_norm(&py["lersl"][i]));
    }
    // eDaten
    let felder: Vec<&str> = bindungen().iter().map(|b| b.feld_id.as_str()).collect();
    for _ in 0..500 {
        let saetze: Vec<Value> = (0..r.n(6)).map(|_| json!({"feld_id": if r.p(85) { *r.wahl(&felder) } else { "unbekannt_xyz" },
            "wert": r.wahl(&[json!(123_456), json!("123"), json!(true), json!(2), json!("ja")]), "kategorie": r.wahl(&[json!("LStB/LSteuer"), Value::Null])})).collect();
        let mit_bindung = r.p(50);
        let py = frage(
            &json!({"fn": "schritt8.eingang.edaten", "saetze": saetze, "ts": TS, "mit_bindung": mit_bindung}),
        );
        let mut s = Store::leer(2025, None);
        let sz: Vec<eingang::edaten::Satz> =
            serde_json::from_value(Value::Array(saetze.clone())).unwrap();
        let rust = match eingang::edaten::uebernehme(
            &mut s,
            &sz,
            Some(TS),
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
        z.pruefe(
            &format!("edaten {saetze:?} bindung={mit_bindung}"),
            &json!({"r": rust, "events": events(&s)}),
            &json!({"r": py_r, "events": py["events"]}),
        );
    }
    let mut neg = Zaehler::default();
    neg.pruefe(
        "negativ",
        &json!({"ok": eingang::vast::cent(Some("45000.00")).unwrap().unwrap() + 1}),
        &py["cent"][0],
    );
    println!("vorjahr+vast+edaten: {} Vergleiche (500 Vorjahr, {} Betraege, 300 LStB, 300 LErsL, 500 eDaten); Abweichungen {}; dokumentiert {}; Negativkontrolle {}",
        z.faelle, betraege.len(), z.abw, z.dokumentiert, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
}
