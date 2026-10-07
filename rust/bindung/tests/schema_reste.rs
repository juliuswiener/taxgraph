//! Die Regeln aus `schema.json`, die `serde` nicht ausdrueckt und die der Lader deshalb selbst
//! pruefen muss (Weg B voll, Stufe 3, W1): Fragetext ohne Gesetzeskuerzel, Mindestlaengen, nicht
//! leere `enum_werte`, genau eine Bedingung in `feld_bedingung`, Grenzen von `instanz_gruppen`,
//! Muster von `regel_bedingung.feld`, `version` mindestens 1.
//!
//! Was schiefgeht, wenn das fehlt: Ein Fragetext mit "§ 9 `EStG`" erreicht den Laien unveraendert.
//! Eine `feld_bedingung` mit `wert` und `wert_nicht` hat zwei Antworten, und eine gewinnt still.
//! Eine leere Auswahl macht eine Enum-Frage unbeantwortbar. Ein Tippfehler im `feld` einer
//! Regelbedingung greift nie. Die Python-Pruefung (`jsonschema`) faellt mit Python weg; Rust prueft
//! dieselben Regeln selbst, auf den echten Daten und auf erfundenen Fehlerfaellen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;

use bindung::{
    lade_bindung, lade_registry, lade_registry_der_wurzel, BindungDatei, BindungFehler,
    RegistryFehler,
};

const BASIS: &str = "  - feld_id: testfeld
    quelle: {regel_id: r, signatur_slot: s}
    typ: int
    askable: false
    hilfe_kurz: Tipp
    beispielwert: 1
    elster_kz: \"E0123456\"
    vz_gueltigkeit: [2025]
    anker_ref: {quelle: Q, zitatanker: Zit}
";

/// Genau 40 Zeichen: die Grenze der langen Begruendungen.
const GRUND_40: &str = "1234567890123456789012345678901234567890";
/// 39 Zeichen: eines zu wenig.
const GRUND_39: &str = "123456789012345678901234567890123456789";

/// Die Datei mit der Basis-Bindung, `zeilen` an die Bindung gehaengt (4 Zeichen eingerueckt) und
/// `abschnitte` hinter `bindungen` (Spalte 0).
fn pruefe_datei(version: u32, zeilen: &str, abschnitte: &str) -> Result<(), BindungFehler> {
    let yaml =
        format!("version: {version}\nscheibe: test\nbindungen:\n{BASIS}{zeilen}{abschnitte}");
    let datei: BindungDatei = serde_yaml_ng::from_str(&yaml).unwrap();
    datei.validieren()
}

/// Eine Pruefung, die einen Text der Laenge `n` einsetzt und das Ergebnis von `validieren` liefert.
type LaengenPruefung<'a> = &'a dyn Fn(usize) -> Result<(), BindungFehler>;

fn pruefe(zeilen: &str) -> Result<(), BindungFehler> {
    pruefe_datei(1, zeilen, "")
}

fn fehlertext(ergebnis: Result<(), BindungFehler>) -> String {
    ergebnis.unwrap_err().to_string()
}

#[test]
fn fragetext_darf_kein_gesetzeskuerzel_nennen() {
    let frage = |text: &str| pruefe(&format!("    fragetext_laie: \"{text}\"\n"));
    for gut in [
        "Wie viel hast du im Jahr gezahlt?",
        "Hast du eine Satzung des Vereins?",
        "Absolut sicher?",
        "Aufwendungen fuer die Reise?",
        "Wohnst du in Spalte 3?",
        "i. S. von Gesetz",
    ] {
        assert!(frage(gut).is_ok(), "{gut:?} ist Klartext");
    }
    for schlecht in [
        "Nach § 9 gefragt?",
        "Was sagt das EStG?",
        "Was sagt das GewStG?",
        "Was sagt das KStG?",
        "Siehe Abs. 2",
        "Kosten i.S.d. Gesetzes?",
        "Kosten i. S. d. Gesetzes?",
        "Kosten i.S. d. Gesetzes?",
        "Siehe Satz 3",
        "Siehe Satz3",
        "Aufwendungen im Jahr?",
    ] {
        let fehler = frage(schlecht).unwrap_err();
        assert!(
            matches!(fehler, BindungFehler::FragetextMitGesetzeskuerzel { .. }),
            "{schlecht:?}: {fehler}"
        );
        assert!(fehler.to_string().contains("testfeld"), "{fehler}");
    }
    // Kein Fragetext, kein Verbot.
    assert!(pruefe("").is_ok());
}

#[test]
fn hilfetext_und_zitatanker_brauchen_drei_zeichen() {
    let hilfe = |text: &str| {
        let yaml = format!(
            "version: 1\nscheibe: test\nbindungen:\n{}",
            BASIS.replace("hilfe_kurz: Tipp", &format!("hilfe_kurz: \"{text}\""))
        );
        serde_yaml_ng::from_str::<BindungDatei>(&yaml)
            .unwrap()
            .validieren()
    };
    let anker = |text: &str| {
        let yaml = format!(
            "version: 1\nscheibe: test\nbindungen:\n{}",
            BASIS.replace("zitatanker: Zit", &format!("zitatanker: \"{text}\""))
        );
        serde_yaml_ng::from_str::<BindungDatei>(&yaml)
            .unwrap()
            .validieren()
    };
    for (name, f) in [
        (
            "hilfe_kurz",
            &hilfe as &dyn Fn(&str) -> Result<(), BindungFehler>,
        ),
        ("anker_ref.zitatanker", &anker),
    ] {
        assert!(f("abc").is_ok(), "{name}: genau 3 Zeichen");
        assert!(
            f("äöü").is_ok(),
            "{name}: 3 Zeichen, 6 Bytes, Zeichen zaehlen"
        );
        for kurz in ["ab", "äö", "a", ""] {
            let fehler = f(kurz).unwrap_err();
            assert!(
                matches!(fehler, BindungFehler::ZuKurz { min: 3, .. }),
                "{name} {kurz:?}: {fehler}"
            );
            assert!(
                fehler.to_string().contains("testfeld") && fehler.to_string().contains(name),
                "{fehler}"
            );
        }
    }
}

#[test]
fn enum_werte_duerfen_nicht_leer_sein() {
    assert!(pruefe("    enum_werte: [a]\n").is_ok());
    let fehler = pruefe("    enum_werte: []\n").unwrap_err();
    assert!(
        matches!(fehler, BindungFehler::EnumWerteLeer { .. }),
        "{fehler}"
    );
    assert!(fehler.to_string().contains("testfeld"), "{fehler}");
}

#[test]
fn feld_bedingung_hat_genau_eine_bedingung_und_eine_lange_begruendung() {
    let bedingung = |wert: &str| {
        pruefe(&format!(
            "    feld_bedingung: {{feld: anderes, {wert}, grund: \"{GRUND_40}\"}}\n"
        ))
    };
    assert!(bedingung("wert: true").is_ok());
    assert!(
        bedingung("wert: false").is_ok(),
        "false ist ein gesetzter Wert"
    );
    assert!(bedingung("wert: 0").is_ok(), "0 ist ein gesetzter Wert");
    assert!(bedingung("wert_nicht: true").is_ok());
    assert!(bedingung("wert_nicht: false").is_ok());
    assert!(bedingung("alter_im_vz: 55").is_ok(), "ein Alter ist die dritte Art");
    for (name, wert) in [
        ("beide", "wert: true, wert_nicht: true"),
        ("beide, falsch", "wert: false, wert_nicht: false"),
        ("wert und Alter", "wert: true, alter_im_vz: 55"),
        ("wert_nicht und Alter", "wert_nicht: true, alter_im_vz: 55"),
        ("alle drei", "wert: true, wert_nicht: true, alter_im_vz: 55"),
    ] {
        let fehler = bedingung(wert).unwrap_err();
        assert!(
            matches!(fehler, BindungFehler::FeldBedingungNichtGenauEins { .. }),
            "{name}: {fehler}"
        );
    }
    let keine = pruefe(&format!(
        "    feld_bedingung: {{feld: anderes, grund: \"{GRUND_40}\"}}\n"
    ))
    .unwrap_err();
    assert!(
        matches!(keine, BindungFehler::FeldBedingungNichtGenauEins { .. }),
        "{keine}"
    );
    assert!(keine.to_string().contains("testfeld"), "{keine}");

    // Begruendung: 40 Zeichen reichen, 39 nicht.
    let kurz = pruefe(&format!(
        "    feld_bedingung: {{feld: anderes, wert: true, grund: \"{GRUND_39}\"}}\n"
    ))
    .unwrap_err();
    assert!(
        matches!(kurz, BindungFehler::ZuKurz { min: 40, .. }),
        "{kurz}"
    );
    assert!(kurz.to_string().contains("feld_bedingung"), "{kurz}");
}

#[test]
fn ableitung_braucht_eine_lange_begruendung() {
    let ableitung = |grund: &str| {
        pruefe(&format!(
            "    ableitung: {{aus: anderes, art: uebernahme, grund: \"{grund}\"}}\n"
        ))
    };
    assert!(ableitung(GRUND_40).is_ok());
    let fehler = ableitung(GRUND_39).unwrap_err();
    assert!(
        matches!(fehler, BindungFehler::ZuKurz { min: 40, .. }),
        "{fehler}"
    );
    assert!(fehler.to_string().contains("ableitung"), "{fehler}");
}

#[test]
fn instanz_gruppe_der_bindung_ist_ein_gueltiger_schluessel() {
    assert!(pruefe("    instanz_gruppe: gruppe_a\n").is_ok());
    for schlecht in ["Gruppe A", "Gruppe", "1gruppe", ""] {
        let fehler = pruefe(&format!("    instanz_gruppe: \"{schlecht}\"\n")).unwrap_err();
        assert!(
            matches!(fehler, BindungFehler::UngueltigeInstanzGruppe { .. }),
            "{schlecht:?}: {fehler}"
        );
    }
}

#[test]
fn luecken_und_regelbedingungen_haben_eine_begruendung_von_fuenf_zeichen() {
    let luecke = |grund: &str| {
        pruefe_datei(
            1,
            "",
            &format!("luecken:\n  - {{regel_id: r, signatur_slot: s, grund: \"{grund}\"}}\n"),
        )
    };
    assert!(luecke("12345").is_ok());
    let fehler = luecke("1234").unwrap_err();
    assert!(
        matches!(fehler, BindungFehler::ZuKurz { min: 5, .. }),
        "{fehler}"
    );
    assert!(fehler.to_string().contains("luecken"), "{fehler}");

    let bedingung = |feld: &str, grund: &str| {
        pruefe_datei(
            1,
            "",
            &format!(
                "regel_bedingungen:\n  - {{regel_id: r, feld: {feld}, wert: true, grund: \"{grund}\"}}\n"
            ),
        )
    };
    assert!(bedingung("anderes", "12345").is_ok());
    let kurz = bedingung("anderes", "1234").unwrap_err();
    assert!(
        matches!(kurz, BindungFehler::ZuKurz { min: 5, .. }),
        "{kurz}"
    );
    assert!(kurz.to_string().contains("regel_bedingungen"), "{kurz}");
    for schlecht in ["Anderes", "1anderes", "an-deres", "an deres"] {
        let fehler = bedingung(&format!("\"{schlecht}\""), "12345").unwrap_err();
        assert!(
            matches!(fehler, BindungFehler::UngueltigesRegelBedingungFeld { .. }),
            "{schlecht:?}: {fehler}"
        );
    }
}

#[test]
fn instanz_gruppen_haben_max_von_eins_bis_zwanzig_und_eine_lange_begruendung() {
    let gruppe = |max: u32, grund: &str| {
        pruefe_datei(
            1,
            "",
            &format!(
                "instanz_gruppen:\n  - {{gruppe: g, anzahl_feld: n, etikett: E, max: {max}, grund: \"{grund}\"}}\n"
            ),
        )
    };
    assert!(gruppe(1, GRUND_40).is_ok());
    assert!(gruppe(20, GRUND_40).is_ok());
    for max in [0, 21, 100] {
        let fehler = gruppe(max, GRUND_40).unwrap_err();
        assert!(
            matches!(fehler, BindungFehler::InstanzGruppeMaxAusserhalb { .. }),
            "max {max}: {fehler}"
        );
        assert!(fehler.to_string().contains("1..=20"), "{fehler}");
    }
    let kurz = gruppe(3, GRUND_39).unwrap_err();
    assert!(
        matches!(kurz, BindungFehler::ZuKurz { min: 40, .. }),
        "{kurz}"
    );
    assert!(kurz.to_string().contains("instanz_gruppen"), "{kurz}");
}

#[test]
fn themen_zuerst_haben_eine_lange_begruendung() {
    let thema = |grund: &str| {
        pruefe_datei(
            1,
            "",
            &format!("themen_zuerst:\n  - {{regel_id: r, grund: \"{grund}\"}}\n"),
        )
    };
    assert!(thema(GRUND_40).is_ok());
    let fehler = thema(GRUND_39).unwrap_err();
    assert!(
        matches!(fehler, BindungFehler::ZuKurz { min: 40, .. }),
        "{fehler}"
    );
    assert!(fehler.to_string().contains("themen_zuerst"), "{fehler}");
}

#[test]
fn version_ist_mindestens_eins() {
    assert!(pruefe_datei(1, "", "").is_ok());
    assert!(pruefe_datei(2, "", "").is_ok());
    let fehler = pruefe_datei(0, "", "").unwrap_err();
    assert!(
        matches!(fehler, BindungFehler::VersionZuKlein(0)),
        "{fehler}"
    );
    assert!(fehlertext(pruefe_datei(0, "", "")).contains("version"));
}

/// Jede Mindestlaenge an n-1, n und n+1. Die Zahlen sind
/// hier ausgeschrieben und nicht aus dem Lader gelesen: sie sind die Zahlen aus `schema.json`, und
/// ein Test, der die Konstante des Laders benutzte, bemerkte nicht, wenn sie sich verschiebt. Ein
/// Mutant `<` zu `<=`, `n` zu `n+1` oder `n-1` faellt an genau einer der drei Stellen auf.
#[test]
fn jede_mindestlaenge_haelt_an_n_minus_1_n_und_n_plus_1() {
    let x = |n: usize| "x".repeat(n);
    let hilfe = |n: usize| {
        let yaml = format!(
            "version: 1\nscheibe: test\nbindungen:\n{}",
            BASIS.replace("hilfe_kurz: Tipp", &format!("hilfe_kurz: \"{}\"", x(n)))
        );
        serde_yaml_ng::from_str::<BindungDatei>(&yaml)
            .unwrap()
            .validieren()
    };
    let anker = |n: usize| {
        let yaml = format!(
            "version: 1\nscheibe: test\nbindungen:\n{}",
            BASIS.replace("zitatanker: Zit", &format!("zitatanker: \"{}\"", x(n)))
        );
        serde_yaml_ng::from_str::<BindungDatei>(&yaml)
            .unwrap()
            .validieren()
    };
    let luecke = |n: usize| {
        pruefe_datei(
            1,
            "",
            &format!(
                "luecken:\n  - {{regel_id: r, signatur_slot: s, grund: \"{}\"}}\n",
                x(n)
            ),
        )
    };
    let regel_bedingung = |n: usize| {
        pruefe_datei(
            1,
            "",
            &format!(
                "regel_bedingungen:\n  - {{regel_id: r, feld: anderes, wert: true, grund: \"{}\"}}\n",
                x(n)
            ),
        )
    };
    let feld_bedingung = |n: usize| {
        pruefe(&format!(
            "    feld_bedingung: {{feld: anderes, wert: true, grund: \"{}\"}}\n",
            x(n)
        ))
    };
    let ableitung = |n: usize| {
        pruefe(&format!(
            "    ableitung: {{aus: anderes, art: uebernahme, grund: \"{}\"}}\n",
            x(n)
        ))
    };
    let instanz_grund = |n: usize| {
        pruefe_datei(
            1,
            "",
            &format!(
                "instanz_gruppen:\n  - {{gruppe: g, anzahl_feld: n, etikett: E, max: 3, grund: \"{}\"}}\n",
                x(n)
            ),
        )
    };
    let thema = |n: usize| {
        pruefe_datei(
            1,
            "",
            &format!("themen_zuerst:\n  - {{regel_id: r, grund: \"{}\"}}\n", x(n)),
        )
    };
    // (Name, kleinste erlaubte Laenge, Pruefung). Unter der Grenze Fehler, ab der Grenze gueltig.
    let laengen: [(&str, usize, LaengenPruefung); 8] = [
        ("hilfe_kurz", 3, &hilfe),
        ("anker_ref.zitatanker", 3, &anker),
        ("luecken.grund", 5, &luecke),
        ("regel_bedingungen.grund", 5, &regel_bedingung),
        ("feld_bedingung.grund", 40, &feld_bedingung),
        ("ableitung.grund", 40, &ableitung),
        ("instanz_gruppen.grund", 40, &instanz_grund),
        ("themen_zuerst.grund", 40, &thema),
    ];
    for (name, n, pruefung) in laengen {
        assert!(
            pruefung(n - 1).is_err(),
            "{name}: {} Zeichen sind zu kurz",
            n - 1
        );
        assert!(pruefung(n).is_ok(), "{name}: {n} Zeichen reichen");
        assert!(pruefung(n + 1).is_ok(), "{name}: {} Zeichen reichen", n + 1);
    }
}

/// `instanz_gruppen.max`: 1 bis 20, beide Raender mit ihren Nachbarn (0/1/2 und 19/20/21).
#[test]
fn instanz_gruppen_max_haelt_an_beiden_raendern() {
    let max = |m: u32| {
        pruefe_datei(
            1,
            "",
            &format!(
                "instanz_gruppen:\n  - {{gruppe: g, anzahl_feld: n, etikett: E, max: {m}, grund: \"{}\"}}\n",
                "x".repeat(40)
            ),
        )
    };
    for (m, gueltig) in [
        (0, false),
        (1, true),
        (2, true),
        (19, true),
        (20, true),
        (21, false),
    ] {
        assert_eq!(max(m).is_ok(), gueltig, "instanz_gruppen.max {m}");
    }
}

/// Der Dienst laedt ueber `lade_bindung` und `lade_registry`, nicht ueber `validieren`: auch dort
/// muessen die Regeln der Bindungen und der Abschnitte greifen.
#[test]
fn lade_bindung_und_lade_registry_pruefen_bindungen_und_abschnitte() {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("schema-reste-laden-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&ordner);
    std::fs::create_dir_all(&ordner).unwrap();
    let schreibe = |name: &str, yaml: String| {
        let pfad = ordner.join(name);
        std::fs::write(&pfad, yaml).unwrap();
        pfad
    };
    let datei = |bindung: &str, abschnitte: &str| {
        format!("version: 1\nscheibe: test\nbindungen:\n{bindung}{abschnitte}")
    };

    let gut = schreibe("bindung_gut.yaml", datei(BASIS, ""));
    assert!(lade_bindung(&gut).is_ok());

    // Abschnitt: eine zu kurze Begruendung in `luecken`.
    let luecke = schreibe(
        "bindung_luecke.yaml",
        datei(
            BASIS,
            "luecken:\n  - {regel_id: r, signatur_slot: s, grund: ab}\n",
        ),
    );
    let fehler = lade_bindung(&luecke).unwrap_err();
    assert!(
        matches!(fehler, BindungFehler::ZuKurz { min: 5, .. }),
        "{fehler}"
    );

    // Bindung: ein Fragetext mit Paragraf.
    let paragraf = schreibe(
        "bindung_paragraf.yaml",
        datei(&format!("{BASIS}    fragetext_laie: \"Nach § 9?\"\n"), ""),
    );
    let fehler = lade_bindung(&paragraf).unwrap_err();
    assert!(
        matches!(fehler, BindungFehler::FragetextMitGesetzeskuerzel { .. }),
        "{fehler}"
    );

    // Die Registry reicht den Fehler weiter und kennt die Datei nicht als gueltig.
    let fehler = lade_registry(&ordner).unwrap_err();
    assert!(matches!(fehler, RegistryFehler::Bindung(_)), "{fehler}");
}

/// Gegenprobe gegen eine Pruefung, die nichts sieht: Die echten Daten tragen jede Art, die hier
/// geprueft wird, in nennenswerter Zahl, und `lade_registry_der_wurzel` laedt sie ohne Fehler.
#[test]
fn die_echten_daten_tragen_jede_geprueften_art_und_erfuellen_die_regeln() {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let registry = lade_registry_der_wurzel(&wurzel).unwrap();
    let bindungen: Vec<_> = registry
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .collect();
    let zaehle = |f: &dyn Fn(&bindung::Bindung) -> bool| bindungen.iter().filter(|b| f(b)).count();
    let je_datei = |f: &dyn Fn(&BindungDatei) -> usize| -> usize {
        registry.dateien.iter().map(|(_, d)| f(d)).sum()
    };
    let mindest = [
        (
            "fragetext_laie",
            zaehle(&|b| b.fragetext_laie.is_some()),
            300,
        ),
        (
            "feld_bedingung",
            zaehle(&|b| b.feld_bedingung.is_some()),
            50,
        ),
        ("ableitung", zaehle(&|b| b.ableitung.is_some()), 3),
        (
            "instanz_gruppe",
            zaehle(&|b| b.instanz_gruppe.is_some()),
            60,
        ),
        ("enum_werte", zaehle(&|b| b.enum_werte.is_some()), 20),
        ("luecken", je_datei(&|d| d.luecken.len()), 100),
        (
            "regel_bedingungen",
            je_datei(&|d| d.regel_bedingungen.len()),
            30,
        ),
        ("instanz_gruppen", je_datei(&|d| d.instanz_gruppen.len()), 5),
        ("themen_zuerst", je_datei(&|d| d.themen_zuerst.len()), 1),
    ];
    for (name, ist, soll) in mindest {
        assert!(
            ist >= soll,
            "{name}: nur {ist} gefunden, erwartet mindestens {soll}"
        );
    }
    for (pfad, datei) in &registry.dateien {
        datei
            .validieren()
            .unwrap_or_else(|e| panic!("{}: {e}", pfad.display()));
    }
}
