//! Das Zeitlimit des Beleg-Lesers ohne Textlayer: EIN `tesseract`-Lauf ueber die ganze Datei darf
//! `TESSERACT_ZEITLIMIT` x `OCR_SEITEN_HOECHSTZAHL` dauern (60 s x 40), nicht nur 60 s, weil die
//! Seitenzahl dort noch nicht feststeht (`beleg_writer.py:238-240`).
//!
//! Der Test wartet wirklich (61 s), weil die Zeit keine Einspritzstelle hat; er laeuft deshalb nur auf
//! Anforderung: `cargo test -p eingang --test ocr_zeitlimit -- --ignored`
//! Gemessen mit einer Mutationsliste von Hand (`berichte/auth-eingang-mutation.md`): mit dem Limit
//! `TESSERACT_ZEITLIMIT` allein bricht `lauf` das Kind nach 60 s ab, und dieser Test wird rot.
//!
//! HERMETISCH: `PATH` zeigt auf ein Verzeichnis mit zwei Skripten (pdftotext ohne Text, tesseract mit
//! 61 s Wartezeit). `sleep` ist kein Builtin, darum steht sein absoluter Pfad im Skript.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::os::unix::fs::PermissionsExt;

use eingang::ocr::{lies_beleg_text, TESSERACT_ZEITLIMIT};

#[test]
#[ignore = "wartet 61 s: manuell mit --ignored"]
fn ein_tesseract_lauf_ueber_die_ganze_datei_darf_laenger_als_eine_seite_brauchen() {
    let sleep = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join("sleep"))
        .find(|p| p.is_file())
        .expect("sleep auf PATH");
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let wartezeit = TESSERACT_ZEITLIMIT.as_secs() + 1;
    for (name, rumpf) in [
        ("pdftotext", "exit 0".to_owned()),
        (
            "tesseract",
            format!("{} {wartezeit}; printf 'fertig\\n'", sleep.display()),
        ),
    ] {
        let datei = bin.join(name);
        std::fs::write(&datei, format!("#!/bin/sh\n{rumpf}\n")).unwrap();
        std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::env::set_var("PATH", &bin);

    let pdf = tmp.path().join("scan.pdf").to_string_lossy().into_owned();
    let (text, konfidenz) = lies_beleg_text(&pdf).unwrap();
    assert_eq!(text, "fertig\n");
    assert!(konfidenz.is_empty());
}
