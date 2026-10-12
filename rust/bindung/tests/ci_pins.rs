//! Waechter der Pins, die ein CI-Lauf braucht: die Catala-Version in `.github/workflows/ci.yml` gegen den Cache-Schluessel,
//! und die GETTSIM-Pins in `requirements-oracle.txt`. Ersatz von vier Tests aus `tests/test_ci_konfiguration.py`
//! (`test_catala_version_ist_festgenagelt_und_passt_zum_cache_schluessel`, `test_gettsim_ist_exakt_festgenagelt`,
//! `test_kein_installationsweg_nennt_eine_eigene_oracle_version`, `test_gepinnte_gettsim_version_ist_die_installierbare`),
//! damit sie die Loeschstufe L2 (Python-Tests) ueberleben. Jede Pruefung ist eine reine Funktion ueber den Text der Datei;
//! `die_catala_pruefung_schlaegt_an`, `die_pin_pruefung_schlaegt_an` und `das_muster_fuer_fremde_pins_trifft_nur_versionen`
//! fuettern sie mit Fassungen, in denen genau der Rueckfall steckt.
//!
//! - Catala: jeder Job, der `opam install -y catala` aufruft, nennt eine feste Version (`catala.1.2.1`), er hat einen Cache-Schritt
//!   mit einem Schluessel `opam-taxgraph-catala-<Version>-ocaml-...` derselben Version, und im ganzen Workflow gibt es nur eine
//!   Version. Ohne Version holte `opam` die jeweils neueste, waehrend der Schluessel eine bestimmte behauptet; bei einem
//!   Cache-Treffer lief dann eine ANDERE Version, als der Schluessel nennt (gemessen 2026-08-18: Schluessel 1.2.0, lokal 1.2.1).
//!   Schaerfer als Python: das Paar aus Install und Schluessel gilt je Job, und ein Job mit Install ohne Cache-Schritt ist rot.
//! - GETTSIM: `gettsim` und `ttsim-backend` stehen in `requirements-oracle.txt` je genau einmal und mit `==`. Eine offene Grenze
//!   (`>=`) verschiebt die Vergleichsbasis still; `ttsim-backend 1.3.0` machte so am 2026-08-20 den Crosscheck ohne Aenderung
//!   bei uns rot.
//! - Kein anderer Installationsweg (`ci.yml`, `requirements-ci.txt`, `Makefile`, `docs/setup.md`, `scripts/*.sh`) nennt eine
//!   eigene Version fuer `gettsim` oder `ttsim-backend`: zwei Pins an zwei Orten laufen auseinander.
//! - Der Test `die_gepinnte_version_ist_die_installierbare` ist `#[ignore]`: er liest die Metadaten aus `oracle/.venv312` und
//!   braucht das venv (lokal, nach `docs/setup.md`). Das Python-Gegenstueck uebersprang sich ohne venv; hier ist ein fehlendes
//!   venv unter `--ignored` ein roter Test, kein stilles Gruen.
//!
//! Grenzen: Die Waechter lesen nur Dateien. Ob `opam` die Version wirklich installiert, ob der Cache trifft, ob `uv` den Pin
//! findet, zeigen erst die CI-Jobs selbst. Eine Version aus einer Variable (`gettsim==$V`) sieht das Muster fuer fremde Pins
//! nicht. `docs/setup.md` und `scripts/*.sh` sind optional (L5 kann sie loeschen); `ci.yml`, `requirements-ci.txt` und
//! `Makefile` muessen da sein.
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
use serde_yaml_ng::Value;

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn lesen(rel: &str) -> String {
    std::fs::read_to_string(wurzel().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn laden(text: &str) -> Value {
    serde_yaml_ng::from_str(text).unwrap()
}

fn run_von(s: &Value) -> &str {
    s.get("run").and_then(Value::as_str).unwrap_or_default()
}

// ---------------------------------------------------------------- Catala

/// Alle Fehler der Catala-Festlegung im Workflow und die Zahl der Jobs, die Catala installieren; leer = jede Version fest und
/// zum Cache-Schluessel passend. Null Jobs ist selbst ein Fehler: ein Waechter, der nichts liest, ist gruen.
fn catala_pruefung(wf: &Value) -> (Vec<String>, usize) {
    let install = Regex::new(r"opam install -y catala(?:\.([0-9.]+))?").unwrap();
    let schluessel_re = Regex::new(r"opam-taxgraph-catala-([0-9.]+)-ocaml").unwrap();
    let Some(jobs) = wf.get("jobs").and_then(Value::as_mapping) else {
        return (vec!["der Workflow hat keine `jobs`".to_string()], 0);
    };
    let mut fehler = Vec::new();
    let (mut alle_versionen, mut alle_schluessel) = (BTreeSet::new(), BTreeSet::new());
    let mut jobs_mit_catala = 0;
    for (name, job) in jobs {
        let name = name.as_str().unwrap_or("?");
        let (mut installiert, mut schluessel, mut hat_install) = (BTreeSet::new(), BTreeSet::new(), false);
        for s in job.get("steps").and_then(Value::as_sequence).into_iter().flatten() {
            for t in install.captures_iter(run_von(s)) {
                hat_install = true;
                match t.get(1) {
                    Some(v) => {
                        installiert.insert(v.as_str().to_string());
                    }
                    None => fehler.push(format!(
                        "Job `{name}`: `opam install -y catala` ohne Version holt die jeweils neueste, unabhaengig vom Cache-Schluessel"
                    )),
                }
            }
            if s.get("uses").and_then(Value::as_str).is_some_and(|u| u.starts_with("actions/cache")) {
                let key = s.get("with").and_then(|w| w.get("key")).and_then(Value::as_str).unwrap_or_default();
                schluessel.extend(schluessel_re.captures_iter(key).map(|t| t[1].to_string()));
            }
        }
        if hat_install {
            jobs_mit_catala += 1;
            if installiert.len() > 1 {
                fehler.push(format!("Job `{name}`: verschiedene Catala-Versionen {installiert:?}"));
            }
            if !installiert.is_empty() && schluessel != installiert {
                fehler.push(format!(
                    "Job `{name}`: Cache-Schluessel nennt {schluessel:?}, installiert wird {installiert:?} (bei einem Cache-Treffer laeuft eine andere Version als die, die der Schluessel nennt)"
                ));
            }
        }
        alle_versionen.extend(installiert);
        alle_schluessel.extend(schluessel);
    }
    if jobs_mit_catala == 0 {
        fehler.push("kein Job installiert Catala (`opam install -y catala`): der Waechter liest nichts".to_string());
    }
    if alle_versionen.len() > 1 {
        fehler.push(format!("verschiedene Catala-Versionen im Workflow: {alle_versionen:?}"));
    }
    if !alle_versionen.is_empty() && alle_schluessel != alle_versionen {
        fehler.push(format!(
            "Cache-Schluessel im Workflow nennen {alle_schluessel:?}, installiert wird {alle_versionen:?}"
        ));
    }
    (fehler, jobs_mit_catala)
}

#[test]
fn die_catala_version_ist_festgenagelt_und_passt_zum_cache_schluessel() {
    let (fehler, _) = catala_pruefung(&laden(&lesen(".github/workflows/ci.yml")));
    assert!(fehler.is_empty(), "Catala-Pin:\n  {}", fehler.join("\n  "));
}

// ---------------------------------------------------------------- GETTSIM

fn ohne_kommentar(z: &str) -> &str {
    z.split('#').next().unwrap_or_default().trim()
}

fn paketname(zeile: &str) -> String {
    zeile.split(['<', '>', '=', '!', '~', '[']).next().unwrap_or_default().trim().to_lowercase()
}

/// Die Version einer Zeile `paket==version`, sonst `None` (offene Grenze, `~=`, Platzhalter, nur der Name).
fn pin_version(zeile: &str, paket: &str) -> Option<String> {
    let re = Regex::new(&format!(r"(?i)^{}\s*==\s*([\w.+!-]+)$", regex::escape(paket))).unwrap();
    re.captures(zeile).map(|t| t[1].to_string())
}

/// Fehler am Pin von `paket` in `requirements-oracle.txt`: nicht da, nicht exakt, oder mehrfach (eine zweite Zeile liesse die
/// erste gelten und die zweite stehen).
fn pin_fehler(text: &str, paket: &str) -> Vec<String> {
    let treffer: Vec<&str> = text.lines().map(ohne_kommentar).filter(|z| paketname(z) == paket).collect();
    match treffer.as_slice() {
        [] => vec![format!("{paket} steht nicht in requirements-oracle.txt")],
        [z] if pin_version(z, paket).is_some() => Vec::new(),
        [z] => vec![format!(
            "{paket} ist nicht exakt festgenagelt: `{z}` (bei einem Oracle ist die Version ein Zahlenwert, kein Ablaufdetail)"
        )],
        mehr => vec![format!("{paket} steht {} Mal in requirements-oracle.txt: {mehr:?}", mehr.len())],
    }
}

#[test]
fn gettsim_und_ttsim_backend_sind_exakt_festgenagelt() {
    let text = lesen("requirements-oracle.txt");
    let fehler: Vec<String> = ["gettsim", "ttsim-backend"].iter().flat_map(|p| pin_fehler(&text, p)).collect();
    assert!(fehler.is_empty(), "GETTSIM-Pin:\n  {}", fehler.join("\n  "));
}

/// Zeilen, die `gettsim` oder `ttsim-backend` mit einem Versions-Spezifizierer nennen (`==`, `>=`, `<`, `~=` …). Die Ziffer
/// danach ist Pflicht: ohne sie traf `echo "=== install gettsim ==="`.
fn neben_pins(name: &str, text: &str) -> Vec<String> {
    let re = Regex::new(r"(?i)\b(gettsim|ttsim[-_.]backend)\s*(===?|[<>!~]=|[<>])\s*\d").unwrap();
    text.lines()
        .enumerate()
        .filter(|(_, z)| re.is_match(z))
        .map(|(n, z)| format!("{name}:{}: {}", n + 1, z.trim()))
        .collect()
}

/// Die Installationswege: `ci.yml`, `requirements-ci.txt` und `Makefile` muessen da sein; `docs/setup.md` und `scripts/*.sh`
/// nur, wenn es sie gibt.
fn installationswege() -> Vec<(String, String)> {
    let mut wege: Vec<(String, String)> = [".github/workflows/ci.yml", "requirements-ci.txt", "Makefile"]
        .iter()
        .map(|rel| ((*rel).to_string(), lesen(rel)))
        .collect();
    if wurzel().join("docs/setup.md").exists() {
        wege.push(("docs/setup.md".to_string(), lesen("docs/setup.md")));
    }
    let mut skripte: Vec<String> = std::fs::read_dir(wurzel().join("scripts"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "sh"))
        .map(|e| format!("scripts/{}", e.file_name().to_string_lossy()))
        .collect();
    skripte.sort();
    wege.extend(skripte.into_iter().map(|rel| {
        let text = lesen(&rel);
        (rel, text)
    }));
    wege
}

#[test]
fn kein_installationsweg_nennt_eine_eigene_oracle_version() {
    let neben: Vec<String> = installationswege().iter().flat_map(|(n, t)| neben_pins(n, t)).collect();
    assert!(
        neben.is_empty(),
        "Version fuer gettsim/ttsim-backend ausserhalb von requirements-oracle.txt; statt eines eigenen Pins `-r requirements-oracle.txt` installieren:\n  {}",
        neben.join("\n  ")
    );
}

/// Ein Waechter, der nichts findet, ist gruen: die Muster sehen im echten Baum, was sie sehen sollen.
#[test]
fn die_waechter_sehen_ueberhaupt_etwas() {
    let (_, jobs) = catala_pruefung(&laden(&lesen(".github/workflows/ci.yml")));
    assert!(jobs >= 1, "kein Job mit `opam install -y catala` im echten Workflow");
    let oracle = lesen("requirements-oracle.txt");
    for paket in ["gettsim", "ttsim-backend"] {
        let zeile = oracle.lines().map(ohne_kommentar).find(|z| paketname(z) == paket).unwrap_or_default();
        assert!(pin_version(zeile, paket).is_some(), "{paket}: kein Pin gelesen");
    }
    // Das Muster fuer fremde Pins trifft den echten Pin selbst (und Kommentare, die einen nennen): mindestens die beiden Zeilen.
    assert!(neben_pins("requirements-oracle.txt", &oracle).len() >= 2, "das Muster sieht die echten Pins nicht");
    let wege = installationswege();
    for rel in [".github/workflows/ci.yml", "requirements-ci.txt", "Makefile"] {
        assert!(wege.iter().any(|(n, t)| n == rel && !t.trim().is_empty()), "{rel} nicht gelesen oder leer");
    }
    // Die optionalen Wege sind nur dann optional, wenn es sie nicht gibt: was auf der Platte liegt, wird gelesen.
    for rel in ["docs/setup.md", "scripts/install-gettsim.sh"] {
        assert_eq!(wurzel().join(rel).exists(), wege.iter().any(|(n, _)| n == rel), "{rel}: Platte und gelesene Wege passen nicht zusammen");
    }
}

// ---------------------------------------------------------------- installierbare Version (lokal)

/// Die gepinnte Zahl muss die Version der Distribution sein, nicht die des Moduls: `gettsim.__version__` sagte 1.2.1, die
/// Metadaten 1.2, und `pip`/`uv` kennen nur die Metadaten (der CI-Lauf scheiterte mit "no version of gettsim==1.2.1"). Gleiches
/// gilt fuer `ttsim-backend`.
#[test]
#[ignore = "braucht oracle/.venv312 (lokal, docs/setup.md): cargo test -p bindung --test ci_pins -- --ignored"]
fn die_gepinnte_version_ist_die_installierbare() {
    let python = wurzel().join("oracle/.venv312/bin/python");
    assert!(python.exists(), "{} fehlt: das venv ist nicht angelegt, der Test kann nichts lesen", python.display());
    let oracle = lesen("requirements-oracle.txt");
    for paket in ["gettsim", "ttsim-backend"] {
        let aus = Command::new(&python)
            .args(["-c", &format!("import importlib.metadata as m; print(m.version({paket:?}))")])
            .output()
            .unwrap();
        assert!(aus.status.success(), "{paket} im venv nicht lesbar: {}", String::from_utf8_lossy(&aus.stderr));
        let installiert = String::from_utf8_lossy(&aus.stdout).trim().to_string();
        let gepinnt = oracle
            .lines()
            .map(ohne_kommentar)
            .filter(|z| paketname(z) == paket)
            .find_map(|z| pin_version(z, paket))
            .unwrap_or_else(|| panic!("{paket} ist nicht exakt gepinnt"));
        assert_eq!(
            gepinnt, installiert,
            "requirements-oracle.txt pinnt {paket}=={gepinnt}, installiert ist laut Metadaten {installiert}: die Zahl stammt vermutlich aus `__version__`"
        );
    }
}

// ---------------------------------------------------------------- Selbsttests

const GUT: &str = r"name: CI
jobs:
  a:
    steps:
      - uses: actions/cache@v4
        with:
          path: ~/.opam
          key: opam-taxgraph-catala-1.2.1-ocaml-4.14.2-${{ runner.os }}
      - name: Catala
        run: |
          set -e
          opam install -y catala.1.2.1
  b:
    steps:
      - uses: actions/cache@v4
        with:
          path: ~/.opam
          key: opam-taxgraph-catala-1.2.1-ocaml-4.14.2-${{ runner.os }}
      - name: Catala
        run: |
          opam install -y catala.1.2.1
          catala --version
  c:
    steps:
      - run: echo kein Catala hier
";

const PINS_GUT: &str = "# Kommentar mit gettsim>=1 darin\ngettsim==1.2\nttsim-backend==1.2.1  # Pin\npyyaml>=6.0\n";

/// Jeder Rueckfall in der Catala-Festlegung wird erkannt, die gute Fassung nicht.
#[test]
fn die_catala_pruefung_schlaegt_an() {
    let (fehler, jobs) = catala_pruefung(&laden(GUT));
    assert!(fehler.is_empty() && jobs == 2, "{fehler:?} / {jobs} Jobs");

    let catala: &[(&str, &str, &str)] = &[
        ("Version fehlt", "opam install -y catala.1.2.1\n  b:", "opam install -y catala\n  b:"),
        ("Version in einem Job anders", "          set -e\n          opam install -y catala.1.2.1", "          set -e\n          opam install -y catala.1.2.0"),
        ("Schluessel in einem Job anders", "catala-1.2.1-ocaml", "catala-1.2.0-ocaml"),
        ("Cache-Schritt eines Jobs fehlt", "      - uses: actions/cache@v4\n        with:\n          path: ~/.opam\n          key: opam-taxgraph-catala-1.2.1-ocaml-4.14.2-${{ runner.os }}\n", ""),
        ("Schluessel ohne Version", "opam-taxgraph-catala-1.2.1-ocaml", "opam-taxgraph-catala-ocaml"),
        ("Schreibweise catala=1.2.1", "opam install -y catala.1.2.1", "opam install -y catala=1.2.1"),
        ("Schluessel anderer Name", "opam-taxgraph-catala-1.2.1-ocaml", "opam-taxgraph-catalax-1.2.1-ocaml"),
        ("zwei Versionen in einem Job", "          catala --version", "          opam install -y catala.1.2.0"),
        ("Cache-Schritt ohne with.key", "          key: opam-taxgraph-catala-1.2.1-ocaml-4.14.2-${{ runner.os }}", "          restore-keys: opam-taxgraph-catala-1.2.1-ocaml"),
    ];
    for (was, alt, neu) in catala {
        let text = GUT.replacen(alt, neu, 1);
        assert_ne!(text, GUT, "{was}: der Mutant aendert nichts");
        assert!(!catala_pruefung(&laden(&text)).0.is_empty(), "{was}: nicht erkannt");
    }
    // Zwei Jobs, zwei Versionen, jeder mit dem passenden Schluessel: je Job gut, im Workflow nicht.
    let zwei = GUT.replacen("catala.1.2.1", "catala.1.2.0", 1).replacen("catala-1.2.1-ocaml", "catala-1.2.0-ocaml", 1);
    assert_ne!(zwei, GUT);
    assert!(!catala_pruefung(&laden(&zwei)).0.is_empty(), "zwei Versionen je mit passendem Schluessel: nicht erkannt");
    // Ein Cache-Schluessel mit anderer Version in einem Job ohne Install.
    let fremd = GUT.replacen(
        "      - run: echo kein Catala hier",
        "      - uses: actions/cache@v4\n        with:\n          key: opam-taxgraph-catala-1.2.0-ocaml-4.14.2-${{ runner.os }}",
        1,
    );
    assert_ne!(fremd, GUT);
    assert!(!catala_pruefung(&laden(&fremd)).0.is_empty(), "fremder Schluessel in einem Job ohne Install: nicht erkannt");

    let ohne_install = GUT.replace("opam install -y catala.1.2.1", "true");
    assert!(!catala_pruefung(&laden(&ohne_install)).0.is_empty(), "kein Install im Workflow: nicht erkannt");
    assert!(!catala_pruefung(&laden("name: CI\n")).0.is_empty(), "Workflow ohne jobs: nicht erkannt");
}

/// Jeder Rueckfall im GETTSIM-Pin wird erkannt, die gute Fassung nicht (auch nicht Gross- und Kleinschreibung oder Leerzeichen).
#[test]
fn die_pin_pruefung_schlaegt_an() {
    for p in ["gettsim", "ttsim-backend"] {
        assert!(pin_fehler(PINS_GUT, p).is_empty(), "{p}: {:?}", pin_fehler(PINS_GUT, p));
    }
    assert!(pin_fehler("GETTSIM == 1.2\nTTSIM_backend==1.2.1\n", "gettsim").is_empty(), "Gross- und Kleinschreibung, Leerzeichen");

    let pins: &[(&str, &str, &str)] = &[
        ("offene Untergrenze", "gettsim==1.2\n", "gettsim>=1.2\n"),
        ("kompatibel", "gettsim==1.2\n", "gettsim~=1.2\n"),
        ("nur der Name", "gettsim==1.2\n", "gettsim\n"),
        ("Platzhalter", "gettsim==1.2\n", "gettsim==1.*\n"),
        ("ttsim-backend offen", "ttsim-backend==1.2.1", "ttsim-backend>=1.2"),
        ("gettsim fehlt", "gettsim==1.2\n", ""),
        ("ttsim-backend fehlt", "ttsim-backend==1.2.1  # Pin\n", ""),
        ("ttsim-backend doppelt", "pyyaml>=6.0\n", "pyyaml>=6.0\nttsim-backend>=1.3\n"),
        ("gettsim mit Extra", "gettsim==1.2\n", "gettsim[all]==1.2\n"),
    ];
    for (was, alt, neu) in pins {
        let text = PINS_GUT.replacen(alt, neu, 1);
        assert_ne!(text, PINS_GUT, "{was}: der Mutant aendert nichts");
        let erkannt = ["gettsim", "ttsim-backend"].iter().any(|p| !pin_fehler(&text, p).is_empty());
        assert!(erkannt, "{was}: nicht erkannt");
    }
}

#[test]
fn das_muster_fuer_fremde_pins_trifft_nur_versionen() {
    for zeile in [
        "pip install gettsim==1.3",
        "uv pip install ttsim-backend<1.3",
        "pip install \"ttsim_backend >= 1\"",
        "  gettsim ~= 1.2",
        "TTSIM.BACKEND==2",
        "echo gettsim>=1",
    ] {
        assert!(!neben_pins("x", zeile).is_empty(), "nicht erkannt: {zeile}");
    }
    for zeile in [
        "echo \"=== install gettsim ===\"",
        "uv pip install --python oracle/.venv312/bin/python -r requirements-oracle.txt",
        "pip install gettsim",
        "gettsim",
        "# ttsim-backend ohne Zahl: ttsim-backend>=x",
        "python oracle/gettsim/golden_crosscheck.py",
    ] {
        assert!(neben_pins("x", zeile).is_empty(), "zu Unrecht erkannt: {zeile}");
    }
    assert_eq!(neben_pins("Datei", "a\nb gettsim==1\n"), vec!["Datei:2: b gettsim==1".to_string()]);
}
