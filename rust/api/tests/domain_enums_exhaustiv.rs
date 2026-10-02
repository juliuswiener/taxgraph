//! Tor 1 aus `REWRITE_PLAN.md` §7 (K8): kein `_`-Arm auf einem Domain-Enum im Produktionscode.
//!
//! clippy kennt die Typen: `wildcard_enum_match_arm` und `match_wildcard_for_single_variants`
//! melden jede Wildcard (`_` und `andere =>`) auf einem Enum ausser `Option` und `Result`. Der Test
//! liest das Enum aus clippys Vorschlag und behaelt die Domain-Enums: Enums aus `rust/*/src` ohne
//! Namen auf `Fehler`/`Error` und ohne die Wertmodelle `PyWert`, `PyInt`, `Zahl`, `PyZahl`. Nativ
//! in clippy braeuchte jede Wildcard auf Wertmodellen und Fremd-Enums ein `allow` (K8: 41 Stellen).
//! clippy laeuft ohne `--all-targets`, Testcode zaehlt nicht.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

/// Arme in gesperrten Crates. Macht der Besitzer einen explizit, wird der Test rot: Zeile loeschen.
const OFFEN: &[&str] = &[];

/// `rust/`, die Workspace-Wurzel.
fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn rs_dateien(dir: &Path, aus: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            rs_dateien(&p, aus);
        } else if p.extension().is_some_and(|x| x == "rs") {
            aus.push(p);
        }
    }
}

/// Enums aus `rust/*/src` ohne Fehler-Enums und ohne die Python-Wertmodelle.
fn fach_enums() -> BTreeSet<String> {
    let kopf = Regex::new(r"(?m)^[ \t]*(?:pub(?:\([a-z:]+\))? )?enum (\w+)").unwrap();
    let mut dateien = Vec::new();
    for k in std::fs::read_dir(wurzel()).unwrap() {
        let src = k.unwrap().path().join("src");
        if src.is_dir() {
            rs_dateien(&src, &mut dateien);
        }
    }
    let mut namen = BTreeSet::new();
    for d in dateien {
        let text = std::fs::read_to_string(d).unwrap();
        namen.extend(kopf.captures_iter(&text).map(|c| c[1].to_owned()));
    }
    namen.retain(|n| {
        !n.ends_with("Fehler")
            && !n.ends_with("Error")
            && !["PyWert", "PyInt", "Zahl", "PyZahl"].contains(&n.as_str())
    });
    namen
}

/// Der Typ hinter `Self`: der letzte `impl`-Kopf vor der Zeile.
fn self_typ(quelle: &str, zeile: usize) -> String {
    let kopf = quelle
        .lines()
        .take(zeile)
        .filter(|z| z.trim_start().starts_with("impl ") || z.trim_start().starts_with("impl<"))
        .last()
        .expect("impl-Kopf vor Self");
    let typ = Regex::new(r"^\s*impl\s*(<[^>]*>)?")
        .unwrap()
        .replace(kopf.rsplit(" for ").next().unwrap(), "");
    Regex::new(r"[A-Z]\w*")
        .unwrap()
        .find(&typ)
        .unwrap()
        .as_str()
        .to_owned()
}

/// Das Enum aus clippys Vorschlag (`w @ PyWert::Null | ...`), `Self` und `use .. as A` aufgeloest.
fn enum_name(vorschlag: &str, datei: &str, zeile: usize) -> String {
    let erst = vorschlag.split(" | ").next().unwrap();
    let erst = erst.split_once(" @ ").map_or(erst, |(_, rest)| rest);
    let name = erst
        .split('(')
        .next()
        .unwrap()
        .rsplit("::")
        .nth(1)
        .expect("Pfad im Vorschlag");
    let quelle = std::fs::read_to_string(wurzel().join(datei)).unwrap();
    if name == "Self" {
        return self_typ(&quelle, zeile);
    }
    let alias = Regex::new(&format!(r"use [\w:]*::(\w+) as {name};")).unwrap();
    alias
        .captures(&quelle)
        .map_or_else(|| name.to_owned(), |c| c[1].to_owned())
}

#[test]
fn kein_wildcard_arm_auf_domain_enum() {
    // Eigenes Zielverzeichnis: andere clippy-Argumente als im CI-Schritt, sonst rechnete jeder
    // Wechsel den Workspace neu.
    let lauf = Command::new(env!("CARGO"))
        .current_dir(wurzel())
        .env(
            "CARGO_TARGET_DIR",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("tor1"),
        )
        .args(["clippy", "--workspace", "--message-format=json", "--"])
        .args(["-W", "clippy::wildcard_enum_match_arm"])
        .args(["-W", "clippy::match_wildcard_for_single_variants"])
        .output()
        .unwrap();
    assert!(
        lauf.status.success(),
        "{}",
        String::from_utf8_lossy(&lauf.stderr)
    );

    let mut arme = BTreeSet::new();
    for z in String::from_utf8(lauf.stdout).unwrap().lines() {
        let m: serde_json::Value = serde_json::from_str(z).unwrap();
        let msg = &m["message"];
        if !msg["code"]["code"]
            .as_str()
            .is_some_and(|c| c.contains("wildcard"))
        {
            continue;
        }
        let ort = msg["spans"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["is_primary"] == true)
            .unwrap();
        let datei = ort["file_name"].as_str().unwrap();
        let zeile = usize::try_from(ort["line_start"].as_u64().unwrap()).unwrap();
        let vorschlag = msg["children"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|c| c["spans"].as_array().unwrap())
            .find_map(|s| s["suggested_replacement"].as_str())
            .unwrap();
        arme.insert((datei.to_owned(), zeile, enum_name(vorschlag, datei, zeile)));
    }
    // Positivkontrolle: ohne Treffer auf `PyWert` misst der Lauf nichts.
    assert!(
        arme.iter().any(|(_, _, e)| e == "PyWert"),
        "keine Wildcard gemeldet: {arme:?}"
    );

    let fach = fach_enums();
    let treffer: Vec<String> = arme
        .iter()
        .filter(|(_, _, e)| fach.contains(e))
        .map(|(d, _, e)| format!("{d} {e}"))
        .collect();
    assert_eq!(
        treffer, OFFEN,
        "Wildcard auf Domain-Enum, oder ein Arm aus OFFEN ist erledigt"
    );
}
