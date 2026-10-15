//! Rote Probe zu Ticket `gruenes-ci-belegt-xsd-checkest-und-parity-nicht` (Entwurf von q-tg-b, 2026-10-13).
//! Heute ROT: es gibt weder `scripts/ci-nicht-geprueft.sh` noch einen Schritt im Job `rust`, der es aufruft,
//! und 8 Verzichtsmeldungen der CI-Schalter stehen in `eprintln!`, das libtest in bestandenen Tests einfaengt.
//! Ablage im Bau: `rust/bindung/tests/ci_nicht_geprueft.rs` (neben `ci_konfiguration.rs`, `ci_pins.rs`); die zwei Fixtures unter
//! `rust/bindung/tests/fixtures/`. Neue Testdatei braucht eine Zeile in `rust/TESTMAP.tsv`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::fmt::Write as _;
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
        .env_remove("CI_NICHT_GEPRUEFT_QUELLEN")
        .output()
        .unwrap()
}

/// Wie [`lauf`], mit den Quelltexten unter `quellen` statt unter `rust/` (nur die nextest-Form liest sie), und mit der
/// Datei, in die das Skript die Tabelle der Job-Zusammenfassung schreibt.
fn lauf_mit(log: &Path, quellen: &Path, zusammenfassung: Option<&Path>) -> Output {
    let mut c = Command::new("bash");
    c.arg(skript()).arg(log).env("CI_NICHT_GEPRUEFT_QUELLEN", quellen);
    match zusammenfassung {
        Some(z) => c.env("GITHUB_STEP_SUMMARY", z),
        None => c.env_remove("GITHUB_STEP_SUMMARY"),
    };
    c.output().unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// Das Attribut eines ERiC-Tests, wie es in den Quelltexten steht. Zusammengesetzt, damit DIESE Datei den Wortlaut nicht
/// enthaelt: das Skript zaehlt ihn unter `rust/` und fande sonst die Zeilen dieser Tests mit.
fn eric_attribut(rest: &str) -> String {
    ["#[ig", "nore = \"braucht die ERiC-Bibliothek", rest, "\"]"].concat()
}

/// Ein Wegwerf-Ordner mit Quelltexten: `n_eric` ERiC-Attribute und ein #[ignore] anderer Art in `a.rs`, ein weiteres ERiC-Attribut
/// in `notiz.txt` (zaehlt nicht: keine `.rs`-Datei) und eines in `target/b.rs` (zaehlt nicht: Bauordner).
fn quellen_ordner(name: &str, n_eric: usize) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ci-nicht-geprueft-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("target")).unwrap();
    let mut a = String::new();
    for i in 0..n_eric {
        let _ = writeln!(a, "{}\nfn t{i}() {{}}", eric_attribut(" und die Hersteller-ID"));
    }
    a.push_str("#[ig");
    a.push_str("nore = \"wartet 301 s: manuell mit --ignored\"]\nfn geparkt() {}\n");
    std::fs::write(dir.join("a.rs"), a).unwrap();
    std::fs::write(dir.join("notiz.txt"), eric_attribut("")).unwrap();
    std::fs::write(dir.join("target/b.rs"), eric_attribut("")).unwrap();
    dir
}

#[test]
fn das_skript_zaehlt_was_der_lauf_nicht_geprueft_hat() {
    // Positivkontrolle: ein Log ohne Verzicht ergibt viermal null und Exit 0 (die Zaehlung erfindet nichts).
    let leer = lauf(&fixture("ci_lauf_alles_geprueft.log"));
    assert!(skript().is_file(), "scripts/ci-nicht-geprueft.sh fehlt");
    assert_eq!(leer.status.code(), Some(0), "Kontrolle: {}", String::from_utf8_lossy(&leer.stderr));
    assert!(
        out(&leer).contains("Schema-Pruefung 0x uebersprungen, Browser-Tests 0x uebersprungen, ERiC/checkESt 0x ignoriert, sonst 0x ignoriert, Verlauf-Pruefung 0x uebersprungen"),
        "{}",
        out(&leer)
    );
    // Ziel: 3 Zeilen der Hilfsfunktion + 1 eigene Wache = 4 Schema, 2 Browser, 3 ERiC-ignoriert, 1 sonst ignoriert, 1 Verlauf.
    let o = lauf(&fixture("ci_lauf_nicht_geprueft.log"));
    assert_eq!(o.status.code(), Some(0));
    assert!(
        out(&o).contains("NICHT GEPRUEFT: Schema-Pruefung 4x uebersprungen, Browser-Tests 2x uebersprungen, ERiC/checkESt 3x ignoriert, sonst 1x ignoriert, Verlauf-Pruefung 1x uebersprungen"),
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
        "| Belege der Testkarte gegen den Git-Verlauf (Hash, Verlaufsform) uebersprungen | 1 |",
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
    // nextest ohne Zusammenfassung (Abbruch beim Bau, Lauf vorzeitig beendet): auch das ist kein Gruen. Die Zeile eines
    // einzelnen Tests, die `test result:` einrueckt, ist keine Zusammenfassung.
    let ohne_summary = tmp.join("ohne-summary.log");
    std::fs::write(&ohne_summary, "    Starting 6 tests across 4 binaries\n    test result: ok. 1 passed; 0 failed; 0 ignored;\n").unwrap();
    let codes: Vec<_> = [&leer, &ohne_ergebnis, &ohne_summary, &tmp.join("fehlt.log")].iter().map(|l| lauf(l).status.code()).collect();
    let _ = std::fs::remove_dir_all(&tmp);
    assert_eq!(codes, vec![Some(2); 4], "leeres Log, Log ohne 'test result:', nextest-Log ohne 'Summary', fehlendes Log: Exit 2 je Fall");
}

/// Der Lauf der CI seit 2026-10-15: `cargo nextest run --profile ci` plus `cargo test --doc` in EINEM Log. nextest nennt keine
/// Ignore-Gruende und rueckt die Zeilen `test result:` der einzelnen Tests ein; das Skript muss dieselben Zahlen liefern wie
/// aus einem `cargo test`-Log.
#[test]
fn das_skript_zaehlt_aus_einem_nextest_log_dieselben_arten() {
    let log = fixture("ci_lauf_nextest.log");
    let quellen = quellen_ordner("nextest", 3);
    let tmp = std::env::temp_dir().join(format!("ci-nicht-geprueft-nextest-summary-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let o = lauf_mit(&log, &quellen, Some(&tmp));
    let tabelle = std::fs::read_to_string(&tmp).unwrap_or_default();
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_dir_all(&quellen);
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
    // Schema: 3 Zeilen der Hilfsfunktion + 1 eigene Wache. Browser 2, Verlauf 1.
    // Ignoriert: 5 uebersprungene aus `Summary` + 1 `ignored` aus dem Doctest = 6; die eingerueckte Zeile eines einzelnen Tests
    // (`7 ignored`) zaehlt NICHT. ERiC: 3 Attribute in `a.rs` (nicht die in `notiz.txt` und `target/b.rs`), sonst 6 - 3 = 3.
    assert!(
        out(&o).contains("NICHT GEPRUEFT: Schema-Pruefung 4x uebersprungen, Browser-Tests 2x uebersprungen, ERiC/checkESt 3x ignoriert, sonst 3x ignoriert, Verlauf-Pruefung 1x uebersprungen"),
        "{}",
        out(&o)
    );
    for zeile in [
        "| XML gegen das amtliche Schema (XSD) uebersprungen | 4 |",
        "| Browser-Tests uebersprungen | 2 |",
        "| Tests gegen ERiC und checkESt (`#[ignore]`) | 3 |",
        "| weitere `#[ignore]`-Tests | 3 |",
        "| Belege der Testkarte gegen den Git-Verlauf (Hash, Verlaufsform) uebersprungen | 1 |",
    ] {
        assert!(tabelle.contains(zeile), "Zusammenfassung ohne {zeile:?}: {tabelle}");
    }
}

/// Mehr ERiC-Tests im Quelltext als uebersprungene Tests im Log heisst: Log und Quelltext gehoeren nicht zusammen. Das Skript
/// rechnete sonst `sonst` negativ aus oder verschwiege den Fehler.
#[test]
fn mehr_eric_tests_im_quelltext_als_uebersprungene_im_log_ist_exit_2() {
    let log = fixture("ci_lauf_nextest.log");
    let passt = quellen_ordner("passt", 5); // 5 ERiC-Attribute, 5 + 1 = 6 uebersprungene im Log: genug
    let zuviel = quellen_ordner("zuviel", 7); // 7 > 6
    let (a, b) = (lauf_mit(&log, &passt, None), lauf_mit(&log, &zuviel, None));
    let _ = std::fs::remove_dir_all(&passt);
    let _ = std::fs::remove_dir_all(&zuviel);
    assert_eq!(a.status.code(), Some(0), "Kontrolle: {}", String::from_utf8_lossy(&a.stderr));
    assert_eq!(b.status.code(), Some(2), "{}", out(&b));
}

/// Ohne `CI_NICHT_GEPRUEFT_QUELLEN` liest das Skript `rust/` neben `scripts/`. Gegenprobe in Rust: dieselbe Zahl, durch einen
/// eigenen Gang ueber die Dateien (ohne den Ordner `target`) gezaehlt.
#[test]
fn ohne_umgebungsvariable_zaehlt_das_skript_die_eric_tests_unter_rust() {
    let mut dateien = Vec::new();
    sammle(&wurzel().join("rust"), &mut dateien);
    let attribut = eric_attribut("");
    let attribut = attribut.trim_end_matches("\"]");
    let erwartet: usize = dateien
        .iter()
        .map(|p|std::fs::read_to_string(p).unwrap_or_default().lines().filter(|z| z.contains(attribut)).count())
        .sum();
    assert!(erwartet >= 1, "Kontrolle: kein ERiC-Test unter rust/ gefunden; der Gang ueber die Dateien sieht nichts");
    let tmp = std::env::temp_dir().join(format!("ci-nicht-geprueft-wurzel-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let log = tmp.join("nextest.log");
    std::fs::write(&log, "     Summary [   1.000s] 5000 tests run: 5000 passed, 4000 skipped\n").unwrap();
    let o = lauf(&log);
    let _ = std::fs::remove_dir_all(&tmp);
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(out(&o).contains(&format!("ERiC/checkESt {erwartet}x ignoriert")), "erwartet {erwartet}: {}", out(&o));
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
        ("grep -c 'UEBERSPRUNGEN: Verlauf fehlt'", "rust/bindung/tests/belege_aufloesbar.rs", "UEBERSPRUNGEN: Verlauf fehlt"),
    ];
    for (aufruf, datei, text) in paare {
        assert!(s.contains(aufruf), "das Skript ruft nicht {aufruf}");
        assert!(lies(datei).contains(text), "{datei} schreibt nicht mehr {text:?}");
    }
    // nextest nennt keinen Ignore-Grund: im nextest-Fall zaehlt das Skript das Attribut im Quelltext. Das Muster im Skript und das
    // Attribut in jeder der fuenf Dateien mit ERiC-Tests sind dieselbe Zeichenfolge.
    let attribut = eric_attribut("");
    let attribut = attribut.trim_end_matches("\"]");
    assert!(s.contains("'#\\[ignore = \"braucht die ERiC-Bibliothek'"), "das Skript sucht das Attribut nicht mehr im Quelltext");
    for datei in [
        "rust/api/tests/einreichen_eric_echt.rs",
        "rust/api/tests/xml_export_hermetisch.rs",
        "rust/bescheid/tests/versorgung_kz_im_xml.rs",
        "rust/bescheid/tests/checkest_blockmatrix.rs",
        "rust/bescheid/tests/checkest_privatverkauf_spike.rs",
    ] {
        assert!(lies(datei).contains(attribut), "{datei} hat das Attribut {attribut:?} nicht mehr");
    }
    // Die nextest-Zusammenfassung: Zeile `Summary [..] N tests run: .., M skipped`.
    assert!(s.contains("grep -c '^ *Summary \\['") && s.contains("\"skipped\""), "das Skript liest die nextest-Zusammenfassung nicht");
}

#[test]
fn der_rust_job_ruft_das_skript_nach_nextest_und_doctests_auch_bei_rot() {
    let text = std::fs::read_to_string(wurzel().join(".github/workflows/ci.yml")).unwrap();
    let wf: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
    let schritte = wf["jobs"]["rust"]["steps"].as_sequence().expect("Job rust ohne Schritte");
    let lauf_von = |s: &serde_yaml_ng::Value| s["run"].as_str().unwrap_or("").to_owned();
    let i_test = schritte.iter().position(|s| lauf_von(s).contains("cargo nextest run")).expect("kein cargo nextest run");
    let i_doc = schritte.iter().position(|s| lauf_von(s).contains("cargo test --doc")).expect("kein cargo test --doc: nextest fuehrt keine Doctests aus");
    assert!(i_test < i_doc, "die Doctests muessen NACH nextest laufen (`tee -a` haengt an das Log von nextest an)");
    // Die Verzichtszeilen stehen auf stderr. Ohne `2>&1` vor dem `tee` fehlen sie im Log, und das Skript zaehlte still null.
    // Ohne `shell: bash` laeuft der Schritt als `bash -e {0}` OHNE pipefail: dann gaebe `cargo nextest run | tee` den Exit von
    // tee (0) zurueck, und ein roter Test machte den Job gruen. `shell: bash` schaltet pipefail an.
    for (i, befehl, tee) in [(i_test, "cargo nextest run", "tee ../cargo-test.log"), (i_doc, "cargo test --doc", "tee -a ../cargo-test.log")] {
        let lauf = lauf_von(&schritte[i]);
        assert!(lauf.contains(tee), "`{befehl}` schreibt sein Log nicht per `{tee}`: {lauf:?}");
        let stellen = (lauf.find(befehl), lauf.find("2>&1"), lauf.find("tee"));
        assert!(matches!(stellen, (Some(c), Some(e), Some(t)) if c < e && e < t), "`{befehl}` leitet stderr nicht per `2>&1` vor dem tee ins Log: {lauf:?}");
        assert_eq!(schritte[i]["shell"].as_str(), Some("bash"), "`{befehl} | tee` braucht `shell: bash` (pipefail)");
    }
    // Ein roter Test darf die Doctests nicht verdecken: so verdeckte `parity --lib` am 2026-10-02 den roten eingang-Doctest.
    assert_eq!(schritte[i_doc]["if"].as_str(), Some("always()"), "die Doctests laufen nicht bei rotem nextest");
    // nextest laeuft mit dem Profil ci (Ausgabe bestandener Tests am Ende) und ohne Abbruch beim ersten roten Test.
    let nextest = lauf_von(&schritte[i_test]);
    assert!(nextest.contains("--profile ci"), "nextest laeuft nicht mit `--profile ci`: ohne success-output fehlen die Verzichtszeilen im Log");
    assert!(nextest.contains("--no-fail-fast"), "nextest bricht beim ersten roten Test ab");
    assert!(lauf_von(&schritte[i_doc]).contains("--no-fail-fast"), "die Doctests brechen beim ersten roten Test ab");
    let nach = schritte[i_doc + 1..].iter().find(|s| lauf_von(s).contains("scripts/ci-nicht-geprueft.sh"));
    let nach = nach.expect("kein Schritt nach den Doctests ruft scripts/ci-nicht-geprueft.sh");
    assert_eq!(nach["if"].as_str(), Some("always()"), "der Schritt laeuft nicht bei rotem Test");
    // Das Log, das tee schreibt, ist das Log, das das Skript liest (nur der Dateiname zaehlt: die Test-Schritte stehen in `rust/`).
    let nach_wort = |text: &str, wort: &str| text.split_whitespace().skip_while(|w| *w != wort).nth(1).map(|w| Path::new(w).file_name().map(std::borrow::ToOwned::to_owned));
    let gelesen = nach_wort(&lauf_von(nach), "scripts/ci-nicht-geprueft.sh");
    for i in [i_test, i_doc] {
        let geschrieben = nach_wort(&lauf_von(&schritte[i]).replace("tee -a", "tee"), "tee");
        assert!(geschrieben.is_some() && geschrieben == gelesen, "tee schreibt {geschrieben:?}, das Skript liest {gelesen:?}");
    }
    // Dass das Profil ci die Ausgabe bestandener Tests einmal am Ende zeigt, haelt ci_konfiguration.rs fest (nextest.toml).
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
            text.contains("TAXGRAPH_OHNE_XSD") || text.contains("TAXGRAPH_OHNE_BROWSER") || text.contains("TAXGRAPH_OHNE_VERLAUF")
        })
        .map(|(i, _)| quelle[..i].matches('\n').count() + 1)
        .collect()
}

fn sammle(dir: &Path, aus: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            // Der Bauordner `target` kann Millionen Dateien haben und enthaelt keine Quelltexte dieses Baums.
            if p.file_name().is_none_or(|n| n != "target") {
                sammle(&p, aus);
            }
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
    // Ernstfall: alle Testdateien unter rust/*/tests und die Quelltexte unter rust/*/src (`elster::testhilfe::schemas_da` liegt in src).
    let mut dateien = Vec::new();
    for krate in std::fs::read_dir(wurzel().join("rust")).unwrap() {
        let krate = krate.unwrap().path();
        for unter in ["tests", "src"] {
            let t = krate.join(unter);
            if t.is_dir() {
                sammle(&t, &mut dateien);
            }
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
