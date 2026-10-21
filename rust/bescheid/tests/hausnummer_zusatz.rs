//! Der Hausnummerzusatz hat ein Bindungsfeld (Vault `backlog/taxgraph/hausnummer-zusatz-versprochen-
//! aber-nicht-einloesbar`, Entscheidung `hausnummer-zusatz-bekommt-ein-bindungsfeld`).
//!
//! `stammdaten_hausnummerzusatz` (Kz `E0101207`, Laenge 1-6, kein Zeilenumbruch) steht in der
//! Bindungstabelle `rust/bindung/daten/` (bei der Uebernahme eine Kopie der gemeinsamen). Doppelt
//! gefuehrt sind nur die Feldlisten der Scheiben: Rust spiegelt `STAMMDATEN_FELDER` und die
//! `felder` von `gesamt` und `rentner_gesamt` von Hand. Darum ein Test je Liste -- fehlt der Name in
//! GENAU EINER, wird genau dieser Test rot, ohne `PARITY=1` und ohne Python.
//!
//! AK1 Bindungsfeld und Listen, AK2 Deklaration, AK3 Grenzen. AK4 (der Zusatz erreicht
//! `absender_strasse`) braucht das ERiC-Schema und liegt in `einreichung_e2e.rs::hausnummer_zusatz`,
//! Byte fuer Byte gegen das XML, das Python baut. Das Python-Gegenstueck:
//! `tests/test_hausnummer_zusatz.py`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::deklaration::{einreichungs_xml, Cfg, EinreichFehler};
use bescheid::testhilfe::{felder, index, params, store};
use domain::{Achsenwert, Feldtyp, Feldzustand, Herkunft, PruefTiefe, Schreiber, Scheibe, Signal2};
use elster::deklariere;
use elster::testhilfe::schemas_da;
use serde_json::json;
use store::{Abweisung, BindungNachschlag, NeuesEvent, Store};

const FELD: &str = "stammdaten_hausnummerzusatz";
const KZ: &str = "E0101207";

// ---------------------------------------------------------------------------------------------- AK1

#[test]
fn ak1_bindungsfeld_steht_in_der_tabelle() {
    let b = index()
        .get(FELD)
        .unwrap_or_else(|| panic!("{FELD} fehlt in der Bindungstabelle"));
    assert_eq!(b.typ, Feldtyp::Text);
    assert_eq!(b.elster_kz.as_ref().map(domain::Kz::as_str), Some(KZ));
    assert_eq!(b.vorjahr, Some(bindung::Vorjahr::Uebernehmbar));
    assert!(b.askable);
    assert_eq!(b.muster.as_deref(), Some(r"^(?:[^\n\r]{1,6})$"));
}

/// `STAMMDATEN_FELDER` (`konstanten.rs`): die Felder, die `einreichen` von der Scheibe verlangt.
/// `an_gesamt` fuehrt keines davon und nennt darum alle als fehlend -- die einzige oeffentliche
/// Stelle, an der die Liste sichtbar wird.
#[test]
fn ak1_liste_stammdaten_felder_fuehrt_das_feld() {
    let datei = serde_json::from_value(
        json!({"version": 1, "veranlagungszeitraum": 2025, "scheibe": "an_gesamt", "events": []}),
    )
    .unwrap();
    let st = Store::aus_datei(datei);
    match einreichungs_xml(&st, index(), params(), "BY", None) {
        Err(EinreichFehler::ScheibeNichtAbgabefaehig {
            fehlende_stammdatenfelder,
            ..
        }) => {
            assert!(
                fehlende_stammdatenfelder.contains(&FELD),
                "{fehlende_stammdatenfelder:?}"
            );
        }
        anders => panic!("{anders:?}"),
    }
}

/// `SCHEIBEN_GESAMT_FELDER` (`scheiben_tabellen.rs`).
#[test]
fn ak1_liste_gesamt_fuehrt_das_feld() {
    let f = Cfg::fuer(Scheibe::Gesamt).felder_roh().unwrap();
    assert!(f.contains(&FELD), "SCHEIBEN_GESAMT_FELDER fuehrt {FELD} nicht");
}

/// `SCHEIBEN_RENTNER_GESAMT_FELDER` (`scheiben_tabellen.rs`).
#[test]
fn ak1_liste_rentner_gesamt_fuehrt_das_feld() {
    let f = Cfg::fuer(Scheibe::RentnerGesamt).felder_roh().unwrap();
    assert!(
        f.contains(&FELD),
        "SCHEIBEN_RENTNER_GESAMT_FELDER fuehrt {FELD} nicht"
    );
}

/// Die Gegenprobe zur ersten Liste: jede Scheibe, die eine Erklaerung tragen soll, fuehrt ALLE
/// `STAMMDATEN_FELDER`. Ein Name nur in `STAMMDATEN_FELDER` machte `gesamt` und `rentner_gesamt`
/// zu Scheiben, die `einreichen` mit 409 abweist.
#[test]
fn ak1_abgabescheiben_fuehren_alle_stammdatenfelder() {
    for scheibe in ["gesamt", "rentner_gesamt"] {
        let datei = serde_json::from_value(
            json!({"version": 1, "veranlagungszeitraum": 2025, "scheibe": scheibe, "events": []}),
        )
        .unwrap();
        let st = Store::aus_datei(datei);
        let r = einreichungs_xml(&st, index(), params(), "BY", None);
        assert!(
            !matches!(r, Err(EinreichFehler::ScheibeNichtAbgabefaehig { .. })),
            "{scheibe}: {r:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------- AK2

fn stamm(zusatz: Option<&str>) -> Vec<(&'static str, serde_json::Value, bool)> {
    let mut v = vec![
        ("stammdaten_nachname", json!("Maier"), true),
        ("stammdaten_vorname", json!("Hans"), true),
        ("stammdaten_strasse", json!("Musterstr."), true),
        ("stammdaten_hausnummer", json!("12"), true),
        ("stammdaten_plz", json!("55555"), true),
        ("stammdaten_wohnort", json!("Musterort"), true),
    ];
    if let Some(z) = zusatz {
        v.push((FELD, json!(z), true));
    }
    v
}

#[test]
fn ak2_deklaration_traegt_den_zusatz_neben_der_hausnummer() {
    let st = store(&stamm(Some("a")));
    let d = deklariere(&felder(&st), index(), 2025, None).unwrap();
    assert_eq!(d.deklaration.get("E0101206"), Some(&json!("12")));
    assert_eq!(d.deklaration.get(KZ), Some(&json!("a")));
}

/// Optional: ohne Antwort kein Kz, kein leeres Element.
#[test]
fn ak2_ohne_zusatz_traegt_die_deklaration_kein_e0101207() {
    let st = store(&stamm(None));
    let d = deklariere(&felder(&st), index(), 2025, None).unwrap();
    assert_eq!(d.deklaration.get("E0101206"), Some(&json!("12")));
    assert!(!d.deklaration.contains_key(KZ), "{:?}", d.deklaration.get(KZ));
}

// ---------------------------------------------------------------------------------------------- AK3

fn schreibe(wert: &str) -> Result<(), Abweisung> {
    schreibe_feld(FELD, wert)
}

fn schreibe_feld(feld: &str, wert: &str) -> Result<(), Abweisung> {
    schreibe_in(&mut Store::leer(2025, None), feld, wert)
}

fn schreibe_in(s: &mut Store, feld: &str, wert: &str) -> Result<(), Abweisung> {
    let bestaetigt = Feldzustand::Bestaetigt {
        signal_2: Signal2::new("klick").unwrap(),
    };
    schreibe_mit(s, feld, wert, bestaetigt)
}

fn schreibe_mit(s: &mut Store, feld: &str, wert: &str, feldzustand: Feldzustand) -> Result<(), Abweisung> {
    // Ein unbekanntes Feld liesse der Store ohne Pruefung durch: die Abweisung unten ist nur dann
    // eine des Musters, wenn das Feld in der Tabelle steht.
    assert!(index().contains_key(feld), "{feld} fehlt in der Bindungstabelle");
    let neu = NeuesEvent {
        feld_id: feld.to_owned(),
        wert: json!(wert).into(),
        feldzustand,
        herkunft: Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: Schreiber::Mensch("ui:laie".to_owned()),
        signal_1: None,
        ersetzt: None,
        ts: Some("2026-10-03T09:00:00+00:00".to_owned()),
    };
    s.append(&neu, None, BindungNachschlag::neu(index())).map(|_| ())
}

#[test]
fn ak3_ein_bis_sechs_zeichen_gehen() {
    // "Äöü ß-" sind sechs Zeichen und mehr als sechs Bytes: gezaehlt wird wie im Schema, nach Zeichen.
    for wert in ["a", "abcdef", "123456", "Äöü ß-"] {
        assert_eq!(schreibe(wert), Ok(()), "{wert:?}");
    }
}

#[test]
fn ak3_sieben_zeichen_leer_und_zeilenumbruch_gehen_nicht() {
    for wert in ["abcdefg", "a\nb", "a\rb", "a\n", "Äöü ß-1"] {
        assert!(
            matches!(schreibe(wert), Err(Abweisung::FormatInkonform { .. })),
            "{wert:?}: {:?}",
            schreibe(wert)
        );
    }
}

/// Fluss J F4: Der Zusatz ist im Schema optional (`E0101207`, `minOccurs="0"`), die leere Antwort heisst "kein Zusatz" und geht
/// durch (`leer_erlaubt`). Vorher lehnte die Typpruefung, die jedes Textfeld trifft (Python: `fail-closed (Typ)`), den leeren
/// Text ab, und wer keinen Zusatz hatte, kam nicht weiter. KONTROLLE: jedes andere Textfeld lehnt ihn weiter ab (die Hausnummer,
/// der Name), und ein nicht leerer Zusatz bleibt an das Muster `{1,6}` gebunden (`ak3_sieben_zeichen_leer_und_zeilenumbruch_...`).
#[test]
fn ak3_der_leere_zusatz_geht_durch_ein_anderer_leerer_text_nicht() {
    assert_eq!(schreibe(""), Ok(()));
    for feld in ["stammdaten_hausnummer", "stammdaten_nachname", "stammdaten_strasse"] {
        assert!(
            matches!(
                schreibe_feld(feld, ""),
                Err(Abweisung::TypInkonform { .. } | Abweisung::FormatInkonform { .. })
            ),
            "{feld}: {:?}",
            schreibe_feld(feld, "")
        );
    }
}

// ---------------------------------------------------------------------------------------------- AK2, Fluss J F4

/// Die Akte aus `fixtures/e2e/hausnummer_zusatz.json` ohne ihren Zusatz; `zusatz` und `bic` kommen ueber den Schreibweg (also
/// durch die Pruefungen des Stores) dazu.
fn akte(zusatz: Option<&str>, bic: Option<&str>) -> Store {
    let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/hausnummer_zusatz.json");
    let mut roh: serde_json::Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    roh["events"].as_array_mut().unwrap().retain(|e| e["feld_id"] != FELD);
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    for (feld, wert) in [(FELD, zusatz), ("stammdaten_bic", bic)] {
        if let Some(w) = wert {
            schreibe_in(&mut s, feld, w).unwrap_or_else(|e| panic!("{feld} {w:?}: {e:?}"));
        }
    }
    s
}

fn xml_der(s: &Store) -> Result<String, EinreichFehler> {
    einreichungs_xml(s, index(), params(), "BY", Some("74931".to_owned())).map(|e| e.xml)
}

/// "Kein Zusatz" und "kein BIC" geben im erzeugten XML kein Element, auch kein leeres: ein leeres `E0101207` verletzte das Schema.
/// Mit Schema laeuft der Writer bis zum XML (die Abgabe ist nicht gesperrt), ohne Schema endet er am Writer (`schemas_da`).
/// KONTROLLE: ein Zusatz "a" steht neben der Hausnummer, sonst belegte das Fehlen des leeren Elements nichts.
#[test]
fn ak2_kein_zusatz_und_kein_bic_geben_im_xml_kein_element() {
    let mit = xml_der(&akte(Some("a"), None));
    assert_eq!(mit.is_ok(), schemas_da(2025), "{mit:?}");
    if let Ok(xml) = mit {
        assert!(xml.contains("<E0101206>7</E0101206>") && xml.contains("<E0101207>a</E0101207>"), "der Zusatz fehlt im XML");
    }
    let leer = xml_der(&akte(Some(""), Some("")));
    assert_eq!(leer.is_ok(), schemas_da(2025), "{leer:?}");
    if let Ok(xml) = leer {
        assert!(xml.contains("<E0101206>7</E0101206>"), "die Hausnummer fehlt im XML");
        for kz in ["E0101207", "E0102201"] {
            assert!(!xml.contains(kz), "{kz} steht im XML, obwohl die Antwort leer war");
        }
    }
    // Die Deklaration (ohne Schema, ohne Writer) traegt ebenfalls kein Kz.
    let st = akte(Some(""), Some(""));
    let d = deklariere(&felder(&st), index(), 2025, None).unwrap();
    for kz in [KZ, "E0102201"] {
        assert!(!d.deklaration.contains_key(kz), "{kz}: {:?}", d.deklaration.get(kz));
    }
    assert!(d.unvollstaendig().is_empty(), "die leere Antwort sperrt: {:?}", d.unvollstaendig());
}

/// Nur die BESTAETIGTE leere Antwort heisst "kein Zusatz". Ein leerer Vorschlag (die Vorjahres-Uebernahme legt Werte als
/// `vorlaeufig` ab) bleibt wie jeder Vorschlag offen: die Erklaerung nennt ihn als unbestaetigt, statt ihn still wegzulassen.
#[test]
fn ein_leerer_vorschlag_bleibt_offen() {
    let mut st = akte(None, None);
    schreibe_mit(&mut st, FELD, "", Feldzustand::Vorlaeufig).unwrap();
    let d = deklariere(&felder(&st), index(), 2025, None).unwrap();
    assert!(d.unvollstaendig().iter().any(|e| e.feld_id == FELD), "{:?}", d.unvollstaendig());
    // KONTROLLE: dieselbe Antwort, bestaetigt, sperrt nicht.
    let st = akte(Some(""), None);
    let d = deklariere(&felder(&st), index(), 2025, None).unwrap();
    assert!(d.unvollstaendig().is_empty(), "{:?}", d.unvollstaendig());
}

/// Der BIC: leer geht durch (im Schema optional, `E0102201`), ein nicht leerer Wert bleibt streng am Schema-Muster
/// (Entscheidung `textfeld-format-aus-xsd-beim-speichern`, 2026-10-01). Der Strich, den Nutzer statt "keine" tippen, geht nicht.
#[test]
fn der_leere_bic_geht_durch_ein_nicht_leerer_bleibt_am_muster() {
    assert_eq!(schreibe_feld("stammdaten_bic", ""), Ok(()));
    for gut in ["PBNKDEFF370", "PBNKDEFF", "pbnkdeff"] {
        assert_eq!(schreibe_feld("stammdaten_bic", gut), Ok(()), "{gut:?}");
    }
    for schlecht in ["-", "keiner", "PBNKDEF", "PBNKDEFF37", "PBNK DEFF", "PBNKDEFF370X", "12345678"] {
        assert!(
            matches!(schreibe_feld("stammdaten_bic", schlecht), Err(Abweisung::FormatInkonform { .. })),
            "{schlecht:?}: {:?}",
            schreibe_feld("stammdaten_bic", schlecht)
        );
    }
}
