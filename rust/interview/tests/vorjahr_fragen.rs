//! Welche Felder aus der Vorjahres-Uebernahme (`vorjahr: uebernehmbar`) trotzdem eine Frage bleiben (Fluss J F1, 2026-10-21).
//!
//! **Worum es geht.** Die Uebernahme legt die Werte des Vorjahres als `vorlaeufig` ab und nimmt die Fragen danach aus der
//! Queue (Entscheid 2026-09-05, `vorjahr-kategorie-ohne-verhalten`). Zwei Arten Feld halten daran etwas fest, das ein Vorschlag
//! nicht ersetzt: ein Gate ueber `regel_bedingungen` (`veranlagung`: erst die bestaetigte Antwort schliesst die Regeln des
//! Ehegatten aus; ein Einzelnutzer wurde nach dem Ehegatten gefragt) und der Ausloeser einer Ableitung (`stammdaten_geburtsdatum`:
//! die Ableitung rechnet nur aus einem bestaetigten Wert; das Geburtsjahr blieb vorlaeufig und die Sperre
//! `ring_betrag_vorlaeufig` hielt die Steuerzahl zurueck). Sie bleiben eine Frage, vorbelegt mit dem Wert des Vorjahres. Alle
//! anderen `uebernehmbar`-Felder bleiben ohne Rueckfrage.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::sync::OnceLock;

use bindung::{Bindung, Registry};
use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber};
use interview::Graph;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEvent, Store};

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| interview::doctest_registry().expect("Registry laedt"))
}

fn graph() -> &'static Graph<'static> {
    static CELL: OnceLock<Graph<'static>> = OnceLock::new();
    CELL.get_or_init(|| Graph::aus_registry(registry()))
}

fn bindungen() -> &'static Vec<Bindung> {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel)
            .expect("Registry laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

/// Ein Store, in den die Vorjahres-Uebernahme die Werte als Vorschlag gelegt hat (`import:vorjahr`, `vorlaeufig`).
fn store_mit_vorjahr(felder: &[(&str, Value)]) -> Store {
    let karte: HashMap<String, &'static Bindung> = store::baue_nachschlag(bindungen());
    let mut s = Store::leer(2025, None);
    for (feld, wert) in felder {
        let neu = NeuesEvent {
            feld_id: (*feld).to_owned(),
            wert: wert.clone().into(),
            feldzustand: Feldzustand::Vorlaeufig,
            herkunft: Herkunft {
                herkunft: Achsenwert::new("vorjahr").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            },
            schreiber: Schreiber::ImportVorjahr,
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-10-21T12:00:00+00:00".to_owned()),
        };
        s.append(&neu, None, BindungNachschlag::neu(&karte)).unwrap();
    }
    s
}

fn queue(s: &Store) -> Vec<&'static str> {
    let ohne: Option<&HashMap<String, i64>> = None;
    interview::naechste_fragen(s, graph().alle(), graph(), ohne)
}

/// Die uebernommenen Felder dieses Tests: drei, die eine Frage bleiben, und drei, die es nicht tun.
fn vorjahr() -> Vec<(&'static str, Value)> {
    vec![
        ("veranlagung", json!("einzel")),
        ("stammdaten_geburtsdatum", json!("15.07.1960")),
        ("stammdaten_geburtsdatum_partner", json!("01.01.1962")),
        ("stammdaten_nachname", json!("Mustermann")),
        ("steuerklasse", json!("4")),
        ("stammdaten_plz", json!("50667")),
    ]
}
const BLEIBT_FRAGE: [&str; 3] = ["veranlagung", "stammdaten_geburtsdatum", "stammdaten_geburtsdatum_partner"];
const OHNE_RUECKFRAGE: [&str; 3] = ["stammdaten_nachname", "steuerklasse", "stammdaten_plz"];

#[test]
fn ein_gate_und_ein_ausloeser_aus_dem_vorjahr_bleiben_eine_frage_alles_andere_nicht() {
    let felder = vorjahr();
    let s = store_mit_vorjahr(&felder);
    let q = queue(&s);
    // KONTROLLE: ohne Uebernahme stehen alle sechs in der Queue; sonst waere "steht nicht darin" kein Befund.
    let leer = queue(&Store::leer(2025, None));
    for (feld, _) in &felder {
        assert!(leer.contains(feld), "{feld} steht im leeren Fall nicht in der Queue");
    }
    for feld in BLEIBT_FRAGE {
        assert!(q.contains(&feld), "{feld} ist ein Gate oder Ausloeser: die Frage bleibt, vorbelegt mit dem Wert des Vorjahres");
    }
    for feld in OHNE_RUECKFRAGE {
        assert!(!q.contains(&feld), "{feld} haelt nichts fest: der Wert aus dem Vorjahr gilt ohne Rueckfrage");
    }
}
