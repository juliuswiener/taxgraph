//! Der BIC steht hinter der IBAN und nur bei einer IBAN, die nicht mit DE beginnt (Fluss J F4, 2026-10-21).
//!
//! **Worum es geht.** Das Schema kennt den BIC (`E0102201`, `minOccurs="0"`) nur "zur IBAN des auslaendischen Geldinstituts"; zum
//! deutschen Konto (`E0102102`) gibt es keinen. Trotzdem fragte die Oberflaeche jeden Nutzer nach dem BIC, und das VOR der IBAN:
//! wer keinen kannte, kam nicht weiter. Jetzt haengt das Feld mit `feld_bedingung: {feld: stammdaten_iban, praefix_nicht: DE}`
//! an der IBAN, faellt bei einer deutschen weg und bleibt bei jeder anderen oder noch nicht beantworteten stehen (fail-closed).
//! Die Fragefolge (hinter der IBAN) halten `nach_ausloesern` (Unit-Test in `fragen.rs`) und der Browser-Lauf in `fluss_j.rs`.
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
use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
use interview::Graph;
use store::{BindungNachschlag, NeuesEvent, Store};

const IBAN: &str = "stammdaten_iban";
const BIC: &str = "stammdaten_bic";

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

/// Ein Store mit der IBAN als Antwort des Nutzers: `bestaetigt` oder als Vorschlag (`vorlaeufig`).
fn store_mit_iban(iban: &str, bestaetigt: bool) -> Store {
    let karte: HashMap<String, &'static Bindung> = store::baue_nachschlag(bindungen());
    let mut s = Store::leer(2025, None);
    let neu = NeuesEvent {
        feld_id: IBAN.to_owned(),
        wert: serde_json::json!(iban).into(),
        feldzustand: if bestaetigt {
            Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() }
        } else {
            Feldzustand::Vorlaeufig
        },
        herkunft: Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: Schreiber::Mensch("ui:laie".to_owned()),
        signal_1: None,
        ersetzt: None,
        ts: Some("2026-10-21T12:00:00+00:00".to_owned()),
    };
    s.append(&neu, None, BindungNachschlag::neu(&karte)).unwrap();
    s
}

fn queue(s: &Store) -> Vec<&'static str> {
    let ohne: Option<&HashMap<String, i64>> = None;
    interview::naechste_fragen(s, graph().alle(), graph(), ohne)
}

#[test]
fn eine_deutsche_iban_nimmt_den_bic_aus_der_queue() {
    // KONTROLLE: ohne Antwort steht der BIC in der Queue, sonst waere "steht nicht darin" kein Befund.
    assert!(queue(&Store::leer(2025, None)).contains(&BIC));
    // Gross, klein, mit Leerzeichen: der Wert wird so gelesen, wie ihn `deklariere` liest.
    for iban in ["DE89370400440532013000", "de89370400440532013000", "  de89 3704 0044 0532 0130 00"] {
        let q = queue(&store_mit_iban(iban, true));
        assert!(!q.contains(&BIC), "{iban:?}: der BIC steht trotz deutscher IBAN in der Queue");
        assert!(!q.contains(&IBAN), "{iban:?}: die beantwortete IBAN steht wieder in der Queue");
    }
}

#[test]
fn jede_andere_iban_und_die_unbeantwortete_behalten_den_bic() {
    for iban in ["GB82WEST12345698765432", "AT611904300234573201", "gb82 west 1234 5698 7654 32", "D"] {
        let q = queue(&store_mit_iban(iban, true));
        assert!(q.contains(&BIC), "{iban:?}: der BIC fehlt in der Queue");
    }
    // Ein Vorschlag (`vorlaeufig`) entscheidet nichts: erst die bestaetigte IBAN nimmt den BIC aus der Queue.
    let q = queue(&store_mit_iban("DE89370400440532013000", false));
    assert!(q.contains(&BIC), "eine vorlaeufige deutsche IBAN nahm den BIC aus der Queue");
}
