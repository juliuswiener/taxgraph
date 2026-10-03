//! Paritaet der Feld-Kennungs-Regel: dieselbe Eingabe an fuenf Rust-Funktionen und an die Python-Referenz
//! (`tools/parity/feld_kennung_oracle.py`). Backlog `feld-kennungs-regel-steht-im-port-dreimal-verschieden`,
//! Entscheidung `feld-kennung-folgt-der-schema-regel-und-instanz-eins-bleibt-gepinnt`.
//!
//! | Rust | Python-Referenz |
//! |---|---|
//! | `bindung::ist_gueltige_feld_id` | `pattern` von `feld_id` in `store/schema.json` und `bindung/schema.json`, wie `jsonschema` es anwendet |
//! | `domain::BasisId::new` | die Schema-Regel, und `parse_instanz` liest den Namen nicht als Instanz |
//! | `domain::FeldId::from_str` | Traverser-Konvention: `instanz_feld_id` + Rundreise (`feld_id` im Orakel) |
//! | `elster::parse_instanz` | `est_mapping.parse_instanz` |
//! | `store::instanz_basis` | die Basis aus `est_mapping.parse_instanz` |
//!
//! Jede Abweichung ist entweder keine (die Regel ist gleich) oder hier BENANNT: mit Klasse, Grund und
//! einer Probe in beide Richtungen -- ein Fall, der abweicht und in keiner Klasse steht, ist ein Fehler, und
//! ein Fall, der in einer Klasse steht und NICHT abweicht, auch (sonst verrottet die Liste). Eingaben: die
//! benannten Faelle der Entscheidung, alle echten `feld_id` der Bindung, jeder Zaehler 0..=120 mit
//! Zeilenumbruch und fuehrender Null, Laengen um die 64, und 20 000 Zufallstexte.
//!
//! `PARITY=1 cargo test -p parity --test feld_kennung_paritaet -- --test-threads 3 --nocapture`
#![allow(
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::type_complexity
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::sync::{Mutex, MutexGuard, OnceLock};

use domain::{BasisId, FeldId};
use parity::Oracle;
use proptest::prelude::RngCore;
use proptest::test_runner::{RngAlgorithm, TestRng};
use serde_json::{json, Value};

/// `(basis, zaehler als Text)`; der Text, weil Python den Zaehler unbeschraenkt fuehrt.
type Paar = Option<(String, String)>;

fn skip() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn oracle() -> MutexGuard<'static, Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Ein Orakel-Aufruf; die aeussere Huelle `{"ok": ..}` wird entfernt (ein `err` dort ist ein Harness-Fehler,
/// keine Paritaetsaussage).
fn frage(fn_: &str, ids: &[String]) -> Value {
    let antwort = oracle()
        .call_json(&json!({"fn": fn_, "ids": ids}))
        .expect("Orakel antwortet");
    match antwort.get("ok") {
        Some(v) => v.clone(),
        None => panic!("Orakel-Harness-Fehler fuer {fn_}: {antwort}"),
    }
}

fn py_bool(ids: &[String]) -> Vec<bool> {
    frage("feld_kennung.schema", ids)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_bool().unwrap())
        .collect()
}

fn py_paare(fn_: &str, ids: &[String]) -> Vec<Paar> {
    let antwort = frage(fn_, ids);
    let liste = antwort.as_array().unwrap();
    assert_eq!(
        liste.len(),
        ids.len(),
        "{fn_}: Antwort und Eingabe verschieden lang"
    );
    liste
        .iter()
        .map(|v| {
            v.as_array().map(|p| {
                (
                    p[0].as_str().unwrap().to_owned(),
                    p[1].as_str().unwrap().to_owned(),
                )
            })
        })
        .collect()
}

// ------------------------------------------------------------------------------------------ die fuenf Funktionen

fn r_schema(s: &str) -> bool {
    bindung::ist_gueltige_feld_id(s)
}

fn r_basis_id(s: &str) -> bool {
    BasisId::new(s).is_ok()
}

fn r_feld_id(s: &str) -> Paar {
    s.parse::<FeldId>()
        .ok()
        .map(|f| (f.basis().as_str().to_owned(), f.instanznummer().to_string()))
}

fn r_parse_instanz(s: &str) -> Paar {
    elster::parse_instanz(s).map(|(b, n)| (b.to_owned(), n.to_string()))
}

fn r_instanz_basis(s: &str) -> Option<String> {
    store::instanz_basis(s).map(str::to_owned)
}

// ------------------------------------------------------------------------------------------ Klassen der Abweichung

/// `s` endet auf `__<Ziffern>` (nichtleere Basis davor): sieht aus wie eine Instanz-Kodierung.
fn ziffernsuffix(s: &str) -> bool {
    s.rsplit_once("__").is_some_and(|(b, z)| {
        !b.is_empty() && !z.is_empty() && z.bytes().all(|c| c.is_ascii_digit())
    })
}

/// Pythons `$` passt auch vor EINEM abschliessenden `\n`; `jsonschema` nimmt `"x\n"` also als Namen an. Rust
/// nimmt ihn nicht an: ein `BasisId` mit Zeilenumbruch waere ein Typ, der seine Regel bricht. Keine echte
/// Kennung enthaelt einen (unten geprueft); die Richtung ist fail-closed.
const ZEILENENDE: &str = "Zeilenumbruch am Ende (Pythons `$`)";

/// Eine Basis, die auf `__<Ziffern>` endet, ohne dass `parse_instanz` sie als Instanz liest (`x__1`, `x__0`,
/// `x__02`, auch `a__1` in `a__1__2`), ist in Python ein gueltiger Name: das Schema laesst ihn zu, und nur
/// `n >= 2` ist eine Instanz. Rust weist ihn ab, damit eine Basis nie wie eine Instanz aussieht (Entscheidung
/// Punkt 1, "die Regel kein `__<Zahl>`-Suffix bleibt"). Keine echte Kennung endet so (unten geprueft). Das ist
/// die benannte Abweichung bei `x__1` (F2, `REWRITE_PLAN.md` §4). `parse_instanz` und `instanz_basis` lesen
/// `x__1` nicht mehr als Instanz, `FeldId` auch nicht: darin sind alle einig; der Unterschied ist nur, dass
/// Python `x__1` als NAME gelten laesst und `BasisId`/`FeldId` nicht.
const ZIFFERNSUFFIX: &str = "Basis endet auf __<Ziffern> (Rust-Strenge, F2)";

/// `FeldId` fuehrt den Zaehler als `u16`; Python unbeschraenkt (`antwort.rs` nennt es). Kein Zaehler im Betrieb
/// kommt in die Naehe (`instanz_gruppe.max`).
const U16: &str = "Zaehler ueber u16::MAX (FeldId)";

/// `elster::parse_instanz` liefert den Zaehler als `u64`; Python unbeschraenkt.
const U64: &str = "Zaehler ueber u64::MAX (parse_instanz)";

fn zaehler_ueber(n: &str, grenze: u128) -> bool {
    n.parse::<u128>().map_or(true, |v| v > grenze)
}

/// Vergleicht Rust und Python je Eingabe. `erlaubt` nennt die Klasse einer Abweichung (oder `None`).
fn pruefe<T: PartialEq + Debug>(
    name: &str,
    ids: &[String],
    py: &[T],
    rs: impl Fn(&str) -> T,
    erlaubt: impl Fn(&str, &T) -> Option<&'static str>,
) -> BTreeMap<&'static str, usize> {
    assert_eq!(ids.len(), py.len(), "{name}");
    let (mut unerklaert, mut unzutreffend) = (Vec::new(), Vec::new());
    let mut klassen: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (s, p) in ids.iter().zip(py) {
        let r = rs(s);
        match (r == *p, erlaubt(s, p)) {
            (true, None) => {}
            (true, Some(k)) => {
                unzutreffend.push(format!("{s:?}: Klasse {k:?}, aber Rust = Python = {r:?}"));
            }
            (false, Some(k)) => *klassen.entry(k).or_default() += 1,
            (false, None) => unerklaert.push(format!("{s:?}: rust={r:?} python={p:?}")),
        }
    }
    println!(
        "  {name}: {} Eingaben, benannte Abweichungen {klassen:?}, unerklaert {}, Klasse ohne Abweichung {}",
        ids.len(),
        unerklaert.len(),
        unzutreffend.len()
    );
    for u in unerklaert.iter().chain(&unzutreffend).take(25) {
        println!("    ABWEICHUNG {name} {u}");
    }
    assert!(
        unerklaert.is_empty(),
        "{name}: {} unerklaerte Abweichungen",
        unerklaert.len()
    );
    assert!(
        unzutreffend.is_empty(),
        "{name}: {} Klassen ohne Abweichung",
        unzutreffend.len()
    );
    klassen
}

// ------------------------------------------------------------------------------------------ Eingaben

fn bindung_ids() -> BTreeSet<String> {
    let reg = bindung::lade_registry(&repo_root().join("produkt/bindung")).expect("bindung laedt");
    reg.dateien
        .into_iter()
        .flat_map(|(_, d)| d.bindungen)
        .map(|b| b.feld_id)
        .collect()
}

fn eingaben(echte: &BTreeSet<String>) -> Vec<String> {
    let basis = "vv_einnahmen";
    let mut ids: Vec<String> = Vec::new();
    // Die benannten Faelle der Entscheidung (Punkt 3).
    for suffix in [
        "", "__0", "__1", "__2", "__02", "__2\n", "__10", "__11", "__100", "\n",
    ] {
        ids.push(format!("{basis}{suffix}"));
    }
    for n in [0, 1, 2, 63, 64, 65, 66, 80, 200] {
        ids.push("a".repeat(n));
        ids.push(format!("{}__2", "a".repeat(n)));
    }
    for s in [
        "Vv_einnahmen",
        "vV_einnahmen",
        "2vv",
        "2vv__2",
        "Vv_einnahmen__2",
        "_vv",
        "vv-x",
        "vv x",
        "vv\u{e4}",
        "a__\u{662}",
        "__",
        "__2",
        "a__",
        "a___2",
        "a__2 ",
        " a__2",
        "a__1__2",
        "a__0__2",
        "a__02__3",
        "a__1__",
        "a__2__3",
        "A__2",
        "a\n",
        "a\n\n",
        "a__2\n\n",
        "a__65535",
        "a__65536",
        "a__70000",
        "a__18446744073709551615",
        "a__18446744073709551616",
        "a__99999999999999999999999999",
        "a__100000000000000000000",
        "a__00",
        "a__000",
    ] {
        ids.push(s.to_owned());
    }
    // Alle echten Kennungen, glatt und als Instanz (die Zaehler an den Kanten der Regel).
    for e in echte {
        ids.push(e.clone());
        for suffix in ["__0", "__1", "__2", "__9", "__10", "__02", "__2\n"] {
            ids.push(format!("{e}{suffix}"));
        }
    }
    // Jeder Zaehler 0..=120, glatt, mit fuehrender Null, mit Zeilenumbruch.
    for n in 0..=120 {
        for suffix in ["", "\n"] {
            ids.push(format!("a__{n}{suffix}"));
            ids.push(format!("a__0{n}{suffix}"));
        }
    }
    ids.sort_unstable();
    ids.dedup();
    let mut rng = TestRng::deterministic_rng(RngAlgorithm::ChaCha);
    let zeichen = [
        'a', 'b', 'z', '_', '_', '_', '0', '1', '2', '9', 'A', '\n', '\u{e4}',
    ];
    for _ in 0..20_000 {
        let len = 1 + (rng.next_u32() % 12) as usize;
        ids.push(
            (0..len)
                .map(|_| zeichen[(rng.next_u32() as usize) % zeichen.len()])
                .collect(),
        );
    }
    ids
}

// ------------------------------------------------------------------------------------------ Anker

/// Feste Antworten, auf BEIDEN Seiten geprueft: das Orakel soll nicht nur "keine" sagen. Je Zeile:
/// Eingabe | Python `schema` | Python `feld_id` | Python `parse_instanz` | Rust `ist_gueltige_feld_id` |
/// Rust `BasisId::new` | Rust `FeldId`. Rust `parse_instanz` und `instanz_basis` antworten wie Python.
const ANKER: &[(
    &str,
    bool,
    Option<(&str, &str)>,
    Option<(&str, &str)>,
    bool,
    bool,
    Option<(&str, &str)>,
)] = &[
    (
        "vv_einnahmen",
        true,
        Some(("vv_einnahmen", "1")),
        None,
        true,
        true,
        Some(("vv_einnahmen", "1")),
    ),
    ("Vv_einnahmen", false, None, None, false, false, None),
    ("2vv", false, None, None, false, false, None),
    (
        "vv_einnahmen__0",
        true,
        Some(("vv_einnahmen__0", "1")),
        None,
        true,
        false,
        None,
    ),
    (
        "vv_einnahmen__1",
        true,
        Some(("vv_einnahmen__1", "1")),
        None,
        true,
        false,
        None,
    ),
    (
        "vv_einnahmen__2",
        true,
        Some(("vv_einnahmen", "2")),
        Some(("vv_einnahmen", "2")),
        true,
        false,
        Some(("vv_einnahmen", "2")),
    ),
    (
        "vv_einnahmen__02",
        true,
        Some(("vv_einnahmen__02", "1")),
        None,
        true,
        false,
        None,
    ),
    (
        "vv_einnahmen__10",
        true,
        Some(("vv_einnahmen", "10")),
        Some(("vv_einnahmen", "10")),
        true,
        false,
        Some(("vv_einnahmen", "10")),
    ),
    // `$` vor dem Zeilenumbruch: Python liest die Instanz, `instanz_basis` und `parse_instanz` ebenso; die
    // Rundreise des Traversers (`instanz_feld_id`) erzeugt `x__2` ohne Zeilenumbruch, also ist es keine Instanz.
    (
        "vv_einnahmen__2\n",
        true,
        None,
        Some(("vv_einnahmen", "2")),
        false,
        false,
        None,
    ),
    (
        "vv_einnahmen\n",
        true,
        Some(("vv_einnahmen\n", "1")),
        None,
        false,
        false,
        None,
    ),
];

fn p(v: Option<(&str, &str)>) -> Paar {
    v.map(|(b, n)| (b.to_owned(), n.to_owned()))
}

#[test]
fn feld_kennung_gegen_python() {
    if skip() {
        return;
    }
    let echte = bindung_ids();
    // Die Bindung, die Python laedt, und die, die Rust laedt, sind dieselben Kennungen.
    let py_echte: BTreeSet<String> = frage("feld_kennung.echte", &[])
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        py_echte, echte,
        "Python und Rust laden verschiedene Kennungen"
    );
    assert!(echte.len() > 300, "nur {} echte Kennungen", echte.len());

    let ids = eingaben(&echte);
    let schema = py_bool(&ids);
    let feld = py_paare("feld_kennung.feld_id", &ids);
    let parse = py_paare("feld_kennung.parse_instanz", &ids);
    println!(
        "[feld_kennung] Eingaben {}, echte Kennungen {}, Python: Schema ja {} / nein {}, Instanz {} / keine {}",
        ids.len(),
        echte.len(),
        schema.iter().filter(|b| **b).count(),
        schema.iter().filter(|b| !**b).count(),
        parse.iter().filter(|x| x.is_some()).count(),
        parse.iter().filter(|x| x.is_none()).count(),
    );

    // Index der Eingabe, damit die Klassen-Probe die Python-Antwort der Eingabe sieht.
    let nach_eingabe: BTreeMap<&str, usize> = ids
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();
    let idx = |s: &str| nach_eingabe[s];

    // 1. bindung::ist_gueltige_feld_id gegen die Schema-Regel.
    let k_schema = pruefe(
        "bindung::ist_gueltige_feld_id",
        &ids,
        &schema,
        r_schema,
        |s, py| (*py && s.ends_with('\n')).then_some(ZEILENENDE),
    );
    // 2. BasisId::new gegen den Begriff "Basis" in Python: ein Name, den das Schema zulaesst und den
    //    `parse_instanz` NICHT als Instanz liest (`est_mapping`: "die EINE Enumerations-Wahrheit"). `x__2` ist
    //    danach keine Basis (beide Seiten einig); `x__1` und `x__0` sind eine, und Rust weist sie ab.
    let basis_py: Vec<bool> = schema
        .iter()
        .zip(&parse)
        .map(|(s, p)| *s && p.is_none())
        .collect();
    let k_basis = pruefe("BasisId::new", &ids, &basis_py, r_basis_id, |s, py| {
        if !*py {
            None
        } else if s.ends_with('\n') {
            Some(ZEILENENDE)
        } else if ziffernsuffix(s) {
            Some(ZIFFERNSUFFIX)
        } else {
            None
        }
    });
    // 3. FeldId::from_str gegen die Konvention des Traversers.
    let k_feld = pruefe("FeldId::from_str", &ids, &feld, r_feld_id, |s, py| {
        let (basis, n) = py.as_ref()?;
        if s.ends_with('\n') {
            Some(ZEILENENDE)
        } else if ziffernsuffix(basis) {
            Some(ZIFFERNSUFFIX)
        } else if zaehler_ueber(n, u128::from(u16::MAX)) {
            Some(U16)
        } else {
            None
        }
    });
    // 4. elster::parse_instanz gegen est_mapping.parse_instanz.
    let k_parse = pruefe(
        "elster::parse_instanz",
        &ids,
        &parse,
        r_parse_instanz,
        |_, py| {
            py.as_ref()
                .filter(|(_, n)| zaehler_ueber(n, u128::from(u64::MAX)))
                .map(|_| U64)
        },
    );
    // 5. store::instanz_basis gegen die Basis aus est_mapping.parse_instanz: KEINE Abweichung.
    let basis_von_py: Vec<Option<String>> = parse
        .iter()
        .map(|x| x.as_ref().map(|(b, _)| b.clone()))
        .collect();
    let k_basis_von = pruefe(
        "store::instanz_basis",
        &ids,
        &basis_von_py,
        r_instanz_basis,
        |_, _| None,
    );
    assert!(k_basis_von.is_empty());

    // Jede Klasse ist erreicht, nicht nur vorgesehen.
    for (funktion, klassen, soll) in [
        ("ist_gueltige_feld_id", &k_schema, vec![ZEILENENDE]),
        ("BasisId::new", &k_basis, vec![ZEILENENDE, ZIFFERNSUFFIX]),
        (
            "FeldId::from_str",
            &k_feld,
            vec![ZEILENENDE, ZIFFERNSUFFIX, U16],
        ),
        ("parse_instanz", &k_parse, vec![U64]),
    ] {
        let erreicht: Vec<&str> = klassen.keys().copied().collect();
        let mut soll = soll;
        soll.sort_unstable();
        assert_eq!(erreicht, soll, "{funktion}: erreichte Klassen");
        assert!(
            klassen.values().all(|n| *n >= 3),
            "{funktion}: eine Klasse mit weniger als 3 Treffern: {klassen:?}"
        );
    }

    // Keine echte Kennung faellt in eine Klasse: die Abweichungen treffen heute keine Bindung.
    for e in &echte {
        let i = idx(e);
        assert!(schema[i], "{e}: Python lehnt eine echte Kennung ab");
        assert!(
            r_schema(e) && r_basis_id(e),
            "{e}: Rust lehnt eine echte Kennung ab"
        );
        assert_eq!(feld[i], Some((e.clone(), "1".to_owned())), "{e}");
        assert_eq!(r_feld_id(e), feld[i], "{e}");
        assert!(
            parse[i].is_none() && r_parse_instanz(e).is_none() && r_instanz_basis(e).is_none(),
            "{e}"
        );
        assert!(
            !ziffernsuffix(e) && !e.ends_with('\n'),
            "{e}: echte Kennung in einer Abweichungsklasse"
        );
        // Als Instanz gelesen, auf beiden Seiten, bis zum Zaehler.
        let zwei = idx(&format!("{e}__2"));
        let soll = Some((e.clone(), "2".to_owned()));
        assert_eq!(parse[zwei], soll, "{e}__2");
        assert_eq!(r_parse_instanz(&format!("{e}__2")), soll, "{e}__2");
        assert_eq!(r_feld_id(&format!("{e}__2")), soll, "{e}__2");
        assert_eq!(
            r_instanz_basis(&format!("{e}__2")),
            Some(e.clone()),
            "{e}__2"
        );
    }

    // Die Anker, und die Tabelle der Entscheidung zum Nachlesen.
    println!("  Eingabe | Python: schema basis feld_id parse | Rust: gueltig BasisId FeldId parse_instanz instanz_basis");
    for (eingabe, py_schema, py_feld, py_parse, rs_schema, rs_basis, rs_feld) in ANKER {
        let i = idx(eingabe);
        assert_eq!(schema[i], *py_schema, "Python schema {eingabe:?}");
        assert_eq!(
            basis_py[i],
            *py_schema && py_parse.is_none(),
            "Python basis {eingabe:?}"
        );
        assert_eq!(feld[i], p(*py_feld), "Python feld_id {eingabe:?}");
        assert_eq!(parse[i], p(*py_parse), "Python parse_instanz {eingabe:?}");
        assert_eq!(
            r_schema(eingabe),
            *rs_schema,
            "Rust ist_gueltige_feld_id {eingabe:?}"
        );
        assert_eq!(r_basis_id(eingabe), *rs_basis, "Rust BasisId {eingabe:?}");
        assert_eq!(r_feld_id(eingabe), p(*rs_feld), "Rust FeldId {eingabe:?}");
        assert_eq!(
            r_parse_instanz(eingabe),
            p(*py_parse),
            "Rust parse_instanz {eingabe:?}"
        );
        assert_eq!(
            r_instanz_basis(eingabe),
            py_parse.map(|(b, _)| b.to_owned()),
            "Rust instanz_basis {eingabe:?}"
        );
        println!(
            "  {eingabe:?} | {py_schema} {} {py_feld:?} {py_parse:?} | {} {} {:?} {:?} {:?}",
            basis_py[i],
            r_schema(eingabe),
            r_basis_id(eingabe),
            r_feld_id(eingabe),
            r_parse_instanz(eingabe),
            r_instanz_basis(eingabe)
        );
    }
    // 65 x a: gueltig, keine Laengengrenze im Namen (die 64 gehoeren dem Router).
    let lang = "a".repeat(65);
    let i = idx(&lang);
    assert!(schema[i] && r_schema(&lang) && r_basis_id(&lang), "65 x a");
    assert_eq!(r_feld_id(&lang), Some((lang.clone(), "1".to_owned())));
    assert_eq!(feld[i], r_feld_id(&lang));
    println!(
        "  65 x a | {} {:?} | {} {} {:?}",
        schema[i],
        feld[i].is_some(),
        r_schema(&lang),
        r_basis_id(&lang),
        r_feld_id(&lang).is_some()
    );
}

/// Negativkontrolle: ein verfaelschtes Rust-Ergebnis MUSS als Abweichung auffallen.
#[test]
fn negativkontrolle() {
    if skip() {
        return;
    }
    let ids: Vec<String> = ["vv_einnahmen", "Vv_einnahmen", "vv_einnahmen__2"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let py = py_bool(&ids);
    assert_eq!(py, [true, false, true]);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // `Vv_einnahmen` wird angenommen, wie `BasisId::new` es vor dem Bau tat.
        pruefe(
            "negativ",
            &ids,
            &py,
            |s| s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
            |_, _| None,
        )
    }));
    assert!(r.is_err(), "die Verfaelschung blieb unbemerkt");
}
