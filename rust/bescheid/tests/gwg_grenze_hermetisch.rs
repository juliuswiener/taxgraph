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

/// Sofortabzug in EURO fuer EIN Geraet mit der Folgefrage `gwg_ohne_vorsteuerabzug` (Abweichung Nr. 27; Python kennt das Feld
/// nicht). `None` = nicht beantwortet. `nur_bestaetigt: false` ist die Schaetzung (`/stand`): ein vorlaeufiges Ja zaehlt dort.
fn abzug_mit_folgefrage(
    netto_cent: i64,
    netto_frage: bool,
    folge: Option<(bool, bool)>,
    verzeichnis: bool,
    nur_bestaetigt: bool,
) -> i64 {
    let mut events = vec![
        ("gwg_anschaffungskosten_netto", json!(netto_cent), true),
        ("gwg_verzeichnis_ab_250", json!(verzeichnis), true),
        ("gwg_bewegliches_selbstaendig_nutzbar", json!(true), true),
        ("gwg_netto_ohne_vorsteuer", json!(netto_frage), true),
    ];
    if let Some((antwort, bestaetigt)) = folge {
        events.push(("gwg_ohne_vorsteuerabzug", json!(antwort), bestaetigt));
    }
    let f = felder(&store(&events));
    let q = Instanzquelle {
        store: None,
        bindung: None,
        nur_bestaetigt,
    };
    gwg_sofortabzug_summe(&f, &q).unwrap().get()
}

/// Die Rechnung hinter der Folgefrage, Erwartung per Handrechnung (EURO, Cent / 100 abgerundet): wer "netto: nein" sagt und
/// die Mehrwertsteuer selbst getragen hat, zieht den eingegebenen Bruttobetrag ab (§ 9b Abs. 1 `EStG`). Ein "nein", die fehlende
/// Antwort, ein Betrag ueber 800,00 EUR und ein Betrag ueber 250,00 EUR ohne Verzeichnis geben 0. Ist die Netto-Frage "ja", zaehlt
/// die Folgefrage nicht.
#[test]
fn gwg_folgefrage_ohne_vorsteuerabzug_bestimmt_den_abzug() {
    let ja = Some((true, true));
    let nein = Some((false, true));
    // (Name, Betrag in Cent, Netto-Frage, Folgefrage, Verzeichnis, Abzug in EURO)
    #[allow(clippy::type_complexity)]
    let faelle: [(&str, i64, bool, Option<(bool, bool)>, bool, i64); 8] = [
        ("790 brutto, Folgefrage ja", 79_000, false, ja, true, 790),
        ("800,00 brutto, Folgefrage ja: die Grenze", 80_000, false, ja, true, 800),
        ("800,01 brutto, Folgefrage ja: darueber", 80_001, false, ja, true, 0),
        ("790 brutto, Folgefrage nein", 79_000, false, nein, true, 0),
        ("790 brutto, Folgefrage unbeantwortet", 79_000, false, None, true, 0),
        ("250,01 brutto, Folgefrage ja, kein Verzeichnis", 25_001, false, ja, false, 0),
        ("790 netto, Folgefrage nein: zaehlt nicht", 79_000, true, nein, true, 790),
        ("790 netto, Folgefrage ja: zaehlt nicht", 79_000, true, ja, true, 790),
    ];
    for (name, netto, netto_frage, folge, verzeichnis, soll) in faelle {
        assert_eq!(abzug_mit_folgefrage(netto, netto_frage, folge, verzeichnis, true), soll, "{name}");
    }
}

/// Die Schaetzung (`nur_bestaetigt: false`, `/stand`) zaehlt ein VORLAEUFIGES Ja der Folgefrage mit und bleibt ohne Folgefrage
/// bei 0. Die festgesetzte Zahl oeffnet ein vorlaeufiges Ja nicht: das haelt die Sperre
/// (`offene_defekte.rs::gwg_vorlaeufige_folgefrage_oeffnet_den_abzug_nicht`).
#[test]
fn gwg_folgefrage_schaetzung_zaehlt_ein_vorlaeufiges_ja() {
    let vorlaeufig_ja = Some((true, false));
    assert_eq!(
        abzug_mit_folgefrage(79_000, false, vorlaeufig_ja, true, false),
        790,
        "Schaetzung: das vorlaeufige Ja zaehlt"
    );
    assert_eq!(
        abzug_mit_folgefrage(79_000, false, None, true, false),
        0,
        "Schaetzung: ohne Folgefrage bleibt es bei 0"
    );
}
