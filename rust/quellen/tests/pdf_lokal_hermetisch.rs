//! `scripts/freeze_pdf_local.py` in Rust (`quellen::pdf`, `taxgraph-quellen pdf-einfrieren`): dieselben Dateien, derselbe Kopf,
//! dieselbe Meta, dieselben Meldungen, dieselben Ablehnungen.
//!
//! GOLDEN: `tests/daten/pdf/probe/` hat das Python-Skript EINMAL an einem von Hand gebauten PDF (erfundener Text, Helvetica)
//! erzeugt: Kopf der `.txt`, Meta (die Prüfsumme des Textes maskiert: sie hängt von der Fassung von `pdftotext` ab) und die vier
//! Meldungen. Der Text selbst kommt im Test von `pdftotext` selbst. Der Byte-Vergleich auf echten Vordrucken steht im Bericht des
//! Auftrags.
//!
//! WACHE: Tests mit dem echten `pdftotext` melden "UEBERSPRUNGEN: pdftotext fehlt" und enden grün, wenn es fehlt (Poppler ist
//! kein Teil von Rust). Die Tests mit einem falschen `pdftotext` brauchen es nicht.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::explicit_write)]

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use quellen::cli::pdf_auftrag;
use quellen::einfrieren::Werkzeuge;
use quellen::pdf::{pdf_einfrieren, PdfAuftrag};
use quellen::Fehler;
use sha2::{Digest, Sha256};

fn probe() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/daten/pdf/probe")
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// `true`, wenn `pdftotext` aufrufbar ist; sonst eine Meldung, die die CI zählt, und `false`.
fn pdftotext_da() -> bool {
    let da = Command::new("pdftotext").arg("-v").output().is_ok();
    if !da {
        writeln!(std::io::stderr(), "UEBERSPRUNGEN: pdftotext fehlt").unwrap();
    }
    da
}

fn argumente(extra: &[&str]) -> Vec<String> {
    let mut a: Vec<String> = fs::read_to_string(probe().join("aufruf.txt")).unwrap().lines().map(str::to_owned).collect();
    a.extend(["--datei".to_owned(), probe().join("probe.pdf").display().to_string()]);
    a.extend(extra.iter().map(|s| (*s).to_owned()));
    a
}

fn auftrag(extra: &[&str]) -> PdfAuftrag {
    pdf_auftrag(&argumente(extra)).unwrap()
}

/// Was `pdftotext -layout` für das Probe-PDF liefert, mit Zeilenenden wie Python (`text=True`).
fn pdftotext_text() -> String {
    let aus = Command::new("pdftotext").args(["-layout"]).arg(probe().join("probe.pdf")).arg("-").output().unwrap();
    String::from_utf8(aus.stdout).unwrap().replace("\r\n", "\n").replace('\r', "\n")
}

fn falsches_pdftotext(dir: &Path, inhalt: &str) -> Werkzeuge {
    let pfad = super::unterprozess_hermetisch::skript(dir, "pdftotext", inhalt);
    Werkzeuge { pdftotext: pfad.display().to_string(), ..Werkzeuge::default() }
}

#[test]
fn ein_pdf_wird_eingefroren_wie_beim_python_skript() {
    if !pdftotext_da() {
        return;
    }
    let w = tempfile::tempdir().unwrap();
    let g = pdf_einfrieren(w.path(), &auftrag(&[]), &Werkzeuge::default()).unwrap();
    let dir = w.path().join("sources/pdfprobe");
    let txt = fs::read_to_string(dir.join("probe.txt")).unwrap();
    let kopf = fs::read_to_string(probe().join("erwartet-kopf.txt")).unwrap();
    assert!(txt.starts_with(&kopf), "Kopf der Textdatei weicht ab:\n{txt}");
    let koerper = txt[kopf.len()..].to_owned();
    assert_eq!(koerper, pdftotext_text());
    assert!(koerper.contains("Außergewöhnliche Belastungen") && koerper.contains("Zeile 1: Betrag in Euro"), "Text: {koerper:?}");
    assert!(fs::read(dir.join("probe.pdf")).unwrap() == fs::read(probe().join("probe.pdf")).unwrap(), "PDF-Kopie nicht byte-gleich");
    let txt_sha = sha(txt.as_bytes());
    let meta = fs::read_to_string(dir.join("probe.meta.yaml")).unwrap();
    assert_eq!(meta, fs::read_to_string(probe().join("erwartet.meta.yaml")).unwrap().replace("<TXT_SHA256>", &txt_sha));
    assert_eq!((g.txt_sha256.as_str(), g.pdf_sha256.as_str()), (txt_sha.as_str(), sha(&fs::read(probe().join("probe.pdf")).unwrap()).as_str()));
    let erwartet_stdout = fs::read_to_string(probe().join("stdout.txt"))
        .unwrap()
        .replace("<W>", &w.path().display().to_string())
        .replace("<TXT_SHA12>", &txt_sha[..12]);
    assert_eq!(format!("{}\n", g.meldungen.join("\n")), erwartet_stdout);
    let mut dateien: Vec<String> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    dateien.sort();
    assert_eq!(dateien, ["probe.meta.yaml", "probe.pdf", "probe.txt"]);
}

#[test]
fn ein_zweiter_lauf_ueberschreibt_ohne_fehler() {
    if !pdftotext_da() {
        return;
    }
    let w = tempfile::tempdir().unwrap();
    let eins = pdf_einfrieren(w.path(), &auftrag(&[]), &Werkzeuge::default()).unwrap();
    let zwei = pdf_einfrieren(w.path(), &auftrag(&[]), &Werkzeuge::default()).unwrap();
    assert_eq!(eins, zwei);
}

#[test]
fn eine_fehlende_anker_passage_lehnt_ab_und_schreibt_nichts_nicht_einmal_den_ordner() {
    if !pdftotext_da() {
        return;
    }
    let w = tempfile::tempdir().unwrap();
    let e = pdf_einfrieren(w.path(), &auftrag(&["--erwarte", "steht nirgends", "--erwarte", "auch nicht"]), &Werkzeuge::default()).unwrap_err();
    // die Anker aus aufruf.txt stehen im Text; nur die zwei neuen fehlen
    assert_eq!(e.to_string(), "Anker fehlen woertlich im Text (Freeze abgebrochen): steht nirgends; auch nicht");
    assert_eq!(fs::read_dir(w.path()).unwrap().count(), 0, "nichts geschrieben");
}

#[test]
fn ein_zu_kurzer_text_lehnt_ab_mit_der_zahl_der_zeichen() {
    if !pdftotext_da() {
        return;
    }
    let w = tempfile::tempdir().unwrap();
    let laenge = pdftotext_text().chars().count();
    let e = pdf_einfrieren(w.path(), &auftrag(&["--min-laenge", "500"]), &Werkzeuge::default()).unwrap_err();
    assert_eq!(e.to_string(), format!("Text zu kurz ({laenge} < 500) — Extraktion pruefen."));
    assert_eq!(fs::read_dir(w.path()).unwrap().count(), 0);
    // genau auf der Grenze reicht
    let ok = pdf_einfrieren(w.path(), &auftrag(&["--min-laenge", &laenge.to_string()]), &Werkzeuge::default());
    assert!(ok.is_ok(), "{ok:?}");
}

#[test]
fn ein_fehlendes_pdf_lehnt_ab_mit_dem_pfad() {
    let w = tempfile::tempdir().unwrap();
    let a = PdfAuftrag { datei: "/nicht/da/x.pdf".into(), ..auftrag(&[]) };
    let e = pdf_einfrieren(w.path(), &a, &Werkzeuge::default()).unwrap_err();
    assert_eq!(e.to_string(), "PDF nicht gefunden: /nicht/da/x.pdf — Julius muss es zuerst lokal ablegen.");
    // ein Ordner ist kein PDF
    let a = PdfAuftrag { datei: w.path().display().to_string(), ..auftrag(&[]) };
    assert!(matches!(pdf_einfrieren(w.path(), &a, &Werkzeuge::default()), Err(Fehler::PdfFehlt { .. })));
}

#[test]
fn ein_fehlendes_pdftotext_ist_der_text_von_python() {
    let w = tempfile::tempdir().unwrap();
    let wz = Werkzeuge { pdftotext: "pdftotext-das-es-nicht-gibt".into(), ..Werkzeuge::default() };
    let e = pdf_einfrieren(w.path(), &auftrag(&[]), &wz).unwrap_err();
    assert_eq!(e.to_string(), "pdftotext nicht gefunden (poppler-utils installieren).");
}

#[test]
fn ein_pdftotext_mit_fehlercode_lehnt_ab_mit_den_ersten_300_zeichen_der_fehlerausgabe() {
    let t = tempfile::tempdir().unwrap();
    let w = tempfile::tempdir().unwrap();
    let wz = falsches_pdftotext(t.path(), "printf 'e%.0s' $(seq 1 400) >&2\nexit 1");
    let e = pdf_einfrieren(w.path(), &auftrag(&[]), &wz).unwrap_err();
    assert_eq!(e.to_string(), format!("pdftotext fehlgeschlagen: {}", "e".repeat(300)));
    assert_eq!(fs::read_dir(w.path()).unwrap().count(), 0);
}

#[test]
fn eine_ausgabe_ohne_utf8_lehnt_ab() {
    let t = tempfile::tempdir().unwrap();
    let w = tempfile::tempdir().unwrap();
    let wz = falsches_pdftotext(t.path(), "printf 'ab\\377cd'");
    let e = pdf_einfrieren(w.path(), &auftrag(&["--min-laenge", "1"]), &wz).unwrap_err();
    assert!(matches!(e, Fehler::KeinUtf8), "{e:?}");
}

#[test]
fn zeilenenden_aus_pdftotext_werden_wie_in_python_zu_lf() {
    let t = tempfile::tempdir().unwrap();
    let w = tempfile::tempdir().unwrap();
    let wz = falsches_pdftotext(t.path(), "printf 'a\\r\\nb\\rc\\n'");
    let a = PdfAuftrag { erwarte: vec![], ..auftrag(&["--min-laenge", "1"]) };
    pdf_einfrieren(w.path(), &a, &wz).unwrap();
    let txt = fs::read_to_string(w.path().join("sources/pdfprobe/probe.txt")).unwrap();
    assert!(txt.ends_with("--- Textextraktion (pdftotext -layout) ---\n\na\nb\nc\n"), "{txt:?}");
}

fn programm(args: &[String]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_taxgraph-quellen")).arg("pdf-einfrieren").args(args).output().unwrap()
}

#[test]
fn das_programm_friert_ein_druckt_die_vier_zeilen_und_lehnt_mit_exit_1_ab() {
    if !pdftotext_da() {
        return;
    }
    let w = tempfile::tempdir().unwrap();
    let mut args = argumente(&[]);
    args.extend(["--wurzel".to_owned(), w.path().display().to_string()]);
    let a = programm(&args);
    assert_eq!(a.status.code(), Some(0), "{}", String::from_utf8_lossy(&a.stderr));
    let stdout = String::from_utf8(a.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 4, "{stdout}");
    assert!(stdout.starts_with(&format!("[freeze] {}/sources/pdfprobe/probe.txt  (Text sha256 ", w.path().display())), "{stdout}");
    assert!(stdout.ends_with("[freeze] Anker ok: ['Außergewöhnliche Belastungen', 'Zeile 1: Betrag']\n[freeze] make sources-check sollte jetzt gruen sein.\n"));
    // Ablehnung: Exit 1, der Text auf stderr, nichts Neues
    let mut args = argumente(&["--erwarte", "steht nirgends"]);
    args.extend(["--wurzel".to_owned(), tempfile::tempdir().unwrap().path().display().to_string(), "--name".to_owned(), "zwei".to_owned()]);
    let b = programm(&args);
    assert_eq!(b.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&b.stderr), "Anker fehlen woertlich im Text (Freeze abgebrochen): steht nirgends\n");
    // falsche Aufrufform: Exit 2
    assert_eq!(programm(&["--name".to_owned(), "x".to_owned()]).status.code(), Some(2));
}
