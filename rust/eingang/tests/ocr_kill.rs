//! Nach dem Zeitlimit wird das Kind beendet: `subprocess.run(..., timeout=...)` ruft bei Ablauf `kill()`
//! und wartet dann (`TimeoutExpired`). Ohne `kill` wartete `lauf` bis zum Ende des Programms, und ein
//! haengendes `pdftotext` hielte die Anfrage so lange fest, wie es will.
//!
//! Der Test wartet wirklich (30 s, `PDFTOTEXT_ZEITLIMIT`), weil die Zeit keine Einspritzstelle hat; er
//! laeuft deshalb nur auf Anforderung: `cargo test -p eingang --test ocr_kill -- --ignored`
//! Gemessen mit einer Mutationsliste von Hand (`berichte/auth-eingang-mutation.md`): ohne `kill` kehrt
//! `lauf` erst nach 45 s zurueck, und dieser Test wird rot.
//!
//! HERMETISCH: `PATH` zeigt auf ein Verzeichnis mit einem Skript `pdftotext`, das 45 s schlaeft und dann
//! eine Marke anlegt. `sleep` ist kein Builtin, darum steht sein absoluter Pfad im Skript; seine
//! Ausgabe geht nach `/dev/null`, damit das ueberlebende `sleep` die Pipe nicht offen haelt.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

use eingang::ocr::{lies_kontoauszug_pdf, OcrFehler, PDFTOTEXT_ZEITLIMIT};

#[test]
#[ignore = "wartet 30 s: manuell mit --ignored"]
fn nach_dem_zeitlimit_wird_das_kind_beendet_und_nicht_abgewartet() {
    let sleep = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join("sleep"))
        .find(|p| p.is_file())
        .expect("sleep auf PATH");
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let marke = tmp.path().join("marke");
    let skript = bin.join("pdftotext");
    std::fs::write(
        &skript,
        format!(
            "#!/bin/sh\n{} 45 >/dev/null; : > {}\n",
            sleep.display(),
            marke.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&skript, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("PATH", &bin);

    let pdf = tmp.path().join("x.pdf").to_string_lossy().into_owned();
    let beginn = Instant::now();
    let fehler = lies_kontoauszug_pdf(&pdf).unwrap_err();
    let dauer = beginn.elapsed();
    assert!(matches!(fehler, OcrFehler::Zeitlimit { .. }), "{fehler:?}");
    assert!(
        dauer >= PDFTOTEXT_ZEITLIMIT && dauer < PDFTOTEXT_ZEITLIMIT + Duration::from_secs(10),
        "nach {dauer:?} zurueck"
    );
    assert!(!marke.exists(), "das Skript lief bis zum Ende");
}
