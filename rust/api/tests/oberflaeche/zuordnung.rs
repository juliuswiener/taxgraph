//! Haelt `ZUORDNUNG.tsv` gegen den Quelltext. Die Tabelle nennt zu JEDER der 231 Funktionen der 22 Playwright-Dateien
//! (`tests/test_ui_*.py` und vier weitere, die mit L2 fallen) einen Pruefpunkt dieser Suite, einen bestehenden Rust-Test
//! oder einen benannten Verlust mit Grund. Ohne diesen Test koennte ein Pruefpunkt aus dem Quelltext verschwinden, waehrend
//! die Tabelle ihn weiter nennt, und die Zuordnung waere eine Behauptung ueber Code, den es nicht mehr gibt.
//!
//! Die Tabelle steht fuer den Stand beim Commit; die Zeilennummern (`rust_zeile`) sind Auskunft und werden hier nicht
//! geprueft, die Namen schon. Ein Test ohne Browser: er liest nur Dateien.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::dienst::wurzel;

const TSV: &str = include_str!("ZUORDNUNG.tsv");

fn zeilen() -> Vec<Vec<&'static str>> {
    TSV.lines().skip(1).map(|z| z.split('\t').collect()).collect()
}

/// Der Quelltext der Suite ausser dieser Datei.
fn quellen() -> String {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/oberflaeche");
    let mut dateien: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs") && p.file_name().is_some_and(|n| n != "zuordnung.rs"))
        .collect();
    dateien.sort();
    dateien.iter().map(|p| std::fs::read_to_string(p).unwrap()).collect::<Vec<_>>().join("\n")
}

#[test]
fn jede_python_funktion_hat_einen_pruefpunkt_oder_einen_benannten_verlust() {
    let z = zeilen();
    assert!(z.iter().all(|r| r.len() == 9), "eine Zeile hat nicht 9 Spalten");
    let alt: Vec<&Vec<&str>> = z.iter().filter(|r| r[1] != "(neu)").collect();
    assert_eq!(alt.len(), 231, "die Tabelle fuehrt nicht genau die 231 Funktionen");
    let eindeutig: BTreeSet<(&str, &str)> = alt.iter().map(|r| (r[0], r[1])).collect();
    assert_eq!(eindeutig.len(), alt.len(), "eine Funktion steht doppelt in der Tabelle");

    let quelle = quellen();
    let repo = wurzel();
    let (mut pruefpunkte, mut vorhanden, mut verluste) = (0, 0, 0);
    for r in &z {
        match r[3] {
            "PRUEFPUNKT" => {
                pruefpunkte += 1;
                assert!(quelle.contains(&format!("\"{}\"", r[4])), "{}: die Kennung {} steht in keinem pp! der Suite", r[1], r[4]);
            }
            "VORHANDEN" => {
                vorhanden += 1;
                let text = std::fs::read_to_string(repo.join(r[5])).unwrap_or_else(|e| panic!("{}: {}: {e}", r[1], r[5]));
                assert!(text.contains(&format!("fn {}(", r[4])), "{}: {} hat keinen Test {}", r[1], r[5], r[4]);
            }
            "VERLUST" => {
                verluste += 1;
                assert!(r[8].chars().count() >= 60, "{}: der Verlust nennt keinen Grund", r[1]);
                assert!(["Geld", "Daten", "Darstellung", "Leistung"].contains(&r[7]), "{}: der Verlust nennt keinen Bezug: {:?}", r[1], r[7]);
            }
            a => panic!("{}: unbekannte Art {a:?}", r[1]),
        }
    }
    // Untergrenzen gegen den stillen Leerlauf: die Tabelle ist nicht leer geworden.
    assert!(pruefpunkte >= 200 && vorhanden >= 5 && verluste >= 1, "{pruefpunkte} Pruefpunkte, {vorhanden} vorhandene Tests, {verluste} Verluste");

    // Die Gegenrichtung: kein Pruefpunkt im Quelltext, den die Tabelle nicht fuehrt.
    let ids: BTreeSet<&str> = z.iter().filter(|r| r[3] == "PRUEFPUNKT").map(|r| r[4]).collect();
    let muster = regex::Regex::new(r#"pp!\(\s*"([A-Z]\.[^"]+)""#).unwrap();
    for m in muster.captures_iter(&quelle) {
        assert!(ids.contains(&m[1]), "der Pruefpunkt {} steht im Quelltext, aber nicht in ZUORDNUNG.tsv", &m[1]);
    }
}
