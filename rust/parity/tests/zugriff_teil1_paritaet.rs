//! Paritaet der Accessor-Schicht Teil 1 (`engine::zugriff::teil1`) gegen
//! `produkt/engine/runner.py::catala_*` (bis `catala_p7_linear_afa`). `REWRITE_PLAN.md` §5.
//!
//! Je Funktion drei Nachweise:
//! (a) Korpus-Replay: jeder Datensatz aus `rust/fixtures/corpus/runner/catala_<name>.*.jsonl`
//!     (rohe Dict-Argumente aus der Unit-Suite): Rust == aufgezeichnet UND Rust == Orakel live.
//! (b) 1000 generierte Dicts (deterministischer Seed) gegen das Orakel, inkl. fehlender Schluessel
//!     (Python-Defaults) und Fehlerparitaet (Ausnahmeklasse).
//! (c) Negativkontrolle: ein um 1 gestoertes Ergebnis und ein Ok/Err-Tausch muessen als
//!     Abweichung zaehlen.
//!
//! Braucht `python3` mit dem Repo-Umfeld -- ohne `PARITY=1` SKIP:
//!
//!   `PARITY`=1 `cargo` test -p parity --test `zugriff_teil1_paritaet` -- --nocapture
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

#[path = "zugriff_teil1/adapter.rs"]
mod adapter;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use bindung::Params;
use domain::{Cent, Euro};
use engine::zugriff::teil1::fehler::EngineFehler;
use engine::zugriff::teil1::{
    afa, belastungen, einkuenfte, ermaessigungen, mobilitaetspraemie, pauschbetraege,
    sonderausgaben, werbungskosten,
};
use parity::Oracle;
use proptest::prelude::*;
use proptest::sample::select;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use serde_json::{json, Value};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip_ohne_parity_env() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

/// EIN Orakel-Prozess je Testbinary (siehe `engine_catala_paritaet.rs`).
fn oracle() -> MutexGuard<'static, Oracle> {
    static ORACLE: OnceLock<Mutex<Oracle>> = OnceLock::new();
    ORACLE
        .get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn params() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(|| Params::lade(&repo_root()).expect("params/ laedt"))
}

/// Ergebnis oder Ausnahmeklasse; Catala-Laufzeitausnahmen als eine Familie `CatalaError`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Ausgang {
    Ok(i64),
    Err(String),
}

const CATALA_KLASSEN: [&str; 10] = [
    "CatalaError",
    "AssertionFailed",
    "NoValue",
    "Conflict",
    "DivisionByZero",
    "ListEmpty",
    "NotSameLength",
    "UncomparableValues",
    "DateError",
    "Impossible",
];

fn aus_python(v: &Value) -> Ausgang {
    if let Some(ok) = v.get("ok") {
        return Ausgang::Ok(
            ok.as_i64()
                .unwrap_or_else(|| panic!("Ergebnis kein i64: {v}")),
        );
    }
    let name = v["err"]
        .as_str()
        .unwrap_or_else(|| panic!("weder ok noch err: {v}"));
    if v.get("catala") == Some(&Value::Bool(true)) || CATALA_KLASSEN.contains(&name) {
        Ausgang::Err("CatalaError".into())
    } else {
        Ausgang::Err(name.into())
    }
}

fn lauf<E>(
    eingabe: Result<E, adapter::PyFehler>,
    f: impl FnOnce(E) -> Result<i64, EngineFehler>,
) -> Ausgang {
    match eingabe {
        Err(py) => Ausgang::Err(py.into()),
        Ok(e) => match f(e) {
            Ok(v) => Ausgang::Ok(v),
            Err(fehler) => Ausgang::Err(
                fehler
                    .python_typ()
                    .map_or_else(|| format!("RustOnly: {fehler}"), String::from),
            ),
        },
    }
}

/// Ruft den Rust-Accessor zum Python-Namen `name` mit rohen Argumenten.
fn rust_ausgang(name: &str, args: &[Value]) -> Ausgang {
    use adapter as a;
    let p = params();
    match name {
        "catala_raumkosten" => lauf(a::raumkosten(args), |e| {
            werbungskosten::raumkosten(&e, p).map(Euro::get)
        }),
        "catala_grundfreibetrag" => lauf(a::jahr(args), |vz| {
            pauschbetraege::grundfreibetrag(vz, p).map(Euro::get)
        }),
        "catala_arbeitnehmer_pauschbetrag" => lauf(a::jahr(args), |vz| {
            pauschbetraege::arbeitnehmer_pauschbetrag(vz, p).map(Euro::get)
        }),
        "catala_entfernungspauschale" => lauf(a::entfernungspauschale(args), |e| {
            werbungskosten::entfernungspauschale(&e, p).map(Euro::get)
        }),
        "catala_ep_ab_21km" => lauf(a::entfernungspauschale(args), |e| {
            werbungskosten::ep_ab_21km(&e, p).map(Euro::get)
        }),
        "catala_p101_mobilitaetspraemie" => lauf(a::p101(args), |e| {
            mobilitaetspraemie::p101_mobilitaetspraemie(&e).map(Euro::get)
        }),
        "catala_p101_mobilitaetspraemie_cent" => lauf(a::p101(args), |e| {
            mobilitaetspraemie::p101_mobilitaetspraemie_cent(&e).map(Cent::get)
        }),
        "catala_werbungskosten_n" => lauf(a::werbungskosten_n(args), |e| {
            werbungskosten::werbungskosten_n(&e, p).map(Euro::get)
        }),
        _ => rust_ausgang_2(name, args),
    }
}

fn rust_ausgang_2(name: &str, args: &[Value]) -> Ausgang {
    use adapter as a;
    match name {
        "catala_vermietung_einkuenfte" => lauf(a::vermietung_einkuenfte(args), |e| {
            einkuenfte::vermietung_einkuenfte(&e).map(Euro::get)
        }),
        "catala_einkuenfte_nichtselbststaendig" => {
            lauf(a::einkuenfte_nichtselbststaendig(args), |e| {
                einkuenfte::einkuenfte_nichtselbststaendig(&e).map(Euro::get)
            })
        }
        "catala_p35a_haushaltsnahe" => lauf(a::p35a_haushaltsnahe(args), |e| {
            ermaessigungen::p35a_haushaltsnahe(&e).map(Euro::get)
        }),
        "catala_p3_nr72_photovoltaik" => lauf(a::p3_nr72_photovoltaik(args), |e| {
            einkuenfte::p3_nr72_photovoltaik(&e).map(Euro::get)
        }),
        "catala_p10b_spenden" => lauf(a::p10b_spenden(args), |e| {
            sonderausgaben::p10b_spenden(&e).map(Euro::get)
        }),
        "catala_p33_zumutbar" => lauf(a::p33_zumutbar(args), |e| {
            belastungen::p33_zumutbar(&e).map(Euro::get)
        }),
        "catala_p33_agb" => lauf(a::p33_agb(args), |e| {
            belastungen::p33_agb(&e).map(Euro::get)
        }),
        "catala_p10_kist" => lauf(a::p10_kist(args), |e| {
            sonderausgaben::p10_kist(&e).map(Euro::get)
        }),
        "catala_p10_4b_erstattungsueberhang" => lauf(a::p10_kist(args), |e| {
            sonderausgaben::p10_4b_erstattungsueberhang(&e).map(Euro::get)
        }),
        "catala_kist" => lauf(a::kist(args), |e| ermaessigungen::kist(&e).map(Cent::get)),
        "catala_p36_abschlusszahlung" => lauf(a::p36_abschlusszahlung(args), |e| {
            ermaessigungen::p36_abschlusszahlung(&e).map(Cent::get)
        }),
        _ => rust_ausgang_3(name, args),
    }
}

fn rust_ausgang_3(name: &str, args: &[Value]) -> Ausgang {
    use adapter as a;
    let p = params();
    match name {
        "catala_p24a_altersentlastung" => lauf(a::p24a_altersentlastung(args), |e| {
            ermaessigungen::p24a_altersentlastung(&e, p).map(Euro::get)
        }),
        "catala_p24b_entlastung" => lauf(a::p24b_entlastung(args), |e| {
            ermaessigungen::p24b_entlastung(&e).map(Euro::get)
        }),
        "catala_p31_familienleistung" => lauf(a::p31_familienleistung(args), |e| {
            ermaessigungen::p31_familienleistung(&e).map(Euro::get)
        }),
        "catala_p21_2_verbilligt" => lauf(a::p21_2_verbilligt(args), |e| {
            einkuenfte::p21_2_verbilligt(&e).map(Euro::get)
        }),
        "catala_p10_kv_pv" => lauf(a::p10_kv_pv(args), |e| {
            sonderausgaben::p10_kv_pv(&e).map(Euro::get)
        }),
        "catala_p10_1_7_berufsausbildung" => lauf(a::p10_1_7_berufsausbildung(args), |e| {
            sonderausgaben::p10_1_7_berufsausbildung(&e).map(Euro::get)
        }),
        "catala_p16_4_freibetrag" => lauf(a::p16_4_freibetrag(args), |e| {
            einkuenfte::p16_4_freibetrag(&e).map(Euro::get)
        }),
        "catala_euer_gewinn" => lauf(a::euer_gewinn(args), |e| {
            einkuenfte::euer_gewinn(&e).map(Euro::get)
        }),
        "catala_mitunternehmer_einkuenfte" => lauf(a::mitunternehmer_einkuenfte(args), |e| {
            einkuenfte::mitunternehmer_einkuenfte(&e).map(Euro::get)
        }),
        "catala_p6_2_gwg" => lauf(a::p6_2_gwg(args), |e| afa::p6_2_gwg(&e).map(Euro::get)),
        "catala_p7_linear_afa" => lauf(a::p7_linear_afa(args), |e| {
            afa::p7_linear_afa(&e).map(Euro::get)
        }),
        other => panic!("kein Rust-Accessor fuer {other}"),
    }
}

fn live(name: &str, args: &[Value]) -> Ausgang {
    aus_python(
        &oracle()
            .call_runner(name, args)
            .expect("Orakel-Aufruf laeuft durch"),
    )
}

/// Alle Korpus-Datensaetze `catala_<name>.*.jsonl` (ohne Kopfzeile).
fn korpus(name: &str) -> Vec<Value> {
    let dir = repo_root().join("rust/fixtures/corpus/runner");
    let praefix = format!("{name}.");
    let mut dateien: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&praefix)
        })
        .collect();
    dateien.sort();
    let mut out = Vec::new();
    for d in dateien {
        for zeile in std::fs::read_to_string(&d)
            .unwrap()
            .lines()
            .filter(|z| !z.trim().is_empty())
        {
            let v: Value = serde_json::from_str(zeile).unwrap();
            if v.get("kopf").is_none() {
                assert!(
                    v["kwargs"]
                        .as_object()
                        .is_none_or(serde_json::Map::is_empty),
                    "kwargs nicht abgebildet: {v}"
                );
                out.push(v);
            }
        }
    }
    out
}

/// Abweichung zweier Ausgaenge (auch fuer die Negativkontrolle).
fn weicht_ab(rust: &Ausgang, python: &Ausgang) -> bool {
    rust != python
}

fn pruefe(name: &str, generator: &BoxedStrategy<Vec<Value>>) {
    if skip_ohne_parity_env() {
        eprintln!("SKIP {name} (PARITY=1 nicht gesetzt)");
        return;
    }
    let mut beispiele = Vec::new();
    // (a) Korpus
    let saetze = korpus(name);
    let (mut diff_aufz, mut diff_live, mut aufz_ungleich_live) = (0usize, 0usize, 0usize);
    for satz in &saetze {
        let args = satz["args"].as_array().unwrap();
        let aufgezeichnet = aus_python(satz);
        let py = live(name, args);
        let rust = rust_ausgang(name, args);
        if weicht_ab(&rust, &aufgezeichnet) {
            diff_aufz += 1;
            beispiele.push(format!(
                "KORPUS {args:?}: rust={rust:?} aufgezeichnet={aufgezeichnet:?}"
            ));
        }
        if weicht_ab(&rust, &py) {
            diff_live += 1;
            beispiele.push(format!("KORPUS {args:?}: rust={rust:?} live={py:?}"));
        }
        if aufgezeichnet != py {
            aufz_ungleich_live += 1;
        }
    }
    // (b) generiert
    let mut runner = TestRunner::deterministic();
    let (mut diff_gen, mut fehlerfaelle, mut n_gen) = (0usize, 0usize, 0usize);
    for _ in 0..1000 {
        let args = generator.new_tree(&mut runner).unwrap().current();
        let py = live(name, &args);
        let rust = rust_ausgang(name, &args);
        n_gen += 1;
        if matches!(py, Ausgang::Err(_)) {
            fehlerfaelle += 1;
        }
        if weicht_ab(&rust, &py) {
            diff_gen += 1;
            beispiele.push(format!("GENERIERT {args:?}: rust={rust:?} live={py:?}"));
        }
    }
    println!(
        "[paritaet] {name}: korpus {} (diff aufgezeichnet {diff_aufz}, diff live {diff_live}, \
         aufgezeichnet!=live {aufz_ungleich_live}); generiert {n_gen} (davon Python-Fehler {fehlerfaelle}), diff {diff_gen}",
        saetze.len()
    );
    for b in beispiele.iter().take(5) {
        println!("    {b}");
    }
    assert_eq!(
        diff_aufz + diff_live + diff_gen,
        0,
        "{name}: Abweichungen, siehe oben"
    );
}

// ---- Generatoren ---------------------------------------------------------------------------

type Feld = BoxedStrategy<Option<(String, Value)>>;

/// Schluessel `k` mit Wahrscheinlichkeit `p` vorhanden.
fn f(k: &'static str, p: f64, s: BoxedStrategy<Value>) -> Feld {
    prop::option::weighted(p, s)
        .prop_map(move |v| v.map(|v| (k.to_string(), v)))
        .boxed()
}

fn d(felder: Vec<Feld>) -> BoxedStrategy<Vec<Value>> {
    felder
        .prop_map(|v| vec![Value::Object(v.into_iter().flatten().collect())])
        .boxed()
}

/// Ganzzahl aus `lo..hi`, plus Zonengrenzen und ihre Nachbarn.
fn g(lo: i64, hi: i64, grenzen: &[i64]) -> BoxedStrategy<Value> {
    let mut rand: Vec<i64> = grenzen.iter().flat_map(|&x| [x - 1, x, x + 1]).collect();
    rand.extend([0, lo, hi - 1]);
    prop_oneof![3 => (lo..hi).prop_map(Value::from), 1 => select(rand).prop_map(Value::from)]
        .boxed()
}

fn b() -> BoxedStrategy<Value> {
    any::<bool>().prop_map(Value::from).boxed()
}

fn vz() -> BoxedStrategy<Value> {
    prop_oneof![9 => select(vec![2024i64, 2025, 2026]), 1 => select(vec![2023i64, 2027])]
        .prop_map(Value::from)
        .boxed()
}

fn flag() -> BoxedStrategy<Value> {
    prop_oneof![
        Just(json!({})),
        any::<bool>().prop_map(|w| json!({"wert": w, "zustand": "bestaetigt"})),
        Just(json!({"wert": null})),
    ]
    .boxed()
}

fn km() -> BoxedStrategy<Value> {
    prop_oneof![
        g(0, 150, &[20, 21]),
        (0u32..1500).prop_map(|x| Value::from(f64::from(x) / 10.0)),
    ]
    .boxed()
}

fn ep_felder(p: f64) -> Vec<Feld> {
    vec![
        f("veranlagungszeitraum", 0.97, vz()),
        f("entfernung_km_roh", p, km()),
        f("arbeitstage", 0.95, g(-3, 260, &[230])),
        f("eigenes_oder_ueberlassenes_kfz", 0.8, b()),
        f("oepnv_kosten_jahr", 0.7, g(0, 8000, &[4500])),
    ]
}

// ---- (a)+(b) je Funktion -------------------------------------------------------------------

#[test]
fn raumkosten() {
    pruefe(
        "catala_raumkosten",
        &d(vec![
            f("veranlagungszeitraum", 0.97, vz()),
            f("arbeitszimmer_vorhanden", 0.8, b()),
            f("ist_mittelpunkt", 0.8, b()),
            f("tatsaechliche_aufwendungen", 0.8, g(-100, 5000, &[1260])),
            f("jahrespauschale_gewaehlt", 0.8, b()),
            f("monate_ohne_mittelpunkt", 0.7, g(0, 13, &[12])),
            f("homeoffice_tage", 0.8, g(0, 300, &[210])),
        ]),
    );
}

#[test]
fn grundfreibetrag() {
    pruefe(
        "catala_grundfreibetrag",
        &(2022i64..2028).prop_map(|j| vec![Value::from(j)]).boxed(),
    );
}

#[test]
fn arbeitnehmer_pauschbetrag() {
    pruefe(
        "catala_arbeitnehmer_pauschbetrag",
        &(2022i64..2028).prop_map(|j| vec![Value::from(j)]).boxed(),
    );
}

#[test]
fn entfernungspauschale() {
    pruefe("catala_entfernungspauschale", &d(ep_felder(0.97)));
}

#[test]
fn ep_ab_21km() {
    pruefe("catala_ep_ab_21km", &d(ep_felder(0.97)));
}

fn p101_felder() -> BoxedStrategy<Vec<Value>> {
    d(vec![
        f("entfernungspauschale_ab_21km", 0.9, g(-100, 3000, &[0])),
        f(
            "zu_versteuerndes_einkommen",
            0.9,
            g(-5000, 20_000, &[11_784, 12_096, 12_348]),
        ),
        f(
            "grundfreibetrag",
            0.9,
            select(vec![0i64, 11_784, 12_096, 12_348, 24_192])
                .prop_map(Value::from)
                .boxed(),
        ),
        f("ist_arbeitnehmer", 0.8, b()),
        f("werbungskosten_gesamt", 0.8, g(0, 4000, &[1230])),
        f(
            "arbeitnehmer_pauschbetrag",
            0.8,
            select(vec![0i64, 1230]).prop_map(Value::from).boxed(),
        ),
    ])
}

#[test]
fn p101_mobilitaetspraemie() {
    pruefe("catala_p101_mobilitaetspraemie", &p101_felder());
}

#[test]
fn p101_mobilitaetspraemie_cent() {
    pruefe("catala_p101_mobilitaetspraemie_cent", &p101_felder());
}

#[test]
fn werbungskosten_n() {
    let mut felder = ep_felder(0.5);
    felder.extend([
        f("unterkunftskosten_monat", 0.4, g(-10, 3000, &[1000, 2000])),
        f("monate", 0.8, g(-1, 13, &[12])),
        f("im_inland", 0.6, b()),
        f("tage_24h", 0.3, g(-2, 120, &[])),
        f("tage_an_abreise", 0.3, g(-2, 60, &[])),
        f("tage_ueber_8h_eintaegig", 0.3, g(-2, 60, &[])),
        f("vpf_tage_24h_nach_drei_monaten", 0.3, g(-2, 60, &[])),
        f("vpf_tage_an_abreise_nach_drei_monaten", 0.2, g(-2, 30, &[])),
        f("vpf_tage_ueber_8h_nach_drei_monaten", 0.2, g(-2, 30, &[])),
        f("vpf_fruehstuecke_gestellt_anzahl", 0.4, g(-2, 150, &[])),
        f("vpf_mittagessen_gestellt_anzahl", 0.3, g(-2, 150, &[])),
        f("vpf_abendessen_gestellt_anzahl", 0.3, g(-2, 150, &[])),
        f("vpf_mahlzeiten_gezahltes_entgelt", 0.3, g(0, 50_000, &[])),
        f("vpf_steuerfreie_erstattung_betrag", 0.3, g(0, 200_000, &[])),
        f(
            "uebernachtung_kosten_monat",
            0.4,
            g(-10, 3000, &[1000, 2000]),
        ),
        f("uebernachtung_monate", 0.8, g(-2, 13, &[12])),
        f("uebernachtung_monate_bisher", 0.7, g(-2, 70, &[36, 48])),
        f("uebernachtung_im_inland", 0.7, b()),
        f("am_anschaffungskosten", 0.4, g(-10, 2000, &[800])),
    ]);
    pruefe("catala_werbungskosten_n", &d(felder));
}

#[test]
fn vermietung_einkuenfte() {
    let e = || g(-1000, 60_000, &[]);
    pruefe(
        "catala_vermietung_einkuenfte",
        &d(vec![
            f("einnahmen", 0.9, e()),
            f("gebaeude_afa", 0.8, e()),
            f("schuldzinsen", 0.8, e()),
            f("erhaltungsaufwand", 0.8, e()),
            f("sonstige_werbungskosten", 0.8, e()),
        ]),
    );
}

#[test]
fn einkuenfte_nichtselbststaendig() {
    pruefe(
        "catala_einkuenfte_nichtselbststaendig",
        &d(vec![
            f("bruttoarbeitslohn", 0.9, g(-1000, 150_000, &[1230])),
            f("werbungskosten", 0.8, g(0, 6000, &[1230])),
            f("veranlagungszeitraum", 0.95, vz()),
        ]),
    );
}

#[test]
fn p35a_haushaltsnahe() {
    pruefe(
        "catala_p35a_haushaltsnahe",
        &d(vec![
            f("hh_minijob_aufwendungen", 0.8, g(-100, 5000, &[2550])),
            f("hh_dienstleistungen", 0.8, g(-100, 30_000, &[20_000])),
            f("hh_handwerker_arbeitskosten", 0.8, g(-100, 10_000, &[6000])),
            f("hh_in_eu_ewr", 0.9, flag()),
            f("hh_rechnung_unbar", 0.9, flag()),
            f("hh_handwerker_keine_foerderung", 0.7, flag()),
            f("p35a_mitveranlagung", 0.6, flag()),
        ]),
    );
}

#[test]
fn p3_nr72_photovoltaik() {
    pruefe(
        "catala_p3_nr72_photovoltaik",
        &d(vec![
            f("pv_einnahmen", 0.9, g(-100, 5000, &[0])),
            f("pv_auf_gebaeude", 0.9, flag()),
            f("pv_bruttoleistung_kwp", 0.9, g(-1, 130, &[30, 60, 90, 100])),
            f("pv_anzahl_einheiten", 0.9, g(-1, 6, &[0])),
        ]),
    );
}

#[test]
fn p10b_spenden() {
    pruefe(
        "catala_p10b_spenden",
        &d(vec![
            f("zuwendungen", 0.9, g(-100, 30_000, &[])),
            f("gesamtbetrag_der_einkuenfte", 0.9, g(-20_000, 150_000, &[])),
        ]),
    );
}

fn zumutbar_felder() -> Vec<Feld> {
    vec![
        f(
            "gesamtbetrag_der_einkuenfte",
            0.9,
            g(-5000, 120_000, &[15_340, 51_130]),
        ),
        f("anzahl_kinder", 0.8, g(0, 6, &[1, 2, 3])),
        f("splitting", 0.8, b()),
    ]
}

#[test]
fn p33_zumutbar() {
    pruefe("catala_p33_zumutbar", &d(zumutbar_felder()));
}

#[test]
fn p33_agb() {
    let mut felder = zumutbar_felder();
    felder.push(f(
        "aussergewoehnliche_belastungen",
        0.9,
        g(-100, 20_000, &[]),
    ));
    pruefe("catala_p33_agb", &d(felder));
}

fn kist_felder() -> BoxedStrategy<Vec<Value>> {
    d(vec![
        f("gezahlte_kirchensteuer", 0.9, g(0, 5000, &[])),
        f("erstattete_kirchensteuer", 0.9, g(0, 5000, &[])),
    ])
}

#[test]
fn p10_kist() {
    pruefe("catala_p10_kist", &kist_felder());
}

#[test]
fn p10_4b_erstattungsueberhang() {
    pruefe("catala_p10_4b_erstattungsueberhang", &kist_felder());
}

#[test]
fn kist() {
    let konf = select(vec![
        "evangelisch",
        "roemisch-katholisch",
        "keine",
        "andere",
        "",
    ])
    .prop_map(Value::from)
    .boxed();
    let land = select(vec![
        "bayern",
        "baden_wuerttemberg",
        "berlin",
        "nordrhein_westfalen",
        "",
    ])
    .prop_map(Value::from)
    .boxed();
    pruefe(
        "catala_kist",
        &d(vec![
            f("konfession", 0.9, konf),
            f("bundesland", 0.9, land),
            f("est_mit_fb", 0.9, g(-1000, 60_000, &[])),
        ]),
    );
}

#[test]
fn p36_abschlusszahlung() {
    let c = || g(-1000, 5_000_000, &[100, 101]);
    pruefe(
        "catala_p36_abschlusszahlung",
        &d(vec![
            f("festzusetzende_est_cent", 0.9, c()),
            f("lohnsteuer_cent", 0.8, c()),
            f("kapitalertragsteuer_cent", 0.6, c()),
            f("kapitalertragsteuer_solz_cent", 0.6, c()),
            f("kapitalertragsteuer_kist_cent", 0.6, c()),
            f("vorauszahlungen_cent", 0.6, c()),
        ]),
    );
}

#[test]
fn p24a_altersentlastung() {
    pruefe(
        "catala_p24a_altersentlastung",
        &d(vec![
            f(
                "veranlagungszeitraum",
                0.8,
                select(vec![2024i64, 2025, 2026])
                    .prop_map(Value::from)
                    .boxed(),
            ),
            f(
                "geburtsjahr",
                0.9,
                g(1925, 2000, &[-1, 1939, 1940, 1959, 1960, 1961, 1993]),
            ),
            f("arbeitslohn", 0.8, g(-100, 60_000, &[])),
            f("positive_andere_einkuenfte", 0.8, g(-100, 30_000, &[])),
        ]),
    );
}

#[test]
fn p24b_entlastung() {
    pruefe(
        "catala_p24b_entlastung",
        &d(vec![
            f("alleinstehend", 0.9, b()),
            f("anzahl_kinder", 0.9, g(0, 7, &[1, 2])),
            f("monate_ohne_voraussetzung", 0.7, g(0, 13, &[12])),
        ]),
    );
}

#[test]
fn p31_familienleistung() {
    let e = || g(0, 60_000, &[]);
    pruefe(
        "catala_p31_familienleistung",
        &d(vec![
            f("est_ohne_freibetraege", 0.9, e()),
            f("est_mit_freibetraegen", 0.9, e()),
            f("kindergeld", 0.9, g(0, 10_000, &[3000])),
        ]),
    );
}

#[test]
fn p21_2_verbilligt() {
    pruefe(
        "catala_p21_2_verbilligt",
        &d(vec![
            f("werbungskosten", 0.9, g(-100, 30_000, &[])),
            f("entgelt_quote_prozent", 0.8, g(0, 150, &[50, 66, 100])),
        ]),
    );
}

#[test]
fn p10_kv_pv() {
    pruefe(
        "catala_p10_kv_pv",
        &d(vec![
            f("basis_kv_pv", 0.9, g(0, 8000, &[1900, 2800])),
            f("weitere_vorsorgeaufwendungen", 0.8, g(0, 4000, &[])),
            f("mit_anspruch_auf_zuschuss", 0.8, b()),
        ]),
    );
}

#[test]
fn p10_1_7_berufsausbildung() {
    pruefe(
        "catala_p10_1_7_berufsausbildung",
        &d(vec![f(
            "berufsausbildung_aufwendungen",
            0.9,
            g(-100, 12_000, &[6000]),
        )]),
    );
}

#[test]
fn p16_4_freibetrag() {
    pruefe(
        "catala_p16_4_freibetrag",
        &d(vec![f(
            "rentner_veraeusserungsgewinn",
            0.9,
            g(-5000, 250_000, &[136_000, 181_000]),
        )]),
    );
}

#[test]
fn euer_gewinn() {
    pruefe(
        "catala_euer_gewinn",
        &d(vec![
            f("betriebseinnahmen", 0.9, g(-1000, 150_000, &[])),
            f("betriebsausgaben", 0.9, g(-1000, 150_000, &[])),
        ]),
    );
}

#[test]
fn mitunternehmer_einkuenfte() {
    let v = || g(0, 30_000, &[]);
    pruefe(
        "catala_mitunternehmer_einkuenfte",
        &d(vec![
            f("gewinnanteil", 0.9, g(-60_000, 120_000, &[])),
            f("verguetung_taetigkeit", 0.7, v()),
            f("verguetung_darlehen", 0.7, v()),
            f("verguetung_ueberlassung", 0.7, v()),
        ]),
    );
}

#[test]
fn p6_2_gwg() {
    pruefe(
        "catala_p6_2_gwg",
        &d(vec![f(
            "gwg_anschaffungskosten_netto",
            0.9,
            g(-10, 2000, &[800]),
        )]),
    );
}

#[test]
fn p7_linear_afa() {
    pruefe(
        "catala_p7_linear_afa",
        &d(vec![
            f(
                "anschaffungskosten_cent",
                0.4,
                g(-100, 2_000_000, &[0, 99, 100]),
            ),
            f("anschaffungskosten", 0.8, g(-100, 20_000, &[0])),
            f("nutzungsdauer", 0.9, g(-1, 20, &[0, 1])),
            f("anschaffung_monat", 0.7, g(-1, 14, &[1, 12])),
            f("ist_anschaffungsjahr", 0.7, b()),
        ]),
    );
}

// ---- (c) Negativkontrolle -------------------------------------------------------------------

/// Der Vergleich MUSS eine um 1 gestoerte Zahl und einen Ok/Err-Tausch als Abweichung zaehlen,
/// sonst beweist ein "0 Abweichungen" nichts. Laeuft ueber einen echten Korpus-Datensatz.
#[test]
fn negativkontrolle() {
    if skip_ohne_parity_env() {
        eprintln!("SKIP negativkontrolle (PARITY=1 nicht gesetzt)");
        return;
    }
    let satz = korpus("catala_p10b_spenden")
        .into_iter()
        .next()
        .expect("Korpus nicht leer");
    let args = satz["args"].as_array().unwrap();
    let rust = rust_ausgang("catala_p10b_spenden", args);
    let py = live("catala_p10b_spenden", args);
    assert!(!weicht_ab(&rust, &py), "Ausgangsfall muss gleich sein");
    let Ausgang::Ok(wert) = rust else {
        panic!("Korpusfall ist kein Ok")
    };
    assert!(weicht_ab(&Ausgang::Ok(wert + 1), &py), "+1 muss auffallen");
    assert!(
        weicht_ab(&Ausgang::Err("CatalaError".into()), &py),
        "Ok/Err-Tausch muss auffallen"
    );
    println!("[paritaet] negativkontrolle: +1 und Ok/Err-Tausch als Abweichung erkannt");
}
