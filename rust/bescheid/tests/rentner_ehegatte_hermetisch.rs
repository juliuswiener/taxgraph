//! Rentner-Scheibe, Zusammenveranlagung: der Ring rechnet Arbeitslohn und Versorgungsbezuege des Ehegatten (§ 19 `EStG`) mit.
//! Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Die Rentner-Erklaerung fragte Lohn und Versorgung nur fuer Person A (Abweichung Nr. 25). Hat der
//! Ehegatte Lohn oder Pension, zaehlte der Rentner-Ring sie als 0 und rechnete die Steuer zu niedrig.
//!
//! **Warum es zaehlt.** Ein Ehegatten-Lohn von 6.000, 60.000 oder 600.000 Euro aenderte die Steuer um 0 Euro. Nur die
//! Felder freizuschalten haette die Erklaerung abgabefaehig gemacht und die Steuer still zu niedrig gelassen
//! (Entscheidung `rentner-ehegatte-lohn-und-versorgung-wird-gefragt-nicht-gesperrt`; Reihenfolge wie bei Nr. 25: der Ring
//! zuerst, dann die Felder).
//!
//! **Wo es sitzt.** `zweige/rentner.rs` (`festzusetzende_est_rentner`) ruft `zweige/gesamt.rs::einkuenfte_ns_aus_lohn`
//! mit der Zusammenveranlagung; der Kern rechnet Lohn und Versorgung von Person B (`*_partner`). Die Tests hier setzen die
//! Felder ohne Scheiben-Gate in den Store, sie messen den Ring; die Scheibe misst `api/tests/rentner_ehegatte_hermetisch.rs`.
//!
//! **Abweichung Nr. 36.** Der Ring rechnet auch den Altersentlastungsbetrag (§ 24a `EStG`) des Ehegatten, mit seinem Lohn und
//! seinem Geburtsjahr (`zweige/rentner.rs` ruft `zweige/gesamt.rs::entlastungen`, den Weg des Gesamt-Zweigs). Vorher: nur
//! Person A.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus dem Gesetzestext und `params/`, nie aus dem Rust-Code gelesen.
//! - Altersentlastungsbetrag (§ 24a, `estg_p24a_2026-07-13.txt`, Tabelle): das auf die Vollendung des 64. Lebensjahres folgende
//!   Kalenderjahr bestimmt Satz und Hoechstbetrag: 2020 (geboren 1955) 16,0 % / 760 Euro, 2025 (geboren 1960) 13,2 % / 627 Euro.
//! - Arbeitslohn des Ehegatten: Einkuenfte = Lohn − 1.230 Euro (`params/2025/arbeitnehmerpauschbetrag.yaml`, § 9a Satz 1
//!   Nr. 1a; jeder Ehegatte hat seinen eigenen).
//! - Versorgungsbezug: Einkuenfte = Bezug − (min(13,2 % der Bemessungsgrundlage, 990) + 297) − 102 Euro
//!   (`estg_p19_2026-07-17.txt`: 2025 → 13,2 / 990 / 297; § 9a Satz 1 Nr. 1b: 102 Euro). Bei 30.000 Euro:
//!   30.000 − 1.287 − 102 = 28.611.
//! - Alters-Gate (§ 19 Abs. 2 Satz 2 Nr. 2): vor dem 63. Lebensjahr (60. bei Grad der Behinderung ab 50) gilt der
//!   Bezug als Arbeitslohn, ohne Versorgungsfreibetrag: 30.000 − 1.230 = 28.770. Der Grad ist der des EHEGATTEN.
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

/// Rentnerin im Beginnjahr 2025 mit 20.000 Euro gesetzlicher Rente, ZUSAMMEN veranlagt (mit den fuenf Kapital-Feldern des
/// Partner-Kegels auf null), alle Kreuze "nein" (die Angaben des Pflicht-Kegels), plus `mehr`. Ohne Angaben zum Ehegatten ist
/// es der Bestand von heute.
fn paar(mehr: Paare) -> Paare {
    mit_veranlagung("zusammen", mehr)
}

fn mit_veranlagung(veranlagung: &'static str, mehr: Paare) -> Paare {
    let mut p: Paare = vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(cent(20_000))),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_rentenfreibetrag", json!(0)),
        ("veranlagung", json!(veranlagung)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ];
    for (f, w) in mehr {
        match p.iter_mut().find(|(g, _)| *g == f) {
            Some(e) => e.1 = w,
            None => p.push((f, w)),
        }
    }
    p
}

fn lohn_partner(euro: i64) -> Paare {
    vec![("bruttoarbeitslohn_partner", json!(cent(euro)))]
}

/// Ein Versorgungsbezug des Ehegatten von `euro` mit der Bemessungsgrundlage gleich dem Bezug, Beginn 2025,
/// beamtenrechtlich (kein Alters-Gate).
fn versorgung_partner(euro: i64) -> Paare {
    vec![
        ("versorgung_jahresrente_partner", json!(cent(euro))),
        ("versorgung_bemessungsgrundlage_partner", json!(cent(euro))),
        ("versorgung_beginn_jahr_partner", json!(2025)),
        ("versorgung_art_partner", json!("beamtenrechtlich")),
    ]
}

/// Ein reiner Pensionaer als Ehegatte (Abweichung Nr. 50): Nr. 3 der Lohnsteuerbescheinigung (Bruttoarbeitslohn) enthaelt den
/// Bezug aus Nr. 8, beide sind `euro`. Der Arbeitslohn ist 0, es bleibt die Versorgung; die Erwartungswerte unten gelten
/// unveraendert. Lohn und Bezug in einem Fall: `versorgung_im_lohn_enthalten.rs`.
fn pensionaer_partner(euro: i64) -> Paare {
    [lohn_partner(euro), versorgung_partner(euro)].concat()
}

// ---------------------------------------------------------------- Arbeitslohn des Ehegatten

/// 6.000, 60.000 und 600.000 Euro Lohn des Ehegatten: der Gesamtbetrag waechst um Lohn minus dessen Arbeitnehmer-
/// Pauschbetrag, und die Steuer steigt mit jedem Schritt. Heute: dreimal dieselbe Zahl, weil der Ring den Lohn nie liest.
/// Die Rente von Person A ist hier 40.000 Euro statt 20.000: bei Zusammenveranlagung (doppelter Grundfreibetrag) laegen
/// 20.000 Euro Rente plus 6.000 Euro Lohn noch bei einer Steuer von 0, und der erste Schritt zeigte nichts.
#[test]
fn der_lohn_des_ehegatten_geht_mit_dem_arbeitnehmer_pauschbetrag_in_den_gesamtbetrag() {
    let rente = || vec![("rentner_jahresrente", json!(cent(40_000)))];
    let ohne = kette(&paar(rente()));
    let mut steuer = vec![ohne.festzusetzende_est.get()];
    for euro in [6_000, 60_000, 600_000] {
        let mit = kette(&paar([rente(), lohn_partner(euro)].concat()));
        assert_eq!(
            mit.gesamtbetrag_der_einkuenfte.get() - ohne.gesamtbetrag_der_einkuenfte.get(),
            euro - 1_230,
            "{euro} Euro Lohn des Ehegatten: Gesamtbetrag der Einkuenfte"
        );
        steuer.push(mit.festzusetzende_est.get());
    }
    assert!(
        steuer.windows(2).all(|w| w[0] < w[1]),
        "die Steuer steigt mit dem Lohn des Ehegatten: {steuer:?}"
    );
}

/// Ein Lohn unter dem Pauschbetrag erzeugt keine negativen Einkuenfte: 1.000 Euro aendern den Gesamtbetrag nicht.
#[test]
fn ein_ehegattenlohn_unter_dem_pauschbetrag_aendert_den_gesamtbetrag_nicht() {
    assert_eq!(
        gdb(&paar(lohn_partner(1_000))),
        gdb(&paar(vec![])),
        "1.000 Euro Lohn des Ehegatten"
    );
}

/// Lohn von Person A und Lohn des Ehegatten sind getrennte Summanden mit je eigenem Pauschbetrag: beide zusammen bringen
/// (20.000 − 1.230) + (30.000 − 1.230).
#[test]
fn der_lohn_beider_ehegatten_hat_je_einen_pauschbetrag() {
    let ohne = gdb(&paar(vec![]));
    let mut p = lohn_partner(30_000);
    p.push(("bruttoarbeitslohn", json!(cent(20_000))));
    assert_eq!(gdb(&paar(p)) - ohne, 18_770 + 28_770);
}

// ---------------------------------------------------------------- Versorgungsbezuege des Ehegatten

/// 0, 30.000 und 60.000 Euro Versorgung des Ehegatten: Freibetrag (13,2 %, hoechstens 990), Zuschlag 297 und Pauschbetrag
/// 102 Euro gehen ab, der Rest in den Gesamtbetrag. Heute: dreimal dieselbe Zahl.
#[test]
fn versorgungsbezuege_des_ehegatten_gehen_nach_freibetrag_zuschlag_und_pauschbetrag_in_den_gesamtbetrag() {
    let ohne = kette(&paar(vec![]));
    let mut steuer = vec![ohne.festzusetzende_est.get()];
    for (euro, einkuenfte) in [(0, 0), (30_000, 28_611), (60_000, 58_611)] {
        let mit = kette(&paar(pensionaer_partner(euro)));
        assert_eq!(
            mit.gesamtbetrag_der_einkuenfte.get() - ohne.gesamtbetrag_der_einkuenfte.get(),
            einkuenfte,
            "{euro} Euro Versorgung des Ehegatten: Gesamtbetrag der Einkuenfte"
        );
        steuer.push(mit.festzusetzende_est.get());
    }
    assert_eq!(steuer[0], steuer[1], "0 Euro Versorgung aendern nichts");
    assert!(
        steuer[1] < steuer[2] && steuer[2] < steuer[3],
        "die Steuer steigt mit der Versorgung des Ehegatten: {steuer:?}"
    );
}

/// Der Versorgungsfreibetrag haengt am Beginnjahr und der Bemessungsgrundlage des EHEGATTEN: Beginn 2005 (Kohorte 2005:
/// 40,0 %, hoechstens 3.000, Zuschlag 900) gegen Beginn 2025 bei gleichem Bezug von 30.000 Euro. 2005: 30.000 − (3.000 + 900)
/// − 102 = 25.998; 2025: 28.611.
#[test]
fn der_versorgungsfreibetrag_folgt_beginnjahr_und_bemessung_des_ehegatten() {
    let ohne = gdb(&paar(vec![]));
    let beginn = |jahr: i64| {
        let mut p = pensionaer_partner(30_000);
        p.push(("versorgung_beginn_jahr_partner", json!(jahr)));
        gdb(&paar(p)) - ohne
    };
    assert_eq!(beginn(2025), 28_611, "Beginn 2025");
    assert_eq!(beginn(2005), 25_998, "Beginn 2005");
    // Eine kleine Bemessungsgrundlage (5.000 Euro) liegt unter dem Hoechstbetrag und senkt den Freibetrag:
    // 13,2 % von 5.000 = 660, Zuschlag 297 (hoechstens Bemessung minus Freibetrag, Satz 5: 4.340, greift nicht).
    let mut p = pensionaer_partner(30_000);
    p.push(("versorgung_bemessungsgrundlage_partner", json!(cent(5_000))));
    assert_eq!(gdb(&paar(p)) - ohne, 30_000 - (660 + 297) - 102, "Bemessung 5.000");
}

/// Alters-Gate (§ 19 Abs. 2 Satz 2 Nr. 2) des Ehegatten: Beginn mit 62 gilt als Arbeitslohn (ohne Freibetrag, mit Pauschbetrag
/// 1.230), Beginn mit 63 als Versorgungsbezug. Ab Grad der Behinderung 50 DES EHEGATTEN liegt die Grenze bei 60; der Grad von
/// Person A zaehlt nicht.
#[test]
fn das_alters_gate_gilt_fuer_den_ehegatten_mit_seinem_eigenen_grad() {
    let ohne = gdb(&paar(vec![]));
    let altersgrenze = |alter: i64, gdb_partner: i64, gdb_a: i64| {
        let mut p = pensionaer_partner(30_000);
        p.push(("versorgung_art_partner", json!("altersgrenze_sonstige")));
        p.push(("versorgung_alter_bei_beginn_partner", json!(alter)));
        p.push(("rentner_grad_der_behinderung_partner", json!(gdb_partner)));
        p.push(("rentner_grad_der_behinderung", json!(gdb_a)));
        gdb(&paar(p)) - ohne
    };
    assert_eq!(altersgrenze(62, 0, 0), 28_770, "62 Jahre: Arbeitslohn");
    assert_eq!(altersgrenze(63, 0, 0), 28_611, "63 Jahre: Versorgungsbezug");
    assert_eq!(altersgrenze(59, 50, 0), 28_770, "59 Jahre, Grad 50: Arbeitslohn");
    assert_eq!(altersgrenze(60, 50, 0), 28_611, "60 Jahre, Grad 50: Versorgungsbezug");
    assert_eq!(
        altersgrenze(60, 0, 50),
        28_770,
        "60 Jahre, Grad 50 nur bei Person A: der Grad des Ehegatten entscheidet, also Arbeitslohn"
    );
}

/// Lohn und Versorgung des Ehegatten zaehlen je einmal (Abweichung Nr. 50): sein Lohn von 50.000 Euro (Nr. 3) enthaelt den Bezug
/// von 30.000 Euro (Nr. 8). Arbeitslohn 20.000 (18.770) plus Versorgung (28.611). Vor Nr. 50 addierten sich Lohn 20.000 und Bezug.
#[test]
fn der_lohn_des_ehegatten_enthaelt_den_bezug_und_beide_zaehlen_je_einmal() {
    let ohne = gdb(&paar(vec![]));
    let mut p = lohn_partner(50_000);
    p.extend(versorgung_partner(30_000));
    assert_eq!(gdb(&paar(p)) - ohne, 18_770 + 28_611);
}

// ---------------------------------------------------------------- § 24a Altersentlastungsbetrag des Ehegatten (Nr. 36)

/// Der Gesamtbetrag der Einkuenfte des Falls mit dem Lohn des Ehegatten `lohn` Euro, abzueglich dem des selben Falls ohne
/// Geburtsjahr des Ehegatten: das ist der Altersentlastungsbetrag des Ehegatten in Euro.
fn altersentlastung_partner(lohn: i64, geburtsjahr: Option<i64>, veranlagung: &'static str) -> i64 {
    let ohne = gdb(&mit_veranlagung(veranlagung, lohn_partner(lohn)));
    let mut p = lohn_partner(lohn);
    if let Some(jahr) = geburtsjahr {
        p.push(("geburtsjahr_partner", json!(jahr)));
    }
    ohne - gdb(&mit_veranlagung(veranlagung, p))
}

/// § 24a (`estg_p24a_2026-07-13.txt`, Tabelle): geboren 1955 vollendet das 64. Lebensjahr 2019, das folgende Kalenderjahr ist
/// 2020: 16,0 % des Arbeitslohns, hoechstens 760 Euro. Bei 20.000 Euro Lohn des Ehegatten greift der Hoechstbetrag (3.200 > 760),
/// bei 2.000 Euro der Prozentsatz (320 < 760). Heute: 0, der Rentner-Ring rechnet § 24a nur fuer Person A.
#[test]
fn der_altersentlastungsbetrag_des_ehegatten_folgt_seinem_geburtsjahr_und_seinem_lohn() {
    assert_eq!(altersentlastung_partner(20_000, Some(1955), "zusammen"), 760, "Hoechstbetrag der Kohorte 2020");
    assert_eq!(altersentlastung_partner(2_000, Some(1955), "zusammen"), 320, "16,0 % von 2.000");
    // Kohorte 2025 (geboren 1960): 13,2 %, hoechstens 627 Euro. Geboren 1961 vollendet das 64. Lebensjahr erst 2025: kein Betrag.
    assert_eq!(altersentlastung_partner(20_000, Some(1960), "zusammen"), 627, "geboren 1960");
    assert_eq!(altersentlastung_partner(20_000, Some(1961), "zusammen"), 0, "geboren 1961");
}

/// Ohne Lohn des Ehegatten ist die Bemessung null (Satz 2: Leibrenten und Versorgungsbezuege bleiben ausser Betracht), und ohne
/// sein Geburtsjahr gibt es keinen Betrag. Das Geburtsjahr von Person A zaehlt fuer den Ehegatten nicht.
#[test]
fn der_altersentlastungsbetrag_des_ehegatten_braucht_seinen_lohn_und_sein_geburtsjahr() {
    assert_eq!(altersentlastung_partner(0, Some(1955), "zusammen"), 0, "ohne Lohn");
    assert_eq!(altersentlastung_partner(20_000, None, "zusammen"), 0, "ohne Geburtsjahr");
    let ohne = gdb(&paar(lohn_partner(20_000)));
    let mut nur_a = lohn_partner(20_000);
    nur_a.push(("geburtsjahr", json!(1955)));
    assert_eq!(ohne - gdb(&paar(nur_a)), 0, "Person A hat keinen Lohn: ihr Geburtsjahr gibt dem Ehegatten nichts");
}

/// Person A und der Ehegatte haben je ihren eigenen Altersentlastungsbetrag mit eigenem Lohn und eigener Kohorte: A geboren
/// 1955 (760 Euro), der Ehegatte geboren 1960 (627 Euro), beide mit 20.000 Euro Lohn: zusammen 1.387 Euro.
#[test]
fn der_altersentlastungsbetrag_beider_ehegatten_zaehlt_je_fuer_sich() {
    let lohn_a = || vec![("bruttoarbeitslohn", json!(cent(20_000)))];
    let ohne = gdb(&paar([lohn_a(), lohn_partner(20_000)].concat()));
    let mit = gdb(&paar(
        [lohn_a(), lohn_partner(20_000), vec![("geburtsjahr", json!(1955)), ("geburtsjahr_partner", json!(1960))]].concat(),
    ));
    assert_eq!(ohne - mit, 760 + 627);
}

/// Gewerbe-Gewinn von Person A (`euro` Euro, Messbetrag 0): die Felder, mit denen die Rentner-Scheibe einen laufenden Gewinn
/// nimmt (`kein_gewinn` auf nein).
fn gewinn_a(euro: i64) -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(cent(euro))),
        ("gewinn_betriebsart", json!("gewerbe")),
        ("gewst_hebesatz", json!(400)),
        ("gewst_messbetrag", json!(0)),
    ]
}

/// Veraeusserungsgewinn nach § 16 von Person A (`euro` Euro) mit beiden Gate-Antworten fuer den Freibetrag nach § 16 Abs. 4
/// (55 Jahre oder berufsunfaehig, erstmalig) auf ja, dazu `kein_gewinn` auf nein.
fn veraeusserungsgewinn_a(euro: i64) -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_veraeusserungsgewinn", json!(cent(euro))),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
    ]
}

/// Der Altersentlastungsbetrag von Person A in Euro: Gesamtbetrag der Einkuenfte ohne Geburtsjahr abzueglich dem mit
/// Geburtsjahr 1955, bei `lohn` Euro Arbeitslohn und den weiteren Angaben `weitere` (Einzelveranlagung).
fn altersentlastung_a(lohn: i64, weitere: &[(&'static str, Value)]) -> i64 {
    let basis = || {
        let mut p = vec![("bruttoarbeitslohn", json!(cent(lohn)))];
        p.extend(weitere.iter().cloned());
        mit_veranlagung("einzel", p)
    };
    let mut mit = basis();
    mit.push(("geburtsjahr", json!(1955)));
    gdb(&basis()) - gdb(&mit)
}

/// § 24a Satz 1: der Prozentsatz gilt fuer den Arbeitslohn UND fuer die positive Summe der Einkuenfte, die nicht aus nichtselb-
/// staendiger Arbeit sind (Gewinn aus Gewerbe und Veraeusserungsgewinn zaehlen, die Rente nicht, Satz 2 Nr. 2). Person A, geboren
/// 1955 (Kohorte 2020: 16,0 %, hoechstens 760 Euro), Rente 20.000 Euro im Hintergrund. Von Hand:
/// - Lohn 2.000 Euro, kein weiteres Einkommen: 16,0 % von 2.000 = 320.
/// - Lohn 2.000 Euro, Gewinn 1.000 Euro: 16,0 % von 3.000 = 480 (ueber 320, unter dem Hoechstbetrag).
/// - Lohn 0, Gewinn 3.000 Euro: 16,0 % von 3.000 = 480 (kein Lohn noetig).
/// - Lohn 2.000 Euro, Gewinn 20.000 Euro: 16,0 % von 22.000 = 3.520, gedeckelt auf 760.
/// - Lohn 2.000 Euro, Veraeusserungsgewinn 46.000 Euro: nach dem Freibetrag von 45.000 Euro (§ 16 Abs. 4, `estg_p16_2026-07-14.txt`;
///   unter der Schwelle von 136.000 Euro ungekuerzt) bleiben 1.000 Euro, also 16,0 % von 3.000 = 480.
///
/// Der Rentner-Zweig reicht diese Summe an `entlastungen` weiter (`max0(summe_euro(&[laufend, netto_vg, p23]))`); ein Aufruf mit
/// 0 gaebe in den Faellen 2 bis 5 jeweils 320 (mit Lohn) oder 0 (ohne Lohn), ohne den laufenden Gewinn oder ohne den
/// Veraeusserungsgewinn in der Summe fehlt der Zuwachs.
#[test]
fn der_altersentlastungsbetrag_von_person_a_rechnet_die_nicht_lohn_einkuenfte_in_die_bemessung() {
    assert_eq!(altersentlastung_a(2_000, &[]), 320, "Lohn 2.000, kein weiteres Einkommen");
    assert_eq!(altersentlastung_a(2_000, &gewinn_a(1_000)), 480, "Lohn 2.000, Gewinn 1.000");
    assert_eq!(altersentlastung_a(0, &gewinn_a(3_000)), 480, "kein Lohn, Gewinn 3.000");
    assert_eq!(altersentlastung_a(2_000, &gewinn_a(20_000)), 760, "Hoechstbetrag");
    assert_eq!(altersentlastung_a(2_000, &veraeusserungsgewinn_a(46_000)), 480, "Veraeusserungsgewinn 46.000 nach Freibetrag");
}

/// Ein Verkauf nach § 23 (`p23_*`) erreicht im Rentner-Ring nie eine Zahl, also gibt es keine Bemessung aus § 23 fuer § 24a:
/// mit `kein_p23_verkauf` auf nein sperrt `einkunftsart_nicht_ring_faehig` (`fremd_arten` der Scheibe), auf ja oder ohne Antwort
/// sperrt `flag_konsistenz_offen`. Das ist der Beleg, dass `p23` in `max0(summe_euro(&[laufend, netto_vg, p23]))` (`zweige/
/// rentner.rs`) kein eigener Test braucht: faellt eine der drei Sperren weg, wird dieser Test rot, und dann braucht `p23` einen.
#[test]
fn ein_verkauf_nach_p23_erreicht_im_rentner_ring_nie_eine_zahl() {
    let verkauf = |flag: Option<bool>| {
        let mut p = vec![
            ("p23_veraeusserungspreis", json!(cent(5_000))),
            ("p23_anschaffung_herstellungskosten", json!(cent(4_000))),
            ("p23_werbungskosten", json!(0)),
            ("p23_veraeusserungs_typ", json!("grundstueck")),
        ];
        if let Some(f) = flag {
            p.push(("kein_p23_verkauf", json!(f)));
        }
        mit_veranlagung("einzel", p)
    };
    // Gegenprobe: dieselbe Frage "kein Verkauf: ja" ohne Verkaufsdaten gibt eine Zahl, die Sperren oben kommen also von den Daten.
    let _ = kette(&mit_veranlagung("einzel", vec![("kein_p23_verkauf", json!(true))]));
    assert_eq!(gesperrt_mit(&verkauf(Some(false))), "einkunftsart_nicht_ring_faehig");
    assert_eq!(gesperrt_mit(&verkauf(Some(true))), "flag_konsistenz_offen");
    assert_eq!(gesperrt_mit(&verkauf(None)), "flag_konsistenz_offen");
}

/// Der Entlastungsbetrag fuer Alleinerziehende (§ 24b) in Euro: Gesamtbetrag der Einkuenfte der Einzelveranlagung ohne die
/// Angaben abzueglich dem mit `alleinstehend`, `kinder` Kindern und `monate` vollen Monaten ohne Voraussetzung.
fn entlastung_24b(alleinstehend: bool, kinder: i64, monate: i64) -> i64 {
    let ohne = gdb(&mit_veranlagung("einzel", vec![]));
    let mit = gdb(&mit_veranlagung(
        "einzel",
        vec![
            ("fam_alleinstehend", json!(alleinstehend)),
            ("fam_anzahl_kinder", json!(kinder)),
            ("fam_monate_ohne_voraussetzung", json!(monate)),
        ],
    ));
    ohne - mit
}

/// § 24b (`estg_p24b_2026-07-09.txt`): Absatz 2: 4.260 Euro fuer das erste Kind, 240 Euro je weiteres Kind; Absatz 4: je voller
/// Monat ohne die Voraussetzung ein Zwoelftel weniger. Von Hand: 1 Kind 4.260; 2 Kinder 4.500; 1 Kind und 6 Monate ohne
/// Voraussetzung 4.260 − 6 × 355 = 2.130. Wer nicht allein steht, bekommt nichts. Der Rentner-Zweig reicht den Betrag aus
/// `entlastungen` an den Gesamtfall weiter (`entlastungsbetrag_alleinerziehende`); ohne ihn bliebe die Rentnerin mit Kind bei 0.
#[test]
fn der_entlastungsbetrag_fuer_alleinerziehende_mindert_den_gesamtbetrag_der_rentnerin() {
    assert_eq!(entlastung_24b(true, 1, 0), 4_260, "ein Kind");
    assert_eq!(entlastung_24b(true, 2, 0), 4_500, "zwei Kinder");
    assert_eq!(entlastung_24b(true, 1, 6), 2_130, "ein Kind, sechs Monate ohne Voraussetzung");
    assert_eq!(entlastung_24b(false, 1, 0), 0, "nicht allein stehend");
    assert_eq!(entlastung_24b(true, 0, 0), 0, "kein Kind");
    // Die Steuer sinkt: dasselbe Kind, einmal allein stehend und einmal nicht (der Kinderfreibetrag haengt nur am Kind).
    let steuer = |alleinstehend: bool| {
        kette(&mit_veranlagung(
            "einzel",
            vec![("fam_alleinstehend", json!(alleinstehend)), ("fam_anzahl_kinder", json!(1))],
        ))
        .festzusetzende_est
        .get()
    };
    assert!(steuer(true) < steuer(false), "{} < {}", steuer(true), steuer(false));
}

/// Bei Einzelveranlagung zaehlt der Ehegatte nicht, also auch sein Altersentlastungsbetrag nicht.
#[test]
fn bei_einzelveranlagung_gibt_es_keinen_altersentlastungsbetrag_fuer_den_ehegatten() {
    assert_eq!(altersentlastung_partner(20_000, Some(1955), "einzel"), 0);
}

/// Person A und der Ehegatte haben je eine eigene Versorgung: beide zusammen bringen 28.611 + 28.611.
#[test]
fn die_versorgung_beider_ehegatten_zaehlt_je_fuer_sich() {
    let ohne = gdb(&paar(vec![]));
    let mut p = pensionaer_partner(30_000);
    p.extend([
        ("bruttoarbeitslohn", json!(cent(30_000))),
        ("versorgung_jahresrente", json!(cent(30_000))),
        ("versorgung_bemessungsgrundlage", json!(cent(30_000))),
        ("versorgung_beginn_jahr", json!(2025)),
        ("versorgung_art", json!("beamtenrechtlich")),
    ]);
    assert_eq!(gdb(&paar(p)) - ohne, 2 * 28_611);
}

// ---------------------------------------------------------------- Nur bei Zusammenveranlagung

/// Bei Einzelveranlagung zaehlen die Angaben zum Ehegatten nicht: Lohn und Versorgung im Store aendern den Gesamtbetrag
/// nicht (der Ehegatte steht dann nicht in der Erklaerung).
#[test]
fn bei_einzelveranlagung_zaehlt_der_ehegatte_nicht() {
    let einzel = |mehr: Paare| mit_veranlagung("einzel", mehr);
    let ohne = kette(&einzel(vec![]));
    let mut p = lohn_partner(60_000);
    p.extend(versorgung_partner(30_000));
    let mit = kette(&einzel(p));
    assert_eq!(mit.gesamtbetrag_der_einkuenfte.get(), ohne.gesamtbetrag_der_einkuenfte.get());
    assert_eq!(mit.festzusetzende_est.get(), ohne.festzusetzende_est.get());
}

// ---------------------------------------------------------------- Sperre und Abgabefaehigkeit

/// AK4: Zusammenveranlagung ohne jede Angabe zum Ehegatten sperrt nicht. Es gibt eine Zahl, und sie ist dieselbe wie mit
/// bestaetigten Nullen (Lohn 0, Versorgung 0): eine fehlende Antwort rechnet wie heute 0.
#[test]
fn zusammenveranlagung_ohne_angaben_zum_ehegatten_bleibt_abgabefaehig() {
    let ohne = kette(&paar(vec![]));
    let mut nullen = lohn_partner(0);
    nullen.extend(versorgung_partner(0));
    let mit_nullen = kette(&paar(nullen));
    assert_eq!(ohne.festzusetzende_est.get(), mit_nullen.festzusetzende_est.get());
    assert_eq!(ohne.gesamtbetrag_der_einkuenfte.get(), mit_nullen.gesamtbetrag_der_einkuenfte.get());
}

/// Ein Versorgungsbezug des Ehegatten ohne Beginnjahr oder ohne Bemessungsgrundlage sperrt wie bei Person A
/// (`versorgungsfreibetrag_offen`): der Ring rechnete ihn sonst mit einem Freibetrag von 0 Euro oder gar nicht. Es entsteht
/// kein neuer Sperrgrund. Bei Einzelveranlagung zaehlt die Angabe nicht, also sperrt sie auch nicht.
#[test]
fn ein_versorgungsbezug_des_ehegatten_ohne_beginn_oder_bemessungsgrundlage_sperrt() {
    let nur_bezug = vec![("versorgung_jahresrente_partner", json!(cent(30_000)))];
    assert_eq!(gesperrt_mit(&paar(nur_bezug.clone())), "versorgungsfreibetrag_offen");
    let ohne_bemessung = vec![
        ("versorgung_jahresrente_partner", json!(cent(30_000))),
        ("versorgung_beginn_jahr_partner", json!(2025)),
    ];
    assert_eq!(gesperrt_mit(&paar(ohne_bemessung)), "versorgungsfreibetrag_offen");
    let ohne_beginn = vec![
        ("versorgung_jahresrente_partner", json!(cent(30_000))),
        ("versorgung_bemessungsgrundlage_partner", json!(cent(30_000))),
    ];
    assert_eq!(gesperrt_mit(&paar(ohne_beginn)), "versorgungsfreibetrag_offen");
    // Einzelveranlagung: die Angaben zum Ehegatten stehen nicht in der Erklaerung.
    let _ = kette(&mit_veranlagung("einzel", nur_bezug));
}

/// Die Sperre von Person A bleibt davon unberuehrt: A mit unvollstaendiger Versorgung sperrt weiter, auch wenn die
/// Versorgung des Ehegatten vollstaendig ist.
#[test]
fn die_sperre_von_person_a_gilt_weiter_neben_einem_vollstaendigen_ehegatten() {
    let mut p = versorgung_partner(30_000);
    p.push(("versorgung_jahresrente", json!(cent(30_000))));
    assert_eq!(gesperrt_mit(&paar(p)), "versorgungsfreibetrag_offen");
}
