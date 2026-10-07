//! Abweichung Nr. 39: Der Kontoauszug schlaegt fuer eine Parteispende nichts vor (`eingang::kontoauszug::uebernehme`).
//!
//! Python ordnet jede Buchung der Kategorie Spende dem Feld `spenden_betrag` zu, auch "Spende SPD Ortsverband"
//! (`produkt/eingang/kontoauszug_writer.py` kennt keinen Parteinamen). Rust ueberspringt eine Spende, deren Zweck einen
//! Parteinamen als ganzes Wort nennt; die Frage nach Parteispenden (`parteispenden_betrag`) beantwortet der Nutzer selbst.
//! Standardlauf, ohne Python: der Vergleich mit dem Orakel nimmt die Parteinamen nicht in sein Korpus auf
//! (gemessen per `grep` ueber `rust/parity`, `rust/eingang`, `rust/llm` und die Python-Testdateien des Korpus: kein Treffer).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Entscheidung `kontoauszug-schlaegt-fuer-parteispenden-nichts-vor` (Vault) und
//! § 34g `EStG` (die Ermaessigung fuer Parteispenden hat ein eigenes Feld). Jeder Fall prueft mit einer Gegenprobe, dass
//! eine Spende ohne Parteinamen weiter vorgeschlagen wird; sonst waere ein Test, der nie etwas schreibt, leer gruen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::cell::Cell;

use eingang::kontoauszug::{uebernehme, Klassifikator, Transaktion};
use llm::pii::Maskiert;
use llm::Kategorie;
use serde_json::{json, Value};
use store::{BindungNachschlag, Store};

fn tx(zweck: &str, betrag: i64) -> Transaktion {
    Transaktion {
        datum: json!("01.03.2025"),
        betrag,
        verwendungszweck: zweck.to_owned(),
    }
}

/// Uebernimmt die Buchungen in einen leeren Store und gibt `(uebernommen, [(feld_id, wert)])` zurueck.
fn lauf(
    buchungen: &[Transaktion],
    klassifikator: Option<&Klassifikator<'_>>,
) -> (usize, Vec<(String, Value)>) {
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let mut store = Store::leer(2025, None);
    let erg = uebernehme(&mut store, buchungen, nachschlag, klassifikator, None, None).unwrap();
    let mut felder: Vec<(String, Value)> = store
        .aktive()
        .map(|(f, e)| {
            (
                f.to_owned(),
                serde_json::to_value(e).unwrap()["wert"].clone(),
            )
        })
        .collect();
    felder.sort_by(|a, b| a.0.cmp(&b.0));
    (erg.uebernommen, felder)
}

/// Die geschlossene Namensliste, so wie ein Mensch sie schreibt (Gross- und Kleinschreibung wechselt).
const NAMEN: [&str; 12] = [
    "Partei",
    "Parteispende",
    "SPD",
    "CDU",
    "CSU",
    "FDP",
    "AfD",
    "BSW",
    "Grüne",
    "Bündnis 90",
    "Die Linke",
    "Freie Wähler",
];

/// AK1: Eine Spende mit Parteinamen bekommt keinen Vorschlag; "Spende Tierheim" im selben Auszug schon.
/// Die Reihenfolge ist absichtlich Partei zuerst: Schriebe die Partei nach `spenden_betrag`, bliebe fuer das Tierheim
/// (gleiches Feld, schon belegt) nichts uebrig, und der Wert unten waere 5.000 statt 2.500.
#[test]
fn eine_spende_mit_parteinamen_bekommt_keinen_vorschlag() {
    let (n, felder) = lauf(&[tx("Spende SPD Ortsverband", -5_000)], None);
    assert_eq!(n, 0, "{felder:?}");
    assert!(felder.is_empty(), "{felder:?}");

    let (n, felder) = lauf(
        &[
            tx("Spende SPD Ortsverband", -5_000),
            tx("Spende Tierheim", -2_500),
        ],
        None,
    );
    assert_eq!(n, 1, "{felder:?}");
    assert_eq!(felder, [("spenden_betrag".to_owned(), json!(2_500))]);
}

/// AK2: Auch der LLM-Weg bleibt draussen. "Beitrag Ortsverband CDU" trifft kein Schluesselwort, der Klassifikator
/// (hier eine Attrappe) nennt Spende. Ohne Parteinamen schreibt dieselbe Attrappe weiter einen Vorschlag.
#[test]
fn auch_der_llm_weg_schreibt_fuer_eine_partei_keinen_vorschlag() {
    let aufrufe = Cell::new(0);
    let attrappe = |_: &Maskiert, _: i64| {
        aufrufe.set(aufrufe.get() + 1);
        Some(Kategorie::Spende)
    };
    let (n, felder) = lauf(&[tx("Beitrag Ortsverband CDU", -6_000)], Some(&attrappe));
    assert_eq!(aufrufe.get(), 1, "der Klassifikator muss den Fall sehen");
    assert_eq!(n, 0, "{felder:?}");
    assert!(felder.is_empty(), "{felder:?}");

    let (n, felder) = lauf(
        &[tx("Beitrag Ortsverein Musterstadt", -6_000)],
        Some(&attrappe),
    );
    assert_eq!(aufrufe.get(), 2);
    assert_eq!(n, 1, "{felder:?}");
    assert_eq!(felder, [("spenden_betrag".to_owned(), json!(6_000))]);
}

/// AK3, Treffer: Jeder Eintrag der Liste, in mehreren Schreibweisen und mit Zeichen, die ein Wort begrenzen.
#[test]
fn jeder_parteiname_unterdrueckt_den_vorschlag() {
    let mut falsch = Vec::new();
    for name in NAMEN {
        for zweck in [
            format!("Spende an {name} Ortsverband"),
            format!("SPENDE {}", name.to_uppercase()),
            format!("spende {}", name.to_lowercase()),
            format!("Spende ({name})"),
            format!("Spende {name}-Ortsverband"),
            format!("Spende Köln {name}"),
        ] {
            let (n, felder) = lauf(&[tx(&zweck, -1_000)], None);
            if n != 0 || !felder.is_empty() {
                falsch.push(format!("{zweck:?} -> {n} {felder:?}"));
            }
        }
    }
    for zweck in [
        // Schraegstrich begrenzt: "Bündnis 90/Die Grünen" nennt "Bündnis 90" als ganzes Wort.
        "Spende Bündnis 90/Die Grünen",
        // Die erste Fundstelle von "afd" liegt mitten in "Schlafdecken", erst die zweite ist ein ganzes Wort.
        "Spende Schlafdecken an die AfD",
    ] {
        let (n, felder) = lauf(&[tx(zweck, -1_000)], None);
        if n != 0 || !felder.is_empty() {
            falsch.push(format!("{zweck:?} -> {n} {felder:?}"));
        }
    }
    assert!(falsch.is_empty(), "Vorschlag trotz Parteiname: {falsch:#?}");
}

/// AK3, Gegenfall: Der Name steht nur MITTEN in einem anderen Wort (davor oder danach ein Buchstabe oder eine Ziffer),
/// die Buchung wird weiter vorgeschlagen. Dazu je ein echtes Wort.
#[test]
fn ein_parteiname_mitten_im_wort_loest_nicht_aus() {
    let mut falsch = Vec::new();
    let mut zwecke = Vec::new();
    for name in NAMEN {
        zwecke.push(format!("Spende x{name}"));
        zwecke.push(format!("Spende {name}x"));
    }
    zwecke.extend(
        [
            "Spende Schlafdecken Aktion", // "afd" mitten im Wort
            "Spende Gegenpartei e.V.",    // "partei" am Wortende
            "Spende Verein Grüner Weg",   // "grüne" am Wortanfang von "grüner"
            "Spende Gruppe CDU2",         // eine Ziffer gehoert zum Wort
            "Spende Bündnis 900",         // "bündnis 90" vor einer weiteren Ziffer
        ]
        .map(str::to_owned),
    );
    for zweck in zwecke {
        let (n, felder) = lauf(&[tx(&zweck, -1_000)], None);
        if n != 1 || felder != [("spenden_betrag".to_owned(), json!(1_000))] {
            falsch.push(format!("{zweck:?} -> {n} {felder:?}"));
        }
    }
    assert!(
        falsch.is_empty(),
        "Teilwort hat den Vorschlag verhindert: {falsch:#?}"
    );
}

/// Die Pruefung gilt nur fuer die Kategorie Spende: "CDU Maler Huber" ist eine Handwerkerbuchung und bleibt eine.
#[test]
fn ein_parteiname_in_einer_anderen_kategorie_aendert_nichts() {
    let (n, felder) = lauf(
        &[tx("Maler Huber fuer CDU Geschaeftsstelle", -48_000)],
        None,
    );
    assert_eq!(n, 1, "{felder:?}");
    assert_eq!(felder, [("hh_handwerker_betrag".to_owned(), json!(48_000))]);
}
