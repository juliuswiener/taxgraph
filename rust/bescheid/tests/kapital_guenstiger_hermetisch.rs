//! § 32d Abs. 6 `EStG`: wann zaehlt die Guenstigerpruefung als gewonnen (`Kapital::guenstiger` -> `Extras`), im
//! Standardlauf (ohne `PARITY=1`, ohne Python).
//!
//! WARUM ES DIESE DATEI GIBT. Die Mutationsmessung `bescheid-elster-mutation` (2026-10-04, Kennung T08) liess
//! `zweige/tarif.rs::kapital` mit `kap_st < abgeltung` -> `<=` in allen Standard-Tests gruen; nur `PARITY=1` faengt es.
//! Die Stelle ist ueber die HTTP-Schnittstelle NICHT erreichbar: `Kapital::guenstiger` landet allein in
//! `Extras::kap_guenstiger_gewonnen`, und `/ergebnis` gibt dieses Feld nicht aus (Python: `extras["kap_guenstiger_gewonnen"]`
//! in `_feste_zahl`, nur an den Elster-Pfad). Darum sitzt der Fall hier auf Funktionsebene (`bescheid_fn` mit `Extras`),
//! nicht in `api/tests/kette_endstand_hermetisch.rs`.
//!
//! Am Gleichstand (`kap_st == abgeltung`) greift die Abgeltungsteuer; der tarifliche Zweig hat dann NICHT gewonnen.
//! Das ist der haeufige Fall bei hohem Einkommen. Der Mutant `<=` meldet dort "gewonnen" und wuerde den Antrag
//! (E1900401) falsch setzen.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Ausgabe des Python-Orakels auf denselben Feldern, gemessen am 2026-10-04:
//! `python3 tools/parity/kette_erwartung.py --datei=rust/bescheid/tests/kapital_guenstiger_hermetisch.rs gesamt
//! kegel_t08 <FALL>_aenderungen`. Das Skript legt den Fall ueber die echten Endpunkte an und liest `kap_guenstiger` aus den
//! `extras` von `_feste_zahl`; `zahl` ist die festzusetzende `ESt` in Cent. Kein Wert ist aus dem Rust-Code abgelesen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::cell::RefCell;

use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::{bescheid_fn, Extras, Umgebung};
use bindung::SlotBeitrag;
use domain::{Feldtyp, PyWert, Vz};
use intervall::{AchsenBindung, Werte};
use serde_json::{json, Value};

type Paare = Vec<(&'static str, Value)>;

/// Slot der Zweige -> Feld, das ihn speist (`rust/bindung/daten`, Scheibe `gesamt`).
const SLOTS: [(&str, &str); 6] = [
    ("arbeitstage", "ep_arbeitstage"),
    ("entfernung_km_roh", "ep_entfernung_km"),
    ("oepnv_kosten_jahr", "ep_oepnv_kosten"),
    ("eigenes_oder_ueberlassenes_kfz", "ep_eigenes_kfz"),
    ("bruttoarbeitslohn", "bruttoarbeitslohn"),
    ("veranlagung", "veranlagung"),
];

/// Alleinstehender Angestellter mit 60.000 EUR Lohn und 30.000 EUR Kapitalertraegen, ohne Pendelstrecke, ohne Konfession.
fn kegel_t08() -> Paare {
    vec![
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("kein_kap", json!(false)),
        ("kap_kapitalertraege", json!(3_000_000)),
        ("kist_konfession", json!("keine")),
    ]
}

/// Wie `kegel_t08`: die Abgeltungsteuer (25 %) greift, die tarifliche Steuer ist hoeher.
fn t08_abgeltung_aenderungen() -> Paare {
    vec![]
}

/// 10.000 EUR Lohn statt 60.000 EUR: die tarifliche Steuer auf die Kapitalertraege ist niedriger als die Abgeltung.
fn t08_guenstiger_aenderungen() -> Paare {
    vec![("bruttoarbeitslohn", json!(1_000_000))]
}

/// Kapitalertraege genau in Hoehe des Sparer-Pauschbetrags (1.000 EUR): nach dem Abzug bleibt nichts, § 32d laeuft nicht.
fn t08_im_pauschbetrag_aenderungen() -> Paare {
    vec![("kap_kapitalertraege", json!(100_000))]
}

fn mit(basis: Paare, aenderungen: Paare) -> Paare {
    let mut raus = basis;
    for (feld, wert) in aenderungen {
        match raus.iter_mut().find(|(f, _)| *f == feld) {
            Some(p) => p.1 = wert,
            None => raus.push((feld, wert)),
        }
    }
    raus
}

fn py(v: &Value) -> PyWert {
    match v {
        Value::Bool(b) => PyWert::Bool(*b),
        Value::String(s) => PyWert::Text(s.clone()),
        Value::Number(n) => PyWert::Ganz(n.as_i64().unwrap()),
        andere => panic!("Slot-Wert {andere}"),
    }
}

/// `(festzusetzende_est_gesamt in Cent, kap_guenstiger_gewonnen)` fuer die Felder `paare`.
fn gesamt(paare: &Paare) -> (i64, Option<bool>) {
    let events: Vec<_> = paare.iter().map(|(f, w)| (*f, w.clone(), true)).collect();
    let f = felder(&store(&events));
    let achsen: Vec<AchsenBindung> = SLOTS
        .iter()
        .map(|(slot, _)| AchsenBindung {
            feld_id: (*slot).to_owned(),
            typ: Feldtyp::Int,
            askable: true,
            enum_werte: Vec::new(),
            bereich: None,
            signatur_slot: Some((*slot).to_owned()),
            slot_beitrag: SlotBeitrag::Exakt,
        })
        .collect();
    let umg = Umgebung {
        achsen: &achsen,
        index: index(),
        params: params(),
    };
    let extras = RefCell::new(Extras::default());
    let rechne = bescheid_fn(
        "festzusetzende_est_gesamt",
        Vz::Vz2025,
        &umg,
        Some(&f),
        None,
        true,
        None,
        Some(&extras),
    )
    .expect("Quantitaet bekannt");
    let mut w = Werte::neu();
    for (slot, feld) in SLOTS {
        let wert = paare.iter().find(|(f, _)| *f == feld).unwrap().1.clone();
        w.setze(slot, py(&wert));
    }
    let zahl = rechne(&w).expect("Zweig rechnet").get();
    let gewonnen = extras.borrow().kap_guenstiger_gewonnen;
    (zahl, gewonnen)
}

/// Hohes Einkommen: die Abgeltungsteuer greift (`kap_st == abgeltung`), die Pruefung ist NICHT gewonnen. Python:
/// `kap_guenstiger_gewonnen` = false. Der Mutant `kap_st <= abgeltung` meldet hier true.
#[test]
fn am_gleichstand_greift_die_abgeltung_und_die_pruefung_ist_nicht_gewonnen() {
    let (zahl, gewonnen) = gesamt(&mit(kegel_t08(), t08_abgeltung_aenderungen()));
    assert_eq!((zahl, gewonnen), (2_117_400, Some(false)), "Abgeltung");
}

/// Niedriges Einkommen: die tarifliche Steuer ist niedriger, die Pruefung ist gewonnen. Python: true. Gegenprobe zum
/// Gleichstandsfall -- ein Wert, der immer false meldet, bestuende nur den ersten Test.
#[test]
fn bei_niedrigem_einkommen_gewinnt_die_guenstigerpruefung() {
    let (zahl, gewonnen) = gesamt(&mit(kegel_t08(), t08_guenstiger_aenderungen()));
    assert_eq!(
        (zahl, gewonnen),
        (660_600, Some(true)),
        "Guenstigerpruefung"
    );
}

/// Kapitalertraege im Sparer-Pauschbetrag: § 32d laeuft nicht, `Extras` bleibt leer (Python: Schluessel fehlt).
#[test]
fn im_pauschbetrag_bleibt_das_ergebnis_der_pruefung_leer() {
    let (zahl, gewonnen) = gesamt(&mit(kegel_t08(), t08_im_pauschbetrag_aenderungen()));
    assert_eq!((zahl, gewonnen), (1_392_400, None), "Pauschbetrag");
}
