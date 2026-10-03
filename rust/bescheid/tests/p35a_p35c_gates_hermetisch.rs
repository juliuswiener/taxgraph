//! § 35a und § 35c `EStG`: die Sperre `p35a_p35c` und die Umkehrung `p35c_foerderung_in_anspruch` in
//! `mit_ring_werten` im Standardlauf (ohne `PARITY=1`, ohne Python).
//!
//! Die Sperre verwandelt eine fehlende Antwort in einen sichtbaren Grund (`rechnung_unbar_offen`,
//! `handwerker_foerderung_offen`, `haushalt_eu_ewr_offen`, `p35c_doppelfoerderung_offen`), statt dass `p35a_haushaltsnahe`
//! und `p35c` still 0 rechnen. Ihre Gegenprobe lief bisher nur in `rust/parity/tests/bescheid_deklaration_paritaet.rs` und
//! damit nur mit `PARITY=1`; die CI faehrt Parity nicht. Gemessen am 2026-10-03 auf 57944f48 (Bericht h8-hermetisch3): drei
//! Mutationen am Aufrufort lassen `cargo test -p bescheid` gruen (181 passed, 0 failed, 10 ignored): in
//! `deklaration/sperre/gesamt.rs::p35a_p35c` (S1) das EU/EWR-Gate ohne den Minijob-Topf und (S2) die § 35c-Sperre ohne
//! den Energieberater-Aufwand, in `deklaration/ring_werte.rs` (W1) `p35c_foerderung_in_anspruch` ohne die Umkehrung. Mit
//! diesen Tests wird jede der drei rot, und zwar mit genau dem Test, der ihre Stelle prueft.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zeile der Tabellen unten ist die Ausgabe des Python-Orakels auf denselben Events:
//! `bescheid_deklaration._an_gesamt_sperrgrund(felder, SCHEIBEN["gesamt"], 2025, store, scheiben_bindung)` (mit Store und
//! Scheiben-Bindung, wie der Rust-Aufruf; ueber `tools/parity/bescheid_oracle`, Wegwerf-Datenwurzel) bzw.
//! `bescheid_deklaration._mit_ring_werten`. Aus dem Gesetz stammen nur die Voraussetzungen, die den Abzug tragen
//! (§ 35a Abs. 3 S. 2 Foerderung, Abs. 4 S. 1 EU/EWR, Abs. 5 S. 3 Rechnung und unbare Zahlung, § 35c Abs. 3 S. 2
//! Doppelfoerderung; `sources/gesetze-im-internet/estg_p35a_2026-07-09.txt`, `estg_p35c_2026-07-13.txt`). DASS eine
//! unbeantwortete Frage sperrt, ist Entwurf des Projekts, nicht Gesetzeswortlaut: dort stuetzt nur das Python-Orakel.
//! Kein Wert ist aus dem Rust-Code abgelesen.
//!
//! Kontrollfaelle (alles beantwortet, Betrag 0, kein Betrag) zeigen, dass die Sperre nicht alles sperrt: ohne sie bestuende
//! auch ein Guard, der immer einen Grund liefert, jeden Fall.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::sync::OnceLock;

use bescheid::deklaration::{an_gesamt_sperrgrund, mit_ring_werten, Cfg};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::{BindungIndex, Instanzquelle};
use domain::{PyWert, Scheibe, Sperrgrund, Vz};
use serde_json::{json, Value};

/// Ein Event `(feld_id, wert, bestaetigt)`.
type Ev = (&'static str, Value, bool);

/// Ein Ja/Nein-Feld; `bestaetigt = false` ist ein vorlaeufiger Wert.
fn b(fid: &'static str, wert: bool, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein Zahlfeld (Cent); `bestaetigt = false` ist ein vorlaeufiger Wert.
fn z(fid: &'static str, wert: i64, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein handgebauter Fall der Sperre: Events und der Grund, den das Python-Orakel liefert.
struct SperrFall {
    gruppe: &'static str,
    name: &'static str,
    events: Vec<Ev>,
    erwartet: Option<&'static str>,
}

/// Ein handgebauter Fall der Ring-Werte: die Felder, die `mit_ring_werten` neu setzt oder aendert.
struct RingFall {
    name: &'static str,
    events: Vec<Ev>,
    erwartet: Vec<(&'static str, PyWert)>,
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn sperr_faelle() -> Vec<SperrFall> {
    vec![
        SperrFall {
            gruppe: "minijob",
            name: "Minijob 1.000 EUR, EU/EWR unbeantwortet",
            events: vec![z("hh_minijob_betrag", 100_000, true)],
            erwartet: Some("haushalt_eu_ewr_offen"),
        },
        SperrFall {
            gruppe: "minijob",
            name: "Minijob 1.000 EUR, EU/EWR ja",
            events: vec![z("hh_minijob_betrag", 100_000, true), b("hh_in_eu_ewr", true, true)],
            erwartet: None,
        },
        SperrFall {
            gruppe: "minijob",
            name: "Minijob 1.000 EUR, EU/EWR ausdruecklich nein (eine Antwort)",
            events: vec![z("hh_minijob_betrag", 100_000, true), b("hh_in_eu_ewr", false, true)],
            erwartet: None,
        },
        SperrFall {
            gruppe: "minijob",
            name: "Minijob 1.000 EUR, EU/EWR nur vorlaeufig ja",
            events: vec![z("hh_minijob_betrag", 100_000, true), b("hh_in_eu_ewr", true, false)],
            erwartet: Some("haushalt_eu_ewr_offen"),
        },
        SperrFall {
            gruppe: "hh",
            name: "Dienstleistung 5.000 EUR ohne Antwort zur unbaren Rechnung",
            events: vec![z("hh_dienstleistung_betrag", 500_000, true), b("hh_in_eu_ewr", true, true)],
            erwartet: Some("rechnung_unbar_offen"),
        },
        SperrFall {
            gruppe: "hh",
            name: "Dienstleistung 5.000 EUR, unbar ja, EU/EWR unbeantwortet",
            events: vec![z("hh_dienstleistung_betrag", 500_000, true), b("hh_rechnung_unbar", true, true)],
            erwartet: Some("haushalt_eu_ewr_offen"),
        },
        SperrFall {
            gruppe: "hh",
            name: "Handwerker 3.000 EUR ohne Antwort zur Foerderung",
            events: vec![z("hh_handwerker_betrag", 300_000, true), b("hh_rechnung_unbar", true, true), b("hh_in_eu_ewr", true, true)],
            erwartet: Some("handwerker_foerderung_offen"),
        },
        SperrFall {
            gruppe: "hh",
            name: "Handwerker 3.000 EUR ohne Antwort zur unbaren Rechnung",
            events: vec![z("hh_handwerker_betrag", 300_000, true), b("hh_in_eu_ewr", true, true), b("hh_handwerker_keine_foerderung", true, true)],
            erwartet: Some("rechnung_unbar_offen"),
        },
        SperrFall {
            gruppe: "hh",
            name: "Handwerker 3.000 EUR, alles beantwortet",
            events: vec![z("hh_handwerker_betrag", 300_000, true), b("hh_rechnung_unbar", true, true), b("hh_in_eu_ewr", true, true), b("hh_handwerker_keine_foerderung", true, true)],
            erwartet: None,
        },
        SperrFall {
            gruppe: "hh",
            name: "Dienstleistung nur in Instanz 2 (Instanz 1 = 0), unbar unbeantwortet",
            events: vec![z("hh_dienstleistung_betrag", 0, true), z("hh_dienstleistung_betrag__2", 100_000, true), b("hh_in_eu_ewr", true, true)],
            erwartet: Some("rechnung_unbar_offen"),
        },
        SperrFall {
            gruppe: "hh",
            name: "Handwerker nur in Instanz 2, Foerderung unbeantwortet, unbar ja",
            events: vec![z("hh_handwerker_betrag", 0, true), z("hh_handwerker_betrag__2", 100_000, true), b("hh_rechnung_unbar", true, true), b("hh_in_eu_ewr", true, true)],
            erwartet: Some("handwerker_foerderung_offen"),
        },
        SperrFall {
            gruppe: "hh",
            name: "Minijob nur in Instanz 2, EU/EWR unbeantwortet",
            events: vec![z("hh_minijob_betrag", 0, true), z("hh_minijob_betrag__2", 100_000, true)],
            erwartet: Some("haushalt_eu_ewr_offen"),
        },
        SperrFall {
            gruppe: "hh",
            name: "kein hh-Betrag, keine Antworten: keine Sperre",
            events: vec![],
            erwartet: None,
        },
        SperrFall {
            gruppe: "p35c",
            name: "Sanierung 20.000 EUR, Doppelfoerderung unbeantwortet",
            events: vec![z("p35c_sanierungsaufwendungen", 2_000_000, true)],
            erwartet: Some("p35c_doppelfoerderung_offen"),
        },
        SperrFall {
            gruppe: "p35c",
            name: "Energieberater 1.000 EUR, Doppelfoerderung unbeantwortet",
            events: vec![z("p35c_energieberater_aufwendungen", 100_000, true)],
            erwartet: Some("p35c_doppelfoerderung_offen"),
        },
        SperrFall {
            gruppe: "p35c",
            name: "Energieberater 1.000 EUR, Doppelfoerderung nur vorlaeufig beantwortet",
            events: vec![z("p35c_energieberater_aufwendungen", 100_000, true), b("p35c_keine_doppelfoerderung", true, false)],
            erwartet: Some("p35c_doppelfoerderung_offen"),
        },
        SperrFall {
            gruppe: "p35c",
            name: "Sanierung 20.000 EUR, keine Doppelfoerderung bestaetigt ja",
            events: vec![z("p35c_sanierungsaufwendungen", 2_000_000, true), b("p35c_keine_doppelfoerderung", true, true)],
            erwartet: None,
        },
        SperrFall {
            gruppe: "p35c",
            name: "Sanierung 20.000 EUR, Doppelfoerderung bestaetigt (keine_doppelfoerderung = nein): eine Antwort",
            events: vec![z("p35c_sanierungsaufwendungen", 2_000_000, true), b("p35c_keine_doppelfoerderung", false, true)],
            erwartet: None,
        },
        SperrFall {
            gruppe: "p35c",
            name: "Energieberater 0 EUR, Doppelfoerderung unbeantwortet: keine Sperre",
            events: vec![z("p35c_energieberater_aufwendungen", 0, true)],
            erwartet: None,
        },
    ]
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn ring_faelle() -> Vec<RingFall> {
    vec![
        RingFall {
            name: "keine Doppelfoerderung ja bestaetigt: p35c_foerderung_in_anspruch = nein",
            events: vec![b("p35c_keine_doppelfoerderung", true, true)],
            erwartet: vec![("p35c_foerderung_in_anspruch", PyWert::Bool(false))],
        },
        RingFall {
            name: "keine Doppelfoerderung nein bestaetigt: p35c_foerderung_in_anspruch = ja",
            events: vec![b("p35c_keine_doppelfoerderung", false, true)],
            erwartet: vec![("p35c_foerderung_in_anspruch", PyWert::Bool(true))],
        },
        RingFall {
            name: "keine Doppelfoerderung nur vorlaeufig: nichts",
            events: vec![b("p35c_keine_doppelfoerderung", true, false)],
            erwartet: vec![],
        },
        RingFall {
            name: "Sanierung 20.000 EUR und keine Doppelfoerderung ja: Einzelbetrag und Umkehrung",
            events: vec![
                z("p35c_sanierungsaufwendungen", 2_000_000, true),
                b("p35c_keine_doppelfoerderung", true, true),
            ],
            erwartet: vec![
                ("p35c_foerderung_in_anspruch", PyWert::Bool(false)),
                ("p35c_massnahme_einzelbetrag", PyWert::Ganz(2_000_000)),
            ],
        },
        RingFall {
            name: "Sanierung nur vorlaeufig: nichts",
            events: vec![z("p35c_sanierungsaufwendungen", 2_000_000, false)],
            erwartet: vec![],
        },
    ]
}

/// Die Bindung der Scheibe `gesamt`: nur ihre Feld-Ids, wie `api._scheibe_bindung`. Der volle Index kennt mehr
/// Flags und liesse `flag_widersprueche` auf Person B ansprechen.
fn scheiben_index() -> &'static BindungIndex<'static> {
    static GESAMT: OnceLock<BindungIndex<'static>> = OnceLock::new();
    GESAMT.get_or_init(|| {
        let ids = Cfg::fuer(Scheibe::Gesamt).felder(|_| Vec::new()).unwrap();
        index()
            .iter()
            .filter(|(k, _)| ids.contains(k))
            .map(|(k, b)| (k.clone(), *b))
            .collect()
    })
}

/// Der Sperrgrund der Scheibe `gesamt` fuer VZ 2025 auf einem Store aus den Events (Store UND Scheiben-Bindung,
/// also Instanz-Summen wie im Betrieb).
fn grund(events: &[Ev]) -> Option<&'static str> {
    let st = store(events);
    let f = felder(&st);
    let q = Instanzquelle {
        store: Some(&st),
        bindung: Some(scheiben_index()),
        nur_bestaetigt: false,
    };
    let cfg = Cfg::fuer(Scheibe::Gesamt);
    an_gesamt_sperrgrund(&f, Some(&cfg), Some(Vz::Vz2025), &q)
        .unwrap()
        .map(Sperrgrund::als_str)
}

/// Alle Faelle der Gruppen durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn pruefe_sperre(gruppen: &[&str]) {
    let faelle: Vec<SperrFall> = sperr_faelle()
        .into_iter()
        .filter(|f| gruppen.contains(&f.gruppe))
        .collect();
    assert!(!faelle.is_empty(), "Gruppen {gruppen:?} sind leer");
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|f| {
            let got = grund(&f.events);
            (got != f.erwartet)
                .then(|| format!("{}: Rust {got:?}, Orakel {:?}", f.name, f.erwartet))
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "{} von {} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len(),
        faelle.len()
    );
}

/// § 35a Abs. 1 bis 5 `EStG`: ein Betrag ohne die Antwort, die den Abzug erst tragen darf, sperrt. Unbare Rechnung
/// (Abs. 5 S. 3) nur bei Dienstleistung oder Handwerker, nie beim Minijob; Foerderung (Abs. 3 S. 2) nur beim
/// Handwerker; EU/EWR (Abs. 4) bei allen drei Toepfen, also auch beim Minijob allein. Ein ausdrueckliches Nein ist
/// eine Antwort, ein nur vorlaeufiges Ja nicht. Der Betrag zaehlt in jeder Instanz.
#[test]
fn p35a_ohne_antwort_sperrt_je_topf() {
    pruefe_sperre(&["minijob", "hh"]);
}

/// § 35c Abs. 3 S. 2 `EStG`: ein Betrag der Sanierung ODER des Energieberaters ohne bestaetigte Antwort zur
/// Doppelfoerderung sperrt; jede bestaetigte Antwort (Ja wie Nein) und ein Betrag von 0 sperren nicht.
#[test]
fn p35c_ohne_antwort_zur_doppelfoerderung_sperrt() {
    pruefe_sperre(&["p35c"]);
}

/// `mit_ring_werten`, Abschnitt Einzelzeilen: `p35c_foerderung_in_anspruch` ist die UMKEHRUNG der bestaetigten Antwort
/// `p35c_keine_doppelfoerderung` (ELSTER-Regel E0240902 fragt umgekehrt zu unserem Gate); ein vorlaeufiger Wert setzt nichts.
#[test]
fn p35c_foerderung_in_anspruch_ist_die_umkehrung() {
    let faelle = ring_faelle();
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|fall| {
            let st = store(&fall.events);
            let vorher = felder(&st);
            let mut nachher = vorher.clone();
            mit_ring_werten(&mut nachher, Some(Vz::Vz2025), params()).unwrap();
            let mut got: Vec<(String, PyWert)> = nachher
                .iter()
                .filter(|(k, v)| vorher.get(*k).map(|x| &x.wert) != Some(&v.wert))
                .map(|(k, v)| (k.clone(), v.wert.clone()))
                .collect();
            got.sort_by(|a, b| a.0.cmp(&b.0));
            let mut erwartet: Vec<(String, PyWert)> = fall
                .erwartet
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect();
            erwartet.sort_by(|a, b| a.0.cmp(&b.0));
            (got != erwartet).then(|| format!("{}: Rust {got:?}, Orakel {erwartet:?}", fall.name))
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "{} von {} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len(),
        faelle.len()
    );
}
