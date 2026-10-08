//! § 10b Abs. 2 `EStG`: der Teil einer Parteispende ueber der Basis der Ermaessigung wirkt als Sonderausgabe (Abweichung Nr. 43 in
//! `rust/fixtures/README.md`; Punkt 4a im Vault-Eintrag `parteispenden-ueber-dem-deckel-waehlervereinigungen-und-kontoauszug-fehlen`).
//!
//! Soll von Hand, nicht aus dem Code gelesen. Quellen: Wortlaut `estg_p34g_*` (Ermaessigung 50 %, Hoechstbetrag je Jahr),
//! `estg_p10b_ab-2020-01-01` und `estg_p10b_2026-07-13` (Abs. 2: Deckel der Sonderausgabe, Satz 2: nur soweit § 34g nicht
//! gewaehrt wurde) und die Anleitung Sonderausgaben 2025 (`anl_sonderausgaben_2025.txt:56-58`: "Hoehere Spenden und Mitgliedsbeitraege
//! als 1.650 oder 3.300 Euro beruecksichtigt Ihr Finanzamt bis maximal 1.650 oder 3.300 Euro als Sonderausgaben"). Julius laesst die
//! Anleitung als Beleg der Lesart gelten (Vault, Entscheidung vom 2026-10-08). Daraus:
//!   Basis = Hoechstbetrag der Ermaessigung / 0,5 (die Spende, die § 34g schon verbraucht);
//!   Sonderausgabe = min(max(0, Spende - Basis), Deckel nach § 10b Abs. 2).
//!   2024/2025: einzeln Basis 1.650 / Deckel 1.650, zusammen 3.300 / 3.300.  2026: einzeln 3.300 / 3.300, zusammen 6.600 / 6.600.
//! Nicht Teil des Tests: die Ausschlussklausel (Partei von der staatlichen Teilfinanzierung ausgeschlossen) und Waehlervereinigungen.
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

const FELD: &str = "parteispenden_betrag";

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

/// `(Name, vz, veranlagung, Spende, allgemeine Spende, erwartete Ermaessigung, erwartete Sonderausgabe)` in Euro.
/// Kontrollen (unter oder auf der Basis: 0 ist richtig) zuerst, dann die Faelle ueber der Basis.
const FAELLE: &[(&str, Vz, Veranlagung, i64, i64, i64, i64)] = &[
    ("K1 unter dem Deckel", Vz::Vz2025, Veranlagung::Einzel, 1_000, 0, 500, 0),
    ("K2 genau auf der Basis", Vz::Vz2025, Veranlagung::Einzel, 1_650, 0, 825, 0),
    ("K3 2026 unter der Basis (3.000 < 3.300)", Vz::Vz2026, Veranlagung::Einzel, 3_000, 0, 1_500, 0),
    ("R1 2025 einzeln 1 Euro ueber der Basis", Vz::Vz2025, Veranlagung::Einzel, 1_651, 0, 825, 1),
    ("R2 2024 einzeln 3.000", Vz::Vz2024, Veranlagung::Einzel, 3_000, 0, 825, 1_350),
    ("R3 2025 einzeln 3.000", Vz::Vz2025, Veranlagung::Einzel, 3_000, 0, 825, 1_350),
    ("R4 2025 einzeln 3.300 (genau Deckel)", Vz::Vz2025, Veranlagung::Einzel, 3_300, 0, 825, 1_650),
    ("R5 2025 einzeln 4.000 (ueber dem Deckel)", Vz::Vz2025, Veranlagung::Einzel, 4_000, 0, 825, 1_650),
    ("R6 2025 zusammen 4.000", Vz::Vz2025, Veranlagung::Zusammen, 4_000, 0, 1_650, 700),
    ("R7 2025 zusammen 8.000", Vz::Vz2025, Veranlagung::Zusammen, 8_000, 0, 1_650, 3_300),
    ("R8 2026 einzeln 5.000", Vz::Vz2026, Veranlagung::Einzel, 5_000, 0, 1_650, 1_700),
    ("R9 2026 zusammen 10.000", Vz::Vz2026, Veranlagung::Zusammen, 10_000, 0, 3_300, 3_400),
    ("R10 2025 einzeln 4.000 + allgemeine Spende 500", Vz::Vz2025, Veranlagung::Einzel, 4_000, 500, 825, 2_150),
];

#[test]
fn ermaessigung_und_sonderausgabe_je_fall() {
    let mut rot = Vec::new();
    for (name, vz, v, spende, allg, erm, sa) in FAELLE {
        let mut paare = vec![(FELD, json!(cent(*spende)))];
        if *allg > 0 {
            paare.push(("spenden_betrag", json!(cent(*allg))));
        }
        let r = lauf(*vz, *v, &paare);
        let (ist_erm, ist_sa) = (r.steuerermaessigungen.get(), r.sonderausgaben.get());
        let ok = ist_erm == *erm && ist_sa == *sa;
        println!(
            "{} | {name:50} | Ermaessigung Soll {erm:>5} Ist {ist_erm:>5} | Sonderausgaben Soll {sa:>5} Ist {ist_sa:>5}",
            if ok { "GRUEN" } else { "ROT  " }
        );
        if !ok {
            rot.push(*name);
        }
    }
    assert!(rot.is_empty(), "ROT: {} von {}: {rot:?}", rot.len(), FAELLE.len());
}

// ---- Erklaerung: die Akte traegt die VOLLE Spende unter E0108701, auch ueber dem Deckel (Kontrolle, bleibt gruen)

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
fn die_erklaerung_traegt_die_volle_spende_auch_ueber_dem_deckel() {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    setze(&mut s, FELD, json!(cent(4_000)));
    let (f, _) = s.materialisiere(None).unwrap();
    for vz in [2024, 2025, 2026] {
        let d = deklariere(&f, index(), vz, None).unwrap();
        let kz = d
            .deklaration
            .get("E0108701")
            .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned));
        assert_eq!(kz.as_deref(), Some("4000"), "{vz}: E0108701 traegt die volle Spende");
        assert!(d.unvollstaendig().is_empty(), "{vz}: keine neue Sperre: {:?}", d.unvollstaendig());
    }
}
