//! Ein Pfad ohne Verzeichnisteil (`fall.json`, `audit.jsonl`) liegt im aktuellen Verzeichnis:
//! `speichere` und `audit::anhaengen` legen kein Elternverzeichnis an und scheitern nicht daran
//! (`create_dir_all("")` waere ein Fehler, ist es aber nicht). Eigene Datei mit EINEM Test: er
//! setzt das Arbeitsverzeichnis fuer den Prozess und darf neben keinem anderen Test laufen.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use store::audit::{anhaengen, lies, AuditAktion};
use store::Store;

#[test]
fn pfade_ohne_verzeichnisteil_liegen_im_aktuellen_verzeichnis() {
    let dir = std::env::temp_dir().join(format!("taxgraph-n4-{}-relativ", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_current_dir(&dir).unwrap();

    let datei = Store::leer(2025, Some("demo".to_owned())).into_datei();
    store::speichere(Path::new("fall.json"), &datei).unwrap();
    assert_eq!(
        store::lade(Path::new("fall.json"))
            .unwrap()
            .fall_id
            .as_deref(),
        Some("demo")
    );

    anhaengen(
        Path::new("audit.jsonl"),
        Some("julius"),
        AuditAktion::Login,
        None,
        None,
    )
    .unwrap();
    assert_eq!(lies(Path::new("audit.jsonl")).unwrap().len(), 1);

    let mut namen: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    namen.sort();
    assert_eq!(namen, ["audit.jsonl", "fall.json"]);
    std::env::set_current_dir(std::env::temp_dir()).unwrap();
    std::fs::remove_dir_all(&dir).ok();
}
