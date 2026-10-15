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
//! - `TAXGRAPH_OHNE_XSD`: jeder Testschritt des Jobs `rust` (`cargo nextest run`, `cargo test --doc`) setzt
//!   `TAXGRAPH_OHNE_XSD` auf `1` (Entscheidung 2026-10-01, Log #142). Die Regel selbst (nur genau `1` ueberspringt) prueft
//!   `elster/tests/schemas_da_flag.rs`.
//! - `TAXGRAPH_OHNE_BROWSER`: die CI setzt es nirgends (Julius 2026-10-15, d-8). Der Runner bringt Chrome und `chromedriver`
//!   mit (Lauf 38459954734: 32 Oberflaechen-Tests, 0 Skips); ein Runner ohne sie macht den Lauf rot, statt still zu uebergehen.
//! - cargo-nextest (Entscheidung Julius 2026-10-15, d-1): der Job `rust` installiert eine feste Version mit SHA-256-Pruefsumme
//!   VOR dem Testschritt (die Version steht im Schrittnamen, im Archivnamen und in der URL und ist dieselbe), fuehrt die Tests
//!   mit `cargo nextest run --profile ci --no-fail-fast` aus und die Doctests (die nextest nicht kennt) danach mit
//!   `cargo test --doc`, `if: always()`. `rust/.config/nextest.toml` haelt das Profil `ci` (kein Abbruch beim ersten roten Test,
//!   Ausgabe bestandener Tests einmal am Ende) und die Gruppe, die die Browser-Tests der Oberflaeche nacheinander laufen laesst.
//!
//! Grenzen: Ob ein Schritt wirklich laeuft, ob die Actions die richtigen sind, ob der Runner das Flag so liest, ob die Pruefsumme
//! zum Archiv passt, prueft der Test nicht; er liest nur die Dateien. Das Profil und die nextest-Konfiguration liest er zeilenweise,
//! ohne TOML-Bibliothek: eine andere Schreibweise (`profile.dev.opt-level = 1` ausserhalb des Abschnitts) ist rot, nicht gruen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use regex::Regex;
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
    // nextest UND die Doctests: beide rufen `elster::testhilfe::schemas_da` (fuenf Doctests in `elster`).
    let tests: Vec<&Value> = schritte
        .iter()
        .filter(|s| {
            s.get("run")
                .and_then(Value::as_str)
                .is_some_and(|r| r.contains("cargo test") || r.contains("cargo nextest run"))
        })
        .collect();
    if tests.is_empty() {
        return vec!["der Job `rust` hat keinen `cargo nextest run`- oder `cargo test`-Schritt".to_string()];
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
                "der Testschritt `{}` setzt TAXGRAPH_OHNE_XSD nicht auf `1`: der Runner hat kein ERiC-Schema, der Schritt wird rot",
                s.get("name").and_then(Value::as_str).unwrap_or("?")
            )
        })
        .collect()
}

/// Die CI ueberspringt die Browser-Tests der Oberflaeche nicht (Julius 2026-10-15, d-8): `TAXGRAPH_OHNE_BROWSER` steht in
/// keiner Zeile des Workflows ausserhalb von Kommentaren (weder als `env` am Workflow, Job oder Schritt noch in einem `run`).
/// Der Runner bringt Chrome und `chromedriver` mit (Lauf 38459954734); ein Runner ohne sie macht den Lauf rot.
fn browser_flag(workflow_text: &str) -> Vec<String> {
    workflow_text
        .lines()
        .enumerate()
        .filter(|(_, z)| z.split('#').next().unwrap_or_default().contains("TAXGRAPH_OHNE_BROWSER"))
        .map(|(n, z)| format!("ci.yml:{}: {} (die CI darf die Browser-Tests nicht ueberspringen)", n + 1, z.trim()))
        .collect()
}

/// cargo-nextest im Job `rust`: eine feste Version mit SHA-256, installiert vor dem Testschritt; der Testschritt ruft
/// `cargo nextest run --profile ci --no-fail-fast`, und die Doctests (die nextest nicht kennt) laufen danach, auch bei rotem
/// nextest. Die Wege, die das Log fuer den Zaehlschritt schreiben, prueft `ci_nicht_geprueft.rs`.
fn nextest(wf: &Value) -> Vec<String> {
    let schritte = match rust_schritte(wf) {
        Ok(s) => s,
        Err(e) => return vec![e],
    };
    let run = |s: &Value| s.get("run").and_then(Value::as_str).unwrap_or_default().to_string();
    let wo = |f: &dyn Fn(&str) -> bool| schritte.iter().position(|s| f(&run(s)));
    let (i_inst, i_test, i_doc) = (
        wo(&|r| r.contains("cargo-nextest") && r.contains("sha256sum")),
        wo(&|r| r.contains("cargo nextest run")),
        wo(&|r| r.contains("cargo test --doc")),
    );
    let mut aus = Vec::new();
    match (i_inst, i_test) {
        (None, _) => aus.push("kein Schritt installiert cargo-nextest mit `sha256sum`: eine Version ohne Pruefsumme laedt, was gerade hinter der URL liegt".to_string()),
        (_, None) => aus.push("kein Schritt ruft `cargo nextest run`".to_string()),
        (Some(a), Some(b)) if a > b => aus.push("cargo-nextest wird NACH dem Testschritt installiert".to_string()),
        _ => {}
    }
    if let Some(i) = i_inst {
        let text = format!("{}\n{}", schritte[i].get("name").and_then(Value::as_str).unwrap_or_default(), run(&schritte[i]));
        let versionen: BTreeSet<String> = Regex::new(r"cargo-nextest[- ](\d+\.\d+\.\d+)")
            .unwrap()
            .captures_iter(&text)
            .map(|t| t[1].to_string())
            .collect();
        if versionen.len() != 1 {
            aus.push(format!("der Installationsschritt nennt {versionen:?} als Version von cargo-nextest, genau eine feste Version wird verlangt (nicht `latest`, nicht zwei Zahlen)"));
        }
        if let Some(v) = versionen.iter().next().filter(|_| versionen.len() == 1) {
            if !text.contains(&format!("/releases/download/cargo-nextest-{v}/")) {
                aus.push(format!("die URL nennt nicht `/releases/download/cargo-nextest-{v}/`: ohne die Version im Pfad holt sie die jeweils neueste"));
            }
        }
        if !Regex::new(r"\b[0-9a-f]{64}\b").unwrap().is_match(&text) {
            aus.push("der Installationsschritt nennt keine SHA-256-Pruefsumme (64 Hexziffern)".to_string());
        }
        if !text.contains("sha256sum -c") {
            aus.push("der Installationsschritt prueft die Summe nicht (`sha256sum -c`)".to_string());
        }
        if !text.contains("https://") || text.contains("http://") {
            aus.push("der Installationsschritt laedt nicht ausschliesslich ueber https".to_string());
        }
    }
    if let Some(t) = i_test {
        let r = run(&schritte[t]);
        for noetig in ["--profile ci", "--no-fail-fast"] {
            if !r.contains(noetig) {
                aus.push(format!("`cargo nextest run` ohne `{noetig}`: {r:?}"));
            }
        }
    }
    match (i_doc, i_test) {
        (None, _) => aus.push("kein Schritt ruft `cargo test --doc`: nextest fuehrt keine Doctests aus".to_string()),
        (Some(d), t) => {
            if t.is_some_and(|t| d < t) {
                aus.push("die Doctests laufen VOR nextest; `tee -a` haengt dann an ein Log, das es noch nicht gibt".to_string());
            }
            if schritte[d].get("if").and_then(Value::as_str) != Some("always()") {
                aus.push("der Doctest-Schritt hat kein `if: always()`: ein roter nextest-Lauf verdeckt die Doctests".to_string());
            }
        }
    }
    aus
}

/// `rust/.config/nextest.toml`: das Profil `ci` bricht nicht beim ersten roten Test ab und zeigt die Ausgabe bestandener Tests
/// einmal am Ende (dort stehen die Verzichtszeilen, die `scripts/ci-nicht-geprueft.sh` zaehlt; "immediate-final" zaehlte doppelt,
/// ein Profil ohne `success-output` zeigte sie nie); die Gruppe `oberflaeche-browser` (ein Test zugleich) gilt fuer das Test-Binary
/// `oberflaeche`: dort hielt ein `static Mutex` die Browser-Tests nacheinander, und nextest hat einen Prozess je Test.
fn nextest_konfiguration(text: &str) -> Vec<String> {
    let mut aus = Vec::new();
    match abschnitt(text, "[profile.ci]") {
        None => aus.push("`[profile.ci]` fehlt in rust/.config/nextest.toml".to_string()),
        Some(ci) => {
            let wert = |k: &str| ci.get(k).map(String::as_str);
            if wert("fail-fast") != Some("false") {
                aus.push(format!("Profil ci: fail-fast ist {:?}, nicht false: der erste rote Test bricht den Lauf ab", wert("fail-fast")));
            }
            if wert("success-output") != Some("\"final\"") {
                aus.push(format!("Profil ci: success-output ist {:?}, nicht \"final\": die Verzichtszeilen bestandener Tests fehlen im Log oder stehen doppelt", wert("success-output")));
            }
            if wert("failure-output").is_none() {
                aus.push("Profil ci: failure-output fehlt".to_string());
            }
        }
    }
    let gruppe = abschnitt(text, "[test-groups]").and_then(|g| g.get("oberflaeche-browser").cloned()).unwrap_or_default();
    if !gruppe.replace(' ', "").contains("max-threads=1") {
        aus.push(format!("Gruppe oberflaeche-browser hat nicht `max-threads = 1`: {gruppe:?}"));
    }
    let ueber = abschnitt(text, "[[profile.default.overrides]]").unwrap_or_default();
    if ueber.get("filter").map(String::as_str) != Some("'binary(oberflaeche)'") || ueber.get("test-group").map(String::as_str) != Some("'oberflaeche-browser'") {
        aus.push(format!("keine Ueberschreibung `filter = 'binary(oberflaeche)'` mit `test-group = 'oberflaeche-browser'` im Profil default: {ueber:?}"));
    }
    aus
}

/// Die Schluessel und Werte eines TOML-Abschnitts (`[kopf]`), Kommentare abgeschnitten.
fn abschnitt(text: &str, kopf: &str) -> Option<BTreeMap<String, String>> {
    let mut aus = None;
    let mut im_abschnitt = false;
    for roh in text.lines() {
        let zeile = roh.split('#').next().unwrap_or("").trim();
        if zeile.is_empty() {
            continue;
        }
        if zeile.starts_with('[') {
            im_abschnitt = zeile == kopf;
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

/// Die Schluessel und Werte von `[profile.dev]`, Kommentare abgeschnitten.
fn dev_profil(cargo: &str) -> Option<BTreeMap<String, String>> {
    abschnitt(cargo, "[profile.dev]")
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
fn die_testschritte_erlauben_ein_fehlendes_schema_ausdruecklich() {
    melde("TAXGRAPH_OHNE_XSD", &ohne_xsd(&workflow()));
}

#[test]
fn die_ci_ueberspringt_die_browser_tests_nicht() {
    let text = std::fs::read_to_string(wurzel().join(".github/workflows/ci.yml")).unwrap();
    melde("TAXGRAPH_OHNE_BROWSER", &browser_flag(&text));
    // Kontrolle: der Workflow nennt das Flag im Kommentar (die Suche sieht es, zaehlt es aber nicht).
    assert!(text.contains("TAXGRAPH_OHNE_BROWSER"), "Kontrolle: der Kommentar im Workflow nennt das Flag nicht mehr; die Suche ist ungeprueft");
    // Jeder Weg, das Flag zu setzen, wird erkannt; ein Kommentar und eine fremde Variable nicht.
    let gut = "      # TAXGRAPH_OHNE_BROWSER war hier\n        env:\n          TAXGRAPH_OHNE_XSD: \"1\"\n";
    assert!(browser_flag(gut).is_empty(), "{:?}", browser_flag(gut));
    for (was, zeile) in [
        ("env am Schritt", "        env:\n          TAXGRAPH_OHNE_BROWSER: \"1\"\n"),
        ("env am Job", "    env:\n      TAXGRAPH_OHNE_BROWSER: 1\n"),
        ("env am Workflow", "env:\n  TAXGRAPH_OHNE_BROWSER: '1'\n"),
        ("export in einem run", "        run: export TAXGRAPH_OHNE_BROWSER=1 && cargo nextest run\n"),
        ("Zeile mit Kommentar dahinter", "          TAXGRAPH_OHNE_BROWSER: \"1\" # Sicherung\n"),
    ] {
        assert!(!browser_flag(&format!("{gut}{zeile}")).is_empty(), "{was}: nicht erkannt");
    }
}

#[test]
fn cargo_nextest_ist_gepinnt_und_vor_dem_testschritt_installiert() {
    melde("nextest", &nextest(&workflow()));
}

#[test]
fn die_nextest_konfiguration_haelt_das_profil_ci_und_die_browser_gruppe() {
    let text = std::fs::read_to_string(wurzel().join("rust/.config/nextest.toml")).unwrap();
    melde("nextest.toml", &nextest_konfiguration(&text));
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
      - name: cargo-nextest 0.9.146 installieren (Pruefsumme)
        run: |
          set -e
          archiv=cargo-nextest-0.9.146-x86_64-unknown-linux-gnu.tar.gz
          curl --proto '=https' -fsSL -o "$archiv" "https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.146/$archiv"
          echo "682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428  $archiv" | sha256sum -c -
      - name: cargo nextest
        shell: bash
        run: cd rust && cargo nextest run --profile ci --no-fail-fast 2>&1 | tee ../cargo-test.log
        env:
          TAXGRAPH_OHNE_XSD: "1"
      - name: cargo test --doc
        if: always()
        shell: bash
        run: cd rust && cargo test --doc --no-fail-fast 2>&1 | tee -a ../cargo-test.log
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
    let pruefungen: [(&str, Pruefung); 6] = [
        ("Zeitlimit", zeitlimit),
        ("Nebenlaeufigkeit", nebenlaeufigkeit),
        ("Token", token),
        ("Cache", cache),
        ("TAXGRAPH_OHNE_XSD", ohne_xsd),
        ("nextest", nextest),
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
        ("Cache: zwei Schritte", cache, "      - name: cargo nextest\n", "      - uses: actions/cache@v4\n        with:\n          path: rust/target\n          key: rust/Cargo.toml\n      - name: cargo nextest\n"),
        ("Cache: kein Cache-Schritt", cache, "actions/cache@v4", "actions/other@v4"),
        ("TAXGRAPH_OHNE_XSD: Flag fehlt", ohne_xsd, "        env:\n          TAXGRAPH_OHNE_XSD: \"1\"\n", ""),
        ("TAXGRAPH_OHNE_XSD: true", ohne_xsd, "TAXGRAPH_OHNE_XSD: \"1\"", "TAXGRAPH_OHNE_XSD: \"true\""),
        ("TAXGRAPH_OHNE_XSD: 0", ohne_xsd, "TAXGRAPH_OHNE_XSD: \"1\"", "TAXGRAPH_OHNE_XSD: \"0\""),
        // Je Testschritt einzeln ohne das Flag (die Doctests rufen `schemas_da` ebenso wie nextest).
        ("TAXGRAPH_OHNE_XSD: Doctest-Schritt ohne Flag", ohne_xsd, "run: cd rust && cargo test --doc --no-fail-fast 2>&1 | tee -a ../cargo-test.log\n        env:\n          TAXGRAPH_OHNE_XSD: \"1\"\n", "run: cd rust && cargo test --doc --no-fail-fast 2>&1 | tee -a ../cargo-test.log\n"),
        ("TAXGRAPH_OHNE_XSD: nextest-Schritt ohne Flag", ohne_xsd, "tee ../cargo-test.log\n        env:\n          TAXGRAPH_OHNE_XSD: \"1\"\n", "tee ../cargo-test.log\n"),
    ];
    for (was, f, alt, neu) in yaml {
        let text = GUT.replace(alt, neu);
        assert_ne!(text, GUT, "{was}: der Mutant aendert nichts");
        assert!(!f(&laden(&text)).is_empty(), "{was}: nicht erkannt");
    }
    // Ohne jeden Testschritt (weder nextest noch Doctests) meldet ohne_xsd das ausdruecklich, statt nichts zu pruefen.
    let leer = GUT.replace("cargo nextest run", "cargo build").replace("cargo test --doc", "cargo build");
    assert_ne!(leer, GUT);
    assert!(!ohne_xsd(&laden(&leer)).is_empty(), "kein Testschritt: nicht erkannt");
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

/// Jeder Rueckfall in der nextest-Festlegung des Jobs `rust` wird erkannt, die gute Fassung nicht.
#[test]
fn die_nextest_pruefung_schlaegt_an() {
    assert!(nextest(&laden(GUT)).is_empty(), "{:?}", nextest(&laden(GUT)));
    let yaml: &[YamlFall] = &[
        ("ohne Pruefsumme", nextest, "682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428", "682c21b7"),
        ("Summe wird nicht geprueft", nextest, "| sha256sum -c -", "| sha256sum -"),
        ("Name nennt eine andere Version", nextest, "cargo-nextest 0.9.146 installieren", "cargo-nextest 0.9.145 installieren"),
        ("URL nennt eine andere Version", nextest, "download/cargo-nextest-0.9.146/", "download/cargo-nextest-0.9.145/"),
        ("URL nimmt die neueste Version", nextest, "releases/download/cargo-nextest-0.9.146/", "releases/latest/download/"),
        ("unverschluesselt", nextest, "https://github.com", "http://github.com"),
        ("ohne Profil ci", nextest, "--profile ci ", ""),
        ("ohne --no-fail-fast", nextest, "cargo nextest run --profile ci --no-fail-fast", "cargo nextest run --profile ci"),
        ("Doctests fehlen", nextest, "cargo test --doc --no-fail-fast", "cargo build"),
        ("Doctests ohne always()", nextest, "        if: always()\n", ""),
    ];
    for (was, f, alt, neu) in yaml {
        let text = GUT.replace(alt, neu);
        assert_ne!(text, GUT, "nextest: {was}: der Mutant aendert nichts");
        assert!(!f(&laden(&text)).is_empty(), "nextest: {was}: nicht erkannt");
    }
    // Umstellungen ganzer Schritte: Installation fehlt, Installation NACH dem Testschritt, Doctests VOR nextest.
    let (von, bis) = (GUT.find("      - name: cargo-nextest 0.9.146").unwrap(), GUT.find("      - name: cargo nextest\n").unwrap());
    let (doc, ende) = (GUT.find("      - name: cargo test --doc\n").unwrap(), GUT.find("  zweiter:").unwrap());
    let (installation, nextest_schritt, doctests) = (&GUT[von..bis], &GUT[bis..doc], &GUT[doc..ende]);
    let ohne_installation = GUT.replacen(installation, "", 1);
    let spaet = format!("{}{nextest_schritt}{doctests}{installation}{}", &GUT[..von], &GUT[ende..]);
    let doctests_zuerst = format!("{}{installation}{doctests}{nextest_schritt}{}", &GUT[..von], &GUT[ende..]);
    for (was, text) in [("Installation fehlt", ohne_installation), ("Installation nach dem Testschritt", spaet), ("Doctests vor nextest", doctests_zuerst)] {
        assert_ne!(text, GUT, "nextest: {was}: der Mutant aendert nichts");
        assert!(!nextest(&laden(&text)).is_empty(), "nextest: {was}: nicht erkannt");
    }
}

const NEXTEST_GUT: &str ="[test-groups]\noberflaeche-browser = { max-threads = 1 }\n\n[[profile.default.overrides]]\nfilter = 'binary(oberflaeche)'\ntest-group = 'oberflaeche-browser'\n\n[profile.ci]\nfail-fast = false # kein Abbruch\nsuccess-output = \"final\"\nfailure-output = \"immediate-final\"\n";

/// Jeder Rueckfall in `rust/.config/nextest.toml` wird erkannt, die gute Fassung nicht (auch nicht mit Kommentar hinter dem Wert).
#[test]
fn die_nextest_konfiguration_schlaegt_an() {
    assert!(nextest_konfiguration(NEXTEST_GUT).is_empty(), "{:?}", nextest_konfiguration(NEXTEST_GUT));
    let faelle: &[(&str, &str, &str)] = &[
        ("fail-fast an", "fail-fast = false", "fail-fast = true"),
        ("fail-fast fehlt", "fail-fast = false # kein Abbruch\n", ""),
        ("success-output immediate-final (doppelt im Log)", "success-output = \"final\"", "success-output = \"immediate-final\""),
        ("success-output fehlt (Verzichtszeilen fehlen im Log)", "success-output = \"final\"\n", ""),
        ("success-output never", "success-output = \"final\"", "success-output = \"never\""),
        ("failure-output fehlt", "failure-output = \"immediate-final\"\n", ""),
        ("Profil ci umbenannt", "[profile.ci]", "[profile.rot]"),
        ("Gruppe fehlt", "oberflaeche-browser = { max-threads = 1 }\n", ""),
        ("Gruppe mit zwei Tests zugleich", "max-threads = 1", "max-threads = 2"),
        ("Ueberschreibung trifft ein anderes Binary", "binary(oberflaeche)", "binary(http)"),
        ("Ueberschreibung ohne Gruppe", "test-group = 'oberflaeche-browser'\n", ""),
        ("Ueberschreibung mit anderer Gruppe", "test-group = 'oberflaeche-browser'", "test-group = 'andere'"),
        ("Ueberschreibung fehlt", "[[profile.default.overrides]]", "[[profile.default.nichts]]"),
    ];
    for (was, alt, neu) in faelle {
        let text = NEXTEST_GUT.replace(alt, neu);
        assert_ne!(text, NEXTEST_GUT, "{was}: der Mutant aendert nichts");
        assert!(!nextest_konfiguration(&text).is_empty(), "{was}: nicht erkannt");
    }
}
