//! Auflage Z (Zeichensatz): ein Textfeld nimmt nur an, was ELSTER annimmt (Ticket
//! `elster-zeichensatz-strenger-als-xml`, Vault `decisions/elster-zeichensatz-beim-speichern-abweisen`).
//! Alles geht durch die ECHTE Bindung und `Store::append`, den Weg, den jeder Schreiber nimmt.
//! Python-Gegenstueck: `tests/test_store_zeichensatz.py`; die Paritaet beider steht in
//! `rust/parity/tests/store_zeichensatz_paritaet.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
use serde_json::{json, Value};
use store::{Abweisung, BindungNachschlag, NeuesEvent, Store};

fn nachschlag() -> BindungNachschlag<'static> {
    static BINDUNGEN: OnceLock<Vec<Bindung>> = OnceLock::new();
    static MAP: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    BindungNachschlag::neu(MAP.get_or_init(|| {
        store::baue_nachschlag(BINDUNGEN.get_or_init(|| {
            let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
            bindung::lade_registry(&pfad)
                .unwrap()
                .dateien
                .into_iter()
                .flat_map(|(_, d)| d.bindungen)
                .collect()
        }))
    }))
}

fn mensch(feld_id: &str, wert: &Value) -> NeuesEvent {
    NeuesEvent {
        feld_id: feld_id.to_string(),
        wert: wert.clone().into(),
        feldzustand: Feldzustand::Bestaetigt {
            signal_2: Signal2::new("klick").unwrap(),
        },
        herkunft: Herkunft {
            herkunft: Achsenwert::new("mensch").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: Schreiber::Mensch("julius".to_string()),
        signal_1: None,
        ersetzt: None,
        ts: Some("2026-01-01T00:00:00+00:00".to_string()),
    }
}

/// Legt `wert` in `feld` ab, in einem leeren Store; Ok(Store) oder die Abweisung samt Store.
fn speichere(feld: &str, wert: &str) -> (Store, Result<store::EventId, Abweisung>) {
    let mut s = Store::leer(2025, None);
    let r = s.append(&mensch(feld, &json!(wert)), None, nachschlag());
    (s, r)
}

/// Nach AK1/AK2: (Bezeichnung, Wert, was die Meldung vom Zeichen nennen muss, was vom Vorschlag).
const VERBOTEN: &[(&str, &str, &str, &str)] = &[
    (
        "geschuetztes Leerzeichen",
        "Kowalski\u{a0}Anna",
        "geschütztes Leerzeichen (U+00A0)",
        "ein normales Leerzeichen",
    ),
    (
        "Gedankenstrich",
        "Müller\u{2013}Straße",
        "„\u{2013}\" (U+2013)",
        "„-\"",
    ),
    (
        "l mit Strich",
        "Wa\u{142}esa",
        "„\u{142}\" (U+0142)",
        "„l\"",
    ),
    (
        "grosses L mit Strich",
        "\u{141}ukasz",
        "„\u{141}\" (U+0141)",
        "„L\"",
    ),
    (
        "Tabulator",
        "Maier\tMüller",
        "Tabulator (U+0009)",
        "ein normales Leerzeichen",
    ),
    (
        "Zeilenumbruch",
        "Maier\nMüller",
        "Zeilenumbruch (U+000A)",
        "ein normales Leerzeichen",
    ),
    (
        "Wagenruecklauf",
        "Maier\rMüller",
        "Zeilenumbruch (U+000D)",
        "ein normales Leerzeichen",
    ),
    (
        "Anfuehrungszeichen unten",
        "\u{201e}Müller",
        "„\u{201e}\" (U+201E)",
        "„\"\"",
    ),
    (
        "typografischer Apostroph",
        "O\u{2019}Brien",
        "„\u{2019}\" (U+2019)",
        "„'\"",
    ),
    (
        "Auslassungspunkte",
        "Fortsetzung\u{2026}",
        "„\u{2026}\" (U+2026)",
        "„...\"",
    ),
    (
        "Breite-null-Leerzeichen",
        "Mül\u{200b}ler",
        "Leerzeichen der Breite null (U+200B)",
        "lösche es",
    ),
    (
        "loser Akzent (macOS)",
        "Mu\u{308}ller",
        "loser Akzent (U+0308)",
        "als ein Zeichen",
    ),
    (
        "kyrillisch (allgemeiner Rat)",
        "\u{41f}етр",
        "„\u{41f}\" (U+041F)",
        "ersetze es durch ein Zeichen",
    ),
    (
        "DEL (Steuerzeichen)",
        "Maier\u{7f}Müller",
        "Steuerzeichen (U+007F)",
        "ersetze es durch ein Zeichen",
    ),
    (
        "NEL (C1-Steuerzeichen)",
        "Maier\u{85}Müller",
        "Steuerzeichen (U+0085)",
        "ein normales Leerzeichen",
    ),
    (
        "Zeilentrenner",
        "Maier\u{2028}Müller",
        "unsichtbares Zeichen (U+2028)",
        "ein normales Leerzeichen",
    ),
    (
        "Leerzeichen anderer Breite",
        "Maier\u{2003}Müller",
        "Leerzeichen besonderer Breite (U+2003)",
        "ein normales Leerzeichen",
    ),
];

#[test]
fn unerlaubtes_zeichen_wird_abgewiesen_mit_zeichen_und_vorschlag() {
    for (name, wert, zeichen, vorschlag) in VERBOTEN {
        let (s, r) = speichere("stammdaten_nachname", wert);
        let fehler = r.expect_err(name);
        assert!(
            matches!(&fehler, Abweisung::ZeichensatzVerletzt { feld_id, .. } if feld_id == "stammdaten_nachname"),
            "{name}: {fehler:?}"
        );
        let meldung = fehler.to_string();
        assert!(
            meldung.starts_with("fail-closed (Zeichensatz): stammdaten_nachname enthält"),
            "{name}: {meldung}"
        );
        assert!(
            meldung.contains(zeichen),
            "{name}: das Zeichen fehlt: {meldung}"
        );
        assert!(
            meldung.contains(vorschlag),
            "{name}: der Vorschlag fehlt: {meldung}"
        );
        for teil in ["Müller", "Maier", "Kowalski", "Brien", "ukasz", "esa"] {
            assert!(
                !meldung.contains(teil),
                "{name}: die Meldung nennt den Wert: {meldung}"
            );
        }
        assert!(
            s.events().is_empty(),
            "{name}: ein abgewiesener Wert darf nichts schreiben"
        );
    }
}

#[test]
fn folgt_der_nutzer_dem_vorschlag_geht_der_name_durch() {
    let (_, r) = speichere("stammdaten_nachname", "Wa\u{142}esa");
    assert!(r.unwrap_err().to_string().contains("„l\""));
    let (s, r) = speichere("stammdaten_nachname", "Walesa");
    r.unwrap();
    assert_eq!(s.events().len(), 1);
}

/// Reihenfolge der Auflagen: ein NUL ist ein Steuerzeichen (Auflage T) und behaelt seine Meldung;
/// ein Muster-Feld mit Zeilenende bleibt eine Format-Abweisung (Auflage F).
#[test]
fn bestehende_abweisungen_behalten_ihre_klasse() {
    let (_, r) = speichere("stammdaten_nachname", "Maier\u{0}");
    assert!(matches!(r.unwrap_err(), Abweisung::TypInkonform { .. }));
    let (_, r) = speichere("kind_idnr", "12345678901\n");
    assert!(matches!(r.unwrap_err(), Abweisung::FormatInkonform { .. }));
}

/// AK3, die Gegenrichtung: Umlaute, ß, € und die uebrigen Zeichen der Menge gehen durch.
/// Umlaute und ß liegen in U+00BF..U+00FF, nicht im ASCII-Block; € in U+20AC.
const ERLAUBT: &[&str] = &[
    "Müller",
    "Größe-Öl Ärger Übel",
    "ÄÖÜäöüß",
    "Straße 5",
    "5 €",
    "Žilina Šmíd",
    "Œuvre Ÿ œ š ž",
    "O'Brien \"Zitat\"",
    "§ 35a Abs. 2",
    "¡Hola! ¿Qué?",
    "ç à é è ñ ø å æ ð þ",
    "~ und ` und ^",
];

#[test]
fn erlaubte_zeichen_gehen_durch() {
    for wert in ERLAUBT {
        let (s, r) = speichere("stammdaten_nachname", wert);
        r.unwrap_or_else(|e| panic!("{wert:?}: {e}"));
        assert_eq!(s.events().len(), 1, "{wert:?}");
    }
    let lang = "a".repeat(300);
    speichere("stammdaten_nachname", &lang).1.unwrap();
}

#[test]
fn jedes_erlaubte_zeichen_geht_einzeln_durch_und_kein_anderes() {
    let mut erlaubt = 0;
    for cp in 0..=0x2FFFu32 {
        let Some(c) = char::from_u32(cp) else {
            continue;
        };
        let (_, r) = speichere("stammdaten_nachname", &format!("a{c}b"));
        let soll = domain::zeichensatz::elster_zeichen(c);
        match r {
            Ok(_) => {
                assert!(
                    soll,
                    "U+{cp:04X} geht durch, ist aber nicht im ELSTER-Zeichensatz"
                );
                erlaubt += 1;
            }
            // U+0000..U+001F ohne Tab/LF/CR meldet Auflage T (Steuerzeichen), der Rest Auflage Z.
            Err(Abweisung::ZeichensatzVerletzt { .. } | Abweisung::TypInkonform { .. }) => {
                assert!(
                    !soll,
                    "U+{cp:04X} ist im ELSTER-Zeichensatz, wird aber abgewiesen"
                );
            }
            Err(e) => panic!("U+{cp:04X}: unerwartete Abweisung {e:?}"),
        }
    }
    // 186 = Standard_E_V2 ohne Zeilenumbruch (StringZUBaseCType minus LF/CR), U+0020..U+2FFF.
    assert_eq!(erlaubt, 186);
}

/// Die Regel gilt fuer `typ: text`; eine Zahl, ein Wahrheitswert, eine Auswahl haben ihre eigenen
/// Pruefungen und gehen an Auflage Z vorbei.
#[test]
fn andere_feldtypen_sind_unberuehrt() {
    let mut s = Store::leer(2025, None);
    s.append(
        &mensch("bruttoarbeitslohn", &json!(5_000_000)),
        None,
        nachschlag(),
    )
    .unwrap();
    s.append(&mensch("veranlagung", &json!("einzel")), None, nachschlag())
        .unwrap();
}

/// Ein Feld ohne Bindung (Altbestand, Test mit absichtlich minimalem Setup) bleibt durchgelassen:
/// Auflage Z haengt an der Bindung, wie T, V, W und F.
#[test]
fn ohne_bindung_prueft_auflage_z_nicht() {
    let leer = HashMap::new();
    let mut s = Store::leer(2025, None);
    s.append(
        &mensch("stammdaten_nachname", &json!("Wa\u{142}esa\u{a0}")),
        None,
        BindungNachschlag::neu(&leer),
    )
    .unwrap();
}
