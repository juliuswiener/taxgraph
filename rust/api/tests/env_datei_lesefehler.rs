//! Backlog `logging-fehlt-noch-beim-env-lesen`: eine vorhandene, aber nicht lesbare `.env` ist eine
//! Warnung im Fehlerlog, kein stilles Ueberspringen.
//!
//! Python (`server._lade_env_dateien`, `server.py:360`) protokolliert `server.env_datei_lesen` auf Stufe
//! WARNING, wenn die Datei existiert und sich nicht lesen laesst (`OSError`, kein UTF-8). Pfad und Inhalt
//! gehen nie ins Protokoll: ein Schluessel in der Datei ist ein Geheimnis. Rust hat das bis hierher
//! uebersprungen (`let Ok(text) = … else { continue }`), und kein Test rief `lade_env_dateien` auf.
//! Folge im Betrieb: `ELSTER_HERSTELLER_ID` oder der LLM-Schluessel fehlen, und niemand sieht warum.
//!
//! Eigene Testdatei mit EINEM Test, weil er die Prozess-Umgebung setzt (`TAXGRAPH_AUDIT_DIR` und zwei
//! Schluessel): in einem gemeinsamen Binaer liefe er gegen jeden Nachbartest, der die Umgebung liest.
//!
//! Der Fall nimmt eine `.env` aus ungueltigem UTF-8 statt einer Rechte-Sperre (`chmod 000`): als `root`
//! liesse sich die lesen, der Test waere dort rot oder leer. Beide Fehler laufen im Produktcode durch
//! dieselbe Zeile (`read_to_string` liefert `Err`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;

use api::konfig::lade_env_dateien;
use serde_json::{json, Value};

const GUELTIG: &str = "TG_ENVTEST_GUELTIG";
const KAPUTT: &str = "TG_ENVTEST_KAPUTT";
const ORT: &str = "server.env_datei_lesen";

fn eintraege(pfad: &std::path::Path) -> (String, Vec<Value>) {
    // Fehlt die Datei, ist das Ergebnis leer: der Test soll dann an der Zahl scheitern, nicht am `unwrap`.
    let text = fs::read_to_string(pfad).unwrap_or_default();
    let zeilen = text
        .lines()
        .map(|z| serde_json::from_str::<Value>(z).unwrap())
        .collect();
    (text, zeilen)
}

#[test]
fn unlesbare_env_datei_wird_als_warnung_protokolliert_ohne_pfad_und_inhalt() {
    let tmp = tempfile::tempdir().unwrap();
    let wurzel = tmp.path().join("repo");
    fs::create_dir_all(&wurzel).unwrap();
    // Das Verzeichnis existiert noch nicht: beim ersten Start legt der Dienst es erst spaeter an.
    let audit = tmp.path().join("audit").join("neu");
    std::env::set_var("TAXGRAPH_AUDIT_DIR", &audit);
    std::env::remove_var(GUELTIG);
    std::env::remove_var(KAPUTT);

    // `.env.maps` ist ein Verzeichnis (Python: `os.path.isfile` falsch, still), `.env.llm` ist gueltig,
    // `.env` ist vorhanden und kein UTF-8. Die erste Zeile von `.env` waere fuer sich gueltig.
    fs::create_dir(wurzel.join(".env.maps")).unwrap();
    fs::write(wurzel.join(".env.llm"), format!("{GUELTIG}=wert\n")).unwrap();
    fs::write(
        wurzel.join(".env"),
        [format!("{KAPUTT}=geheim\n").as_bytes(), b"\xff\xfe\n"].concat(),
    )
    .unwrap();

    lade_env_dateien(&wurzel);

    let log = audit.join("fehler.log");
    let (text, zeilen) = eintraege(&log);
    assert_eq!(
        zeilen.len(),
        1,
        "genau eine Warnung fuer die kaputte `.env` erwartet (nicht fuer das Verzeichnis, nicht fuer die \
         gueltige Datei); fehler.log: {text:?}"
    );
    assert_eq!(zeilen[0]["stufe"], json!("WARNING"), "{text}");
    assert_eq!(zeilen[0]["ort"], json!(ORT), "{text}");
    for verboten in [
        "geheim",
        KAPUTT,
        wurzel.to_str().unwrap(),
        tmp.path().to_str().unwrap(),
    ] {
        assert!(
            !text.contains(verboten),
            "Pfad oder Inhalt der `.env` steht im Protokoll ({verboten:?}): {text}"
        );
    }

    // Die Warnung ersetzt das Ueberspringen nicht: die gueltige Datei wirkt, die kaputte setzt nichts.
    assert_eq!(std::env::var(GUELTIG).as_deref(), Ok("wert"));
    assert!(std::env::var_os(KAPUTT).is_none());

    // Kontrolle: eine Wurzel ohne kaputte Datei schreibt nichts dazu.
    let sauber = tmp.path().join("sauber");
    fs::create_dir_all(&sauber).unwrap();
    lade_env_dateien(&sauber);
    let (text, zeilen) = eintraege(&log);
    assert_eq!(zeilen.len(), 1, "{text}");

    std::env::remove_var(GUELTIG);
    std::env::remove_var("TAXGRAPH_AUDIT_DIR");
}
