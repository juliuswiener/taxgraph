//! § 34g Satz 1 Nr. 2 `EStG`: Spenden an unabhaengige Waehlervereinigungen senken die Steuer um die Haelfte, getrennt von den
//! Parteien (Abweichung Nr. 44 in `rust/fixtures/README.md`; Punkt 4b im Vault-Eintrag
//! `parteispenden-ueber-dem-deckel-waehlervereinigungen-und-kontoauszug-fehlen`, AK7).
//!
//! Soll von Hand, nicht aus dem Code gelesen. Quellen: Wortlaut `estg_p34g_2024-12-18`, `estg_p34g_2025-12-18` und
//! `estg_p34g_2026-09-26`, Satz 2: "Die Ermaessigung betraegt 50 Prozent der Ausgaben, hoechstens jeweils 825 Euro fuer Ausgaben nach
//! den Nummern 1 und 2, im Fall der Zusammenveranlagung von Ehegatten hoechstens jeweils 1 650 Euro" (2026: 1 650 und 3 300).
//! "Jeweils" heisst: Nr. 1 (Parteien) und Nr. 2 (Waehlervereinigungen) haben getrennte Hoechstbetraege. Daraus:
//!   Ermaessigung Nr. 2 = min(int(Spende x 0,5), Hoechstbetrag), getrennt von Nr. 1.
//!   Sonderausgabe aus Nr. 2 = 0: `estg_p10b_ab-2020-01-01.txt:41` (Abs. 2 Satz 1) nennt nur "politische Parteien im Sinne des § 2
//!   des Parteiengesetzes", Waehlervereinigungen kommen im Absatz nicht vor.
//! Nicht Teil des Tests: die Voraussetzungen von Nr. 2 (Verein ohne Parteicharakter, Mandat oder Anzeige der Wahlteilnahme); das
//! Produkt fragt sie nicht ab, der Hilfetext des Feldes nennt sie.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::abzuege::{shared_steuer_sonder_agb, SteuerSonderAgb};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::Instanzquelle;
use domain::{Achsenwert, Euro, Herkunft, HerkunftVektor, PruefTiefe, Veranlagung, Vz, Zustand};
use elster::deklariere;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const PARTEI: &str = "parteispenden_betrag";
const WV: &str = "waehlervereinigungen_betrag";

const fn cent(euro: i64) -> i64 {
    euro * 100
}

fn lauf(vz: Vz, veranlagung: Veranlagung, paare: &[(&str, Value)]) -> SteuerSonderAgb {
    let events: Vec<(&str, Value, bool)> =
        paare.iter().map(|(f, w)| (*f, w.clone(), true)).collect();
    let f = felder(&store(&events));
    let leer = Instanzquelle {
        store: None,
        bindung: Some(index()),
        nur_bestaetigt: true,
    };
    shared_steuer_sonder_agb(Euro::new(45_000), Euro::new(0), veranlagung, &f, vz, &leer, params()).unwrap()
}

/// `(Name, vz, veranlagung, Partei, Waehlervereinigung, erwartete Ermaessigung, erwartete Sonderausgabe)` in Euro.
/// Kontrollen (Nr. 1 allein, schon vor Nr. 44 gruen) zuerst, dann die Faelle mit Waehlervereinigung.
#[allow(clippy::type_complexity)]
const FAELLE: &[(&str, Vz, Veranlagung, i64, i64, i64, i64)] = &[
    // Kontrollen: ohne Waehlervereinigung bleibt alles wie nach Nr. 43.
    ("K1 Partei 4.000, 2025 einzeln", Vz::Vz2025, Veranlagung::Einzel, 4_000, 0, 825, 1_650),
    ("K2 Partei 500, 2025 einzeln", Vz::Vz2025, Veranlagung::Einzel, 500, 0, 250, 0),
    // Nur Waehlervereinigung (Nr. 2).
    ("W1 2025 einzeln 500", Vz::Vz2025, Veranlagung::Einzel, 0, 500, 250, 0),
    ("W2 2025 einzeln 1.650 (genau Deckel-Basis)", Vz::Vz2025, Veranlagung::Einzel, 0, 1_650, 825, 0),
    ("W3 2025 einzeln 3.000 (ueber dem Deckel, KEINE Sonderausgabe)", Vz::Vz2025, Veranlagung::Einzel, 0, 3_000, 825, 0),
    ("W4 2025 zusammen 3.000", Vz::Vz2025, Veranlagung::Zusammen, 0, 3_000, 1_500, 0),
    ("W5 2025 zusammen 4.000", Vz::Vz2025, Veranlagung::Zusammen, 0, 4_000, 1_650, 0),
    ("W6 2024 einzeln 2.000", Vz::Vz2024, Veranlagung::Einzel, 0, 2_000, 825, 0),
    ("W7 2026 einzeln 3.000", Vz::Vz2026, Veranlagung::Einzel, 0, 3_000, 1_500, 0),
    ("W8 2026 einzeln 5.000", Vz::Vz2026, Veranlagung::Einzel, 0, 5_000, 1_650, 0),
    ("W9 2026 zusammen 5.000", Vz::Vz2026, Veranlagung::Zusammen, 0, 5_000, 2_500, 0),
    ("W10 2026 zusammen 8.000", Vz::Vz2026, Veranlagung::Zusammen, 0, 8_000, 3_300, 0),
    ("W11 2025 einzeln 1.001 (halber Euro faellt weg)", Vz::Vz2025, Veranlagung::Einzel, 0, 1_001, 500, 0),
    // Nr. 1 und Nr. 2 nebeneinander: zwei getrennte Hoechstbetraege.
    ("N1 2025 einzeln Partei 1.000 + WV 1.000", Vz::Vz2025, Veranlagung::Einzel, 1_000, 1_000, 1_000, 0),
    ("N2 2025 einzeln Partei 3.000 + WV 3.000", Vz::Vz2025, Veranlagung::Einzel, 3_000, 3_000, 1_650, 1_350),
    ("N3 2025 einzeln Partei 4.000 + WV 4.000", Vz::Vz2025, Veranlagung::Einzel, 4_000, 4_000, 1_650, 1_650),
    ("N4 2026 zusammen Partei 10.000 + WV 10.000", Vz::Vz2026, Veranlagung::Zusammen, 10_000, 10_000, 6_600, 3_400),
];

#[test]
fn ermaessigung_und_sonderausgabe_je_fall() {
    let mut rot = Vec::new();
    for (name, vz, v, partei, wv, erm, sa) in FAELLE {
        let mut paare = Vec::new();
        if *partei > 0 {
            paare.push((PARTEI, json!(cent(*partei))));
        }
        if *wv > 0 {
            paare.push((WV, json!(cent(*wv))));
        }
        let r = lauf(*vz, *v, &paare);
        let (ist_erm, ist_sa) = (r.steuerermaessigungen.get(), r.sonderausgaben.get());
        let ok = ist_erm == *erm && ist_sa == *sa;
        println!(
            "{} | {name:62} | Ermaessigung Soll {erm:>5} Ist {ist_erm:>5} | Sonderausgaben Soll {sa:>5} Ist {ist_sa:>5}",
            if ok { "GRUEN" } else { "ROT  " }
        );
        if !ok {
            rot.push(*name);
        }
    }
    assert!(rot.is_empty(), "ROT: {} von {}: {rot:?}", rot.len(), FAELLE.len());
}

// ---- Erklaerung: die Akte traegt die VOLLE Spende unter E0108801, auch ueber dem Hoechstbetrag

fn setze(s: &mut Store, feld: &str, wert: Value) {
    let leer = HashMap::new();
    s.append_roh(
        &NeuesEventRoh {
            feld_id: feld.to_owned(),
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: HerkunftVektor::Voll(Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }),
            schreiber: "ui:laie".to_owned(),
            signal: Signal {
                signal_1: Some(None),
                signal_2: Some(format!("ok@{feld}")),
                signal_2_fehlt: false,
            },
            signal_2_fremd: None,
            ersetzt: None,
            ts: Some("2026-09-01T00:00:00+00:00".to_owned()),
        },
        None,
        BindungNachschlag::neu(&leer),
    )
    .unwrap();
}

#[test]
fn die_erklaerung_traegt_die_volle_spende_auch_ueber_dem_hoechstbetrag() {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    setze(&mut s, WV, json!(cent(4_000)));
    let (f, _) = s.materialisiere(None).unwrap();
    for vz in [2024, 2025, 2026] {
        let d = deklariere(&f, index(), vz, None).unwrap();
        let kz = |k: &str| {
            d.deklaration
                .get(k)
                .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned))
        };
        assert_eq!(kz("E0108801").as_deref(), Some("4000"), "{vz}: E0108801 traegt die volle Spende");
        assert_eq!(kz("E0108701"), None, "{vz}: die Parteizeile bleibt leer");
        assert_eq!(kz("E0108105"), None, "{vz}: die allgemeine Spende bleibt leer");
        assert!(d.unvollstaendig().is_empty(), "{vz}: keine Sperre: {:?}", d.unvollstaendig());
    }
}
