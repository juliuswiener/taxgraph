//! Regeln fuer `muster` in `Bindung::validieren` (Weg B voll, Stufe 3, Schema-Luecke "Bindung.muster
//! als Regex"): `muster` ist eine Regex, die die `regex`-Crate uebersetzt, und der `beispielwert`
//! passt dazu.
//!
//! Was schiefgeht, wenn das fehlt: `store::passt_muster` uebersetzt das Muster bei jedem Schreiben
//! neu und wertet ein Muster, das nicht kompiliert, als "passt nicht" (fail-closed). Ein Tippfehler
//! im Muster (`[0-9]{11`) oder ein Muster, das Pythons `re` kennt und die `regex`-Crate nicht
//! (Lookahead, Rueckverweis), laedt also ohne Meldung, und das Feld weist danach jeden Wert mit
//! "Format inkonform" ab. Das gilt fuer Felder, die ELSTER braucht (Identifikationsnummer, Steuernummer, Datum).
//! Zuvor fing das nur der eingefrorene Python-Vergleich in `stand_und_fragen_wie_python` auf, und
//! der entfaellt mit Stufe 2.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;

use bindung::{lade_registry_der_wurzel, BindungDatei, BindungFehler};

/// Ein Textfeld mit `muster` und `beispielwert` (beides als YAML-Skalar, schon in Anfuehrungszeichen).
fn pruefe(muster: &str, beispielwert: &str) -> Result<(), BindungFehler> {
    let yaml = format!(
        "version: 1\nscheibe: test\nbindungen:\n  - feld_id: testfeld\n    quelle: {{regel_id: r, signatur_slot: s}}\n    typ: text\n    \
         askable: false\n    hilfe_kurz: Tipp\n    beispielwert: {beispielwert}\n    muster: {muster}\n    \
         elster_kz: \"E0123456\"\n    vz_gueltigkeit: [2025]\n    anker_ref: {{quelle: Q, zitatanker: Zit}}\n"
    );
    let datei: BindungDatei = serde_yaml_ng::from_str(&yaml).unwrap();
    datei.bindungen[0].validieren()
}

#[test]
fn ein_gueltiges_muster_mit_passendem_beispielwert_ist_gueltig() {
    assert!(pruefe("'[0-9]{11}'", "'12345678901'").is_ok());
    // Ein Muster, das selbst schon `^...$` traegt (so im Bestand, hausnummer_zusatz), bleibt gueltig.
    assert!(pruefe(r"'^(?:[^\n\r]{1,6})$'", "'12a'").is_ok());
    // Ohne muster gibt es nichts zu pruefen.
    let yaml = "version: 1\nscheibe: test\nbindungen:\n  - feld_id: testfeld\n    quelle: {regel_id: r, signatur_slot: s}\n    typ: text\n    \
                askable: false\n    hilfe_kurz: Tipp\n    beispielwert: egal\n    elster_kz: \"E0123456\"\n    vz_gueltigkeit: [2025]\n    \
                anker_ref: {quelle: Q, zitatanker: Zit}\n";
    let datei: BindungDatei = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(datei.bindungen[0].muster.is_none());
    assert!(datei.bindungen[0].validieren().is_ok());
}

#[test]
fn ein_muster_das_nicht_kompiliert_wird_abgewiesen() {
    for muster in [
        "'('",          // Gruppe nie geschlossen
        "'[0-9]{11'",   // Wiederholung nie geschlossen
        "'[0-9]{11})'", // Gruppe nie geoeffnet
        "'[0-9'",       // Klasse nie geschlossen
        "'*abc'",       // nichts, was sich wiederholt
        "'(?=[0-9])x'", // Lookahead: Python ja, regex-Crate nein
        r"'(a)\1'",     // Rueckverweis: Python ja, regex-Crate nein
        "'(?<=a)b'",    // Lookbehind
        r"'abc\'",      // Backslash am Ende: maskiert das schliessende ")" des Rahmens
    ] {
        let fehler = pruefe(muster, "'x'").unwrap_err();
        assert!(
            matches!(&fehler, BindungFehler::UngueltigesMuster { feld_id, .. } if feld_id == "testfeld"),
            "{muster}: {fehler}"
        );
        assert!(
            fehler.to_string().contains("testfeld") && fehler.to_string().contains("muster"),
            "{muster}: {fehler}"
        );
    }
}

#[test]
fn das_muster_gilt_fuer_den_ganzen_wert() {
    // `re.fullmatch` in Python: Text vor oder nach dem Treffer zaehlt nicht.
    for (muster, beispiel) in [
        ("'[0-9]{11}'", "'x12345678901'"),
        ("'[0-9]{11}'", "'12345678901x'"),
        ("'[0-9]{11}'", "'123456789012'"),
        // Ein Oder auf oberster Ebene gehoert in die Rahmen-Gruppe: `^a|bc$` waere "a am Anfang
        // oder bc am Ende" und liesse `ax` durch.
        ("'a|bc'", "'ax'"),
        ("'a|bc'", "'abc'"),
        ("'a|bc'", "'xbc'"),
    ] {
        let fehler = pruefe(muster, beispiel).unwrap_err();
        assert!(
            matches!(
                &fehler,
                BindungFehler::BeispielwertPasstNichtZumMuster { .. }
            ),
            "{muster} gegen {beispiel}: {fehler}"
        );
    }
    for beispiel in ["'a'", "'bc'"] {
        assert!(pruefe("'a|bc'", beispiel).is_ok(), "{beispiel}");
    }
}

#[test]
fn der_beispielwert_muss_zum_eigenen_muster_passen() {
    let fehler = pruefe("'[0-9]{11}'", "'abc'").unwrap_err();
    assert!(
        matches!(
            &fehler,
            BindungFehler::BeispielwertPasstNichtZumMuster { feld_id, beispielwert, muster }
                if feld_id == "testfeld" && beispielwert == "abc" && muster == "[0-9]{11}"
        ),
        "{fehler}"
    );
    let text = fehler.to_string();
    assert!(
        text.contains("testfeld") && text.contains("abc") && text.contains("[0-9]{11}"),
        "{text}"
    );
    // Das Muster kommt vor dem Beispielwert: ein kaputtes Muster meldet sich als kaputtes Muster,
    // auch wenn der Beispielwert ohnehin nicht passt.
    let fehler = pruefe("'[0-9'", "'abc'").unwrap_err();
    assert!(
        matches!(&fehler, BindungFehler::UngueltigesMuster { .. }),
        "{fehler}"
    );
}

#[test]
fn ein_beispielwert_der_kein_text_ist_wird_nicht_gegen_das_muster_geprueft() {
    // Der Store wertet `muster` nur fuer Textwerte (`store.rs`: `(Some(muster), Some(s))`).
    // ponytail: ein Zahl- oder Bool-Beispiel an einem Feld mit muster ist hier kein Thema; ein
    // solches Feld gibt es nicht, und der Beispielwert-Typ gegen `typ` prueft diese Datei nicht.
    assert!(pruefe("'[0-9]{11}'", "5").is_ok());
}

/// Der Rahmen in `Bindung::validieren` muss der in `store::passt_muster` sein, sonst prueft der
/// Lader ein anderes Muster, als der Store spaeter benutzt. `bindung` kann `store` nicht aufrufen
/// (`store` haengt von `bindung` ab), also haelt dieser Test die eine Zeile dort fest.
/// ponytail: Quelltext-Pruefung statt gemeinsamer Funktion; wer `store.rs` anfassen darf, zieht
/// `passt_muster` auf eine `bindung::muster_regex` um und loescht diesen Test.
#[test]
fn der_rahmen_ist_der_des_stores() {
    let store = Path::new(env!("CARGO_MANIFEST_DIR")).join("../store/src/store.rs");
    let text = std::fs::read_to_string(&store).unwrap();
    assert!(
        text.contains(r#"regex::Regex::new(&format!("^(?:{muster})$"))"#),
        "{} bildet das Muster nicht mehr als ^(?:muster)$ ab: bindung_datei.rs::pruefe_muster muss \
         denselben Rahmen benutzen, sonst prueft der Lader ein anderes Muster als der Store",
        store.display()
    );
}

/// Gegenprobe gegen eine leere Pruefung: die echten Daten haben Muster-Felder, und jedes erfuellt
/// die Regeln (`lade_registry_der_wurzel` prueft sie beim Laden, hier ein zweites Mal ausdruecklich).
#[test]
fn die_echten_daten_haben_musterfelder_und_erfuellen_die_regeln() {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let registry = lade_registry_der_wurzel(&wurzel).unwrap();
    let mit_muster: Vec<_> = registry
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .filter(|b| b.muster.is_some())
        .collect();
    assert!(
        mit_muster.len() >= 20,
        "nur {} Musterfelder gefunden (bei Anlage 22): die Pruefung sahe sonst nichts",
        mit_muster.len()
    );
    for b in mit_muster {
        b.validieren()
            .unwrap_or_else(|e| panic!("{}: {e}", b.feld_id));
    }
}
