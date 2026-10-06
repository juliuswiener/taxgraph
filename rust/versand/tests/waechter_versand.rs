//! Der Versand loest nie von selbst aus: ein Waechter ueber den Quelltext des Workspace.
//!
//! `versand` ist eine eigene Crate, damit der Dienst `api` den Sendepfad nicht erreichen kann. Das
//! haelt nur, solange (1) `ERIC_SENDE` nirgends sonst vorkommt, (2) keine andere Crate an `versand`
//! haengt und (3) `sende` genau eine Aufrufstelle hat: das Programm, das zwei Huerden vor sich hat.
//! Jeder Test zaehlt vorher, was er durchsucht hat; ein leerer Scan waere sonst ein gruener Test.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

fn rust_wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().unwrap()
}

/// Alle Dateien mit einer der Endungen unter `dir`, ohne `target/` und versteckte Ordner.
fn dateien(dir: &Path, endungen: &[&str], aus: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let pfad = e.unwrap().path();
        let name = pfad.file_name().unwrap().to_string_lossy().into_owned();
        if pfad.is_dir() {
            if name != "target" && !name.starts_with('.') {
                dateien(&pfad, endungen, aus);
            }
        } else if endungen.iter().any(|x| name.ends_with(x)) {
            aus.push(pfad);
        }
    }
}

/// Die Crates des Workspace (Ordner mit einer `Cargo.toml`), ohne `versand`.
fn andere_crates() -> Vec<PathBuf> {
    let mut crates: Vec<PathBuf> = std::fs::read_dir(rust_wurzel())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("Cargo.toml").is_file() && p.file_name().unwrap() != "versand")
        .collect();
    crates.sort();
    crates
}

fn quelltexte_der_anderen() -> Vec<(PathBuf, String)> {
    let mut alle = Vec::new();
    for c in andere_crates() {
        dateien(&c, &[".rs", ".c", ".h"], &mut alle);
    }
    let texte: Vec<_> = alle
        .into_iter()
        .map(|p| {
            let t = std::fs::read_to_string(&p).unwrap_or_default();
            (p, t)
        })
        .collect();
    assert!(andere_crates().len() >= 10, "der Scan sieht zu wenige Crates");
    assert!(texte.len() >= 100, "der Scan sieht nur {} Dateien", texte.len());
    texte
}

#[test]
fn eric_sende_steht_nur_in_der_crate_versand() {
    let treffer: Vec<_> = quelltexte_der_anderen()
        .into_iter()
        .filter(|(_, t)| t.contains("ERIC_SENDE"))
        .map(|(p, _)| p)
        .collect();
    assert!(
        treffer.is_empty(),
        "ERIC_SENDE steht ausserhalb von rust/versand: {treffer:?}"
    );
    // Der Scan sieht das Echte: in `versand` ist das Flag definiert.
    let lib = std::fs::read_to_string(rust_wurzel().join("versand/src/lib.rs")).unwrap();
    assert!(lib.contains("pub const ERIC_SENDE: u32 = 1 << 2;"));
}

#[test]
fn keine_andere_crate_haengt_an_versand() {
    let mut haengt = Vec::new();
    for c in andere_crates() {
        let toml = std::fs::read_to_string(c.join("Cargo.toml")).unwrap();
        let direkt = toml.lines().map(str::trim).any(|z| {
            z.strip_prefix("versand")
                .is_some_and(|r| r.trim_start().starts_with(['=', '.']))
                || (z.starts_with("package") && z.contains("\"versand\""))
        });
        if direkt {
            haengt.push(c);
        }
    }
    assert!(haengt.is_empty(), "diese Crates haengen an versand: {haengt:?}");

    let nutzer: Vec<_> = quelltexte_der_anderen()
        .into_iter()
        .filter(|(p, t)| {
            p.extension().is_some_and(|e| e == "rs")
                && (t.contains("versand::") || t.contains("use versand"))
        })
        .map(|(p, _)| p)
        .collect();
    assert!(nutzer.is_empty(), "diese Dateien nutzen versand: {nutzer:?}");

    // Der Scan sieht das Echte: `versand` haengt an `elster` (nicht umgekehrt) und steht im Workspace.
    let eigene = std::fs::read_to_string(rust_wurzel().join("versand/Cargo.toml")).unwrap();
    assert!(eigene.lines().any(|z| z.starts_with("elster")));
    let wurzel = std::fs::read_to_string(rust_wurzel().join("Cargo.toml")).unwrap();
    assert!(wurzel.contains("\"versand\""), "versand fehlt im Workspace");
}

#[test]
fn sende_hat_genau_eine_aufrufstelle_und_sie_liegt_im_programm() {
    let mut quellen = Vec::new();
    dateien(&rust_wurzel().join("versand/src"), &[".rs"], &mut quellen);
    assert!(quellen.len() >= 5, "{quellen:?}");
    let mut aufrufe = Vec::new();
    for q in &quellen {
        let text = std::fs::read_to_string(q).unwrap();
        for z in text.lines() {
            let z = z.trim_start();
            if !z.starts_with("//") && !z.starts_with("pub fn sende(") && z.contains("sende(") {
                aufrufe.push((q.file_name().unwrap().to_string_lossy().into_owned(), z.to_owned()));
            }
        }
    }
    // Die Einheitstests in lib.rs duerfen nichts senden; eine Aufrufstelle ausser `cli.rs` waere ein
    // zweiter Weg zum Sendeaufruf ohne die Huerden des Programms.
    assert_eq!(aufrufe.len(), 1, "Aufrufstellen von sende: {aufrufe:?}");
    assert!(aufrufe.iter().all(|(datei, _)| datei == "cli.rs"), "{aufrufe:?}");
}
