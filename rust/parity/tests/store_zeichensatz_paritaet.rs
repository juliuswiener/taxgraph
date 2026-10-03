//! Paritaet des ELSTER-Zeichensatzes (Auflage Z, Ticket `elster-zeichensatz-strenger-als-xml`):
//! `domain::zeichensatz` und `Store::append` gegen die Python-Referenz `produkt/store/zeichensatz.py`
//! und `store.append_event` (Orakel `tools/parity/zeichensatz_oracle.py`).
//!
//! Zwei Aussagen:
//!  1. Die MENGE ist gleich: fuer jeden Codepunkt U+0000..U+10FFFF sagt Rust dasselbe wie Python,
//!     ob er in den Zeichensatz gehoert (Gegenstueck zum Schema-Vergleich in
//!     `rust/elster/tests/eigenschaften.rs::zeichensatz_gleicht_dem_xsd`: Schema = Python = Rust).
//!  2. Die TEXTE sind gleich, WORTGLEICH: fuer jedes Zeichen bis U+3000 plus Stichproben darueber
//!     melden Regel (`feld_meldung`), XML-Sperre (`element_meldung`) und der Weg durch den Store
//!     (`Store::append` gegen `append_event` mit der echten Bindung) denselben Wortlaut. Wo Python
//!     schon vor der Zeichenregel abweist (Auflage T: NUL und die uebrigen Steuerzeichen), gilt die
//!     Klasse, nicht der Wortlaut (wie in `store_append_paritaet.rs`).
//!
//! `PARITY=1 cargo test -p parity --test store_zeichensatz_paritaet -- --test-threads 3`
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::sync::{Mutex, MutexGuard, OnceLock};

use bindung::Bindung;
use domain::zeichensatz::{element_meldung, erstes_unerlaubtes_zeichen, feld_meldung};
use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
use parity::Oracle;
use serde_json::{json, Value};
use store::{Abweisung, BindungNachschlag, NeuesEvent, Store};

const FELD: &str = "stammdaten_nachname";
const ELEMENT: &str = "E0100201";

fn skip() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn oracle() -> MutexGuard<'static, Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Ein Orakel-Aufruf; die aeussere Huelle `{"ok": ..}` wird entfernt (ein `err` dort ist ein
/// Harness-Fehler, keine Paritaetsaussage).
fn frage(anfrage: &Value) -> Value {
    let antwort = oracle().call_json(anfrage).expect("Orakel antwortet");
    match antwort.get("ok") {
        Some(v) => v.clone(),
        None => panic!("Orakel-Harness-Fehler fuer {anfrage:.200}: {antwort}"),
    }
}

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        bindung::lade_registry(&repo_root().join("produkt").join("bindung"))
            .expect("bindung-registry laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, datei)| datei.bindungen)
            .collect()
    })
}

/// Was `Store::append` mit der echten Bindung auf einen leeren Store sagt (sonst `None`).
fn rust_store(text: &str) -> Option<Abweisung> {
    static MAP: OnceLock<std::collections::HashMap<String, &'static Bindung>> = OnceLock::new();
    let map = MAP.get_or_init(|| store::baue_nachschlag(bindungen()));
    let neu = NeuesEvent {
        feld_id: FELD.to_string(),
        wert: json!(text).into(),
        feldzustand: Feldzustand::Bestaetigt {
            signal_2: Signal2::new("ok").unwrap(),
        },
        herkunft: Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: Schreiber::Mensch("laie".to_string()),
        signal_1: None,
        ersetzt: None,
        ts: Some("2026-10-03T12:00:00+00:00".to_string()),
    };
    Store::leer(2025, None)
        .append(&neu, None, BindungNachschlag::neu(map))
        .err()
}

/// Aussage 1: die Menge, Codepunkt fuer Codepunkt, ueber den ganzen Unicode-Raum.
#[test]
fn zeichenmenge_gleicht_python_ueber_alle_codepunkte() {
    if skip() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht das Python-Umfeld)");
        return;
    }
    let python = frage(&json!({"fn": "zeichensatz.erlaubt", "von": 0, "bis": 0x10_FFFF}));
    let python = python.as_str().unwrap();
    assert_eq!(
        python.chars().count(),
        0x11_0000,
        "eine Antwort je Codepunkt"
    );
    let mut abweichend = Vec::new();
    let mut erlaubt = 0;
    for (cp, p) in (0u32..).zip(python.chars()) {
        let r = match char::from_u32(cp) {
            None => '-',
            Some(c) if erstes_unerlaubtes_zeichen(&c.to_string()).is_none() => '1',
            Some(_) => '0',
        };
        erlaubt += usize::from(r == '1');
        if r != p {
            abweichend.push(format!("U+{cp:04X} rust={r} python={p}"));
        }
    }
    assert!(
        abweichend.is_empty(),
        "{} Abweichungen, erste: {:?}",
        abweichend.len(),
        &abweichend[..abweichend.len().min(20)]
    );
    // Gegenprobe, dass der Vergleich etwas gesehen hat: 186 Zeichen, nicht 0 von 0.
    assert_eq!(erlaubt, 186);
}

/// Aussage 2: die Texte, wortgleich.
#[test]
fn meldungen_gleichen_python_wortgleich() {
    if skip() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht das Python-Umfeld)");
        return;
    }
    // Jedes Zeichen bis U+3000 (die Tabelle der Vorschlaege, die Namen und alle Bereiche liegen darin)
    // und Stichproben darueber: BOM, Ersatzzeichen, Emoji, hoechster Codepunkt, Mathe-Ziffer, privater
    // Bereich, Ende der BMP.
    let stichproben = [
        0x3001u32, 0xFEFF, 0xFFFD, 0xFFFF, 0xE000, 0x1_F600, 0x1_D7CE, 0x10_FFFF,
    ];
    let mut texte: Vec<String> = (0..=0x3000u32)
        .chain(stichproben)
        .filter_map(char::from_u32)
        .map(|c| format!("a{c}b"))
        .collect();
    // Das ERSTE unerlaubte Zeichen zaehlt, nicht das letzte und nicht das haeufigste.
    texte.extend(
        [
            "Maier–Müller\u{a0}Anna",
            "Wałęsa",
            "Müller\u{2013}Straße 5 €",
            "O’Brien „Zitat“",
            "Zeile eins\nZeile zwei\tTab",
            "Mu\u{308}ller",
            "ÄÖÜäöüß € Œuvre",
        ]
        .map(str::to_owned),
    );
    let mut abweichend = Vec::new();
    for stueck in texte.chunks(2000) {
        let antwort = frage(&json!({
            "fn": "zeichensatz.meldungen", "feld_id": FELD, "element": ELEMENT, "texte": stueck,
        }));
        let antwort = antwort.as_array().unwrap();
        assert_eq!(antwort.len(), stueck.len());
        for (text, py) in stueck.iter().zip(antwort) {
            let c = erstes_unerlaubtes_zeichen(text);
            match (c, py.as_object()) {
                (None, None) => {}
                (Some(c), Some(py)) => {
                    let feld = feld_meldung(FELD, c);
                    let element = element_meldung(ELEMENT, text).unwrap();
                    if py["feld"] != feld {
                        abweichend.push(format!(
                            "{text:?} feld: rust={feld:?} python={}",
                            py["feld"]
                        ));
                    }
                    if py["element"] != element {
                        abweichend.push(format!(
                            "{text:?} element: rust={element:?} python={}",
                            py["element"]
                        ));
                    }
                    // Store: bei Zeichensatz der Wortlaut, bei Steuerzeichen (Auflage T) die Klasse.
                    let store_py = &py["store"];
                    match (rust_store(text), store_py["klasse"].as_str()) {
                        (
                            Some(a @ Abweisung::ZeichensatzVerletzt { .. }),
                            Some("ZeichensatzVerletzt"),
                        ) => {
                            if store_py["msg"] != a.to_string() {
                                abweichend.push(format!(
                                    "{text:?} store: rust={:?} python={}",
                                    a.to_string(),
                                    store_py["msg"]
                                ));
                            }
                        }
                        (Some(Abweisung::TypInkonform { .. }), Some("Typ")) => {}
                        (r, p) => abweichend
                            .push(format!("{text:?} store-Klasse: rust={r:?} python={p:?}")),
                    }
                }
                (c, py) => abweichend.push(format!("{text:?}: rust={c:?} python={py:?}")),
            }
        }
    }
    assert!(
        abweichend.is_empty(),
        "{} Abweichungen von {} Texten, erste: {:#?}",
        abweichend.len(),
        texte.len(),
        &abweichend[..abweichend.len().min(10)]
    );
}
