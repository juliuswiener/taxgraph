//! **Pin, kein Fix.** Haelt fest, was heute gilt: ein LEERER Kegel auf `rentner_gesamt`
//! liefert eine **Zahl** (`Cent(0)`) statt eines Grundes, waehrend die vier anderen Scheiben
//! abbrechen. Der Befund ist gemessen, aber im Betrieb **nicht erreichbar** -- deshalb wird
//! er nicht repariert, sondern festgehalten.
//!
//! Zwei Riegel greifen bei einem leeren Kegel nicht:
//!
//! - `meet_zustand([])` ist `Bestaetigt` ("leeres Aggregat = neutral", `store.py:56`).
//! - `len(zustaende) < len(kegel)` ist `0 < 0` = **falsch**.
//!
//! Der Rust-Port ist an dieser Stelle **treu**: Python liefert an derselben Stelle
//! `(0, 0, extras)`, live gemessen. Kein Portfehler -- ein Befund der Python-Seite.
//!
//! # Wie tief der Kegel mindestens belegt sein muss
//!
//! `rentner_gesamt` traegt **28** Kegel-Felder aus 12 Regeln. Er wird leer, wenn
//! [`interview::relevante_kegel_felder`] **alle** Felder streicht -- das tut es nur fuer
//! Felder, deren Regel der Nutzer **bestaetigt abbestellt** hat. Erschoepfend gesucht ueber
//! alle Felder, die eine Kegel-Regel ausschliessen koennen (Gates plus `regel_bedingungen`),
//! bleibt als Minimum **15 von 28** uebrig:
//!
//! | Scheibe | Kegel | ausschliessbare Felder | Minimum belegt |
//! |---|---|---|---|
//! | `ep` | 4 | 0 | **4** |
//! | `an_gesamt` | 33 | 20 | **18** |
//! | `gesamt` | 35 | 19 | **20** |
//! | `rentner_gesamt` | 28 | 23 | **15** |
//!
//! Die Suche lief ueber **alle** Kombinationen von hoechstens zwei Schaltern -- nicht ueber
//! alle Teilmengen (2^23 waere zu viel). Die 15 ist also eine **Schranke innerhalb dieses
//! Suchraums**, kein Beweis fuer beliebig viele Schalter. Die Zahl steht hier als Prosa; die
//! ausfuehrbare Zusicherung dieses Tests ist der **Nenner** (28), nicht der Zaehler.
//!
//! Wird dieser Test rot, weil der leere Kegel DOCH erreichbar ist, ist **das** der Fund --
//! dann gehoert er gemeldet, nicht wegglaettet.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::OnceLock;

use bescheid::deklaration::{feste_zahl, Cfg};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Umgebung;
use domain::{Scheibe, Vz};
use interview::Graph;

fn graph() -> &'static Graph<'static> {
    static CELL: OnceLock<Graph<'static>> = OnceLock::new();
    static REG: OnceLock<bindung::Registry> = OnceLock::new();
    CELL.get_or_init(|| {
        let reg = REG.get_or_init(|| interview::doctest_registry().unwrap());
        Graph::aus_registry(reg)
    })
}

fn umgebung() -> Umgebung<'static> {
    Umgebung {
        achsen: &[],
        index: index(),
        params: params(),
    }
}

/// Der leere Kegel, alle fuenf Scheiben -- je eine Zeile mit ihrem Ausgang.
///
/// Vier brechen ab, eine gibt eine Zahl frei. Der Ausgang wird hier auf eine von drei
/// Formen gebracht (`Zahl`, `Grund`, `SlotFehlt`), damit die Tabelle lesbar bleibt.
#[test]
fn leerer_kegel_je_scheibe_heute() {
    let leer = felder(&store(&[]));
    let g = Some(graph());

    // Der Nenner, ausfuehrbar: 28 Kegel-Felder traegt `rentner_gesamt` (die 15 oben nicht).
    assert_eq!(
        Cfg::fuer(Scheibe::RentnerGesamt).kegel_roh().unwrap().len(),
        28,
        "rentner_gesamt: Kegel-Felder -- die Zahl im Modul-Docstring haengt daran"
    );

    let ausgang = |s: Scheibe| match feste_zahl(
        &leer,
        &Cfg::fuer(s),
        Vz::Vz2025,
        &[],
        &umgebung(),
        None,
        g,
    ) {
        Ok(Ok(_)) => "Zahl",
        Ok(Err(_)) => "Grund",
        Err(bescheid::BescheidFehler::SlotFehlt(_)) => "SlotFehlt",
        Err(_) => "Fehler",
    };

    // Die vier, die abbrechen -- fail-closed.
    assert_eq!(ausgang(Scheibe::Ep), "SlotFehlt", "ep");
    assert_eq!(ausgang(Scheibe::AnGesamt), "SlotFehlt", "an_gesamt");
    assert_eq!(ausgang(Scheibe::Gesamt), "SlotFehlt", "gesamt");
    assert_eq!(ausgang(Scheibe::NVorGwg), "Grund", "n_vor_gwg");

    // Und die eine, die das NICHT tut. Steht hier, damit die Abweichung sichtbar ist.
    assert_eq!(
        ausgang(Scheibe::RentnerGesamt),
        "Zahl",
        "rentner_gesamt -- der Befund. Bricht das hier mit \"SlotFehlt\" ab, ist der \
         leere Kegel nicht mehr der Sonderfall und dieser Pin gehoert angepasst"
    );
}

/// Die rentner-Zahl einzeln -- mit dem Wert, nicht nur der Form.
///
/// `Cent(0)` ist der Kern des Befunds: eine festgesetzte Steuer von null aus **gar keiner**
/// Eingabe. Python liefert an derselben Stelle `(0, 0, ...)`.
#[test]
fn rentner_gesamt_mit_leerem_kegel_gibt_cent_null_frei() {
    let r = feste_zahl(
        &felder(&store(&[])),
        &Cfg::fuer(Scheibe::RentnerGesamt),
        Vz::Vz2025,
        &[],
        &umgebung(),
        None,
        Some(graph()),
    );
    match r {
        Ok(Ok(z)) => assert_eq!(
            z.zahl,
            domain::Cent::new(0),
            "rentner_gesamt mit leerem Kegel -- Python liefert (0, 0, extras)"
        ),
        other => panic!("rentner_gesamt: erwartet wurde Cent(0), war {other:?}"),
    }
}
