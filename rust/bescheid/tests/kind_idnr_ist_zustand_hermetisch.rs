//! Was der Rechenweg heute aus `kind_idnr` macht, an allen DREI Stellen, die das Feld lesen (Auftrag `IdNr` Stufe 1, Julius
//! 2026-10-06: „die Steueridentifikationsnummer auch in Rust testen“). Standardlauf, ohne Python, ohne `PARITY=1`.
//!
//! Die drei Stellen kennen je ein eigenes Praedikat „`IdNr` vorhanden“, alle gleich: Text mit mindestens 11 Zeichen
//! (Python `len(idnr) < 11`):
//! - `abzuege.rs::kind_kv_pv_summe` (§ 10 Abs. 1 Nr. 3 S. 2, Kind-KV/PV),
//! - `abzuege.rs::kind_behinderten_pb_daten` (§ 33b Abs. 5 S. 5, Kind-Pauschbetrag),
//! - `deklaration/sperre/gesamt.rs::kind_pb_uebertragen` (Sperre `behinderungsbedingte_aufwendungen_wahlrecht_offen`).
//!
//! GESICHERT (kein Ist-Zustand, jede Aenderung ist ein Fehler): 11 Ziffern zaehlen; 10 Zeichen, ein leerer Text und ein fehlendes
//! Feld zaehlen nicht.
//!
//! IST-ZUSTAND, Satz 12 nicht gebaut, Entscheidung offen: Zeichen, die KEINE `IdNr` sind, zaehlen heute als vorhanden: 12 Ziffern,
//! 11 Buchstaben, 10 Ziffern mit Zeilenumbruch (= 11 Zeichen). Der Store haelt sie am Schreibweg draussen
//! (`rust/api/tests/kind_idnr_schreibweg_hermetisch.rs`, Muster `^[0-9]{11}$`); der Rechenweg prueft nicht noch einmal, und im
//! Bestand liegen 7 bestaetigte Altwerte, die das Muster verletzen (5 zu kurz, 2 zu lang; Bericht `kind-vorarbeit.md`, 3.2).
//! Wird der Rechenweg strenger (ein gemeinsames Praedikat „genau 11 Ziffern“), werden genau diese Faelle rot: dann ist die
//! Entscheidung gefallen, und die Erwartung gehoert in dieselbe Aenderung. Rot ohne diese Entscheidung heisst: eine der drei
//! Stellen wurde allein veraendert.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::abzuege::{kind_behinderten_pb_daten, kind_kv_pv_summe};
use bescheid::deklaration::{an_gesamt_sperrgrund, Cfg};
use bescheid::testhilfe::{felder, index, store};
use bescheid::{BindungIndex, Instanzquelle};
use domain::{Scheibe, Sperrgrund, Vz};
use serde_json::{json, Value};

/// Ein Event `(feld_id, wert, bestaetigt)`.
type Ev = (&'static str, Value, bool);

/// Eine `kind_idnr` und ob sie als vorhanden zaehlt.
struct Idnr {
    name: &'static str,
    /// `None`: das Feld fehlt ganz.
    wert: Option<&'static str>,
    zaehlt: bool,
    /// `true`: Ist-Zustand ohne Entscheidung (siehe Moduldoku), `false`: gesichert.
    ist_zustand: bool,
}

const FAELLE: [Idnr; 7] = [
    Idnr {
        name: "11 Ziffern",
        wert: Some("12345678901"),
        zaehlt: true,
        ist_zustand: false,
    },
    Idnr {
        name: "10 Zeichen",
        wert: Some("1234567890"),
        zaehlt: false,
        ist_zustand: false,
    },
    Idnr {
        name: "leerer Text",
        wert: Some(""),
        zaehlt: false,
        ist_zustand: false,
    },
    Idnr {
        name: "Feld fehlt",
        wert: None,
        zaehlt: false,
        ist_zustand: false,
    },
    Idnr {
        name: "12 Ziffern",
        wert: Some("123456789012"),
        zaehlt: true,
        ist_zustand: true,
    },
    Idnr {
        name: "11 Buchstaben",
        wert: Some("abcdefghijk"),
        zaehlt: true,
        ist_zustand: true,
    },
    Idnr {
        name: "10 Ziffern und ein Zeilenumbruch",
        wert: Some("1234567890\n"),
        zaehlt: true,
        ist_zustand: true,
    },
];

/// Die Events: die `IdNr` (falls vorhanden, bestaetigt) vor den Feldern der gepruefen Stelle.
fn events(f: &Idnr, rest: Vec<Ev>) -> Vec<Ev> {
    let mut e: Vec<Ev> = f
        .wert
        .map(|w| ("kind_idnr", json!(w), true))
        .into_iter()
        .collect();
    e.extend(rest);
    e
}

/// Meldet jede Abweichung samt ihrer Art, damit eine Mutation jeden roten Fall zeigt.
fn pruefe(stelle: &str, got: impl Fn(&Idnr) -> bool) {
    let abweichend: Vec<String> = FAELLE
        .iter()
        .filter(|f| got(f) != f.zaehlt)
        .map(|f| {
            format!(
                "{stelle}, {}: {} (erwartet: {}){}",
                f.name,
                if f.zaehlt { "zaehlt nicht" } else { "zaehlt" },
                if f.zaehlt { "zaehlt" } else { "zaehlt nicht" },
                if f.ist_zustand {
                    " [IST-ZUSTAND: der Rechenweg wurde strenger, Entscheidung gefallen?]"
                } else {
                    " [GESICHERT]"
                }
            )
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "{} von {} Faellen weichen ab: {abweichend:#?}",
        abweichend.len(),
        FAELLE.len()
    );
}

/// § 10 Abs. 1 Nr. 3 S. 2: Kind-KV/PV zaehlen nur mit `IdNr`; 1.000 + 500 EUR in Cent.
#[test]
fn kind_kv_pv_zaehlt_nur_mit_mindestens_elf_zeichen() {
    pruefe("Kind-KV/PV", |f| {
        let st = store(&events(
            f,
            vec![
                ("kind_kv", json!(100_000), true),
                ("kind_pv", json!(50_000), true),
            ],
        ));
        let q = Instanzquelle {
            store: Some(&st),
            bindung: Some(index()),
            nur_bestaetigt: true,
        };
        match kind_kv_pv_summe(&q).unwrap().get() {
            150_000 => true,
            0 => false,
            andere => panic!("{}: unerwartete Summe {andere}", f.name),
        }
    });
}

/// § 33b Abs. 5 S. 5: der Kind-Pauschbetrag wird nur mit `IdNr` uebertragen (`GdB` 50, Antrag, vom Kind nicht genutzt).
#[test]
fn kind_pauschbetrag_wird_nur_mit_mindestens_elf_zeichen_uebertragen() {
    pruefe("Kind-Pauschbetrag", |f| {
        let st = store(&events(
            f,
            vec![
                ("kind_behinderten_pb_antrag", json!(true), true),
                ("kind_pb_nicht_selbst_genutzt", json!(true), true),
                ("kind_grad_der_behinderung", json!(50), true),
            ],
        ));
        let q = Instanzquelle {
            store: Some(&st),
            bindung: Some(index()),
            nur_bestaetigt: true,
        };
        match kind_behinderten_pb_daten(&q).unwrap().len() {
            1 => true,
            0 => false,
            andere => panic!("{}: {andere} Kinder", f.name),
        }
    });
}

/// Die Bindung der Scheibe `rentner_gesamt`: nur ihre Feld-Ids, wie `api._scheibe_bindung` (so in
/// `sperre_scheiben_hermetisch.rs`).
fn scheiben_index() -> BindungIndex<'static> {
    let ids = Cfg::fuer(Scheibe::RentnerGesamt)
        .felder(|_| Vec::new())
        .unwrap();
    index()
        .iter()
        .filter(|(k, _)| ids.contains(k))
        .map(|(k, b)| (k.clone(), *b))
        .collect()
}

/// Die Sperre `behinderungsbedingte_aufwendungen_wahlrecht_offen` entfaellt, wenn der Kind-Pauschbetrag uebertragen wird;
/// dazu braucht das Kind die `IdNr` (`kind_pb_uebertragen`). `GdB` 50 und Aufwendungen verlangen sonst eine Antwort auf das Wahlrecht.
#[test]
fn sperre_kind_pb_uebertragen_zaehlt_nur_mit_mindestens_elf_zeichen() {
    let index = scheiben_index();
    pruefe("Sperre Wahlrecht", |f| {
        let st = store(&events(
            f,
            vec![
                ("rentner_grad_der_behinderung", json!(50), true),
                ("behinderungsbedingte_aufwendungen", json!(100_000), true),
                ("kind_behinderten_pb_antrag", json!(true), true),
                ("kind_pb_nicht_selbst_genutzt", json!(true), true),
            ],
        ));
        let fe = felder(&st);
        let q = Instanzquelle {
            store: Some(&st),
            bindung: Some(&index),
            nur_bestaetigt: false,
        };
        let cfg = Cfg::fuer(Scheibe::RentnerGesamt);
        let grund = an_gesamt_sperrgrund(&fe, Some(&cfg), Some(Vz::Vz2025), &q)
            .unwrap()
            .map(Sperrgrund::als_str);
        match grund {
            None => true,
            Some("behinderungsbedingte_aufwendungen_wahlrecht_offen") => false,
            andere => panic!("{}: unerwarteter Sperrgrund {andere:?}", f.name),
        }
    });
}
