//! Rentner-Scheibe: der Ring rechnet Arbeitslohn und Versorgungsbezuege (§ 19 `EStG`) mit, bevor die Scheibe sie fragt.
//! Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Die Rentner-Erklaerung fragte die Lohnsteuer, aber nie den Bruttoarbeitslohn und die
//! Versorgungsbezuege, und der Rentner-Ring kannte beide nicht. Wer sie trotzdem hatte, rechnete der Ring zu niedrig.
//!
//! **Warum es zaehlt.** Ein Lohn von 6.000, 60.000 oder 600.000 Euro aenderte die Steuer um 0 Euro. Nur die Felder
//! freizuschalten haette die Erklaerung abgabefaehig gemacht und die Steuer still zu niedrig gelassen
//! (Entscheidung `rentner-ring-liest-versorgungsbezuege-vor-der-scheibe`: der Ring zuerst, dann die Felder).
//!
//! **Wo es sitzt.** `zweige/rentner.rs` (`festzusetzende_est_rentner`) ruft denselben Kern wie der Gesamt-Ring
//! (`zweige/gesamt.rs::einkuenfte_ns_aus_lohn`). Die Tests hier setzen die Felder ohne Scheiben-Gate in den Store, sie
//! messen den Ring; die Scheibe misst `api/tests/rentner_lohn_versorgung_hermetisch.rs`.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus dem Gesetzestext und `params/`, nie aus dem Rust-Code gelesen.
//! - Arbeitslohn: Einkuenfte = Lohn − 1.230 Euro (`params/2025/arbeitnehmerpauschbetrag.yaml`, § 9a Satz 1 Nr. 1a).
//! - Versorgungsbezug: Einkuenfte = Bezug − (min(13,2 % der Bemessungsgrundlage, 990) + 297) − 102 Euro
//!   (`estg_p19_2026-07-17.txt`: 2025 → 13,2 / 990 / 297; § 9a Satz 1 Nr. 1b: 102 Euro). Bei 30.000 Euro:
//!   30.000 − 1.287 − 102 = 28.611.
//! - Alters-Gate (§ 19 Abs. 2 Satz 2 Nr. 2): vor dem 63. Lebensjahr (60. bei Grad der Behinderung ab 50) gilt der
//!   Bezug als Arbeitslohn, ohne Versorgungsfreibetrag: 30.000 − 1.230 = 28.770.
//! - § 24a: 2020er Kohorte (Geburtsjahr 1955) 16,0 % bis 760 Euro (`estg_p24a_2026-07-13.txt`); der Arbeitslohn zaehlt
//!   in die Bemessung, die Rente nicht (Satz 2 Nr. 2).
//! - Tarif: `params/2025/einkommensteuertarif_p32a.yaml`, Sonderausgaben-Pauschbetrag 36 Euro.
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
        _ => panic!(
            "KONTROLLE: Kegel-Feld {} hat keinen Abwesenheitswert",
            b.feld_id
        ),
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

/// Ein Fall der Scheibe, mit der Scheiben-Bindung (`api._scheibe_bindung`) als Index und Achsen.
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

/// Die Kette eines Falls mit Zahl; jeder andere Ausgang ist ein Fehlschlag des Tests.
fn kette(paare: &[(&'static str, Value)]) -> Kette {
    match ergebnis(&fall(paare)) {
        Ausgang::Zahl(k) => (*k).expect("KONTROLLE: Kette fehlt"),
        Ausgang::Anders(was) => panic!("KONTROLLE: erwartet eine Zahl, bekommen {was}"),
        Ausgang::Gesperrt(g) => panic!("KONTROLLE: erwartet eine Zahl, bekommen die Sperre {g}"),
    }
}

fn gesperrt_mit(paare: &[(&'static str, Value)]) -> String {
    match ergebnis(&fall(paare)) {
        Ausgang::Gesperrt(g) => g,
        Ausgang::Anders(was) => panic!("KONTROLLE: erwartet eine Sperre, bekommen {was}"),
        Ausgang::Zahl(_) => panic!("KONTROLLE: erwartet eine Sperre, bekommen eine Zahl"),
    }
}

/// Gesamtbetrag der Einkuenfte in Euro.
fn gdb(paare: &[(&'static str, Value)]) -> i64 {
    kette(paare).gesamtbetrag_der_einkuenfte.get()
}

/// Rentnerin im Beginnjahr 2025 mit 20.000 Euro gesetzlicher Rente, einzeln veranlagt, alle Kreuze "nein" (die Angaben
/// des Pflicht-Kegels), plus `mehr`. Ohne Lohn und ohne Versorgung ist es der Bestand von heute.
fn rentner(mehr: Paare) -> Paare {
    let mut p: Paare = vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(cent(20_000))),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_rentenfreibetrag", json!(0)),
        ("veranlagung", json!("einzel")),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ];
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

/// § 32a Abs. 1 `EStG` fuer VZ 2025, von Hand aus `params/2025/einkommensteuertarif_p32a.yaml` (Euro, abgerundet).
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

// ---------------------------------------------------------------- Arbeitslohn

/// 6.000, 60.000 und 600.000 Euro Lohn: der Gesamtbetrag waechst um Lohn minus Arbeitnehmer-Pauschbetrag, die
/// Steuer steigt mit jedem Schritt, und die tarifliche Steuer folgt dem Tarif auf das zvE (Sonderausgaben-Pauschbetrag
/// 36 Euro). Heute: dreimal dieselbe Zahl, weil der Ring den Lohn nie liest.
#[test]
fn der_lohn_geht_mit_dem_arbeitnehmer_pauschbetrag_in_den_gesamtbetrag() {
    let ohne = kette(&rentner(vec![]));
    let mut steuer = vec![ohne.festzusetzende_est.get()];
    for euro in [6_000, 60_000, 600_000] {
        let mit = kette(&rentner(lohn(euro)));
        assert_eq!(
            mit.gesamtbetrag_der_einkuenfte.get() - ohne.gesamtbetrag_der_einkuenfte.get(),
            euro - 1_230,
            "{euro} Euro Lohn: Gesamtbetrag der Einkuenfte"
        );
        assert_eq!(
            mit.tarifliche_est.get(),
            tarif_2025(mit.zu_versteuerndes_einkommen.get()),
            "{euro} Euro Lohn: die tarifliche Steuer folgt dem Tarif auf das zvE {}",
            mit.zu_versteuerndes_einkommen.get()
        );
        steuer.push(mit.festzusetzende_est.get());
    }
    assert!(
        steuer.windows(2).all(|w| w[0] < w[1]),
        "die Steuer steigt mit dem Lohn: {steuer:?}"
    );
}

/// Ein Lohn unter dem Pauschbetrag erzeugt keine negativen Einkuenfte: 1.000 Euro Lohn aendern den Gesamtbetrag nicht.
#[test]
fn ein_lohn_unter_dem_pauschbetrag_aendert_den_gesamtbetrag_nicht() {
    assert_eq!(gdb(&rentner(lohn(1_000))), gdb(&rentner(vec![])));
}

// ---------------------------------------------------------------- Versorgungsbezuege

/// 0, 30.000 und 60.000 Euro Versorgung: Freibetrag (13,2 %, hoechstens 990), Zuschlag 297 und Pauschbetrag 102 Euro
/// gehen ab, der Rest in den Gesamtbetrag. Heute: dreimal dieselbe Zahl.
#[test]
fn versorgungsbezuege_gehen_nach_freibetrag_zuschlag_und_pauschbetrag_in_den_gesamtbetrag() {
    let ohne = kette(&rentner(vec![]));
    let mut steuer = vec![ohne.festzusetzende_est.get()];
    for (euro, einkuenfte) in [(0, 0), (30_000, 28_611), (60_000, 58_611)] {
        let mit = kette(&rentner(versorgung(euro)));
        assert_eq!(
            mit.gesamtbetrag_der_einkuenfte.get() - ohne.gesamtbetrag_der_einkuenfte.get(),
            einkuenfte,
            "{euro} Euro Versorgung: Gesamtbetrag der Einkuenfte"
        );
        steuer.push(mit.festzusetzende_est.get());
    }
    assert_eq!(steuer[0], steuer[1], "0 Euro Versorgung aendern nichts");
    assert!(
        steuer[1] < steuer[2] && steuer[2] < steuer[3],
        "die Steuer steigt mit der Versorgung: {steuer:?}"
    );
}

/// Alters-Gate (§ 19 Abs. 2 Satz 2 Nr. 2): Beginn mit 62 gilt als Arbeitslohn (ohne Freibetrag, mit Pauschbetrag
/// 1.230), Beginn mit 63 als Versorgungsbezug. Ab Grad der Behinderung 50 liegt die Grenze bei 60.
#[test]
fn das_alters_gate_gilt_auch_auf_der_rentner_scheibe() {
    let ohne = gdb(&rentner(vec![]));
    let altersgrenze = |alter: i64, gdb_grad: i64| {
        let mut p = versorgung(30_000);
        p.push(("versorgung_art", json!("altersgrenze_sonstige")));
        p.push(("versorgung_alter_bei_beginn", json!(alter)));
        p.push(("rentner_grad_der_behinderung", json!(gdb_grad)));
        gdb(&rentner(p)) - ohne
    };
    assert_eq!(altersgrenze(62, 0), 28_770, "62 Jahre: Arbeitslohn");
    assert_eq!(altersgrenze(63, 0), 28_611, "63 Jahre: Versorgungsbezug");
    assert_eq!(
        altersgrenze(59, 50),
        28_770,
        "59 Jahre, Grad 50: Arbeitslohn"
    );
    assert_eq!(
        altersgrenze(60, 50),
        28_611,
        "60 Jahre, Grad 50: Versorgungsbezug"
    );
}

/// Lohn und Versorgung addieren sich: 20.000 Euro Lohn (18.770) und 30.000 Euro Versorgung (28.611).
#[test]
fn lohn_und_versorgung_addieren_sich() {
    let ohne = gdb(&rentner(vec![]));
    let mut p = lohn(20_000);
    p.extend(versorgung(30_000));
    assert_eq!(gdb(&rentner(p)) - ohne, 18_770 + 28_611);
}

/// Die Sperre fuer einen Versorgungsbezug ohne Beginnjahr und Bemessungsgrundlage greift jetzt auch hier: der Ring
/// liest den Bezug, also rechnet er ihn nicht mit einem Freibetrag von 0 Euro.
#[test]
fn ein_versorgungsbezug_ohne_beginn_und_bemessungsgrundlage_sperrt() {
    let p = rentner(vec![("versorgung_jahresrente", json!(cent(30_000)))]);
    assert_eq!(gesperrt_mit(&p), "versorgungsfreibetrag_offen");
}

// ---------------------------------------------------------------- Nachbarn, die den Lohn lesen muessen

/// § 24a: Der Arbeitslohn zaehlt in die Bemessung des Altersentlastungsbetrags, die Rente nicht (Satz 2 Nr. 2).
/// Geboren 1955 (Kohorte 2020: 16,0 %, hoechstens 760 Euro): mit 20.000 Euro Lohn sind es 760 Euro weniger im
/// Gesamtbetrag; ohne Lohn ist die Bemessung null, und das Geburtsjahr aendert nichts.
#[test]
fn der_altersentlastungsbetrag_rechnet_den_lohn_mit_und_die_rente_nicht() {
    let geboren = vec![("geburtsjahr", json!(1955))];
    let mit_lohn = gdb(&rentner(lohn(20_000)));
    let mit_lohn_und_alter = gdb(&rentner([lohn(20_000), geboren.clone()].concat()));
    assert_eq!(mit_lohn - mit_lohn_und_alter, 760, "Lohn in der Bemessung");
    assert_eq!(
        gdb(&rentner(geboren)),
        gdb(&rentner(vec![])),
        "ohne Lohn: die Rente bleibt ausser Betracht"
    );
}

/// § 35 Abs. 1 Satz 2 `EStG`: der Ermaessigungshoechstbetrag ist die tarifliche Steuer mal Gewinn aus Gewerbe durch
/// die Summe aller positiven Einkuenfte, und der Lohn gehoert in diese Summe. Gewerbe mit 5.000 Euro Gewinn,
/// Messbetrag 1.500 Euro (vierfach 6.000, der Hoechstbetrag greift vorher); ohne Lohn und mit 60.000 Euro Lohn.
/// Die Summe ist hier der Gesamtbetrag der Einkuenfte (nur positive Einkuenfte, kein Altersentlastungsbetrag).
/// Ohne den Lohn im Nenner stuende ueber 5.000 Euro Ermaessigung da statt 1.420.
#[test]
fn die_gewerbesteuer_ermaessigung_zaehlt_den_lohn_im_nenner() {
    let gewerbe = |messbetrag: i64| -> Paare {
        vec![
            ("kein_gewinn", json!(false)),
            ("einkuenfte_gewinn", json!(cent(5_000))),
            ("gewinn_betriebsart", json!("gewerbe")),
            ("gewst_hebesatz", json!(400)),
            ("gewst_messbetrag", json!(messbetrag)),
        ]
    };
    for (lohn_euro, soll) in [(0, 470), (60_000, 1_420)] {
        let ohne_35 = kette(&rentner([gewerbe(0), lohn(lohn_euro)].concat()));
        let mit_35 = kette(&rentner([gewerbe(150_000), lohn(lohn_euro)].concat()));
        let ermaessigung = ohne_35.festzusetzende_est.get() - mit_35.festzusetzende_est.get();
        let summe = ohne_35.gesamtbetrag_der_einkuenfte.get();
        assert_eq!(
            ermaessigung,
            ohne_35.tarifliche_est.get() * 5_000 / summe,
            "{lohn_euro} Euro Lohn: Steuer {} mal 5.000 durch Summe {summe}",
            ohne_35.tarifliche_est.get()
        );
        assert_eq!(ermaessigung, soll, "{lohn_euro} Euro Lohn: Ermaessigung");
    }
}
