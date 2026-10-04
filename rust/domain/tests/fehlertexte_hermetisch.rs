//! Meldungs- und Anzeigetexte von `domain`, die kein Bestandstest festnagelt (Mutationsmessung N4d, Bericht
//! `mutation-bindung-domain`). Jeder Test steht fuer Mutanten, die im Standardlauf (ohne `PARITY=1`, ohne Python) gruen
//! blieben: ein `#[error("..")]`, ein `write!(f, "..")` oder ein Textliteral wurde durch `x` ersetzt, und die Tests
//! verglichen nur den Variantentyp (`matches!`, `is_err`, `PartialEq` auf dem Fehlerwert), nie den Text.
//!
//! * Die Texte gehen in `fehler_log.rs` und in die HTTP-Antworten; ein leerer oder falscher Text laesst den Nutzer und den
//!   Betrieb ohne Ursache zurueck. Jeder erwartete Text ist die woertliche Meldung aus dem Quelltext, hier einzeln und
//!   ausgeschrieben, damit eine Aenderung am Text eine bewusste Aenderung am Test braucht.
//! * `py_wert.rs`, `visit_map`: ein doppelter Schluessel ersetzt den Wert an der Stelle des ersten (wie Python-`dict`).
//!   Der Bestandstest hatte nur Doppelte am ersten Schluessel; die Stellenzaehlung `paare.len()` blieb ungeprueft.
//! * `wert.rs`, `Feldtyp::Text`: Text laenger als fuenf Zeichen ist gueltig.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use domain::{
    meet_herkunft, Achsenwert, Cent, Euro, FallId, FeldId, Feldtyp, Herkunft, HerkunftAlt,
    HerkunftVektor, Kz, PruefTiefe, PyFehler, PyWert, Scheibe, Signal2, Sperrgrund, VorschlagTyp,
    Vz, Wert,
};
use serde::Deserialize;

// ---- Fehlertexte ----------------------------------------------------------------------------------------------------

#[test]
fn kennungen_und_schluessel_melden_ihren_text() {
    assert_eq!(
        FallId::new("a b").unwrap_err().to_string(),
        "ungueltige fall_id (nur [A-Za-z0-9_-]{1,64})"
    );
    assert_eq!(
        "1x".parse::<FeldId>().unwrap_err().to_string(),
        "ungueltige Basis-Feld-Id \"1x\" (erwartet ^[a-z][a-z0-9_]*$ ohne __<Zahl>-Suffix)"
    );
    assert_eq!(
        "a__1".parse::<FeldId>().unwrap_err().to_string(),
        "ungueltige Instanz-Kodierung in \"a__1\" (erwartet __<n> mit n >= 2, keine fuehrende Null)"
    );
    assert_eq!(
        Kz::new("E010").unwrap_err().to_string(),
        "ungueltige Kz \"E010\" (erwartet E und sieben Ziffern 0-9)"
    );
    assert_eq!(
        "gibt_es_nicht"
            .parse::<Sperrgrund>()
            .unwrap_err()
            .to_string(),
        "unbekannter Sperrgrund \"gibt_es_nicht\""
    );
    assert_eq!(
        "x".parse::<Scheibe>().unwrap_err().to_string(),
        "unbekannte Scheibe \"x\" (erwartet ep, n_vor_gwg, an_gesamt, gesamt oder rentner_gesamt)"
    );
    assert_eq!(
        "x".parse::<VorschlagTyp>().unwrap_err().to_string(),
        "unbekannter Vorschlags-Typ \"x\" (erwartet llm, beleg, kontoauszug oder maps)"
    );
    assert_eq!(
        Vz::try_from(2023_u16).unwrap_err().to_string(),
        "kein unterstuetzter Veranlagungszeitraum: 2023"
    );
}

#[test]
fn betrag_herkunft_und_signal_melden_ihren_text() {
    assert_eq!(
        Euro::new(i64::MAX).to_cent().unwrap_err().to_string(),
        "Euro-Betrag 9223372036854775807 EUR ueberschreitet den Cent-Wertebereich"
    );
    assert_eq!(
        Achsenwert::new("").unwrap_err().to_string(),
        "ein Herkunfts-Achsenwert darf nicht leer sein"
    );
    assert_eq!(
        Signal2::new("  ").unwrap_err().to_string(),
        "signal_2 darf nicht leer sein (fail-closed: zustand=bestaetigt braucht ein Signal)"
    );
    let alt = HerkunftVektor::Alt(HerkunftAlt {
        herkunft: Achsenwert::new("vorjahr").unwrap(),
    });
    assert_eq!(
        meet_herkunft([alt]).unwrap_err().to_string(),
        "Herkunfts-Vektor in Alt-Form (ohne pruef_tiefe/haftung) kann nicht gemeinsam gemeint werden"
    );
}

#[test]
fn wert_fehler_melden_ihren_text() {
    let enum_werte = ["a".to_owned(), "b".to_owned()];
    assert_eq!(
        Wert::aus_pywert(&PyWert::Text("c".into()), Feldtyp::Enum, Some(&enum_werte))
            .unwrap_err()
            .to_string(),
        "Enum-Wert \"c\" ist keiner der zulaessigen enum_werte"
    );
    assert_eq!(
        Wert::aus_pywert(&PyWert::Text("2025-01-01".into()), Feldtyp::Datum, None)
            .unwrap_err()
            .to_string(),
        "Datum \"2025-01-01\" entspricht nicht dem Format TT.MM.JJJJ"
    );
}

// ---- Anzeige --------------------------------------------------------------------------------------------------------

#[test]
fn anzeige_von_betrag_scheibe_achsenwert_und_signal() {
    assert_eq!(Cent::new(-1500).to_string(), "-1500 Cent");
    assert_eq!(Euro::new(42).to_string(), "42 EUR");
    for (scheibe, text) in [
        (Scheibe::Ep, "ep"),
        (Scheibe::NVorGwg, "n_vor_gwg"),
        (Scheibe::AnGesamt, "an_gesamt"),
        (Scheibe::Gesamt, "gesamt"),
        (Scheibe::RentnerGesamt, "rentner_gesamt"),
    ] {
        assert_eq!(scheibe.to_string(), text);
    }
    assert_eq!(Achsenwert::new("mensch").unwrap().to_string(), "mensch");
    assert_eq!(
        Signal2::new("beweist@fam_anzahl_kinder=2")
            .unwrap()
            .to_string(),
        "beweist@fam_anzahl_kinder=2"
    );
}

// ---- PyFehler und PyWert --------------------------------------------------------------------------------------------

/// Jede Variante reicht ihren Text unveraendert durch (`#[error("{0}")]`).
#[test]
fn py_fehler_zeigt_den_text_der_variante() {
    for fehler in [
        PyFehler::TypFehler("typ".to_owned()),
        PyFehler::WertFehler("wert".to_owned()),
        PyFehler::Ueberlauf("ueberlauf".to_owned()),
        PyFehler::I64Grenze("i64".to_owned()),
        PyFehler::DezimalGrenze("dezimal".to_owned()),
    ] {
        let text = match &fehler {
            PyFehler::TypFehler(t)
            | PyFehler::WertFehler(t)
            | PyFehler::Ueberlauf(t)
            | PyFehler::I64Grenze(t)
            | PyFehler::DezimalGrenze(t) => t.clone(),
        };
        assert_eq!(fehler.to_string(), text);
    }
}

/// Die echten Erzeuger der Fehler und ihr Text, der Python-Meldung bzw. der Rust-Grenze (D4, D18) gleich.
#[test]
fn py_wert_meldet_die_texte_der_grenzen() {
    assert_eq!(
        PyWert::Gleit(f64::NAN).int().unwrap_err().to_string(),
        "cannot convert float NaN to integer"
    );
    assert_eq!(
        PyWert::Gleit(f64::INFINITY).int().unwrap_err().to_string(),
        "cannot convert float infinity to integer"
    );
    assert_eq!(
        PyWert::GrossGanz(u64::MAX).int().unwrap_err().to_string(),
        "int() liegt ausserhalb von i64"
    );
    assert_eq!(
        PyWert::Gleit(f64::NAN).dezimal().unwrap_err().to_string(),
        "Decimal kennt NaN, inf und Betraege ueber 7,9e28 nicht"
    );
    assert_eq!(
        PyWert::Gleit(1e29).dezimal().unwrap_err().to_string(),
        "Decimal kennt NaN, inf und Betraege ueber 7,9e28 nicht"
    );
}

/// `canonical_json` kennt fuer NaN und +-inf keinen Wert: ein FEHLER mit vollem Text, kein stiller `null`.
#[test]
fn zu_json_meldet_nan_und_inf_mit_vollem_text() {
    for (f, anzeige) in [
        (f64::NAN, "NaN"),
        (f64::INFINITY, "inf"),
        (f64::NEG_INFINITY, "-inf"),
    ] {
        let fehler = PyWert::Gleit(f).zu_json().unwrap_err();
        assert!(matches!(fehler, PyFehler::DezimalGrenze(_)), "{anzeige}");
        assert_eq!(
            fehler.to_string(),
            format!(
                "{anzeige} ist in JSON nicht darstellbar (NaN/inf); canonical_json kennt dafuer keinen Wert, \
                 und ein stiller null-Wert waere hash-fremd"
            )
        );
    }
}

/// Der Besucher nennt in der Fehlermeldung, was er erwartet hat: ein Byte-Feld ist kein JSON- oder YAML-Wert.
#[test]
fn besucher_nennt_den_erwarteten_wert() {
    let bytes = serde::de::value::BytesDeserializer::<serde::de::value::Error>::new(b"ab");
    assert_eq!(
        PyWert::deserialize(bytes).unwrap_err().to_string(),
        "invalid type: byte array, expected einen JSON- oder YAML-Wert"
    );
}

/// Ein doppelter Schluessel behaelt die Stelle des ersten und nimmt den Wert des letzten (Python-`dict`). Die Doppelte
/// stehen am ersten, am mittleren und am letzten Schluessel, einmal gemischt mit einem dritten Schluessel dazwischen.
#[test]
fn doppelter_schluessel_ersetzt_den_wert_an_der_stelle_des_ersten() {
    let objekt = |paare: &[(&str, i64)]| {
        PyWert::Objekt(
            paare
                .iter()
                .map(|(k, v)| ((*k).to_owned(), PyWert::Ganz(*v)))
                .collect(),
        )
    };
    for (json, soll) in [
        (r#"{"a":1,"a":2}"#, objekt(&[("a", 2)])),
        (r#"{"a":1,"b":2,"a":3}"#, objekt(&[("a", 3), ("b", 2)])),
        (r#"{"a":1,"b":2,"b":3}"#, objekt(&[("a", 1), ("b", 3)])),
        (
            r#"{"a":1,"b":2,"c":3,"b":4,"c":5,"a":6}"#,
            objekt(&[("a", 6), ("b", 4), ("c", 5)]),
        ),
        (
            r#"{"a":1,"b":2,"c":3,"c":4}"#,
            objekt(&[("a", 1), ("b", 2), ("c", 4)]),
        ),
    ] {
        let ist: PyWert = serde_json::from_str(json).unwrap();
        assert_eq!(ist, soll, "{json}");
    }
}

// ---- wert.rs --------------------------------------------------------------------------------------------------------

/// Ein Text-Wert ist ab einem Zeichen gueltig, auch laenger als fuenf Zeichen (kein Laengenfilter).
#[test]
fn text_wert_laenger_als_fuenf_zeichen_ist_gueltig() {
    for text in [
        "a",
        "12345",
        "123456",
        "Muster-Strasse 12 a",
        &"x".repeat(300),
    ] {
        assert_eq!(
            Wert::aus_pywert(&PyWert::Text(text.to_owned()), Feldtyp::Text, None),
            Ok(Wert::Text(text.to_owned())),
            "{text:?}"
        );
    }
    assert!(Wert::aus_pywert(&PyWert::Text(String::new()), Feldtyp::Text, None).is_err());
}

/// Gegenprobe zum Vektor-Test oben: der volle Vektor besteht den Meet (kein Alt-Fehler), damit der Text-Test nicht
/// zufaellig an einem Fehler allgemein haengt.
#[test]
fn voller_vektor_besteht_den_meet() {
    let voll = HerkunftVektor::Voll(Herkunft {
        herkunft: Achsenwert::new("mensch").unwrap(),
        pruef_tiefe: PruefTiefe::Amtlich,
        haftung: Achsenwert::new("nutzer").unwrap(),
    });
    assert!(meet_herkunft([voll]).is_ok());
}
