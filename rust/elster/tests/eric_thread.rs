//! Alle ERiC-Aufrufe laufen auf EINEM Thread, gleich aus welchem Thread sie kommen.
//!
//! Pythons Server ruft ERiC (Singlethread-API) immer aus seinem Hauptthread. [`elster::validiere`]
//! gibt dafuer jeden Auftrag an einen eigenen Thread. Der Test baut die Attrappe von
//! `libericapi.so` (`parity/tests/eric_attrappe/eric_attrappe.c`, die auch der Vergleichslauf
//! benutzt), ruft `validiere` aus mehreren Threads nacheinander auf und liest, auf welchem Thread
//! die Attrappe jeden Aufruf sah. Nie die echte Bibliothek.
//!
//! Braucht `cc`. Fehlt es, scheitert der Test — ausser mit `TAXGRAPH_OHNE_CC=1`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

#[test]
fn alle_aufrufe_laufen_auf_einem_thread() {
    let wurzel = std::env::temp_dir().join(format!("eric_thread_{}", std::process::id()));
    let (lib, steuer) = (wurzel.join("lib"), wurzel.join("steuer"));
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::create_dir_all(&steuer).unwrap();
    let quelle = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity/tests/eric_attrappe/eric_attrappe.c");
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
        eprintln!("[eric_thread] UEBERSPRUNGEN: kein cc, TAXGRAPH_OHNE_CC=1");
        return;
    }
    std::fs::write(steuer.join("skript"), "0\n").unwrap();
    // Dieser Test ist der einzige seines Binaries: niemand liest die Umgebung gleichzeitig.
    std::env::set_var("ERIC_DIR", &wurzel);
    std::env::set_var("ERIC_ATTRAPPE_DIR", &steuer);

    let aufruf = || elster::validiere(b"<Elster/>", "ESt_2025").unwrap();
    assert_eq!(aufruf().0, 0);
    for _ in 0..6 {
        let h = std::thread::spawn(aufruf);
        assert_eq!(h.join().unwrap().0, 0);
    }
    assert_eq!(aufruf().0, 0);

    let mut threads = BTreeSet::new();
    let mut n = 0;
    while let Ok(meta) = std::fs::read_to_string(steuer.join(format!("gesehen/{}.meta", n + 1))) {
        n += 1;
        let t = meta.lines().find(|z| z.starts_with("thread=")).unwrap();
        threads.insert(t.to_owned());
    }
    assert_eq!(n, 8, "die Attrappe sah nicht alle Aufrufe");
    // Das Log-Verzeichnis ist wie bei `tempfile.mkdtemp` nur fuer den Besitzer lesbar: `eric.log`
    // nennt Auszuege der Erklaerung.
    let log = elster::eric_log_pfad().unwrap();
    let dir = log.parent().unwrap();
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.len() == "eric_checkest_".len() + 8 && name.starts_with("eric_checkest_"),
        "{name}"
    );
    let modus = std::fs::metadata(dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(modus, 0o700, "Log-Verzeichnis {dir:?}");
    assert_eq!(threads.len(), 1, "ERiC lief auf mehreren Threads: {threads:?}");
    let _ = std::fs::remove_dir_all(&wurzel);
}
