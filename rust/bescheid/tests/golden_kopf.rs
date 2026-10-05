//! Der Kopf des Golden-Korpus: `rust/fixtures/golden_cases.json` wird gelesen und die Zaehlung gehalten.
//!
//! Cutover-Audit `audits/cutover-bereitschaft-rust-port-2026-10-03.md`, Befund 6: Kein Rust-Test liest das
//! Feld `erwartung`. Die Parity-Suiten (`golden_faelle` in `bescheid_deklaration`, `bescheid_blatt`,
//! `bescheid_zweige`) nehmen je Fall nur `sachverhalt` und rechnen gegen Python; `erwartung` kommt in
//! keinem dieser Laeufe vor. Ein Fall ohne Erwartung waere dort still weiter gruen, und Geloeschtes fiele
//! nicht auf. Dieser Test liest die Datei zur Laufzeit und haelt drei Zusicherungen: Zahl der Faelle,
//! eine Erwartung je Fall, eindeutige Fall-IDs.
//!
//! **Was er nicht prueft: den WERT.** Dass `tarifliche_est` 8 Euro sind oder `festzusetzende_est` 6629,
//! entscheidet weiterhin allein `make golden` (Python, `golden/golden_lauf.py` gegen die Catala-Bibliothek).
//! Dafuer gibt es keinen Rust-Pruefer (Befund 6 — das ist eine eigene Entscheidung, kein Anschluss an diesen Test).
//!
//! Die Python-Seite liest `erwartung` schon: `golden/golden_lauf.py` (Wert gegen die Catala-Bibliothek)
//! und `tests/test_einheiten.py` (Key-Konvention `*_cent` -> Cent). Beide lesen `golden/cases/*.yaml`.
//! Dieser Test haelt dieselbe Form gegen die Rust-Fixture — die Parity-Laeufe tun das nicht, sie lesen
//! nur `sachverhalt`.
//!
//! Die Datei entsteht aus `golden/cases/*.yaml` durch `tools/parity/extract_golden.py`; daher die Zahl in
//! [`N_FAELLE`] und die Reihenfolge im letzten Test.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Die Zahl der Faelle. Gemessen 2026-10-04 auf `f77a7776`: 135 `.yaml`-Dateien unter `golden/cases/` und
/// 135 Elemente in `rust/fixtures/golden_cases.json`. Wer einen Fall ergaenzt, hebt die Zahl im selben
/// Commit — zusammen mit der YAML-Datei und dem Lauf, der ihn rechnet.
const N_FAELLE: usize = 135;

/// Der Pfad des Korpus. `GOLDEN_CASES_DATEI` erlaubt den Rot-Nachweis auf einer WEGWERF-Kopie (eine
/// Zeile geaendert, nie im Repo): derselbe Testcode, dieselben Zusicherungen, eine andere Datei.
fn datei() -> PathBuf {
    match std::env::var("GOLDEN_CASES_DATEI") {
        Ok(p) => PathBuf::from(p),
        Err(_) => wurzel().join("rust/fixtures/golden_cases.json"),
    }
}

/// Der Schluessel, der die Erwartung traegt; `extract_golden.py` schreibt ihn jedem Fall.
const ERWARTUNG: &str = "erwartung";

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Zaehlt im Rohtext die Stellen `"erwartung"`, `:`, `{` — gleich, wie die Datei eingerueckt ist und ob
/// Leerraum um den Doppelpunkt steht. Eine feste Zeichenfolge zaehlte in einer kompakt geschriebenen Datei null.
fn zaehle_erwartungsbloecke(text: &str) -> usize {
    let schluessel = format!("\"{ERWARTUNG}\"");
    text.match_indices(&schluessel)
        .filter(|(i, _)| {
            text[i + schluessel.len()..]
                .trim_start()
                .strip_prefix(':')
                .is_some_and(|rest| rest.trim_start().starts_with('{'))
        })
        .count()
}

/// Die Zaehlung haengt nicht von der Schreibweise der Datei ab.
#[test]
fn ak2_blockzaehlung_ist_formatunabhaengig() {
    let eingerueckt = "[\n  {\"id\": \"a\", \"erwartung\": {\n      \"tarifliche_est\": 1\n  }}\n]";
    let kompakt = r#"[{"id":"a","erwartung":{"tarifliche_est":1}},{"id":"b","erwartung":{"tarifliche_est":2}}]"#;
    let mit_leerraum = "{\"erwartung\"\n :\t{ }}";
    let kein_block = r#"{"id":"erwartung","erwartung":5,"x":["erwartung"]}"#;
    assert_eq!(zaehle_erwartungsbloecke(eingerueckt), 1);
    assert_eq!(zaehle_erwartungsbloecke(kompakt), 2);
    assert_eq!(zaehle_erwartungsbloecke(mit_leerraum), 1);
    assert_eq!(zaehle_erwartungsbloecke(kein_block), 0);
}

/// AK1: die Datei existiert und zaehlt genau [`N_FAELLE`] Faelle.
#[test]
fn ak1_korpus_da_und_in_der_gemessenen_zahl() {
    let pfad = datei();
    let text = std::fs::read_to_string(&pfad).unwrap_or_else(|e| {
        panic!(
            "{} lesen: {e} (die Datei macht: python3 tools/parity/extract_golden.py)",
            pfad.display()
        )
    });
    let faelle: Vec<Value> =
        serde_json::from_str(&text).expect("golden_cases.json ist kein JSON-Feld von Faellen");
    assert_eq!(
        faelle.len(),
        N_FAELLE,
        "Faelle im Korpus: festgehalten {N_FAELLE}, jetzt {} — die Zahl hebt nur, wer den Korpus selbst erweitert",
        faelle.len()
    );
    // Die Zahl kommt aus `golden/cases/*.yaml`. Wer die eine Quelle austauscht, ohne die andere zu ziehen,
    // sieht es hier und nicht erst im naechsten Parity-Lauf.
    let yaml = yaml_ids(&wurzel().join("golden/cases"));
    assert_eq!(
        yaml.len(),
        faelle.len(),
        "{} YAML-Faelle, {} JSON-Faelle",
        yaml.len(),
        faelle.len()
    );
}

/// AK2: jeder Fall traegt genau eine Erwartung — kein fehlender Block, kein leerer Block, kein Feld
/// doppelt im Text, und der Wert ist eine ganze Zahl.
///
/// Warum GENAU eins: `golden/golden_lauf.py` waehlt den Erwartungswert ueber eine feste Kette
/// (`tarifliche_est` else `festzusetzende_est` else …). Ein zweites Feld im selben Fall wird von keinem
/// Leser gesehen — es waere eine Erwartung, die niemand haelt. Die Kette kennt ausserdem nur die acht
/// Namen, die AK2 in der Verteilung ausdruckt; ein neuer Name fiele in Python still auf `None`.
#[test]
fn ak2_jeder_fall_hat_genau_eine_erwartung() {
    let faelle = lade();
    let mut verteilung: BTreeMap<&str, usize> = BTreeMap::new();
    for fall in &faelle {
        let id = fall["id"].as_str().unwrap_or("(ohne id)");
        let erw = fall
            .get(ERWARTUNG)
            .unwrap_or_else(|| panic!("{id}: Feld `{ERWARTUNG}` fehlt"));
        let felder = erw
            .as_object()
            .unwrap_or_else(|| panic!("{id}: `{ERWARTUNG}` ist kein Objekt: {erw}"));
        assert_eq!(
            felder.len(),
            1,
            "{id}: `{ERWARTUNG}` traegt {} Felder ({:?}), erwartet genau eins",
            felder.len(),
            felder.keys()
        );
        let (feld, wert) = felder.iter().next().unwrap();
        assert!(
            wert.is_i64(),
            "{id}: Erwartung `{feld}` ist keine ganze Zahl ({wert})"
        );
        *verteilung.entry(feld.as_str()).or_default() += 1;
    }
    assert_eq!(
        verteilung.values().sum::<usize>(),
        faelle.len(),
        "die Feldnamen zusammengezaehlt"
    );
    // Ein im Text doppelt geschriebener Schluessel `erwartung` laest serde_json als einen Fall mit dem
    // letzten Block; der erste waere still weg. Rohe Zaetzung gegen die Zahl der Faelle.
    let text = std::fs::read_to_string(datei()).unwrap();
    let roh = zaehle_erwartungsbloecke(&text);
    assert_eq!(
        roh,
        faelle.len(),
        "{roh} Blöcke `{ERWARTUNG}` im Text, {} Faelle — ein Fall traegt den Schluessel doppelt",
        faelle.len()
    );
    println!(
        "erwartung-Feld je Fall: {} Namen, Summe {} — {:?}",
        verteilung.len(),
        faelle.len(),
        verteilung
    );
}

/// AK3: die Fall-IDs sind eindeutig und nicht leer.
#[test]
fn ak3_fall_ids_sind_eindeutig() {
    let faelle = lade();
    let ids: Vec<&str> = faelle
        .iter()
        .map(|f| f["id"].as_str().unwrap_or_else(|| panic!("Fall ohne id: {f}")))
        .collect();
    assert!(
        ids.iter().all(|i| !i.trim().is_empty()),
        "eine Fall-ID ist leer"
    );
    let menge: BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(
        menge.len(),
        ids.len(),
        "{} IDs, nur {} verschiedene — Doppeln: {:?}",
        ids.len(),
        menge.len(),
        doppelte(&ids)
    );
}

/// Die Reihenfolge, die `extract_golden.py` setzt (sortierte YAML-Namen); der Kopf ist nur dann eindeutig
/// adressierbar, wenn ID und Datei zusammengehören.
#[test]
fn ids_sind_die_yaml_dateien_in_ihrer_reihenfolge() {
    let faelle = lade();
    let ids: Vec<&str> = faelle.iter().map(|f| f["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        yaml_ids(&wurzel().join("golden/cases")).iter().map(String::as_str).collect::<Vec<_>>(),
        "die JSON-Faelle sind nicht mehr die YAML-Dateien in sortierter Reihenfolge"
    );
}

fn lade() -> Vec<Value> {
    let text = std::fs::read_to_string(datei()).expect("rust/fixtures/golden_cases.json");
    serde_json::from_str(&text).expect("golden_cases.json: JSON-Feld der Faelle")
}

/// Die Fall-IDs aus `golden/cases/<id>.yaml`, sortiert wie `extract_golden.py`.
fn yaml_ids(ordner: &Path) -> Vec<String> {
    let mut namen: Vec<String> = std::fs::read_dir(ordner)
        .unwrap_or_else(|e| panic!("{} lesen: {e}", ordner.display()))
        .filter_map(|e| {
            let n = e.expect("Verzeichniseintrag").file_name();
            let n = n.to_string_lossy().to_string();
            n.strip_suffix(".yaml").map(str::to_owned)
        })
        .collect();
    namen.sort();
    namen
}

fn doppelte<'a>(ids: &'a [&'a str]) -> Vec<&'a str> {
    let mut gesehen = BTreeSet::new();
    let mut doppen = Vec::new();
    for i in ids {
        if !gesehen.insert(*i) {
            doppen.push(*i);
        }
    }
    doppen
}
