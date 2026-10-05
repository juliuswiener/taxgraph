//! Die Scheibenlisten in `scheiben_tabellen.rs` (Felder und Kegel je Scheibe) sind seit Weg B leicht
//! (Entscheidung 2026-10-05) **von Hand gepflegte Rust-Quelle**, nicht mehr aus Python erzeugt.
//! Vorher hielt nur der Parity-Test `konstanten_gleich` (`PARITY=1`, nie in der CI) sie gegen
//! `SCHEIBEN` in Python. Diese Datei haelt die Eigenschaften fest, die ohne Python noch gelten
//! muessen:
//!
//! - Jedes Feld einer Scheibe steht in der Registry (`produkt/bindung` **und** `rust/bindung/felder`).
//!   Ein Feld, das dort fehlt, laesst `scheibe_bindung` die ganze Scheibe mit 500 abweisen.
//! - Der Kegel (die Felder, die ueber Freigabe oder Sperre des Scheibenbetrags entscheiden) liegt in
//!   den Feldern, ist ohne Dopplung und fragbar. Fehlt ein Kegel-Feld oder steht eines doppelt, zaehlt
//!   `feste_zahl` ein Pflichtfeld nicht oder zweimal -- ohne Fehlermeldung, nur im Betrag.
//! - Die Laengen stehen fest. Eine Aenderung ist gewollt, dann aendert sie auch die Zahl hier.
//!
//! Mutationsprobe (Instruktor, 2026-10-05): in `SCHEIBEN_GESAMT_KEGEL` `agb_notwendig_angemessen`
//! durch ein Duplikat von `agb_zwangslaeufig` ersetzen -- vorher in keinem Rust-Test rot, jetzt
//! `kegel_ist_dopplungsfrei_und_liegt_in_den_feldern`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::OnceLock;

use bescheid::deklaration::Cfg;
use bindung::{Bindung, Registry};
use domain::Scheibe;

/// Die vier Scheiben, die ihre Feldliste selbst tragen. `NVorGwg` liest sie aus ihrer YAML.
const MIT_LISTE: [Scheibe; 4] = [
    Scheibe::Ep,
    Scheibe::AnGesamt,
    Scheibe::Gesamt,
    Scheibe::RentnerGesamt,
];

/// `(Scheibe, Felder, Kegel)`. Die Zahlen sind der Stand vom 2026-10-05 (`9c07d98e`).
const LAENGEN: [(Scheibe, usize, usize); 4] = [
    (Scheibe::Ep, 6, 4),
    (Scheibe::AnGesamt, 84, 33),
    (Scheibe::Gesamt, 352, 35),
    (Scheibe::RentnerGesamt, 250, 28),
];

/// Felder, die in einer Liste zweimal stehen. `geburtsjahr` stand schon in `SCHEIBEN['rentner_gesamt']`
/// in Python doppelt und wurde so uebernommen. Wirkung der Dopplung: nicht gemessen; jede weitere
/// Dopplung ist ein Fehler.
const BEKANNTE_DOPPLUNG: [(Scheibe, &str); 1] = [(Scheibe::RentnerGesamt, "geburtsjahr")];

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel).expect("Registry (Python und Rust) laedt")
    })
}

fn bindungen() -> BTreeMap<&'static str, &'static Bindung> {
    registry()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .map(|b| (b.feld_id.as_str(), b))
        .collect()
}

fn felder(s: Scheibe) -> &'static [&'static str] {
    Cfg::fuer(s)
        .felder_roh()
        .unwrap_or_else(|| panic!("{s:?} traegt keine eigene Feldliste"))
}

fn kegel(s: Scheibe) -> &'static [&'static str] {
    Cfg::fuer(s)
        .kegel_roh()
        .unwrap_or_else(|| panic!("{s:?} traegt keinen eigenen Kegel"))
}

/// Felder, die in `liste` mehr als einmal stehen, sortiert.
fn doppelte(liste: &[&'static str]) -> Vec<&'static str> {
    let mut gesehen = BTreeSet::new();
    let mut doppelt = BTreeSet::new();
    for f in liste {
        if !gesehen.insert(*f) {
            doppelt.insert(*f);
        }
    }
    doppelt.into_iter().collect()
}

#[test]
fn jedes_feld_jeder_scheibe_steht_in_der_registry() {
    let alle = bindungen();
    for s in MIT_LISTE {
        let fehlt: Vec<&str> = felder(s)
            .iter()
            .copied()
            .filter(|f| !alle.contains_key(f))
            .collect();
        assert!(
            fehlt.is_empty(),
            "{s:?}: Felder ohne Bindung in produkt/bindung oder rust/bindung/felder: {fehlt:?}"
        );
    }
}

/// `n_vor_gwg` hat keine eigene Liste: ihre Felder stehen in einer YAML, ihre Teil-Ringe in der
/// Tabelle. Beides muss in der Registry stehen.
#[test]
fn n_vor_gwg_datei_und_teil_ringe_stehen_in_der_registry() {
    let cfg = Cfg::fuer(Scheibe::NVorGwg);
    assert!(cfg.felder_roh().is_none() && cfg.kegel_roh().is_none());
    let datei = cfg.felder_datei().expect("n_vor_gwg nennt ihre Datei");
    let (_, inhalt) = registry()
        .dateien
        .iter()
        .find(|(p, _)| p.file_name().is_some_and(|n| n == datei))
        .unwrap_or_else(|| panic!("{datei} fehlt in der Registry"));
    assert!(!inhalt.bindungen.is_empty(), "{datei} bindet kein Feld");
    let alle = bindungen();
    assert!(!cfg.teil_ringe().is_empty());
    for (familie, _, felder) in cfg.teil_ringe() {
        let fehlt: Vec<&&str> = felder.iter().filter(|f| !alle.contains_key(**f)).collect();
        assert!(fehlt.is_empty(), "Teil-Ring {familie}: Felder ohne Bindung: {fehlt:?}");
    }
}

#[test]
fn felder_sind_dopplungsfrei_bis_auf_die_bekannte() {
    for s in MIT_LISTE {
        let erwartet: Vec<&str> = BEKANNTE_DOPPLUNG
            .iter()
            .filter(|(b, _)| *b == s)
            .map(|(_, f)| *f)
            .collect();
        assert_eq!(doppelte(felder(s)), erwartet, "{s:?}: doppelte Felder");
    }
}

#[test]
fn kegel_ist_dopplungsfrei_und_liegt_in_den_feldern() {
    for s in MIT_LISTE {
        let k = kegel(s);
        assert_eq!(doppelte(k), Vec::<&str>::new(), "{s:?}: doppelter Kegel-Eintrag");
        let in_felder: BTreeSet<&str> = felder(s).iter().copied().collect();
        let draussen: Vec<&&str> = k.iter().filter(|f| !in_felder.contains(**f)).collect();
        assert!(draussen.is_empty(), "{s:?}: Kegel-Felder ausserhalb der Felder: {draussen:?}");
    }
}

/// Ein Kegel-Feld, das der Nutzer nicht gefragt wird, kann nie bestaetigt werden: die Scheibe bliebe
/// fuer immer gesperrt. Heute sind alle 100 Kegel-Felder fragbar.
#[test]
fn jedes_kegel_feld_ist_fragbar() {
    let alle = bindungen();
    for s in MIT_LISTE {
        let nicht: Vec<&&str> = kegel(s)
            .iter()
            .filter(|f| alle.get(**f).is_none_or(|b| !b.askable))
            .collect();
        assert!(nicht.is_empty(), "{s:?}: Kegel-Felder, die nicht askable sind: {nicht:?}");
    }
}

#[test]
fn laengen_aendern_sich_nur_mit_absicht() {
    for (s, n_felder, n_kegel) in LAENGEN {
        assert_eq!(felder(s).len(), n_felder, "{s:?}: Anzahl Felder");
        assert_eq!(kegel(s).len(), n_kegel, "{s:?}: Anzahl Kegel-Felder");
    }
}
