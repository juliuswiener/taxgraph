//! Der Werbungskosten-Pauschbetrag von 102 Euro gilt EINMAL fuer alle Renten einer Person (Abweichung Nr. 55). Im Standardlauf, ohne
//! `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** § 9a Satz 1 Nr. 3 `EStG` gibt 102 Euro "insgesamt" fuer die Einnahmen aus § 22 Nr. 1, 1a und 5: die Renten der
//! Anlage R und die Leistungen der Anlage R-AV/bAV zusammen. Der Ring zog ihn je Rente der Anlage R ab (Python `_rente_instanz`) und
//! zog ihn fuer die Mitteilungen ein zweites Mal ab.
//!
//! **Warum es zaehlt.** Wer eine gesetzliche Rente und eine Betriebsrente bezieht, bekam 204 statt 102 Euro Abzug: der Gesamtbetrag der
//! Einkuenfte lag um 102 Euro zu niedrig, die Steuer still zu niedrig. Bei drei Renten der Anlage R waren es 306 Euro.
//!
//! **Wo es sitzt.** `zweige/rentner.rs`: `rente_instanz` liefert den Anteil VOR dem Pauschbetrag (Boden 0 je Rente), `renten_summe`
//! summiert, `festzusetzende_est_rentner` zieht den Pauschbetrag einmal ueber die Summe aus Anlage R und Mitteilungen
//! (`engine::zugriff::teil2::rente::renten_nach_pauschbetrag`). Der Ehegatte (Person B) behaelt seinen eigenen Pauschbetrag
//! (`renten_partner`). Die Scheibe `gesamt` kennt keine Anlage-R-Rente (`kein_sonstige` = nein sperrt dort), sie rechnet nur die
//! Mitteilungen (`rav_bav_ring_hermetisch.rs`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand. Gesetzliche Rente mit Beginn 2025: Besteuerungsanteil 83,5 % (§ 22 Nr. 1 Satz 3
//! Buchst. a Doppelbuchst. aa, `params/kohorten/rente_besteuerungsanteil_p22.yaml`), also 12.000 Euro = 10.020 Euro; die Doctests
//! der Engine nennen 9.918 Euro nach dem Pauschbetrag. Nr. 1 der Mitteilung ist voll steuerpflichtig. Gemessen wird der Unterschied des
//! Gesamtbetrags der Einkuenfte zur Akte ohne Rente und ohne Mitteilung.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::HashMap;

use bescheid::deklaration::{feste_zahl, Cfg};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Umgebung;
use bescheid::Felder;
use bindung::Bindung;
use domain::{Feldtyp, Scheibe, Vz};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const VZ: Vz = Vz::Vz2025;
const SCHEIBE: Scheibe = Scheibe::RentnerGesamt;

type Paare = Vec<(String, Value)>;

fn p(feld: &str, wert: Value) -> (String, Value) {
    (feld.to_owned(), wert)
}

/// Euro in Cent.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

/// Der Abwesenheitswert, nie der illustrative Beispielwert (wie `standardwert` in `rav_bav_ring_hermetisch.rs`).
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

/// Der volle Pflicht-Kegel der Scheibe; was der Test setzt, gewinnt, der Rest folgt bestaetigt.
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(String, Value)]) -> Paare {
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Paare = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .rev()
                .find(|(g, _)| g == *f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            ((*f).to_owned(), w)
        })
        .collect();
    raus.extend(
        gesetzt
            .iter()
            .filter(|(f, _)| !kegel.contains(&f.as_str()))
            .cloned(),
    );
    raus
}

/// Der Gesamtbetrag der Einkuenfte in Euro; ein Fall ohne Zahl ist ein Fehlschlag des Tests.
fn gdb(paare: &[(String, Value)]) -> i64 {
    let cfg = Cfg::fuer(SCHEIBE);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let alle = kegel_fuer(&cfg, paare);
    let events: Vec<(&str, Value, bool)> = alle
        .iter()
        .map(|(f, w)| (f.as_str(), w.clone(), true))
        .collect();
    let store: Store = store(&events);
    let felder: Felder = felder(&store);
    let achsen: Vec<AchsenBindung> = teil.iter().map(|b| AchsenBindung::from(*b)).collect();
    let index: HashMap<String, &'static Bindung> =
        teil.iter().map(|b| (b.feld_id.clone(), *b)).collect();
    let kegel = cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &achsen,
        index: &index,
        params: params(),
    };
    match feste_zahl(&felder, &cfg, VZ, &kegel, &umg, Some(&store), None) {
        Ok(Ok(z)) => z
            .extras
            .kette
            .expect("KONTROLLE: Kette fehlt")
            .gesamtbetrag_der_einkuenfte
            .get(),
        Ok(Err(k)) => panic!("KONTROLLE: erwartet eine Zahl, bekommen {:?}", k.grund),
        Err(e) => panic!("KONTROLLE: erwartet eine Zahl, bekommen {e:?}"),
    }
}

/// Der Name des Feldes in der Instanz `n` (Instanz 1 traegt den Basisnamen).
fn name(basis: &str, n: u8) -> String {
    if n == 1 {
        basis.to_owned()
    } else {
        format!("{basis}__{n}")
    }
}

/// Eine gesetzliche Rente der Anlage R in der Instanz `n`: Beginn `beginn`, Jahresbetrag in Euro, `freibetrag` in Euro (nur ab dem
/// zweiten Rentenjahr, sonst 0).
fn rente(n: u8, beginn: i64, euro: i64, freibetrag: i64) -> Paare {
    vec![
        p(&name("rentner_renten_art", n), json!("gesetzliche_rente")),
        p(&name("rentner_jahresrente", n), json!(cent(euro))),
        p(&name("rentner_renten_beginn_jahr", n), json!(beginn)),
        p(&name("rentner_alter_bei_rentenbeginn", n), json!(65)),
        p(
            &name("rentner_rentenfreibetrag", n),
            json!(cent(freibetrag)),
        ),
    ]
}

/// Eine Mitteilung der Instanz 1 mit Betrag Nr. 1 und Nr. 5 in Euro (Beginn der Nr. 5 am 05.05.2020 = 65 Jahre, 18 %).
fn mitteilung(nr1: i64, nr5: i64) -> Paare {
    vec![
        p("rav_bav_nr1_betrag", json!(cent(nr1))),
        p("rav_bav_nr4_betrag", json!(0)),
        p("rav_bav_nr5_betrag", json!(cent(nr5))),
        p("rav_bav_nr5_beginn", json!("05.05.2020")),
        p("rav_bav_andere_nummern", json!(false)),
        p("rav_bav_basisrente", json!(false)),
    ]
}

/// Die Akte: Einzelveranlagung, geboren am 05.05.1955, die gegebenen Renten der Anlage R.
fn akte(renten: Paare) -> Paare {
    let mut a = vec![
        p("veranlagung", json!("einzel")),
        p("stammdaten_geburtsdatum", json!("05.05.1955")),
        p("kein_gewinn", json!(true)),
        p("kein_kap", json!(true)),
        p("kein_vuv", json!(true)),
        p("kein_sonstige", json!(false)),
        p("agb_zwangslaeufig", json!(true)),
        p("agb_notwendig_angemessen", json!(true)),
    ];
    a.extend(renten);
    a
}

/// Dieselbe Akte mit dem Kreuz "ich habe eine Rente mit Leistungsmitteilung" und der Mitteilung.
fn mit_mitteilung(renten: Paare, m: Paare) -> Paare {
    let mut a = akte(renten);
    a.push(p("kein_rav_bav", json!(false)));
    a.extend(m);
    a
}

/// Die Akte ohne jede Rente: Anlage R mit Jahresbetrag 0, keine Mitteilung.
fn leer() -> i64 {
    gdb(&akte(rente(1, 2025, 0, 0)))
}

/// Anlage R und Mitteilung teilen sich einen Pauschbetrag. Zur Kontrolle stehen die zwei Einzelwerte davor: 9.918 Euro (Anlage R
/// allein: 10.020 − 102) und 2.898 Euro (Mitteilung allein: 3.000 − 102). Beides zusammen ist 10.020 + 3.000 − 102 = 12.918 Euro,
/// nicht 9.918 + 2.898 = 12.816 Euro. Mit Nr. 5 (10.000 Euro, 18 % = 1.800 Euro) wird der Ertragsanteil zusammen mit der Anlage R
/// um den einen Pauschbetrag gekuerzt: 10.020 + 1.800 − 102 = 11.718 Euro.
#[test]
fn anlage_r_und_mitteilung_teilen_sich_einen_pauschbetrag() {
    let null = leer();
    let r = || rente(1, 2025, 12_000, 0);
    assert_eq!(gdb(&akte(r())) - null, 9_918, "KONTROLLE: Anlage R allein");
    assert_eq!(
        gdb(&mit_mitteilung(rente(1, 2025, 0, 0), mitteilung(3_000, 0))) - null,
        2_898,
        "KONTROLLE: Mitteilung allein"
    );
    assert_eq!(
        gdb(&mit_mitteilung(r(), mitteilung(3_000, 0))) - null,
        12_918,
        "Anlage R und Nr. 1"
    );
    assert_eq!(
        gdb(&mit_mitteilung(r(), mitteilung(0, 10_000))) - null,
        11_718,
        "Anlage R und Nr. 5"
    );
}

/// Zwei Renten der Anlage R teilen sich einen Pauschbetrag (Python zog 102 Euro je Rente ab): 12.000 Euro = 10.020 Euro und 6.000 Euro
/// = 5.010 Euro, zusammen 15.030 − 102 = 14.928 Euro, nicht 9.918 + 4.908 = 14.826 Euro.
#[test]
fn zwei_renten_der_anlage_r_teilen_sich_einen_pauschbetrag() {
    let null = leer();
    let zwei = [rente(1, 2025, 12_000, 0), rente(2, 2025, 6_000, 0)].concat();
    assert_eq!(gdb(&akte(zwei)) - null, 14_928);
}

/// Der Anteil einer Rente vor dem Pauschbetrag hat den Boden 0: ab dem zweiten Rentenjahr rechnet die Engine "Jahresbetrag − Freibetrag",
/// und ein Freibetrag ueber dem Jahresbetrag (1.000 − 5.000 = −4.000 Euro) mindert eine zweite Rente nicht. Die zweite Rente (3.000
/// Euro, Beginn 2025 = 2.505 Euro) bleibt bei 2.505 − 102 = 2.403 Euro.
#[test]
fn ein_negativer_anteil_einer_rente_mindert_die_andere_nicht() {
    let null = leer();
    let beide = [rente(1, 2020, 1_000, 5_000), rente(2, 2025, 3_000, 0)].concat();
    assert_eq!(gdb(&akte(beide)) - null, 2_403);
}

/// Der Ehegatte (Person B) behaelt seinen eigenen Pauschbetrag: § 9a gibt ihn je Person. Person A mit Nr. 1 (3.000 Euro, 2.898 Euro
/// nach ihrem Pauschbetrag) und der Ehegatte mit einer Rente von 12.000 Euro (9.918 Euro nach seinem Pauschbetrag) bringen zusammen
/// 12.816 Euro, nicht 12.918 Euro. Der Ring misst hier ohne Guard; die Zusammenveranlagung mit einer Mitteilung sperrt die Abgabe
/// (`rav_bav_waechter_hermetisch.rs`).
#[test]
fn der_ehegatte_behaelt_seinen_eigenen_pauschbetrag() {
    let partner = |euro: i64| {
        vec![
            p("veranlagung", json!("zusammen")),
            p("kein_sonstige_partner", json!(false)),
            p("rentner_renten_art_partner", json!("gesetzliche_rente")),
            p("rentner_jahresrente_partner", json!(cent(euro))),
            p("rentner_renten_beginn_jahr_partner", json!(2025)),
            p("rentner_alter_bei_rentenbeginn_partner", json!(65)),
            p("rentner_rentenfreibetrag_partner", json!(0)),
        ]
    };
    let ohne = [akte(rente(1, 2025, 0, 0)), partner(0)].concat();
    let mit = [
        mit_mitteilung(rente(1, 2025, 0, 0), mitteilung(3_000, 0)),
        partner(12_000),
    ]
    .concat();
    assert_eq!(gdb(&mit) - gdb(&ohne), 2_898 + 9_918);
}

/// Der Pauschbetrag mindert zuerst die Renten, die nicht in die Bemessung des Altersentlastungsbetrags eingehen (die Anlage R,
/// § 24a Satz 2 Nr. 2). Mit einer Rente der Anlage R ueber 102 Euro bleibt Nr. 1 (2.000 Euro) voll in der Bemessung: 16 % = 320 Euro,
/// Zuwachs 2.000 − 320 = 1.680 Euro. Nr. 1 allein trueg den Pauschbetrag selbst: Bemessung 1.898 Euro.
#[test]
fn die_altersentlastung_bemisst_sich_nach_dem_betrag_ueber_dem_gemeinsamen_pauschbetrag() {
    let alt = [p("geburtsjahr", json!(1955))];
    let r = || rente(1, 2025, 12_000, 0);
    let mit = [mit_mitteilung(r(), mitteilung(2_000, 0)), alt.to_vec()].concat();
    let ohne = [akte(r()), alt.to_vec()].concat();
    assert_eq!(gdb(&mit) - gdb(&ohne), 1_680);
}
