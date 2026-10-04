//! `domain::zeichensatz` gegen das Python-Orakel `produkt/store/zeichensatz.py`, hermetisch (Mutationsmessung N4, Bericht
//! `mutation-bindung-domain`). Der Paritaetstest `rust/parity/tests/store_zeichensatz_paritaet.rs` prueft dasselbe live, braucht
//! aber `PARITY=1` und Python; im Standardlauf stand die Tabelle `VORSCHLAEGE`/`NAMEN`/`NAMEN_BEREICHE` ohne Test da (46 Mutanten
//! der Zuordnung und `!elster_zeichen` in `erstes_unerlaubtes_zeichen` blieben gruen).
//!
//! Das Fixture `rust/fixtures/zeichensatz_orakel.json` haelt je Codepunkt (U+0000..U+10FFFF ohne Surrogate), ob Python ihn
//! durchlaesst, wie die Meldung ihn nennt und welchen Rat sie gibt. Neu erzeugen: `python3 tools/parity/extract_zeichensatz_orakel.py`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use domain::zeichensatz::{
    element_meldung, elster_zeichen, erstes_unerlaubtes_zeichen, feld_meldung, zeichen_anzeige,
    zeichen_rat,
};
use serde_json::Value;

const FIXTURE: &str = include_str!("../../fixtures/zeichensatz_orakel.json");

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}

fn zahl(v: &Value) -> u32 {
    u32::try_from(v.as_u64().unwrap()).unwrap()
}

/// Alle Codepunkte ausser den Surrogaten.
fn alle_zeichen() -> impl Iterator<Item = char> {
    (0..=0x10_ffff).filter_map(char::from_u32)
}

#[test]
fn erlaubte_menge_wie_python_fuer_jeden_codepunkt() {
    let f = fixture();
    let bereiche: Vec<(u32, u32)> = f["erlaubt"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| (zahl(&b[0]), zahl(&b[1])))
        .collect();
    let mut erlaubt = 0_u32;
    for c in alle_zeichen() {
        let soll = bereiche
            .iter()
            .any(|(von, bis)| (*von..=*bis).contains(&u32::from(c)));
        assert_eq!(
            elster_zeichen(c),
            soll,
            "elster_zeichen(U+{:04X})",
            u32::from(c)
        );
        let s = c.to_string();
        assert_eq!(
            erstes_unerlaubtes_zeichen(&s),
            if soll { None } else { Some(c) },
            "U+{:04X}",
            u32::from(c)
        );
        erlaubt += u32::from(soll);
    }
    assert_eq!(
        erlaubt, 186,
        "Zahl der erlaubten Zeichen (Standard_E_V2 ohne LF/CR)"
    );
}

#[test]
fn anzeige_und_rat_wie_python_fuer_jeden_codepunkt() {
    let f = fixture();
    let mut naechster = 0_u32;
    let mut geprueft = 0_u32;
    for lauf in f["laeufe"].as_array().unwrap() {
        let (von, bis) = (zahl(&lauf[0]), zahl(&lauf[1]));
        let rat = lauf[3].as_str().unwrap();
        // Die Laeufe schliessen lueckenlos aneinander, nur die Surrogate fehlen.
        if naechster == 0xd800 {
            naechster = 0xe000;
        }
        assert_eq!(
            von, naechster,
            "Lauf beginnt nicht dort, wo der vorige endete"
        );
        naechster = bis + 1;
        for cp in von..=bis {
            let c = char::from_u32(cp).unwrap();
            let punkt = format!("U+{cp:04X}");
            let anzeige = match lauf[2].as_str() {
                Some(name) => format!("{name} ({punkt})"),
                None => format!("„{c}\" ({punkt})"),
            };
            assert_eq!(zeichen_anzeige(c), anzeige, "{punkt}");
            assert_eq!(zeichen_rat(c), rat, "{punkt}");
            geprueft += 1;
        }
    }
    assert_eq!(naechster, 0x11_0000, "die Laeufe enden bei U+10FFFF");
    assert_eq!(geprueft, 0x11_0000 - 0x800);
}

#[test]
fn volle_meldungen_wie_python() {
    let f = fixture();
    for p in f["feld_meldung"].as_array().unwrap() {
        let c = p[1].as_str().unwrap().chars().next().unwrap();
        assert_eq!(
            feld_meldung(p[0].as_str().unwrap(), c),
            p[2].as_str().unwrap(),
            "{p}"
        );
    }
    for p in f["element_meldung"].as_array().unwrap() {
        assert_eq!(
            element_meldung(p[0].as_str().unwrap(), p[1].as_str().unwrap()).as_deref(),
            p[2].as_str(),
            "{p}"
        );
    }
}

/// Das ERSTE unerlaubte Zeichen, nicht das erste erlaubte; ein leerer Text hat keins.
#[test]
fn erstes_unerlaubtes_zeichen_ist_das_erste_unerlaubte() {
    assert_eq!(
        erstes_unerlaubtes_zeichen("abc\u{142}d\u{2013}"),
        Some('\u{142}')
    );
    assert_eq!(erstes_unerlaubtes_zeichen("\u{2013}abc"), Some('\u{2013}'));
    assert_eq!(erstes_unerlaubtes_zeichen("Müller € ß"), None);
    assert_eq!(erstes_unerlaubtes_zeichen(""), None);
    assert_eq!(erstes_unerlaubtes_zeichen("a\nb"), Some('\n'));
}
