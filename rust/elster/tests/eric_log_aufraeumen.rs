//! Beim Laden von ERiC legt das Programm ein neues Log-Verzeichnis `eric_checkest_<8 Zeichen>` im
//! Temp-Ordner an und raeumt die alten weg: es bleiben die 20 juengsten (das neue zaehlt mit).
//!
//! Der Test legt im (per `TMPDIR` umgelenkten) Temp-Ordner 30 alte Verzeichnisse mit gestaffelter
//! Aenderungszeit an, dazu Fallen, die die Aufraeumung nicht anfassen darf (Symlink auf ein fremdes
//! Verzeichnis, Datei, falsch benannte Verzeichnisse) und ein Verzeichnis, das sich nicht loeschen
//! laesst. Dann ruft er `elster::validiere` gegen die Attrappe von `libericapi.so`
//! (`elster/tests/eric_attrappe/eric_attrappe.c`). Nie die echte Bibliothek. Die Mindestalter-Grenze
//! (juenger als 10 Minuten bleibt immer) prueft ein Test in `eric/mod.rs`: ein Prozess laedt ERiC nur einmal.
//!
//! Braucht `cc`. Fehlt es, scheitert der Test — ausser mit `TAXGRAPH_OHNE_CC=1`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

fn aelter_machen(pfad: &Path, alter: Duration) {
    std::fs::File::open(pfad)
        .unwrap()
        .set_modified(SystemTime::now() - alter)
        .unwrap();
}

fn stunden(n: u64) -> Duration {
    Duration::from_secs(n * 3600)
}

#[test]
fn nur_die_20_juengsten_log_verzeichnisse_bleiben() {
    let wurzel = std::env::temp_dir().join(format!("eric_logaufr_{}", std::process::id()));
    let (lib, steuer, tmp) = (wurzel.join("lib"), wurzel.join("steuer"), wurzel.join("tmp"));
    for d in [&lib, &steuer, &tmp] {
        std::fs::create_dir_all(d).unwrap();
    }
    let quelle = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/eric_attrappe/eric_attrappe.c");
    let gebaut = Command::new("cc")
        .args(["-shared", "-fPIC", "-O0", "-o"])
        .arg(lib.join("libericapi.so"))
        .arg(&quelle)
        .status();
    if !gebaut.as_ref().is_ok_and(std::process::ExitStatus::success) {
        assert!(
            std::env::var("TAXGRAPH_OHNE_CC").as_deref() == Ok("1"),
            "cc baut die Attrappe nicht ({gebaut:?}); nur TAXGRAPH_OHNE_CC=1 erlaubt das Fehlen"
        );
        eprintln!("[eric_log_aufraeumen] UEBERSPRUNGEN: kein cc, TAXGRAPH_OHNE_CC=1");
        return;
    }
    std::fs::write(steuer.join("skript"), "0\n").unwrap();

    // 30 alte Log-Verzeichnisse, `alt00000` das juengste (1 h), `alt00029` das aelteste.
    for i in 0..30_u64 {
        let d = tmp.join(format!("eric_checkest_alt{i:05}"));
        std::fs::create_dir(&d).unwrap();
        aelter_machen(&d, stunden(1) + Duration::from_secs(60 * i));
    }

    // Fallen, alle aelter als jedes `alt*` und damit weit hinter Rang 20.
    // Fremdes Verzeichnis, auf das ein Symlink mit passendem Namen zeigt.
    let fremd = tmp.join("fremd");
    std::fs::create_dir(&fremd).unwrap();
    std::fs::write(fremd.join("inhalt"), "bleibt\n").unwrap();
    aelter_machen(&fremd, stunden(5));
    let link = tmp.join("eric_checkest_link0001");
    std::os::unix::fs::symlink(&fremd, &link).unwrap();
    // Datei mit passendem Namen.
    let datei = tmp.join("eric_checkest_datei001");
    std::fs::write(&datei, "kein Verzeichnis\n").unwrap();
    aelter_machen(&datei, stunden(5));
    // Verzeichnisse, deren Name nicht genau Praefix plus 8 Zeichen aus `a-z0-9_` ist.
    let falsche_namen = [
        "eric_checkest_kurz123",   // 7 Zeichen
        "eric_checkest_zulang123", // 9 Zeichen
        "eric_checkest_ABCDEFGH",  // Grossbuchstaben
        "eric_checkestxabcdefgh",  // Praefix ohne Unterstrich
        "andere_alt00001",
    ];
    for n in falsche_namen {
        let d = tmp.join(n);
        std::fs::create_dir(&d).unwrap();
        aelter_machen(&d, stunden(5));
    }
    // Ein Verzeichnis, das sich nicht loeschen laesst: ein Unterverzeichnis ohne Schreibrecht. Es ist
    // aelter als `alt00025` und juenger als `alt00026`: bricht die Aufraeumung bei dem Fehler ab,
    // bleiben `alt00026` bis `alt00029` liegen.
    let stur = tmp.join("eric_checkest_stur0001");
    let sperre = stur.join("drin");
    std::fs::create_dir_all(&sperre).unwrap();
    std::fs::write(sperre.join("x"), "x\n").unwrap();
    std::fs::set_permissions(&sperre, std::fs::Permissions::from_mode(0o500)).unwrap();
    aelter_machen(&stur, stunden(1) + Duration::from_secs(60 * 25 + 30));
    // Wer root ist, loescht trotz fehlendem Schreibrecht: dann gibt es hier nichts zu pruefen.
    let probe = tmp.join("probe");
    std::fs::create_dir_all(probe.join("d")).unwrap();
    std::fs::write(probe.join("d/x"), "x\n").unwrap();
    std::fs::set_permissions(probe.join("d"), std::fs::Permissions::from_mode(0o500)).unwrap();
    let ohne_wirkung = std::fs::remove_file(probe.join("d/x")).is_ok();
    std::fs::set_permissions(probe.join("d"), std::fs::Permissions::from_mode(0o700)).unwrap();

    // Dieser Test ist der einzige seines Binaries: niemand liest die Umgebung gleichzeitig.
    std::env::set_var("TMPDIR", &tmp);
    std::env::set_var("ERIC_DIR", &wurzel);
    std::env::set_var("ERIC_ATTRAPPE_DIR", &steuer);
    let (rc, _) = elster::validiere(b"<Elster/>", "ESt_2025").unwrap();
    assert_eq!(rc, 0, "ein Fehler beim Aufraeumen darf den ERiC-Start nicht abbrechen");

    let mut alt: Vec<String> = std::fs::read_dir(&tmp)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("eric_checkest_alt"))
        .collect();
    alt.sort();
    // Das neue Verzeichnis zaehlt mit: von den 30 alten bleiben die 19 juengsten.
    let erwartet: Vec<String> = (0..19).map(|i| format!("eric_checkest_alt{i:05}")).collect();
    assert_eq!(alt, erwartet, "es muessen die juengsten bleiben");
    let neu = elster::eric_log_pfad().unwrap();
    assert!(neu.parent().unwrap().starts_with(&tmp), "{neu:?}");
    assert!(neu.parent().unwrap().is_dir(), "{neu:?}");

    assert!(link.symlink_metadata().unwrap().file_type().is_symlink(), "Symlink entfernt");
    assert_eq!(std::fs::read_to_string(fremd.join("inhalt")).unwrap(), "bleibt\n");
    assert!(datei.is_file(), "Datei entfernt");
    for n in falsche_namen {
        assert!(tmp.join(n).is_dir(), "{n} entfernt");
    }
    if ohne_wirkung {
        eprintln!("[eric_log_aufraeumen] root: das nicht loeschbare Verzeichnis ist loeschbar");
    } else {
        assert!(sperre.join("x").is_file(), "das nicht loeschbare Verzeichnis ist weg");
    }

    std::fs::set_permissions(&sperre, std::fs::Permissions::from_mode(0o700)).unwrap();
    let _ = std::fs::remove_dir_all(&wurzel);
}
