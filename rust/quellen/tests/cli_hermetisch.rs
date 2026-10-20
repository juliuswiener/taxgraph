//! Das Programm `taxgraph-quellen` als Ganzes: Exit-Codes wie die Python-Skripte (0 gut, 1 abgelehnt, 2 falsche Aufrufform),
//! `pruefe` auf einem kleinen Baum, `einfrieren` von der Abfrage über `file://` bis zur Datei, ohne Netz.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::explicit_write)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};

fn programm(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_taxgraph-quellen")).args(args).output().unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn fixtur() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/daten/einfrieren/normal")
}

#[test]
fn ohne_befehl_mit_unbekanntem_befehl_und_mit_falschen_optionen_ist_der_exit_code_2() {
    for args in [
        vec![],
        vec!["tanzen"],
        vec!["pruefe", "--gibtesnicht", "x"],
        vec!["pruefe", "unerwartet"],
        vec!["pruefe", "--wurzel"],
        vec!["einfrieren", "--name", "n"],
        vec!["einfrieren", "--url", "u", "--name", "n", "--norm", "N", "--start", "s", "--abrufdatum", "d", "--ebene", "kreis"],
        vec!["einfrieren", "--url", "u", "--name", "n", "--norm", "N", "--start", "s", "--abrufdatum", "d", "--authority", "x"],
        vec!["einfrieren", "--url", "u", "--name", "n", "--norm", "N", "--start", "s", "--abrufdatum", "d", "--min-laenge", "viel"],
    ] {
        let a = programm(&args);
        assert_eq!(a.status.code(), Some(2), "{args:?}: {}", text(&a.stderr));
        assert!(text(&a.stderr).contains("Aufruf: taxgraph-quellen"), "{args:?}");
    }
    let a = programm(&["einfrieren", "--name", "n"]);
    assert!(text(&a.stderr).contains("--url"), "die erste fehlende Pflichtoption steht in der Meldung: {}", text(&a.stderr));
}

#[test]
fn die_hilfe_endet_mit_0_und_nennt_alle_drei_befehle_die_es_gibt() {
    let a = programm(&["--help"]);
    assert_eq!(a.status.code(), Some(0));
    let s = text(&a.stdout);
    assert!(s.contains("pruefe") && s.contains("einfrieren"), "{s}");
}

#[test]
fn pruefe_druckt_die_zeilen_und_endet_mit_0_oder_1() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("sources/a");
    fs::create_dir_all(&dir).unwrap();
    let wortlaut = format!("--- Wortlaut {}\n", "x".repeat(80));
    fs::write(dir.join("q.txt"), &wortlaut).unwrap();
    let sha = format!("{:x}", Sha256::digest(wortlaut.as_bytes()));
    fs::write(dir.join("q.meta.yaml"), format!("dokument:\n  datei: \"q.txt\"\n  sha256: \"{sha}\"\n")).unwrap();
    let wurzel = t.path().to_str().unwrap();
    let gut = programm(&["pruefe", "--wurzel", wurzel]);
    assert_eq!((gut.status.code(), text(&gut.stdout)), (Some(0), "OK       sources/a/q.txt\n".to_owned()));
    // auch in der Schreibweise --wurzel=…
    let gleich = programm(&["pruefe", &format!("--wurzel={wurzel}")]);
    assert_eq!(text(&gleich.stdout), text(&gut.stdout));
    fs::write(dir.join("q.txt"), "anders\n").unwrap();
    let schlecht = programm(&["pruefe", "--wurzel", wurzel]);
    assert_eq!((schlecht.status.code(), text(&schlecht.stdout)), (Some(1), "MISMATCH sources/a/q.txt\n".to_owned()));
}

#[test]
fn pruefe_ohne_wurzel_prueft_den_baum_in_dem_das_programm_gebaut_wurde_und_der_ist_ok() {
    let a = programm(&["pruefe"]);
    assert_eq!(a.status.code(), Some(0), "{}", text(&a.stdout).lines().filter(|z| !z.starts_with("OK ")).collect::<Vec<_>>().join("\n"));
    assert!(text(&a.stdout).lines().count() > 100);
}

#[test]
fn einfrieren_von_file_url_bis_zur_datei_ist_das_golden_ergebnis_mit_der_url_der_datei() {
    let t = tempfile::tempdir().unwrap();
    let url = format!("file://{}", fixtur().join("eingabe.html").display());
    let out = t.path().join("aus");
    fs::create_dir_all(&out).unwrap();
    let mut args: Vec<String> = fs::read_to_string(fixtur().join("aufruf.txt")).unwrap().lines().map(str::to_owned).collect();
    let i = args.iter().position(|a| a == "--url").unwrap();
    args[i + 1] = url.clone();
    args.extend(["--out".to_owned(), out.display().to_string()]);
    let a = programm(&std::iter::once("einfrieren".to_owned()).chain(args).collect::<Vec<_>>().iter().map(String::as_str).collect::<Vec<_>>());
    assert_eq!(a.status.code(), Some(0), "{}", text(&a.stderr));
    // erwartet: die Python-Ausgabe, nur mit der URL der Datei; die Prüfsumme gehört zum Text MIT dieser URL
    let golden_txt = fs::read_to_string(fixtur().join("erwartet/fall_normal.txt")).unwrap();
    let erwartet_txt = golden_txt.replace("https://example.invalid/normal.html", &url);
    assert_eq!(fs::read_to_string(out.join("fall_normal.txt")).unwrap(), erwartet_txt);
    let sha = format!("{:x}", Sha256::digest(erwartet_txt.as_bytes()));
    let golden_meta = fs::read_to_string(fixtur().join("erwartet/fall_normal.meta.yaml")).unwrap();
    let alt_sha = golden_meta.lines().find(|l| l.trim().starts_with("sha256:")).unwrap().to_owned();
    let erwartet_meta = golden_meta.replace("https://example.invalid/normal.html", &url).replace(&alt_sha, &format!("  sha256: \"{sha}\""));
    assert_eq!(fs::read_to_string(out.join("fall_normal.meta.yaml")).unwrap(), erwartet_meta);
    assert_eq!(text(&a.stdout), format!("fall_normal: 1775 Zeichen, 2 Passage(n) verifiziert, sha256 {}\n", &sha[..16]));
}

#[test]
fn einfrieren_ohne_out_schreibt_nach_wurzel_sources_gesetze_im_internet() {
    let t = tempfile::tempdir().unwrap();
    fs::create_dir_all(t.path().join("sources/gesetze-im-internet")).unwrap();
    let url = format!("file://{}", fixtur().join("eingabe.html").display());
    let a = programm(&[
        "einfrieren", "--url", &url, "--name", "ohne_out", "--norm", "N", "--start", "(1) 1 Erfundene Werbungskosten sind",
        "--abrufdatum", "2026-01-01", "--wurzel", t.path().to_str().unwrap(),
    ]);
    assert_eq!(a.status.code(), Some(0), "{}", text(&a.stderr));
    assert!(t.path().join("sources/gesetze-im-internet/ohne_out.txt").is_file());
    assert!(t.path().join("sources/gesetze-im-internet/ohne_out.meta.yaml").is_file());
}

#[test]
fn einfrieren_lehnt_ab_mit_exit_1_dem_text_von_python_und_ohne_dateien() {
    let t = tempfile::tempdir().unwrap();
    let url = format!("file://{}", fixtur().join("eingabe.html").display());
    let a = programm(&[
        "einfrieren", "--url", &url, "--name", "n", "--norm", "N", "--start", "gibt es nicht", "--abrufdatum", "2026-01-01", "--out",
        t.path().to_str().unwrap(),
    ]);
    assert_eq!((a.status.code(), text(&a.stderr)), (Some(1), "Start-Passage nicht gefunden: 'gibt es nicht'\n".to_owned()));
    assert_eq!(fs::read_dir(t.path()).unwrap().count(), 0);
    let b = programm(&["einfrieren", "--url", "ftp://example.invalid/x", "--name", "n", "--norm", "N", "--start", "s", "--abrufdatum", "d"]);
    assert_eq!(b.status.code(), Some(1));
    assert!(text(&b.stderr).contains("nur http://, https:// und file:// sind erlaubt"), "{}", text(&b.stderr));
}

#[test]
fn eine_unbekannte_entity_im_abruf_lehnt_ab_und_nennt_den_namen() {
    let t = tempfile::tempdir().unwrap();
    let html = t.path().join("e.html");
    fs::write(&html, format!("<p>START &foo; {}</p>", "Wort ".repeat(120))).unwrap();
    let out = t.path().join("aus");
    fs::create_dir_all(&out).unwrap();
    let a = programm(&[
        "einfrieren", "--url", &format!("file://{}", html.display()), "--name", "n", "--norm", "N", "--start", "START", "--abrufdatum",
        "2026-01-01", "--out", out.to_str().unwrap(),
    ]);
    assert_eq!(a.status.code(), Some(1));
    assert!(text(&a.stderr).contains("&foo;"), "{}", text(&a.stderr));
    assert_eq!(fs::read_dir(&out).unwrap().count(), 0, "nichts geschrieben");
}
