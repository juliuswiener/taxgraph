//! § 34 Abs. 3 `EStG`, das 55. Lebensjahr im Jahr der Vollendung (Julius 2026-10-07, Abweichung Nr. 32): was die Antwort auf
//! "Hattest du deinen 55. Geburtstag schon vor dem Verkauf?" an Ring-Wert und Antragszeile aendert. Im Standardlauf, ohne
//! `PARITY=1`, ohne Python, ohne `ERiC`.
//!
//! **Worum es geht.** Wer seinen Betrieb verkauft, kann einmal im Leben einen ermaessigten Steuersatz beantragen, wenn er
//! das 55. Lebensjahr vollendet hat oder dauernd berufsunfaehig ist. Die Software kennt nur das Geburtsjahr und nahm bisher
//! an: Veranlagungsjahr minus Geburtsjahr mindestens 55 heisst berechtigt. Wer im Veranlagungsjahr erst 55 wird, kann den
//! Geburtstag nach dem Verkauf haben und ist dann noch nicht berechtigt. Im Jahr der Vollendung fragt die Software jetzt
//! nach; "nein" nimmt die Berechtigung ueber das Alter, "ja" und keine Antwort lassen sie.
//!
//! **Warum es zaehlt.** Ohne die Frage rechnet die Software dem Verkaeufer mit 55. Geburtstag im selben Jahr den
//! ermaessigten Satz, den das Finanzamt nicht gibt (500.000 Euro Gewinn, Zusammenveranlagung, VZ 2025: 114.946 statt
//! 191.188 Euro Steuer, `p34_partner_hermetisch.rs`). Die Erklaerung traegt dann einen Antrag, der nicht berechtigt ist.
//!
//! **Wo es sitzt.** `abzuege::abs3_eligible` und `abs3_eligible_partner` (Regel in `abs3_berechtigt`), gelesen vom Ring
//! (`deklaration/ring_werte.rs::p34_antrag`), vom Chooser (`zweige/tarif.rs`) und von den Sperren (`deklaration/sperre.rs`).
//! Hier steht die Seite des Rings und der Antragszeile; die Steuer und die Sperre pruefen
//! `rust/api/tests/p34_abs3_55_geburtstag_hermetisch.rs` (ueber die echten Routen) und der Unit-Test in `abzuege.rs`.
//!
//! Kennzahlen (XSD E10-2024 und E10-2025): Antragszeile Gewerbe `E0801602`, Basiszeile `E0801301`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::BTreeMap;

use bescheid::deklaration::mit_ring_werten;
use bescheid::testhilfe::{felder, index, params, store};
use domain::Vz;
use serde_json::{json, Value};

type Ev = (&'static str, Value, bool);

/// Die Feldnamen je Person.
struct Person {
    name: &'static str,
    geburtsjahr: &'static str,
    antwort: &'static str,
    berufsunfaehig: &'static str,
    einmal: &'static str,
    antrag: &'static str,
    gewinn: &'static str,
    betriebsart: &'static str,
    rentner_alter: &'static str,
    freibetrag: &'static str,
    ring: &'static str,
}

const A: Person = Person {
    name: "A",
    geburtsjahr: "geburtsjahr",
    antwort: "alter_55_vor_verkauf",
    berufsunfaehig: "dauernd_berufsunfaehig",
    einmal: "ermaessigung_einmal_genutzt",
    antrag: "antrag_ermaessigter_satz",
    gewinn: "rentner_veraeusserungsgewinn",
    betriebsart: "rentner_veraeusserungs_betriebsart",
    rentner_alter: "rentner_alter_55_oder_berufsunfaehig",
    freibetrag: "rentner_freibetrag_erstmalig",
    ring: "p34_abs3_antragsbetrag",
};

const PARTNER: Person = Person {
    name: "Partner",
    geburtsjahr: "geburtsjahr_partner",
    antwort: "alter_55_vor_verkauf_partner",
    berufsunfaehig: "dauernd_berufsunfaehig_partner",
    einmal: "ermaessigung_einmal_genutzt_partner",
    antrag: "antrag_ermaessigter_satz_partner",
    gewinn: "rentner_veraeusserungsgewinn_partner",
    betriebsart: "rentner_veraeusserungs_betriebsart_partner",
    rentner_alter: "rentner_alter_55_oder_berufsunfaehig_partner",
    freibetrag: "rentner_freibetrag_erstmalig_partner",
    ring: "p34_abs3_antragsbetrag_partner",
};

/// Die Person mit 500.000 Euro Veraeusserungsgewinn (Gewerbe), beantragt, nie zuvor genutzt; Geburtsjahr, Antwort (None =
/// keine; sonst Wert und ob bestaetigt) und Berufsunfaehigkeit setzt der Aufrufer.
fn person(p: &Person, gj: i64, antwort: Option<(bool, bool)>, bu: bool) -> Vec<Ev> {
    let mut ev: Vec<Ev> = vec![
        (p.gewinn, json!(50_000_000), true),
        (p.betriebsart, json!("gewerbe"), true),
        (p.rentner_alter, json!(true), true),
        (p.freibetrag, json!(true), true),
        (p.antrag, json!(true), true),
        (p.einmal, json!(false), true),
        (p.geburtsjahr, json!(gj), true),
        (p.berufsunfaehig, json!(bu), true),
    ];
    if let Some((wert, bestaetigt)) = antwort {
        ev.push((p.antwort, json!(wert), bestaetigt));
    }
    ev
}

/// Zusammenveranlagung mit den Angaben der Personen.
fn zusammen(personen: Vec<Vec<Ev>>) -> Vec<Ev> {
    let mut ev: Vec<Ev> = vec![("veranlagung", json!("zusammen"), true)];
    ev.extend(personen.into_iter().flatten());
    ev
}

/// Fall einer Person allein (Zusammenveranlagung), siehe [`person`].
fn fall(p: &Person, gj: i64, antwort: Option<(bool, bool)>, bu: bool) -> Vec<Ev> {
    zusammen(vec![person(p, gj, antwort, bu)])
}

/// Ring-Wert und die beiden Eimer der Deklaration (VZ 2025).
struct Lauf {
    ring: bool,
    person_a: BTreeMap<String, Value>,
    person_b: BTreeMap<String, Value>,
}

fn lauf(p: &Person, ev: &[Ev]) -> Lauf {
    let mut f = felder(&store(ev));
    mit_ring_werten(&mut f, Some(Vz::Vz2025), params()).unwrap();
    let d = elster::deklariere(&f, index(), 2025, None).unwrap();
    Lauf {
        ring: f.contains_key(p.ring),
        person_a: d.deklaration.clone(),
        person_b: d.person_b.clone(),
    }
}

/// Der Chooser rechnet Abs. 3 (Ring-Wert da) und die Antragszeile steht in der Anlage der PERSON, nirgends sonst.
fn rechnet_abs3(p: &Person, ev: &[Ev]) -> bool {
    let l = lauf(p, ev);
    let (eigene, fremde) = if p.name == "A" {
        (&l.person_a, &l.person_b)
    } else {
        (&l.person_b, &l.person_a)
    };
    let zeile = eigene.contains_key("E0801602");
    assert_eq!(l.ring, zeile, "{}: Ring-Wert und Antragszeile gehen zusammen", p.name);
    assert!(!fremde.contains_key("E0801602"), "{}: die Zeile steht in der Anlage der anderen Person", p.name);
    l.ring
}

/// Im Jahr der Vollendung (VZ 2025, geboren 1970) entscheidet die Antwort: "nein" nimmt die Berechtigung und damit Ring-Wert und
/// Antragszeile, "ja" und keine Antwort lassen sie. Dauernde Berufsunfaehigkeit ersetzt das Alter auch nach "nein".
#[test]
fn im_jahr_der_vollendung_nimmt_nein_die_antragszeile_und_ja_oder_keine_antwort_nicht() {
    for p in [&A, &PARTNER] {
        let n = p.name;
        assert!(rechnet_abs3(p, &fall(p, 1970, None, false)), "{n}: keine Antwort rechnet wie bisher");
        assert!(rechnet_abs3(p, &fall(p, 1970, Some((true, true)), false)), "{n}: ja");
        assert!(!rechnet_abs3(p, &fall(p, 1970, Some((false, true)), false)), "{n}: nein");
        assert!(rechnet_abs3(p, &fall(p, 1970, Some((false, true)), true)), "{n}: nein, aber dauernd berufsunfaehig");
    }
}

/// Die Antwort gilt nur im Jahr der Vollendung. Davor (54 Jahre) macht "ja" niemanden berechtigt, danach (56 und 65 Jahre)
/// aendert "nein" nichts: eine alte Antwort darf nicht mehr wirken, wenn das Geburtsjahr sich spaeter aendert.
#[test]
fn ausserhalb_des_vollendungsjahres_bleibt_die_antwort_ohne_wirkung() {
    for p in [&A, &PARTNER] {
        let n = p.name;
        assert!(!rechnet_abs3(p, &fall(p, 1971, Some((true, true)), false)), "{n}: 54 Jahre, ja");
        assert!(!rechnet_abs3(p, &fall(p, 1971, None, false)), "{n}: 54 Jahre, keine Antwort");
        assert!(rechnet_abs3(p, &fall(p, 1969, Some((false, true)), false)), "{n}: 56 Jahre, nein");
        assert!(rechnet_abs3(p, &fall(p, 1960, Some((false, true)), false)), "{n}: 65 Jahre, nein");
    }
}

/// Nur eine bestaetigte Antwort urteilt: ein vorlaeufiges "nein" nimmt die Berechtigung nicht (wie jede vorlaeufige Angabe).
#[test]
fn ein_vorlaeufiges_nein_zaehlt_nicht() {
    for p in [&A, &PARTNER] {
        assert!(
            rechnet_abs3(p, &fall(p, 1970, Some((false, false)), false)),
            "{}: ein vorlaeufiges nein",
            p.name
        );
    }
}

/// Die Antwort gilt je Person: A sagt "nein", der Partner nichts, beide beantragen bei eigenem Gewinn. Das "nein" von A nimmt A
/// den Ring-Wert, nicht dem Partner; und umgekehrt.
#[test]
fn die_antwort_gilt_je_person() {
    let ring = |ev: &[Ev]| {
        let mut f = felder(&store(ev));
        mit_ring_werten(&mut f, Some(Vz::Vz2025), params()).unwrap();
        (f.contains_key(A.ring), f.contains_key(PARTNER.ring))
    };
    let nur_a_nein = zusammen(vec![
        person(&A, 1970, Some((false, true)), false),
        person(&PARTNER, 1970, None, false),
    ]);
    assert_eq!(ring(&nur_a_nein), (false, true), "A nein, Partner keine Antwort");
    let nur_partner_nein = zusammen(vec![
        person(&A, 1970, None, false),
        person(&PARTNER, 1970, Some((false, true)), false),
    ]);
    assert_eq!(ring(&nur_partner_nein), (true, false), "A keine Antwort, Partner nein");
}
