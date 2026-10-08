//! DBA-Freistellung (Abweichung Nr. 40, § 32b Abs. 1 S. 1 Nr. 3 `EStG`): die Sperre `dba_freistellung_offen` des
//! Gesamt-Guards `an_gesamt_sperrgrund`, Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Einkuenfte aus einem Staat, dessen Abkommen sie in Deutschland freistellt, sind hier steuerfrei,
//! erhoehen aber den Steuersatz auf das uebrige Einkommen (Progressionsvorbehalt). Der Guard sperrt jeden Fall, der solche
//! Einkuenfte ueber 0 Euro traegt, solange die Rechnung den Vorbehalt nicht anwendet.
//!
//! **Warum es zaehlt.** Ohne Sperre liefe der Fall durch: der Bescheid zeigte den Satz ohne die Auslandseinkuenfte, bei
//! 50.000 Euro Lohn und 20.000 Euro freigestellten Einkuenften 2.514 Euro zu wenig (10.691 statt 13.205 Euro, VZ 2025,
//! Vault `dba-freistellung-progressionsvorbehalt-wird-berechnet-aber-nie-angewendet`).
//!
//! Die Freistellung folgt aus `dba_methode` ODER aus Staat und Einkunftsart (`einkuenfte::dba_methode_der_akte`); beide
//! Wege sperren. Die Reihenfolge im Guard ist festgehalten: mehrere Staaten, Kapital und die Kombination mit Lohnersatz
//! behalten ihren eigenen Grund.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: der Grund `dba_freistellung_offen` ist Rust-eigen (Python kennt ihn nicht, Julius
//! 2026-10-08); die Methode je Staat und Einkunftsart steht in `sources/dba/` und ist in
//! `p34c_abzug_rechnung.rs::bei_freistellung_ueber_staat_und_einkunftsart_gibt_es_keinen_abzug` schon gepinnt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::sync::OnceLock;

use bescheid::deklaration::{an_gesamt_sperrgrund, Cfg};
use bescheid::testhilfe::{felder, index, store};
use bescheid::{BindungIndex, Instanzquelle};
use domain::{Scheibe, Sperrgrund, Vz};
use serde_json::{json, Value};

const SCHEIBEN: [Scheibe; 2] = [Scheibe::Gesamt, Scheibe::RentnerGesamt];
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const METHODE: &str = "dba_methode";

/// Cent aus Euro.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

/// Die Bindung der Scheibe: nur ihre Feld-Ids, wie `api._scheibe_bindung`.
fn scheiben_index(scheibe: Scheibe) -> &'static BindungIndex<'static> {
    static GESAMT: OnceLock<BindungIndex<'static>> = OnceLock::new();
    static RENTNER: OnceLock<BindungIndex<'static>> = OnceLock::new();
    let zelle = if scheibe == Scheibe::RentnerGesamt {
        &RENTNER
    } else {
        &GESAMT
    };
    zelle.get_or_init(|| {
        let ids = Cfg::fuer(scheibe).felder(|_| Vec::new()).unwrap();
        index()
            .iter()
            .filter(|(k, _)| ids.contains(k))
            .map(|(k, b)| (k.clone(), *b))
            .collect()
    })
}

/// Der Sperrgrund der Scheibe fuer VZ 2025 auf einem Store aus den bestaetigten Paaren.
fn grund(scheibe: Scheibe, paare: &[(&str, Value)]) -> Option<&'static str> {
    let events: Vec<(&str, Value, bool)> = paare.iter().map(|(f, w)| (*f, w.clone(), true)).collect();
    let st = store(&events);
    let f = felder(&st);
    let q = Instanzquelle {
        store: Some(&st),
        bindung: Some(scheiben_index(scheibe)),
        nur_bestaetigt: false,
    };
    let cfg = Cfg::fuer(scheibe);
    an_gesamt_sperrgrund(&f, Some(&cfg), Some(Vz::Vz2025), &q)
        .unwrap()
        .map(Sperrgrund::als_str)
}

/// Der Grund auf beiden Scheiben; weichen sie ab, ist der Fall rot.
fn grund_beide(paare: &[(&str, Value)]) -> Option<&'static str> {
    let g: Vec<Option<&str>> = SCHEIBEN.iter().map(|s| grund(*s, paare)).collect();
    assert_eq!(g[0], g[1], "Gesamt und RentnerGesamt weichen ab: {g:?}");
    g[0]
}

/// KONTROLLE zuerst: die Anrechnung (kein Abkommen oder Anrechnungsabkommen) sperrt NICHT, auch mit Auslandseinkuenften
/// und gezahlter Steuer. Sonst belegte die Sperre unten nichts: ein Guard, der bei jedem Auslandsfall sperrt, liesse jeden
/// Test gruen.
#[test]
fn kontrolle_die_anrechnung_sperrt_nicht() {
    let faelle: [(&str, Vec<(&str, Value)>); 5] = [
        ("ohne Auslandsangaben", vec![]),
        (
            "ohne Abkommen",
            vec![(STEUER, json!(cent(700))), (EINKUENFTE, json!(cent(5_000)))],
        ),
        (
            "Methode Anrechnung",
            vec![
                (STEUER, json!(cent(700))),
                (EINKUENFTE, json!(cent(5_000))),
                (METHODE, json!("dba_anrechnung")),
            ],
        ),
        (
            "Niederlande pauschal",
            vec![
                (STEUER, json!(cent(700))),
                (EINKUENFTE, json!(cent(5_000))),
                ("dba_staat", json!("nl")),
            ],
        ),
        (
            "Polen, Dividenden",
            vec![
                (STEUER, json!(cent(700))),
                (EINKUENFTE, json!(cent(5_000))),
                ("dba_staat", json!("pl")),
                ("dba_einkunftsart", json!("dividenden")),
            ],
        ),
    ];
    for (name, paare) in faelle {
        assert_eq!(grund_beide(&paare), None, "{name}");
    }
}

/// Die Freistellung ueber das Feld `dba_methode` sperrt, auf beiden Scheiben; Beispiel aus dem Ticket: 20.000 Euro
/// freigestellt.
#[test]
fn freistellung_ueber_die_methode_sperrt() {
    let paare = [
        (EINKUENFTE, json!(cent(20_000))),
        (METHODE, json!("dba_freistellung")),
    ];
    assert_eq!(grund_beide(&paare), Some("dba_freistellung_offen"));
}

/// Die Freistellung ueber Staat und Einkunftsart sperrt ebenso, ohne Antwort auf `dba_methode`: USA und Oesterreich
/// pauschal, Polen nur fuer Ruhegehaelter.
#[test]
fn freistellung_ueber_staat_und_einkunftsart_sperrt() {
    let faelle: [(&str, Option<&str>); 3] = [("us", None), ("at", None), ("pl", Some("ruhegehaelter"))];
    for (staat, art) in faelle {
        let mut paare = vec![(EINKUENFTE, json!(cent(20_000))), ("dba_staat", json!(staat))];
        if let Some(a) = art {
            paare.push(("dba_einkunftsart", json!(a)));
        }
        assert_eq!(
            grund_beide(&paare),
            Some("dba_freistellung_offen"),
            "{staat} {art:?}"
        );
    }
}

/// Ohne Auslandseinkuenfte ueber 0 gibt es nichts, was den Satz anhebt: weder ohne das Feld noch mit 0 noch mit nur
/// gezahlter Steuer sperrt der Guard (die Erklaerung sperrt dann an ihrer eigenen Stelle, nicht der Bescheid).
#[test]
fn ohne_freigestellte_einkuenfte_sperrt_der_bescheid_nicht() {
    let faelle: [(&str, Vec<(&str, Value)>); 3] = [
        ("nur die Methode", vec![(METHODE, json!("dba_freistellung"))]),
        (
            "Einkuenfte 0",
            vec![(EINKUENFTE, json!(0)), (METHODE, json!("dba_freistellung"))],
        ),
        (
            "nur gezahlte Steuer",
            vec![(STEUER, json!(cent(700))), (METHODE, json!("dba_freistellung"))],
        ),
    ];
    for (name, paare) in faelle {
        assert_eq!(grund_beide(&paare), None, "{name}");
    }
}

/// Die Reihenfolge im Guard: mehrere Staaten, Kapital zusammen mit Auslandseinkuenften und Lohnersatz zusammen mit
/// Auslandseinkuenften behalten ihren eigenen Grund, auch bei Freistellung.
#[test]
fn die_anderen_dba_gruende_gehen_vor() {
    let frei = [
        (EINKUENFTE, json!(cent(20_000))),
        (METHODE, json!("dba_freistellung")),
    ];
    let mit = |extra: (&'static str, Value)| {
        let mut p = frei.to_vec();
        p.push(extra);
        p
    };
    assert_eq!(
        grund_beide(&mit(("dba_mehrere_staaten", json!(true)))),
        Some("dba_multi_country_offen")
    );
    assert_eq!(
        grund_beide(&mit(("kap_kapitalertraege", json!(cent(1_000))))),
        Some("dba_kapital_offen")
    );
    assert_eq!(
        grund_beide(&mit(("p32b_progressionseinkuenfte", json!(cent(5_000))))),
        Some("p32b_kombi_offen")
    );
}
