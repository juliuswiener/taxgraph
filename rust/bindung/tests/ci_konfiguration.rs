//! Waechter der CI-Konfiguration, soweit sie den Job `rust` und den Workflow im Ganzen betrifft: `.github/workflows/ci.yml`
//! und das Profil `[profile.dev]` in `rust/Cargo.toml`.
//!
//! Warum in Rust: Sechs der 31 Tests in `tests/test_ci_konfiguration.py` pruefen diese Dateien. Mit `tests/` faellt der Rest
//! der Datei (Catala-Version, GETTSIM-Pins, conftest-Guard, Python-Imports), diese sechs aber haengen am Job `rust`, der
//! bleibt. Jede Pruefung ist eine reine Funktion ueber den Text der Datei; `die_pruefungen_schlagen_an` fuettert sie mit
//! Fassungen, in denen genau der Rueckfall steckt.
//!
//! - Zeitlimit: jeder Job hat `timeout-minutes` (ein haengender Job liefe sonst sechs Stunden). Schaerfer als Python: die
//!   Zahl muss groesser als 0 sein, und der Job `rust` muss da sein.
//! - Keine doppelten Laeufe: ein `concurrency`-Block mit `cancel-in-progress: true`. Schaerfer als Python: die Gruppe
//!   haengt an `github.head_ref` (das fasst den `push`- und den `pull_request`-Lauf eines Commits zusammen).
//! - Token nur lesen: `permissions` im Workflow mit `contents: read`, nirgends `write`. Schaerfer als Python: auch ein
//!   `permissions` an einem Job zaehlt (es hebt das des Workflows auf), und `write-all` ist keine Tabelle.
//! - Cache-Schluessel: ein einziger Cache-Schritt im Job `rust`, er cacht `rust/target`, und sein Schluessel haengt an
//!   `rust/Cargo.toml` (dort steht das Profil; die Stufe steckt im Hash jedes Artefakts).
//! - Profil: `[profile.dev]` hat `opt-level = 1`, und `debug-assertions` und `overflow-checks` sind nicht `false`.
//!   `overflow-checks = true` ausdruecklich haelt `api/tests/ueberlauf_profil_waechter.rs` fest.
//! - `TAXGRAPH_OHNE_XSD`: der `cargo test`-Schritt des Jobs `rust` setzt `TAXGRAPH_OHNE_XSD` auf `1` (Entscheidung
//!   2026-10-01, Log #142). Die Regel selbst (nur genau `1` ueberspringt) prueft `elster/tests/schemas_da_flag.rs`.
//!
//! Grenzen: Ob ein Schritt wirklich laeuft, ob die Actions die richtigen sind, ob der Runner das Flag so liest, prueft der
//! Test nicht; er liest nur die Dateien. Das Profil liest er zeilenweise, ohne TOML-Bibliothek: eine andere Schreibweise
//! (`profile.dev.opt-level = 1` ausserhalb des Abschnitts) ist rot, nicht gruen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn laden(text: &str) -> Value {
    serde_yaml_ng::from_str(text).unwrap()
}

fn text_von(w: &Value) -> Option<String> {
    match w {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn name(k: &Value) -> String {
    text_von(k).unwrap_or_else(|| "?".to_string())
}

fn jobs(wf: &Value) -> Vec<(String, &Value)> {
    wf.get("jobs")
        .and_then(Value::as_mapping)
        .map(|m| m.iter().map(|(k, v)| (name(k), v)).collect())
        .unwrap_or_default()
}

/// Die Schritte des Jobs `rust`.
fn rust_schritte(wf: &Value) -> Result<&Vec<Value>, String> {
    wf.get("jobs")
        .and_then(|j| j.get("rust"))
        .and_then(|j| j.get("steps"))
        .and_then(Value::as_sequence)
        .ok_or_else(|| "der Job `rust` mit `steps` fehlt".to_string())
}

fn zeitlimit(wf: &Value) -> Vec<String> {
    let alle = jobs(wf);
    let mut aus: Vec<String> = alle
        .iter()
        .filter(|(_, job)| {
            job.get("timeout-minutes")
                .and_then(Value::as_u64)
                .is_none_or(|m| m == 0)
        })
        .map(|(n, _)| format!("Job `{n}` hat kein Zeitlimit (`timeout-minutes` groesser als 0)"))
        .collect();
    if !alle.iter().any(|(n, _)| n == "rust") {
        aus.push("der Job `rust` fehlt".to_string());
    }
    aus
}

fn nebenlaeufigkeit(wf: &Value) -> Vec<String> {
    let Some(c) = wf.get("concurrency").and_then(Value::as_mapping) else {
        return vec!["kein `concurrency`-Block: jeder PR-Commit faehrt zwei volle Laeufe".into()];
    };
    let mut aus = Vec::new();
    if c.get("cancel-in-progress").and_then(Value::as_bool) != Some(true) {
        aus.push("`cancel-in-progress` ist nicht `true`: ueberholte Laeufe laufen weiter".into());
    }
    let gruppe = c.get("group").and_then(text_von).unwrap_or_default();
    if !gruppe.contains("github.head_ref") {
        aus.push(format!(
            "die Gruppe `{gruppe}` haengt nicht an `github.head_ref`: der push- und der pull_request-Lauf eines Commits teilen sie nicht"
        ));
    }
    aus
}

/// Fehler an einer `permissions`-Angabe: keine Tabelle, oder ein `write` darin.
fn rechte_fehler(wo: &str, p: &Value) -> Vec<String> {
    let Some(m) = p.as_mapping() else {
        return vec![format!(
            "{wo}: `permissions` ist keine Tabelle (`write-all` oder Text)"
        )];
    };
    m.iter()
        .filter(|(_, v)| v.as_str() == Some("write"))
        .map(|(k, _)| format!("{wo}: Schreibrecht `{}`", name(k)))
        .collect()
}

fn token(wf: &Value) -> Vec<String> {
    let mut aus = Vec::new();
    match wf.get("permissions") {
        None => aus.push("kein `permissions`-Block: das Standard-Token darf schreiben".into()),
        Some(p) => {
            aus.extend(rechte_fehler("Workflow", p));
            if p.get("contents").and_then(Value::as_str) != Some("read") {
                aus.push("`contents` ist nicht `read`".into());
            }
        }
    }
    for (n, job) in jobs(wf) {
        if let Some(p) = job.get("permissions") {
            aus.extend(rechte_fehler(&format!("Job `{n}`"), p));
        }
    }
    aus
}

fn cache(wf: &Value) -> Vec<String> {
    let schritte = match rust_schritte(wf) {
        Ok(s) => s,
        Err(e) => return vec![e],
    };
    let caches: Vec<&Value> = schritte
        .iter()
        .filter(|s| {
            s.get("uses")
                .and_then(Value::as_str)
                .is_some_and(|u| u.starts_with("actions/cache@"))
        })
        .collect();
    if caches.len() != 1 {
        return vec![format!(
            "der Job `rust` hat {} Cache-Schritte statt einem",
            caches.len()
        )];
    }
    let mit = |k: &str| {
        caches[0]
            .get("with")
            .and_then(|w| w.get(k))
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    let mut aus = Vec::new();
    if !mit("path").contains("rust/target") {
        aus.push("der Job `rust` cacht `rust/target` nicht mehr".to_string());
    }
    if !mit("key").contains("rust/Cargo.toml") {
        aus.push(format!(
            "der Cache-Schluessel `{}` haengt nicht an `rust/Cargo.toml`, wo das Profil steht: nach einer Aenderung der Stufe bleibt der alte Cache stehen",
            mit("key")
        ));
    }
    aus
}

fn ohne_xsd(wf: &Value) -> Vec<String> {
    let schritte = match rust_schritte(wf) {
        Ok(s) => s,
        Err(e) => return vec![e],
    };
    let tests: Vec<&Value> = schritte
        .iter()
        .filter(|s| {
            s.get("run")
                .and_then(Value::as_str)
                .is_some_and(|r| r.contains("cargo test"))
        })
        .collect();
    if tests.is_empty() {
        return vec!["der Job `rust` hat keinen `cargo test`-Schritt".to_string()];
    }
    tests
        .iter()
        .filter(|s| {
            s.get("env")
                .and_then(|e| e.get("TAXGRAPH_OHNE_XSD"))
                .and_then(text_von)
                .as_deref()
                != Some("1")
        })
        .map(|s| {
            format!(
                "der `cargo test`-Schritt `{}` setzt TAXGRAPH_OHNE_XSD nicht auf `1`: der Runner hat kein ERiC-Schema, der Schritt wird rot",
                s.get("name").and_then(Value::as_str).unwrap_or("?")
            )
        })
        .collect()
}

/// Die Schluessel und Werte von `[profile.dev]`, Kommentare abgeschnitten.
fn dev_profil(cargo: &str) -> Option<BTreeMap<String, String>> {
    let mut aus = None;
    let mut im_abschnitt = false;
    for roh in cargo.lines() {
        let zeile = roh.split('#').next().unwrap_or("").trim();
        if zeile.is_empty() {
            continue;
        }
        if zeile.starts_with('[') {
            im_abschnitt = zeile == "[profile.dev]";
            if im_abschnitt {
                aus.get_or_insert_with(BTreeMap::new);
            }
            continue;
        }
        if im_abschnitt {
            if let Some((k, v)) = zeile.split_once('=') {
                aus.get_or_insert_with(BTreeMap::new)
                    .insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    aus
}

fn profil(cargo: &str) -> Vec<String> {
    let Some(dev) = dev_profil(cargo) else {
        return vec!["`[profile.dev]` fehlt in rust/Cargo.toml".to_string()];
    };
    let mut aus = Vec::new();
    if dev.get("opt-level").map(String::as_str) != Some("1") {
        aus.push(format!(
            "`[profile.dev]` opt-level ist {:?}, nicht 1: die Laeufe werden 2- bis 5-mal so langsam, und der Cache-Schluessel des Jobs `rust` passt nicht mehr",
            dev.get("opt-level")
        ));
    }
    for schalter in ["debug-assertions", "overflow-checks"] {
        if dev.get(schalter).map(String::as_str) == Some("false") {
            aus.push(format!(
                "`[profile.dev]` {schalter} ist ausgeschaltet: Ueberlauf und verletzte Annahmen in der Geldrechnung bleiben in den Tests unbemerkt"
            ));
        }
    }
    aus
}

fn melde(wer: &str, fehler: &[String]) {
    assert!(fehler.is_empty(), "{wer}:\n  {}", fehler.join("\n  "));
}

fn workflow() -> Value {
    laden(&std::fs::read_to_string(wurzel().join(".github/workflows/ci.yml")).unwrap())
}

#[test]
fn jeder_job_hat_ein_zeitlimit() {
    melde("Zeitlimit", &zeitlimit(&workflow()));
}

#[test]
fn laeufe_werden_nicht_verdoppelt() {
    melde("Nebenlaeufigkeit", &nebenlaeufigkeit(&workflow()));
}

#[test]
fn token_darf_nur_lesen() {
    melde("Token", &token(&workflow()));
}

#[test]
fn der_rust_cache_schluessel_kennt_die_optimierungsstufe() {
    melde("Cache", &cache(&workflow()));
}

#[test]
fn der_cargo_test_schritt_erlaubt_ein_fehlendes_schema_ausdruecklich() {
    melde("TAXGRAPH_OHNE_XSD", &ohne_xsd(&workflow()));
}

#[test]
fn rust_tests_laufen_mit_opt_level_1_und_pruefungen_bleiben_an() {
    let cargo = std::fs::read_to_string(wurzel().join("rust/Cargo.toml")).unwrap();
    melde("Profil", &profil(&cargo));
}

/// Eine Pruefung ueber den Workflow.
type Pruefung = fn(&Value) -> Vec<String>;
/// Ein Fall: Name, Pruefung, gesuchter Text, Ersatz.
type YamlFall = (&'static str, Pruefung, &'static str, &'static str);

const GUT: &str = r#"name: CI
"on":
  push:
  pull_request:
concurrency:
  group: ${{ github.workflow }}-${{ github.head_ref || github.ref }}
  cancel-in-progress: true
permissions:
  contents: read
jobs:
  rust:
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v4
      - name: Cargo-Registry cachen
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            rust/target
          key: cargo-${{ hashFiles('rust/Cargo.lock', 'rust/Cargo.toml') }}
      - name: cargo test
        run: cd rust && cargo test --no-fail-fast
        env:
          TAXGRAPH_OHNE_XSD: "1"
  zweiter:
    runs-on: ubuntu-latest
    timeout-minutes: 5
    steps:
      - run: echo zweiter
"#;

const CARGO_GUT: &str = "[workspace]\nmembers = []\n\n[profile.dev]\nopt-level = 1 # Stufe\noverflow-checks = true\n\n[profile.release]\nopt-level = 3\n";

/// Jede Pruefung schlaegt an, wenn genau ihr Rueckfall in der Datei steckt, und schweigt bei der guten Fassung.
#[test]
fn die_pruefungen_schlagen_an() {
    let gut = laden(GUT);
    let pruefungen: [(&str, Pruefung); 5] = [
        ("Zeitlimit", zeitlimit),
        ("Nebenlaeufigkeit", nebenlaeufigkeit),
        ("Token", token),
        ("Cache", cache),
        ("TAXGRAPH_OHNE_XSD", ohne_xsd),
    ];
    for (wer, f) in pruefungen {
        assert!(
            f(&gut).is_empty(),
            "{wer} meldet die gute Fassung: {:?}",
            f(&gut)
        );
    }
    assert!(profil(CARGO_GUT).is_empty(), "{:?}", profil(CARGO_GUT));

    let yaml: &[YamlFall] = &[
        ("Zeitlimit: Job rust ohne", zeitlimit, "    timeout-minutes: 20\n", ""),
        ("Zeitlimit: anderer Job ohne", zeitlimit, "    timeout-minutes: 5\n", ""),
        ("Zeitlimit: 0", zeitlimit, "timeout-minutes: 20", "timeout-minutes: 0"),
        ("Zeitlimit: keine Zahl", zeitlimit, "timeout-minutes: 20", "timeout-minutes: lang"),
        ("Zeitlimit: Job rust heisst anders", zeitlimit, "  rust:\n", "  rost:\n"),
        ("Nebenlaeufigkeit: Block fehlt", nebenlaeufigkeit, "concurrency:\n  group: ${{ github.workflow }}-${{ github.head_ref || github.ref }}\n  cancel-in-progress: true\n", ""),
        ("Nebenlaeufigkeit: nicht abbrechen", nebenlaeufigkeit, "cancel-in-progress: true", "cancel-in-progress: false"),
        ("Nebenlaeufigkeit: Gruppe ohne head_ref", nebenlaeufigkeit, "${{ github.workflow }}-${{ github.head_ref || github.ref }}", "${{ github.run_id }}"),
        ("Token: Block fehlt", token, "permissions:\n  contents: read\n", ""),
        ("Token: contents write", token, "contents: read", "contents: write"),
        ("Token: write-all", token, "permissions:\n  contents: read\n", "permissions: write-all\n"),
        ("Token: weiteres Schreibrecht", token, "  contents: read\n", "  contents: read\n  pull-requests: write\n"),
        ("Token: Schreibrecht am Job", token, "    timeout-minutes: 5\n", "    timeout-minutes: 5\n    permissions:\n      contents: write\n"),
        ("Cache: Schluessel ohne Cargo.toml", cache, "'rust/Cargo.lock', 'rust/Cargo.toml'", "'rust/Cargo.lock'"),
        ("Cache: rust/target nicht gecacht", cache, "            rust/target\n", ""),
        ("Cache: zwei Schritte", cache, "      - name: cargo test\n", "      - uses: actions/cache@v4\n        with:\n          path: rust/target\n          key: rust/Cargo.toml\n      - name: cargo test\n"),
        ("Cache: kein Cache-Schritt", cache, "actions/cache@v4", "actions/other@v4"),
        ("TAXGRAPH_OHNE_XSD: Flag fehlt", ohne_xsd, "        env:\n          TAXGRAPH_OHNE_XSD: \"1\"\n", ""),
        ("TAXGRAPH_OHNE_XSD: true", ohne_xsd, "TAXGRAPH_OHNE_XSD: \"1\"", "TAXGRAPH_OHNE_XSD: \"true\""),
        ("TAXGRAPH_OHNE_XSD: 0", ohne_xsd, "TAXGRAPH_OHNE_XSD: \"1\"", "TAXGRAPH_OHNE_XSD: \"0\""),
        ("TAXGRAPH_OHNE_XSD: kein cargo test", ohne_xsd, "cargo test --no-fail-fast", "cargo build"),
    ];
    for (was, f, alt, neu) in yaml {
        let text = GUT.replace(alt, neu);
        assert_ne!(text, GUT, "{was}: der Mutant aendert nichts");
        assert!(!f(&laden(&text)).is_empty(), "{was}: nicht erkannt");
    }
    // Ein Wert `1` ohne Anfuehrungszeichen ist fuer GitHub derselbe Text `1`.
    let zahl = GUT.replace("TAXGRAPH_OHNE_XSD: \"1\"", "TAXGRAPH_OHNE_XSD: 1");
    assert_eq!(ohne_xsd(&laden(&zahl)).len(), 0);

    let toml: &[(&str, &str, &str)] = &[
        ("opt-level 0", "opt-level = 1 # Stufe", "opt-level = 0"),
        (
            "opt-level fehlt im Abschnitt",
            "opt-level = 1 # Stufe\n",
            "",
        ),
        (
            "opt-level nur auskommentiert",
            "opt-level = 1 # Stufe",
            "# opt-level = 1",
        ),
        (
            "overflow-checks aus",
            "overflow-checks = true",
            "overflow-checks = false",
        ),
        (
            "debug-assertions aus",
            "overflow-checks = true",
            "debug-assertions = false",
        ),
        ("Abschnitt umbenannt", "[profile.dev]", "[profile.test]"),
        (
            "Abschnitt fehlt",
            "[profile.dev]\nopt-level = 1 # Stufe\noverflow-checks = true\n\n",
            "",
        ),
    ];
    for (was, alt, neu) in toml {
        let text = CARGO_GUT.replace(alt, neu);
        assert_ne!(text, CARGO_GUT, "{was}: der Mutant aendert nichts");
        assert!(!profil(&text).is_empty(), "{was}: nicht erkannt");
    }
    // `opt-level = 1` zaehlt nur im Abschnitt `[profile.dev]`, nicht im Nachbarabschnitt.
    let falsch = "[profile.dev]\noverflow-checks = true\n\n[profile.release]\nopt-level = 1\n";
    assert_ne!(profil(falsch).len(), 0);
}
