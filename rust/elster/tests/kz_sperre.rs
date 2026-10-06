//! Kz-Sperre (Weg B voll, Stufe 3, Waechter W10): was ein Kz im ELSTER-XML am Ende wirklich
//! schreibt, und welche Zuordnung `feld_id` → Kz die Bindung traegt.
//!
//! - **Ja-Typen** (`ja_typ_nein_wird_nach_typ_geschrieben`): ein bool-Kz mit `false` wird bei
//!   `JaNein12` als `2` geschrieben (eine echte Antwort), bei einem Ankreuzfeld (`Ja1`/`JaX`/`Ja2`)
//!   weggelassen. Vorbild: `tests/test_kz_bindung_durchgang.py::test_janein12_nein_wird_geschrieben_
//!   nicht_weggelassen`. Messung 2026-10-05: ein Mutant, der den `JaNein12`-Zweig in
//!   `xml.rs::blatt_text` auf `None` setzt, liess alle 2051 Tests gruen. Das Schema liegt nur lokal;
//!   die CI sieht diese Pruefung ueber den hermetischen Gegenstueck-Test in `src/xml.rs`.
//! - **Zuordnung** (`zuordnung_feld_zu_kz_ist_bewusst`): `kz_zuordnung.tsv` haelt fest, welches Kz
//!   jedes Feld der Bindung traegt und ob das Feld `askable` (fragbar) ist. Vertauschte sich ein
//!   Paar gueltiger Kz (Lohnsteuer unter dem Kz der Kirchensteuer), blieb jeder Rust-Test gruen:
//!   nur Pins gegen Python-Antworten wurden rot (Sonde K4, 5 Tests, alle Pins). Dasselbe galt fuer
//!   `askable: true -> false` bei `kinderbetreuungskosten` (Mutant D3): das Feld wird nicht mehr
//!   gefragt, sein Kz verschwindet still aus Formular und Deklaration, und nur
//!   `stand_und_fragen_wie_python` wurde rot (zufaellig). Hermetisch, braucht kein Schema.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use bindung::Bindung;
use domain::Feldtyp;
use elster::testhilfe::schemas_da;
use elster::{deklariere, erzeuge_xml, finde_schema, kz_meta, Felder, KzMeta, XmlOptionen};
use serde_json::json;

/// Das Jahr, gegen dessen Schema geprueft wird (wie `bindungs_typ_vs_xsd_typ.rs`).
const VZ: i64 = 2025;

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        // Die Bindung des Dienstes (`rust/bindung/daten`), nie ein fester Pfad im Test.
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel)
            .expect("Bindung laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

/// Die Typ-Angaben der Kz aus E10 des Jahres [`VZ`].
fn e10_meta() -> HashMap<String, KzMeta> {
    let pfad = finde_schema(VZ, "E10-{jahr}.xsd").expect("E10-Schema fehlt");
    kz_meta(&pfad, "E10").unwrap()
}

/// Das XML einer Deklaration, die genau dieses eine Kz mit `false` traegt.
fn xml_mit_nein(kz: &str) -> String {
    let mut d = deklariere(&Felder::new(), index(), 2025, None).unwrap();
    d.deklaration.insert(kz.to_owned(), json!(false));
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        ..XmlOptionen::default()
    };
    erzeuge_xml(&d, &opt).unwrap_or_else(|e| panic!("{kz}: erzeuge_xml scheitert: {e}"))
}

/// Jede bool-Bindung mit einem Ja-Typ im Schema: `false` heisst bei `JaNein12` `<Kz>2</Kz>`, bei
/// `Ja1`/`JaX`/`Ja2` kein Element. Schleife ueber die echte Bindung, damit ein neues Ja-Kz ohne
/// Zutun mitgeprueft wird; beide Familien muessen vorkommen, sonst prueft die Schleife nichts.
#[test]
fn ja_typ_nein_wird_nach_typ_geschrieben() {
    if !schemas_da(VZ) {
        return;
    }
    let meta = e10_meta();
    let (mut janein12, mut ankreuz) = (Vec::new(), Vec::new());
    for b in bindungen().iter().filter(|b| b.typ == Feldtyp::Bool) {
        let Some(kz) = b.elster_kz.as_ref().map(domain::Kz::as_str) else {
            continue;
        };
        let Some(m) = meta.get(kz).filter(|m| m.is_ja) else {
            continue;
        };
        let xml = xml_mit_nein(kz);
        if m.type_name.starts_with("JaNein12") {
            assert!(
                xml.contains(&format!("<{kz}>2</{kz}>")),
                "{} ({kz}, {}): JaNein12-Nein fehlt im XML — die gegebene Antwort ist verschwunden",
                b.feld_id,
                m.type_name
            );
            janein12.push(kz);
        } else {
            assert!(
                !xml.contains(&format!("<{kz}")),
                "{} ({kz}, {}): ein Ankreuzfeld mit Nein darf kein Element ergeben",
                b.feld_id,
                m.type_name
            );
            ankreuz.push(kz);
        }
    }
    assert!(
        !janein12.is_empty() && !ankreuz.is_empty(),
        "die Schleife sah JaNein12 {janein12:?} und Ankreuzfelder {ankreuz:?}; beide Familien muessen vorkommen"
    );
}

// ---------------------------------------------------------------- Zuordnung feld_id -> Kz

/// Die eingefrorene Zuordnung, neben dem Test.
const ZUORDNUNG: &str = include_str!("kz_zuordnung.tsv");

/// Eine Zeile der Zuordnung: das Kz des Felds und ob das Feld fragbar (`askable`) ist.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Zeile {
    kz: String,
    askable: bool,
}

/// Die Zeilen der TSV: `feld_id <Tab> Kz <Tab> askable`; `#`-Zeilen und Leerzeilen zaehlen nicht.
/// Eine doppelte `feld_id`, eine Zeile ohne drei Spalten und ein `askable` ausser `true`/`false`
/// sind Fehler der Datei, kein Befund der Bindung.
fn lies_tsv(text: &str) -> BTreeMap<String, Zeile> {
    let mut m = BTreeMap::new();
    for (i, zeile) in text.lines().enumerate() {
        if zeile.is_empty() || zeile.starts_with('#') {
            continue;
        }
        let spalten: Vec<&str> = zeile.split('\t').collect();
        assert!(
            spalten.len() == 3,
            "kz_zuordnung.tsv Zeile {}: drei Spalten (feld_id, Kz, askable) erwartet, gefunden {}",
            i + 1,
            spalten.len()
        );
        let askable = match spalten[2] {
            "true" => true,
            "false" => false,
            anderes => panic!(
                "kz_zuordnung.tsv Zeile {}: askable muss true oder false sein, nicht {anderes:?}",
                i + 1
            ),
        };
        let vorher = m.insert(
            spalten[0].to_owned(),
            Zeile {
                kz: spalten[1].to_owned(),
                askable,
            },
        );
        assert!(
            vorher.is_none(),
            "kz_zuordnung.tsv Zeile {}: feld_id {} steht zweimal",
            i + 1,
            spalten[0]
        );
    }
    m
}

/// Was `ist` (die Registry) von `soll` (der TSV) unterscheidet, eine Zeile je Abweichung.
fn abweichungen(soll: &BTreeMap<String, Zeile>, ist: &BTreeMap<String, Zeile>) -> Vec<String> {
    let mut a = Vec::new();
    for (feld, z) in ist {
        match soll.get(feld) {
            None => a.push(format!(
                "neu: {feld} traegt in der Bindung {} (askable {}), die TSV kennt das Feld nicht",
                z.kz, z.askable
            )),
            Some(s) if s.kz != z.kz => a.push(format!(
                "geaendert: {feld} traegt in der Bindung {}, die TSV haelt {}",
                z.kz, s.kz
            )),
            Some(s) if s.askable != z.askable => a.push(format!(
                "askable geaendert: {feld} ({}) ist in der Bindung askable {}, die TSV haelt {} — \
                 ein Feld, das nicht mehr gefragt wird, verliert sein Kz still aus Formular und Deklaration",
                z.kz, z.askable, s.askable
            )),
            Some(_) => {}
        }
    }
    for (feld, s) in soll {
        if !ist.contains_key(feld) {
            a.push(format!(
                "weg: die TSV haelt {feld} -> {}, die Bindung hat fuer das Feld kein Kz mehr",
                s.kz
            ));
        }
    }
    a
}

/// Jedes Feld der Bindung mit `elster_kz` steht mit genau diesem Kz und demselben `askable` in der
/// TSV, und umgekehrt. Ein Unterschied ist ein Entscheid: Zeile aendern und den Grund in den Commit
/// schreiben.
#[test]
fn zuordnung_feld_zu_kz_ist_bewusst() {
    let ist: BTreeMap<String, Zeile> = bindungen()
        .iter()
        .filter_map(|b| {
            b.elster_kz.as_ref().map(|k| {
                (
                    b.feld_id.clone(),
                    Zeile {
                        kz: k.as_str().to_owned(),
                        askable: b.askable,
                    },
                )
            })
        })
        .collect();
    let soll = lies_tsv(ZUORDNUNG);
    assert!(
        soll.len() >= 100,
        "die TSV haelt nur {} Zeilen — leer oder abgeschnitten?",
        soll.len()
    );
    let a = abweichungen(&soll, &ist);
    assert!(
        a.is_empty(),
        "Registry und kz_zuordnung.tsv laufen auseinander ({} Abweichungen). Absicht: Zeile in \
         rust/elster/tests/kz_zuordnung.tsv anpassen und im Commit begruenden.\n{}",
        a.len(),
        a.join("\n")
    );
}

/// Der Vergleich selbst, an erfundenen Zeilen: er findet ein vertauschtes Paar (beide Kz gueltig),
/// ein verlorenes Kz, ein neues Feld und ein umgeschaltetes `askable` in beide Richtungen, und er
/// schweigt, wo nichts abweicht.
#[test]
fn zuordnung_vergleich_erkennt_tausch_verlust_neues_feld_und_askable() {
    let paar = |l: &[(&str, &str, bool)]| -> BTreeMap<String, Zeile> {
        l.iter()
            .map(|(f, k, a)| {
                (
                    (*f).to_owned(),
                    Zeile {
                        kz: (*k).to_owned(),
                        askable: *a,
                    },
                )
            })
            .collect()
    };
    let soll = paar(&[("lohn", "E1", true), ("kist", "E2", true), ("ring", "E3", false)]);
    assert!(abweichungen(&soll, &soll).is_empty());
    let getauscht = paar(&[("lohn", "E2", true), ("kist", "E1", true), ("ring", "E3", false)]);
    assert_eq!(abweichungen(&soll, &getauscht).len(), 2);
    let verloren = paar(&[("lohn", "E1", true), ("kist", "E2", true)]);
    let a = abweichungen(&soll, &verloren);
    assert_eq!(a.len(), 1, "{a:?}");
    assert!(a[0].starts_with("weg: "), "{a:?}");
    let neu = paar(&[
        ("lohn", "E1", true),
        ("kist", "E2", true),
        ("ring", "E3", false),
        ("x", "E4", true),
    ]);
    let a = abweichungen(&soll, &neu);
    assert_eq!(a.len(), 1, "{a:?}");
    assert!(a[0].starts_with("neu: "), "{a:?}");
    // fragbar -> nicht fragbar (das Feld wird nicht mehr gefragt) und nicht fragbar -> fragbar
    // (ein berechneter Wert wuerde von der Eingabe ueberschrieben).
    for (feld, kz, jetzt) in [("lohn", "E1", false), ("ring", "E3", true)] {
        let mut ist = soll.clone();
        ist.insert(feld.to_owned(), Zeile { kz: kz.to_owned(), askable: jetzt });
        let a = abweichungen(&soll, &ist);
        assert_eq!(a.len(), 1, "{feld}: {a:?}");
        assert!(a[0].starts_with("askable geaendert: "), "{a:?}");
    }
}

/// Die TSV selbst ist wohlgeformt: jede zweite Spalte ist ein Kz (`E` und sieben Ziffern), jede
/// dritte `true` oder `false` (`lies_tsv` bricht sonst ab).
#[test]
fn zuordnung_tsv_traegt_nur_gueltige_kz() {
    for (feld, z) in &lies_tsv(ZUORDNUNG) {
        assert!(
            domain::Kz::new(z.kz.clone()).is_ok(),
            "{feld}: {} ist kein Kz",
            z.kz
        );
    }
}
