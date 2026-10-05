//! Gate gegen den Tippfehler in einer Feld-Kennung (Backlog `feld-kennung-mit-tippfehler-liest-still-null`,
//! Entscheidung `feld-kennungs-gate-liest-das-verzeichnis-und-bricht-bei-null-treffern-ab`).
//!
//! Die Leser [`bescheid::feld_int_oder_null`], `feld_euro_oder_null` und `wert` lesen ein fehlendes Feld
//! als 0 bzw. `None` (`PARITÄT: fail-open default`; "nicht gesetzt" ist beim Nutzer der Normalfall). Eine
//! falsch geschriebene Kennung im Code liest deshalb ebenfalls still 0 -- eine Einkunft fiele ohne
//! Fehler aus der Rechnung. Dieser Test liest den Quelltext von `rust/bescheid/src/**` und gleicht jedes
//! Literal, das an einen der drei Leser geht, mit den `feld_id`s der Bindung ab. Das Gegenstück in Python
//! ist `tests/test_bindungstabelle.py::test_i_api_read_keys_sind_in_bindung`.
//!
//! Die Leser selbst bleiben fail-open; geprüft wird nur der Quelltext.
//!
//! Was er NICHT sieht (gelesen, nicht geprüft): Aufrufe, deren Kennung kein Literal ist -- eine Variable,
//! eine Konstante, `format!("{k}{suffix}")` oder ein lokaler Closure (`c("x")`, `ci("x")`). `fid("x")` mit
//! Suffix-Closure zählt als Literal `x`. Die Zahl der übersprungenen Aufrufe steht in der Fehlermeldung
//! der Mindestzahl-Prüfung.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use regex::Regex;

/// Mindestzahl verschiedener Literale, die das Gate lesen MUSS (ohne `AUSNAHMEN`). Gemessen, nicht
/// geschätzt (k9, 2026-10-03, dieser Scan auf `rust/bescheid/src/**` bei `b4fbe8b`): 223 Aufrufe der
/// drei Leser, 184 davon mit Literal, 143 verschiedene Literale, minus die Ausnahme `x` = 142. Ein Lauf
/// darunter hat Aufrufe verloren (Datei verschoben, Leser umbenannt) und prüft nichts mehr -- rot, nicht
/// grün. Wächst NUR bewusst; wer einen Aufruf mit Absicht löscht, senkt die Zahl im selben Commit.
const MIN_LITERALE: usize = 142;

/// Ausnahmen: (Datei unter `src/`, Literal, Grund). Kommentarzeilen (`//`, `///`, `//!`) zählen nicht
/// als Code; damit entfällt der Doctest von `feld_int_oder_null` (`"a"`, `"b"`, `"fehlt"`, `lib.rs`
/// Zeilen 268-270) ohne Eintrag hier.
const AUSNAHMEN: &[(&str, &str, &str)] = &[(
    "lib.rs",
    "x",
    "Eigenschaftstest `zahl_oder_null_wie_pywert`: legt sich das Feld mit `ein_feld(\"x\", ..)` selbst an \
     und liest es zurück; er liest nichts aus der Bindung.",
)];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn rs_dateien(dir: &Path, aus: &mut Vec<PathBuf>) {
    for eintrag in std::fs::read_dir(dir).unwrap() {
        let pfad = eintrag.unwrap().path();
        if pfad.is_dir() {
            rs_dateien(&pfad, aus);
        } else if pfad.extension().is_some_and(|e| e == "rs") {
            aus.push(pfad);
        }
    }
}

/// Was der Scan fand.
#[derive(Debug, Default)]
struct Befund {
    /// Literal -> Dateien (relativ zu `src/`), in denen es an einen Leser geht.
    literale: BTreeMap<String, BTreeSet<String>>,
    /// Aufrufe der drei Leser insgesamt (ohne Kommentarzeilen und ohne die Definition `fn ...`).
    aufrufe: usize,
    /// Davon Aufrufe, deren Kennung ein Literal ist (geprüft).
    mit_literal: usize,
}

/// Der Leser-Aufruf: Name, `(`, erstes Argument ohne Komma und Klammer, `,`, dann ein Literal oder
/// `&fid("Literal")`. Das erste Argument ist hier immer ein Pfad wie `f`, `&f`, `fi`, `&inst.felder`.
fn muster() -> (Regex, Regex) {
    (
        Regex::new(r"\b(?:wert|feld_int_oder_null|feld_euro_oder_null)\(").unwrap(),
        Regex::new(
            r#"\b(?:wert|feld_int_oder_null|feld_euro_oder_null)\(\s*[^,()]*,\s*(?:&fid\()?"([^"\\]*)""#,
        )
        .unwrap(),
    )
}

fn scanne(dateien: &[(String, String)]) -> Befund {
    let (aufruf, mit_literal) = muster();
    let mut b = Befund::default();
    for (name, text) in dateien {
        // Kommentarzeilen raus: Doctests und Prosa sind Beispiele, keine Leser. Die Zeilenlänge bleibt
        // nicht erhalten und muss es nicht (kein Zeilenbezug im Befund).
        let code: String = text
            .lines()
            .filter(|z| !z.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        // `fn wert<'a>(`/`fn feld_int_oder_null(` sind Definitionen, keine Aufrufe.
        b.aufrufe += aufruf
            .find_iter(&code)
            .filter(|m| !code[..m.start()].trim_end().ends_with("fn"))
            .count();
        for k in mit_literal.captures_iter(&code) {
            b.mit_literal += 1;
            b.literale
                .entry(k[1].to_owned())
                .or_default()
                .insert(name.clone());
        }
    }
    b
}

fn quelldateien() -> Vec<(String, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut pfade = Vec::new();
    rs_dateien(&src, &mut pfade);
    pfade.sort();
    pfade
        .into_iter()
        .map(|p| {
            let rel = p.strip_prefix(&src).unwrap().to_string_lossy().into_owned();
            (rel, std::fs::read_to_string(&p).unwrap())
        })
        .collect()
}

fn bindungs_kennungen() -> BTreeSet<String> {
    let reg = bindung::lade_registry_der_wurzel(&repo_root())
        .unwrap_or_else(|e| panic!("Registry-Aufbau gescheitert: {e}"));
    reg.dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter().map(|b| b.feld_id.clone()))
        .collect()
}

/// Das Gate selbst, getrennt vom Einlesen, damit die Gegenproben es mit anderen Mengen füttern.
/// Rückgabe: die Meldung, wenn das Gate rot ist.
fn pruefe(b: &Befund, bindung: &BTreeSet<String>) -> Result<(), String> {
    let echt: BTreeMap<&String, &BTreeSet<String>> = b
        .literale
        .iter()
        .filter(|(l, dateien)| {
            !AUSNAHMEN
                .iter()
                .any(|(d, lit, _)| lit == l && dateien.len() == 1 && dateien.contains(*d))
        })
        .collect();
    if echt.len() < MIN_LITERALE {
        return Err(format!(
            "Das Gate liest nur {} Literale, die Mindestzahl ist {MIN_LITERALE} ({} Aufrufe der drei \
             Leser gefunden, davon {} ohne Literal). Die Aufrufe liegen nicht mehr dort, wo das Gate \
             sucht (rust/bescheid/src/**) -- ein Gate ohne Treffer prüft nichts.",
            echt.len(),
            b.aufrufe,
            b.aufrufe - b.mit_literal,
        ));
    }
    let veraltet: Vec<&str> = AUSNAHMEN
        .iter()
        .filter(|(d, l, _)| !b.literale.get(*l).is_some_and(|ds| ds.contains(*d)))
        .map(|(_, l, _)| *l)
        .collect();
    if !veraltet.is_empty() {
        return Err(format!(
            "AUSNAHMEN nennt {veraltet:?}, aber der Scan findet sie nicht mehr -- Eintrag streichen, \
             sonst verdeckt die Liste später etwas anderes."
        ));
    }
    let unbekannt: Vec<String> = echt
        .iter()
        .filter(|(l, _)| !bindung.contains(l.as_str()))
        .map(|(l, d)| {
            format!(
                "{l} (in {})",
                d.iter().cloned().collect::<Vec<_>>().join(", ")
            )
        })
        .collect();
    if !unbekannt.is_empty() {
        return Err(format!(
            "Der Rechenkern liest Feld-Kennungen, die in keiner bindung_*.yaml als feld_id stehen: \
             {unbekannt:?}. Das Feld wurde umbenannt/gelöscht oder die Kennung ist falsch geschrieben, \
             die Lesestelle liest still 0 -> stiller Fehlbetrag (keine Test-Rot-Warnung)."
        ));
    }
    Ok(())
}

#[test]
fn lesestellen_des_rechenkerns_sind_in_der_bindung() {
    let b = scanne(&quelldateien());
    eprintln!(
        "GEMESSEN: {} Aufrufe, {} mit Literal, {} verschiedene Literale",
        b.aufrufe,
        b.mit_literal,
        b.literale.len()
    );
    if let Err(m) = pruefe(&b, &bindungs_kennungen()) {
        panic!("{m}");
    }
}

/// Gegenprobe zur Mindestzahl: liest der Scan nichts (leeres Verzeichnis), MUSS das Gate rot werden und
/// die Mindestzahl nennen.
#[test]
fn gate_bricht_ab_wenn_es_zu_wenig_liest() {
    let leer = scanne(&[]);
    let m = pruefe(&leer, &bindungs_kennungen())
        .expect_err("ein Lauf ohne Treffer darf nicht grün sein");
    assert!(m.contains(&MIN_LITERALE.to_string()), "{m}");
}

/// Gegenprobe zum Namensabgleich: ein Tippfehler an einem Leser MUSS auffallen, auch wenn der Rest der
/// Aufrufe stimmt.
#[test]
fn gate_faengt_einen_tippfehler() {
    let mut dateien = quelldateien();
    dateien.push((
        "zweige/tippfehler.rs".to_owned(),
        r#"let a = feld_int_oder_null(f, "bruttoarbeitslon")?;"#.to_owned(),
    ));
    let m = pruefe(&scanne(&dateien), &bindungs_kennungen())
        .expect_err("ein Tippfehler im Quelltext darf nicht grün sein");
    assert!(m.contains("bruttoarbeitslon"), "{m}");
}
