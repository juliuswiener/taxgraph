//! Waechter der Stufe-B-Verdrahtung: `Makefile`, `.github/workflows/ci.yml` und `requirements-ci.txt`, soweit sie die
//! Python-Tests unter `pipeline/tests/` betreffen.
//!
//! Warum in Rust: Die Stufe-B-Tests liegen seit `orch/stufeb-tests` unter `pipeline/tests/` und laufen ueber
//! `make unit-stufe-b` und den CI-Job `stufe-b`. Ihre Verdrahtung hielt bisher nur `tests/test_ci_konfiguration.py`, und
//! `tests/` faellt mit dem Loeschplan (L2). Drei Mutanten blieben dort gruen: `all` ohne `unit-stufe-b`, der Job `stufe-b`
//! mit einer Einzeldatei statt dem Verzeichnis, und der Manifest-Waechter selbst lag in der Datei, die faellt.
//!
//! - `all` im Makefile ruft `unit-stufe-b` auf (Voraussetzung der Zeile `all:`, nicht der `.PHONY`-Zeile und nicht ein
//!   Kommentar), und das Rezept von `unit-stufe-b` ruft pytest auf dem Verzeichnis `pipeline/tests` auf, nicht auf einer Datei.
//! - Der Job `stufe-b` in `ci.yml` ruft pytest auf dem Verzeichnis `pipeline/tests` auf und installiert ueber
//!   `-r requirements-ci.txt`.
//! - Jedes Fremdpaket, das die Python-Dateien unter `pipeline/tests/` auf Modulebene importieren, direkt oder ueber ein
//!   eigenes Modul des Repos, steht in `requirements-ci.txt`. Das ist die Pruefung aus
//!   `test_alle_importierten_fremdpakete_stehen_im_manifest` (Anlass 2026-08-18: `requests` fehlte, jede Sammlung brach ab),
//!   nur ab `pipeline/tests/` statt ab `tests/`, ohne Python-AST: Modulebene heisst hier "Zeile faengt in Spalte 0 an", die
//!   Docstrings werden ausgelassen, und wie dort endet die Suche in einer Datei an `pytest.importorskip(...)`.
//!
//! Nicht uebernommen aus `tests/test_ci_konfiguration.py`: das Scannen ab `tests/` (faellt mit L2, und ein Wachtposten ueber
//! eine geloeschte Wurzel ist keiner), die Catala- und GETTSIM-Pins, die conftest-Guards, die ERiC-Marker, die Skripte, die
//! CI startet. Das sind andere Jobs oder Dateien, die mit `tests/` und `produkt/` entfallen; ihre Pins prueft der Job selbst.
//! Sechs Pruefungen am Job `rust` stehen in `ci_konfiguration.rs`.
//!
//! Grenzen: Der Test liest nur Dateien. Ob ein Schritt wirklich laeuft, ob `continue-on-error` oder `if:` ihn abschaltet, ob
//! die Python-Version stimmt, prueft er nicht. Die Liste der Stdlib-Module ist die Vereinigung von 3.12 und 3.14; ein Paket,
//! das in einer neueren Python-Version Stdlib wird, meldet er als fehlend (laut, nicht still). Ein eigenes Modul, das wie ein
//! Fremdpaket heisst, verdeckt es; `kein_manifest_paket_wird_von_einem_eigenen_modul_verdeckt` faengt das fuer die Pakete des
//! Manifests. Das Importieren selbst liest er mit zwei Regeln, nicht mit einem Parser: `import a, b` und `from a import b` in
//! Spalte 0, Fortsetzungszeilen mit `\` nicht.
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

fn lies(rel: &str) -> String {
    std::fs::read_to_string(wurzel().join(rel)).unwrap()
}

// ---------------------------------------------------------------- Makefile

/// Voraussetzungen der Zeile `ziel: a b c` (Fortsetzung mit `\`, Kommentar ab `#` ab). `None`: kein solches Ziel. `ziel::`,
/// `ziel:=` und die `.PHONY`-Zeile zaehlen nicht.
fn voraussetzungen(makefile: &str, ziel: &str) -> Option<Vec<String>> {
    let kopf = format!("{ziel}:");
    let zeilen: Vec<&str> = makefile.lines().collect();
    let i = zeilen.iter().position(|z| {
        z.strip_prefix(&kopf)
            .is_some_and(|rest| !rest.starts_with([':', '=']))
    })?;
    let mut text = zeilen[i][kopf.len()..].to_string();
    let mut j = i;
    while text.trim_end().ends_with('\\') && j + 1 < zeilen.len() {
        text = text.trim_end().trim_end_matches('\\').to_string() + " " + zeilen[j + 1];
        j += 1;
    }
    let ohne_kommentar = text.split('#').next().unwrap_or_default();
    Some(
        ohne_kommentar
            .split_whitespace()
            .map(str::to_string)
            .collect(),
    )
}

/// Rezeptzeilen des Ziels: die Zeilen mit Tabulator direkt unter `ziel:`.
fn rezept(makefile: &str, ziel: &str) -> Option<Vec<String>> {
    let kopf = format!("{ziel}:");
    let zeilen: Vec<&str> = makefile.lines().collect();
    let i = zeilen.iter().position(|z| z.starts_with(&kopf))?;
    Some(
        zeilen[i + 1..]
            .iter()
            .take_while(|z| z.starts_with('\t'))
            .map(|z| z.trim().to_string())
            .collect(),
    )
}

/// Eine Zeile mit `pytest` und dem Verzeichnis `pipeline/tests` als eigenem Wort (nicht `pipeline/tests/test_x.py`).
fn pytest_auf_verzeichnis(zeile: &str) -> bool {
    let worte: Vec<&str> = zeile
        .split_whitespace()
        .map(|w| w.trim_matches(['"', '\'']))
        .collect();
    worte.contains(&"pytest")
        && worte
            .iter()
            .any(|w| matches!(*w, "pipeline/tests" | "pipeline/tests/"))
}

fn make_fehler(makefile: &str) -> Vec<String> {
    let mut aus = Vec::new();
    match voraussetzungen(makefile, "all") {
        None => aus.push("das Ziel `all` fehlt".to_string()),
        Some(v) if !v.iter().any(|x| x == "unit-stufe-b") => aus.push(format!(
            "`all` ruft `unit-stufe-b` nicht auf (Voraussetzungen: {})",
            v.join(" ")
        )),
        Some(_) => {}
    }
    match rezept(makefile, "unit-stufe-b") {
        None => aus.push("das Ziel `unit-stufe-b` fehlt".to_string()),
        Some(r) if !r.iter().any(|z| pytest_auf_verzeichnis(z)) => aus.push(format!(
            "das Rezept von `unit-stufe-b` ruft pytest nicht auf dem Verzeichnis `pipeline/tests` auf: {r:?}"
        )),
        Some(_) => {}
    }
    aus
}

// ---------------------------------------------------------------- ci.yml

/// Alle Zeilen aller `run`-Schritte des Jobs `stufe-b`.
fn run_zeilen(job: &Value) -> Vec<String> {
    job.get("steps")
        .and_then(Value::as_sequence)
        .map(|s| {
            s.iter()
                .filter_map(|schritt| schritt.get("run").and_then(Value::as_str))
                .flat_map(str::lines)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn installiert_ueber_manifest(zeile: &str) -> bool {
    let worte: Vec<&str> = zeile.split_whitespace().collect();
    worte
        .windows(2)
        .any(|p| p[0] == "-r" && p[1] == "requirements-ci.txt")
}

fn job_fehler(workflow: &str) -> Vec<String> {
    let wf: Value = serde_yaml_ng::from_str(workflow).unwrap();
    let Some(job) = wf.get("jobs").and_then(|j| j.get("stufe-b")) else {
        return vec!["der Job `stufe-b` fehlt".to_string()];
    };
    let zeilen = run_zeilen(job);
    let mut aus = Vec::new();
    if !zeilen.iter().any(|z| pytest_auf_verzeichnis(z)) {
        aus.push(format!(
            "der Job `stufe-b` ruft pytest nicht auf dem Verzeichnis `pipeline/tests` auf: {zeilen:?}"
        ));
    }
    if !zeilen.iter().any(|z| installiert_ueber_manifest(z)) {
        aus.push(format!(
            "der Job `stufe-b` installiert nicht ueber `-r requirements-ci.txt`: {zeilen:?}"
        ));
    }
    aus
}

// ---------------------------------------------------------------- Manifest

/// Namen, unter denen ein Paket importiert wird, weichen manchmal vom Paketnamen ab (wie im Python-Waechter).
const ALIAS: [(&str, &str); 4] = [
    ("pil", "pillow"),
    ("yaml", "pyyaml"),
    ("jwt", "pyjwt"),
    ("dateutil", "python-dateutil"),
];

/// Paketnamen aus einer requirements-Datei, klein, ohne Version, Extras und Kommentare.
fn pakete(requirements: &str) -> BTreeSet<String> {
    requirements
        .lines()
        .map(|z| z.split('#').next().unwrap_or_default().trim())
        .filter(|z| !z.is_empty())
        .map(|z| {
            z.split(['<', '>', '=', '!', '~', '['])
                .next()
                .unwrap_or_default()
                .trim()
                .to_lowercase()
        })
        .collect()
}

/// Verzeichnisse, in denen nichts Eigenes steckt: versteckte, `__pycache__`, Bauverzeichnisse.
fn uebersprungen(name: &str) -> bool {
    name.starts_with('.')
        || name == "__pycache__"
        || name.starts_with("target")
        || name == "node_modules"
}

fn sammle_py(dir: &Path, aus: &mut Vec<PathBuf>) {
    let Ok(eintraege) = std::fs::read_dir(dir) else {
        return;
    };
    for e in eintraege {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if p.is_dir() {
            if !uebersprungen(&name) {
                sammle_py(&p, aus);
            }
        } else if p.extension().is_some_and(|x| x == "py") {
            aus.push(p);
        }
    }
}

/// Die eigenen Module des Repos: Dateiname ohne `.py` und Namen der Verzeichnisse, unter denen eine `.py`-Datei liegt
/// (`from pipeline.ui import x` meint ein Verzeichnis ohne `__init__.py`). Dazu je Name die Dateien (aufsteigend sortiert).
fn eigene_module(wurzel: &Path) -> (BTreeSet<String>, BTreeMap<String, Vec<PathBuf>>) {
    let mut dateien = Vec::new();
    sammle_py(wurzel, &mut dateien);
    dateien.sort();
    let mut namen = BTreeSet::new();
    let mut je_name: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for d in dateien {
        let rel = d.strip_prefix(wurzel).unwrap();
        for teil in rel.parent().into_iter().flat_map(Path::components) {
            namen.insert(teil.as_os_str().to_string_lossy().into_owned());
        }
        let stamm = d.file_stem().unwrap().to_string_lossy().into_owned();
        namen.insert(stamm.clone());
        je_name.entry(stamm).or_default().push(d);
    }
    (namen, je_name)
}

/// Erster Namensteil vor einem Punkt.
fn kopf_des_namens(modul: &str) -> String {
    modul.split('.').next().unwrap_or_default().to_string()
}

/// Importierte Modulnamen (erster Teil) aus den Anweisungen, die in Spalte 0 beginnen. Ausgelassen: Docstrings und andere
/// dreifach gequotete Bloecke, relative Importe (`from .x import y`), eingerueckte Importe (in `try`, `if`, Funktionen).
/// Die Suche endet an der ersten Zeile in Spalte 0, die `importorskip(` enthaelt: alles danach wird nur erreicht, wenn
/// das genannte Paket da ist.
fn modulebene_importe(text: &str) -> Vec<String> {
    let import = Regex::new(r"^import\s+(\S.*)$").unwrap();
    let von = Regex::new(r"^from\s+([^\s.]\S*)\s+import\b").unwrap();
    let mut aus = Vec::new();
    let mut offen: Option<&str> = None;
    for zeile in text.lines() {
        if let Some(ende) = offen {
            if zeile.contains(ende) {
                offen = None;
            }
            continue;
        }
        if zeile.starts_with(|c: char| !c.is_whitespace() && c != '#') {
            if zeile.contains("importorskip(") {
                break;
            }
            if let Some(m) = von.captures(zeile) {
                aus.push(kopf_des_namens(&m[1]));
            } else if let Some(m) = import.captures(zeile) {
                let anweisung = m[1].split('#').next().unwrap_or_default();
                for teil in anweisung.split(',') {
                    if let Some(name) = teil.split_whitespace().next() {
                        aus.push(kopf_des_namens(name.trim_end_matches(';')));
                    }
                }
            }
        }
        for ende in ["\"\"\"", "'''"] {
            if zeile.matches(ende).count() % 2 == 1 {
                offen = Some(ende);
            }
        }
    }
    aus
}

struct Analyse {
    dateien: usize,
    /// Fremdpaket (Importname) -> erste Datei, die es importiert.
    fremd: BTreeMap<String, String>,
    /// Davon die, die nicht im Manifest stehen, als `name (zuerst in datei)`.
    fehlend: Vec<String>,
}

/// Die transitive Huelle der Modulebene-Importe ab `start`; eigene Module werden verfolgt, Stdlib und Fremdpakete nicht.
fn analysiere(wurzel: &Path, start: &str, requirements: &str) -> Analyse {
    let manifest = pakete(requirements);
    let (eigene, je_name) = eigene_module(wurzel);
    let mut offen: Vec<PathBuf> = je_name
        .values()
        .flatten()
        .filter(|p| p.parent() == Some(&wurzel.join(start)))
        .cloned()
        .collect();
    let mut gesehen = BTreeSet::new();
    let mut fremd = BTreeMap::new();
    while let Some(datei) = offen.pop() {
        if !gesehen.insert(datei.clone()) {
            continue;
        }
        let text = String::from_utf8_lossy(&std::fs::read(&datei).unwrap()).into_owned();
        for name in modulebene_importe(&text) {
            if STDLIB.contains(&name.as_str()) {
                continue;
            }
            if let Some(dateien) = je_name.get(&name) {
                offen.extend(dateien.iter().cloned());
            } else if !eigene.contains(&name) {
                let rel = datei
                    .strip_prefix(wurzel)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                fremd.entry(name).or_insert(rel);
            }
        }
    }
    let fehlend = fremd
        .iter()
        .filter(|(name, _)| {
            let klein = name.to_lowercase();
            let paket = ALIAS
                .iter()
                .find(|(a, _)| *a == klein)
                .map_or(klein.as_str(), |(_, p)| *p);
            !manifest.contains(paket)
        })
        .map(|(name, datei)| format!("{name} (zuerst in {datei})"))
        .collect();
    Analyse {
        dateien: gesehen.len(),
        fremd,
        fehlend,
    }
}

/// Pakete des Manifests, deren Importname ein eigenes Modul traegt: der Waechter hielte sie fuer eigene und schwiege.
fn verdeckt(wurzel: &Path, requirements: &str) -> Vec<String> {
    let (eigene, _) = eigene_module(wurzel);
    let eigene_klein: BTreeSet<String> = eigene.iter().map(|n| n.to_lowercase()).collect();
    let mut aus = Vec::new();
    for paket in pakete(requirements) {
        let mut namen = vec![paket.replace('-', "_")];
        namen.extend(
            ALIAS
                .iter()
                .filter(|(_, p)| *p == paket)
                .map(|(a, _)| (*a).to_string()),
        );
        for n in namen {
            if eigene_klein.contains(&n) {
                aus.push(format!(
                    "das Paket `{paket}` heisst im Repo wie ein eigenes Modul (`{n}`)"
                ));
            }
        }
    }
    aus
}

// ---------------------------------------------------------------- die Pruefungen am echten Baum

fn melde(wer: &str, fehler: &[String]) {
    assert!(fehler.is_empty(), "{wer}:\n  {}", fehler.join("\n  "));
}

#[test]
fn all_ruft_unit_stufe_b_auf_und_das_ziel_fuehrt_das_verzeichnis_aus() {
    melde("Makefile", &make_fehler(&lies("Makefile")));
}

#[test]
fn der_job_stufe_b_fuehrt_pipeline_tests_als_verzeichnis_aus() {
    melde("ci.yml", &job_fehler(&lies(".github/workflows/ci.yml")));
}

#[test]
fn jedes_fremdpaket_der_stufe_b_tests_steht_im_manifest() {
    let a = analysiere(&wurzel(), "pipeline/tests", &lies("requirements-ci.txt"));
    melde("requirements-ci.txt", &a.fehlend);
}

/// Ein Waechter, der nichts findet, ist gruen. Gemessen 2026-10-06 (Python-Prototyp, gleiche Regeln): 34 Dateien,
/// Fremdpakete pytest, requests, yaml. Die Untergrenzen sind lockerer als die Messung; sinkt die Zahl darunter, ist
/// `pipeline/tests/` verschoben oder der Scan beschnitten.
#[test]
fn der_waechter_sieht_den_echten_baum() {
    let a = analysiere(&wurzel(), "pipeline/tests", &lies("requirements-ci.txt"));
    assert!(
        a.dateien >= 20,
        "nur {} Dateien in der Huelle ab pipeline/tests, erwartet mindestens 20",
        a.dateien
    );
    assert!(
        !a.fremd.is_empty(),
        "kein einziges Fremdpaket gefunden: die Einordnung eigen/fremd ist kaputt"
    );
}

#[test]
fn kein_manifest_paket_wird_von_einem_eigenen_modul_verdeckt() {
    melde(
        "Verdeckung",
        &verdeckt(&wurzel(), &lies("requirements-ci.txt")),
    );
}

// ---------------------------------------------------------------- die Pruefungen schlagen an

const MAKE_GUT: &str = ".PHONY: all unit-stufe-b\n\nall: unit unit-stufe-b tests s02\n\nunit:\n\tpython3 -m pytest tests/ -q\n\nunit-stufe-b:\n\tpython3 -m pytest pipeline/tests -q\n\nclean:\n\trm -r x\n";

#[test]
fn die_make_pruefung_schlaegt_an() {
    assert!(
        make_fehler(MAKE_GUT).is_empty(),
        "{:?}",
        make_fehler(MAKE_GUT)
    );
    let faelle: &[(&str, &str, &str)] = &[
        (
            "all ohne unit-stufe-b",
            "all: unit unit-stufe-b tests s02",
            "all: unit tests s02",
        ),
        (
            "all nennt es nur im Kommentar",
            "all: unit unit-stufe-b tests s02",
            "all: unit tests s02 # unit-stufe-b",
        ),
        (
            "all fehlt, nur .PHONY nennt es",
            "all: unit unit-stufe-b tests s02\n",
            "",
        ),
        (
            "all als Variable",
            "all: unit unit-stufe-b tests s02",
            "all:= unit tests s02",
        ),
        (
            "Ziel fehlt",
            "unit-stufe-b:\n\tpython3 -m pytest pipeline/tests -q\n",
            "",
        ),
        (
            "Rezept mit einer Einzeldatei",
            "pytest pipeline/tests -q",
            "pytest pipeline/tests/test_quellen.py -q",
        ),
        (
            "Rezept auf tests/",
            "pytest pipeline/tests -q",
            "pytest tests/ -q",
        ),
        (
            "Rezept ohne pytest",
            "python3 -m pytest pipeline/tests -q",
            "ls pipeline/tests",
        ),
        (
            "Rezept leer",
            "unit-stufe-b:\n\tpython3 -m pytest pipeline/tests -q\n",
            "unit-stufe-b:\n",
        ),
    ];
    for (was, alt, neu) in faelle {
        let text = MAKE_GUT.replace(alt, neu);
        assert_ne!(text, MAKE_GUT, "{was}: der Mutant aendert nichts");
        assert!(!make_fehler(&text).is_empty(), "{was}: nicht erkannt");
    }
    // Gute Schreibweisen bleiben gruen: Fortsetzungszeile, Verzeichnis mit Schraegstrich, Anfuehrungszeichen.
    let fortgesetzt = MAKE_GUT.replace(
        "all: unit unit-stufe-b tests s02",
        "all: unit \\\n  unit-stufe-b tests s02",
    );
    assert!(
        make_fehler(&fortgesetzt).is_empty(),
        "{:?}",
        make_fehler(&fortgesetzt)
    );
    let schraeg = MAKE_GUT.replace("pytest pipeline/tests -q", "pytest \"pipeline/tests/\" -q");
    assert!(
        make_fehler(&schraeg).is_empty(),
        "{:?}",
        make_fehler(&schraeg)
    );
}

const CI_GUT: &str = r"name: CI
jobs:
  anderer:
    steps:
      - run: python3 -m pytest tests/ -q
  stufe-b:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Python-Abhaengigkeiten
        run: pip install -r requirements-ci.txt
      - name: pytest pipeline/tests
        run: python3 -m pytest pipeline/tests -q
";

#[test]
fn die_job_pruefung_schlaegt_an() {
    assert!(job_fehler(CI_GUT).is_empty(), "{:?}", job_fehler(CI_GUT));
    let faelle: &[(&str, &str, &str)] = &[
        ("Job umbenannt", "  stufe-b:\n", "  stufe-bb:\n"),
        ("Einzeldatei statt Verzeichnis", "pytest pipeline/tests -q", "pytest pipeline/tests/test_quellen.py -q"),
        ("tests/ statt pipeline/tests", "pytest pipeline/tests -q", "pytest tests/ -q"),
        ("pytest nur im Namen", "run: python3 -m pytest pipeline/tests -q", "run: echo pipeline/tests"),
        ("Installation ohne Manifest", "pip install -r requirements-ci.txt", "pip install pytest pyyaml"),
        ("Manifest nur im Namen", "pip install -r requirements-ci.txt", "pip install pytest # requirements-ci.txt"),
        ("keine Schritte", "    steps:\n      - uses: actions/checkout@v4\n      - name: Python-Abhaengigkeiten\n        run: pip install -r requirements-ci.txt\n      - name: pytest pipeline/tests\n        run: python3 -m pytest pipeline/tests -q\n", ""),
    ];
    for (was, alt, neu) in faelle {
        let text = CI_GUT.replace(alt, neu);
        assert_ne!(text, CI_GUT, "{was}: der Mutant aendert nichts");
        assert!(!job_fehler(&text).is_empty(), "{was}: nicht erkannt");
    }
}

/// Ein Baum im Temp-Verzeichnis; `None` als Inhalt gibt es nicht, jede Datei hat Text.
fn baum(dateien: &[(&str, &str)]) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static ZAEHLER: AtomicUsize = AtomicUsize::new(0);
    let wurzel = std::env::temp_dir().join(format!(
        "ci-stufe-b-{}-{}",
        std::process::id(),
        ZAEHLER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&wurzel);
    for (pfad, inhalt) in dateien {
        let p = wurzel.join(pfad);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, inhalt).unwrap();
    }
    wurzel
}

const TEST_A: &str = r#""""Doc.
import imdoc
from ghost import x
"""
from __future__ import annotations

import os
import sys, json as j
import yaml
import client
from pipeline.ui import service
from . import rel
from .rel import y
import pytest

try:
    import nuruntry
except ImportError:
    pass

if True:
    import nureingerueckt


def f():
    import nurinfunktion


'''Mehr
import imdoc2
'''
pytest.importorskip("gettsim")
import dahinter
"#;

/// Welche Importe zaehlen: genau die in Spalte 0 vor `importorskip`, ausserhalb von Docstrings.
#[test]
fn die_importe_werden_nach_den_regeln_gelesen() {
    let namen = modulebene_importe(TEST_A);
    assert_eq!(
        namen,
        [
            "__future__",
            "os",
            "sys",
            "json",
            "yaml",
            "client",
            "pipeline",
            "pytest"
        ]
    );
    assert_eq!(
        modulebene_importe("import a.b.c as d, e;\nfrom f.g import (\n    h,\n)\n"),
        ["a", "e", "f"]
    );
    assert_eq!(modulebene_importe("important = 1\nimport_x = 2\n# import kommentar\n").len(), 0);
}

fn tree_a() -> Vec<(&'static str, &'static str)> {
    vec![
        ("pipeline/tests/test_a.py", TEST_A),
        ("pipeline/client.py", "import requests\nimport os\n"),
        ("pipeline/ui/service.py", "import fastapi\n"),
        // Verdeckt nichts: versteckter Ordner, Bauverzeichnis und eine Datei ausserhalb der Huelle.
        (".versteckt/requests.py", ""),
        ("rust/target-x/requests/m.py", ""),
        ("pipeline/ausserhalb.py", "import numpy\n"),
    ]
}

#[test]
fn der_waechter_meldet_ein_fehlendes_manifestpaket_und_nur_das() {
    let wurzel = baum(&tree_a());
    let ohne = analysiere(
        &wurzel,
        "pipeline/tests",
        "pytest>=8.0\npyyaml>=6.0 # Kommentar\n",
    );
    let ganz = analysiere(
        &wurzel,
        "pipeline/tests",
        "pytest>=8.0\npyyaml>=6.0\nRequests[socks]>=2.31\n",
    );
    let fremd: Vec<&str> = ohne.fremd.keys().map(String::as_str).collect();
    let _ = std::fs::remove_dir_all(&wurzel);
    // Fremd sind genau pytest, requests (ueber das eigene Modul `client`) und yaml; fastapi liegt hinter einem
    // Punktpfad, numpy ausserhalb der Huelle, `nuruntry` und die anderen stehen nicht in Spalte 0.
    assert_eq!(fremd, ["pytest", "requests", "yaml"]);
    assert_eq!(ohne.fehlend, ["requests (zuerst in pipeline/client.py)"]);
    assert!(ganz.fehlend.is_empty(), "{:?}", ganz.fehlend);
    assert_eq!(ohne.dateien, 2, "test_a.py und client.py");
}

#[test]
fn jedes_fehlende_paket_wird_einzeln_gemeldet() {
    let wurzel = baum(&tree_a());
    let nichts = analysiere(&wurzel, "pipeline/tests", "");
    let alias = analysiere(&wurzel, "pipeline/tests", "pytest\nPyYAML\nrequests\n");
    let _ = std::fs::remove_dir_all(&wurzel);
    assert_eq!(
        nichts.fehlend,
        [
            "pytest (zuerst in pipeline/tests/test_a.py)",
            "requests (zuerst in pipeline/client.py)",
            "yaml (zuerst in pipeline/tests/test_a.py)"
        ]
    );
    assert!(
        alias.fehlend.is_empty(),
        "yaml ist das Paket PyYAML: {:?}",
        alias.fehlend
    );
}

#[test]
fn ein_eigenes_modul_gleichen_namens_verdeckt_das_paket_und_wird_gemeldet() {
    let mut dateien = tree_a();
    dateien.push(("tools/requests.py", ""));
    dateien.push(("tools/yaml/x.py", ""));
    let wurzel = baum(&dateien);
    let manifest = "pytest\npyyaml\nrequests\nbcrypt\n";
    let aus = verdeckt(&wurzel, manifest);
    let sauber = baum(&tree_a());
    let ohne_verdeckung = verdeckt(&sauber, manifest);
    let _ = std::fs::remove_dir_all(&wurzel);
    let _ = std::fs::remove_dir_all(&sauber);
    assert_eq!(aus.len(), 2, "{aus:?}");
    assert!(aus.iter().any(|z| z.contains("`requests`")), "{aus:?}");
    assert!(
        aus.iter()
            .any(|z| z.contains("`pyyaml`") && z.contains("`yaml`")),
        "{aus:?}"
    );
    assert!(ohne_verdeckung.is_empty(), "{ohne_verdeckung:?}");
}

#[test]
fn das_manifest_wird_ohne_version_und_kommentar_gelesen() {
    let p = pakete("# Kopf\npytest>=8.0\n\nPyYAML == 6 # Kommentar\npytest-xdist>=3.5\nRequests[socks]~=2\nPillow\n");
    let soll: BTreeSet<String> = ["pytest", "pyyaml", "pytest-xdist", "requests", "pillow"]
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(p, soll);
}

// ---------------------------------------------------------------- Stdlib

/// `sys.stdlib_module_names`, Vereinigung aus Python 3.12 (CI) und 3.14 (lokal), 320 Namen.
#[rustfmt::skip]
const STDLIB: &[&str] = &[
    "__future__", "_abc", "_aix_support", "_android_support", "_apple_support", "_ast", "_ast_unparse", "_asyncio",
    "_bisect", "_blake2", "_bz2", "_codecs", "_codecs_cn", "_codecs_hk", "_codecs_iso2022", "_codecs_jp",
    "_codecs_kr", "_codecs_tw", "_collections", "_collections_abc", "_colorize", "_compat_pickle", "_compression",
    "_contextvars", "_crypt", "_csv", "_ctypes", "_curses", "_curses_panel", "_datetime", "_dbm", "_decimal",
    "_elementtree", "_frozen_importlib", "_frozen_importlib_external", "_functools", "_gdbm", "_hashlib", "_heapq",
    "_hmac", "_imp", "_interpchannels", "_interpqueues", "_interpreters", "_io", "_ios_support", "_json", "_locale",
    "_lsprof", "_lzma", "_markupbase", "_md5", "_msi", "_multibytecodec", "_multiprocessing", "_opcode",
    "_opcode_metadata", "_operator", "_osx_support", "_overlapped", "_pickle", "_posixshmem", "_posixsubprocess",
    "_py_abc", "_py_warnings", "_pydatetime", "_pydecimal", "_pyio", "_pylong", "_pyrepl", "_queue", "_random",
    "_remote_debugging", "_scproxy", "_sha1", "_sha2", "_sha3", "_signal", "_sitebuiltins", "_socket", "_sqlite3",
    "_sre", "_ssl", "_stat", "_statistics", "_string", "_strptime", "_struct", "_suggestions", "_symtable",
    "_sysconfig", "_thread", "_threading_local", "_tkinter", "_tokenize", "_tracemalloc", "_types", "_typing",
    "_uuid", "_warnings", "_weakref", "_weakrefset", "_winapi", "_wmi", "_zoneinfo", "_zstd", "abc", "aifc",
    "annotationlib", "antigravity", "argparse", "array", "ast", "asyncio", "atexit", "audioop", "base64", "bdb",
    "binascii", "bisect", "builtins", "bz2", "cProfile", "calendar", "cgi", "cgitb", "chunk", "cmath", "cmd", "code",
    "codecs", "codeop", "collections", "colorsys", "compileall", "compression", "concurrent", "configparser",
    "contextlib", "contextvars", "copy", "copyreg", "crypt", "csv", "ctypes", "curses", "dataclasses", "datetime",
    "dbm", "decimal", "difflib", "dis", "doctest", "email", "encodings", "ensurepip", "enum", "errno", "faulthandler",
    "fcntl", "filecmp", "fileinput", "fnmatch", "fractions", "ftplib", "functools", "gc", "genericpath", "getopt",
    "getpass", "gettext", "glob", "graphlib", "grp", "gzip", "hashlib", "heapq", "hmac", "html", "http", "idlelib",
    "imaplib", "imghdr", "importlib", "inspect", "io", "ipaddress", "itertools", "json", "keyword", "lib2to3",
    "linecache", "locale", "logging", "lzma", "mailbox", "mailcap", "marshal", "math", "mimetypes", "mmap",
    "modulefinder", "msilib", "msvcrt", "multiprocessing", "netrc", "nis", "nntplib", "nt", "ntpath", "nturl2path",
    "numbers", "opcode", "operator", "optparse", "os", "ossaudiodev", "pathlib", "pdb", "pickle", "pickletools",
    "pipes", "pkgutil", "platform", "plistlib", "poplib", "posix", "posixpath", "pprint", "profile", "pstats", "pty",
    "pwd", "py_compile", "pyclbr", "pydoc", "pydoc_data", "pyexpat", "queue", "quopri", "random", "re", "readline",
    "reprlib", "resource", "rlcompleter", "runpy", "sched", "secrets", "select", "selectors", "shelve", "shlex",
    "shutil", "signal", "site", "smtplib", "sndhdr", "socket", "socketserver", "spwd", "sqlite3", "sre_compile",
    "sre_constants", "sre_parse", "ssl", "stat", "statistics", "string", "stringprep", "struct", "subprocess",
    "sunau", "symtable", "sys", "sysconfig", "syslog", "tabnanny", "tarfile", "telnetlib", "tempfile", "termios",
    "textwrap", "this", "threading", "time", "timeit", "tkinter", "token", "tokenize", "tomllib", "trace",
    "traceback", "tracemalloc", "tty", "turtle", "turtledemo", "types", "typing", "unicodedata", "unittest", "urllib",
    "uu", "uuid", "venv", "warnings", "wave", "weakref", "webbrowser", "winreg", "winsound", "wsgiref", "xdrlib",
    "xml", "xmlrpc", "zipapp", "zipfile", "zipimport", "zlib", "zoneinfo",
];
