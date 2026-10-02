//! Paritaet `rust/api/src/flow.rs` gegen `produkt/haut/flow.py` (`tools/parity/flow_oracle.py`),
//! Suite 18.
//!
//! Verglichen wird, was ein Leser der Datei sieht: Byte fuer Byte jede Zeile von `flow.jsonl`
//! (ohne `ts`, dessen Form eigens geprueft wird), dazu der Dateimodus und ob die Datei entsteht.
//! Beide Seiten bekommen denselben JSON-TEXT; Python liest ihn mit `json.loads`, Rust mit
//! `serde_json` in `PyWert` — nur so haben beide dieselbe Einfuegereihenfolge.
//!
//! - `konstanten`: Dateiname, Kappungsgrenze, Liste der UI-Sorten.
//! - `schalter`: `an()` ueber `TAXGRAPH_FLOW` x `TAXGRAPH_KI_DEBUG`, mit Leerraum und Fremdzeichen.
//! - `zeile_und_dumps`: `schreibe` auf generierten JSON-Werten (Zahlen, Escapes, Nicht-BMP, doppelte
//!   Schluessel, Reihenfolge) und festen Randfaellen.
//! - `kappung`: `gekappt` an der Grenze, eins davor und eins dahinter.
//! - `melde_ui`: Schalter x Rumpf (alle Sorten, jede Abweisung, Kappung, `fall` leer).
//! - `ergebnis_notiert`, `kopf_der_queue`: Zaehlung, Kappung bei 12 und bei 90 Zeichen.
//! - `bestehende_datei`: Rechte und Inhalt einer vorhandenen Datei.
//! - `dokumentierte_abweichungen`: die zwei Stellen, an denen Rust bewusst anders ist.
//!
//!   `PARITY`=1 `cargo` test -p parity --test `flow_paritaet` -- --nocapture
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::ffi::OsString;
use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use api::flow;
use api::ApiFehler;
use domain::PyWert;
use parity::Oracle;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
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
    let a = oracle()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .call_json(anfrage)
        .expect("orakel antwortet");
    a.get("ok")
        .cloned()
        .unwrap_or_else(|| panic!("Orakel-Fehler: {a}"))
}

#[derive(Default)]
struct Zaehler {
    faelle: usize,
    abw: usize,
}
impl Zaehler {
    fn pruefe(&mut self, was: &str, rust: &Value, py: &Value) {
        self.faelle += 1;
        if rust != py {
            self.abw += 1;
            if self.abw <= 5 {
                eprintln!("ABWEICHUNG {was}\n  rust: {rust}\n  py:   {py}");
            }
        }
    }

    fn fertig(&self, was: &str, mindestens: usize) {
        eprintln!("{was}: {} Faelle, {} Abweichungen", self.faelle, self.abw);
        assert!(
            self.faelle >= mindestens,
            "{was}: nur {} Faelle",
            self.faelle
        );
        assert_eq!(self.abw, 0, "{was}: Rust weicht von Python ab");
    }
}

// ---------------------------------------------------------------- Umgebung, Ablage, Datei lesen

static UMGEBUNG: Mutex<()> = Mutex::new(());

/// Setzt beide Schalter, haelt die Sperre, stellt beim Ende (auch beim Panic) die alten Werte her.
struct Umgebung(
    Vec<(&'static str, Option<OsString>)>,
    #[allow(dead_code)] MutexGuard<'static, ()>,
);

impl Umgebung {
    fn neu(flow: Option<&str>, debug: Option<&str>) -> Self {
        let sperre = UMGEBUNG.lock().unwrap_or_else(PoisonError::into_inner);
        let paare = [("TAXGRAPH_FLOW", flow), ("TAXGRAPH_KI_DEBUG", debug)];
        let alt = paare
            .iter()
            .map(|(k, _)| (*k, std::env::var_os(k)))
            .collect();
        for (k, v) in paare {
            match v {
                Some(w) => std::env::set_var(k, w),
                None => std::env::remove_var(k),
            }
        }
        Self(alt, sperre)
    }
}

impl Drop for Umgebung {
    fn drop(&mut self) {
        for (k, v) in &self.0 {
            match v {
                Some(w) => std::env::set_var(k, w),
                None => std::env::remove_var(k),
            }
        }
    }
}

/// `YYYY-MM-DDTHH:MM:SS`, danach nichts oder `.ffffff` (Python laesst 0 Mikrosekunden weg).
fn ts_form(ts: &str) -> bool {
    let b = ts.as_bytes();
    if b.len() != 19 && b.len() != 26 {
        return false;
    }
    let ziffer = |i: usize| b[i].is_ascii_digit();
    let feste = [(4, b'-'), (7, b'-'), (10, b'T'), (13, b':'), (16, b':')];
    (0..19).all(|i| {
        feste
            .iter()
            .find(|(p, _)| *p == i)
            .map_or_else(|| ziffer(i), |(_, c)| b[i] == *c)
    }) && (b.len() == 19 || (b[19] == b'.' && (20..26).all(ziffer)))
}

/// Gegenstueck zu `_lies` im Orakel: `ts` abtrennen und pruefen, dazu Modus und Zeilenende.
fn lies(ablage: &Path) -> Value {
    let pfad = ablage.join(flow::DATEI);
    let Ok(meta) = std::fs::metadata(&pfad) else {
        return Value::Null;
    };
    let text = std::fs::read_to_string(&pfad).unwrap();
    let teile: Vec<&str> = text.split('\n').collect();
    let mut ts_ok = true;
    let zeilen: Vec<String> = teile[..teile.len() - 1]
        .iter()
        .map(|z| {
            let rest = z
                .strip_prefix("{\"ts\": \"")
                .and_then(|r| r.split_once("+00:00\", "))
                .filter(|(ts, _)| ts_form(ts));
            if let Some((_, nach)) = rest {
                format!(", {nach}")
            } else {
                ts_ok = false;
                (*z).to_owned()
            }
        })
        .collect();
    json!({
        "ts_ok": ts_ok,
        "zeilen": zeilen,
        "endet_mit_zeilenende": text.ends_with('\n'),
        "modus": meta.permissions().mode() & 0o777,
    })
}

/// Ein Lauf: Schalter, optional eine vorhandene Datei. Beide Seiten bekommen dasselbe.
#[derive(Clone, Default)]
struct Lauf {
    flow: Option<String>,
    debug: Option<String>,
    vorher: Option<(String, u32)>,
}

impl Lauf {
    fn an() -> Self {
        Self {
            flow: Some("1".into()),
            ..Self::default()
        }
    }

    fn aus() -> Self {
        Self::default()
    }

    fn anfrage(&self, name: &str, mut rest: Value) -> Value {
        let o = rest.as_object_mut().unwrap();
        o.insert("fn".into(), json!(name));
        o.insert("flow".into(), json!(self.flow));
        o.insert("debug".into(), json!(self.debug));
        if let Some((text, modus)) = &self.vorher {
            o.insert("datei_vorher".into(), json!(text));
            o.insert("modus_vorher".into(), json!(modus));
        }
        rest
    }

    /// Schalter setzen, frische Ablage anlegen (samt Vorab-Datei), `f` rufen, Datei lesen.
    fn rust<T>(&self, f: impl FnOnce(&Path) -> T) -> (T, Value) {
        let _u = Umgebung::neu(self.flow.as_deref(), self.debug.as_deref());
        let tmp = tempfile::tempdir().unwrap();
        if let Some((text, modus)) = &self.vorher {
            let p = tmp.path().join(flow::DATEI);
            std::fs::write(&p, text).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(*modus)).unwrap();
        }
        let r = f(tmp.path());
        (r, lies(tmp.path()))
    }
}

fn pw(text: &str) -> PyWert {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

// ---------------------------------------------------------------- JSON-Werte als Text erzeugen

/// Ein JSON-Wert als TEXT, damit Schluesselreihenfolge, doppelte Schluessel und die Schreibweise
/// der Zahlen (`1E5`, `-0`, `1e-7`) unveraendert zu beiden Seiten kommen.
#[derive(Clone, Debug)]
enum J {
    Null,
    Bool(bool),
    Zahl(String),
    Text(String, bool),
    Liste(Vec<J>),
    Obj(Vec<(String, J)>),
}

/// `ascii`: alles ausserhalb von ASCII als `\uXXXX` (Nicht-BMP als Ersatzpaar) statt roh.
fn json_text(s: &str, ascii: bool) -> String {
    let t = serde_json::to_string(s).unwrap();
    if !ascii {
        return t;
    }
    t.chars()
        .map(|c| {
            if c.is_ascii() {
                c.to_string()
            } else {
                let mut b = [0u16; 2];
                c.encode_utf16(&mut b)
                    .iter()
                    .fold(String::new(), |mut s, u| {
                        let _ = write!(s, "\\u{u:04x}");
                        s
                    })
            }
        })
        .collect()
}

impl J {
    fn text(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Bool(b) => b.to_string(),
            Self::Zahl(z) => z.clone(),
            Self::Text(s, ascii) => json_text(s, *ascii),
            Self::Liste(l) => format!(
                "[{}]",
                l.iter().map(Self::text).collect::<Vec<_>>().join(", ")
            ),
            Self::Obj(o) => format!(
                "{{{}}}",
                o.iter()
                    .map(|(k, w)| format!("{}: {}", json_text(k, false), w.text()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

fn zeichen() -> impl Strategy<Value = char> {
    prop::sample::select(vec![
        'a',
        'Z',
        '0',
        ' ',
        '"',
        '\\',
        '/',
        '\n',
        '\t',
        '\r',
        '\u{8}',
        '\u{c}',
        '\u{0}',
        '\u{1}',
        '\u{1f}',
        '\u{7f}',
        '\u{85}',
        'ä',
        'ß',
        '€',
        '\u{2028}',
        '\u{2029}',
        '\u{ffff}',
        '😀',
        '\u{10ffff}',
    ])
}

fn text_strat() -> impl Strategy<Value = String> {
    prop::collection::vec(zeichen(), 0..30).prop_map(|v| v.into_iter().collect())
}

fn schluessel() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["a", "b", "c", "", "ä", "😀", "\"", "art", "inhalt"])
        .prop_map(str::to_owned)
}

/// Zahlen so, wie ein Client sie schreibt: ganz, gross, gebrochen, mit Exponent.
fn zahl() -> impl Strategy<Value = String> {
    prop_oneof![
        (-1000i64..1000).prop_map(|n| n.to_string()),
        any::<i64>().prop_map(|n| n.to_string()),
        any::<u64>().prop_map(|n| n.to_string()),
        any::<f64>()
            .prop_filter("endlich", |f| f.is_finite())
            .prop_map(|f| format!("{f:?}")),
        (-99999i32..99999, -320i32..300).prop_map(|(m, e)| format!("{m}e{e}")),
        (-99999i32..99999, 0u32..400).prop_map(|(m, k)| format!("{m}.{k}E+3")),
    ]
}

fn j_strat() -> impl Strategy<Value = J> {
    let blatt = prop_oneof![
        Just(J::Null),
        any::<bool>().prop_map(J::Bool),
        zahl().prop_map(J::Zahl),
        (text_strat(), any::<bool>()).prop_map(|(s, a)| J::Text(s, a)),
    ];
    blatt.prop_recursive(4, 40, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(J::Liste),
            prop::collection::vec((schluessel(), inner), 0..6).prop_map(J::Obj),
        ]
    })
}

/// `n` Werte, jedes Mal derselbe Strom (deterministischer Zufall).
fn werte<S: Strategy>(strat: &S, n: usize) -> Vec<S::Value> {
    let mut runner = TestRunner::new_with_rng(
        Config::default(),
        TestRng::deterministic_rng(RngAlgorithm::ChaCha),
    );
    (0..n)
        .map(|_| strat.new_tree(&mut runner).unwrap().current())
        .collect()
}

/// Feste Randfaelle fuer `inhalt`; Schreibweisen, an denen zwei JSON-Leser auseinanderlaufen
/// koennten. Jeder Text muss auf BEIDEN Seiten gelesen werden koennen.
fn randfaelle() -> Vec<String> {
    let mut v: Vec<String> = [
        "null",
        "true",
        "false",
        "0",
        "-1",
        "7",
        "1.5",
        "-0.0",
        "0.0",
        "1e16",
        "1e-7",
        "1E5",
        "1e5",
        "1e2",
        "100.0",
        "123456789012345678",
        "9223372036854775807",
        "-9223372036854775808",
        "9223372036854775808",
        "18446744073709551615",
        "1e308",
        "-1e308",
        "5e-324",
        "2.5e-324",
        "0.1",
        "0.30000000000000004",
        "1.7976931348623157e308",
        "1e22",
        "1e21",
        "1.0e+16",
        "123456789.123456789",
        "0.00001",
        "0.0001",
        "[]",
        "{}",
        "[[]]",
        "[{}]",
        "{\"a\": {}}",
        "{\"a\": 1, \"b\": 2, \"a\": 3}",
        "{\"b\": 1, \"a\": 2}",
        "{\"\": 1}",
        "{\"a\": [1, 2, {\"b\": null}]}",
        "\"\"",
        "\"a\"",
        "\"\\u00e4\"",
        "\"\\ud83d\\ude00\"",
        "\"\\u2028\"",
        "\"\\u007f\"",
        "\"\\u0000\"",
        "\"\\u001f\"",
        "\"/\"",
        "\"\\/\"",
        "\"\\\"\"",
        "\"\\\\\"",
        "\"\\b\\f\\n\\r\\t\"",
        "\"\u{85}\"",
        "\"\u{feff}\"",
        "\"\u{10ffff}\"",
        "\"\u{ffff}\"",
        "\"\u{e000}\"",
        "[1,2 , 3]",
        " { \"a\" : [ 1 , 2 ] } ",
        "[1e0, 1E0, 1e+0, 1e-0]",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    v.push(format!("\"{}\"", "ä".repeat(3998)));
    v.push(format!("\"{}\"", "ä".repeat(3999)));
    v.push(format!("\"{}\"", "x".repeat(3998)));
    v.push(format!("\"{}\"", "x".repeat(3999)));
    v.push(format!("[{}]", vec!["1"; 1300].join(", ")));
    v
}

// ---------------------------------------------------------------- Tests

/// Die drei Konstanten, auf die alles andere baut.
#[test]
fn konstanten() {
    if skip() {
        return;
    }
    let k = frage(&json!({"fn": "flow.konstanten"}));
    assert_eq!(k["datei"], flow::DATEI);
    assert_eq!(k["max_zeichen"], flow::MAX_ZEICHEN);
    let arten = frage(&json!({"fn": "flow.ui_arten"}));
    assert_eq!(
        arten,
        json!(flow::UI_ARTEN),
        "UI-Sorten, sortiert wie `sorted(UI_ARTEN)`"
    );
}

/// `an()` zur Aufrufzeit: jede Kombination beider Variablen, auch mit Leerraum und Zeichen, die
/// `strip()` kennt (U+001C..U+001F, U+0085, U+00A0, U+2003) oder nicht (U+200B, volle Breite).
#[test]
fn schalter() {
    if skip() {
        return;
    }
    let werte_je: [Option<&str>; 22] = [
        None,
        Some(""),
        Some("1"),
        Some("0"),
        Some(" 1"),
        Some("1 "),
        Some("\t1\n"),
        Some("\u{b}1\u{c}"),
        Some("\u{1c}1\u{1f}"),
        Some("\u{85}1"),
        Some("\u{a0}1\u{a0}"),
        Some("\u{2003}1"),
        Some("\u{3000}1"),
        Some("\u{200b}1"),
        Some("\u{feff}1"),
        Some("１"),
        Some("true"),
        Some("11"),
        Some("01"),
        Some("1\u{0301}"),
        Some("+1"),
        Some("2"),
    ];
    let mut z = Zaehler::default();
    let mut an = 0;
    for f in werte_je {
        for d in werte_je {
            let lauf = Lauf {
                flow: f.map(str::to_owned),
                debug: d.map(str::to_owned),
                vorher: None,
            };
            let (rust, _) = lauf.rust(|_| flow::an());
            an += usize::from(rust);
            let py = frage(&lauf.anfrage("flow.an", json!({})));
            z.pruefe(&format!("FLOW={f:?} KI_DEBUG={d:?}"), &json!(rust), &py);
        }
    }
    z.fertig("schalter", 400);
    assert!(
        an > 40 && an < 400,
        "beide Antworten muessen vorkommen: {an} von 484 an"
    );
}

/// `schreibe` auf generierten und festen JSON-Werten: die Zeile Byte fuer Byte, `ts` nur der Form
/// nach, dazu der Modus 0600 und das Zeilenende. Je Fall eine frische Ablage mit eigener `fall`
/// (Text, leer, fehlend).
#[test]
fn zeile_und_dumps() {
    if skip() {
        return;
    }
    let mut z = Zaehler::default();
    let mut texte: Vec<String> = randfaelle();
    texte.extend(werte(&j_strat(), 1500).iter().map(J::text));
    let falle = [Some("f1"), Some(""), None, Some("fä😀\"")];
    for (i, inhalt) in texte.iter().enumerate() {
        let fall = falle[i % falle.len()];
        let art = ["antwort", "fragen", "ki", "ä\"art"][i % 4];
        let lauf = Lauf::an();
        let ((), rust) = lauf.rust(|a| flow::schreibe(a, fall, art, &pw(inhalt)));
        let py = frage(&lauf.anfrage(
            "flow.schreibe",
            json!({"fall": fall, "art": art, "inhalt": inhalt}),
        ))["datei"]
            .clone();
        z.pruefe(&format!("schreibe {fall:?} {art} {inhalt}"), &rust, &py);
        assert_eq!(rust["ts_ok"], true, "{rust}");
        assert_eq!(rust["modus"], 0o600, "Datei mit 0600: {rust}");
    }
    z.fertig("zeile_und_dumps", 1500);
}

/// `gekappt` an der Grenze: bei `len - 1`, `len`, `len + 1` der JSON-Fassung und bei festen Grenzen.
#[test]
fn kappung() {
    if skip() {
        return;
    }
    let mut z = Zaehler::default();
    let mut gekappt_n = 0;
    let mut texte: Vec<String> = randfaelle();
    texte.extend(werte(&j_strat(), 400).iter().map(J::text));
    for inhalt in &texte {
        let laenge = flow::dumps(&pw(inhalt)).chars().count();
        let mut grenzen = vec![0, 1, 2, 5, 40, laenge.saturating_sub(1), laenge, laenge + 1];
        grenzen.sort_unstable();
        grenzen.dedup();
        for g in grenzen {
            let rust = json!(flow::dumps(&flow::gekappt(&pw(inhalt), g)));
            gekappt_n += usize::from(rust != json!(flow::dumps(&pw(inhalt))));
            let py = frage(&json!({"fn": "flow.gekappt", "inhalt": inhalt, "grenze": g}));
            z.pruefe(&format!("gekappt({inhalt}, {g})"), &rust, &py);
        }
    }
    z.fertig("kappung", 2000);
    assert!(gekappt_n > 500, "es muss auch gekappt werden: {gekappt_n}");
}

/// Der Rumpf der Route: Schalter x Rumpf x `fall`. Gleiche Antwort (Status, Text, Fehlerwortlaut)
/// UND gleiche Datei. `ValueError` auf Pythons Seite entspricht `ApiFehler::Status(400, ..)`.
#[test]
fn melde_ui() {
    if skip() {
        return;
    }
    let mut bodies: Vec<String> = Vec::new();
    let inhalte = [
        None,
        Some("null".to_owned()),
        Some("\"text\"".to_owned()),
        Some("{\"weg\": \"fragebogen\", \"n\": [1, 2.5, null]}".to_owned()),
        Some(format!("{{\"offen\": \"{}\"}}", "x".repeat(5000))),
        Some(format!("\"{}\"", "ä".repeat(3998))),
        Some(format!("\"{}\"", "ä".repeat(3999))),
        Some("[1, 2, 3]".to_owned()),
    ];
    for art in flow::UI_ARTEN {
        for inhalt in &inhalte {
            bodies.push(match inhalt {
                None => format!("{{\"art\": \"{art}\"}}"),
                Some(i) => format!("{{\"art\": \"{art}\", \"inhalt\": {i}}}"),
            });
        }
    }
    bodies.extend(
        [
            "{}",
            "{\"inhalt\": 1}",
            "{\"art\": null}",
            "{\"art\": 5}",
            "{\"art\": 1.5}",
            "{\"art\": true}",
            "{\"art\": [\"weg_gewaehlt\"]}",
            "{\"art\": {\"weg_gewaehlt\": 1}}",
            "{\"art\": \"\"}",
            "{\"art\": \"erfunden\"}",
            "{\"art\": \"WEG_GEWAEHLT\"}",
            "{\"art\": \" weg_gewaehlt\"}",
            "{\"art\": \"weg_gewaehlt \"}",
            "{\"art\": \"weg_gewaehlt\\u0000\"}",
            "{\"art\": \"weg_gewaehlt\\u0301\"}",
            "{\"art\": \"ä\"}",
            "{\"art\": \"x\", \"art\": \"weg_gewaehlt\"}",
            "{\"art\": \"weg_gewaehlt\", \"art\": \"x\"}",
            "{\"inhalt\": 1, \"art\": \"pruefliste_weiter\"}",
            "{\"art\": \"weg_gewaehlt\", \"extra\": {\"a\": 1}}",
            "[]",
            "[{\"art\": \"weg_gewaehlt\"}]",
            "\"weg_gewaehlt\"",
            "null",
            "5",
            "1.5",
            "true",
            "false",
            "\"\"",
            "0",
        ]
        .map(str::to_owned),
    );
    let mut z = Zaehler::default();
    let (mut ok_n, mut fehler_n, mut aus_n, mut gekappt_n) = (0, 0, 0, 0);
    for lauf in [
        Lauf::an(),
        Lauf::aus(),
        Lauf {
            debug: Some(" 1 ".into()),
            ..Lauf::default()
        },
    ] {
        for (i, body) in bodies.iter().enumerate() {
            let fall = ["f1", "", "fä😀"][i % 3];
            let (antwort, datei) = lauf.rust(|a| flow::melde_ui(a, fall, &pw(body)));
            let rust = match antwort {
                Ok(a) => json!({"status": a.status, "body": a.body}),
                Err(ApiFehler::Status(400, msg)) => json!({"err": "ValueError", "msg": msg}),
                Err(anders) => panic!("{body}: {anders:?}"),
            };
            let py = frage(&lauf.anfrage("flow.melde_ui", json!({"fall": fall, "body": body})));
            if rust.get("err").is_some() {
                fehler_n += 1;
            } else if rust["body"]["mitgeschrieben"] == false {
                aus_n += 1;
            } else {
                ok_n += 1;
                gekappt_n += usize::from(
                    datei["zeilen"][0]
                        .as_str()
                        .is_some_and(|l| l.contains("gekappt_bei")),
                );
            }
            z.pruefe(
                &format!(
                    "melde_ui({fall:?}, {body}) flow={:?} debug={:?}",
                    lauf.flow, lauf.debug
                ),
                &json!({"antwort": rust, "datei": datei}),
                &py,
            );
        }
    }
    z.fertig("melde_ui", 200);
    eprintln!("melde_ui: mitgeschrieben {ok_n}, abgewiesen {fehler_n}, ohne Schalter {aus_n}, davon gekappt {gekappt_n}");
    // Jeder Ausgang muss vorkommen, sonst vergleicht der Test Leeres mit Leerem.
    assert!(ok_n >= 80 && fehler_n >= 40 && aus_n >= 60 && gekappt_n >= 10);
}

/// `ergebnis_notiert`: Zaehlung ALLER benannten Felder, die ersten 12, alle Formen von `offen`
/// (Liste in jeder Laenge, `null`, fehlt, leer) und `grund`/`zahl_cent` in beliebiger Form.
/// Ein `offen`, das keine Liste ist, bleibt aussen vor (A2).
#[test]
fn ergebnis_notiert() {
    if skip() {
        return;
    }
    let mut z = Zaehler::default();
    let mut objekte: Vec<String> = Vec::new();
    for n in [0usize, 1, 2, 11, 12, 13, 14, 40] {
        let offen = (0..n)
            .map(|i| format!("\"feld_{i}\""))
            .collect::<Vec<_>>()
            .join(", ");
        objekte.push(format!(
            "{{\"grund\": \"offen\", \"zahl_cent\": null, \"offen\": [{offen}]}}"
        ));
        objekte.push(format!(
            "{{\"offen\": [{offen}], \"zahl_cent\": 12345, \"grund\": null}}"
        ));
    }
    objekte.extend(
        [
            "{}",
            "{\"offen\": null}",
            "{\"offen\": []}",
            "{\"grund\": \"x\"}",
            "{\"zahl_cent\": 0}",
            "{\"zahl_cent\": -5, \"grund\": \"ä\", \"offen\": [1, null, [2], {\"a\": 1}]}",
            "{\"grund\": {\"a\": [1]}, \"zahl_cent\": 1.5, \"offen\": [\"a\"]}",
        ]
        .map(str::to_owned),
    );
    let mut genannt = 0;
    for (i, obj) in objekte.iter().enumerate() {
        let fall = ["f1", "f2", ""][i % 3];
        for lauf in [Lauf::an(), Lauf::aus()] {
            let ((), rust) = lauf.rust(|a| flow::ergebnis_notiert(a, fall, &pw(obj)));
            let py =
                frage(&lauf.anfrage("flow.ergebnis_notiert", json!({"fall": fall, "obj": obj})))
                    ["datei"]
                    .clone();
            genannt += usize::from(rust != Value::Null);
            z.pruefe(
                &format!("ergebnis_notiert({fall:?}, {obj}) flow={:?}", lauf.flow),
                &rust,
                &py,
            );
        }
    }
    z.fertig("ergebnis_notiert", 40);
    assert!(
        genannt >= 20,
        "ohne Schalter keine Datei, mit Schalter eine: {genannt}"
    );
}

/// `kopf_der_queue`: die ersten `n`, der Fragetext auf 90 Zeichen (Codepunkte, nicht Bytes), fehlende
/// Schluessel, `null`-Text. Eine Frage, die kein `dict` ist, bleibt aussen vor (A2).
#[test]
fn kopf_der_queue() {
    if skip() {
        return;
    }
    let mut z = Zaehler::default();
    let text = |n: usize, c: &str| format!("\"{}\"", c.repeat(n));
    let eine_frage = |i: usize, t: &str| {
        format!("{{\"feld_id\": \"f{i}\", \"fragetext_laie\": {t}, \"instanz_anzahl\": {i}}}")
    };
    let mut listen: Vec<String> = vec!["[]".into()];
    for n in [1usize, 5, 6, 7, 20] {
        let fs: Vec<String> = (0..n).map(|i| eine_frage(i, &text(100, "ä"))).collect();
        listen.push(format!("[{}]", fs.join(", ")));
    }
    for laenge in [0usize, 1, 89, 90, 91, 200] {
        for zeichen in ["a", "ä", "😀", "\\u0000", "\\\""] {
            listen.push(format!("[{}]", eine_frage(1, &text(laenge, zeichen))));
        }
    }
    listen.extend(
        [
            "[{\"feld_id\": \"x\"}]",
            "[{\"feld_id\": \"x\", \"fragetext_laie\": null}]",
            "[{\"feld_id\": \"x\", \"fragetext_laie\": \"\", \"instanz_anzahl\": null}]",
            "[{\"fragetext_laie\": \"nur Text\"}]",
            "[{\"feld_id\": 5, \"fragetext_laie\": \"t\", \"instanz_anzahl\": 1.5}]",
            "[{}, {}]",
            "[{\"feld_id\": \"a\", \"fragetext_laie\": \"t\", \"instanz_anzahl\": 2, \"extra\": 1}]",
        ]
        .map(str::to_owned),
    );
    for liste in &listen {
        let fragen: Vec<PyWert> = match pw(liste) {
            PyWert::Liste(l) => l,
            anders => panic!("{anders:?}"),
        };
        for n in [0usize, 1, 6, 100] {
            let rust = json!(flow::dumps(&flow::kopf_der_queue(&fragen, n)));
            let py = frage(&json!({"fn": "flow.kopf", "fragen": liste, "wie_viele": n}));
            z.pruefe(&format!("kopf({liste}, {n})"), &rust, &py);
        }
    }
    z.fertig("kopf_der_queue", 150);
}

/// Eine vorhandene Datei bleibt, wie sie ist: Rechte (0644, 0600, 0400 = nicht beschreibbar) und
/// Inhalt; die neue Zeile haengt an, oder — bei 0400 — es geschieht still nichts.
#[test]
fn bestehende_datei() {
    if skip() {
        return;
    }
    let mut z = Zaehler::default();
    for modus in [0o644u32, 0o600, 0o400, 0o666] {
        for alt in ["", "alt\n", "a\nb\n", "{\"nicht\": \"unsere zeile\"}\n"] {
            let lauf = Lauf {
                vorher: Some((alt.to_owned(), modus)),
                ..Lauf::an()
            };
            let ((), rust) =
                lauf.rust(|a| flow::schreibe(a, Some("f1"), "antwort", &pw("{\"a\": 1}")));
            let py = frage(&lauf.anfrage(
                "flow.schreibe",
                json!({"fall": "f1", "art": "antwort", "inhalt": "{\"a\": 1}"}),
            ))["datei"]
                .clone();
            z.pruefe(&format!("bestehend {modus:o} {alt:?}"), &rust, &py);
            assert_eq!(rust["modus"], modus, "Rechte bleiben: {rust}");
        }
    }
    z.fertig("bestehende_datei", 16);
}

/// Die zwei Stellen, an denen Rust bewusst von Python abweicht (Kopf von `flow.rs`, PARITAET):
/// Ganzzahlen jenseits von `u64` und `-0`. Der Test haelt fest, WAS jede Seite schreibt — wer die
/// Abweichung schliesst, muss ihn mit aendern.
#[test]
fn dokumentierte_abweichungen() {
    if skip() {
        return;
    }
    let schreibe = |inhalt: &str| {
        let lauf = Lauf::an();
        let ((), rust) = lauf.rust(|a| flow::schreibe(a, Some("f"), "antwort", &pw(inhalt)));
        let py = frage(&lauf.anfrage(
            "flow.schreibe",
            json!({"fall": "f", "art": "antwort", "inhalt": inhalt}),
        ))["datei"]
            .clone();
        (rust["zeilen"][0].clone(), py["zeilen"][0].clone())
    };
    let ende = |zeile: &Value| {
        zeile
            .as_str()
            .unwrap()
            .rsplit_once("\"inhalt\": ")
            .unwrap()
            .1
            .to_owned()
    };

    // D1: Python haelt jede Ganzzahl, Rust ab u64::MAX + 1 nur noch einen f64.
    let (rs, py) = schreibe("18446744073709551616");
    assert_eq!(ende(&py), "18446744073709551616}");
    assert_eq!(ende(&rs), "1.8446744073709552e+19}");
    let (rs, py) = schreibe("-9223372036854775809");
    assert_eq!(ende(&py), "-9223372036854775809}");
    assert_eq!(ende(&rs), "-9.223372036854776e+18}");
    // Genau an der Grenze stimmt es noch (steht auch in `randfaelle`).
    let (rs, py) = schreibe("18446744073709551615");
    assert_eq!(ende(&rs), ende(&py));

    // `-0`: Python liest eine Ganzzahl (`0`), `serde_json` einen f64 (`-0.0`).
    let (rs, py) = schreibe("-0");
    assert_eq!(ende(&py), "0}");
    assert_eq!(ende(&rs), "-0.0}");
    // `-0.0` bleibt auf beiden Seiten gleich.
    let (rs, py) = schreibe("-0.0");
    assert_eq!(ende(&rs), ende(&py));
}
