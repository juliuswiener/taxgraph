//! `scripts/freeze_source.py` in Rust (`quellen::einfrieren`): dieselbe Eingabe-HTML ergibt BYTE-GLEICHE `.txt` und
//! `.meta.yaml`, dieselbe Meldung und dieselbe Ablehnung wie das Python-Skript.
//!
//! GOLDEN: die Ordner unter `tests/daten/einfrieren/` hat `scripts/freeze_source.py` EINMAL erzeugt (das Netz ist dabei durch
//! eine Attrappe ersetzt, die die Bytes von `eingabe.*` liefert; `fetch()` des Skripts lief mit: gzip-Magic, `decode(…,
//! "replace")`). Der Test liest dieselbe `aufruf.txt` mit demselben Argument-Leser wie das Programm und vergleicht Byte für
//! Byte. Die Eingaben sind erfunden; kein Rechtstext steht in den Fixturen.
//!
//! Randfälle (je ein Ordner): `<script>` und `</p>` darin, `&nbsp;`/`&sect;`/`&ndash;`, alle 253 Entities der Tabelle,
//! Start ohne Ende, `ende` zu nah am Start (Zeichen, nicht Bytes), Start nicht gefunden, zu kurz, `--erwarte` fehlt, rohes
//! gzip, ungültiges UTF-8, Großbuchstaben-Tags, offene Tags, Kommentare. Weicht Rust absichtlich ab (unbekannte Entity,
//! `&name` ohne Semikolon), steht der Test in `entity_hermetisch.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use quellen::cli::einfrier_auftrag;
use quellen::einfrieren::{friere_ein, text_aus_antwort, Werkzeuge};

fn daten() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/daten/einfrieren")
}

fn gzip_da() -> bool {
    std::process::Command::new("gzip").arg("--version").output().is_ok_and(|o| o.status.success())
}

/// Führt einen Fall aus und vergleicht mit dem Python-Ergebnis.
fn fall(name: &str) {
    let dir = daten().join(name);
    let aufruf = fs::read_to_string(dir.join("aufruf.txt")).unwrap();
    let eingabe = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("eingabe."))
        .unwrap();
    if eingabe.to_string_lossy().ends_with(".gz") && !gzip_da() {
        writeln!(std::io::stderr(), "UEBERSPRUNGEN: gzip fehlt, Fall {name} nicht geprueft").unwrap();
        return;
    }
    let out = tempfile::tempdir().unwrap();
    let mut args: Vec<String> = aufruf.lines().map(str::to_owned).collect();
    args.push("--out".into());
    args.push(out.path().display().to_string());
    let auftrag = einfrier_auftrag(&args, Path::new("/nicht/benutzt")).unwrap();
    let text = text_aus_antwort(fs::read(&eingabe).unwrap(), &Werkzeuge::default()).unwrap();
    let ergebnis = friere_ein(&auftrag, &text);
    let geschrieben: Vec<String> = {
        let mut v: Vec<String> =
            fs::read_dir(out.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    };
    if dir.join("fehler.txt").exists() {
        let fehler = ergebnis.expect_err(&format!("{name}: Python lehnt ab, Rust nicht"));
        assert_eq!(format!("{fehler}\n"), fs::read_to_string(dir.join("fehler.txt")).unwrap(), "{name}: Meldung");
        assert_eq!(geschrieben, Vec::<String>::new(), "{name}: bei einer Ablehnung wird nichts geschrieben");
        return;
    }
    let gefroren = ergebnis.unwrap_or_else(|e| panic!("{name}: Python nimmt an, Rust lehnt ab: {e}"));
    let mut erwartet: Vec<String> = fs::read_dir(dir.join("erwartet"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    erwartet.sort();
    assert_eq!(geschrieben, erwartet, "{name}: dieselben Dateien");
    for datei in &erwartet {
        let rust = fs::read(out.path().join(datei)).unwrap();
        let python = fs::read(dir.join("erwartet").join(datei)).unwrap();
        assert!(rust == python, "{name}: {datei} ist nicht byte-gleich (Rust {} Bytes, Python {} Bytes)", rust.len(), python.len());
    }
    assert_eq!(format!("{}\n", gefroren.meldung), fs::read_to_string(dir.join("stdout.txt")).unwrap(), "{name}: Meldung");
}

macro_rules! faelle {
    ($($name:ident),* $(,)?) => {
        $(#[test] fn $name() { fall(stringify!($name)); })*
        const ALLE: &[&str] = &[$(stringify!($name)),*];
    };
}

faelle!(
    normal,
    bmf_schreiben,
    ohne_ende,
    ende_zu_nah,
    ende_zu_nah_umlaute,
    ende_nicht_gefunden,
    start_am_ende,
    gzip,
    ungueltiges_utf8,
    entities_vielfalt,
    entities_alle,
    sonderfaelle,
    kleine_laenge,
    fehler_start,
    fehler_zu_kurz,
    fehler_erwarte,
);

#[test]
fn jeder_ordner_der_fixturen_hat_einen_test_und_umgekehrt() {
    let mut im_baum: Vec<String> =
        fs::read_dir(daten()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    im_baum.sort();
    let mut im_test: Vec<String> = ALLE.iter().map(|s| (*s).to_owned()).collect();
    im_test.sort();
    assert_eq!(im_baum, im_test);
    let ablehnungen = im_baum.iter().filter(|n| daten().join(n).join("fehler.txt").exists()).count();
    assert_eq!((im_baum.len() - ablehnungen, ablehnungen), (13, 3), "13 Annahmen und 3 Ablehnungen");
}
