//! § 34g `EStG`, Spenden an politische Parteien (`parteispenden_betrag`, Abweichung Nr. 31 in `rust/fixtures/README.md`): die
//! Rechnung. Im Standardlauf, ohne `PARITY=1`, ohne Python. Python kennt das Feld nicht.
//!
//! **Worum es geht.** Eine Spende an eine Partei senkt nicht das Einkommen, sondern die Steuer selbst: um die Haelfte des
//! Betrags, hoechstens um einen Deckel je Veranlagungsjahr (825 Euro bis 2025, 1.650 Euro ab 2026; bei Zusammenveranlagung das
//! Doppelte). Die Rechnung traegt den Betrag in `steuerermaessigungen` ein; der Teil ueber der Basis der Ermaessigung wirkt
//! zusaetzlich als Sonderausgabe (§ 10b Abs. 2, Abweichung Nr. 43, geprueft in `parteispenden_sonderausgaben.rs`). Die Faelle
//! dieser Datei liegen alle auf oder unter der Basis; dort bleiben die Sonderausgaben unveraendert.
//!
//! **Warum es zaehlt.** Vor dem Bau fehlte das Feld: wer 500 Euro an eine Partei gespendet hatte, trug sie als gewoehnliche
//! Spende ein (154 Euro Ersparnis) oder liess sie weg (0 Euro). Zustehend sind 250 Euro.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: von Hand aus dem Wortlaut unter `sources/gesetze-im-internet/estg_p34g_*.txt`: "Die
//! Ermaessigung betraegt 50 Prozent der Ausgaben, hoechstens jeweils 825 Euro ... im Fall der Zusammenveranlagung von Ehegatten
//! hoechstens jeweils 1 650 Euro" (Fassungen 2024-12-18 und 2025-12-18) und "1 650 Euro ... 3 300 Euro" (Fassung 2026-09-26). Dass
//! `params/<vz>/parteispenden_p34g.yaml` dasselbe sagt, prueft `rust/bindung/tests/parteispenden_werte_gegen_gesetz.rs` gegen den
//! Text, nicht dieser Test.
//!
//! Abgerundet wird auf ganze Euro (`ponytail`: halbe Euro der Ermaessigung fallen weg, hoechstens 0,50 Euro zu wenig; der
//! Gesetzestext nennt keine Rundung).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    clippy::disallowed_types
)]

use std::path::{Path, PathBuf};

use bescheid::abzuege::{shared_steuer_sonder_agb, SteuerSonderAgb};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::{BescheidFehler, Instanzquelle};
use bindung::Params;
use domain::{Euro, Veranlagung, Vz};
use engine::zugriff::teil1::fehler::EngineFehler as EngineFehler1;
use engine::zugriff::teil2::EngineFehler as EngineFehler2;
use serde_json::{json, Value};

const FELD: &str = "parteispenden_betrag";

/// Cent aus Euro.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

fn lauf_mit(
    p: &Params,
    vz: Vz,
    veranlagung: Veranlagung,
    paare: &[(&str, Value)],
) -> Result<SteuerSonderAgb, BescheidFehler> {
    let events: Vec<(&str, Value, bool)> =
        paare.iter().map(|(f, w)| (*f, w.clone(), true)).collect();
    let f = felder(&store(&events));
    let leer = Instanzquelle {
        store: None,
        bindung: Some(index()),
        nur_bestaetigt: true,
    };
    shared_steuer_sonder_agb(
        Euro::new(45_000),
        Euro::new(0),
        veranlagung,
        &f,
        vz,
        &leer,
        p,
    )
}

fn lauf(vz: Vz, veranlagung: Veranlagung, paare: &[(&str, Value)]) -> SteuerSonderAgb {
    lauf_mit(params(), vz, veranlagung, paare).unwrap()
}

/// Die Ermaessigung in Euro bei `spende` Euro Parteispende.
fn ermaessigung(vz: Vz, veranlagung: Veranlagung, spende: i64) -> i64 {
    lauf(vz, veranlagung, &[(FELD, json!(cent(spende)))])
        .steuerermaessigungen
        .get()
}

/// KONTROLLE zuerst: ohne das Feld ist die Ermaessigung 0, und der Sonderausgabenabzug hat einen Wert, mit dem die Tests unten
/// vergleichen. Sonst belegte "Sonderausgaben unveraendert" nichts: ein Lauf, der schon ohne Spende 0 haette, liesse es gruen.
#[test]
fn kontrolle_ohne_das_feld_ist_die_ermaessigung_null() {
    let ohne = lauf(Vz::Vz2025, Veranlagung::Einzel, &[]);
    assert_eq!(ohne.steuerermaessigungen.get(), 0);
    let mit_allgemeiner_spende = lauf(
        Vz::Vz2025,
        Veranlagung::Einzel,
        &[("spenden_betrag", json!(cent(500)))],
    );
    assert_eq!(
        mit_allgemeiner_spende.steuerermaessigungen.get(),
        0,
        "eine allgemeine Spende ist keine Steuerermaessigung"
    );
    assert_eq!(
        mit_allgemeiner_spende.sonderausgaben.get(),
        500,
        "KONTROLLE: die allgemeine Spende zaehlt als Sonderausgabe (20-%-Deckel bei 45.000 Euro: 9.000 Euro)"
    );
}

/// AK2: 500 Euro an eine Partei, ledig: 250 Euro Ermaessigung in allen drei Jahren, und der Betrag steht NICHT in den
/// Sonderausgaben.
#[test]
fn fuenfhundert_euro_geben_zweihundertfuenfzig_ermaessigung_und_keine_sonderausgabe() {
    for vz in [Vz::Vz2024, Vz::Vz2025, Vz::Vz2026] {
        let ohne = lauf(vz, Veranlagung::Einzel, &[]);
        let mit = lauf(vz, Veranlagung::Einzel, &[(FELD, json!(cent(500)))]);
        assert_eq!(mit.steuerermaessigungen.get(), 250, "{vz:?}");
        assert_eq!(
            mit.sonderausgaben.get(),
            ohne.sonderausgaben.get(),
            "{vz:?}: die Parteispende darf den Sonderausgabenabzug nicht aendern"
        );
        assert_eq!(
            mit.aussergewoehnliche_belastungen.get(),
            ohne.aussergewoehnliche_belastungen.get(),
            "{vz:?}"
        );
    }
}

/// Der Deckel je Veranlagungsjahr und Veranlagungsart: `(vz, veranlagung, spende in Euro, erwartete Ermaessigung in Euro)`.
/// Die Grenzen liegen je Jahr beidseitig an der Schwelle (Spende = 2 x Deckel) und an der Rundung (ungerader Betrag).
const FAELLE: &[(Vz, Veranlagung, i64, i64)] = &[
    // 2024 und 2025: Deckel 825 Euro einzeln, 1.650 Euro zusammen (Fassungen 2024-12-18 und 2025-12-18).
    (Vz::Vz2024, Veranlagung::Einzel, 1_649, 824),
    (Vz::Vz2024, Veranlagung::Einzel, 1_650, 825),
    (Vz::Vz2024, Veranlagung::Einzel, 3_000, 825),
    (Vz::Vz2024, Veranlagung::Zusammen, 3_000, 1_500),
    (Vz::Vz2024, Veranlagung::Zusammen, 3_300, 1_650),
    (Vz::Vz2024, Veranlagung::Zusammen, 4_000, 1_650),
    (Vz::Vz2025, Veranlagung::Einzel, 1_649, 824),
    (Vz::Vz2025, Veranlagung::Einzel, 1_650, 825),
    (Vz::Vz2025, Veranlagung::Einzel, 1_700, 825),
    (Vz::Vz2025, Veranlagung::Einzel, 3_000, 825),
    (Vz::Vz2025, Veranlagung::Zusammen, 3_000, 1_500),
    (Vz::Vz2025, Veranlagung::Zusammen, 3_300, 1_650),
    (Vz::Vz2025, Veranlagung::Zusammen, 4_000, 1_650),
    // 2026: Deckel 1.650 Euro einzeln, 3.300 Euro zusammen (Fassung 2026-09-26).
    (Vz::Vz2026, Veranlagung::Einzel, 1_700, 850),
    (Vz::Vz2026, Veranlagung::Einzel, 3_299, 1_649),
    (Vz::Vz2026, Veranlagung::Einzel, 3_300, 1_650),
    (Vz::Vz2026, Veranlagung::Einzel, 4_000, 1_650),
    (Vz::Vz2026, Veranlagung::Zusammen, 3_300, 1_650),
    (Vz::Vz2026, Veranlagung::Zusammen, 6_600, 3_300),
    (Vz::Vz2026, Veranlagung::Zusammen, 8_000, 3_300),
];

/// AK2: die Ermaessigung ist die Haelfte der Spende, hoechstens der Deckel des Jahres; zusammen gilt der doppelte Deckel.
#[test]
fn die_ermaessigung_ist_die_haelfte_hoechstens_der_deckel_je_jahr_und_veranlagung() {
    for (vz, veranlagung, spende, erwartet) in FAELLE {
        assert_eq!(
            ermaessigung(*vz, *veranlagung, *spende),
            *erwartet,
            "{vz:?} {veranlagung:?}: {spende} Euro Parteispende"
        );
    }
}

/// Leer, 0 und ein Betrag unter 2 Euro (halbe Euro fallen weg) geben 0; ein Fall ohne Feld bleibt unberuehrt.
#[test]
fn leer_null_und_ein_euro_geben_keine_ermaessigung() {
    assert_eq!(
        lauf(Vz::Vz2025, Veranlagung::Einzel, &[])
            .steuerermaessigungen
            .get(),
        0
    );
    assert_eq!(ermaessigung(Vz::Vz2025, Veranlagung::Einzel, 0), 0);
    assert_eq!(ermaessigung(Vz::Vz2025, Veranlagung::Einzel, 1), 0);
    assert_eq!(ermaessigung(Vz::Vz2025, Veranlagung::Einzel, 2), 1);
}

/// Die Parteispende steht neben § 35a und allgemeiner Spende, ohne eine davon zu aendern: die Ermaessigungen addieren sich,
/// die allgemeine Spende bleibt Sonderausgabe.
#[test]
fn parteispende_und_allgemeine_spende_wirken_nebeneinander() {
    let nur_allgemein = lauf(
        Vz::Vz2025,
        Veranlagung::Einzel,
        &[("spenden_betrag", json!(cent(500)))],
    );
    let beide = lauf(
        Vz::Vz2025,
        Veranlagung::Einzel,
        &[
            ("spenden_betrag", json!(cent(500))),
            (FELD, json!(cent(500))),
        ],
    );
    assert_eq!(
        beide.sonderausgaben.get(),
        nur_allgemein.sonderausgaben.get()
    );
    assert_eq!(beide.steuerermaessigungen.get(), 250);
    // § 35a: 2.000 Euro Dienstleistung = 400 Euro Ermaessigung; die Parteispende kommt dazu.
    let mit_35a = lauf(
        Vz::Vz2025,
        Veranlagung::Einzel,
        &[
            ("hh_dienstleistung_betrag", json!(cent(2_000))),
            ("hh_rechnung_unbar", json!(true)),
            ("hh_in_eu_ewr", json!(true)),
            (FELD, json!(cent(500))),
        ],
    );
    let nur_35a = lauf(
        Vz::Vz2025,
        Veranlagung::Einzel,
        &[
            ("hh_dienstleistung_betrag", json!(cent(2_000))),
            ("hh_rechnung_unbar", json!(true)),
            ("hh_in_eu_ewr", json!(true)),
        ],
    );
    assert_eq!(
        mit_35a.steuerermaessigungen.get() - nur_35a.steuerermaessigungen.get(),
        250,
        "die Parteispende addiert sich zu § 35a: mit {mit_35a:?}, ohne {nur_35a:?}"
    );
}

// ------------------------------------------------------------------------------ Jahr ohne belegte Fassung

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kopiere(von: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for e in std::fs::read_dir(von).unwrap() {
        let e = e.unwrap();
        let ziel = nach.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            kopiere(&e.path(), &ziel);
        } else {
            std::fs::copy(e.path(), &ziel).unwrap();
        }
    }
}

/// Entscheidung `parteispenden-deckel-kommt-je-jahr-aus-der-eingefrorenen-fassung`: Ein Jahr ohne belegte Fassung bleibt
/// gesperrt. Fehlt die Parameterdatei eines Jahres, rechnet eine Parteispende ueber 0 NICHT mit einem geratenen Deckel und
/// nicht mit 0, sondern bricht mit einem Parameterfehler ab. Ohne Betrag liest die Rechnung die Datei nie; ein Fall ohne
/// Parteispende rechnet in diesem Jahr weiter.
#[test]
fn ein_jahr_ohne_parameterdatei_sperrt_die_parteispende_und_nur_sie() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("p34g-ohne-datei-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    kopiere(&wurzel().join("params"), &tmp.join("params"));
    let datei = tmp.join("params/2025/parteispenden_p34g.yaml");
    assert!(
        datei.is_file(),
        "KONTROLLE: die Kopie fuehrt die Datei, die der Test gleich entfernt"
    );
    std::fs::remove_file(&datei).unwrap();
    let ohne_datei = Params::lade(&tmp).unwrap();

    // Mit Datei (die echten Params) rechnet derselbe Fall.
    assert_eq!(ermaessigung(Vz::Vz2025, Veranlagung::Einzel, 500), 250);

    let r = lauf_mit(
        &ohne_datei,
        Vz::Vz2025,
        Veranlagung::Einzel,
        &[(FELD, json!(cent(500)))],
    );
    match r {
        Err(BescheidFehler::EngineTeil2(EngineFehler2::Basis(EngineFehler1::Params(e)))) => {
            let text = e.to_string();
            assert!(
                text.contains("parteispenden_p34g.yaml") && text.contains("2025"),
                "der Fehler nennt Datei und Jahr nicht: {text}"
            );
        }
        andere => panic!("erwartet einen Parameterfehler, erhalten {andere:?}"),
    }
    for paare in [vec![], vec![(FELD, json!(0))]] {
        let ok = lauf_mit(&ohne_datei, Vz::Vz2025, Veranlagung::Einzel, &paare).unwrap();
        assert_eq!(ok.steuerermaessigungen.get(), 0, "{paare:?}");
    }
    // Die anderen Jahre derselben Kopie rechnen weiter.
    let r26 = lauf_mit(
        &ohne_datei,
        Vz::Vz2026,
        Veranlagung::Einzel,
        &[(FELD, json!(cent(500)))],
    )
    .unwrap();
    assert_eq!(r26.steuerermaessigungen.get(), 250);
    let _ = std::fs::remove_dir_all(&tmp);
}
