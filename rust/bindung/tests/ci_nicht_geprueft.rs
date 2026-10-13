//! Rote Probe zu Ticket `gruenes-ci-belegt-xsd-checkest-und-parity-nicht` (Entwurf von q-tg-b, 2026-10-13).
//! Heute ROT: es gibt weder `scripts/ci-nicht-geprueft.sh` noch einen Schritt im Job `rust`, der es aufruft,
//! und 8 Verzichtsmeldungen der CI-Schalter stehen in `eprintln!`, das libtest in bestandenen Tests einfaengt.
//! Ablage im Bau: `rust/bindung/tests/ci_nicht_geprueft.rs` (neben `ci_konfiguration.rs`, `ci_pins.rs`); die zwei Fixtures unter
//! `rust/bindung/tests/fixtures/`. Neue Testdatei braucht eine Zeile in `rust/TESTMAP.tsv`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn skript() -> PathBuf {
    wurzel().join("scripts/ci-nicht-geprueft.sh")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn lauf(log: &Path) -> Output {
    Command::new("bash")
        .arg(skript())
        .arg(log)
        .env_remove("GITHUB_STEP_SUMMARY")
        .output()
        .unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn das_skript_zaehlt_was_der_lauf_nicht_geprueft_hat() {
    // Positivkontrolle: ein Log ohne Verzicht ergibt viermal null und Exit 0 (die Zaehlung erfindet nichts).
    let leer = lauf(&fixture("ci_lauf_alles_geprueft.log"));
    assert!(skript().is_file(), "scripts/ci-nicht-geprueft.sh fehlt");
    assert_eq!(leer.status.code(), Some(0), "Kontrolle: {}", String::from_utf8_lossy(&leer.stderr));
    assert!(
        out(&leer).contains("Schema-Pruefung 0x uebersprungen, Browser-Tests 0x uebersprungen, ERiC/checkESt 0x ignoriert, sonst 0x ignoriert"),
        "{}",
        out(&leer)
    );
    // Ziel: 3 Zeilen der Hilfsfunktion + 1 eigene Wache = 4 Schema, 2 Browser, 3 ERiC-ignoriert, 1 sonst ignoriert.
    let o = lauf(&fixture("ci_lauf_nicht_geprueft.log"));
    assert_eq!(o.status.code(), Some(0));
    assert!(
        out(&o).contains("NICHT GEPRUEFT: Schema-Pruefung 4x uebersprungen, Browser-Tests 2x uebersprungen, ERiC/checkESt 3x ignoriert, sonst 1x ignoriert"),
        "{}",
        out(&o)
    );
    assert!(out(&o).contains("::warning title=Nicht geprueft::"), "{}", out(&o));
    // Die Tabelle fuer die Job-Zusammenfassung: gleiche Zahlen, eine Zeile je Art.
    let tmp = std::env::temp_dir().join(format!("ci-nicht-geprueft-summary-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let m = Command::new("bash")
        .arg(skript())
        .arg(fixture("ci_lauf_nicht_geprueft.log"))
        .env("GITHUB_STEP_SUMMARY", &tmp)
        .output()
        .unwrap();
    let tabelle = std::fs::read_to_string(&tmp).unwrap_or_default();
    let _ = std::fs::remove_file(&tmp);
    assert_eq!(m.status.code(), Some(0));
    for zeile in [
        "| XML gegen das amtliche Schema (XSD) uebersprungen | 4 |",
        "| Browser-Tests uebersprungen | 2 |",
        "| Tests gegen ERiC und checkESt (`#[ignore]`) | 3 |",
        "| weitere `#[ignore]`-Tests | 1 |",
    ] {
        assert!(tabelle.contains(zeile), "Zusammenfassung ohne {zeile:?}: {tabelle}");
    }
}

#[test]
fn ein_leeres_log_ist_kein_gruen() {
    // Kontrolle zuerst: das Skript gibt es und kann Exit 0 (sonst waere jeder Exit-Code-Test unten trivial erfuellt).
    assert_eq!(lauf(&fixture("ci_lauf_alles_geprueft.log")).status.code(), Some(0), "Kontrolle: das Skript fehlt oder ist kaputt");
    let tmp = std::env::temp_dir().join(format!("ci-nicht-geprueft-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let leer = tmp.join("leer.log");
    std::fs::write(&leer, "").unwrap();
    let ohne_ergebnis = tmp.join("ohne.log");
    std::fs::write(&ohne_ergebnis, "running 0 tests\n").unwrap();
    let codes: Vec<_> = [&leer, &ohne_ergebnis, &tmp.join("fehlt.log")].iter().map(|l| lauf(l).status.code()).collect();
    let _ = std::fs::remove_dir_all(&tmp);
    assert_eq!(codes, vec![Some(2); 3], "leeres Log, Log ohne 'test result:', fehlendes Log: Exit 2 je Fall");
}

#[test]
fn das_skript_sucht_die_zeilen_die_der_quelltext_schreibt() {
    // Die Muster im Skript und die Texte im Quelltext sind dieselbe Zeichenfolge; wer eine Seite umbenennt, zaehlt sonst still null.
    // Geprueft wird der Aufruf `grep -c '<Muster>'`, nicht das Muster allein: der Kommentar im Skript nennt es auch und erfuellte sonst die Pruefung.
    let lies = |p: &str| std::fs::read_to_string(wurzel().join(p)).unwrap_or_else(|e| panic!("{p}: {e}"));
    let s = lies("scripts/ci-nicht-geprueft.sh");
    let paare = [
        ("grep -c 'XSD NICHT geprueft'", "rust/elster/src/testhilfe.rs", "XSD NICHT geprueft"),
        ("grep -c 'UEBERSPRUNGEN: kein Browser'", "rust/api/tests/oberflaeche/browser.rs", "UEBERSPRUNGEN: kein Browser"),
        ("grep -c 'UEBERSPRUNGEN: .*E10-2025.xsd'", "rust/api/tests/oberflaeche/fluss_g.rs", "UEBERSPRUNGEN: E10-2025.xsd fehlt"),
        ("grep -c 'UEBERSPRUNGEN: .*E10-2025.xsd'", "rust/api/tests/einreichen_attrappe_hermetisch.rs", "UEBERSPRUNGEN: E10-2025.xsd fehlt"),
        ("grep -c 'ignored, braucht die ERiC-Bibliothek'", "rust/bescheid/tests/checkest_blockmatrix.rs", "braucht die ERiC-Bibliothek"),
    ];
    for (aufruf, datei, text) in paare {
        assert!(s.contains(aufruf), "das Skript ruft nicht {aufruf}");
        assert!(lies(datei).contains(text), "{datei} schreibt nicht mehr {text:?}");
    }
}

#[test]
fn der_rust_job_ruft_das_skript_nach_dem_cargo_test_auch_bei_rot() {
    let text = std::fs::read_to_string(wurzel().join(".github/workflows/ci.yml")).unwrap();
    let wf: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
    let schritte = wf["jobs"]["rust"]["steps"].as_sequence().expect("Job rust ohne Schritte");
    let lauf_von = |s: &serde_yaml_ng::Value| s["run"].as_str().unwrap_or("").to_owned();
    let i_test = schritte.iter().position(|s| lauf_von(s).contains("cargo test")).expect("kein cargo test");
    assert!(lauf_von(&schritte[i_test]).contains("tee"), "cargo test schreibt sein Log nicht in eine Datei (tee)");
    // Ohne `shell: bash` laeuft der Schritt als `bash -e {0}` OHNE pipefail: dann gaebe `cargo test | tee` den Exit von tee (0)
    // zurueck, und ein rotes cargo test machte den Job gruen. `shell: bash` schaltet pipefail an.
    assert_eq!(schritte[i_test]["shell"].as_str(), Some("bash"), "cargo test | tee braucht `shell: bash` (pipefail)");
    let nach = schritte[i_test + 1..].iter().find(|s| lauf_von(s).contains("scripts/ci-nicht-geprueft.sh"));
    let nach = nach.expect("kein Schritt nach cargo test ruft scripts/ci-nicht-geprueft.sh");
    assert_eq!(nach["if"].as_str(), Some("always()"), "der Schritt laeuft nicht bei rotem cargo test");
    // Das Log, das tee schreibt, ist das Log, das das Skript liest (nur der Dateiname zaehlt: der Test-Schritt steht in `rust/`).
    let nach_wort = |text: &str, wort: &str| text.split_whitespace().skip_while(|w| *w != wort).nth(1).map(|w| Path::new(w).file_name().map(std::borrow::ToOwned::to_owned));
    let geschrieben = nach_wort(&lauf_von(&schritte[i_test]), "tee");
    let gelesen = nach_wort(&lauf_von(nach), "scripts/ci-nicht-geprueft.sh");
    assert!(geschrieben.is_some() && geschrieben == gelesen, "tee schreibt {geschrieben:?}, das Skript liest {gelesen:?}");
}

/// Zeilen (1-basiert), an denen ein `eprintln!` einen Verzicht der CI-Schalter meldet. libtest faengt `eprintln!` in bestandenen
/// Tests ein: die CI saehe die Zeile nie, die Zaehlung im Skript bliebe 0 (so war es bei 24 Browser-Tests).
fn verzichte_per_eprintln(quelle: &str) -> Vec<usize> {
    let aufruf = ["eprint", "ln!("].concat();
    quelle
        .match_indices(&aufruf)
        .filter(|(i, _)| {
            let ende = quelle[*i..].find(");").map_or(quelle.len(), |e| i + e);
            let text = &quelle[*i..ende];
            text.contains("TAXGRAPH_OHNE_XSD") || text.contains("TAXGRAPH_OHNE_BROWSER")
        })
        .map(|(i, _)| quelle[..i].matches('\n').count() + 1)
        .collect()
}

fn sammle(dir: &Path, aus: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            sammle(&p, aus);
        } else if p.extension().is_some_and(|x| x == "rs") {
            aus.push(p);
        }
    }
}

#[test]
fn kein_verzicht_der_ci_schalter_meldet_sich_nur_per_eprintln() {
    // Kontrolle des Suchers: ein eprintln-Verzicht trifft, eine Direktschrift an stderr trifft nicht.
    let schlecht = ["fn t() {\n    ", "eprint", "ln!(\"UEBERSPRUNGEN: kein Browser, TAXGRAPH_OHNE_BROWSER=1\");\n}\n"].concat();
    let gut = "fn t() {\n    let _ = writeln!(std::io::stderr(), \"UEBERSPRUNGEN: kein Browser, TAXGRAPH_OHNE_BROWSER=1\");\n}\n";
    assert_eq!(verzichte_per_eprintln(&schlecht), vec![2], "Kontrolle: der Sucher sieht einen eprintln-Verzicht nicht");
    assert!(verzichte_per_eprintln(gut).is_empty(), "Kontrolle: der Sucher meldet eine Direktschrift");
    // Ernstfall: alle Testdateien unter rust/*/tests.
    let mut dateien = Vec::new();
    for krate in std::fs::read_dir(wurzel().join("rust")).unwrap() {
        let t = krate.unwrap().path().join("tests");
        if t.is_dir() {
            sammle(&t, &mut dateien);
        }
    }
    assert!(dateien.len() > 100, "nur {} Testdateien gefunden; der Sucher liest zu wenig", dateien.len());
    let treffer: Vec<String> = dateien
        .iter()
        .flat_map(|p| {
            let z = verzichte_per_eprintln(&std::fs::read_to_string(p).unwrap());
            let name = p.strip_prefix(wurzel()).unwrap().display().to_string();
            z.into_iter().map(move |n| format!("{name}:{n}"))
        })
        .collect();
    assert!(treffer.is_empty(), "Verzichtsmeldungen per eprintln!, die die CI nie sieht: {treffer:?}");
}
