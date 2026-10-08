//! § 34c Abs. 2 `EStG`, Abzug statt Anrechnung (Abweichung Nr. 41): die Sperre `dba_abzug_offen` des Gesamt-Guards
//! `an_gesamt_sperrgrund`. Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Der Bescheid rechnet den Abzug nur in einem Fall: Arbeitslohn einer einzeln veranlagten Person ohne
//! Rente-Scheibe und ohne fiktive Steuer (`p34c_abzug_einkuenfte.rs`). In jedem anderen Fall weiss die Rechnung nicht, bei
//! welcher Einkunftsart sie die Steuer abziehen soll, oder ob der Betrag zu kuerzen ist. Dort sperrt der Guard, statt den
//! Abzug still beim Einkommen abzuziehen (die alte Rechnung) oder gar nicht.
//!
//! **Warum es zaehlt.** Ohne die Sperre bekaeme jemand mit Abzug und Zinsen, Mieteinnahmen, Rente oder Zusammenveranlagung
//! eine Zahl, die die Einkuenfte nicht kuerzt (zu hohe Steuer) oder die fiktive Steuer mit abzieht (§ 34c Abs. 2 gewaehrt den
//! Abzug nur fuer die gezahlte Steuer; Anleitung Anlage AUS 2025, Zeile 13).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: der Grund `dba_abzug_offen` ist Rust-eigen (Julius 2026-10-08, Punkt 2, Option 1: eine Wahl
//! fuer den ganzen Fall plus Sperre); Python rechnet den Abzug beim Einkommen und kennt keinen Grund.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::sync::OnceLock;

use bescheid::deklaration::{an_gesamt_sperrgrund, Cfg};
use bescheid::testhilfe::{felder, index, store};
use bescheid::{BindungIndex, Instanzquelle};
use domain::{Scheibe, Sperrgrund, Vz};
use serde_json::{json, Value};

const WAHL: &str = "dba_abzug_statt_anrechnung";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const ART: &str = "dba_einkunftsart";
const FIKTIV: &str = "dba_fiktive_steuer_vorhanden";
const OFFEN: &str = "dba_abzug_offen";

type Paare = Vec<(&'static str, Value)>;

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
    let events: Vec<(&str, Value, bool)> =
        paare.iter().map(|(f, w)| (*f, w.clone(), true)).collect();
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

/// Der Fall, den der Bau rechnet: Einzelveranlagung, ein Staat ohne Abkommen, Arbeitslohn, 2.000 Euro Steuer auf 10.000 Euro
/// Auslandseinkuenfte, Abzug gewaehlt.
fn abzug() -> Paare {
    vec![
        ("veranlagung", json!("einzel")),
        ("dba_staat", json!("sonstiger_staat")),
        ("dba_methode", json!("kein_dba")),
        (ART, json!("unselbstaendige_arbeit")),
        (STEUER, json!(cent(2_000))),
        (EINKUENFTE, json!(cent(10_000))),
        (WAHL, json!(true)),
    ]
}

/// `abzug()` mit ersetzten oder ergaenzten Feldern.
fn abzug_mit(mehr: &[(&'static str, Value)]) -> Paare {
    let mut p = abzug();
    for (feld, wert) in mehr {
        p.retain(|(g, _)| g != feld);
        p.push((feld, wert.clone()));
    }
    p
}

/// `abzug()` ohne das genannte Feld.
fn abzug_ohne(feld: &str) -> Paare {
    abzug().into_iter().filter(|(g, _)| *g != feld).collect()
}

/// KONTROLLE zuerst: der Fall, den der Bau rechnet, sperrt in der Gesamt-Scheibe NICHT. Sonst belegten alle Sperren unten
/// nichts: jeder Fall sperrte aus demselben Grund, den die Kontrolle schon zeigt.
#[test]
fn kontrolle_der_gerechnete_fall_sperrt_nicht() {
    assert_eq!(grund(Scheibe::Gesamt, &abzug()), None);
}

/// AK2: jeder Fall, dessen Quell-Anlage der Bau nicht sicher kennt, sperrt mit `dba_abzug_offen`.
#[test]
fn faelle_ohne_sichere_quell_anlage_sperren() {
    let faelle: Vec<(&str, Scheibe, Paare)> = vec![
        ("Einkunftsart fehlt", Scheibe::Gesamt, abzug_ohne(ART)),
        (
            "Zinsen",
            Scheibe::Gesamt,
            abzug_mit(&[(ART, json!("zinsen"))]),
        ),
        (
            "Unternehmensgewinne",
            Scheibe::Gesamt,
            abzug_mit(&[(ART, json!("unternehmensgewinne"))]),
        ),
        (
            "unbewegliches Vermoegen",
            Scheibe::Gesamt,
            abzug_mit(&[(ART, json!("unbewegliches_vermoegen"))]),
        ),
        (
            "Zusammenveranlagung",
            Scheibe::Gesamt,
            abzug_mit(&[("veranlagung", json!("zusammen"))]),
        ),
        ("Rentner-Scheibe", Scheibe::RentnerGesamt, abzug()),
    ];
    for (name, scheibe, paare) in faelle {
        assert_eq!(grund(scheibe, &paare), Some(OFFEN), "{name}");
    }
}

/// AK3: die fiktive Steuer sperrt (§ 34c Abs. 2 gewaehrt den Abzug nur fuer die gezahlte Steuer). Nur ein "ja" sperrt: "nein"
/// und die fehlende Antwort rechnen, wie bei `dba_mehrere_staaten`.
#[test]
fn eine_fiktive_steuer_sperrt() {
    assert_eq!(
        grund(Scheibe::Gesamt, &abzug_mit(&[(FIKTIV, json!(true))])),
        Some(OFFEN)
    );
    assert_eq!(
        grund(Scheibe::Gesamt, &abzug_mit(&[(FIKTIV, json!(false))])),
        None,
        "ein Nein auf die Frage sperrt nicht"
    );
}

/// Die Sperre gilt nur beim gewaehlten Abzug. Ohne Wahl, mit Wahl `false`, ohne Steuer oder ohne Auslandseinkuenfte bleiben
/// dieselben Faelle frei (Anrechnung oder nichts zu tun): sonst sperrte die Sperre jeden Fall mit Zinsen und Auslandssteuer.
#[test]
fn ohne_gewaehlten_abzug_sperrt_nichts() {
    let stoerer = [
        (ART, json!("zinsen")),
        ("veranlagung", json!("zusammen")),
        (FIKTIV, json!(true)),
    ];
    let ohne_abzug = [
        ("ohne Wahl", abzug_ohne(WAHL)),
        ("Wahl nein", abzug_mit(&[(WAHL, json!(false))])),
        ("ohne Steuer", abzug_mit(&[(STEUER, json!(0))])),
        (
            "ohne Auslandseinkuenfte",
            abzug_mit(&[(EINKUENFTE, json!(0))]),
        ),
    ];
    for (name, basis) in ohne_abzug {
        for (feld, wert) in &stoerer {
            let mut p = basis.clone();
            p.retain(|(g, _)| g != feld);
            p.push((feld, wert.clone()));
            assert_ne!(
                grund(Scheibe::Gesamt, &p),
                Some(OFFEN),
                "{name} mit {feld}={wert}"
            );
        }
    }
    // Die Rentner-Scheibe sperrt ohne gewaehlten Abzug ebenfalls nicht mit diesem Grund.
    assert_ne!(
        grund(Scheibe::RentnerGesamt, &abzug_ohne(WAHL)),
        Some(OFFEN)
    );
}

/// Bei Freistellung gibt es keinen Abzug (Abweichung Nr. 35): die Wahl loest `dba_abzug_offen` nicht aus, auch nicht mit
/// den Stoerern. Die Freistellung hat ihren eigenen Grund (`dba_freistellung_offen`) und rechnet allein.
#[test]
fn bei_freistellung_sperrt_der_abzug_nicht() {
    let p = abzug_mit(&[
        ("dba_methode", json!("dba_freistellung")),
        (ART, json!("zinsen")),
    ]);
    assert_ne!(grund(Scheibe::Gesamt, &p), Some(OFFEN));
}

/// Die anderen DBA-Gruende gehen vor: mehrere Staaten behalten ihren Grund, auch bei gewaehltem Abzug und Zinsen.
#[test]
fn mehrere_staaten_gehen_vor() {
    let p = abzug_mit(&[("dba_mehrere_staaten", json!(true)), (ART, json!("zinsen"))]);
    assert_eq!(grund(Scheibe::Gesamt, &p), Some("dba_multi_country_offen"));
}
