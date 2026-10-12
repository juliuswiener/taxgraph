//! Waechter des CI-Jobs `catala-orakel` in `.github/workflows/ci.yml`: der Frischetor des Catala-Orakels.
//!
//! `rust/catala-sys/orakel/orakel.txt` haelt erzeugte Eingaben samt Ergebnis des Catala-INTERPRETERS; der Rust-Test
//! `catala-sys/tests/catala_orakel.rs` spielt sie gegen das eingecheckte C-Backend ab. Der Job erzeugt die Datei neu und
//! vergleicht mit dem Stand im Repo: eine Regel- oder Generatoraenderung ohne neu erzeugte Datei macht ihn rot. Er ist der
//! Ersatz der gelaufenen Parity-Suiten und muss die Python-Suite (Loeschstufe L2) ueberleben; deshalb steht er eigenstaendig
//! neben `catala-toolchain`. Dieser Test liest nur die Datei und haelt fest, was den Job wirkungslos machen wuerde:
//!
//! - der Job existiert, ist nicht mit `continue-on-error` entschaerft und hat kein `if` (ein uebersprungener Job ist nicht rot);
//! - genau ein Schritt ruft `orakel_erzeugen` auf, ohne `if`, ohne `continue-on-error`, mit `set -e`, ohne `|| true`, ohne `exit`;
//! - in diesem Schritt stehen drei Zeilen wortgleich, unmittelbar hintereinander und am Ende des Schritts: `git ls-files
//!   --error-unmatch` auf die Datei (`git diff` meldet fuer eine Datei ausserhalb des Index nichts, das waere ein stilles
//!   Gruen), der Aufruf des Generators, `git diff --exit-code` auf dieselbe Datei (eine Zeile dazwischen, etwa `git checkout`
//!   der Datei, machte den Vergleich wirkungslos); dazu `rustup default 1.99.0` (gleicher Pin wie Job `rust`);
//! - ein Schritt installiert `catala.1.2.1`;
//! - kein Schritt des Jobs braucht Python: kein `pip`, kein `clerk build`, kein `assemble_catala`, kein `pytest`, kein
//!   `requirements-ci`, kein `actions/setup-python`. Gemessen im frischen Worktree ohne `_build/` und
//!   `oracle/gettsim/_catala` mit Attrappen statt python/pip: kein Aufruf, Erzeugen und Diff rc=0.
//!
//! Grenzen: Ob ein Schritt wirklich laeuft, ob der Runner `bash -e` nimmt (ein `shell:` am Schritt wuerde das aendern, der
//! Waechter sieht es nicht; `set -e` steht im Text), ob der opam-Cache trifft, prueft der Test nicht. Er liest nur die Datei.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

const JOB: &str = "catala-orakel";
const LS_FILES: &str = "git ls-files --error-unmatch rust/catala-sys/orakel/orakel.txt";
const ERZEUGEN: &str = "(cd rust && cargo run -p catala-sys --example orakel_erzeugen)";
const DIFF: &str = "git diff --exit-code -- rust/catala-sys/orakel/orakel.txt";
const VERBOTEN: [&str; 7] = [
    "pip install",
    "pip3",
    "clerk build",
    "assemble_catala",
    "pytest",
    "requirements-ci",
    "python",
];

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn laden(text: &str) -> Value {
    serde_yaml_ng::from_str(text).unwrap()
}

fn workflow() -> Value {
    laden(&std::fs::read_to_string(wurzel().join(".github/workflows/ci.yml")).unwrap())
}

fn run_von(s: &Value) -> &str {
    s.get("run").and_then(Value::as_str).unwrap_or_default()
}

fn name_von(s: &Value) -> &str {
    s.get("name").and_then(Value::as_str).unwrap_or("?")
}

/// `continue-on-error` steht da und ist nicht ausdruecklich `false` (auch ein Ausdruck zaehlt als entschaerft).
fn entschaerft(v: Option<&Value>) -> bool {
    v.is_some_and(|w| w.as_bool() != Some(false))
}

/// Alle Fehler am Job `catala-orakel`; leer = der Frischetor ist wirksam.
fn orakel_job(wf: &Value) -> Vec<String> {
    let Some(job) = wf.get("jobs").and_then(|j| j.get(JOB)) else {
        return vec![format!("der Job `{JOB}` fehlt")];
    };
    let mut aus = Vec::new();
    if entschaerft(job.get("continue-on-error")) {
        aus.push("der Job hat `continue-on-error`: ein roter Frischetor bliebe gruen".to_string());
    }
    if job.get("if").is_some() {
        aus.push("der Job hat ein `if`: er koennte uebersprungen werden (Status skipped, nicht rot)".to_string());
    }
    let Some(schritte) = job.get("steps").and_then(Value::as_sequence) else {
        aus.push("der Job hat keine `steps`".to_string());
        return aus;
    };
    for s in schritte {
        let (run, uses) = (
            run_von(s),
            s.get("uses").and_then(Value::as_str).unwrap_or_default(),
        );
        for v in VERBOTEN.iter().filter(|v| run.contains(**v)) {
            aus.push(format!(
                "Schritt `{}` ruft `{v}` auf: der Job soll ohne Python-Bauschritte laufen",
                name_von(s)
            ));
        }
        if uses.starts_with("actions/setup-python") {
            aus.push(format!("Schritt `{}` richtet Python ein", name_von(s)));
        }
    }
    if !schritte
        .iter()
        .any(|s| run_von(s).contains("opam install -y catala.1.2.1"))
    {
        aus.push("kein Schritt installiert `catala.1.2.1`".to_string());
    }
    let erzeuger: Vec<&Value> = schritte
        .iter()
        .filter(|s| run_von(s).contains("orakel_erzeugen"))
        .collect();
    let [schritt] = erzeuger.as_slice() else {
        aus.push(format!(
            "{} Schritte rufen `orakel_erzeugen` auf statt einem",
            erzeuger.len()
        ));
        return aus;
    };
    aus.extend(frischetor_schritt(schritt));
    aus
}

fn frischetor_schritt(schritt: &Value) -> Vec<String> {
    let mut aus = Vec::new();
    let run = run_von(schritt);
    if schritt.get("if").is_some() {
        aus.push("der Frischetor-Schritt hat ein `if`: er koennte uebersprungen werden".to_string());
    }
    if entschaerft(schritt.get("continue-on-error")) {
        aus.push("der Frischetor-Schritt hat `continue-on-error`".to_string());
    }
    if !run.contains("set -e") {
        aus.push("der Frischetor-Schritt hat kein `set -e`".to_string());
    }
    if run.contains("|| true") || run.contains("set +e") {
        aus.push("der Frischetor-Schritt schluckt Fehler (`|| true` oder `set +e`)".to_string());
    }
    let zeilen: Vec<&str> = run.lines().map(str::trim).collect();
    if zeilen.iter().any(|l| l.starts_with("exit")) {
        aus.push("der Frischetor-Schritt beendet sich vorzeitig (`exit`)".to_string());
    }
    let pos = |z: &str| zeilen.iter().position(|l| *l == z);
    let letzte = zeilen.iter().rposition(|l| !l.is_empty());
    match (pos(LS_FILES), pos(ERZEUGEN), pos(DIFF)) {
        (Some(a), Some(b), Some(c)) if b == a + 1 && c == b + 1 && Some(c) == letzte => {}
        (a, b, c) => aus.push(format!(
            "die Zeilen `{LS_FILES}`, `{ERZEUGEN}`, `{DIFF}` stehen nicht wortgleich, unmittelbar hintereinander und am Ende des Schritts (Zeilen {a:?}, {b:?}, {c:?}, letzte {letzte:?})"
        )),
    }
    if !zeilen.contains(&"rustup default 1.99.0") {
        aus.push("der Schritt legt Rust nicht auf `1.99.0` fest".to_string());
    }
    aus
}

#[test]
fn der_job_catala_orakel_haelt_den_frischetor_wirksam() {
    let fehler = orakel_job(&workflow());
    assert!(fehler.is_empty(), "Frischetor:\n  {}", fehler.join("\n  "));
}

/// Ein Waechter, der nichts findet, ist gruen: im echten Workflow liegt der Job mit seinem Generator-Schritt.
#[test]
fn der_waechter_sieht_den_job_im_echten_workflow() {
    let wf = workflow();
    let schritte = wf["jobs"][JOB]["steps"].as_sequence().unwrap();
    assert!(schritte.len() >= 4, "{} Schritte", schritte.len());
    assert_eq!(
        schritte
            .iter()
            .filter(|s| run_von(s).contains("orakel_erzeugen"))
            .count(),
        1
    );
}

const GUT: &str = r"name: CI
jobs:
  catala-orakel:
    runs-on: ubuntu-latest
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - name: Catala installieren
        run: |
          set -e
          opam init --bare -y || true
          opam install -y catala.1.2.1
      - name: Catala-Orakel neu erzeugen
        run: |
          set -e
          rustup toolchain install 1.99.0 --profile minimal
          rustup default 1.99.0
          git ls-files --error-unmatch rust/catala-sys/orakel/orakel.txt
          (cd rust && cargo run -p catala-sys --example orakel_erzeugen)
          git diff --exit-code -- rust/catala-sys/orakel/orakel.txt
";

/// Jeder Rueckfall wird erkannt, die gute Fassung nicht. In der guten Fassung steht ein `|| true` im Schritt
/// `opam init`: das ist erlaubt, nur der Generator-Schritt darf keines tragen.
#[test]
fn die_pruefung_schlaegt_an() {
    assert!(orakel_job(&laden(GUT)).is_empty(), "{:?}", orakel_job(&laden(GUT)));

    let faelle: &[(&str, &str, &str)] = &[
        ("Job umbenannt", "  catala-orakel:\n", "  catala-orakel-x:\n"),
        ("Job ohne steps", "    steps:\n", "    schritte:\n"),
        ("Job continue-on-error", "    timeout-minutes: 45\n", "    timeout-minutes: 45\n    continue-on-error: true\n"),
        ("Diff-Zeile fehlt", "          git diff --exit-code -- rust/catala-sys/orakel/orakel.txt\n", ""),
        ("Diff mit || true", "--exit-code -- rust/catala-sys/orakel/orakel.txt\n", "--exit-code -- rust/catala-sys/orakel/orakel.txt || true\n"),
        ("Diff auf andere Datei", "git diff --exit-code -- rust/catala-sys/orakel/orakel.txt", "git diff --exit-code -- rust/catala-sys/orakel/orakel2.txt"),
        ("ls-files fehlt", "          git ls-files --error-unmatch rust/catala-sys/orakel/orakel.txt\n", ""),
        ("Generator ersetzt", "--example orakel_erzeugen", "--example anderes_beispiel"),
        ("Schritt continue-on-error", "      - name: Catala-Orakel neu erzeugen\n", "      - name: Catala-Orakel neu erzeugen\n        continue-on-error: true\n"),
        ("Schritt mit if false", "      - name: Catala-Orakel neu erzeugen\n", "      - name: Catala-Orakel neu erzeugen\n        if: false\n"),
        ("set -e fehlt", "          set -e\n          rustup", "          rustup"),
        ("set +e", "          set -e\n          rustup", "          set +e\n          rustup"),
        ("Rust nicht gepinnt", "rustup default 1.99.0", "rustup default 1.98.0"),
        ("Diff vor dem Generator", "          (cd rust && cargo run -p catala-sys --example orakel_erzeugen)\n          git diff --exit-code -- rust/catala-sys/orakel/orakel.txt\n", "          git diff --exit-code -- rust/catala-sys/orakel/orakel.txt\n          (cd rust && cargo run -p catala-sys --example orakel_erzeugen)\n"),
        ("Catala nicht installiert", "          opam install -y catala.1.2.1\n", ""),
        ("pip install", "      - uses: actions/checkout@v4\n", "      - uses: actions/checkout@v4\n      - run: pip install -r requirements-ci.txt\n"),
        ("clerk build", "          opam install -y catala.1.2.1\n", "          opam install -y catala.1.2.1\n          clerk build p32a-python\n"),
        ("assemble_catala", "          opam install -y catala.1.2.1\n", "          opam install -y catala.1.2.1\n          bash oracle/gettsim/assemble_catala.sh\n"),
        ("pytest", "      - uses: actions/checkout@v4\n", "      - uses: actions/checkout@v4\n      - run: python3 -m pytest tests/ -q\n"),
        ("setup-python", "      - uses: actions/checkout@v4\n", "      - uses: actions/checkout@v4\n      - uses: actions/setup-python@v5\n"),
        ("zweiter Generator-Schritt", "      - uses: actions/checkout@v4\n", "      - uses: actions/checkout@v4\n      - run: cargo run --example orakel_erzeugen\n"),
        ("Job mit if", "    timeout-minutes: 45\n", "    timeout-minutes: 45\n    if: false\n"),
        ("exit 0 zwischen Generator und Diff", "          git diff --exit-code -- rust/catala-sys/orakel/orakel.txt\n", "          exit 0\n          git diff --exit-code -- rust/catala-sys/orakel/orakel.txt\n"),
        ("git checkout der Datei vor dem Diff", "          git diff --exit-code -- rust/catala-sys/orakel/orakel.txt\n", "          git checkout -- rust/catala-sys/orakel/orakel.txt\n          git diff --exit-code -- rust/catala-sys/orakel/orakel.txt\n"),
        ("exit 0 am Anfang des Schritts", "          set -e\n          rustup", "          set -e\n          exit 0\n          rustup"),
        ("Zeile hinter dem Diff", "--exit-code -- rust/catala-sys/orakel/orakel.txt\n", "--exit-code -- rust/catala-sys/orakel/orakel.txt\n          echo fertig\n"),
    ];
    for (was, alt, neu) in faelle {
        let text = GUT.replace(alt, neu);
        assert_ne!(text, GUT, "{was}: der Mutant aendert nichts");
        assert!(!orakel_job(&laden(&text)).is_empty(), "{was}: nicht erkannt");
    }
}
