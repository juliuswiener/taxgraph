//! Der Versorgungsbezug steckt im Bruttoarbeitslohn (Abweichung Nr. 50): der Bescheid zaehlt ihn nicht doppelt. Im
//! Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Wer Versorgungsbezuege hat, bekommt eine Lohnsteuerbescheinigung. Nr. 3 ist der Bruttoarbeitslohn und
//! ENTHAELT den Bezug; Nr. 8 nennt den Bezug noch einmal einzeln ("in 3. enthaltene Versorgungsbezuege", Vordruck Anlage N
//! Zeile 11: "im Bruttoarbeitslohn laut Zeile 5 enthalten"). TaxGraph fragt beide Betraege. Der Bescheid addierte sie bisher:
//! der Bezug zahlte doppelt Steuer.
//!
//! **Warum es zaehlt.** Wer Nr. 3 und Nr. 8 abschreibt, bekam einen Gesamtbetrag der Einkuenfte, der um den Bezug (abzueglich
//! Pauschbetraege) zu hoch lag. Die Abgabe war gesperrt (Abweichung Nr. 42), es ging keine falsche Erklaerung hinaus.
//!
//! **Wo es sitzt.** `zweige/gesamt.rs::einkuenfte_ns_aus_lohn` (Lohn minus Bezug als Arbeitslohn, der Bezug als Versorgungs-
//! einkunft) und `deklaration/sperre/gesamt.rs::versorgung_ueber_lohn` (Bezug groesser als Lohn: Sperre mit lesbarem Text).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus dem Gesetzestext und `params/`, nie aus dem Rust-Code gelesen.
//! - Arbeitslohn: Einkuenfte = Lohn − 1.230 Euro, nie unter 0 (`estg_p9a_2026-07-09.txt`, Satz 1 Nr. 1 Buchstabe a).
//! - Versorgungsbezug: Einkuenfte = Bezug − (min(13,2 % der Bemessungsgrundlage, 990) + 297) − 102 Euro
//!   (`estg_p19_2026-07-17.txt` Tabelle: 2025 → 13,2 / 990 / 297; `estg_p9a_2026-07-09.txt` Nr. 1 Buchstabe b: 102 Euro).
//!   Bei 30.000 Euro (Bemessungsgrundlage gleich Bezug, 13,2 % = 3.960 greifen den Hoechstbetrag 990):
//!   30.000 − 1.287 − 102 = 28.611.
//! - Der Lohn enthaelt den Bezug: Arbeitslohn = Lohn − Bezug.
//!   * Lohn 50.000, Bezug 30.000: Arbeitslohn 20.000 → 18.770; Versorgung 28.611; zusammen 47.381.
//!     (Additiv, wie bisher, waeren es 48.770 + 28.611 = 77.381.)
//!   * Lohn gleich Bezug 30.000 (reiner Pensionaer): Arbeitslohn 0 → 0; Versorgung 28.611.
//! - Alters-Gate (§ 19 Abs. 2 Satz 2 Nr. 2): vor dem 63. Lebensjahr (60. bei Grad der Behinderung ab 50) ist der Bezug
//!   Arbeitslohn ohne Versorgungsfreibetrag. Er steckt in Nr. 3 und zaehlt EINMAL: Lohn 50.000 → 50.000 − 1.230 = 48.770.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types,
    clippy::too_many_lines
)]

use std::collections::HashMap;

use bescheid::deklaration::{an_gesamt_sperrgrund, feste_zahl, Cfg};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::ausgaben::Kette;
use bescheid::zweige::Umgebung;
use bescheid::{Felder, Instanzquelle};
use bindung::Bindung;
use domain::{Feldtyp, Scheibe, Vz};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const VZ: Vz = Vz::Vz2025;

/// Der Grund (`Sperrgrund::als_str`), wenn der Bezug groesser ist als der Bruttoarbeitslohn.
const UEBER_LOHN: &str = "versorgung_ueber_lohn";

type Paare = Vec<(&'static str, Value)>;

/// Euro in Cent.
const fn cent(euro: i64) -> i64 {
    euro * 100
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
        _ => panic!("KONTROLLE: Kegel-Feld {} hat keinen Abwesenheitswert", b.feld_id),
    }
}

/// `tests/_kegel.py::kegel_fuer`: der volle Pflicht-Kegel; was der Test setzt, gewinnt, der Rest folgt bestaetigt.
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(&'static str, Value)]) -> Paare {
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Paare = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .rev()
                .find(|(g, _)| g == f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            (*f, w)
        })
        .collect();
    raus.extend(gesetzt.iter().filter(|(f, _)| !kegel.contains(f)).cloned());
    raus
}

/// Ein Fall der Rentner-Scheibe, mit der Scheiben-Bindung (`api._scheibe_bindung`) als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

fn fall(paare: &[(&'static str, Value)]) -> Fall {
    let cfg = Cfg::fuer(Scheibe::RentnerGesamt);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe liest Felder aus {d}"))
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

#[derive(Debug)]
enum Ausgang {
    Gesperrt(String),
    Zahl(Box<Option<Kette>>),
    Anders(String),
}

/// Der Ausgang von `_ergebnis_roh` ohne HTTP: erst der K2-Guard, dann `feste_zahl` (nur bestaetigte Werte).
fn ergebnis(f: &Fall) -> Ausgang {
    let q = Instanzquelle {
        store: Some(&f.store),
        bindung: Some(&f.index),
        nur_bestaetigt: false,
    };
    match an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(VZ), &q) {
        Ok(Some(g)) => return Ausgang::Gesperrt(g.als_str().to_owned()),
        Ok(None) => {}
        Err(e) => return Ausgang::Anders(format!("Guard: {e:?}")),
    }
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, VZ, &kegel, &umg, Some(&f.store), None) {
        Ok(Ok(z)) => Ausgang::Zahl(Box::new(z.extras.kette)),
        Ok(Err(k)) => Ausgang::Anders(format!("ohne Zahl: {:?}", k.grund)),
        Err(e) => Ausgang::Anders(format!("{e:?}")),
    }
}

/// Gesamtbetrag der Einkuenfte in Euro; jeder andere Ausgang ist ein Fehlschlag des Tests.
fn gdb(paare: &[(&'static str, Value)]) -> i64 {
    match ergebnis(&fall(paare)) {
        Ausgang::Zahl(k) => (*k).expect("KONTROLLE: Kette fehlt").gesamtbetrag_der_einkuenfte.get(),
        Ausgang::Anders(was) => panic!("KONTROLLE: erwartet eine Zahl, bekommen {was}"),
        Ausgang::Gesperrt(g) => panic!("KONTROLLE: erwartet eine Zahl, bekommen die Sperre {g}"),
    }
}

/// Der Sperrgrund eines Falls; eine Zahl oder ein anderer Ausgang ist ein Fehlschlag des Tests.
fn gesperrt_mit(paare: &[(&'static str, Value)]) -> String {
    match ergebnis(&fall(paare)) {
        Ausgang::Gesperrt(g) => g,
        Ausgang::Anders(was) => panic!("KONTROLLE: erwartet eine Sperre, bekommen {was}"),
        Ausgang::Zahl(_) => panic!("KONTROLLE: erwartet eine Sperre, bekommen eine Zahl"),
    }
}

/// Rentnerin im Beginnjahr 2025 mit 20.000 Euro gesetzlicher Rente, alle Kreuze "nein", plus `mehr`. Mit `zusammen` zusammen
/// veranlagt (mit den fuenf Kapital-Feldern des Partner-Kegels auf null).
fn rentner(zusammen: bool, mehr: Paare) -> Paare {
    let mut p: Paare = vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(cent(20_000))),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_rentenfreibetrag", json!(0)),
        ("veranlagung", json!(if zusammen { "zusammen" } else { "einzel" })),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ];
    if zusammen {
        p.extend([
            ("kap_kapitalertraege_partner", json!(0)),
            ("kap_gewinn_aktien_partner", json!(0)),
            ("kap_gewinn_sonstige_partner", json!(0)),
            ("kap_verlust_aktien_partner", json!(0)),
            ("kap_verlust_sonstige_partner", json!(0)),
        ]);
    }
    for (f, w) in mehr {
        match p.iter_mut().find(|(g, _)| *g == f) {
            Some(e) => e.1 = w,
            None => p.push((f, w)),
        }
    }
    p
}

fn lohn(euro: i64) -> Paare {
    vec![("bruttoarbeitslohn", json!(cent(euro)))]
}

fn lohn_partner(euro: i64) -> Paare {
    vec![("bruttoarbeitslohn_partner", json!(cent(euro)))]
}

/// Ein Versorgungsbezug von `euro` mit der Bemessungsgrundlage gleich dem Bezug, Beginn 2025, beamtenrechtlich (kein
/// Alters-Gate).
fn versorgung(euro: i64) -> Paare {
    vec![
        ("versorgung_jahresrente", json!(cent(euro))),
        ("versorgung_bemessungsgrundlage", json!(cent(euro))),
        ("versorgung_beginn_jahr", json!(2025)),
        ("versorgung_art", json!("beamtenrechtlich")),
    ]
}

fn versorgung_partner(euro: i64) -> Paare {
    vec![
        ("versorgung_jahresrente_partner", json!(cent(euro))),
        ("versorgung_bemessungsgrundlage_partner", json!(cent(euro))),
        ("versorgung_beginn_jahr_partner", json!(2025)),
        ("versorgung_art_partner", json!("beamtenrechtlich")),
    ]
}

// ---------------------------------------------------------------- AK1: der Bezug zaehlt einmal

/// Person A: Lohn 50.000 enthaelt den Bezug 30.000. Arbeitslohn 20.000 (18.770) plus Versorgung (28.611) = 47.381.
/// Additiv (der Stand vor Nr. 50) waeren es 77.381.
#[test]
fn der_lohn_enthaelt_den_bezug_person_a() {
    let ohne = gdb(&rentner(false, vec![]));
    let mit = gdb(&rentner(false, [lohn(50_000), versorgung(30_000)].concat()));
    assert_eq!(mit - ohne, 18_770 + 28_611);
}

/// Reiner Pensionaer: Nr. 3 und Nr. 8 sind gleich (30.000). Der Arbeitslohn ist 0, es bleibt die Versorgung (28.611). Der
/// Arbeitnehmer-Pauschbetrag macht aus 0 keine negativen Einkuenfte.
#[test]
fn ein_reiner_pensionaer_zahlt_nur_auf_die_versorgung() {
    let ohne = gdb(&rentner(false, vec![]));
    let mit = gdb(&rentner(false, [lohn(30_000), versorgung(30_000)].concat()));
    assert_eq!(mit - ohne, 28_611);
}

/// Person B (Zusammenveranlagung): dieselbe Rechnung in der zweiten Anlage N, Pauschbetraege je Person.
#[test]
fn der_lohn_enthaelt_den_bezug_person_b() {
    let ohne = gdb(&rentner(true, vec![]));
    let mit = gdb(&rentner(true, [lohn_partner(50_000), versorgung_partner(30_000)].concat()));
    assert_eq!(mit - ohne, 18_770 + 28_611);
    let rein = gdb(&rentner(true, [lohn_partner(30_000), versorgung_partner(30_000)].concat()));
    assert_eq!(rein - ohne, 28_611, "reiner Pensionaer als Ehegatte");
}

/// Beide Personen mit eigenem Lohn und Bezug: jede Person rechnet fuer sich.
#[test]
fn beide_ehegatten_rechnen_je_fuer_sich() {
    let ohne = gdb(&rentner(true, vec![]));
    let p = [
        lohn(50_000),
        versorgung(30_000),
        lohn_partner(30_000),
        versorgung_partner(30_000),
    ]
    .concat();
    assert_eq!(gdb(&rentner(true, p)) - ohne, (18_770 + 28_611) + 28_611);
}

/// Alters-Gate: vor dem 63. Lebensjahr (60. bei Grad 50) ist der Bezug Arbeitslohn ohne Freibetrag und steckt in Nr. 3. Er
/// zaehlt einmal (48.770), ab dem Gate als Versorgung (47.381).
#[test]
fn das_alters_gate_haelt_den_bezug_im_lohn() {
    let ohne = gdb(&rentner(false, vec![]));
    let altersgrenze = |alter: i64, grad: i64| {
        let mut p = [lohn(50_000), versorgung(30_000)].concat();
        p.push(("versorgung_art", json!("altersgrenze_sonstige")));
        p.push(("versorgung_alter_bei_beginn", json!(alter)));
        p.push(("rentner_grad_der_behinderung", json!(grad)));
        gdb(&rentner(false, p)) - ohne
    };
    assert_eq!(altersgrenze(62, 0), 48_770, "62 Jahre: Arbeitslohn, einmal gezaehlt");
    assert_eq!(altersgrenze(63, 0), 47_381, "63 Jahre: Versorgungsbezug");
    assert_eq!(altersgrenze(59, 50), 48_770, "59 Jahre, Grad 50: Arbeitslohn");
    assert_eq!(altersgrenze(60, 50), 47_381, "60 Jahre, Grad 50: Versorgungsbezug");
}

/// KONTROLLE: ohne Bezug aendert sich nichts. Der Lohn allein ist Lohn minus Pauschbetrag (20.000 − 1.230 = 18.770).
#[test]
fn kontrolle_ohne_bezug_rechnet_der_lohn_wie_vorher() {
    let ohne = gdb(&rentner(false, vec![]));
    assert_eq!(gdb(&rentner(false, lohn(20_000))) - ohne, 18_770);
    assert_eq!(gdb(&rentner(true, [lohn_partner(20_000)].concat())) - gdb(&rentner(true, vec![])), 18_770);
}

// ---------------------------------------------------------------- AK2: Bezug groesser als Lohn

/// Person A: ein Bezug ueber dem Bruttoarbeitslohn kann nicht in Nr. 3 stecken. Der Bescheid sperrt mit eigenem Grund. Rand:
/// ein Euro weniger Lohn als Bezug sperrt, gleich viel nicht.
#[test]
fn ein_bezug_ueber_dem_lohn_sperrt_person_a() {
    assert_eq!(gesperrt_mit(&rentner(false, [lohn(20_000), versorgung(30_000)].concat())), UEBER_LOHN);
    assert_eq!(gesperrt_mit(&rentner(false, [lohn(29_999), versorgung(30_000)].concat())), UEBER_LOHN);
    assert_eq!(
        gesperrt_mit(&rentner(false, versorgung(30_000))),
        UEBER_LOHN,
        "ohne Lohn: der Nutzer hat Nr. 3 nicht eingetragen"
    );
    assert!(gdb(&rentner(false, [lohn(30_000), versorgung(30_000)].concat())) > 0, "gleich viel: keine Sperre");
}

/// Person B bei Zusammenveranlagung. KONTROLLE: bei Einzelveranlagung zaehlt der Ehegatte nicht, also sperrt er nicht.
#[test]
fn ein_bezug_ueber_dem_lohn_sperrt_person_b_nur_bei_zusammenveranlagung() {
    let p = || [lohn_partner(20_000), versorgung_partner(30_000)].concat();
    assert_eq!(gesperrt_mit(&rentner(true, p())), UEBER_LOHN);
    assert_eq!(gesperrt_mit(&rentner(true, versorgung_partner(30_000))), UEBER_LOHN, "ohne Lohn des Ehegatten");
    assert!(gdb(&rentner(false, p())) > 0, "einzel: der Bezug des Ehegatten zaehlt nicht, der Fall rechnet");
}

/// Der Grund ist lesbar: er nennt Nr. 3 und Nr. 8 der Lohnsteuerbescheinigung und sagt, was der Nutzer tut.
#[test]
fn der_grund_nennt_die_beiden_nummern_und_was_zu_tun_ist() {
    let grund: domain::Sperrgrund = UEBER_LOHN.parse().expect("der Grund ist unbekannt");
    let text = grund.klartext().expect("Klartext fehlt");
    assert!(
        text.contains("Nummer 3") && text.contains("Nummer 8") && text.contains("Bruttoarbeitslohn"),
        "{text}"
    );
}
