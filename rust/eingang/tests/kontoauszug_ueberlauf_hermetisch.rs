//! Ueberlaufpfade des Kontoauszug-Imports (`eingang::kontoauszug`): Betraege, die kein `i64` faengt,
//! scheitern als `KontoauszugFehler::BetragUeberlauf` (API: 500 `OverflowError`) — Standardlauf, ohne Python.
//!
//! Die Mutanten (Bericht h8-hermetisch5, gemessen 2026-10-04 auf f77a7776 + den h4-Tests, Bestand 137
//! passed / 0 failed): E1 `eur_cent_signed`: `ganz.parse::<i64>()?.checked_mul(100)` -> `wrapping_mul(100)`;
//! E3 `aus_json`: `PyInt::Ueberlauf => return Err(BetragUeberlauf(..))` -> `PyInt::Wert(0)` (der Fehlerzweig
//! ist fort, der Betrag wird still 0); E4 `betrag_tragbar`: `PyInt::Ueberlauf => false` -> `true`;
//! E5 `uebernehme`: `tx.betrag.checked_abs().ok_or_else(|| BetragUeberlauf(..))?` -> `wrapping_abs()` ohne
//! Fehlerzweig. E2 (das Nachkomma-`checked_add`) faengt der Bestand (`betrag_tabelle`,
//! `unlesbarer_betrag_faellt_einzeln_raus`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Python laesst an denselben Stellen keine Zahl durch, die es nicht exakt
//! in die Akte schreiben kann — `produkt/eingang/kontoauszug_writer._eur_cent_signed` prueft
//! `cent > _I64_MAX` und liefert `None` ("PARITÄT: Rust rechnet in i64; darueber unlesbar wie dort"),
//! `_betrag_tragbar` faengt den `OverflowError` von `int(inf)` zu `False`, und `uebernehme_kontoauszug`
//! rechnet `abs(betrag)` unbeschraenkt: `-2**63` ergibt `2**63` Cent, was der Store mit
//! `ValueError: fail-closed (F2/Magnitude)` abweist (gemessen ueber `tools/parity/schritt8_oracle.py`,
//! `eingang.konto`). Rust bricht dort mit `BetragUeberlauf`/`OverflowError` AB — earlier und mit einer
//! anderen Klasse als Python (Python erreicht erst den Store). Das ist die dokumentierte
//! fail-closed-Konvention der Ueberlauf-Naht (`rust/bescheid/src/lib.rs:17`), kein Python-Gleichklang;
//! der Fall traegt die Art "klassen-abweichung" im Namen. Die Grenze selbst (2^63, 2^64) ist die
//! i64-Grenze, die Python an der Rumpftuer haelt (`server._ganzzahl_im_i64`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use eingang::kontoauszug::{
    aus_json, betrag_tragbar, eur_cent_signed, uebernehme, KontoauszugFehler, Transaktion,
};
use serde_json::json;
use store::Store;

/// Die Zahlengrenzen, die `eur_cent_signed` als unlesbar verwerfen muss (Python: `None` bei
/// `cent > _I64_MAX`; i64::MAX selbst ist lesbar).
#[test]
fn eur_cent_signed_verwirft_betraege_ausserhalb_i64_und_liest_die_grenze() {
    let mut falsch = Vec::new();
    for (roh, erwartet) in [
        // 92.233.720.368.547.758,07 EUR = 9.223.372.036.854.775.807 Cent = i64::MAX (ganze Grenze).
        ("92.233.720.368.547.758,07", Some(i64::MAX)),
        ("92.233.720.368.547.758,08", None),
        // Das Vorzeichen ist gleichgueltig: Python vergleicht den absoluten Cent-Wert gegen
        // `_I64_MAX` (`kontoauszug_writer._eur_cent_signed`), deshalb ist auch -i64::MAX unlesbar.
        ("-92.233.720.368.547.758,08", None),
        ("-92.233.720.368.547.758,09", None),
        ("-12.345.678.901.234.567.890,12", None),
        // Gegenprobe: ein Alltagsbetrag bleibt lesbar.
        ("1.234,56", Some(123_456)),
    ] {
        if let Some(e) = erwartet {
            if eur_cent_signed(roh) != Some(e) {
                falsch.push(format!(
                    "{roh}: {e} erwartet, gekommen {:?}",
                    eur_cent_signed(roh)
                ));
            }
        } else if eur_cent_signed(roh).is_some() {
            falsch.push(format!(
                "{roh}: None erwartet, gekommen {:?}",
                eur_cent_signed(roh)
            ));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// Ein JSON-Betrag ausserhalb i64 ist `BetragUeberlauf` (API: 500 `OverflowError`) — und ein solcher
/// Betrag ist nicht tragbar, faellt also einzeln raus, statt still als 0 gebucht zu werden.
#[test]
fn ein_betrag_ausserhalb_i64_ist_ueberlauf_und_nicht_tragbar() {
    let mut falsch = Vec::new();
    // Werte, die `serde_json::Value` halten kann (u64 ueber i64::MAX): `py_int` liefert
    // `PyInt::Ueberlauf`. Groessere Zahlen erreicht kein `Value` ohne `arbitrary_precision` — die
    // Rumpftuer (`api::kontoauszug::json_laden`) eretzt sie vorher durch eine Marke.
    for (name, wert) in [
        ("2^63", json!(9_223_372_036_854_775_808_u64)),
        ("u64::MAX", json!(18_446_744_073_709_551_615_u64)),
    ] {
        let liste = json!([{"datum": "01.03.2025", "betrag": wert, "verwendungszweck": "Maler"}]);
        match aus_json(&liste) {
            Err(KontoauszugFehler::BetragUeberlauf(_)) => {}
            Err(e) => falsch.push(format!("{name}: BetragUeberlauf erwartet, gekommen {e:?}")),
            Ok(tx) => falsch.push(format!(
                "{name}: BetragUeberlauf erwartet, gekommen Ok({tx:?})"
            )),
        }
        let tx = json!({"datum": "01.03.2025", "betrag": wert, "verwendungszweck": "Maler"});
        if betrag_tragbar(&tx) {
            falsch.push(format!("{name}: nicht tragbar erwartet, gekommen tragbar"));
        }
    }
    // Gegenproben: `betrag_tragbar` prueft zwei Grenzen. Die Magnitude (Python: `abs(int) < 10^10`,
    // `betrag_grenze_und_verwurf`): i64::MAX ist innerhalb i64, aber nicht tragbar. Ein Betrag knapp
    // unter der Magnitude ist beides.
    let grenze = json!({"datum": "d", "betrag": json!(9_223_372_036_854_775_807_i64), "verwendungszweck": "Maler"});
    if betrag_tragbar(&grenze) {
        falsch.push("i64::MAX: ueber der Magnitude-Grenze, muesste untragbar sein".to_owned());
    }
    let ok = json!({"datum": "d", "betrag": json!(-1500_i64), "verwendungszweck": "Maler"});
    if !betrag_tragbar(&ok) {
        falsch.push("-1500: tragbarer Betrag wird verworfen".to_owned());
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// `uebernehme` nimmt den Betragssatz absolut in den Store. `i64::MIN` hat kein positives Gegenstueck:
/// Rust meldet `BetragUeberlauf`, ein `wrapping_abs` schreibt die negative Zahl in die Akte. Python
/// rechnet `abs(-2**63) = 2**63` und der Store weist mit `ValueError (F2/Magnitude)` ab — dieselbe
/// Abweisung, andere Klasse und frueher (Art: "klassen-abweichung",fail-closed-Konvention).
#[test]
fn uebernehme_meldet_ueberlauf_statt_einen_gewickelten_betrag() {
    let nachschlag = eingang::doctest_bindung()
        .map(store::BindungNachschlag::neu)
        .unwrap();
    let tx = Transaktion {
        datum: json!("01.03.2025"),
        betrag: i64::MIN,
        verwendungszweck: "Malermeister Arbeiten".to_owned(),
    };
    let mut store = Store::leer(2025, None);
    let erg = uebernehme(&mut store, &[tx], nachschlag, None, None, None);
    assert!(
        matches!(erg, Err(KontoauszugFehler::BetragUeberlauf(_))),
        "BetragUeberlauf erwartet, gekommen {erg:?}"
    );
    assert!(
        store.aktive().next().is_none(),
        "die Akte muss leer bleiben"
    );
    // Gegenprobe: -i64::MAX (ein Betrag, den `abs` noch traegt) laeuft bis zur Store-Grenze durch,
    // bricht aber NICHT an `checked_abs` — die Fehlerart ist die der Akte, nicht des Ueberlaufs.
    let tx2 = Transaktion {
        datum: json!("01.03.2025"),
        betrag: -i64::MAX,
        verwendungszweck: "Malermeister Arbeiten".to_owned(),
    };
    let mut s2 = Store::leer(2025, None);
    let n2 = eingang::doctest_bindung()
        .map(store::BindungNachschlag::neu)
        .unwrap();
    let e2 = uebernehme(&mut s2, &[tx2], n2, None, None, None);
    assert!(
        !matches!(e2, Err(KontoauszugFehler::BetragUeberlauf(_))),
        "-i64::MAX darf nicht als Ueberlauf enden: {e2:?}"
    );
}
