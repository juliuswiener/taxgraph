//! § 34 Abs. 3 `EStG` fuer den Ehegatten (B Option 1, Julius 2026-10-06, „B Option 1 bauen“): hat NUR der Partner einen
//! Veraeusserungsgewinn und beantragt er den ermaessigten Steuersatz, rechnet die Software ihn auf SEINEN Gewinn. Haben
//! BEIDE einen Gewinn, bleibt die Berechnung gesperrt. Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Wer seinen Betrieb verkauft, kann einmal im Leben einen ermaessigten Steuersatz beantragen, ab 55
//! Jahren oder bei dauernder Berufsunfaehigkeit. Die Software kannte den Antrag nur fuer Person A. Hatte der Ehegatte
//! den Gewinn, blieb ihm nur die Fuenftelregel, auch wenn er berechtigt war und den Antrag wollte.
//!
//! **Warum es zaehlt.** Der Partner zahlt ohne die Moeglichkeit zu viel Steuer. Bei 500.000 Euro Gewinn und
//! Zusammenveranlagung sind es unten 76.242 Euro (Fuenftelregel 191.188 gegen 114.946 Euro). Es ging kein falscher Wert
//! raus, es fehlte eine Moeglichkeit.
//!
//! **Wo es sitzt.** Die Berechtigung (`abzuege::abs3_eligible_partner`), der Chooser (`zweige/tarif.rs::p34_chooser`),
//! die Sperren (`deklaration/sperre.rs`) und die Antragszeile (`deklaration/ring_werte.rs::p34_antrag`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus `params/2025/einkommensteuertarif_p32a.yaml` und dem Wortlaut
//! von § 34 Abs. 3 (`sources/gesetze-im-internet/estg_p34_2026-07-13.txt`: 56 Prozent des durchschnittlichen Steuersatzes,
//! mindestens 14 Prozent, auf den Teil des Gewinns bis 5 Millionen Euro; auf das verbleibende zu versteuernde Einkommen
//! die allgemeinen Tarifvorschriften). Kein Wert ist aus dem Rust-Code abgelesen. Geeicht ist die Rechenvorschrift an zwei
//! Zahlen aus dem Python-Server (`kette_endstand_hermetisch.rs`): `g6` (Einzel, 5 Mio Euro, Abs. 3, 1.263.270 Euro) und
//! `g22` (Zusammen, Fuenftelung, 34.594 Euro); beide trifft die Handrechnung auf den Euro.
//!
//! Der Fall: Zusammenveranlagung, Person A mit 60.000 Euro Arbeitslohn (Gesamtbetrag 58.770 Euro nach dem
//! Arbeitnehmer-Pauschbetrag 1.230 Euro), der Partner mit einem Veraeusserungsgewinn `X` ueber 181.000 Euro (dort ist der
//! Freibetrag nach § 16 Abs. 4 null), Sonderausgaben-Pauschbetrag 72 Euro: zvE = 58.698 + `X`. Splitting: das Doppelte der
//! Steuer auf die Haelfte.
//!
//! | `X` (Euro)  | zvE         | Fuenftelregel (Abs. 1) | Abs. 3: Steuer auf zvE - `X` | 56 % des Durchschnittssatzes | Steuer auf `X` | Abs. 3 gesamt |
//! |-------------|-------------|------------------------|------------------------------|------------------------------|----------------|---------------|
//! | 200.000     | 258.698     | 71.658                 | 8.238                        | 18,796 %                     | 37.591         | 45.829        |
//! | 500.000     | 558.698     | 191.188                | 8.238                        | 21,342 %                     | 106.708        | 114.946       |
//! | 5.000.000   | 5.058.698   | 2.156.648              | 8.238                        | 24,774 %                     | 1.238.693      | 1.246.931     |
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
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

const GRUND_BEIDE_A: &str = "abs3_partner_gewinn_offen";
const GRUND_BEIDE_B: &str = "abs3_partner_antrag_gewinn_offen";
const GRUND_UEBER_5MIO: &str = "abs3_partner_antrag_ueber_5mio_offen";
const GRUND_BU_OFFEN: &str = "berufsunfaehigkeit_partner_offen";

/// Euro in Cent.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

type Paare = Vec<(&'static str, Value)>;
type Ereignisse = Vec<(&'static str, Value, bool)>;

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

fn fall(scheibe: Scheibe, basis: &[(&'static str, Value)], roh: &Ereignisse) -> Fall {
    let cfg = Cfg::fuer(scheibe);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let mut events: Vec<(&str, Value, bool)> = kegel_fuer(&cfg, basis)
        .into_iter()
        .map(|(f, w)| (f, w, true))
        .collect();
    events.extend(roh.iter().cloned());
    let store = store(&events);
    Fall {
        cfg,
        index: teil.iter().map(|b| (b.feld_id.clone(), *b)).collect(),
        achsen: teil.iter().map(|b| AchsenBindung::from(*b)).collect(),
        felder: felder(&store),
        store,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Ausgang {
    Gesperrt(String),
    Zahl(i64, Box<Option<Kette>>),
    Anders(String),
}

/// Der Ausgang von `_ergebnis_roh` ohne HTTP: erst der K2-Guard, dann `feste_zahl` (nur bestaetigte Werte).
fn ergebnis(f: &Fall) -> Ausgang {
    if f.cfg.guard() {
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
    }
    ohne_sperre(f)
}

/// Der Ausgang ohne den K2-Guard, nur `feste_zahl`: so erreicht ein Test den Chooser (`zweige/tarif.rs::p34_chooser`)
/// auch dort, wo im Produkt die Sperren davor greifen. Der Chooser ist die zweite Linie hinter ihnen.
fn ohne_sperre(f: &Fall) -> Ausgang {
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, VZ, &kegel, &umg, Some(&f.store), None) {
        Ok(Ok(z)) => Ausgang::Zahl(z.zahl.get(), Box::new(z.extras.kette)),
        Ok(Err(k)) => Ausgang::Anders(format!("ohne Zahl: {:?}", k.grund)),
        Err(e) => Ausgang::Anders(format!("{e:?}")),
    }
}

/// Die Kette eines Falls mit Zahl; jeder andere Ausgang ist ein Fehlschlag des Tests.
fn kette(a: Ausgang) -> Kette {
    match a {
        Ausgang::Zahl(_, k) => (*k).expect("KONTROLLE: Kette fehlt"),
        andere => panic!("KONTROLLE: erwartet eine Zahl, bekommen {andere:?}"),
    }
}

/// Die vier Stufen der Kette in Euro: Gesamtbetrag, zvE, tarifliche und festzusetzende `ESt`.
fn stufen(k: &Kette) -> [i64; 4] {
    [
        k.gesamtbetrag_der_einkuenfte.get(),
        k.zu_versteuerndes_einkommen.get(),
        k.tarifliche_est.get(),
        k.festzusetzende_est.get(),
    ]
}

fn gesperrt(g: &str) -> Ausgang {
    Ausgang::Gesperrt(g.to_owned())
}

/// Zusammenveranlagung, A mit 60.000 Euro Lohn, der Partner mit `x_euro` Veraeusserungsgewinn (Freibetrag § 16 Abs. 4
/// erfuellt, ab 181.000 Euro null). Der Partner-Antrag samt Berechtigung (geboren 1960) steht NICHT drin; der Aufrufer
/// setzt ihn mit [`mit_antrag`].
fn partner_gewinn(x_euro: i64) -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("kein_gewinn", json!(false)),
        ("rentner_veraeusserungsgewinn_partner", json!(cent(x_euro))),
        ("rentner_veraeusserungs_betriebsart_partner", json!("gewerbe")),
        ("rentner_alter_55_oder_berufsunfaehig_partner", json!(true)),
        ("rentner_freibetrag_erstmalig_partner", json!(true)),
    ]
}

/// Die Antrags-Angaben des Partners: Antrag gestellt, geboren 1960 (65 Jahre), nicht berufsunfaehig, nie zuvor genutzt.
fn mit_antrag(mut basis: Paare) -> Paare {
    basis.extend([
        ("antrag_ermaessigter_satz_partner", json!(true)),
        ("geburtsjahr_partner", json!(1960)),
        ("dauernd_berufsunfaehig_partner", json!(false)),
        ("ermaessigung_einmal_genutzt_partner", json!(false)),
    ]);
    basis
}

/// Ersetzt (oder ergaenzt) Felder einer Paar-Liste.
fn mit(mut basis: Paare, aenderungen: Paare) -> Paare {
    for (f, w) in aenderungen {
        match basis.iter_mut().find(|(g, _)| *g == f) {
            Some(p) => p.1 = w,
            None => basis.push((f, w)),
        }
    }
    basis
}

fn gesamt(basis: &[(&'static str, Value)]) -> Ausgang {
    ergebnis(&fall(Scheibe::Gesamt, basis, &Vec::new()))
}

// ---------------------------------------------------------------- Rechnung (AK2)

/// Hat nur der Partner den Gewinn und beantragt er den Satz, gilt Abs. 3: die Steuer auf zvE - `X` plus 56 Prozent des
/// Durchschnittssatzes auf `X`. Gerundet wird der Teil auf `X` nach unten (Eichung `g6`). Drei Gewinne: 200.000,
/// 500.000 und genau 5.000.000 Euro (die Grenze gehoert dazu).
#[test]
fn partner_mit_antrag_rechnet_den_ermaessigten_satz_bei_200k_500k_5mio() {
    // (X in Euro, zvE, Abs. 3 gesamt)
    let faelle = [
        (200_000, 258_698, 45_829),
        (500_000, 558_698, 114_946),
        (5_000_000, 5_058_698, 1_246_931),
    ];
    for (x, zve, soll) in faelle {
        let k = kette(gesamt(&mit_antrag(partner_gewinn(x))));
        assert_eq!(
            stufen(&k),
            [zve + 72, zve, soll, soll],
            "Abs. 3, Partner-Gewinn {x}"
        );
    }
}

/// Gegenprobe im selben Fall: ohne Antrag gilt die Fuenftelregel (Abs. 1), von Amts wegen, fuer den Gewinn des Partners.
/// Das ist der Stand VOR dem Bau und bleibt so, solange niemand den Antrag stellt.
#[test]
fn ohne_antrag_gilt_die_fuenftelregel_fuer_den_partner_gewinn() {
    let faelle = [
        (200_000, 258_698, 71_658),
        (500_000, 558_698, 191_188),
        (5_000_000, 5_058_698, 2_156_648),
    ];
    for (x, zve, soll) in faelle {
        let k = kette(gesamt(&partner_gewinn(x)));
        assert_eq!(stufen(&k), [zve + 72, zve, soll, soll], "Fuenftelung {x}");
    }
}

/// Der Antrag allein genuegt nicht (§ 34 Abs. 3 Satz 1: 55. Lebensjahr vollendet ODER dauernd berufsunfaehig; Satz 4:
/// nur einmal im Leben). Fehlt die Berechtigung, bleibt es bei der Fuenftelregel, ohne Sperre, wenn die Berufsunfaehigkeit
/// beantwortet ist.
#[test]
fn antrag_ohne_berechtigung_bleibt_fuenftelregel() {
    let fuenftel = [
        stufen(&kette(gesamt(&partner_gewinn(500_000)))),
        // geboren 1971: im Jahr 2025 erst 54 Jahre alt
        stufen(&kette(gesamt(&mit(
            mit_antrag(partner_gewinn(500_000)),
            vec![("geburtsjahr_partner", json!(1971))],
        )))),
        // berechtigt, aber der Satz ist schon einmal in Anspruch genommen
        stufen(&kette(gesamt(&mit(
            mit_antrag(partner_gewinn(500_000)),
            vec![("ermaessigung_einmal_genutzt_partner", json!(true))],
        )))),
    ];
    assert_eq!(fuenftel[0], [558_770, 558_698, 191_188, 191_188]);
    assert_eq!(fuenftel[1], fuenftel[0], "zu jung, nicht berufsunfaehig");
    assert_eq!(fuenftel[2], fuenftel[0], "schon einmal genutzt");
}

/// Die Altersgrenze: wer im Veranlagungsjahr 55 wird (geboren 1970), ist berechtigt; ein Jahr juenger (1971) nicht.
/// Dauernde Berufsunfaehigkeit ersetzt das Alter.
#[test]
fn die_berechtigung_hat_die_grenze_bei_55_und_die_berufsunfaehigkeit_ersetzt_das_alter() {
    let abs3 = [558_770, 558_698, 114_946, 114_946];
    let nur = |gj: i64, bu: bool| {
        gesamt(&mit(
            mit_antrag(partner_gewinn(500_000)),
            vec![
                ("geburtsjahr_partner", json!(gj)),
                ("dauernd_berufsunfaehig_partner", json!(bu)),
            ],
        ))
    };
    assert_eq!(stufen(&kette(nur(1970, false))), abs3, "55 Jahre");
    assert_eq!(stufen(&kette(nur(1971, false))), [558_770, 558_698, 191_188, 191_188], "54 Jahre");
    assert_eq!(stufen(&kette(nur(1990, true))), abs3, "jung, aber dauernd berufsunfaehig");
}

/// Der Antrag von A gilt nicht fuer den Gewinn des Partners (`g22` aus der Kette: A berechtigt mit Antrag, aber ohne
/// eigenen Gewinn; der Partner ohne Antrag): es bleibt bei der Fuenftelregel, ohne Sperre. Und umgekehrt: Antrag nur beim
/// Partner, A berechtigt, aber ohne Gewinn: Abs. 3 fuer den Partner.
#[test]
fn der_antrag_gilt_je_person_fuer_den_eigenen_gewinn() {
    let a_antrag = vec![
        ("antrag_ermaessigter_satz", json!(true)),
        ("geburtsjahr", json!(1970)),
        ("dauernd_berufsunfaehig", json!(false)),
        ("ermaessigung_einmal_genutzt", json!(false)),
    ];
    // nur A beantragt: der Partner-Gewinn bleibt in der Fuenftelregel
    let k = kette(gesamt(&mit(partner_gewinn(500_000), a_antrag.clone())));
    assert_eq!(stufen(&k), [558_770, 558_698, 191_188, 191_188]);
    // beide beantragen, nur der Partner hat einen Gewinn: Abs. 3 fuer den Partner
    let k = kette(gesamt(&mit(mit_antrag(partner_gewinn(500_000)), a_antrag)));
    assert_eq!(stufen(&k), [558_770, 558_698, 114_946, 114_946]);
}

// ---------------------------------------------------------------- Chooser ohne Sperre (zweite Linie)

fn gesamt_ohne_sperre(basis: &[(&'static str, Value)]) -> Ausgang {
    ohne_sperre(&fall(Scheibe::Gesamt, basis, &Vec::new()))
}

/// Ueber 5 Millionen Euro nimmt der Chooser den ermaessigten Satz fuer den Partner nicht, auch wenn keine Sperre davor
/// steht: die Grenze steht in `p34_chooser` selbst. Im Produkt sperrt `abs3_partner_antrag_ueber_5mio_offen` davor
/// (`ueber_fuenf_millionen_sperrt_und_die_grenze_selbst_nicht`); hier ist sie umgangen. Mit Antrag rechnet der Chooser
/// dann wie ohne Antrag die Fuenftelregel (2.156.648 Euro, Handrechnung wie oben), nicht den Abs.-3-Wert auf den ganzen
/// Gewinn (1.246.931 Euro). Eine Zahl auf den Gewinn ueber der Grenze ist das nicht, nur der Beleg, dass der Chooser
/// Abs. 3 dort nicht anwendet.
#[test]
fn der_chooser_nimmt_abs3_fuer_den_partner_ueber_fuenf_millionen_nicht() {
    let beantragt = gesamt_ohne_sperre(&mit_antrag(partner_gewinn(5_000_001)));
    let unbeantragt = gesamt_ohne_sperre(&partner_gewinn(5_000_001));
    assert_eq!(beantragt, unbeantragt, "Antrag ueber 5 Mio ist im Chooser wirkungslos");
    let k = kette(beantragt);
    assert_eq!(stufen(&k), [5_058_771, 5_058_699, 2_156_648, 2_156_648]);
    // dieselbe Kontrolle auf der Grenze: dort gilt der Antrag (1.246.931 Euro, siehe oben)
    let k = kette(gesamt_ohne_sperre(&mit_antrag(partner_gewinn(5_000_000))));
    assert_eq!(stufen(&k), [5_058_770, 5_058_698, 1_246_931, 1_246_931]);
}

/// Haben beide einen Gewinn und beantragen beide den Satz, rechnet der Chooser Abs. 3 fuer A (A zuerst): der Gewinn des
/// Partners bliebe ungeglaettet. Im Produkt sperrt `abs3_partner_gewinn_offen` davor
/// (`beide_gewinn_mit_antrag_von_a_behaelt_den_bisherigen_grund`); hier ist die Sperre umgangen, damit die Reihenfolge im
/// Chooser selbst gepinnt ist. Handrechnung VZ 2025 (A 300.000, Partner 500.000 Euro, zvE 858.698): A zuerst
/// 212.920 + 68.068 = 280.988 Euro (Satz 22,69 % auf 300.000); der Partner zuerst waere 128.828 + 113.448 = 242.276 Euro.
#[test]
fn der_chooser_nimmt_bei_zwei_antraegen_den_gewinn_von_a() {
    let a = vec![
        ("rentner_veraeusserungsgewinn", json!(cent(300_000))),
        ("rentner_veraeusserungs_betriebsart", json!("gewerbe")),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("antrag_ermaessigter_satz", json!(true)),
        ("geburtsjahr", json!(1970)),
        ("dauernd_berufsunfaehig", json!(false)),
        ("ermaessigung_einmal_genutzt", json!(false)),
    ];
    let beide = mit(mit_antrag(partner_gewinn(500_000)), a);
    assert_eq!(gesamt(&beide), gesperrt(GRUND_BEIDE_A), "im Produkt gesperrt");
    let k = kette(gesamt_ohne_sperre(&beide));
    assert_eq!(stufen(&k), [858_770, 858_698, 280_988, 280_988]);
}

// ---------------------------------------------------------------- Sperren (AK3)

/// Beide haben einen Gewinn und der Partner beantragt: die Software rechnet nicht (A glaettet die Fuenftelregel, der
/// Partner den ermaessigten Satz, und wie beide zusammen laufen, ist offen). Der Grund ist der des Partners.
#[test]
fn beide_gewinn_mit_antrag_des_partners_sperrt() {
    let e = mit(
        mit_antrag(partner_gewinn(500_000)),
        vec![
            ("rentner_veraeusserungsgewinn", json!(cent(200_000))),
            ("rentner_veraeusserungs_betriebsart", json!("gewerbe")),
            ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
            ("rentner_freibetrag_erstmalig", json!(true)),
        ],
    );
    assert_eq!(gesamt(&e), gesperrt(GRUND_BEIDE_B));
}

/// Der rohe Gewinn von A entscheidet, nicht der Gewinn nach dem Freibetrag: 40.000 Euro liegen unter dem Freibetrag
/// (netto null) und sperren trotzdem, wie bei A-Antrag mit Partner-Gewinn. Ein Gewinn von 0, fehlend oder negativ
/// sperrt nicht.
#[test]
fn der_rohe_gewinn_von_a_entscheidet() {
    let basis = |a: Value| {
        mit(
            mit_antrag(partner_gewinn(500_000)),
            vec![
                ("rentner_veraeusserungsgewinn", a),
                ("rentner_veraeusserungs_betriebsart", json!("gewerbe")),
                ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
                ("rentner_freibetrag_erstmalig", json!(true)),
            ],
        )
    };
    for roh in [1, cent(40_000)] {
        assert_eq!(gesamt(&basis(json!(roh))), gesperrt(GRUND_BEIDE_B), "A-Gewinn {roh}");
    }
    let abs3 = [558_770, 558_698, 114_946, 114_946];
    for roh in [0, -1] {
        // ein Gewinn von A, der nicht positiv ist, aendert den Gesamtbetrag nicht; die Kette bleibt die des Partner-Falls
        let a = gesamt(&basis(json!(roh)));
        assert!(matches!(a, Ausgang::Zahl(..)), "A-Gewinn {roh}: {a:?}");
    }
    let k = kette(gesamt(&mit_antrag(partner_gewinn(500_000))));
    assert_eq!(stufen(&k), abs3, "A ganz ohne Gewinn");
}

/// A beantragt und beide haben einen Gewinn: der vorhandene Grund (`abs3_partner_gewinn_offen`) bleibt wie er war. Haben
/// beide den Antrag gestellt, gilt der von A zuerst.
#[test]
fn beide_gewinn_mit_antrag_von_a_behaelt_den_bisherigen_grund() {
    let a = vec![
        ("rentner_veraeusserungsgewinn", json!(cent(200_000))),
        ("rentner_veraeusserungs_betriebsart", json!("gewerbe")),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("antrag_ermaessigter_satz", json!(true)),
        ("geburtsjahr", json!(1970)),
        ("dauernd_berufsunfaehig", json!(false)),
        ("ermaessigung_einmal_genutzt", json!(false)),
    ];
    // nur A beantragt
    assert_eq!(gesamt(&mit(partner_gewinn(500_000), a.clone())), gesperrt(GRUND_BEIDE_A));
    // beide beantragen
    assert_eq!(gesamt(&mit(mit_antrag(partner_gewinn(500_000)), a)), gesperrt(GRUND_BEIDE_A));
}

/// Ohne Zusammenveranlagung zaehlt der Partner nicht: kein Gewinn des Partners in der Rechnung, keine Sperre, dieselbe
/// Zahl wie ohne seine Angaben. Auch 6 Millionen Euro Gewinn des Partners sperren nicht.
#[test]
fn einzelveranlagung_liest_die_partner_angaben_nicht() {
    let einzel = |zusatz: Paare| {
        mit(
            mit(partner_gewinn(6_000_000), zusatz),
            vec![("veranlagung", json!("einzel"))],
        )
    };
    let ohne = kette(gesamt(&einzel(Vec::new())));
    let mit_antrag = kette(gesamt(&einzel(vec![
        ("antrag_ermaessigter_satz_partner", json!(true)),
        ("geburtsjahr_partner", json!(1960)),
        ("dauernd_berufsunfaehig_partner", json!(false)),
        ("ermaessigung_einmal_genutzt_partner", json!(false)),
    ])));
    assert_eq!(ohne, mit_antrag);
    // der Partner-Gewinn zaehlt bei Einzelveranlagung nicht: Gesamtbetrag = A allein
    assert_eq!(ohne.gesamtbetrag_der_einkuenfte.get(), 58_770);
}

/// Ueber 5 Millionen Euro Gewinn des Partners rechnet die Software nicht (der Teil darueber ist offen, wie bei A); auf der
/// Grenze (5.000.000 Euro) rechnet sie.
#[test]
fn ueber_fuenf_millionen_sperrt_und_die_grenze_selbst_nicht() {
    assert_eq!(
        gesamt(&mit_antrag(partner_gewinn(5_000_001))),
        gesperrt(GRUND_UEBER_5MIO)
    );
    assert!(matches!(
        gesamt(&mit_antrag(partner_gewinn(5_000_000))),
        Ausgang::Zahl(..)
    ));
    // ohne Antrag oder ohne Berechtigung ist die Grenze ohne Belang: die Fuenftelregel rechnet auch darueber
    assert!(matches!(gesamt(&partner_gewinn(5_000_001)), Ausgang::Zahl(..)));
    let zu_jung = mit(
        mit_antrag(partner_gewinn(5_000_001)),
        vec![("geburtsjahr_partner", json!(1990))],
    );
    assert!(matches!(gesamt(&zu_jung), Ausgang::Zahl(..)));
}

/// Mit Antrag, aber ohne erkennbare Berechtigung fragt die Software nach der Berufsunfaehigkeit, statt still die
/// Fuenftelregel zu rechnen: die Frage kann die Berechtigung herstellen. Beantwortet (auch mit Nein) oder berechtigt durch das
/// Alter, sperrt sie nicht.
#[test]
fn offene_berufsunfaehigkeit_des_partners_sperrt_bis_zur_antwort() {
    let ohne_bu_antwort = |gj: i64| {
        let mut e = mit_antrag(partner_gewinn(500_000));
        e.retain(|(f, _)| *f != "dauernd_berufsunfaehig_partner");
        mit(e, vec![("geburtsjahr_partner", json!(gj))])
    };
    // jung und unbeantwortet: Sperre
    assert_eq!(gesamt(&ohne_bu_antwort(1990)), gesperrt(GRUND_BU_OFFEN));
    // Geburtsjahr unbekannt (Wert 0 wie fehlend): Sperre
    let mut e = ohne_bu_antwort(1990);
    e.retain(|(f, _)| *f != "geburtsjahr_partner");
    assert_eq!(gesamt(&e), gesperrt(GRUND_BU_OFFEN));
    // alt genug: die Frage aendert nichts
    assert!(matches!(gesamt(&ohne_bu_antwort(1960)), Ausgang::Zahl(..)));
    // beantwortet mit Nein: Fuenftelregel, keine Sperre
    let nein = mit(
        mit_antrag(partner_gewinn(500_000)),
        vec![("geburtsjahr_partner", json!(1990))],
    );
    assert_eq!(
        stufen(&kette(gesamt(&nein))),
        [558_770, 558_698, 191_188, 191_188]
    );
    // ohne Antrag ist die Frage gegenstandslos
    let mut ohne_antrag = partner_gewinn(500_000);
    ohne_antrag.push(("geburtsjahr_partner", json!(1990)));
    assert!(matches!(gesamt(&ohne_antrag), Ausgang::Zahl(..)));
}

/// Die Berufsunfaehigkeit von A sperrt wie vor diesem Bau (`berufsunfaehigkeit_offen`): Antrag, keine Altersberechtigung, keine
/// Antwort. Die Sperre teilt sich ihre Hilfsfunktion mit der des Partners; bis hierher lief sie fuer A nur in der PARITY-Suite
/// und in Python. Die Antwort des Partners ersetzt die von A nicht.
#[test]
fn die_berufsunfaehigkeit_von_a_sperrt_wie_bisher_bis_zur_antwort() {
    let a = |gj: i64, bu: Option<bool>, partner_bu: Option<bool>| {
        let mut e = mit(
            partner_gewinn(0),
            vec![
                ("rentner_veraeusserungsgewinn", json!(cent(200_000))),
                ("rentner_veraeusserungs_betriebsart", json!("gewerbe")),
                ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
                ("rentner_freibetrag_erstmalig", json!(true)),
                ("antrag_ermaessigter_satz", json!(true)),
                ("geburtsjahr", json!(gj)),
                ("ermaessigung_einmal_genutzt", json!(false)),
            ],
        );
        if let Some(b) = bu {
            e.push(("dauernd_berufsunfaehig", json!(b)));
        }
        if let Some(b) = partner_bu {
            e.push(("dauernd_berufsunfaehig_partner", json!(b)));
        }
        gesamt(&e)
    };
    let offen = gesperrt("berufsunfaehigkeit_offen");
    assert_eq!(a(1990, None, None), offen, "jung, unbeantwortet");
    assert_eq!(a(0, None, None), offen, "Geburtsjahr unbekannt, unbeantwortet");
    assert_eq!(a(1990, None, Some(true)), offen, "die Antwort des Partners ersetzt die von A nicht");
    assert!(matches!(a(1990, Some(false), None), Ausgang::Zahl(..)), "beantwortet mit Nein");
    assert!(matches!(a(1990, Some(true), None), Ausgang::Zahl(..)), "berufsunfaehig");
    assert!(matches!(a(1960, None, None), Ausgang::Zahl(..)), "alt genug, die Frage aendert nichts");
}

/// Nur bestaetigte Angaben urteilen: ein vorlaeufiger Antrag rechnet nicht Abs. 3 und sperrt nicht. Auch dort nicht, wo ein
/// bestaetigter Antrag sperrte: ueber 5 Millionen Euro, bei offener Berufsunfaehigkeit und bei einem Gewinn von A.
#[test]
fn ein_vorlaeufiger_antrag_zaehlt_nicht() {
    let vorlaeufig: Ereignisse = vec![("antrag_ermaessigter_satz_partner", json!(true), false)];
    let ohne_antrag = |mut basis: Paare| {
        basis.retain(|(f, _)| *f != "antrag_ermaessigter_satz_partner");
        basis
    };
    let a_gewinn = vec![
        ("rentner_veraeusserungsgewinn", json!(cent(200_000))),
        ("rentner_veraeusserungs_betriebsart", json!("gewerbe")),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
    ];
    let basis = mit_antrag(partner_gewinn(500_000));
    let f = fall(Scheibe::Gesamt, &ohne_antrag(basis), &vorlaeufig);
    assert_eq!(
        stufen(&kette(ergebnis(&f))),
        [558_770, 558_698, 191_188, 191_188]
    );
    // jeder dieser Faelle sperrt mit BESTAETIGTEM Antrag (siehe die Tests oben); mit vorlaeufigem rechnet er die Fuenftelregel
    let sperrende = [
        ("ueber 5 Mio", mit_antrag(partner_gewinn(5_000_001))),
        (
            "Berufsunfaehigkeit offen",
            mit(
                mit_antrag(partner_gewinn(500_000)),
                vec![("geburtsjahr_partner", json!(1990))],
            ),
        ),
        ("beide Gewinn", mit(mit_antrag(partner_gewinn(500_000)), a_gewinn)),
    ];
    for (name, mut basis) in sperrende {
        if name == "Berufsunfaehigkeit offen" {
            basis.retain(|(f, _)| *f != "dauernd_berufsunfaehig_partner");
        }
        assert!(
            matches!(gesamt(&basis), Ausgang::Gesperrt(_)),
            "{name}: Kontrolle, der bestaetigte Antrag sperrt: {:?}",
            gesamt(&basis)
        );
        let f = fall(Scheibe::Gesamt, &ohne_antrag(basis), &vorlaeufig);
        assert!(
            matches!(ergebnis(&f), Ausgang::Zahl(..)),
            "{name}: ein vorlaeufiger Antrag sperrt nicht: {:?}",
            ergebnis(&f)
        );
    }
}

/// Der Antrag des Partners gilt fuer SEINEN Gewinn: hat nur A einen Gewinn, aendert er an As Rechnung nichts (Fuenftelregel,
/// 191.188 Euro bei 500.000 Euro Gewinn, derselbe Wert wie beim Partner in `ohne_antrag_gilt_die_fuenftelregel...`).
#[test]
fn der_antrag_des_partners_ohne_eigenen_gewinn_aendert_die_rechnung_von_a_nicht() {
    let nur_a = vec![
        ("rentner_veraeusserungsgewinn", json!(cent(500_000))),
        ("rentner_veraeusserungs_betriebsart", json!("gewerbe")),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
    ];
    let ohne = mit(partner_gewinn(0), nur_a.clone());
    let mit_partner_antrag = mit(mit_antrag(partner_gewinn(0)), nur_a);
    let soll = [558_770, 558_698, 191_188, 191_188];
    assert_eq!(stufen(&kette(gesamt(&ohne))), soll, "ohne Antrag des Partners");
    assert_eq!(stufen(&kette(gesamt(&mit_partner_antrag))), soll, "mit Antrag des Partners ohne dessen Gewinn");
}

// ---------------------------------------------------------------- die andere Scheibe

/// Die Rentner-Scheibe (`rentner_gesamt`) rechnet denselben Chooser: ohne Antrag die Fuenftelregel, mit Antrag Abs. 3.
/// Fall: A mit 20.000 Euro gesetzlicher Rente (Beginn 2025: Besteuerungsanteil 83,5 Prozent nach § 22 Nr. 1 Satz 3
/// Buchstabe a Doppelbuchstabe aa, `sources/gesetze-im-internet/estg_p22_2026-07-13.txt`, = 16.700 Euro, abzueglich
/// Werbungskosten-Pauschbetrag 102 Euro nach `params/2025/renten_werbungskostenpauschbetrag_p9a.yaml` = 16.598 Euro; dieselbe
/// Zahl hat der Python-Lauf `r3` der Kette), dazu 500.000 Euro Gewinn des Partners ohne Freibetrag: Gesamtbetrag 516.598 Euro,
/// zvE 516.526 Euro (Sonderausgaben-Pauschbetrag 72 Euro). Fuenftelregel 137.420 Euro, Abs. 3 105.769 Euro (Steuer auf
/// zvE - X = 16.526 Euro: 2 x Grundtarif auf 8.263 = 0; Teil auf X: 56 Prozent von 2 x tarif(258.263) / 516.526 auf 500.000).
#[test]
fn die_rentner_scheibe_rechnet_den_partner_antrag_ebenso() {
    let basis = |antrag: bool| {
        let mut b = vec![
            ("veranlagung", json!("zusammen")),
            ("rentner_renten_art", json!("gesetzliche_rente")),
            ("rentner_jahresrente", json!(2_000_000)),
            ("rentner_renten_beginn_jahr", json!(2025)),
            ("rentner_alter_bei_rentenbeginn", json!(65)),
            ("kein_sonstige", json!(false)),
            ("kein_gewinn", json!(false)),
            ("rentner_veraeusserungsgewinn_partner", json!(cent(500_000))),
            ("rentner_veraeusserungs_betriebsart_partner", json!("gewerbe")),
            ("rentner_alter_55_oder_berufsunfaehig_partner", json!(true)),
            ("rentner_freibetrag_erstmalig_partner", json!(true)),
            // 0, nicht 60.000: bis Abweichung Nr. 33 las der Rentner-Ring den Lohn des Ehegatten nie, der Wert war
            // ohne Wirkung. Seither zaehlt er (Lohn - 1.230 EUR im Gesamtbetrag); die Handrechnung oben hat keinen.
            ("bruttoarbeitslohn_partner", json!(0)),
            ("kap_kapitalertraege_partner", json!(0)),
            ("kap_gewinn_aktien_partner", json!(0)),
            ("kap_gewinn_sonstige_partner", json!(0)),
            ("kap_verlust_aktien_partner", json!(0)),
            ("kap_verlust_sonstige_partner", json!(0)),
        ];
        if antrag {
            b.extend([
                ("antrag_ermaessigter_satz_partner", json!(true)),
                ("geburtsjahr_partner", json!(1960)),
                ("dauernd_berufsunfaehig_partner", json!(false)),
                ("ermaessigung_einmal_genutzt_partner", json!(false)),
            ]);
        }
        b
    };
    let ohne = kette(ergebnis(&fall(Scheibe::RentnerGesamt, &basis(false), &Vec::new())));
    let mit = kette(ergebnis(&fall(Scheibe::RentnerGesamt, &basis(true), &Vec::new())));
    assert_eq!(stufen(&ohne), [516_598, 516_526, 137_420, 137_420], "Fuenftelregel");
    assert_eq!(stufen(&mit), [516_598, 516_526, 105_769, 105_769], "Abs. 3");
}
