//! § 24a `EStG`: der private Verkauf (§ 23) und die sonstigen Leistungen (§ 22 Nr. 3) gehoeren zur Bemessung des
//! Altersentlastungsbetrags von Person A (Abweichung Nr. 63, Bezug Nr. 62 und Nr. 54). Beide Zweige (`festzusetzende_est_gesamt`
//! und `festzusetzende_est_rentner`), im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** § 24a Satz 2 nimmt aus der Bemessung nur Versorgungsbezuege, Leibrenten (§ 22 Nr. 1 Satz 3 Buchst. a), § 22
//! Nr. 4 Satz 4 Buchst. b und zwei Faelle des § 22 Nr. 5 heraus. § 23 und § 22 Nr. 3 zaehlen also mit. Der Gesamt-Zweig bildete die
//! Bemessung aus einem Wert (`g.einkuenfte_sonstige`), der an dieser Stelle noch 0 war; der Rentner-Zweig nahm § 23 mit, aber nicht
//! § 22 Nr. 3. Python hat dieselbe Luecke (`bescheid_zweige.py` Zeile 671 liest den Wert, Zeile 770 setzt ihn erst danach).
//!
//! **Warum es zaehlt.** Wer 65 ist und 1.000 Euro aus einem privaten Verkauf hat, bekam bis zu 160 Euro Freibetrag zu wenig,
//! bei hohen Gewinnen bis zum Hoechstbetrag (VZ 2025: 760 Euro).
//!
//! METHODE. Gemessen wird der Altersentlastungsbetrag von Person A als Unterschied des Gesamtbetrags der Einkuenfte: derselbe Fall
//! ohne und mit Geburtsjahr 1955. Die Erwartung kommt von Hand aus dem Gesetz, nie aus dem Rust-Code:
//! - Kohorte (§ 24a, `estg_p24a_2026-07-13.txt`, Tabelle): das auf die Vollendung des 64. Lebensjahres folgende Kalenderjahr
//!   2020 (geboren 1955) gibt 16,0 %, Hoechstbetrag 760 Euro.
//! - Bemessung = Arbeitslohn (brutto) + positive Summe der uebrigen Einkuenfte, ohne Leibrenten (Satz 2 Nr. 2).
//! - Privater Verkauf (§ 23 Abs. 3 Satz 5): ein Gewinn unter 1.000 Euro bleibt steuerfrei (Freigrenze), ab 1.000 Euro zaehlt er
//!   voll; nach mehr als zehn Jahren (Grundstueck) ist der Verkauf steuerfrei.
//! - Sonstige Leistungen (§ 22 Nr. 3 Satz 2): unter 256 Euro steuerfrei (Freigrenze), ab 256 Euro voll.
//! - Nr. 5 (Zeile 15 der Anlage R-AV/bAV): Ertragsanteil nach Alter bei Rentenbeginn, 65 = 18 %; gehoert zu den Leibrenten und
//!   zaehlt nicht in die Bemessung.
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
const BEIDE: [Scheibe; 2] = [Scheibe::RentnerGesamt, Scheibe::Gesamt];

type Paare = Vec<(String, Value)>;

fn p(feld: &str, wert: Value) -> (String, Value) {
    (feld.to_owned(), wert)
}

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

/// Ein Fall der Scheibe, mit der Scheiben-Bindung als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

fn fall(scheibe: Scheibe, paare: &[(String, Value)]) -> Fall {
    let cfg = Cfg::fuer(scheibe);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let alle = kegel_fuer(&cfg, paare);
    let events: Vec<(&str, Value, bool)> = alle
        .iter()
        .map(|(f, w)| (f.as_str(), w.clone(), true))
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

/// Der Gesamtbetrag der Einkuenfte in Euro; ein Fall ohne Zahl ist ein Fehlschlag des Tests.
fn gdb(scheibe: Scheibe, paare: &[(String, Value)]) -> i64 {
    let f = fall(scheibe, paare);
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, VZ, &kegel, &umg, Some(&f.store), None) {
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

/// Die Akte: Einzelveranlagung, geboren am 05.05.1955, Lohn `lohn` Euro; bei `rentner_gesamt` ohne Anlage-R-Rente (Jahresbetrag 0).
/// Das Kreuz `kein_sonstige` steht auf nein: die sonstigen Einkuenfte (Nr. 3) sind in Kraft.
fn akte(scheibe: Scheibe, lohn: i64) -> Paare {
    let mut a = vec![
        p("veranlagung", json!("einzel")),
        p("stammdaten_geburtsdatum", json!("05.05.1955")),
        p("bruttoarbeitslohn", json!(cent(lohn))),
        p("kein_sonstige", json!(false)),
    ];
    if scheibe == Scheibe::RentnerGesamt {
        a.extend([
            p("rentner_renten_art", json!("gesetzliche_rente")),
            p("rentner_jahresrente", json!(0)),
            p("rentner_renten_beginn_jahr", json!(2025)),
            p("rentner_alter_bei_rentenbeginn", json!(65)),
            p("rentner_rentenfreibetrag", json!(0)),
            p("kein_gewinn", json!(true)),
            p("kein_kap", json!(true)),
            p("kein_vuv", json!(true)),
            p("agb_zwangslaeufig", json!(true)),
            p("agb_notwendig_angemessen", json!(true)),
        ]);
    }
    a
}

/// Der Altersentlastungsbetrag von Person A in Euro: Gesamtbetrag der Einkuenfte ohne Geburtsjahr minus mit Geburtsjahr 1955.
fn altersentlastung(scheibe: Scheibe, lohn: i64, mehr: &[(String, Value)]) -> i64 {
    let ohne = [akte(scheibe, lohn), mehr.to_vec()].concat();
    let mit = [ohne.clone(), vec![p("geburtsjahr", json!(1955))]].concat();
    gdb(scheibe, &ohne) - gdb(scheibe, &mit)
}

/// Ein vollstaendiger privater Verkauf nach § 23 von Person A: Grundstueck, gekauft `kauf`, verkauft `verkauf` (TT.MM.JJJJ, Verkauf
/// 2025), Preis und Anschaffungskosten in Cent, das Kreuz `kein_p23_verkauf` auf nein.
fn privatverkauf(kauf: &str, verkauf: &str, preis: i64, kosten: i64) -> Paare {
    vec![
        p("kein_p23_verkauf", json!(false)),
        p("p23_veraeusserungs_typ", json!("grundstueck")),
        p("p23_bezeichnung", json!("Testobjekt")),
        p("p23_anschaffungsdatum", json!(kauf)),
        p("p23_veraeusserungsdatum", json!(verkauf)),
        p("p23_veraeusserungspreis", json!(preis)),
        p("p23_anschaffung_herstellungskosten", json!(kosten)),
        p("p23_werbungskosten", json!(0)),
        p("p23_afa", json!(0)),
        p("p23_eigennutzung", json!("nein")),
        p("p23_gebaeude_in_frist", json!(false)),
    ]
}

/// Ein steuerpflichtiger Verkauf mit dem Gewinn `gewinn_cent` (Kauf 2020, Verkauf 2025, innerhalb der Frist von zehn Jahren).
fn verkauf_mit_gewinn(gewinn_cent: i64) -> Paare {
    privatverkauf("15.06.2020", "10.03.2025", cent(10_000) + gewinn_cent, cent(10_000))
}

/// § 24a Satz 1 und 2: der steuerpflichtige private Verkauf gehoert zur Bemessung. Person A, geboren 1955 (16,0 %, hoechstens 760
/// Euro). Von Hand:
/// - Lohn 2.000, Gewinn 1.000,00: 16,0 % von 3.000 = 480 (ohne den Verkauf 320).
/// - Lohn 2.000, Gewinn 999,99: unter der Freigrenze, der Gewinn zaehlt nicht: 16,0 % von 2.000 = 320.
/// - Lohn 0, Gewinn 3.000: 16,0 % von 3.000 = 480 (kein Lohn noetig).
/// - Lohn 2.000, Gewinn 3.000, aber nach mehr als zehn Jahren verkauft (steuerfrei): 320.
/// - Lohn 2.000, Gewinn 20.000: 16,0 % von 22.000 = 3.520, der Hoechstbetrag greift: 760.
#[test]
fn der_steuerpflichtige_privatverkauf_gehoert_zur_bemessung() {
    for s in BEIDE {
        assert_eq!(
            altersentlastung(s, 2_000, &verkauf_mit_gewinn(cent(1_000))),
            480,
            "{s}: Lohn 2.000, Gewinn 1.000,00"
        );
        assert_eq!(
            altersentlastung(s, 2_000, &verkauf_mit_gewinn(cent(1_000) - 1)),
            320,
            "{s}: Lohn 2.000, Gewinn 999,99 unter der Freigrenze"
        );
        assert_eq!(
            altersentlastung(s, 0, &verkauf_mit_gewinn(cent(3_000))),
            480,
            "{s}: kein Lohn, Gewinn 3.000"
        );
        let frei = privatverkauf("15.06.2014", "10.03.2025", cent(13_000), cent(10_000));
        assert_eq!(
            altersentlastung(s, 2_000, &frei),
            320,
            "{s}: Lohn 2.000, Gewinn 3.000 steuerfrei nach der Frist"
        );
        assert_eq!(
            altersentlastung(s, 2_000, &verkauf_mit_gewinn(cent(20_000))),
            760,
            "{s}: Lohn 2.000, Gewinn 20.000, Hoechstbetrag"
        );
    }
}

/// § 24a Satz 1 und 2: die sonstigen Leistungen nach § 22 Nr. 3 gehoeren zur Bemessung (die Nr. 3 steht nicht in Satz 2). Von Hand:
/// - Lohn 2.000, Nr. 3 300 Euro (ab der Freigrenze von 256 Euro voll): 16,0 % von 2.300 = 368.
/// - Lohn 2.000, Nr. 3 255,99 Euro: unter der Freigrenze, nichts: 320.
/// - Lohn 0, Nr. 3 300 Euro: 16,0 % von 300 = 48.
#[test]
fn die_sonstigen_leistungen_nr3_gehoeren_zur_bemessung() {
    let nr3 = |cent_wert: i64| vec![p("p22_nr3_einkuenfte", json!(cent_wert))];
    for s in BEIDE {
        assert_eq!(
            altersentlastung(s, 2_000, &nr3(cent(300))),
            368,
            "{s}: Lohn 2.000, Nr. 3 300 Euro"
        );
        assert_eq!(
            altersentlastung(s, 2_000, &nr3(cent(256) - 1)),
            320,
            "{s}: Lohn 2.000, Nr. 3 255,99 Euro unter der Freigrenze"
        );
        assert_eq!(
            altersentlastung(s, 0, &nr3(cent(300))),
            48,
            "{s}: kein Lohn, Nr. 3 300 Euro"
        );
    }
}

/// Kontrolle der Verdrahtung: beide Posten erhoehen den Gesamtbetrag der Einkuenfte selbst (ohne Geburtsjahr, also ohne § 24a), der
/// Verkauf um seinen Gewinn von 1.000 Euro, die Nr. 3 um 300 Euro. Die Werte, die in die Bemessung gehen, sind dieselben, die in die
/// sonstigen Einkuenfte gehen; fehlt einer dort, ist er hier rot, auch wenn die Bemessung stimmt.
#[test]
fn der_verkauf_und_die_nr3_erhoehen_den_gesamtbetrag_der_einkuenfte() {
    for s in BEIDE {
        let ohne = gdb(s, &akte(s, 0));
        let mit_verkauf = gdb(s, &[akte(s, 0), verkauf_mit_gewinn(cent(1_000))].concat());
        assert_eq!(mit_verkauf - ohne, 1_000, "{s}: Verkauf mit Gewinn 1.000 Euro");
        let mit_nr3 = gdb(s, &[akte(s, 0), vec![p("p22_nr3_einkuenfte", json!(cent(300)))]].concat());
        assert_eq!(mit_nr3 - ohne, 300, "{s}: Nr. 3 mit 300 Euro");
    }
}

/// Die Leibrente bleibt aus der Bemessung (§ 24a Satz 2 Nr. 2): eine Mitteilung mit Nr. 5 (10.000 Euro, Beginn mit 65 = 18 %) bringt
/// 1.800 − 102 = 1.698 Euro Einkuenfte, die in die Bemessung NICHT eingehen. Mit einem Verkauf von 1.000 Euro Gewinn bleibt der
/// Betrag bei 16,0 % von 1.000 = 160. Der Kirchensteuer-Ueberhang (500 Euro erstattet, nichts gezahlt, § 10 Abs. 4b Satz 3) erhoeht
/// den Gesamtbetrag der Einkuenfte, ist aber keine Einkunft der Bemessung: ebenfalls 160.
#[test]
fn leibrente_und_kist_ueberhang_gehen_nicht_in_die_bemessung() {
    let mitteilung = vec![
        p("kein_rav_bav", json!(false)),
        p("rav_bav_nr1_betrag", json!(0)),
        p("rav_bav_nr4_betrag", json!(0)),
        p("rav_bav_nr4_beginn", json!("01.04.2022")),
        p("rav_bav_nr5_betrag", json!(cent(10_000))),
        p("rav_bav_nr5_beginn", json!("05.05.2020")),
        p("rav_bav_andere_nummern", json!(false)),
        p("rav_bav_basisrente", json!(false)),
    ];
    let ueberhang = vec![p("kist_erstattet", json!(cent(500))), p("kist_gezahlt", json!(0))];
    for s in BEIDE {
        let verkauf = verkauf_mit_gewinn(cent(1_000));
        assert_eq!(
            altersentlastung(s, 0, &verkauf),
            160,
            "{s}: nur der Verkauf (Kontrolle)"
        );
        assert_eq!(
            altersentlastung(s, 0, &[verkauf.clone(), mitteilung.clone()].concat()),
            160,
            "{s}: Verkauf und Leibrente Nr. 5"
        );
        assert_eq!(
            altersentlastung(s, 0, &[verkauf, ueberhang.clone()].concat()),
            160,
            "{s}: Verkauf und KiSt-Ueberhang"
        );
    }
}
