//! Waechter der Testkarte `rust/TESTMAP.tsv`, in beide Richtungen: jede Testdatei hat eine Zeile, und jede
//! Zeile nennt eine Testdatei, die es gibt. Dazu die Form: sieben Spalten, der Kopf, Zahlen in den Spalten 2 bis 4.
//!
//! Warum in Rust: bisher prueft nur `tests/test_testmap_vollstaendig.py` die Karte, und nur die Richtung
//! Datei -> Zeile. Mit `tests/` faellt dieser Test, und die Karte bliebe ungeprueft. Eine Zeile ohne Datei
//! blieb bisher unbemerkt (eine geloeschte oder umbenannte Testdatei, deren Zeile weiter in den Zaehlungen
//! des Cutover-Plans steht).
//!
//! Gesucht wird wie im Python-Test: `tests/**/*.py` (nur solange `tests/` besteht), `pipeline/tests/**/*.py`
//! (die Stufe-B-Tests, `make unit-stufe-b`), `rust/*/tests/**/*.rs`, `rust/*/tests/**/*.py`, `rust/*/tests/**/*.c`. Daten (yaml, txt, proptest-regressions) und `#[cfg(test)]` im
//! Quelltext zaehlen nicht. Versteckte Ordner, `__pycache__` und Bauverzeichnisse (`rust/target*`) bleiben aus.
//!
//! Grenzen: Der Test sucht die Dateien, nicht ihren Inhalt. Spalte 2 und 3 halten den Stand bei der Anlage; ob
//! sie noch stimmen, prueft niemand. Ob Spalte 7 nur Ersatz nennt, der im Baum existiert, prueft niemand.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const KOPF: [&str; 7] = [
    "file",
    "lines_bei_anlage",
    "n_tests_bei_anlage",
    "xfail",
    "category",
    "target_module",
    "replacing_guarantee_or_note",
];

fn repo_wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn karte_lesen() -> String {
    std::fs::read_to_string(repo_wurzel().join("rust/TESTMAP.tsv")).unwrap()
}

/// Alle Dateien mit einer der Endungen unter `dir`, ohne versteckte Ordner und `__pycache__`.
fn sammle(dir: &Path, endungen: &[&str], aus: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "__pycache__" {
            continue;
        }
        if p.is_dir() {
            sammle(&p, endungen, aus);
        } else if p
            .extension()
            .is_some_and(|x| endungen.iter().any(|e| x == *e))
        {
            aus.push(p);
        }
    }
}

/// Relative Pfade (mit `/`) aller Testdateien unter `wurzel`.
fn testdateien(wurzel: &Path) -> BTreeSet<String> {
    let mut pfade = Vec::new();
    for python in [wurzel.join("tests"), wurzel.join("pipeline/tests")] {
        if python.is_dir() {
            sammle(&python, &["py"], &mut pfade);
        }
    }
    for e in std::fs::read_dir(wurzel.join("rust")).unwrap() {
        let krate = e.unwrap().path();
        let name = krate.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with('.') || name.starts_with("target") {
            continue;
        }
        let tests = krate.join("tests");
        if tests.is_dir() {
            sammle(&tests, &["rs", "py", "c"], &mut pfade);
        }
    }
    pfade
        .iter()
        .map(|p| {
            p.strip_prefix(wurzel)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

/// Spalte 1 jeder Datenzeile (der Kopf zaehlt nicht).
fn dateien_der_karte(text: &str) -> Vec<String> {
    text.split('\n')
        .skip(1)
        .filter(|z| !z.is_empty())
        .map(|z| z.split('\t').next().unwrap().to_string())
        .collect()
}

/// Testdateien ohne Zeile in der Karte.
fn ohne_zeile(dateien: &BTreeSet<String>, karte: &[String]) -> Vec<String> {
    let eingetragen: BTreeSet<&String> = karte.iter().collect();
    dateien
        .iter()
        .filter(|d| !eingetragen.contains(d))
        .cloned()
        .collect()
}

/// Zeilen der Karte, deren Datei es nicht gibt.
fn ohne_datei(dateien: &BTreeSet<String>, karte: &[String]) -> Vec<String> {
    karte
        .iter()
        .filter(|k| !dateien.contains(*k))
        .cloned()
        .collect()
}

/// Formfehler der Karte, je einer pro Zeile und Mangel: Kopf, sieben Spalten, Zahlen in 2 bis 4, keine leere
/// Spalte (1, 5, 6, 7), kein Leerraum am Rand einer Spalte (auch kein `\r`), keine leere Zeile, keine doppelte Datei.
fn formfehler(text: &str) -> Vec<String> {
    let mut aus = Vec::new();
    let zeilen: Vec<&str> = text.split('\n').collect();
    if zeilen.first().copied() != Some(KOPF.join("\t").as_str()) {
        aus.push(format!("Kopf ist nicht `{}`", KOPF.join("\\t")));
    }
    let mut gesehen = BTreeSet::new();
    for (i, zeile) in zeilen.iter().enumerate().skip(1) {
        let nr = i + 1;
        if zeile.is_empty() {
            // Nur der Schluss nach dem letzten Zeilenumbruch darf leer sein.
            if i + 1 != zeilen.len() {
                aus.push(format!("Zeile {nr}: leer"));
            }
            continue;
        }
        let spalten: Vec<&str> = zeile.split('\t').collect();
        if spalten.len() != KOPF.len() {
            aus.push(format!(
                "Zeile {nr}: {} Spalten statt {}",
                spalten.len(),
                KOPF.len()
            ));
            continue;
        }
        for (k, s) in spalten.iter().enumerate() {
            if *s != s.trim() {
                aus.push(format!("Zeile {nr}: Spalte {} hat Leerraum am Rand", k + 1));
            }
        }
        for k in [0, 4, 5, 6] {
            if spalten[k].trim().is_empty() {
                aus.push(format!("Zeile {nr}: Spalte {} ist leer", k + 1));
            }
        }
        for (k, s) in spalten.iter().enumerate().skip(1).take(3) {
            if s.parse::<u64>().is_err() {
                aus.push(format!(
                    "Zeile {nr}: Spalte {} ist keine Zahl: `{s}`",
                    k + 1
                ));
            }
        }
        if !gesehen.insert(spalten[0]) {
            aus.push(format!("Zeile {nr}: `{}` steht zweimal", spalten[0]));
        }
    }
    aus
}

/// Jede Testdatei hat eine Zeile.
#[test]
fn jede_testdatei_hat_eine_zeile_in_der_karte() {
    let fehlend = ohne_zeile(
        &testdateien(&repo_wurzel()),
        &dateien_der_karte(&karte_lesen()),
    );
    assert!(
        fehlend.is_empty(),
        "{} Testdateien ohne Zeile in rust/TESTMAP.tsv (7 Spalten: {}):\n  {}",
        fehlend.len(),
        KOPF.join(", "),
        fehlend.join("\n  ")
    );
}

/// Jede Zeile nennt eine Testdatei, die es gibt.
#[test]
fn jede_zeile_der_karte_nennt_eine_testdatei() {
    let verwaist = ohne_datei(
        &testdateien(&repo_wurzel()),
        &dateien_der_karte(&karte_lesen()),
    );
    assert!(
        verwaist.is_empty(),
        "{} Zeilen in rust/TESTMAP.tsv ohne Testdatei (Datei geloescht oder umbenannt? Zeile streichen oder anpassen):\n  {}",
        verwaist.len(),
        verwaist.join("\n  ")
    );
}

/// Die Form der Karte.
#[test]
fn jede_zeile_hat_sieben_spalten_und_stimmige_werte() {
    let fehler = formfehler(&karte_lesen());
    assert!(
        fehler.is_empty(),
        "{} Formfehler in rust/TESTMAP.tsv:\n  {}",
        fehler.len(),
        fehler.join("\n  ")
    );
}

/// Der Finder sieht den echten Baum: die Datei selbst und, solange `tests/` und `pipeline/tests/` bestehen, auch deren
/// Python-Testdateien. Ein Finder, der kaum etwas findet, faellt ausserdem an den Zeilen ohne Datei auf.
#[test]
fn der_finder_sieht_den_echten_baum() {
    let wurzel = repo_wurzel();
    let dateien = testdateien(&wurzel);
    assert!(
        dateien.contains("rust/bindung/tests/testmap_vollstaendig.rs"),
        "der Finder findet diese Datei nicht"
    );
    if wurzel.join("tests").is_dir() {
        assert!(
            dateien.iter().any(|d| d.starts_with("tests/test_")
                && Path::new(d).extension().is_some_and(|x| x == "py")),
            "tests/ besteht, aber der Finder sieht keine Python-Testdatei"
        );
    }
    if wurzel.join("pipeline/tests").is_dir() {
        assert!(
            dateien.iter().any(|d| d.starts_with("pipeline/tests/test_")
                && Path::new(d).extension().is_some_and(|x| x == "py")),
            "pipeline/tests/ besteht, aber der Finder sieht keine Python-Testdatei"
        );
    }
}

fn schreibe(wurzel: &Path, pfad: &str) {
    let p = wurzel.join(pfad);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, "").unwrap();
}

/// Der Finder auf einem gebauten Baum: Tiefe, alle drei Endungen unter `rust/`, Python nur unter `tests/` und
/// `pipeline/tests/`; Daten, `__pycache__`, versteckte Ordner, Bauverzeichnisse und `src/` bleiben aus; ohne `tests/`
/// und ohne `pipeline/tests/` bricht er nicht ab.
#[test]
fn der_finder_folgt_dem_muster_und_laesst_fremdes_aus() {
    let wurzel = std::env::temp_dir().join(format!("testmap-waechter-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&wurzel);
    for p in [
        "tests/test_a.py",
        "tests/sub/test_b.py",
        "tests/__pycache__/test_a.py",
        "tests/.pytest_cache/x.py",
        "tests/daten.txt",
        "pipeline/tests/test_c.py",
        "pipeline/tests/sub/test_d.py",
        "pipeline/tests/__pycache__/test_c.py",
        "pipeline/tests/.pytest_cache/x.py",
        "pipeline/tests/daten.yaml",
        "pipeline/quellen.py",
        "rust/x/tests/y.rs",
        "rust/x/tests/sub/z.rs",
        "rust/x/tests/sub/tief/w.py",
        "rust/x/tests/h.c",
        "rust/x/tests/daten.yaml",
        "rust/x/tests/__pycache__/v.py",
        "rust/x/src/lib.rs",
        "rust/x/src/tests/inline.rs",
        "rust/target-bau/tests/q.rs",
        "rust/.versteckt/tests/r.rs",
    ] {
        schreibe(&wurzel, p);
    }
    let erwartet: BTreeSet<String> = [
        "tests/test_a.py",
        "tests/sub/test_b.py",
        "pipeline/tests/test_c.py",
        "pipeline/tests/sub/test_d.py",
        "rust/x/tests/y.rs",
        "rust/x/tests/sub/z.rs",
        "rust/x/tests/sub/tief/w.py",
        "rust/x/tests/h.c",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    let mit_tests = testdateien(&wurzel);
    std::fs::remove_dir_all(wurzel.join("tests")).unwrap();
    std::fs::remove_dir_all(wurzel.join("pipeline/tests")).unwrap();
    let ohne_tests = testdateien(&wurzel);
    let _ = std::fs::remove_dir_all(&wurzel);
    assert_eq!(mit_tests, erwartet);
    let nur_rust: BTreeSet<String> = erwartet
        .iter()
        .filter(|d| d.starts_with("rust/"))
        .cloned()
        .collect();
    assert_eq!(ohne_tests, nur_rust);
}

/// Jeder Helfer schlaegt an, wenn er etwas findet, und schweigt, wenn nichts da ist.
#[test]
fn die_helfer_erkennen_ihre_fehlerfaelle() {
    let dateien: BTreeSet<String> = ["rust/a/tests/x.rs", "tests/test_y.py"]
        .iter()
        .map(ToString::to_string)
        .collect();
    let karte = |z: &[&str]| z.iter().map(ToString::to_string).collect::<Vec<_>>();

    assert!(ohne_zeile(&dateien, &karte(&["rust/a/tests/x.rs", "tests/test_y.py"])).is_empty());
    assert_eq!(
        ohne_zeile(&dateien, &karte(&["rust/a/tests/x.rs"])),
        ["tests/test_y.py"]
    );
    assert!(ohne_datei(&dateien, &karte(&["rust/a/tests/x.rs", "tests/test_y.py"])).is_empty());
    assert_eq!(
        ohne_datei(
            &dateien,
            &karte(&["rust/a/tests/x.rs", "rust/a/tests/weg.rs"])
        ),
        ["rust/a/tests/weg.rs"]
    );
    // Eine Zeile mit anderem Pfad zaehlt weder als Zeile fuer die alte Datei noch als Datei fuer die neue.
    let umbenannt = karte(&["rust/a/tests/x2.rs", "tests/test_y.py"]);
    assert_eq!(ohne_zeile(&dateien, &umbenannt), ["rust/a/tests/x.rs"]);
    assert_eq!(ohne_datei(&dateien, &umbenannt), ["rust/a/tests/x2.rs"]);

    assert_eq!(
        dateien_der_karte(&format!(
            "{}\nr1\t1\t1\t0\tA\tB\tC\nr2\t1\t1\t0\tA\tB\tC\n",
            KOPF.join("\t")
        )),
        ["r1", "r2"]
    );
}

/// Jeder Formfehler wird gefunden; eine gute Karte hat keinen.
#[test]
fn die_formpruefung_erkennt_ihre_fehlerfaelle() {
    let kopf = KOPF.join("\t");
    let gut = "d1\t1\t0\t0\tGOLDEN\tmodul\tnotiz";
    let karte = |zeilen: &[&str]| format!("{kopf}\n{}\n", zeilen.join("\n"));

    assert!(formfehler(&karte(&[gut, "d2\t2\t3\t0\tTOOLING\t-\tx y"])).is_empty());
    assert!(formfehler(&format!("{kopf}\n")).is_empty());

    let falsche_spalten = [
        ("sechs Spalten", "d2\t1\t0\t0\tGOLDEN\tmodul"),
        ("acht Spalten", "d2\t1\t0\t0\tGOLDEN\tmodul\tnotiz\tmehr"),
        ("Zahl fehlt", "d2\tviele\t0\t0\tGOLDEN\tmodul\tnotiz"),
        ("negative Zahl", "d2\t1\t-1\t0\tGOLDEN\tmodul\tnotiz"),
        ("leere Zahl", "d2\t1\t0\t\tGOLDEN\tmodul\tnotiz"),
        ("leere Datei", "\t1\t0\t0\tGOLDEN\tmodul\tnotiz"),
        ("leere Kategorie", "d2\t1\t0\t0\t\tmodul\tnotiz"),
        ("leeres Zielmodul", "d2\t1\t0\t0\tGOLDEN\t\tnotiz"),
        ("leere Notiz", "d2\t1\t0\t0\tGOLDEN\tmodul\t"),
        ("Leerzeichen am Rand", "d2\t1\t0\t0\tGOLDEN\tmodul\tnotiz "),
        ("Zeilenende CRLF", "d2\t1\t0\t0\tGOLDEN\tmodul\tnotiz\r"),
        ("doppelte Datei", gut),
    ];
    for (was, zeile) in falsche_spalten {
        assert_eq!(
            formfehler(&karte(&[gut, zeile])).len(),
            1,
            "genau ein Fehler erwartet: {was}"
        );
    }
    assert_eq!(
        formfehler(&format!("{kopf}\n{gut}\n\n{}\n", gut.replace("d1", "d3"))).len(),
        1,
        "leere Zeile mitten"
    );
    assert!(
        !formfehler(&format!("file\t{gut}\n")).is_empty(),
        "falscher Kopf"
    );
    assert!(
        !formfehler(&format!("{}\n{gut}\n", kopf.replace("xfail", "xf"))).is_empty(),
        "Kopf mit anderem Namen"
    );
    assert!(!formfehler(&format!("{gut}\n")).is_empty(), "Kopf fehlt");
}
