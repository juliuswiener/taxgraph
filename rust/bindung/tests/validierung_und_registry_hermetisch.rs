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

use bindung::{
    lade_bindung, lade_registry, BindungDatei, BindungFehler, Bindungspunkt, RegistryFehler,
};

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

// ---- vz_gueltigkeit, Quelle, Luecke, strenge Schluessel, lade_bindung -------------------------------------------------------

fn mit_vz(feld_id: &str, vz: &str) -> Result<(), BindungFehler> {
    let yaml = format!(
        "version: 1\nscheibe: test\nbindungen:\n  - feld_id: {feld_id}\n    quelle: {{regel_id: r, signatur_slot: s}}\n    typ: bool\n    \
         askable: false\n    hilfe_kurz: T\n    beispielwert: true\n    elster_kz: \"E0123456\"\n    vz_gueltigkeit: {vz}\n    \
         anker_ref: {{quelle: Q, zitatanker: Z}}\n"
    );
    let datei: BindungDatei = serde_yaml_ng::from_str(&yaml).unwrap();
    datei.bindungen[0].validieren()
}

/// Eine Bindung ohne Veranlagungszeitraum gilt fuer keinen: `vz_gueltigkeit: []` ist `LeereVzGueltigkeit` mit der `feld_id`; ein
/// Eintrag genuegt. Die Pruefung der `feld_id` kommt zuerst.
#[test]
fn leere_vz_gueltigkeit_weist_die_bindung_ab() {
    match mit_vz("testfeld", "[]") {
        Err(BindungFehler::LeereVzGueltigkeit { feld_id }) => assert_eq!(feld_id, "testfeld"),
        anderes => panic!("erwartet LeereVzGueltigkeit, bekam {anderes:?}"),
    }
    assert!(mit_vz("testfeld", "[2025]").is_ok(), "ein Eintrag");
    assert!(
        mit_vz("testfeld", "[2024, 2025, 2026]").is_ok(),
        "drei Eintraege"
    );
    assert!(
        matches!(mit_vz("Testfeld", "[]"), Err(BindungFehler::UngueltigeFeldId(f)) if f == "Testfeld"),
        "ungueltige feld_id vor leerer vz_gueltigkeit"
    );
    assert!(matches!(
        mit_vz("Testfeld", "[2025]"),
        Err(BindungFehler::UngueltigeFeldId(_))
    ));
}

/// `lade_bindung` reicht den Fehler einer ungueltigen Bindung weiter (statt ihn zu verschlucken), auch fuer die zweite Bindung der
/// Datei; `lade_registry` ebenso.
#[test]
fn lade_bindung_gibt_den_validierungsfehler_weiter() {
    let d = verzeichnis("validierung");
    let ok = gueltig("feld_a");
    let zwei = format!(
        "{ok}  - feld_id: feld_b\n    quelle: {{regel_id: r, signatur_slot: s}}\n    typ: bool\n    askable: false\n    hilfe_kurz: T\n    \
         beispielwert: true\n    elster_kz: \"E0123456\"\n    vz_gueltigkeit: []\n    anker_ref: {{quelle: Q, zitatanker: Z}}\n"
    );
    std::fs::write(d.join("bindung_ok.yaml"), &ok).unwrap();
    assert_eq!(
        lade_bindung(&d.join("bindung_ok.yaml"))
            .unwrap()
            .bindungen
            .len(),
        1
    );
    std::fs::write(d.join("bindung_zwei.yaml"), &zwei).unwrap();
    match lade_bindung(&d.join("bindung_zwei.yaml")) {
        Err(BindungFehler::LeereVzGueltigkeit { feld_id }) => assert_eq!(feld_id, "feld_b"),
        anderes => panic!("erwartet LeereVzGueltigkeit fuer feld_b, bekam {anderes:?}"),
    }
    let r = lade_registry(&d);
    assert!(
        matches!(
            r,
            Err(RegistryFehler::Bindung(
                BindungFehler::LeereVzGueltigkeit { .. }
            ))
        ),
        "Registry: {r:?}"
    );
    assert!(matches!(
        lade_bindung(&d.join("fehlt.yaml")),
        Err(BindungFehler::Io { .. })
    ));
    std::fs::write(d.join("kaputt.yaml"), "version: [\n").unwrap();
    assert!(matches!(
        lade_bindung(&d.join("kaputt.yaml")),
        Err(BindungFehler::Yaml { .. })
    ));
    std::fs::remove_dir_all(&d).unwrap();
}

/// Eine Datei mit jedem optionalen Abschnitt. Jeder Abschnitt ist `deny_unknown_fields` (das Schema ist `additionalProperties:
/// false`): ein unbekannter Schluessel ist ein Fehler und nennt den Schluessel, nie ein stilles Weglassen.
const VOLL: &str = "version: 1
scheibe: test
bindungen:
  - feld_id: testfeld
    quelle: {regel_id: r, signatur_slot: s}
    typ: int
    askable: false
    hilfe_kurz: T
    beispielwert: 1
    elster_kz: \"E0123456\"
    vz_gueltigkeit: [2025]
    anker_ref: {quelle: Q, zitatanker: Z}
    bereich: {min: 0, max: 10}
    beweist: {feld_id: anderes, wert: true}
    feld_bedingung: {feld: anderes, wert: true, grund: G}
    ableitung: {aus: anderes, art: uebernahme, grund: G}
luecken:
  - {regel_id: r, signatur_slot: s, grund: G}
regel_bedingungen:
  - {regel_id: r, feld: anderes, wert: true, grund: G}
instanz_gruppen:
  - {gruppe: g, anzahl_feld: n, etikett: E, max: 3, grund: G}
themen_zuerst:
  - {regel_id: r, grund: G}
";

#[test]
fn jeder_abschnitt_weist_unbekannte_schluessel_ab() {
    let ok: BindungDatei = serde_yaml_ng::from_str(VOLL).unwrap();
    assert!(ok.bindungen[0].validieren().is_ok());
    // (Abschnitt, Text im Original, Text mit zusaetzlichem Schluessel)
    let abschnitte = [
        ("Datei", "scheibe: test\n", "scheibe: test\nextra: 1\n"),
        ("Bindung", "typ: int\n", "typ: int\n    extra: 1\n"),
        (
            "quelle (Bindung)",
            "{regel_id: r, signatur_slot: s}\n    typ",
            "{regel_id: r, signatur_slot: s, extra: 1}\n    typ",
        ),
        ("anker_ref", "zitatanker: Z}", "zitatanker: Z, extra: 1}"),
        ("bereich", "max: 10}", "max: 10, extra: 1}"),
        (
            "beweist",
            "wert: true}\n    feld_bedingung",
            "wert: true, extra: 1}\n    feld_bedingung",
        ),
        (
            "feld_bedingung",
            "grund: G}\n    ableitung",
            "grund: G, extra: 1}\n    ableitung",
        ),
        (
            "ableitung",
            "art: uebernahme, grund: G}",
            "art: uebernahme, grund: G, extra: 1}",
        ),
        (
            "luecke",
            "signatur_slot: s, grund: G}",
            "signatur_slot: s, grund: G, extra: 1}",
        ),
        (
            "regel_bedingung",
            "feld: anderes, wert: true, grund: G}\ninstanz",
            "feld: anderes, wert: true, grund: G, extra: 1}\ninstanz",
        ),
        (
            "instanz_gruppe",
            "max: 3, grund: G}",
            "max: 3, grund: G, extra: 1}",
        ),
        (
            "thema_zuerst",
            "- {regel_id: r, grund: G}",
            "- {regel_id: r, grund: G, extra: 1}",
        ),
    ];
    for (name, alt, neu) in abschnitte {
        assert_eq!(
            VOLL.matches(alt).count(),
            1,
            "{name}: Anker nicht eindeutig"
        );
        let fehler = serde_yaml_ng::from_str::<BindungDatei>(&VOLL.replace(alt, neu))
            .unwrap_err()
            .to_string();
        assert!(fehler.contains("unknown field `extra`"), "{name}: {fehler}");
    }
}

/// `Quelle` und `Luecke`: genau EINES von `signatur_slot` / `geltungsbedingung`, und jedes landet im richtigen Bindungspunkt.
#[test]
fn quelle_und_luecke_binden_den_richtigen_punkt() {
    let slot = Bindungspunkt::SignaturSlot("s".to_owned());
    let geltung = Bindungspunkt::Geltungsbedingung("g".to_owned());
    let ok: BindungDatei = serde_yaml_ng::from_str(VOLL).unwrap();
    assert_eq!(ok.bindungen[0].quelle.bindungspunkt, slot);
    assert_eq!(ok.luecken[0].bindungspunkt, slot);
    let mit_g = VOLL
        .replace(
            "{regel_id: r, signatur_slot: s}\n    typ",
            "{regel_id: r, geltungsbedingung: g}\n    typ",
        )
        .replace(
            "{regel_id: r, signatur_slot: s, grund: G}",
            "{regel_id: r, geltungsbedingung: g, grund: G}",
        );
    let g: BindungDatei = serde_yaml_ng::from_str(&mit_g).unwrap();
    assert_eq!(g.bindungen[0].quelle.bindungspunkt, geltung);
    assert_eq!(g.luecken[0].bindungspunkt, geltung);
    assert_eq!(g.bindungen[0].quelle.regel_id, "r");
    assert_eq!(g.luecken[0].grund, "G");
    // beides oder keins: Fehler mit dem Wortlaut der Regel
    for (name, alt, beides, keins, wortlaut) in [
        (
            "Quelle",
            "{regel_id: r, signatur_slot: s}\n    typ",
            "{regel_id: r, signatur_slot: s, geltungsbedingung: g}\n    typ",
            "{regel_id: r}\n    typ",
            "quelle braucht genau eines von signatur_slot/geltungsbedingung",
        ),
        (
            "Luecke",
            "{regel_id: r, signatur_slot: s, grund: G}",
            "{regel_id: r, signatur_slot: s, geltungsbedingung: g, grund: G}",
            "{regel_id: r, grund: G}",
            "luecke braucht genau eines von signatur_slot/geltungsbedingung",
        ),
    ] {
        for (art, neu) in [("beides", beides), ("keins", keins)] {
            let fehler = serde_yaml_ng::from_str::<BindungDatei>(&VOLL.replace(alt, neu))
                .unwrap_err()
                .to_string();
            let meldung = fehler.split(" at line").next().unwrap();
            assert!(meldung.ends_with(wortlaut), "{name} {art}: {fehler}");
        }
    }
}
