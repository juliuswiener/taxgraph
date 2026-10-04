//! Wachter fuer den Jahreswechsel: jedes Jahr unter `params/` kennt auch `domain::Vz`.
//!
//! Der Server nimmt ein Fall-Jahr an, sobald `params/` einen Eintrag mit diesem Namen hat
//! (`routen/fall.rs::verfuegbare_jahre`). Die Rechnung kennt ihre Jahre aber fest als `domain::Vz`
//! (2024..=2026). Legt jemand `params/2027` an und erweitert `Vz` nicht, legt Rust den Fall mit 201
//! an und antwortet danach auf `stand`, `fragen` und `ergebnis` mit 500 (Python rechnet mit 200).
//! Dieser Test wird beim Anlegen des Ordners rot, nicht erst bei der ersten Anfrage im neuen Jahr,
//! und nennt Ordner und Jahr. Er braucht weder Python noch `PARITY=1` noch Catala; er liest nur den
//! Ordnernamen. Quelle: Vault-Ticket `neues-params-jahr-ohne-vz-erweiterung-macht-in-rust-500`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use domain::Vz;

/// `params/` der Repo-Wurzel, relativ zu `rust/api`.
fn params() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../params")
}

/// Namen der Jahres-Eintraege in `params`: jeder Eintrag, dessen Name nur aus ASCII-Ziffern besteht.
/// Dieselbe Regel wie `verfuegbare_jahre` im Server (sie zaehlt auch Dateien, nicht nur Ordner);
/// `kohorten` und die `.py`/`.md`-Dateien zaehlen nicht.
fn jahres_namen(params: &Path) -> Vec<String> {
    let mut namen: Vec<String> = std::fs::read_dir(params)
        .unwrap_or_else(|e| panic!("{} nicht lesbar: {e}", params.display()))
        .map(|eintrag| eintrag.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| !name.is_empty() && name.bytes().all(|b| b.is_ascii_digit()))
        .collect();
    namen.sort();
    namen
}

/// Ein Befund je Jahres-Eintrag, den `Vz` nicht kennt; Ordner und Jahr stehen in der Zeile.
fn ohne_vz(namen: &[String]) -> Vec<String> {
    let mut befunde = Vec::new();
    for name in namen {
        match name.parse::<u16>() {
            Ok(jahr) => {
                if let Err(e) = Vz::try_from(jahr) {
                    befunde.push(format!(
                        "params/{name}: domain::Vz kennt das Jahr {jahr} nicht ({e}). Abhilfe: Variante, \
                         `jahr()` und `TryFrom<u16>` in rust/domain/src/vz.rs um {jahr} erweitern"
                    ));
                }
            }
            Err(e) => befunde.push(format!(
                "params/{name}: kein Veranlagungszeitraum, der Name passt nicht in u16 ({e})"
            )),
        }
    }
    befunde
}

#[test]
fn jedes_jahr_unter_params_ist_ein_vz() {
    let params = params();
    let namen = jahres_namen(&params);
    assert!(
        !namen.is_empty(),
        "kein Jahres-Eintrag unter {} gefunden: falscher Pfad oder Regel? (ein Lauf ohne Pruefling ist kein gruener)",
        params.display()
    );
    let befunde = ohne_vz(&namen);
    assert!(
        befunde.is_empty(),
        "{} von {} Jahren unter params/ kennt domain::Vz nicht:\n{}",
        befunde.len(),
        namen.len(),
        befunde.join("\n")
    );
}

/// Die Gegenprobe im Test selbst, unabhaengig davon, welche Jahre `Vz` heute kennt: ein Verzeichnis
/// mit genau den Jahren von `Vz` ist sauber; kommt das naechste Jahr dazu, nennt der Befund diesen
/// Ordner und das Jahr; ein Name ausserhalb u16 wird auch genannt; `kohorten` und eine Datei mit
/// Buchstaben zaehlen nicht.
#[test]
fn ein_jahr_ohne_vz_wird_mit_ordner_und_jahr_genannt() {
    let bekannt: Vec<u16> = (0..=u16::MAX)
        .filter(|jahr| Vz::try_from(*jahr).is_ok())
        .collect();
    assert!(!bekannt.is_empty(), "Vz kennt kein Jahr");
    let naechstes = bekannt.iter().max().unwrap().checked_add(1).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let params = dir.path().join("params");
    std::fs::create_dir(&params).unwrap();
    let mut erwartet: Vec<String> = bekannt.iter().map(u16::to_string).collect();
    erwartet.sort();
    for name in erwartet.iter().map(String::as_str).chain(["kohorten"]) {
        std::fs::create_dir(params.join(name)).unwrap();
    }
    std::fs::write(params.join("schema.md"), "").unwrap();

    let namen = jahres_namen(&params);
    assert_eq!(namen, erwartet);
    assert_eq!(
        ohne_vz(&namen),
        Vec::<String>::new(),
        "Kontrolle: bekannte Jahre"
    );

    std::fs::create_dir(params.join(naechstes.to_string())).unwrap();
    let befunde = ohne_vz(&jahres_namen(&params));
    let ordner = format!("params/{naechstes}:");
    let jahr = format!("Jahr {naechstes} nicht");
    assert!(
        matches!(befunde.as_slice(), [b] if b.starts_with(&ordner) && b.contains(&jahr)),
        "genau {naechstes} fehlt, mit Ordner und Jahr im Befund: {befunde:?}"
    );

    std::fs::create_dir(params.join("70000")).unwrap();
    let befunde = ohne_vz(&jahres_namen(&params));
    assert!(
        matches!(befunde.as_slice(), [_, b] if b.starts_with("params/70000:")),
        "{naechstes} und 70000: {befunde:?}"
    );
}
