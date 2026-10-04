//! Der Zeitstempel eines Events ohne `ts` ist UTC (`+00:00`), egal in welcher Zeitzone der Server
//! laeuft: Python `_now()` ist `datetime.now(timezone.utc).isoformat()` (`store.py:41`). Eine
//! Ortszeit gaebe `+02:00` und eine andere Kennung je Rechner. Eigene Datei mit EINEM Test: er setzt
//! `TZ` fuer den Prozess und darf dabei neben keinem anderen Test laufen.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::HashMap;

use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, PyWert, Schreiber, Signal2};
use store::{BindungNachschlag, NeuesEvent, Store};

#[test]
fn zeitstempel_ohne_ts_ist_utc_auch_in_einer_anderen_zeitzone() {
    // POSIX-Form, braucht keine tzdata: UTC+2 ohne Sommerzeit.
    std::env::set_var("TZ", "UTC-2");
    let neu = NeuesEvent {
        feld_id: "ep_arbeitstage".to_owned(),
        wert: PyWert::Ganz(220),
        feldzustand: Feldzustand::Bestaetigt {
            signal_2: Signal2::new("klick").unwrap(),
        },
        herkunft: Herkunft {
            herkunft: Achsenwert::new("mensch").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: Schreiber::Mensch("julius".to_owned()),
        signal_1: None,
        ersetzt: None,
        ts: None,
    };
    let leer = HashMap::new();
    let mut s = Store::leer(2025, None);
    s.append(&neu, None, BindungNachschlag::neu(&leer)).unwrap();
    let ts = &s.events()[0].ts;
    assert!(ts.ends_with("+00:00"), "{ts}");
}
