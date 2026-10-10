//! Kein Rust-Code baut den Pfad zur Python-Bindung (`produkt/bindung`) zusammen. Weg B voll,
//! Entscheidung 2026-10-05: der Dienst und seine Tests lesen `rust/bindung/daten`, und zwar nur
//! ueber `bindung::lade_registry_der_wurzel`.
//!
//! Warum ein Text-Waechter: Ein Rueckfall auf `produkt/bindung` kompiliert, laeuft und bleibt gruen,
//! solange beide Verzeichnisse gleich sind. Er faellt erst auf, wenn ein Feld nur in `daten` steht
//! und ein Test es nicht sieht. Der Waechter zaehlt jede Stelle, die den Pfad als Zeichenkette
//! (Anfuehrungszeichen) nennt, auch in Doctests.
//!
//! Ausnahmen stehen in [`AUSNAHMEN`] mit Grund und mit der erlaubten Trefferzahl. Eine Ausnahme
//! gilt nicht fuer die ganze Datei: ein Treffer mehr als erlaubt ist ein Fehler (sonst bliebe ein
//! Rueckfall in einer freigestellten Datei unbemerkt), ein Treffer weniger auch (die Liste
//! schrumpft nur bewusst; Stufe 2 leert sie, wenn die Python-Fixtures eingefroren sind). Nur
//! `parity/` hat keine Obergrenze und muss mindestens einen Treffer behalten.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};

/// Eine freigestellte Datei oder ein freigestelltes Verzeichnis.
struct Ausnahme {
    /// Pfad ab `rust/`; ein Schraegstrich am Ende meint ein Verzeichnis.
    pfad: &'static str,
    /// Genau so viele Treffer sind erlaubt. `None`: keine Obergrenze (mindestens einer).
    erlaubt: Option<usize>,
    grund: &'static str,
}

impl Ausnahme {
    fn trifft(&self, rel: &str) -> bool {
        if self.pfad.ends_with('/') {
            rel.starts_with(self.pfad)
        } else {
            rel == self.pfad
        }
    }
}

const AUSNAHMEN: &[Ausnahme] = &[
    Ausnahme {
        pfad: "parity/",
        erlaubt: None,
        grund: "PARITY=1-Suiten vergleichen Rust mit dem Python-Orakel, das produkt/bindung liest; sie fallen mit Python weg",
    },
    Ausnahme {
        pfad: "bindung/tests/daten_verzeichnis.rs",
        erlaubt: Some(1),
        grund: "Attrappe: legt das Python-Verzeichnis nur an, um zu belegen, dass der Lader es nicht ansieht",
    },
    Ausnahme {
        pfad: "api/tests/daten_im_dienst.rs",
        erlaubt: Some(1),
        grund: "Attrappe: legt das Python-Verzeichnis nur an, um zu belegen, dass der Dienst es nicht ansieht",
    },
    Ausnahme {
        pfad: "interview/src/lib.rs",
        erlaubt: Some(1),
        grund: "nur python_orakel_registry: die Eingabe der eingefrorenen Python-Antworten (interview, konsistenz); Stufe 2",
    },
    Ausnahme {
        pfad: "eingang/tests/orakel_werte.rs",
        erlaubt: Some(1),
        grund: "vergleicht mit Pythons eingefrorener Antwort ueber produkt/bindung; Stufe 2",
    },
    Ausnahme {
        pfad: "intervall/tests/orakel_werte.rs",
        erlaubt: Some(1),
        grund: "vergleicht mit Pythons eingefrorener Antwort (368 Felder) ueber produkt/bindung; Stufe 2",
    },
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

/// Wie oft die Zeile den Pfad zur Python-Bindung als Zeichenkette nennt: `produkt/bindung`
/// zwischen Anfuehrungszeichen, oder `.join("produkt").join("bindung")`. Jedes Vorkommen zaehlt
/// einzeln, auch zwei in einer Zeile. Prosa in einem Kommentar (`produkt/bindung` in Backticks)
/// ist kein Treffer.
fn zaehle_pfad(zeile: &str) -> usize {
    let pfad = python_bindung();
    let mut n = 0;
    let mut ab = 0;
    while let Some(i) = zeile[ab..].find(&pfad) {
        let pos = ab + i;
        if zeile[..pos].matches('"').count() % 2 == 1 {
            n += 1;
        }
        ab = pos + pfad.len();
    }
    n + zeile.matches(&python_bindung_als_join()).count()
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

/// Alle `.rs`-Dateien unter `rust/` mit ihren Treffern: `(Pfad ab rust/, Zeilennummern)`. Eine
/// Zeile mit zwei Treffern steht zweimal in der Liste, die Laenge ist also die Trefferzahl.
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
        let mut zeilen: Vec<usize> = Vec::new();
        for (i, z) in text.lines().enumerate() {
            zeilen.extend(std::iter::repeat_n(i + 1, zaehle_pfad(z)));
        }
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

/// Treffer ausserhalb der Ausnahmen und Treffer ueber der erlaubten Zahl, je Datei mit Zeilen.
fn zu_viele(gefunden: &[(String, Vec<usize>)], ausnahmen: &[Ausnahme]) -> Vec<String> {
    let mut aus = Vec::new();
    for (rel, zeilen) in gefunden {
        match ausnahmen.iter().find(|a| a.trifft(rel)) {
            None => aus.push(format!("rust/{rel}: Zeilen {zeilen:?}: keine Ausnahme")),
            Some(Ausnahme {
                erlaubt: Some(erlaubt),
                grund,
                ..
            }) if zeilen.len() > *erlaubt => aus.push(format!(
                "rust/{rel}: {} Treffer, erlaubt {erlaubt} ({grund}), Zeilen {zeilen:?}",
                zeilen.len()
            )),
            Some(_) => {}
        }
    }
    aus
}

/// Ausnahmen, die weniger treffen als erlaubt (bei `None`: gar nichts mehr).
fn zu_wenige(gefunden: &[(String, Vec<usize>)], ausnahmen: &[Ausnahme]) -> Vec<String> {
    let mut aus = Vec::new();
    for a in ausnahmen {
        let n: usize = gefunden
            .iter()
            .filter(|(rel, _)| a.trifft(rel))
            .map(|(_, z)| z.len())
            .sum();
        match a.erlaubt {
            Some(erlaubt) if n < erlaubt => aus.push(format!(
                "Ausnahme `{}` ({}) erlaubt {erlaubt} Treffer, es sind {n}: die Zahl in AUSNAHMEN senken oder die Ausnahme streichen",
                a.pfad, a.grund
            )),
            None if n == 0 => aus.push(format!(
                "Ausnahme `{}` ({}) trifft nichts mehr: aus AUSNAHMEN streichen",
                a.pfad, a.grund
            )),
            _ => {}
        }
    }
    aus
}

/// Der Waechter selbst: kein Treffer ausserhalb der Ausnahmen, und keine Ausnahme mit mehr
/// Treffern als erlaubt.
#[test]
fn kein_rust_code_nennt_den_pfad_zur_python_bindung() {
    let zuviel = zu_viele(&treffer(), AUSNAHMEN);
    assert!(
        zuviel.is_empty(),
        "Rust-Code nennt den Pfad zur Python-Bindung; er soll `bindung::lade_registry_der_wurzel` rufen:\n{}",
        zuviel.join("\n")
    );
}

/// Die Liste der Ausnahmen darf nur bewusst schrumpfen: jede Ausnahme trifft genau so oft wie
/// erlaubt. Sonst bliebe ein Freibrief fuer eine Stelle stehen, die es nicht mehr gibt.
#[test]
fn jede_ausnahme_trifft_noch_genau_so_oft_wie_erlaubt() {
    let zuwenig = zu_wenige(&treffer(), AUSNAHMEN);
    assert!(zuwenig.is_empty(), "{}", zuwenig.join("\n"));
}

/// Der Erkenner selbst: er findet den Pfad im Zeichenkettenliteral und im `join`, zaehlt jedes
/// Vorkommen und laesst Prosa in Backticks in Ruhe. Ohne diesen Test koennte ein kaputter Erkenner
/// die Tests oben gruen halten.
#[test]
fn erkenner_findet_den_pfad_nur_als_zeichenkette() {
    let p = python_bindung();
    assert_eq!(zaehle_pfad(&format!("let x = w.join(\"{p}\");")), 1);
    assert_eq!(zaehle_pfad(&format!("let x = w.join(\"../../{p}\");")), 1);
    assert_eq!(
        zaehle_pfad(&format!(
            "/// # let pfad = Path::new(m).join(\"../../{p}\");"
        )),
        1
    );
    assert_eq!(
        zaehle_pfad(&format!("let x = concat!(env!(\"M\"), \"/../../{p}\");")),
        1
    );
    assert_eq!(zaehle_pfad(&format!("r.{}", python_bindung_als_join())), 1);
    // Zwei Vorkommen in einer Zeile zaehlen zweimal.
    assert_eq!(zaehle_pfad(&format!("[\"{p}\", \"{p}\"]")), 2);
    // Prosa: kein Treffer.
    assert_eq!(zaehle_pfad(&format!("/// Die Bindung liegt in `{p}`.")), 0);
    assert_eq!(zaehle_pfad(&format!("//! Quelle: {p}/schema.json")), 0);
    // Der neue Ort ist keiner.
    assert_eq!(zaehle_pfad("let x = w.join(\"rust/bindung/daten\");"), 0);
}

/// Die Pruefung der Trefferzahl selbst, an erfundenen Treffern: eine Ausnahme mit Obergrenze
/// schlaegt bei einem Treffer mehr UND bei einem weniger an, eine ohne Obergrenze nur, wenn sie
/// nichts mehr trifft, und eine Datei ohne Ausnahme ist immer ein Verstoss. Ohne diesen Test
/// koennte die Zaehlung selbst kaputt sein, ohne dass der Waechter es zeigt.
#[test]
fn trefferzahl_pruefung_schlaegt_bei_mehr_und_bei_weniger_an() {
    const A: &[Ausnahme] = &[
        Ausnahme {
            pfad: "a/lib.rs",
            erlaubt: Some(1),
            grund: "g",
        },
        Ausnahme {
            pfad: "frei/",
            erlaubt: None,
            grund: "g",
        },
    ];
    let t = |p: &str, z: &[usize]| (p.to_owned(), z.to_vec());

    // genau erlaubt: sauber
    let genau = [t("a/lib.rs", &[7]), t("frei/x.rs", &[1, 2, 3, 4, 5])];
    assert_eq!(zu_viele(&genau, A).len(), 0);
    assert_eq!(zu_wenige(&genau, A).len(), 0);

    // einer mehr als erlaubt: zu viele, mit Dateiname und Zeilen
    let mehr = [t("a/lib.rs", &[7, 90]), t("frei/x.rs", &[1])];
    let v = zu_viele(&mehr, A);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(
        v[0].contains("rust/a/lib.rs") && v[0].contains("[7, 90]"),
        "{v:?}"
    );
    assert_eq!(zu_wenige(&mehr, A).len(), 0);

    // einer weniger als erlaubt: zu wenige
    let weniger = [t("frei/x.rs", &[1])];
    let w = zu_wenige(&weniger, A);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].contains("a/lib.rs"), "{w:?}");

    // freigestelltes Verzeichnis ohne Treffer: zu wenige
    let leer = [t("a/lib.rs", &[7])];
    let w = zu_wenige(&leer, A);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].contains("frei/"), "{w:?}");

    // Datei ohne Ausnahme: immer zu viel
    let fremd = [t("b/c.rs", &[3])];
    assert_eq!(zu_viele(&fremd, A).len(), 1);
}
