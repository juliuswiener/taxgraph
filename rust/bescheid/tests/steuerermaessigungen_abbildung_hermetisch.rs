//! § 35a `EStG`: die Abbildung Feld -> Eingabe in `abzuege.rs::steuerermaessigungen` im Standardlauf (ohne
//! `PARITY=1`, ohne Python).
//!
//! `p35a_haushaltsnahe` (Crate `engine`) rechnet richtig, wenn man ihr die richtigen sieben Werte gibt
//! (`rust/engine/tests/haushaltsnahe_hermetisch.rs`). Dass die Bescheid-Schicht ihr die richtigen Werte
//! gibt, prueft bisher nur Parity (`bescheid_blatt_paritaet.rs`, nur mit `PARITY=1`; die CI faehrt Parity
//! nicht). Gemessen am 2026-10-03 auf e50f7d36: 21 Mutationen an `hh_summe` und `steuerermaessigungen`
//! (Bericht h8-hermetisch2). 17 lassen `cargo test -p bescheid` gruen (163 passed, 0 failed). 3 werden nur
//! zufaellig rot: sie loeschen die einzige Lesestelle eines Feld-Kennung-Literals, und die Mindestzahl in
//! `feld_kennung_gate.rs` (141 < 142) schlaegt an, nicht das Verhalten. 1 (`cent_zu_euro` rundet auf) faengt
//! schon der Bestand (5 Tests). Mit diesen Tests werden alle 21 rot, jede mit 1 bis 3 der drei Tests.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl ist die Ausgabe des Python-Orakels
//! `bescheid_abzuege._shared_steuer_sonder_agb` (`steuerermaessigungen`, ueber `tools/parity/bescheid_oracle`
//! mit leerer Wegwerf-Datenwurzel) auf denselben Events UND stimmt mit der Gesetzes-Arithmetik aus den
//! GEMEINTEN Topfsummen ueberein (20 %, Deckel 510 / 4.000 / 1.200 EUR; Abs. 4 EU/EWR, Abs. 5 S. 3 unbare
//! Zahlung, Abs. 3 S. 2 Foerderung; `sources/gesetze-im-internet/estg_p35a_*.txt`). Das Orakel hat beides
//! gegeneinander geprueft und bricht bei einer Abweichung ab. Kein Wert ist aus dem Rust-Code abgelesen.
//!
//! Zahlen in den Events sind Cent (die Felder `hh_*_betrag` und `hh_*` sind Cent-Felder); `erwartet` ist die
//! Steuerermaessigung in EUR. Die Topfwerte der Gate-Faelle (1.000 / 5.000 / 3.000 EUR) liegen unter den
//! Deckeln, damit ein vertauschtes Feld die Zahl aendert und kein Deckel den Fehler schluckt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::abzuege::shared_steuer_sonder_agb;
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::Instanzquelle;
use domain::{Euro, Veranlagung, Vz};
use serde_json::{json, Value};

/// Ein Event `(feld_id, wert, bestaetigt)`.
type Ev = (&'static str, Value, bool);

/// Ein Ja/Nein-Feld, bestaetigt.
fn b(fid: &'static str, wert: bool) -> Ev {
    (fid, json!(wert), true)
}

/// Ein Zahlfeld (Cent).
fn z(fid: &'static str, wert: i64, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein handgebauter Fall; `erwartet` in EUR ist die Ausgabe des Python-Orakels.
struct Fall {
    gruppe: &'static str,
    name: &'static str,
    events: Vec<Ev>,
    /// Python `store_uebergeben=false`: kein Store, nur die Instanz-1-Basis aus den Feldern.
    ohne_store: bool,
    nur_bestaetigt: bool,
    erwartet: i64,
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn faelle() -> Vec<Fall> {
    vec![
        Fall {
            gruppe: "gates",
            name: "alle Gates offen, drei Toepfe (Minijob 1.000, Dienstleistung 5.000, Handwerker 3.000 EUR)",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_minijob_betrag", 100_000, true),
                z("hh_dienstleistung_betrag", 500_000, true),
                z("hh_handwerker_betrag", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 1800,
        },
        Fall {
            gruppe: "gates",
            name: "EU/EWR ausdruecklich nein: alles 0",
            events: vec![
                b("hh_in_eu_ewr", false),
                b("hh_rechnung_unbar", true),
                z("hh_minijob_betrag", 100_000, true),
                z("hh_dienstleistung_betrag", 500_000, true),
                z("hh_handwerker_betrag", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 0,
        },
        Fall {
            gruppe: "gates",
            name: "EU/EWR unbeantwortet: alles 0 (nur ein ausdrueckliches Ja zaehlt)",
            events: vec![
                b("hh_rechnung_unbar", true),
                z("hh_minijob_betrag", 100_000, true),
                z("hh_dienstleistung_betrag", 500_000, true),
                z("hh_handwerker_betrag", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 0,
        },
        Fall {
            gruppe: "gates",
            name: "unbare Rechnung ausdruecklich nein: nur der Minijob",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", false),
                z("hh_minijob_betrag", 100_000, true),
                z("hh_dienstleistung_betrag", 500_000, true),
                z("hh_handwerker_betrag", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 200,
        },
        Fall {
            gruppe: "gates",
            name: "unbare Rechnung unbeantwortet: nur der Minijob",
            events: vec![
                b("hh_in_eu_ewr", true),
                z("hh_minijob_betrag", 100_000, true),
                z("hh_dienstleistung_betrag", 500_000, true),
                z("hh_handwerker_betrag", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 200,
        },
        Fall {
            gruppe: "gates",
            name: "Handwerker oeffentlich gefoerdert (keine_foerderung = nein): Handwerker faellt weg",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                b("hh_handwerker_keine_foerderung", false),
                z("hh_minijob_betrag", 100_000, true),
                z("hh_dienstleistung_betrag", 500_000, true),
                z("hh_handwerker_betrag", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 1200,
        },
        Fall {
            gruppe: "gates",
            name: "keine_foerderung = ja: Handwerker zaehlt",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                b("hh_handwerker_keine_foerderung", true),
                z("hh_minijob_betrag", 100_000, true),
                z("hh_dienstleistung_betrag", 500_000, true),
                z("hh_handwerker_betrag", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 1800,
        },
        Fall {
            gruppe: "gates",
            name: "Mitveranlagung ja: Cent-Summe halbiert",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                b("p35a_mitveranlagung", true),
                z("hh_minijob_betrag", 100_000, true),
                z("hh_dienstleistung_betrag", 500_000, true),
                z("hh_handwerker_betrag", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 900,
        },
        Fall {
            gruppe: "gates",
            name: "Mitveranlagung ausdruecklich nein: nicht halbiert",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                b("p35a_mitveranlagung", false),
                z("hh_minijob_betrag", 100_000, true),
                z("hh_dienstleistung_betrag", 500_000, true),
                z("hh_handwerker_betrag", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 1800,
        },
        Fall {
            gruppe: "betraege",
            name: "zwei Handwerker-Instanzen werden summiert (1.000 + 2.000 EUR)",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_handwerker_betrag", 100_000, true),
                z("hh_handwerker_betrag__2", 200_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 600,
        },
        Fall {
            gruppe: "betraege",
            name: "Minijob (2 Instanzen) und Dienstleistung (3 Instanzen) getrennt summiert",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_minijob_betrag", 50_000, true),
                z("hh_minijob_betrag__2", 50_000, true),
                z("hh_dienstleistung_betrag", 100_000, true),
                z("hh_dienstleistung_betrag__2", 100_000, true),
                z("hh_dienstleistung_betrag__3", 100_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 800,
        },
        Fall {
            gruppe: "betraege",
            name: "unbestaetigte zweite Instanz zaehlt bei nur_bestaetigt nicht",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_handwerker_betrag", 100_000, true),
                z("hh_handwerker_betrag__2", 200_000, false),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 200,
        },
        Fall {
            gruppe: "betraege",
            name: "dieselbe Lage ohne nur_bestaetigt: die zweite Instanz zaehlt",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_handwerker_betrag", 100_000, true),
                z("hh_handwerker_betrag__2", 200_000, false),
            ],
            ohne_store: false,
            nur_bestaetigt: false,
            erwartet: 600,
        },
        Fall {
            gruppe: "betraege",
            name: "nur Flat-Werte (Bestandsdaten): Rueckfall auf die Summenfelder",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_minijob_aufwendungen", 100_000, true),
                z("hh_dienstleistungen", 500_000, true),
                z("hh_handwerker_arbeitskosten", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 1800,
        },
        Fall {
            gruppe: "betraege",
            name: "Instanz schlaegt Flat-Wert (Handwerker 1.000 EUR Instanz, 9.000 EUR Flat)",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_handwerker_betrag", 100_000, true),
                z("hh_handwerker_arbeitskosten", 900_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 200,
        },
        Fall {
            gruppe: "betraege",
            name: "Instanz mit Betrag 0 faellt auf den Flat-Wert zurueck",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_handwerker_betrag", 0, true),
                z("hh_handwerker_arbeitskosten", 300_000, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 600,
        },
        Fall {
            gruppe: "betraege",
            name: "ohne Store: nur das Betragsfeld der Instanz 1 (Flat-Wert 9.000 EUR zaehlt nicht)",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_handwerker_betrag", 100_000, true),
                z("hh_handwerker_arbeitskosten", 900_000, true),
            ],
            ohne_store: true,
            nur_bestaetigt: true,
            erwartet: 200,
        },
        Fall {
            gruppe: "betraege",
            name: "ohne Store, nur Flat-Wert: 0",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_handwerker_arbeitskosten", 900_000, true),
            ],
            ohne_store: true,
            nur_bestaetigt: true,
            erwartet: 0,
        },
        Fall {
            gruppe: "cent",
            name: "Minijob 124,99 EUR: Euro abgerundet, nicht gerundet",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_minijob_betrag", 12_499, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 24,
        },
        Fall {
            gruppe: "cent",
            name: "Dienstleistung 1.234,99 EUR",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_dienstleistung_betrag", 123_499, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 246,
        },
        Fall {
            gruppe: "cent",
            name: "Handwerker 1.004,99 EUR",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_handwerker_betrag", 100_499, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 200,
        },
        Fall {
            gruppe: "cent",
            name: "alle drei Toepfe mit Cent-Rest",
            events: vec![
                b("hh_in_eu_ewr", true),
                b("hh_rechnung_unbar", true),
                z("hh_minijob_betrag", 12_499, true),
                z("hh_dienstleistung_betrag", 123_499, true),
                z("hh_handwerker_betrag", 100_499, true),
            ],
            ohne_store: false,
            nur_bestaetigt: true,
            erwartet: 472,
        },
    ]
}

/// `steuerermaessigungen` (EUR) fuer den Fall, ueber den oeffentlichen Weg `shared_steuer_sonder_agb`.
fn rechne(fall: &Fall) -> i64 {
    let st = store(&fall.events);
    // Bei `nur_bestaetigt` ist `f` schon auf bestaetigte Felder gefiltert (`bescheid_zweige.py:1487`,
    // `bescheid_oracle._ctx`); der Store selbst behaelt alle Events und liefert die Instanzen.
    let f = if fall.nur_bestaetigt {
        let bestaetigt: Vec<Ev> = fall.events.iter().filter(|e| e.2).cloned().collect();
        felder(&store(&bestaetigt))
    } else {
        felder(&st)
    };
    let q = Instanzquelle {
        store: (!fall.ohne_store).then_some(&st),
        bindung: Some(index()),
        nur_bestaetigt: fall.nur_bestaetigt,
    };
    shared_steuer_sonder_agb(
        Euro::new(0),
        Euro::new(0),
        Veranlagung::Einzel,
        &f,
        Vz::Vz2025,
        &q,
        params(),
    )
    .unwrap()
    .steuerermaessigungen
    .get()
}

/// Alle Faelle der Gruppe durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn pruefe(gruppe: &str) {
    let faelle: Vec<Fall> = faelle()
        .into_iter()
        .filter(|f| f.gruppe == gruppe)
        .collect();
    assert!(!faelle.is_empty(), "Gruppe {gruppe} ist leer");
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|f| {
            let got = rechne(f);
            (got != f.erwartet).then(|| format!("{}: Rust {got}, Orakel {}", f.name, f.erwartet))
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "{} von {} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len(),
        faelle.len()
    );
}

/// Die vier Ja/Nein-Felder kommen richtig an: EU/EWR (Abs. 4), unbare Rechnung (Abs. 5 S. 3), Foerderung
/// (Abs. 3 S. 2: nur ein AUSDRUECKLICHES Nein zu `keine_foerderung` heisst gefoerdert) und Mitveranlagung.
/// Unbeantwortet zaehlt nie als Ja.
#[test]
fn ja_nein_felder_kommen_an() {
    pruefe("gates");
}

/// Die Betraege kommen richtig an: Instanzen werden je Topf summiert, unbestaetigte zaehlen bei
/// `nur_bestaetigt` nicht, der Flat-Wert (Bestandsdaten) gilt nur ohne Instanzsumme, ohne Store zaehlt nur das
/// Betragsfeld der Instanz 1.
#[test]
fn betraege_instanzen_und_flat_rueckfall() {
    pruefe("betraege");
}

/// Cent werden zu ganzen Euro ABGERUNDET (Python `// 100`), nicht gerundet und nicht ungeteilt uebergeben.
/// Die Betraege (124,99 / 1.234,99 / 1.004,99 EUR) sind so gewaehlt, dass ein Aufrunden die abgerundete
/// Gesamtsumme um 1 EUR verschiebt.
#[test]
fn cent_werden_zu_euro_abgerundet() {
    pruefe("cent");
}
