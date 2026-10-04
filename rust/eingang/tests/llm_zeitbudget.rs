//! Das Zeitbudget der LLM-Rueckfaelle (`LLM_ZEITBUDGET`, 300 s) in `kontoauszug::uebernehme`:
//! ein Klassifikator, der laenger braucht, laesst die uebrigen unklaren Buchungen unklassifiziert
//! (gezaehlt in `llm_uebersprungen`) statt sie weiter zu fragen. Python: `LLM_ZEITBUDGET_S`.
//!
//! Der Test wartet das Budget wirklich ab (301 s), weil die Zeit keine Einspritzstelle hat; er laeuft
//! deshalb nur auf Anforderung:
//! `cargo test -p eingang --test llm_zeitbudget -- --ignored`
//! Gemessen mit einer Mutationsliste von Hand (`berichte/auth-eingang-mutation.md`): ohne die
//! Zeitpruefung in `uebernehme` fragt die Schleife alle weiteren Buchungen, und dieser Test wird rot.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::cell::Cell;
use std::time::Duration;

use eingang::kontoauszug::{uebernehme, Transaktion, LLM_ZEITBUDGET};
use store::{BindungNachschlag, Store};

#[test]
#[ignore = "wartet 301 s: manuell mit --ignored"]
fn nach_dem_zeitbudget_fragt_die_uebernahme_das_llm_nicht_mehr() {
    let tx = |i: i64| Transaktion {
        datum: serde_json::json!("01.03.2025"),
        betrag: -100 - i,
        verwendungszweck: format!("Posten {i}"),
    };
    let buchungen = [tx(0), tx(1), tx(2)];
    let aufrufe = Cell::new(0);
    let langsam = |_: &llm::pii::Maskiert, _: i64| {
        aufrufe.set(aufrufe.get() + 1);
        if aufrufe.get() == 1 {
            std::thread::sleep(LLM_ZEITBUDGET + Duration::from_secs(1));
        }
        None
    };
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let mut store = Store::leer(2025, None);
    let r = uebernehme(
        &mut store,
        &buchungen,
        nachschlag,
        Some(&langsam),
        None,
        None,
    )
    .unwrap();
    assert_eq!(aufrufe.get(), 1, "nach dem Budget kein weiterer Aufruf");
    assert_eq!((r.uebernommen, r.llm_uebersprungen), (0, 2));
}
