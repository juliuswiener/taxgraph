//! Die 250-EUR-Grenze des GWG-Sofortabzugs (§ 6 Abs. 2 Satz 4 `EStG`), im Standardlauf (ohne `PARITY=1`, ohne Python):
//! ohne Verzeichnis (`gwg_verzeichnis_ab_250 == false`) gilt der Sofortabzug bis EINSCHLIESSLICH 250,00 EUR, ab 250,01 EUR
//! ist er 0 (das Geraet gehoert in die `AfA`).
//!
//! WARUM ES DIESE DATEI GIBT. Die Mutationsmessung `bescheid-elster-mutation` (2026-10-04, Kennung E02) liess
//! `einkuenfte::gwg_abzug` mit `netto > 25_000` -> `netto >= 25_000` in ALLEN Standard-Tests und auch mit `PARITY=1`
//! (`bescheid_blatt_paritaet`: Zufallsfaelle, reale Faelle, Golden) gruen: kein Fall trifft genau 250,00 EUR. Der Fehler waere
//! eine Zahl ohne Sperre: bei genau 250,00 EUR ohne Verzeichnis zoege Rust 0 statt 250 EUR ab, in der Sperre
//! (`sperre::gesamt::gwg`, Schwelle `> 25_000`) bliebe der Fall unauffaellig (Gegenstueck `offene_defekte.rs`
//! `gwg_sofortabzug_bleibt_wo_er_zusteht` prueft nur den Sperrgrund, nicht den Betrag).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Ausgabe des Python-Orakels, gemessen 2026-10-04 auf `0197bf76`:
//! `bescheid_einkuenfte._gwg_sofortabzug_summe(felder, None, None, True)` (Python `netto > 25000`, `bescheid_einkuenfte.py:61`)
//! mit den Feldern dieser Tabelle, dazu `gwg_bewegliches_selbstaendig_nutzbar` und `gwg_netto_ohne_vorsteuer` jeweils `true`.
//! Kein Wert ist aus dem Rust-Code abgelesen. Dieselben Eingaben liefen im Witness `w_e02` gegen das Original (Python 250,
//! Rust 250) und gegen den Mutanten (Rust 0).
//!
//! ponytail: die Erwartungswerte sind eingefroren; aendert sich Pythons Schwelle, rechnet man sie am Orakel nach.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::einkuenfte::gwg_sofortabzug_summe;
use bescheid::testhilfe::{felder, store};
use bescheid::Instanzquelle;
use serde_json::json;

/// Sofortabzug in EURO fuer EIN Geraet (Basis-Feld, ohne Store/Bindung wie Pythons `store is None`).
fn abzug(netto_cent: i64, verzeichnis: bool) -> i64 {
    let f = felder(&store(&[
        ("gwg_anschaffungskosten_netto", json!(netto_cent), true),
        ("gwg_verzeichnis_ab_250", json!(verzeichnis), true),
        ("gwg_bewegliches_selbstaendig_nutzbar", json!(true), true),
        ("gwg_netto_ohne_vorsteuer", json!(true), true),
    ]));
    let q = Instanzquelle {
        store: None,
        bindung: None,
        nur_bestaetigt: true,
    };
    gwg_sofortabzug_summe(&f, &q).unwrap().get()
}

/// `(netto in Cent, Verzeichnis gefuehrt, Python-Wert in EURO)`.
const FAELLE: [(i64, bool, i64); 4] = [
    // Die Grenze selbst: 250,00 EUR ohne Verzeichnis zaehlen noch (Python `netto > 25000` greift erst darueber).
    (25_000, false, 250),
    // Ein Cent darueber ohne Verzeichnis: 0. Faengt eine Grenze, die nach oben rutscht (`> 25_001`).
    (25_001, false, 0),
    // Ein Cent darueber MIT Verzeichnis: Sofortabzug von 250 EUR (Euro-Rundung nach unten).
    (25_001, true, 250),
    // Die Grenze mit Verzeichnis: das Verzeichnis aendert unter der Grenze nichts.
    (25_000, true, 250),
];

#[test]
fn gwg_250_euro_grenze_wie_python() {
    for (netto, verzeichnis, soll) in FAELLE {
        assert_eq!(
            abzug(netto, verzeichnis),
            soll,
            "netto {netto} Cent, Verzeichnis {verzeichnis}"
        );
    }
}
