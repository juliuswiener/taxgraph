//! Wertwache der engine gegen das PYTHON-Orakel, hermetisch (ohne `PARITY=1`, ohne Python).
//!
//! `rust/fixtures/wertwache_orakel.json` haelt feste Grenzfaelle samt den Antworten des laufenden Orakels
//! (`runner.catala_*` ueber `tools/parity/oracle.py`), eingefroren (der Erzeuger ist geloescht; Verlauf:
//! `git show 2dd056a6:tools/parity/extract_wertwache_orakel.py`). Dieser Test spielt dieselben rohen Sachverhalt-dicts ueber dieselben Adapter wie die Parity-Suiten
//! (Kopien in `rust/engine/tests/zugriff_adapter/teil{1,2}.rs`) gegen `engine::zugriff` und vergleicht Wert oder
//! Ausnahmeklasse.
//!
//! Warum das noetig ist: der Zufallsgenerator von `zugriff_teil2_paritaet` zieht Grenzwerte nur selten und nie
//! gemeinsam. 15 Mutanten an Schwellen und Saetzen blieben ohne `PARITY=1` gruen (Messung 2026-10-04, Bericht
//! `wertwache-engine.md`): `* 119` -> `* 118` im Solz, verbleibendes zvE `< 0` -> `<= 0` in der Fuenftelregelung,
//! `erhoehte <= 0` -> `< 0` im Progressionsvorbehalt u. a. Zwei davon fand selbst `PARITY_N=100000` erst nach
//! 1 bzw. 4 Treffern. Hier stehen die Nachbarn jeder Schwelle fest im Fixture.
//!
//! Das Fixture ist gegen einen Operator-Sweep ueber die Rumpfe der elf Funktionen gehaertet (Bericht
//! `wertwache-sweep.md`): `<`/`<=`, `>`/`>=`, `==`/`!=`, `min`/`max`, `.max(0)` -> `.max(1)`, Literal +-1, `&&`/`||`;
//! 249 Mutanten, 227 rot, 21 nachweislich gleichwertig, 1 ohne Kompilat. Die Zeilen der Gitter sind je Zweig kommentiert
//! (im Verlauf des geloeschten Erzeugers, siehe oben).
//!
//! Je Funktion ein Test, damit ein roter Lauf die Stelle benennt. Die Datei wird nicht neu erzeugt.
#![allow(
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

// Die Adapter (dict -> typisierte Eingabe, mit Pythons Lesestellen) sind Kopien der Adapter der Parity-Suiten,
// bei der Anlage byte-gleich (sha256 im Commit); hier wird nur ein Teil davon gerufen. Die Originale in
// `rust/parity/tests/zugriff_teil{1,2}/adapter.rs` bleiben, bis die Parity faellt.
#[allow(dead_code)]
#[path = "zugriff_adapter/teil1.rs"]
mod adapter1;
#[allow(dead_code)]
#[path = "zugriff_adapter/teil2.rs"]
mod adapter2;

use std::path::PathBuf;
use std::sync::OnceLock;

use bindung::Params;
use engine::zugriff::teil1::{einkuenfte, mobilitaetspraemie};
use serde_json::{json, Value};

const FIXTURE: &str = include_str!("../../fixtures/wertwache_orakel.json");

/// Ausnahmeklassen des Catala-Laufzeitsystems; alle zaehlen als eine Familie (`CatalaError`).
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

/// Wert oder Ausnahmeklasse.
#[derive(Debug, Clone, PartialEq)]
enum Ausgang {
    Ok(Value),
    Err(String),
}

fn params() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(|| {
        let wurzel = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        Params::lade(&wurzel).expect("params/ laedt")
    })
}

fn fixture() -> &'static Value {
    static F: OnceLock<Value> = OnceLock::new();
    F.get_or_init(|| serde_json::from_str(FIXTURE).expect("wertwache_orakel.json ist JSON"))
}

/// Antwort des Orakels: `{"ok": v}` oder `{"err": Klasse, "catala": bool}`.
fn python(antwort: &Value) -> Ausgang {
    if let Some(v) = antwort.get("ok") {
        return Ausgang::Ok(v.clone());
    }
    let typ = antwort["err"]
        .as_str()
        .unwrap_or_else(|| panic!("weder ok noch err: {antwort}"));
    let catala = antwort.get("catala") == Some(&Value::Bool(true));
    Ausgang::Err(if catala || CATALA_KLASSEN.contains(&typ) {
        "CatalaError".to_string()
    } else {
        typ.to_string()
    })
}

/// Teil-1-Accessor: Adapter-Fehler = Pythons Ausnahmeklasse; Engine-Fehler ohne Python-Gegenstueck bleiben sichtbar.
fn teil1<E, T: Into<Value>>(
    eingabe: Result<E, &'static str>,
    f: impl FnOnce(&E) -> Result<T, engine::zugriff::teil1::fehler::EngineFehler>,
) -> Ausgang {
    match eingabe {
        Err(py) => Ausgang::Err(py.to_string()),
        Ok(e) => match f(&e) {
            Ok(v) => Ausgang::Ok(v.into()),
            Err(fehler) => Ausgang::Err(
                fehler
                    .python_typ()
                    .map_or_else(|| format!("RustOnly: {fehler}"), String::from),
            ),
        },
    }
}

/// Der Rust-Accessor zum Python-Namen `fn_name` mit rohen Argumenten.
fn rust(fn_name: &str, args: &[Value]) -> Ausgang {
    match fn_name {
        "catala_p3_nr72_photovoltaik" => teil1(adapter1::p3_nr72_photovoltaik(args), |e| {
            einkuenfte::p3_nr72_photovoltaik(e).map(|x| json!(x.get()))
        }),
        "catala_p101_mobilitaetspraemie" => teil1(adapter1::p101(args), |e| {
            mobilitaetspraemie::p101_mobilitaetspraemie(e).map(|x| json!(x.get()))
        }),
        "catala_p101_mobilitaetspraemie_cent" => teil1(adapter1::p101(args), |e| {
            mobilitaetspraemie::p101_mobilitaetspraemie_cent(e).map(|x| json!(x.get()))
        }),
        _ => {
            let name = fn_name
                .strip_prefix("catala_")
                .unwrap_or_else(|| panic!("kein catala_-Name: {fn_name}"));
            let (_, f) = adapter2::FUNKTIONEN
                .iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("kein Teil-2-Adapter fuer {name}"));
            match f(args, params()) {
                Ok(v) => Ausgang::Ok(v),
                Err(adapter2::Fehl::Py(t)) => Ausgang::Err(t.to_string()),
                Err(adapter2::Fehl::Engine(e)) => Ausgang::Err(
                    adapter2::python_typ(&e).map_or_else(|| "CatalaError".to_string(), String::from),
                ),
            }
        }
    }
}

/// Alle Faelle der Funktion `fn_name` gegen das Orakel. Liefert (Faelle, davon Antwort ungleich 0, davon Fehler).
/// Ein Lauf ohne Fall waere gruen und sagte nichts; die Aufrufer pruefen die Mindestzahl.
fn pruefe(fn_name: &str) -> (usize, usize, usize) {
    let (mut n, mut nicht_null, mut fehler) = (0, 0, 0);
    let mut abweichungen = Vec::new();
    for fall in fixture()["faelle"].as_array().expect("faelle ist ein Array") {
        if fall["fn"] != fn_name {
            continue;
        }
        let args = fall["args"].as_array().expect("args ist ein Array");
        let py = python(&fall["py"]);
        let rs = rust(fn_name, args);
        n += 1;
        match &py {
            Ausgang::Ok(v) if v != &json!(0) => nicht_null += 1,
            Ausgang::Err(_) => fehler += 1,
            Ausgang::Ok(_) => {}
        }
        if rs != py {
            abweichungen.push(format!("{} rust={rs:?} python={py:?}", fall["args"]));
        }
    }
    assert!(
        abweichungen.is_empty(),
        "{fn_name}: {} von {n} Faellen weichen vom Orakel ab, z. B.:\n{}",
        abweichungen.len(),
        abweichungen
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    (n, nicht_null, fehler)
}

/// Mindestens `faelle` Faelle, davon `rechnend` mit Antwort ungleich 0: sonst vergleicht der Test nur Nullen.
fn mit_mindestens(fn_name: &str, faelle: usize, rechnend: usize) {
    let (n, nicht_null, _) = pruefe(fn_name);
    assert!(n >= faelle, "{fn_name}: nur {n} Faelle im Fixture, erwartet >= {faelle}");
    assert!(
        nicht_null >= rechnend,
        "{fn_name}: nur {nicht_null} Faelle mit Antwort ungleich 0, erwartet >= {rechnend}"
    );
}

#[test]
fn solz_grenzen_gegen_orakel() {
    mit_mindestens("catala_solz", 300, 200);
}

#[test]
fn fuenftel_grenzen_gegen_orakel() {
    let (n, nicht_null, fehler) = pruefe("catala_fuenftel");
    assert!(n >= 140 && nicht_null >= 20, "fuenftel: {n} Faelle, {nicht_null} rechnend");
    assert!(fehler >= 10, "fuenftel: nur {fehler} Fehlerantworten (zvE <= 0 mit ao > zvE fehlt?)");
}

#[test]
fn p32b_1_grenzen_gegen_orakel() {
    mit_mindestens("catala_p32b_1", 140, 30);
}

#[test]
fn p34c_1_grenzen_gegen_orakel() {
    mit_mindestens("catala_p34c_1", 400, 90);
}

#[test]
fn kst_nenner_b_grenzen_gegen_orakel() {
    mit_mindestens("catala_kst_nenner_b", 900, 650);
}

#[test]
fn behinderten_pb_grenzen_gegen_orakel() {
    mit_mindestens("catala_behinderten_pb", 70, 40);
}

#[test]
fn p33a_unterhalt_grenzen_gegen_orakel() {
    mit_mindestens("catala_p33a_unterhalt", 400, 270);
}

#[test]
fn renten_einkuenfte_grenzen_gegen_orakel() {
    let (n, nicht_null, fehler) = pruefe("catala_renten_einkuenfte");
    assert!(n >= 800 && nicht_null >= 300, "renten: {n} Faelle, {nicht_null} rechnend");
    // Aa-Folgejahr ohne Freibetrag, Beginn nach dem VZ, nicht ringfaehige Art, fehlende Pflichtfelder
    assert!(fehler >= 200, "renten: nur {fehler} Fehlerantworten (Aa-Zweig im Fixture?)");
}

#[test]
fn photovoltaik_grenzen_gegen_orakel() {
    mit_mindestens("catala_p3_nr72_photovoltaik", 100, 30);
}

#[test]
fn mobilitaetspraemie_grenzen_gegen_orakel() {
    mit_mindestens("catala_p101_mobilitaetspraemie", 140, 40);
    mit_mindestens("catala_p101_mobilitaetspraemie_cent", 140, 40);
}

/// Jeder Fall des Fixtures gehoert zu einem der Tests oben (kein Fall ohne Pruefer).
#[test]
fn fixture_hat_nur_gepruefte_funktionen() {
    let bekannt = [
        "catala_solz",
        "catala_fuenftel",
        "catala_p32b_1",
        "catala_p34c_1",
        "catala_kst_nenner_b",
        "catala_behinderten_pb",
        "catala_p33a_unterhalt",
        "catala_renten_einkuenfte",
        "catala_p3_nr72_photovoltaik",
        "catala_p101_mobilitaetspraemie",
        "catala_p101_mobilitaetspraemie_cent",
    ];
    for fall in fixture()["faelle"].as_array().expect("faelle ist ein Array") {
        let name = fall["fn"].as_str().expect("fn ist ein String");
        assert!(bekannt.contains(&name), "Fall fuer {name} ohne Test");
    }
}
