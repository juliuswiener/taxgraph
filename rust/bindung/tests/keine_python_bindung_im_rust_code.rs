//! Kein Rust-Code baut den Pfad zur Python-Bindung (`produkt/bindung`) zusammen. Weg B voll,
//! Entscheidung 2026-10-05: der Dienst und seine Tests lesen `rust/bindung/daten`, und zwar nur
//! ueber `bindung::lade_registry_der_wurzel`.
//!
//! Warum ein Text-Waechter: Ein Rueckfall auf `produkt/bindung` kompiliert, laeuft und bleibt gruen,
//! solange beide Verzeichnisse gleich sind. Er faellt erst auf, wenn ein Feld nur in `daten` steht
//! und ein Test es nicht sieht. Der Waechter findet jede Stelle, die den Pfad als Zeichenkette
//! (Anfuehrungszeichen) nennt, auch in Doctests.
//!
//! Ausnahmen stehen in [`AUSNAHMEN`] mit Grund. Eine Ausnahme, die nichts mehr trifft, ist selbst
//! ein Fehler: die Liste soll nur schrumpfen (Stufe 2 leert sie, wenn die Python-Fixtures
//! eingefroren sind).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};

/// `(Pfad ab `rust/` mit Schraegstrich-Ende fuer ein Verzeichnis, Grund)`.
const AUSNAHMEN: &[(&str, &str)] = &[
    (
        "parity/",
        "PARITY=1-Suiten vergleichen Rust mit dem Python-Orakel, das produkt/bindung liest; sie fallen mit Python weg",
    ),
    (
        "bindung/tests/daten_verzeichnis.rs",
        "Attrappe: legt das Python-Verzeichnis nur an, um zu belegen, dass der Lader es nicht ansieht",
    ),
    (
        "api/tests/daten_im_dienst.rs",
        "Attrappe: legt das Python-Verzeichnis nur an, um zu belegen, dass der Dienst es nicht ansieht",
    ),
    (
        "interview/src/lib.rs",
        "python_orakel_registry: die Eingabe der eingefrorenen Python-Antworten (interview, konsistenz); Stufe 2",
    ),
    (
        "eingang/tests/orakel_werte.rs",
        "vergleicht mit Pythons eingefrorener Antwort ueber produkt/bindung; Stufe 2",
    ),
    (
        "intervall/tests/orakel_werte.rs",
        "vergleicht mit Pythons eingefrorener Antwort (368 Felder) ueber produkt/bindung; Stufe 2",
    ),
];

fn rust_wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Der gesuchte Pfad, so zusammengesetzt, dass diese Datei ihn nicht selbst als Treffer traegt.
fn python_bindung() -> String {
    ["produkt", "bindung"].join("/")
}

/// Der gleiche Pfad als verkettetes `join`.
fn python_bindung_als_join() -> String {
    ["join(\"produkt\")", "join(\"bindung\")"].join(".")
}

/// Wahr, wenn die Zeile den Pfad zur Python-Bindung als Zeichenkette nennt: `produkt/bindung`
/// zwischen Anfuehrungszeichen, oder `.join("produkt").join("bindung")`. Prosa in einem Kommentar
/// (`produkt/bindung` in Backticks) ist kein Treffer.
fn nennt_den_pfad(zeile: &str) -> bool {
    let pfad = python_bindung();
    let mut ab = 0;
    while let Some(i) = zeile[ab..].find(&pfad) {
        let pos = ab + i;
        if zeile[..pos].matches('"').count() % 2 == 1 {
            return true;
        }
        ab = pos + pfad.len();
    }
    zeile.contains(&python_bindung_als_join())
}

fn alle_rs(dir: &Path, aus: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if p.is_dir() {
            // Bauverzeichnisse (`target`, `target-gate-main`, ...) liegen auch unter `rust/`.
            if !name.starts_with("target") && !name.starts_with('.') {
                alle_rs(&p, aus);
            }
        } else if p.extension().is_some_and(|x| x == "rs") {
            aus.push(p);
        }
    }
}

/// Alle `.rs`-Dateien unter `rust/` mit ihren Treffern: `(Pfad ab rust/, Zeilennummern)`.
fn treffer() -> Vec<(String, Vec<usize>)> {
    let wurzel = rust_wurzel().canonicalize().unwrap();
    let mut dateien = Vec::new();
    alle_rs(&wurzel, &mut dateien);
    dateien.sort();
    let mut aus = Vec::new();
    for d in dateien {
        // Der Erkenner nennt den Pfad in seinen Begruendungen und Tests selbst.
        if d.ends_with("bindung/tests/keine_python_bindung_im_rust_code.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&d).unwrap();
        let zeilen: Vec<usize> = text
            .lines()
            .enumerate()
            .filter(|(_, z)| nennt_den_pfad(z))
            .map(|(i, _)| i + 1)
            .collect();
        if !zeilen.is_empty() {
            let rel = d
                .strip_prefix(&wurzel)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            aus.push((rel, zeilen));
        }
    }
    aus
}

fn ausgenommen(rel: &str) -> bool {
    AUSNAHMEN.iter().any(|(p, _)| {
        if p.ends_with('/') {
            rel.starts_with(p)
        } else {
            rel == *p
        }
    })
}

/// Der Waechter selbst: kein Treffer ausserhalb der Ausnahmen.
#[test]
fn kein_rust_code_nennt_den_pfad_zur_python_bindung() {
    let fremde: Vec<String> = treffer()
        .into_iter()
        .filter(|(rel, _)| !ausgenommen(rel))
        .map(|(rel, z)| format!("rust/{rel}: Zeilen {z:?}"))
        .collect();
    assert!(
        fremde.is_empty(),
        "Rust-Code nennt den Pfad zur Python-Bindung; er soll `bindung::lade_registry_der_wurzel` rufen:\n{}",
        fremde.join("\n")
    );
}

/// Die Liste der Ausnahmen darf nur schrumpfen: jede Ausnahme trifft noch mindestens einen Pfad.
/// Sonst bliebe ein Freibrief fuer eine Stelle stehen, die es nicht mehr gibt.
#[test]
fn jede_ausnahme_trifft_noch_etwas() {
    let gefunden = treffer();
    for (pfad, grund) in AUSNAHMEN {
        let trifft = gefunden.iter().any(|(rel, _)| {
            if pfad.ends_with('/') {
                rel.starts_with(pfad)
            } else {
                rel == pfad
            }
        });
        assert!(
            trifft,
            "Ausnahme `{pfad}` ({grund}) trifft nichts mehr: aus AUSNAHMEN streichen"
        );
    }
}

/// Der Erkenner selbst: er findet den Pfad im Zeichenkettenliteral und im `join`, und er laesst
/// Prosa in Backticks in Ruhe. Ohne diesen Test koennte ein kaputter Erkenner beide Tests oben
/// gruen halten.
#[test]
fn erkenner_findet_den_pfad_nur_als_zeichenkette() {
    let p = python_bindung();
    assert!(nennt_den_pfad(&format!("let x = w.join(\"{p}\");")));
    assert!(nennt_den_pfad(&format!("let x = w.join(\"../../{p}\");")));
    assert!(nennt_den_pfad(&format!(
        "/// # let pfad = Path::new(m).join(\"../../{p}\");"
    )));
    assert!(nennt_den_pfad(&format!(
        "let x = concat!(env!(\"M\"), \"/../../{p}\");"
    )));
    assert!(nennt_den_pfad(&format!("r.{}", python_bindung_als_join())));
    // Prosa: kein Treffer.
    assert!(!nennt_den_pfad(&format!("/// Die Bindung liegt in `{p}`.")));
    assert!(!nennt_den_pfad(&format!("//! Quelle: {p}/schema.json")));
    // Der neue Ort ist keiner.
    assert!(!nennt_den_pfad("let x = w.join(\"rust/bindung/daten\");"));
}
