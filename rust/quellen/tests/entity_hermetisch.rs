//! `quellen::entity::dekodiere`: dieselben Zeichen wie `html.unescape`, aber streng. Wo Python still etwas anderes tut
//! (unbekannter Name stehen lassen, `&sect` ohne Semikolon deuten, unzulässige Zahl durch U+FFFD ersetzen), ist es hier ein
//! Fehler mit Namen: ein stilles Weglassen verändert einen Rechtstext unbemerkt.
//!
//! Die Gleichheit mit Python für alle 253 Namen der Tabelle und für Zahlenreferenzen steht in den Golden-Fällen
//! `entities_alle` und `entities_vielfalt` (`einfrieren_golden.rs`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::explicit_write)]

use quellen::entity::dekodiere;
use quellen::entity_tabelle::BENANNT;
use quellen::Fehler;

#[test]
fn die_tabelle_ist_nach_bytes_sortiert_ohne_doppelte_und_hat_253_namen() {
    assert_eq!(BENANNT.len(), 253);
    for paar in BENANNT.windows(2) {
        assert!(paar[0].0.as_bytes() < paar[1].0.as_bytes(), "{} steht vor {}", paar[0].0, paar[1].0);
    }
}

#[test]
fn jeder_name_der_tabelle_wird_zu_seinem_zeichen() {
    for (name, zeichen) in BENANNT {
        assert_eq!(dekodiere(&format!("&{name};")).unwrap(), zeichen.to_string(), "&{name};");
    }
}

#[test]
fn die_namen_unterscheiden_gross_und_klein() {
    assert_eq!(dekodiere("&Auml;&auml;").unwrap(), "Ää");
    assert!(matches!(dekodiere("&AUML;"), Err(Fehler::UnbekannteEntity { name }) if name == "AUML"));
}

#[test]
fn zahlenreferenzen_dezimal_und_hex_in_beiden_schreibweisen() {
    assert_eq!(dekodiere("&#65;&#228;&#8364;&#128512;").unwrap(), "Aä€😀");
    assert_eq!(dekodiere("&#x41;&#xE4;&#X20AC;&#x1F600;&#xa0;").unwrap(), "Aä€😀\u{a0}");
    assert_eq!(dekodiere("&#9;&#10;&#12;&#13;&#32;").unwrap(), "\t\n\x0c\r ");
}

#[test]
fn ein_durchgang_nicht_zwei() {
    assert_eq!(dekodiere("&amp;nbsp;").unwrap(), "&nbsp;");
    assert_eq!(dekodiere("&amp;amp;").unwrap(), "&amp;");
    assert_eq!(dekodiere("&amp;#65;").unwrap(), "&#65;");
}

#[test]
fn ein_kaufmannsund_ohne_buchstaben_oder_raute_dahinter_bleibt_stehen() {
    // auch `&#` ohne Ziffer und `&#x` ohne Hexziffer: Python lässt sie stehen
    for s in ["a & b", "&", "&&", "&;", "& ", "a&", "&1", "&\n", "&<", "&-", "&#;", "&# 5;", "&#x;", "&#xZZ;", "&#-5;"] {
        assert_eq!(dekodiere(s).unwrap(), s, "{s:?}");
    }
    assert_eq!(dekodiere("&&amp;").unwrap(), "&&");
}

#[test]
fn eine_unbekannte_entity_ist_ein_fehler_mit_dem_namen() {
    match dekodiere("Text &foo; Ende") {
        Err(Fehler::UnbekannteEntity { name }) => assert_eq!(name, "foo"),
        anderes => panic!("{anderes:?}"),
    }
    let text = dekodiere("&gibtesnicht;").unwrap_err().to_string();
    assert!(text.contains("&gibtesnicht;") && text.contains("nichts geschrieben"), "{text}");
    // HTML-5-Namen, die der HTML-4-Satz nicht hat
    for n in ["NewLine", "lbrack", "colon", "excl", "nbsp1"] {
        assert!(matches!(dekodiere(&format!("&{n};")), Err(Fehler::UnbekannteEntity { .. })), "{n}");
    }
}

#[test]
fn ohne_semikolon_ist_ein_fehler_auch_bei_bekanntem_namen() {
    // Python deutet `&sect` ohne Semikolon als § (Altlast von HTML 4); hier ist es nicht eindeutig.
    for (s, name) in [("&sect 9", "sect"), ("&sect9", "sect9"), ("AT&T", "T"), ("&amp", "amp"), ("&#65", "#65"), ("&#x41 ", "#x41")] {
        match dekodiere(s) {
            Err(Fehler::EntityOhneSemikolon { name: n }) => assert_eq!(n, name, "{s:?}"),
            anderes => panic!("{s:?}: {anderes:?}"),
        }
    }
}

#[test]
fn unzulaessige_zahlenreferenzen_sind_ein_fehler() {
    // Python ersetzt oder löscht sie still (U+FFFD, cp1252, leer): hier ein Fehler mit der Referenz im Text.
    for s in [
        "&#0;", "&#1;", "&#8;", "&#11;", "&#14;", "&#31;", "&#127;", "&#128;", "&#150;", "&#159;", "&#xD800;", "&#xDFFF;",
        "&#x110000;", "&#xFFFE;", "&#xFFFF;", "&#xFDD0;", "&#xFDEF;", "&#x1FFFE;", "&#x10FFFF;", "&#99999999999999999999;",
    ] {
        match dekodiere(s) {
            Err(Fehler::UnzulaessigeZeichenreferenz { text }) => assert_eq!(text, s),
            anderes => panic!("{s:?}: {anderes:?}"),
        }
    }
}

#[test]
fn die_grenzen_der_erlaubten_zahlen() {
    for (s, z) in [("&#160;", "\u{a0}"), ("&#55295;", "\u{d7ff}"), ("&#57344;", "\u{e000}"), ("&#64975;", "\u{fdcf}"), ("&#1114109;", "\u{10fffd}")] {
        assert_eq!(dekodiere(s).unwrap(), z, "{s}");
    }
}
