//! Ueberlaufpfade der Zweige und des Slot-Rahmens der Bescheid-Schicht, im Standardlauf (ohne `PARITY=1`, ohne Python):
//! `p35_gezahlte_gewst` (Messbetrag mal Hebesatz je Betrieb) und die Slot-Summe in `feste_zahl` melden
//! `BescheidFehler::Ueberlauf(<Stelle>)` statt eine abgeschnittene Zahl oder einen Python-Fehler.
//!
//! Python rechnet mit beliebig grossen Ganzzahlen; Rust rechnet in `i64`/`i128` mit `checked_*` und meldet `Ueberlauf` (fail-closed,
//! `python_klasse() == None`, Modul-Doku `bescheid/src/lib.rs`). Gemessen am 2026-10-04 auf 0aa91677 (Bericht h8-hermetisch5, je eine
//! Mutation am Aufrufort): B1 `rechnen::mal` (`checked_mul` -> `wrapping_mul`), B2 `rechnen::mal_div` (`try_from` -> `as i64`),
//! B3 `tarif` `4 * q_eur` (`checked_mul` -> `wrapping_mul`), B4 `feste_zahl::slot_fehler` (`SlotFehler::Ueberlauf` -> `Python {
//! ValueError }`), B5 `einkuenfte::gewst_je_betrieb` (`try_from` -> `as i64`) lassen `cargo test -p bescheid -p api` gruen (370 passed,
//! 0 failed, 11 ignored). Mit diesen Tests werden B1, B2, B4 und B5 rot, jede mit dem Test, der ihre Stelle prueft; B3 bleibt gruen,
//! weil KEIN Eingang die Stelle erreicht (Begruendung am Test `kapital_kist_basis_meldet_ueberlauf_und_rechnet_knapp`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: der Python-Wert jedes Falls ist die Ausgabe des Python-Orakels (`orakel_b5.py`: Python
//! `_p35_gezahlte_gewst` mit unbeschraenkten Ganzzahlen; `orakel_bs5.py`: `api.ergebnis` im selben Prozess, `FAELLE` auf ein
//! Wegwerf-Verzeichnis, Pflicht-Kegel ueber `tests/_kegel.py`, VZ 2025; fuer die Kinder-Faelle mit abgeschalteter Bereichspruefung
//! des Stores, `OHNE_BEREICH=1`; Skripte und Laeufe: Anlagen zum Bericht). Jeder Fall traegt
//! eine von drei Marken im Namen: "Python-Wert ausserhalb i64" (klar): es gibt keinen `i64`-Wert, Rust MUSS `Ueberlauf` melden.
//! "nur ein Zwischenprodukt ausserhalb i64" (zwischen): Python liefert einen gueltigen Wert, Rust meldet `Ueberlauf`; das ist die
//! dokumentierte fail-closed-Konvention von Rust und KEINE Python-Stuetze (nie eine falsche Zahl). "passt gerade noch" (knapp):
//! alles passt, Rust MUSS den Python-Wert liefern.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::HashMap;

use bescheid::deklaration::{an_gesamt_sperrgrund, feste_zahl, Cfg};
use bescheid::einkuenfte::p35_gezahlte_gewst;
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Umgebung;
use bescheid::{BescheidFehler, Felder, Instanzquelle};
use bindung::Bindung;
use domain::{Euro, Feldtyp, Scheibe, Vz};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;

/// Die Erwartung eines Falls: der Python-Wert, wenn er in `i64` liegt, sonst die Marke der Stelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Erw {
    Zahl(i64),
    Ueberlauf(&'static str),
}

fn ist(r: Result<Euro, BescheidFehler>) -> Result<Erw, String> {
    match r {
        Ok(e) => Ok(Erw::Zahl(e.get())),
        Err(BescheidFehler::Ueberlauf(m)) => Ok(Erw::Ueberlauf(m)),
        Err(e) => Err(format!("anderer Fehler: {e:?}")),
    }
}

/// B5: `(Name, messbetrag_a, hebesatz_a, messbetrag_b, hebesatz_b, Erwartung)`; Name = Art + Fall. Python:
/// `messbetrag_a * hebesatz_a // 100 + messbetrag_b * hebesatz_b // 100` (`_p35_gezahlte_gewst`).
#[allow(clippy::type_complexity)]
const GEWST: [(&str, i64, i64, i64, i64, Erw); 17] = [
    ("knapp: A max x 100 %", MAX, 100, 0, 0, Erw::Zahl(MAX)),
    (
        "klar: A max x 101 %",
        MAX,
        101,
        0,
        0,
        Erw::Ueberlauf("Messbetrag*Hebesatz"),
    ),
    ("knapp: A min x 100 %", MIN, 100, 0, 0, Erw::Zahl(MIN)),
    (
        "klar: A min x 101 %",
        MIN,
        101,
        0,
        0,
        Erw::Ueberlauf("Messbetrag*Hebesatz"),
    ),
    (
        "knapp: A 1 x Hebesatz max",
        1,
        MAX,
        0,
        0,
        Erw::Zahl(92_233_720_368_547_758),
    ),
    (
        "klar: A max x Hebesatz max",
        MAX,
        MAX,
        0,
        0,
        Erw::Ueberlauf("Messbetrag*Hebesatz"),
    ),
    (
        "knapp: A -1 x 50 % (Floor, -1 statt 0)",
        -1,
        50,
        0,
        0,
        Erw::Zahl(-1),
    ),
    ("knapp: A max x -100 %", MAX, -100, 0, 0, Erw::Zahl(-MAX)),
    (
        "klar: A max x -101 %",
        MAX,
        -101,
        0,
        0,
        Erw::Ueberlauf("Messbetrag*Hebesatz"),
    ),
    (
        "klar: B max x 101 %",
        0,
        0,
        MAX,
        101,
        Erw::Ueberlauf("Messbetrag*Hebesatz"),
    ),
    ("knapp: B max x 100 %", 0, 0, MAX, 100, Erw::Zahl(MAX)),
    (
        "zwischen: A max x 101 % ausserhalb, Summe mit B -max x 100 % = 92233720368547758",
        MAX,
        101,
        -MAX,
        100,
        Erw::Ueberlauf("Messbetrag*Hebesatz"),
    ),
    (
        "klar: Summe max + 1 (je Betrieb passt)",
        MAX,
        100,
        1,
        100,
        Erw::Ueberlauf("Addition"),
    ),
    (
        "klar: Summe min - 1 (je Betrieb passt)",
        MIN,
        100,
        -1,
        100,
        Erw::Ueberlauf("Addition"),
    ),
    (
        "knapp: Summe max - 1 + 1",
        MAX - 1,
        100,
        1,
        100,
        Erw::Zahl(MAX),
    ),
    (
        "klar: Summe zweimal -max",
        MAX,
        -100,
        MAX,
        -100,
        Erw::Ueberlauf("Addition"),
    ),
    ("knapp: beide 0", 0, 0, 0, 0, Erw::Zahl(0)),
];

/// B5 (Treffer `gewst_je_betrieb`: `try_from` -> `as i64`): jeder Betrieb mit Ergebnis ausserhalb `i64` meldet die Marke
/// "Messbetrag*Hebesatz", nie ein abgeschnittener Wert; die Gegenproben am Rand rechnen den Python-Wert.
#[test]
fn gewerbesteuer_je_betrieb_meldet_ueberlauf_und_rechnet_knapp_richtig() {
    let mut falsch = Vec::new();
    for (name, ma, ha, mb, hb, erw) in GEWST {
        let r = ist(p35_gezahlte_gewst(Euro::new(ma), ha, Euro::new(mb), hb));
        if r != Ok(erw) {
            falsch.push(format!("{name}: {r:?}, erwartet {erw:?}"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

// ---------------------------------------------------------------- Slot-Summe in feste_zahl (B4)

const VZ: Vz = Vz::Vz2025;

/// `dict(paare)`: der letzte Wert je Feld gewinnt, die Reihenfolge ist die des ersten Auftretens.
fn als_dict(paare: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut raus: Vec<(&'static str, Value)> = Vec::new();
    for (f, w) in paare {
        match raus.iter_mut().find(|(g, _)| g == f) {
            Some(e) => e.1 = w.clone(),
            None => raus.push((f, w.clone())),
        }
    }
    raus
}

/// `tests/_kegel.py::standardwert`: der Abwesenheitswert, nie der illustrative Beispielwert.
fn standardwert(b: &Bindung) -> Value {
    if let Some(w) = &b.abwesenheitswert {
        return w.clone();
    }
    match b.typ {
        Feldtyp::Bool => json!(b.feld_id.starts_with("kein_") || b.feld_id.starts_with("keine_")),
        Feldtyp::Cent | Feldtyp::Int => json!(0),
        Feldtyp::Enum => b.beispielwert.clone(),
        _ => panic!(
            "KONTROLLE: Kegel-Feld {} hat keinen Abwesenheitswert",
            b.feld_id
        ),
    }
}

/// `tests/_kegel.py::kegel_fuer`: der volle Pflicht-Kegel; was der Test setzt, gewinnt.
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let gesetzt = als_dict(gesetzt);
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Vec<(&'static str, Value)> = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .find(|(g, _)| g == f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            (*f, w)
        })
        .collect();
    raus.extend(gesetzt.into_iter().filter(|(f, _)| !kegel.contains(f)));
    raus
}

/// Ein Fall der Scheibe `gesamt`, mit der Scheiben-Bindung (`api._scheibe_bindung`) als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

fn fall(paare: &[(&'static str, Value)]) -> Fall {
    let cfg = Cfg::fuer(Scheibe::Gesamt);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: gesamt liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let events: Vec<(&str, Value, bool)> = kegel_fuer(&cfg, paare)
        .into_iter()
        .map(|(f, w)| (f, w, true))
        .collect();
    let store = store(&events);
    Fall {
        cfg,
        index: teil.iter().map(|b| (b.feld_id.clone(), *b)).collect(),
        achsen: teil.iter().map(|b| AchsenBindung::from(*b)).collect(),
        felder: felder(&store),
        store,
    }
}

/// Der Ausgang von `_ergebnis_roh` (`api.py:568`) ohne HTTP: erst der K2-Guard, dann `feste_zahl`.
#[derive(Debug, PartialEq, Eq)]
enum Ausgang {
    Bestaetigt(i64),
    Gesperrt(String),
    OhneZahl(String),
    Ueberlauf(&'static str),
    Anders(String),
}

fn ergebnis(f: &Fall) -> Ausgang {
    if f.cfg.guard() {
        let q = Instanzquelle {
            store: Some(&f.store),
            bindung: Some(&f.index),
            nur_bestaetigt: false,
        };
        match an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(VZ), &q) {
            Ok(Some(g)) => return Ausgang::Gesperrt(format!("{g:?}")),
            Ok(None) => {}
            Err(BescheidFehler::Ueberlauf(m)) => return Ausgang::Ueberlauf(m),
            Err(e) => return Ausgang::Anders(format!("Guard: {e:?}")),
        }
    }
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, VZ, &kegel, &umg, Some(&f.store), None) {
        Ok(Ok(z)) => Ausgang::Bestaetigt(z.zahl.get()),
        Ok(Err(k)) => Ausgang::OhneZahl(format!("{:?}", k.grund)),
        Err(BescheidFehler::Ueberlauf(m)) => Ausgang::Ueberlauf(m),
        Err(e) => Ausgang::Anders(format!("{e:?}")),
    }
}

/// `_STAMM` der Gewinn-Tests (`test_gewinn_quelle_offen_trifft_alle_betriebsarten.py`): keine Kegel-Felder.
fn stamm_meier() -> Vec<(&'static str, Value)> {
    vec![
        ("stammdaten_nachname", json!("Meier")),
        ("stammdaten_vorname", json!("Klaus")),
        ("stammdaten_geburtsdatum", json!("01.01.1970")),
        ("stammdaten_strasse", json!("Teststr.")),
        ("stammdaten_hausnummer", json!("1")),
        ("stammdaten_plz", json!("10115")),
        ("stammdaten_wohnort", json!("Berlin")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_art_est_erklaerung", json!(true)),
        ("kist_konfession", json!("keine")),
    ]
}

/// `_BASIS + _PFLICHT_ZUSATZ + veranlagung + Betriebsart` derselben Python-Tests (Gewinneinkuenfte angegeben).
fn gewinn_basis() -> Vec<(&'static str, Value)> {
    let mut paare = stamm_meier();
    paare.extend([
        ("bruttoarbeitslohn", json!(0)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("kap_kapitalertraege", json!(0)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
        ("kein_gewinn", json!(false)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("basis_kv", json!(0)),
        ("basis_pv", json!(0)),
        ("versicherungsart", json!("gesetzlich_an")),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(false)),
        ("ep_arbeitstage", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("veranlagung", json!("einzel")),
        ("gewinn_betriebsart", json!("gewerbe")),
        ("gewinn_bezeichnung", json!("Testfall")),
    ]);
    paare
}

/// Ein Fall: `gewinn_basis` plus die gesetzten Felder (der letzte Wert je Feld gewinnt).
fn mit(gesetzt: &[(&'static str, Value)]) -> Fall {
    let mut paare = gewinn_basis();
    paare.extend_from_slice(gesetzt);
    fall(&paare)
}

/// Zwei Summanden desselben Slots (`slot_beitrag: summand`); `kein_gewinn` wahr, damit kein EUeR-Guard mitrechnet.
fn zwei(a: &'static str, x: i64, b: &'static str, y: i64) -> Fall {
    mit(&[("kein_gewinn", json!(true)), (a, json!(x)), (b, json!(y))])
}

/// EUeR-Cluster ohne Betriebseinnahmen: die beiden Summanden des Betriebsausgaben-Slots (`bindung_an_gesamt.yaml:457,475`);
/// `einkuenfte_gewinn` 0, damit weder `gewinn_quelle_offen` noch `luf_euer_offen` sperrt.
fn euer(sonstige_ausgaben: i64, afa: i64) -> Fall {
    mit(&[
        ("betriebseinnahmen", json!(0)),
        ("sonstige_betriebsausgaben", json!(sonstige_ausgaben)),
        ("afa_jahresbetrag", json!(afa)),
        ("einkuenfte_gewinn", json!(0)),
    ])
}

const FESTE_ZAHL: Ausgang = Ausgang::Ueberlauf("feste_zahl");

/// B4 (Treffer `feste_zahl::slot_fehler`: `SlotFehler::Ueberlauf` -> `Python { ValueError }`): zwei Summanden desselben
/// Slots, deren Summe ausserhalb `i64` liegt, melden `Ueberlauf("feste_zahl")` (`python_klasse() == None`), keinen
/// Python-Fehler. Python liefert fuer ALLE Faelle `grund = bestaetigt`, `zahl_cent = 0` (kein Einkommen, `ESt` 0, `orakel_bs5.py`);
/// die Faelle ausserhalb sind also "zwischen" (nur die Slot-Summe liegt ausserhalb `i64`), nie "klar". Die EUeR-Summanden
/// (`betriebsausgaben`) addiert der Guard schon vorab: dort meldet die Marke "Addition" (derselbe Fall, eine Stelle frueher);
/// `feste_zahl` erreichen darum die Vorsorge-Slots.
///
/// Die zwei EUeR-Faelle mit Summe `i64::MAX` (Guard laesst sie durch): der Gesamt-Scope gibt dort ein zvE ausserhalb `i64` aus
/// (Verlust `i64::MAX` ct plus Pauschalen), `gesamt_kette` liest es. Vor dem Shim-Guard (h8-shim-guard) kam der Wert mod 2^63 an und
/// die Rechnung lief durch (`Bestaetigt(0)`, zufaellig gleich Python, weil das Vorzeichen erhalten bleibt); jetzt meldet der Guard das
/// Feld `Einkommensteuertarif__zu_versteuerndes_einkommen`. Python: `grund = bestaetigt`, `zahl_cent = 0` (`orakel_bs5.py`, Anlage
/// `orakel_bs5_euer.out`, Fall `euer_max_0` und `euer_max_minus_1_plus_1`). Das ist die neue Abweichung 1g ("zwischen").
#[test]
fn slot_summe_ausserhalb_i64_meldet_feste_zahl_ueberlauf_und_knapp_rechnet() {
    let zve_ueberlauf = || {
        Ausgang::Anders(
            r#"EngineTeil2(Basis(Ueberlauf("Einkommensteuertarif__zu_versteuerndes_einkommen")))"#
                .to_owned(),
        )
    };
    let faelle: Vec<(&str, Fall, Ausgang)> = vec![
        (
            "knapp: basis_kv max + basis_pv 0",
            zwei("basis_kv", MAX, "basis_pv", 0),
            Ausgang::Bestaetigt(0),
        ),
        (
            "knapp: basis_kv max - 1 + basis_pv 1",
            zwei("basis_kv", MAX - 1, "basis_pv", 1),
            Ausgang::Bestaetigt(0),
        ),
        (
            "zwischen: basis_kv max + basis_pv 1",
            zwei("basis_kv", MAX, "basis_pv", 1),
            FESTE_ZAHL,
        ),
        (
            "zwischen: basis_kv max + basis_pv max",
            zwei("basis_kv", MAX, "basis_pv", MAX),
            FESTE_ZAHL,
        ),
        (
            "zwischen: vor_an_anteil_rv max + vor_ag_anteil_rv 1",
            zwei("vor_an_anteil_rv", MAX, "vor_ag_anteil_rv", 1),
            FESTE_ZAHL,
        ),
        (
            "zwischen: Arbeitslosenversicherung max + Unfall/Haftpflicht 1",
            zwei(
                "vorsorge_arbeitslosenversicherung",
                MAX,
                "vorsorge_unfall_haftpflicht",
                1,
            ),
            FESTE_ZAHL,
        ),
        (
            "zwischen: EUeR max + 0 (Guard im Shim: zvE; Python bestaetigt 0)",
            euer(MAX, 0),
            zve_ueberlauf(),
        ),
        (
            "zwischen: EUeR max - 1 + 1 (Guard im Shim: zvE; Python bestaetigt 0)",
            euer(MAX - 1, 1),
            zve_ueberlauf(),
        ),
        (
            "zwischen: EUeR max + 1 (Guard: Addition)",
            euer(MAX, 1),
            Ausgang::Ueberlauf("Addition"),
        ),
        (
            "zwischen: EUeR 2 x max (Guard: Addition)",
            euer(MAX, MAX),
            Ausgang::Ueberlauf("Addition"),
        ),
    ];
    let mut falsch = Vec::new();
    for (name, f, erw) in &faelle {
        let ist = ergebnis(f);
        if ist != *erw {
            falsch.push(format!("{name}: {ist:?}, erwartet {erw:?}"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// Der Fehler der Engine beim Umrechnen eines Euro-Betrags nach Cent (`Euro::to_cent`, Marke "Euro->Cent").
fn euro_nach_cent() -> Ausgang {
    Ausgang::Anders(r#"EngineTeil2(Basis(Ueberlauf("Euro->Cent")))"#.to_owned())
}

/// B1 (Treffer `rechnen::mal`: `checked_mul` -> `wrapping_mul`): `fam_anzahl_kinder` mal Kinderfreibetrag (4.800 EUR einzel
/// 2025) in `tarif::rahmen`. Die Bindung begrenzt das Feld auf 0..=20 (der Python-Store lehnt 21 mit 422 ab); die Faelle hier
/// stehen AUSSERHALB davon, wie in einer von Hand geschriebenen Akte (der Rust-`Store::aus_datei` prueft den Bereich nicht), und
/// das Orakel lief mit abgeschalteter Bereichspruefung (`OHNE_BEREICH=1`). Python liefert fuer ALLE Faelle `bestaetigt`,
/// `zahl_cent = 575700` (die Kinder aendern die Guenstigerpruefung nicht); Rust meldet ab `Freibetrag > i64::MAX` die Marke
/// "Multiplikation", also "zwischen". Bei `i64::MAX / 4800` Kindern passt das Produkt gerade noch (9.223.372.036.854.772.800), die
/// Engine scheitert dann eine Stelle spaeter bei Euro -> Cent: die Grenze der Stelle, ohne dass `mal` selbst anschlaegt.
#[test]
fn kinderfreibetrag_ausserhalb_i64_meldet_multiplikation_und_rechnet_knapp() {
    let kinder = |n: i64| {
        mit(&[
            ("einkuenfte_gewinn", json!(3_500_000)),
            ("fam_anzahl_kinder", json!(n)),
        ])
    };
    let faelle: Vec<(&str, i64, Ausgang)> = vec![
        (
            "knapp: 20 Kinder (Bindungsgrenze)",
            20,
            Ausgang::Bestaetigt(575_700),
        ),
        (
            "zwischen: i64::MAX / 4800 Kinder, Freibetrag passt, Euro -> Cent nicht",
            MAX / 4800,
            euro_nach_cent(),
        ),
        (
            "zwischen: i64::MAX / 4800 + 1 Kinder",
            MAX / 4800 + 1,
            Ausgang::Ueberlauf("Multiplikation"),
        ),
        (
            "zwischen: 10^16 Kinder",
            10_i64.pow(16),
            Ausgang::Ueberlauf("Multiplikation"),
        ),
    ];
    let mut falsch = Vec::new();
    for (name, n, erw) in &faelle {
        let ist = ergebnis(&kinder(*n));
        if ist != *erw {
            falsch.push(format!("{name}: {ist:?}, erwartet {erw:?}"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// B2 (Treffer `rechnen::mal_div`: `try_from(..)` -> `as i64`): die Kapital-KiSt-Basis `(Kapital - 4 * q) * 10000 / (400 + k)` in
/// `tarif::kapital`. `q` (auslaendische Steuer, Cent) ist negativ in der Eingabe moeglich (Bindung ohne Untergrenze), dann waechst
/// die Basis. Kirchensteuerpflichtig (roem.-kath., NRW, k = 9), Lohn 24.000 EUR, Kapital 9 * 10^18 Cent ("gross") oder 5.000 EUR
/// ("klein"). Python (Orakel `orakel_bs5.py`, `api.ergebnis`): "klein, q = min" und "gross, q = 0" liegen mit `zahl_cent` in
/// `i64` ("knapp": Rust MUSS den Wert liefern), "gross, q = min" liegt mit `zahl_cent` = 11.220.901.747.535.441.900 AUSSERHALB
/// (Rust MUSS `Ueberlauf("a*b//c")` melden, nie einen abgeschnittenen Wert).
///
/// B3 (`4 * q_eur`, `checked_mul`): KEIN Fall erreicht es. `q_eur = min(q_cent // 100, kap_st)` und `q_cent` ist `i64`, also
/// `q_eur <= 92.233.720.368.547.758` und `4 * q_eur <= 368.934.881.474.191.032 < i64::MAX`; die Faelle "q = max" zeigen die obere
/// Kante (`q_eur` = `kap_st`, kein Fehler, Python-Wert).
#[test]
fn kapital_kist_basis_meldet_ueberlauf_und_rechnet_knapp() {
    let kap = |k: i64, q: i64| {
        mit(&[
            ("kist_konfession", json!("roemisch-katholisch")),
            ("kist_gezahlt", json!(0)),
            ("kist_erstattet", json!(0)),
            ("kist_bundesland", json!("nordrhein_westfalen")),
            ("kein_kap", json!(false)),
            ("kein_gewinn", json!(true)),
            ("bruttoarbeitslohn", json!(2_400_000)),
            ("kap_kapitalertraege", json!(k)),
            ("kap_q_auslaendische_steuer", json!(q)),
        ])
    };
    let gross = 9_000_000_000_000_000_000_i64;
    let faelle: Vec<(&str, i64, i64, Ausgang)> = vec![
        (
            "knapp: 5.000 EUR, q 100 EUR (test_p51a_kist_bemessung)",
            500_000,
            10_000,
            Ausgang::Bestaetigt(321_200),
        ),
        (
            "knapp: gross, q 0",
            gross,
            0,
            Ausgang::Bestaetigt(2_200_488_997_555_220_900),
        ),
        (
            "knapp: klein, q = min",
            500_000,
            MIN,
            Ausgang::Bestaetigt(9_020_412_749_980_551_900),
        ),
        (
            "klar: gross, q = min",
            gross,
            MIN,
            Ausgang::Ueberlauf("a*b//c"),
        ),
        (
            "knapp: gross, q = max (q_eur = kap_st)",
            gross,
            MAX,
            Ausgang::Bestaetigt(233_200),
        ),
        (
            "knapp: klein, q = max (q_eur = kap_st)",
            500_000,
            MAX,
            Ausgang::Bestaetigt(233_200),
        ),
    ];
    let mut falsch = Vec::new();
    for (name, k, q, erw) in &faelle {
        let ist = ergebnis(&kap(*k, *q));
        if ist != *erw {
            falsch.push(format!("{name}: {ist:?}, erwartet {erw:?}"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}
