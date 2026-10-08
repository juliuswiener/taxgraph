//! § 34c Abs. 2 `EStG`, Abzug statt Anrechnung (Abweichung Nr. 41 in `rust/fixtures/README.md`): der Abzug kuerzt die
//! EINKUENFTE, nicht das Einkommen. Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Wer die im Ausland gezahlte Steuer abziehen will, statt sie anrechnen zu lassen, zieht sie "bei der
//! Ermittlung der Einkuenfte" ab (§ 34c Abs. 2 S. 1). Die Anleitung zur Anlage AUS 2025 (Zeile 10) macht daraus fuer Arbeitslohn
//! Werbungskosten der Anlage N. Der Bescheid zog sie bis Nr. 41 erst beim Einkommen ab und ueberging damit den
//! Arbeitnehmer-Pauschbetrag: Lohn 51.266 Euro, 2.000 Euro Steuer, Steuer 9.988 statt 10.419 Euro.
//!
//! **Warum es zaehlt.** 431 Euro zu wenig Steuer im Bescheid fuer jeden, der den Abzug waehlt (Lohn 51.266 Euro, 2.000 Euro
//! Steuer, Einzelveranlagung, VZ 2025).
//!
//! **Wo es sitzt.** `rust/bescheid/src/einkuenfte.rs` (`dba_abzug_werbungskosten`, `shared_dba_sonstige`) und
//! `rust/bescheid/src/zweige/gesamt.rs::ns_werbungskosten`. Die Sperren fuer die Faelle, die der Bau nicht rechnet, pruefen
//! `p34c_abzug_sperre.rs`; die Rechnung der Anrechnung und der Freistellung bleibt in `p34c_abzug_rechnung.rs`.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Handrechnung aus `params/2025/einkommensteuertarif_p32a.yaml` (`tarif_2025` unten, Euro
//! abgerundet), nie aus dem Rust-Tarif. Fall: Lohn 51.266 Euro. Ohne Abzug sind es 51.266 - 1.230 (Pauschbetrag) = 50.036
//! Euro Einkuenfte, minus 36 Euro Sonderausgaben-Pauschbetrag = zvE 50.000 Euro, Steuer 10.691 Euro. Mit 2.000 Euro Abzug
//! sind die Werbungskosten 2.000 Euro (mehr als 1.230, sie ERSETZEN den Pauschbetrag): 49.266 Euro Einkuenfte, zvE 49.230
//! Euro, Steuer 10.419 Euro.
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
const WAHL: &str = "dba_abzug_statt_anrechnung";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const UNFALL: &str = "ep_unfallkosten";
const ART: &str = "dba_einkunftsart";
const FIKTIV: &str = "dba_fiktive_steuer_vorhanden";

type Paare = Vec<(&'static str, Value)>;

/// Cent aus Euro.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

/// Der Wert, mit dem ein Kegel-Feld ohne Antwort belegt wird (Abwesenheitswert der Bindung, sonst 0/`false`).
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

/// Die Paare plus ein Standardwert fuer jedes Kegel-Feld der Scheibe, das sie nicht nennen.
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

struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

fn fall(scheibe: Scheibe, paare: &[(&'static str, Value)]) -> Fall {
    let cfg = Cfg::fuer(scheibe);
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

/// Die Steuer in Euro und die Kette der Gesamt-Scheibe; der Guard muss vorher frei sein (sonst ist der Fall falsch gebaut).
fn lauf(paare: &[(&'static str, Value)]) -> (i64, Kette) {
    rechne(paare, true)
}

/// Wie [`lauf`], aber OHNE den Guard: die Rechnung allein, auch fuer Faelle, die der Guard sperrt. Zeigt, was die Rechnung
/// selbst traegt, unabhaengig vom Guard davor.
fn lauf_ohne_guard(paare: &[(&'static str, Value)]) -> (i64, Kette) {
    rechne(paare, false)
}

fn rechne(paare: &[(&'static str, Value)], mit_guard: bool) -> (i64, Kette) {
    let f = fall(Scheibe::Gesamt, paare);
    let q = Instanzquelle {
        store: Some(&f.store),
        bindung: Some(&f.index),
        nur_bestaetigt: false,
    };
    if mit_guard {
        match an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(VZ), &q) {
            Ok(None) => {}
            Ok(Some(g)) => panic!("KONTROLLE: erwartet eine Zahl, bekommen die Sperre {g}"),
            Err(e) => panic!("KONTROLLE: Guard: {e:?}"),
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
        Ok(Ok(z)) => {
            assert_eq!(z.zahl.get() % 100, 0, "Cent-Rest in der Steuer: {z:?}");
            (
                z.zahl.get() / 100,
                z.extras.kette.expect("KONTROLLE: keine Kette"),
            )
        }
        Ok(Err(k)) => panic!("KONTROLLE: ohne Zahl: {:?}", k.grund),
        Err(e) => panic!("KONTROLLE: {e:?}"),
    }
}

/// § 32a Abs. 1 `EStG` VZ 2025 von Hand aus `params/2025/einkommensteuertarif_p32a.yaml`, Euro abgerundet.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn tarif_2025(zve: i64) -> i64 {
    let z = zve as f64;
    let wert = if zve <= 12_096 {
        0.0
    } else if zve <= 17_443 {
        let y = (z - 12_096.0) / 10_000.0;
        (932.30 * y + 1_400.0) * y
    } else if zve <= 68_480 {
        let y = (z - 17_443.0) / 10_000.0;
        (176.64 * y + 2_397.0) * y + 1_015.13
    } else if zve <= 277_825 {
        0.42 * z - 10_911.92
    } else {
        0.45 * z - 19_246.67
    };
    wert.floor() as i64
}

/// Lohn 51.266 Euro, Einzelveranlagung, ein Staat ohne Abkommen, Einkunftsart Arbeitslohn, 2.000 Euro gezahlte Steuer auf
/// 10.000 Euro Auslandseinkuenfte (Teilmenge des Lohns). `wahl` waehlt den Abzug; `mehr` ersetzt oder ergaenzt Felder.
fn arbeitslohn(wahl: Option<bool>, mehr: &[(&'static str, Value)]) -> (i64, Kette) {
    lauf(&arbeitslohn_paare(wahl, mehr))
}

fn arbeitslohn_paare(wahl: Option<bool>, mehr: &[(&'static str, Value)]) -> Paare {
    let mut p: Paare = vec![
        ("bruttoarbeitslohn", json!(cent(51_266))),
        ("veranlagung", json!("einzel")),
        ("dba_staat", json!("sonstiger_staat")),
        ("dba_methode", json!("kein_dba")),
        ("dba_einkunftsart", json!("unselbstaendige_arbeit")),
        (STEUER, json!(cent(2_000))),
        (EINKUENFTE, json!(cent(10_000))),
    ];
    if let Some(w) = wahl {
        p.push((WAHL, json!(w)));
    }
    for (feld, wert) in mehr {
        p.retain(|(g, _)| g != feld);
        p.push((feld, wert.clone()));
    }
    p
}

/// KONTROLLE zuerst: ohne jede Auslandsangabe gilt die Handrechnung (zvE 50.000 Euro, Steuer 10.691 Euro). Sonst belegten die
/// Zahlen unten nichts: ein falscher Ausgangspunkt gaebe jedem Abzug-Test dieselbe Abweichung.
#[test]
fn kontrolle_ohne_auslandsangaben_gilt_die_handrechnung() {
    let (steuer, kette) = lauf(&[
        ("bruttoarbeitslohn", json!(cent(51_266))),
        ("veranlagung", json!("einzel")),
    ]);
    assert_eq!(kette.zu_versteuerndes_einkommen.get(), 50_000);
    assert_eq!(tarif_2025(50_000), 10_691, "die Handformel stimmt nicht");
    assert_eq!(steuer, 10_691);
}

/// KONTROLLE: ohne Wahl und mit Wahl `false` rechnet die Anrechnung. Die 2.000 Euro liegen unter dem Hoechstbetrag
/// (10.691 Euro * 10.000 / 50.000 = 2.138 Euro), also mindern sie die Steuer voll: 10.691 - 2.000 = 8.691 Euro. Das zvE bleibt
/// bei 50.000 Euro, der Abzug fasst die Einkuenfte nicht an.
#[test]
fn kontrolle_die_anrechnung_laesst_die_einkuenfte_stehen() {
    for wahl in [None, Some(false)] {
        let (steuer, kette) = arbeitslohn(wahl, &[]);
        assert_eq!(
            kette.zu_versteuerndes_einkommen.get(),
            50_000,
            "Wahl {wahl:?}"
        );
        assert_eq!(steuer, 10_691 - 2_000, "Wahl {wahl:?}");
    }
}

/// AK1: der Abzug kuerzt die Einkuenfte. 2.000 Euro Werbungskosten ersetzen den Pauschbetrag von 1.230 Euro: Einkuenfte
/// 49.266 Euro, zvE 49.230 Euro, Steuer 10.419 Euro (heute: 48.000 Euro und 9.988 Euro, das Einkommen).
#[test]
fn der_abzug_kuerzt_die_einkuenfte_nicht_das_einkommen() {
    let (steuer, kette) = arbeitslohn(Some(true), &[]);
    assert_eq!(kette.zu_versteuerndes_einkommen.get(), 49_230);
    assert_eq!(tarif_2025(49_230), 10_419, "die Handformel stimmt nicht");
    assert_eq!(steuer, 10_419);
    // Der Abzug rechnet keine Anrechnung zusaetzlich: sonst laege die Steuer um die 2.000 Euro tiefer.
    assert_ne!(steuer, 10_419 - 2_000);
}

/// Der Abzug zaehlt mit den uebrigen Werbungskosten zusammen: 800 Euro Unfallkosten plus 2.000 Euro Abzug sind 2.800 Euro,
/// Einkuenfte 48.466 Euro, zvE 48.430 Euro. Ein Abzug, der die Summe nicht bildet (er ersetzt die 800 Euro, oder er steht
/// neben dem Pauschbetrag), landet bei einem anderen zvE.
#[test]
fn der_abzug_bildet_mit_den_uebrigen_werbungskosten_eine_summe() {
    let (steuer, kette) = arbeitslohn(Some(true), &[(UNFALL, json!(cent(800)))]);
    assert_eq!(kette.zu_versteuerndes_einkommen.get(), 48_430);
    assert_eq!(steuer, tarif_2025(48_430));
    assert_ne!(
        tarif_2025(48_430),
        tarif_2025(49_230),
        "die Fallwahl trennt die Faelle nicht"
    );
}

/// Grenze: bleiben die Werbungskosten samt Abzug unter dem Pauschbetrag (1.000 Euro < 1.230 Euro), gilt der Pauschbetrag
/// weiter, und der Abzug aendert die Zahl nicht. Das folgt daraus, dass der Abzug "wie Werbungskosten" zaehlt (Anleitung
/// Anlage AUS 2025, Zeile 10; der Pauschbetrag steht anstelle der Werbungskosten, § 9a S. 1 Nr. 1 Buchst. a `EStG`).
#[test]
fn unter_dem_pauschbetrag_aendert_der_abzug_die_zahl_nicht() {
    let (steuer, kette) = arbeitslohn(Some(true), &[(STEUER, json!(cent(1_000)))]);
    assert_eq!(kette.zu_versteuerndes_einkommen.get(), 50_000);
    assert_eq!(steuer, 10_691);
}

/// Der Abzug braucht BEIDE Betraege (wie bisher): ohne Auslandseinkuenfte oder ohne gezahlte Steuer gibt es nichts abzuziehen.
#[test]
fn ohne_steuer_oder_ohne_auslandseinkuenfte_zieht_der_abzug_nichts_ab() {
    for (name, mehr) in [
        ("ohne Auslandseinkuenfte", (EINKUENFTE, json!(0))),
        ("ohne gezahlte Steuer", (STEUER, json!(0))),
    ] {
        let (_, kette) = arbeitslohn(Some(true), &[mehr]);
        assert_eq!(kette.zu_versteuerndes_einkommen.get(), 50_000, "{name}");
    }
}

/// Bei Freistellung gibt es keinen Abzug (Abweichung Nr. 35, § 34c Abs. 6): die Wahl aendert die Einkuenfte nicht, das zvE
/// bleibt bei 50.000 Euro (der Progressionsvorbehalt hebt nur den Satz).
#[test]
fn bei_freistellung_kuerzt_die_wahl_die_einkuenfte_nicht() {
    let (_, kette) = arbeitslohn(Some(true), &[("dba_methode", json!("dba_freistellung"))]);
    assert_eq!(kette.zu_versteuerndes_einkommen.get(), 50_000);
}

/// Die Rechnung kuerzt nur im Fall, den sie traegt (`dba_abzug_getragen`), auch wenn der Guard davor fehlt: bei Zinsen als
/// Einkunftsart und bei fiktiver Steuer bleibt das zvE bei 50.000 Euro (der Abzug geht nicht an Anlage N, und er wird auch
/// nicht beim Einkommen abgezogen). Ohne diese Grenze haengte ein Aufrufer ohne Guard die Zinssteuer an den Arbeitslohn.
#[test]
fn ausserhalb_des_getragenen_falls_kuerzt_die_rechnung_nichts() {
    let faelle = [
        ("Zinsen", (ART, json!("zinsen"))),
        ("fiktive Steuer", (FIKTIV, json!(true))),
    ];
    for (name, mehr) in faelle {
        let (_, kette) = lauf_ohne_guard(&arbeitslohn_paare(Some(true), &[mehr]));
        assert_eq!(kette.zu_versteuerndes_einkommen.get(), 50_000, "{name}");
    }
}
