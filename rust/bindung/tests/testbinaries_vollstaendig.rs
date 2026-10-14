//! Waechter der Test-Binaries: Wo ein Crate seine Testdateien zu einem Binary `it` zusammenlegt (`autotests = false`
//! in der `Cargo.toml`, Datei `tests/it.rs` mit `#[path = "x.rs"] mod x;` je Testdatei), muss jede Testdatei an
//! genau einer Stelle stehen: in `it.rs` oder als eigenes `[[test]]`. Eine Datei an keiner Stelle wird nie
//! uebersetzt, ihre Tests laufen still nicht mehr (`cargo test` bleibt gruen). Eine Datei an beiden Stellen laeuft doppelt.
//!
//! Warum zusammengelegt: jede Testdatei war ein eigenes Binary, eine Aenderung an der Bibliothek band 270 (domain)
//! und 143 (bescheid) Test-Programme neu. Eigene Ziele bleiben Dateien, die Umgebung oder Arbeitsverzeichnis des
//! Prozesses aendern (`set_var`, `remove_var`, `set_current_dir`; in einem gemeinsamen Prozess stoeren sie jeden anderen
//! Test) und Dateien, die per `--test <name>` aus Makefile, Skripten, Karte oder Pruefungen aufgerufen werden.
//!
//! Grenzen: Der Test liest Dateinamen, nicht Inhalt. Dass eine zusammengelegte Datei keinen Prozesszustand veraendert
//! (Umgebung, Arbeitsverzeichnis, feste Ports oder Dateien), prueft er nicht; nextest (ein Prozess je Test) deckt das
//! nur auf, wenn jemand es laeuft.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn rust_wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap()
}

/// Die Ziele `tests/...` einer `Cargo.toml`, ohne das Vorwort `tests/` (Zeilen `path = "tests/..."`).
fn deklarierte_ziele(toml: &str) -> BTreeSet<String> {
    toml.lines()
        .filter_map(|z| z.trim().strip_prefix("path = \"tests/"))
        .filter_map(|r| r.strip_suffix('"'))
        .map(ToString::to_string)
        .collect()
}

/// Die Dateien, die `tests/it.rs` einbindet (Zeilen `#[path = "x.rs"]`).
fn gebuendelte_dateien(it_rs: &str) -> BTreeSet<String> {
    it_rs
        .lines()
        .filter_map(|z| z.trim().strip_prefix("#[path = \""))
        .filter_map(|r| r.strip_suffix("\"]"))
        .map(ToString::to_string)
        .collect()
}

/// Fehler eines Crates. `it_rs` ist der Inhalt von `tests/it.rs`, falls es die Datei gibt; `dateien` die `*.rs` direkt
/// unter `tests/`, `ordner` die Unterordner von `tests/` mit einer `main.rs`.
fn pruefe(
    toml: &str,
    it_rs: Option<&str>,
    dateien: &BTreeSet<String>,
    ordner: &BTreeSet<String>,
) -> Vec<String> {
    let ohne_auto = toml.lines().any(|z| z.trim() == "autotests = false");
    let mut fehler = Vec::new();
    if it_rs.is_some() && !ohne_auto {
        fehler.push("tests/it.rs besteht, aber `autotests = false` fehlt (jede Datei liefe zusaetzlich einzeln)".into());
    }
    if !ohne_auto {
        return fehler; // Crate ohne Zusammenlegung: jede Datei ist ein eigenes Ziel, nichts zu pruefen
    }
    let ziele = deklarierte_ziele(toml);
    let gebuendelt = it_rs.map(gebuendelte_dateien).unwrap_or_default();
    if it_rs.is_some() && !ziele.contains("it.rs") {
        fehler.push("`[[test]]` mit path = \"tests/it.rs\" fehlt".into());
    }
    for d in dateien.iter().filter(|d| *d != "it.rs") {
        match (gebuendelt.contains(d), ziele.contains(d)) {
            (false, false) => fehler.push(format!(
                "tests/{d} steht weder in tests/it.rs noch als `[[test]]` (wird nie uebersetzt)"
            )),
            (true, true) => fehler.push(format!(
                "tests/{d} steht in tests/it.rs UND als `[[test]]` (laeuft doppelt)"
            )),
            _ => {}
        }
    }
    for o in ordner {
        if !ziele.contains(&format!("{o}/main.rs")) {
            fehler.push(format!(
                "tests/{o}/main.rs steht nicht als `[[test]]` (wird nie uebersetzt)"
            ));
        }
    }
    fehler
}

/// Ein Crate des Baums: Name, `Cargo.toml`, `tests/it.rs`, Dateien und Ordner unter `tests/`.
struct Krate {
    name: String,
    toml: String,
    it_rs: Option<String>,
    dateien: BTreeSet<String>,
    ordner: BTreeSet<String>,
}

fn krates() -> Vec<Krate> {
    let mut aus = Vec::new();
    for e in std::fs::read_dir(rust_wurzel()).unwrap() {
        let dir = e.unwrap().path();
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with('.') || name.starts_with("target") || !dir.join("Cargo.toml").is_file() {
            continue;
        }
        let tests = dir.join("tests");
        if !tests.is_dir() {
            continue;
        }
        let (mut dateien, mut ordner) = (BTreeSet::new(), BTreeSet::new());
        for t in std::fs::read_dir(&tests).unwrap() {
            let p = t.unwrap().path();
            let n = p.file_name().unwrap().to_string_lossy().into_owned();
            if p.is_file() && p.extension().is_some_and(|x| x == "rs") {
                dateien.insert(n);
            } else if p.join("main.rs").is_file() {
                ordner.insert(n);
            }
        }
        aus.push(Krate {
            name,
            toml: std::fs::read_to_string(dir.join("Cargo.toml")).unwrap(),
            it_rs: std::fs::read_to_string(tests.join("it.rs")).ok(),
            dateien,
            ordner,
        });
    }
    aus
}

/// Jede Testdatei jedes zusammengelegten Crates steht an genau einer Stelle.
#[test]
fn jede_testdatei_steht_in_it_rs_oder_als_eigenes_ziel() {
    let fehler: Vec<String> = krates()
        .iter()
        .flat_map(|k| {
            pruefe(&k.toml, k.it_rs.as_deref(), &k.dateien, &k.ordner)
                .into_iter()
                .map(|f| format!("{}: {f}", k.name))
        })
        .collect();
    assert!(
        fehler.is_empty(),
        "{} Fehler bei den Test-Binaries (neue Testdatei? in tests/it.rs eintragen: `#[path = \"x.rs\"] mod x;`; ruft sie jemand mit `--test x` auf oder aendert sie Umgebung/Arbeitsverzeichnis? dann `[[test]]` in der Cargo.toml):\n  {}",
        fehler.len(),
        fehler.join("\n  ")
    );
}

/// Der Finder sieht den echten Baum: mindestens ein Crate ist zusammengelegt, und `it.rs` bindet dort Dateien ein.
/// Ein Finder, der nichts findet, haelt sonst den Test gruen.
#[test]
fn der_finder_sieht_zusammengelegte_crates() {
    let gelegt: Vec<Krate> = krates()
        .into_iter()
        .filter(|k| k.toml.lines().any(|z| z.trim() == "autotests = false"))
        .collect();
    assert!(!gelegt.is_empty(), "kein Crate mit `autotests = false` gefunden");
    assert!(
        gelegt
            .iter()
            .any(|k| k.it_rs.as_deref().is_some_and(|t| gebuendelte_dateien(t).len() >= 2)),
        "kein it.rs bindet Dateien ein"
    );
}

/// Jeder Fehlerfall wird gefunden; ein guter Aufbau und ein Crate ohne Zusammenlegung haben keinen.
#[test]
fn die_pruefung_erkennt_ihre_fehlerfaelle() {
    let satz = |z: &[&str]| z.iter().map(ToString::to_string).collect::<BTreeSet<String>>();
    let toml = "[package]\nautotests = false\n\n[[test]]\nname = \"it\"\npath = \"tests/it.rs\"\n\n[[test]]\nname = \"eigen\"\npath = \"tests/eigen.rs\"\n\n[[test]]\nname = \"ui\"\npath = \"tests/ui/main.rs\"\n";
    let it = "#[path = \"a.rs\"]\nmod a;\n#[path = \"b.rs\"]\nmod b;\n";
    let dateien = satz(&["a.rs", "b.rs", "eigen.rs", "it.rs"]);
    let ordner = satz(&["ui"]);
    assert_eq!(pruefe(toml, Some(it), &dateien, &ordner), Vec::<String>::new());

    // Eine Datei, die niemand einbindet.
    let mehr = satz(&["a.rs", "b.rs", "c.rs", "eigen.rs", "it.rs"]);
    let f = pruefe(toml, Some(it), &mehr, &ordner);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].contains("c.rs"), "{f:?}");
    // Eine Datei an beiden Stellen.
    let doppelt = format!("{it}#[path = \"eigen.rs\"]\nmod eigen;\n");
    let f = pruefe(toml, Some(&doppelt), &dateien, &ordner);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].contains("UND"), "{f:?}");
    // Ein Ordner mit main.rs ohne Ziel.
    let f = pruefe(toml, Some(it), &dateien, &satz(&["ui", "zwei"]));
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].contains("zwei"), "{f:?}");
    // it.rs ohne `autotests = false`: die Datei bliebe einzeln und liefe zusaetzlich im Bund.
    let f = pruefe("[package]\nname = \"x\"\n", Some(it), &dateien, &ordner);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].contains("autotests"), "{f:?}");
    // `autotests = false`, aber kein `[[test]]` fuer it.rs.
    let ohne_ziel = "[package]\nautotests = false\n\n[[test]]\nname = \"eigen\"\npath = \"tests/eigen.rs\"\n\n[[test]]\nname = \"ui\"\npath = \"tests/ui/main.rs\"\n";
    let f = pruefe(ohne_ziel, Some(it), &dateien, &ordner);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].contains("tests/it.rs"), "{f:?}");
    // Ein Crate ohne Zusammenlegung ist nicht betroffen, auch mit vielen Dateien.
    assert_eq!(pruefe("[package]\nname = \"x\"\n", None, &mehr, &ordner), Vec::<String>::new());
}
