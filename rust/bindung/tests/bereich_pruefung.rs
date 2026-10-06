//! Regeln fuer `bereich` in `Bindung::validieren` (Weg B voll, Stufe 3, W2): nur bei cent/int, `min`
//! hoechstens `max`, ein negativer cent-Bereich braucht eine Begruendung, der `beispielwert` liegt im
//! eigenen Bereich.
//!
//! Was schiefgeht, wenn das fehlt: Der Store weist eine Zahl ausserhalb von `min..=max` ab (die 0
//! nicht). Ein verdrehter oder falscher Bereich macht ein Feld also unbenutzbar, ohne dass der
//! Dienst beim Start etwas meldet; der Nutzer sieht nur "Wert ausserhalb des Bereichs". Ein
//! `beispielwert` ausserhalb erzeugt in jedem Lauf, der ihn unbesehen uebernimmt, einen Fehler, der
//! wie ein Befund aussieht (2026-08-12: drei Tage-Felder mit `min` 0 und `beispielwert` 0, obwohl
//! ELSTER mehr als 0 verlangt).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;

use bindung::{lade_registry_der_wurzel, BindungDatei, BindungFehler};

/// Eine Bindung vom Typ `typ` mit `beispielwert` und optionalem `bereich` (YAML-Flow-Map).
fn pruefe(typ: &str, beispielwert: &str, bereich: Option<&str>) -> Result<(), BindungFehler> {
    let bereich = bereich.map_or(String::new(), |b| format!("    bereich: {b}\n"));
    let yaml = format!(
        "version: 1\nscheibe: test\nbindungen:\n  - feld_id: testfeld\n    quelle: {{regel_id: r, signatur_slot: s}}\n    typ: {typ}\n    \
         askable: false\n    hilfe_kurz: Tipp\n    beispielwert: {beispielwert}\n    enum_werte: [a]\n{bereich}    \
         elster_kz: \"E0123456\"\n    vz_gueltigkeit: [2025]\n    anker_ref: {{quelle: Q, zitatanker: Zit}}\n"
    );
    let datei: BindungDatei = serde_yaml_ng::from_str(&yaml).unwrap();
    datei.bindungen[0].validieren()
}

#[test]
fn bereich_gibt_es_nur_bei_cent_und_int() {
    let bereich = Some("{min: 0, max: 5}");
    assert!(pruefe("cent", "1", bereich).is_ok());
    assert!(pruefe("int", "1", bereich).is_ok());
    for (typ, beispiel) in [
        ("bool", "true"),
        ("enum", "a"),
        ("datum", "\"2025-01-01\""),
        ("text", "x"),
    ] {
        let fehler = pruefe(typ, beispiel, bereich).unwrap_err();
        assert!(
            matches!(fehler, BindungFehler::BereichBeiFremdemTyp { .. }),
            "typ {typ}: {fehler}"
        );
        assert!(
            fehler.to_string().contains("testfeld") && fehler.to_string().contains("cent/int"),
            "{fehler}"
        );
    }
}

#[test]
fn min_darf_nicht_ueber_max_liegen() {
    assert!(pruefe("int", "5", Some("{min: 4, max: 5}")).is_ok());
    assert!(
        pruefe("int", "5", Some("{min: 5, max: 5}")).is_ok(),
        "min gleich max"
    );
    let fehler = pruefe("int", "5", Some("{min: 6, max: 5}")).unwrap_err();
    assert!(
        matches!(
            fehler,
            BindungFehler::BereichVerdreht { min: 6, max: 5, .. }
        ),
        "{fehler}"
    );
    assert!(fehler.to_string().contains("testfeld"), "{fehler}");
}

#[test]
fn negativer_cent_bereich_braucht_eine_begruendung() {
    // Verluste: ein negativer Betrag ist hier gewollt und muss gesagt werden.
    assert!(pruefe(
        "cent",
        "0",
        Some("{min: -5, max: 5, grund: \"Verlust moeglich\"}")
    )
    .is_ok());
    for ohne in ["{min: -5, max: 5}", "{min: -5, max: 5, grund: \"\"}"] {
        let fehler = pruefe("cent", "0", Some(ohne)).unwrap_err();
        assert!(
            matches!(
                fehler,
                BindungFehler::NegativerCentBereichOhneGrund { min: -5, .. }
            ),
            "{ohne}: {fehler}"
        );
    }
    // Nur cent: ein negativer int-Bereich braucht keinen Grund, und 0 ist nicht negativ.
    assert!(pruefe("int", "0", Some("{min: -5, max: 5}")).is_ok());
    assert!(pruefe("cent", "0", Some("{min: 0, max: 5}")).is_ok());
}

#[test]
fn beispielwert_liegt_im_eigenen_bereich() {
    let bereich = Some("{min: 1, max: 5}");
    for innen in ["1", "3", "5", "4.5"] {
        assert!(pruefe("int", innen, bereich).is_ok(), "{innen}");
    }
    for aussen in ["0", "6", "5.5", "0.5", "-3"] {
        let fehler = pruefe("int", aussen, bereich).unwrap_err();
        assert!(
            matches!(
                fehler,
                BindungFehler::BeispielwertAusserhalbBereich { min: 1, max: 5, .. }
            ),
            "{aussen}: {fehler}"
        );
        assert!(fehler.to_string().contains(aussen), "{fehler}");
    }
    // Negativer Bereich: die Grenzen gelten mit Vorzeichen.
    let negativ = Some("{min: -5, max: -1, grund: \"Verlust\"}");
    assert!(pruefe("cent", "-3", negativ).is_ok());
    assert!(pruefe("cent", "0", negativ).is_err());
    assert!(pruefe("cent", "-6", negativ).is_err());
}

#[test]
fn nicht_numerischer_beispielwert_und_fehlender_bereich_werden_nicht_geprueft() {
    // Ein Bool oder Text als beispielwert an einem Zahlfeld ist hier kein Bereichsthema.
    assert!(pruefe("int", "true", Some("{min: 1, max: 5}")).is_ok());
    assert!(pruefe("int", "x", Some("{min: 1, max: 5}")).is_ok());
    // Ohne bereich gibt es nichts zu pruefen.
    assert!(pruefe("int", "999999", None).is_ok());
    assert!(pruefe("bool", "true", None).is_ok());
}

/// Gegenprobe gegen eine leere Pruefung: die echten Daten haben Bereichsfelder, und jedes erfuellt
/// die Regeln (`lade_registry_der_wurzel` prueft sie beim Laden, hier ein zweites Mal ausdruecklich).
#[test]
fn die_echten_daten_haben_bereichsfelder_und_erfuellen_die_regeln() {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let registry = lade_registry_der_wurzel(&wurzel).unwrap();
    let mit_bereich: Vec<_> = registry
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .filter(|b| b.bereich.is_some())
        .collect();
    assert!(
        mit_bereich.len() >= 40,
        "nur {} Bereichsfelder gefunden (bei Anlage 42): die Pruefung sahe sonst nichts",
        mit_bereich.len()
    );
    for b in mit_bereich {
        b.validieren()
            .unwrap_or_else(|e| panic!("{}: {e}", b.feld_id));
    }
}
