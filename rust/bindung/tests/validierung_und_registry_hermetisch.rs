//! Validierungsregeln von `Bindung::validieren` und Dateifilter von `lade_registry`, hermetisch. Messung N4
//! (`berichte/mutation-bindung-domain.md`): Mutanten an diesen Stellen ueberlebten die Bestandstests, weil weder die Mindestlaenge des
//! Fragetexts noch die Bool-und-askable-Regel noch der Dateifilter je mit Grenzfaellen gerufen wurden.
//!
//! Eine Bindung mit falscher Regel faellt sonst erst beim Betrieb auf: eine Frage ohne lesbaren Text, ein "invertiertes" Feld, das
//! keine Ja/Nein-Frage ist, oder eine Notizdatei, die als Bindungstabelle geladen wird.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use bindung::{lade_registry, BindungDatei, BindungFehler, RegistryFehler};

fn bindung(
    typ: &str,
    askable: bool,
    fragetext: Option<&str>,
    invertiert: bool,
) -> Result<(), BindungFehler> {
    let frage = fragetext.map_or(String::new(), |t| format!("    fragetext_laie: \"{t}\"\n"));
    let yaml = format!(
        "version: 1\nscheibe: test\nbindungen:\n  - feld_id: testfeld\n    quelle: {{regel_id: r, signatur_slot: s}}\n    typ: {typ}\n    \
         askable: {askable}\n{frage}    frage_invertiert: {invertiert}\n    hilfe_kurz: T\n    beispielwert: true\n    \
         elster_kz: \"E0123456\"\n    vz_gueltigkeit: [2025]\n    anker_ref: {{quelle: Q, zitatanker: Z}}\n"
    );
    let datei: BindungDatei = serde_yaml_ng::from_str(&yaml).unwrap();
    datei.bindungen[0].validieren()
}

#[test]
fn fragetext_braucht_mindestens_fuenf_zeichen() {
    assert!(
        bindung("bool", true, Some("abcde"), false).is_ok(),
        "genau 5 Zeichen"
    );
    assert!(
        bindung("bool", true, Some("abcdef"), false).is_ok(),
        "6 Zeichen"
    );
    assert!(
        bindung("bool", true, Some("äöüßé"), false).is_ok(),
        "5 Zeichen, 10 Bytes: Zeichen zaehlen, nicht Bytes"
    );
    for kurz in ["abcd", "äöüß", "a", ""] {
        assert!(
            matches!(
                bindung("bool", true, Some(kurz), false),
                Err(BindungFehler::AskableOhneFragetext { .. })
            ),
            "{kurz:?} ist zu kurz"
        );
    }
    assert!(
        matches!(
            bindung("bool", true, None, false),
            Err(BindungFehler::AskableOhneFragetext { .. })
        ),
        "askable ohne Fragetext"
    );
    assert!(
        bindung("bool", false, None, false).is_ok(),
        "nicht askable braucht keinen Fragetext"
    );
    assert!(
        bindung("bool", false, Some("ab"), false).is_ok(),
        "nicht askable: Laenge egal"
    );
}

#[test]
fn invertierte_frage_braucht_bool_und_askable() {
    assert!(
        bindung("bool", true, Some("Hast Du Kinder?"), true).is_ok(),
        "bool + askable"
    );
    for (typ, askable) in [("bool", false), ("int", true), ("int", false)] {
        let fragetext = askable.then_some("Wie viele Kinder?");
        assert!(
            matches!(
                bindung(typ, askable, fragetext, true),
                Err(BindungFehler::InvertiertOhneBoolAskable { .. })
            ),
            "typ={typ} askable={askable} mit frage_invertiert muss scheitern"
        );
    }
    assert!(
        bindung("int", true, Some("Wie viele Kinder?"), false).is_ok(),
        "ohne frage_invertiert gilt die Regel nicht"
    );
}

fn verzeichnis(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("bindung-registry-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn gueltig(feld_id: &str) -> String {
    format!(
        "version: 1\nscheibe: test\nbindungen:\n  - feld_id: {feld_id}\n    quelle: {{regel_id: r, signatur_slot: s}}\n    typ: bool\n    \
         askable: false\n    hilfe_kurz: T\n    beispielwert: true\n    elster_kz: \"E0123456\"\n    vz_gueltigkeit: [2025]\n    \
         anker_ref: {{quelle: Q, zitatanker: Z}}\n"
    )
}

/// Nur `bindung_*.yaml` zaehlt: Name mit Praefix UND Endung `yaml` (gleich welcher Schreibung). Alles andere ist kaputter Inhalt und
/// wuerde beim Laden scheitern, wenn der Filter es durchliesse.
#[test]
fn registry_laedt_nur_bindung_praefix_mit_yaml_endung() {
    let d = verzeichnis("filter");
    std::fs::write(d.join("bindung_a.yaml"), gueltig("feld_a")).unwrap();
    std::fs::write(d.join("bindung_b.YAML"), gueltig("feld_b")).unwrap();
    for kaputt in [
        "bindung_ohne_endung",
        "bindung_c.txt",
        "Bindung_d.yaml",
        "andere.yaml",
        "FELD_BESTAND.yaml",
        "README.md",
    ] {
        std::fs::write(d.join(kaputt), "kaputt: [\n").unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::OsStr::from_bytes(b"bindung_\xff.yaml");
        std::fs::write(d.join(name), "kaputt: [\n").unwrap();
    }
    let r = lade_registry(&d).unwrap();
    let namen: Vec<_> = r
        .dateien
        .iter()
        .map(|(p, _)| p.file_name().unwrap().to_str().unwrap().to_owned())
        .collect();
    assert_eq!(namen, ["bindung_a.yaml", "bindung_b.YAML"]);
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn registry_weist_doppelte_feld_id_ueber_dateigrenzen_ab() {
    let d = verzeichnis("doppelt");
    std::fs::write(d.join("bindung_a.yaml"), gueltig("feld_x")).unwrap();
    std::fs::write(d.join("bindung_b.yaml"), gueltig("feld_x")).unwrap();
    match lade_registry(&d).unwrap_err() {
        RegistryFehler::DoppelteFeldId {
            feld_id,
            erste,
            zweite,
        } => {
            assert_eq!(feld_id, "feld_x");
            assert_eq!(erste.file_name().unwrap(), "bindung_a.yaml");
            assert_eq!(zweite.file_name().unwrap(), "bindung_b.yaml");
        }
        anderer => panic!("falscher Fehler: {anderer}"),
    }
    std::fs::remove_dir_all(&d).unwrap();
}
