//! § 34 Abs. 3 `EStG` fuer den Ehegatten (B Option 1, 2026-10-06): die ANTRAGSZEILE der Steuererklaerung. Im
//! Standardlauf, ohne `PARITY=1`, ohne Python, ohne `ERiC`.
//!
//! **Worum es geht.** Wer den ermaessigten Steuersatz beantragt, muss das in der Anlage ausdruecklich eintragen: eine
//! eigene Zeile neben dem Veraeusserungsgewinn, mit dem Gewinn, fuer den der Antrag gilt. Die Zeile gibt es je Betriebsart
//! (Gewerbebetrieb, selbstaendige Arbeit, Land- und Forstwirtschaft) mit je eigener Kennzahl. Fuer Person A gab es sie
//! schon; der Ehegatte hat dieselbe Anlage ein zweites Mal (Person B).
//!
//! **Warum es zaehlt.** Rechnet die Software den ermaessigten Satz, die Erklaerung traegt den Antrag aber nicht, erhebt
//! das Finanzamt die Fuenftelregel: die Software zeigt dann eine niedrigere Steuer, als das Finanzamt festsetzt. Landet
//! die Zeile in der Anlage von Person A statt in der des Ehegatten, gilt der Antrag fuer den falschen Menschen.
//!
//! **Wo es sitzt.** Der Wert entsteht in `deklaration/ring_werte.rs::p34_antrag` (`p34_abs3_antragsbetrag_partner`), die
//! Kennzahl waehlt `PARTNER_VERZWEIGUNG` in `rust/elster/src/tabellen.rs` nach
//! `rentner_veraeusserungs_betriebsart_partner`, und der Wert geht in den Eimer `person_b`.
//!
//! Kennzahlen (XSD E10-2024 und E10-2025, `rust/elster/src/tabellen.rs`): Basiszeile `E0801301` (Gewerbe) / `E0804501`
//! (selbstaendig) / `E0901201` (Land- und Forstwirtschaft), Antragszeile `E0801602` / `E0805003` / `E0901704`. Fuer 2026
//! liegt kein Schema vor: die Zeile ist fuer 2026 ungeprueft (wie bei Person A).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use bescheid::deklaration::{einreichungs_xml, mit_ring_werten, Cfg, EinreichFehler};
use bescheid::testhilfe::{felder, index, params, store};
use bindung::Bindung;
use domain::{Achsenwert, Feldtyp, Herkunft, PruefTiefe, Scheibe, Vz, Zustand};
use elster::testhilfe::schemas_da;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

type Ev = (&'static str, Value, bool);

/// Die Zeilen je Betriebsart: (Betriebsart, Basiszeile, Antragszeile).
const ARTEN: [(&str, &str, &str); 3] = [
    ("gewerbe", "E0801301", "E0801602"),
    ("selbstaendig", "E0804501", "E0805003"),
    ("land_forst", "E0901201", "E0901704"),
];

/// Zusammenveranlagung, der Partner mit `gewinn_euro` Veraeusserungsgewinn der Betriebsart `art`, beantragt, geboren 1960,
/// nicht berufsunfaehig, den Satz nie genutzt.
fn partner_mit_antrag(art: &'static str, gewinn_euro: i64) -> Vec<Ev> {
    vec![
        ("veranlagung", json!("zusammen"), true),
        ("rentner_veraeusserungsgewinn_partner", json!(gewinn_euro * 100), true),
        ("rentner_veraeusserungs_betriebsart_partner", json!(art), true),
        ("rentner_alter_55_oder_berufsunfaehig_partner", json!(true), true),
        ("rentner_freibetrag_erstmalig_partner", json!(true), true),
        ("antrag_ermaessigter_satz_partner", json!(true), true),
        ("geburtsjahr_partner", json!(1960), true),
        ("dauernd_berufsunfaehig_partner", json!(false), true),
        ("ermaessigung_einmal_genutzt_partner", json!(false), true),
    ]
}

/// Ersetzt (oder ergaenzt) Ereignisse.
fn mit(mut basis: Vec<Ev>, aenderungen: &[Ev]) -> Vec<Ev> {
    for (f, w, b) in aenderungen {
        match basis.iter_mut().find(|(g, ..)| g == f) {
            Some(e) => *e = (f, w.clone(), *b),
            None => basis.push((f, w.clone(), *b)),
        }
    }
    basis
}

/// Der Ring-Wert und die beiden Eimer der Deklaration (VZ 2025).
struct Lauf {
    ring_a: bool,
    ring_partner: bool,
    person_a: BTreeMap<String, Value>,
    person_b: BTreeMap<String, Value>,
}

fn lauf(ev: &[Ev]) -> Lauf {
    let mut f = felder(&store(ev));
    mit_ring_werten(&mut f, Some(Vz::Vz2025), params()).unwrap();
    let d = elster::deklariere(&f, index(), 2025, None).unwrap();
    Lauf {
        ring_a: f.contains_key("p34_abs3_antragsbetrag"),
        ring_partner: f.contains_key("p34_abs3_antragsbetrag_partner"),
        person_a: d.deklaration.clone(),
        person_b: d.person_b.clone(),
    }
}

/// Mit Antrag und Berechtigung steht die Antragszeile in Person B, je Betriebsart unter ihrer Kennzahl, mit demselben Betrag
/// wie die Basiszeile (500.000 Euro); in Person A steht keine der beiden Zeilen.
#[test]
fn die_antragszeile_steht_in_person_b_je_betriebsart_unter_ihrer_kennzahl() {
    for (art, basis, antrag) in ARTEN {
        let l = lauf(&partner_mit_antrag(art, 500_000));
        assert!(l.ring_partner && !l.ring_a, "{art}: Ring-Wert");
        assert_eq!(l.person_b.get(basis), Some(&json!(500_000)), "{art}: Basiszeile {basis} in Person B");
        assert_eq!(l.person_b.get(antrag), Some(&json!(500_000)), "{art}: Antragszeile {antrag} in Person B");
        for (_, b, a) in ARTEN {
            assert!(
                !l.person_a.contains_key(b) && !l.person_a.contains_key(a),
                "{art}: Person A traegt {b} oder {a}: {:?}",
                l.person_a
            );
        }
        // die beiden anderen Betriebsarten bleiben leer
        for (andere, b, a) in ARTEN {
            if andere != art {
                assert!(!l.person_b.contains_key(a) && !l.person_b.contains_key(b), "{art}: {andere}");
            }
        }
    }
}

/// Die Basiszeile ist der ROHE Gewinn, auch wenn der Freibetrag nach § 16 Abs. 4 ihn mindert (bis 181.000 Euro): die
/// Antragszeile traegt denselben Betrag, nicht den Gewinn nach Freibetrag. 100.000 Euro roh, 45.000 Euro Freibetrag.
#[test]
fn die_antragszeile_traegt_den_rohen_gewinn_wie_die_basiszeile() {
    let l = lauf(&partner_mit_antrag("gewerbe", 100_000));
    assert_eq!(l.person_b.get("E0801301"), Some(&json!(100_000)));
    assert_eq!(l.person_b.get("E0801602"), Some(&json!(100_000)));
}

/// Keine Antragszeile ohne Antrag, ohne Berechtigung, nach einmaliger Nutzung, bei Einzelveranlagung, ueber 5 Millionen
/// Euro und bei einem vorlaeufigen Antrag; der Gewinn selbst (Basiszeile) steht, wo er hingehoert. Die Zeile entsteht
/// nur, wenn der Chooser Abs. 3 auch rechnet (`abs3_wird_gerechnet_partner`).
#[test]
fn die_antragszeile_entsteht_nur_wenn_der_chooser_abs3_rechnet() {
    let basis = partner_mit_antrag("gewerbe", 500_000);
    let faelle: Vec<(&str, Vec<Ev>, bool)> = vec![
        ("ohne Antrag", mit(basis.clone(), &[("antrag_ermaessigter_satz_partner", json!(false), true)]), false),
        ("Antrag unbeantwortet", basis.iter().filter(|(f, ..)| *f != "antrag_ermaessigter_satz_partner").cloned().collect(), false),
        ("vorlaeufiger Antrag", mit(basis.clone(), &[("antrag_ermaessigter_satz_partner", json!(true), false)]), false),
        ("zu jung", mit(basis.clone(), &[("geburtsjahr_partner", json!(1971), true)]), false),
        ("jung, aber berufsunfaehig", mit(basis.clone(), &[("geburtsjahr_partner", json!(1990), true), ("dauernd_berufsunfaehig_partner", json!(true), true)]), true),
        ("schon einmal genutzt", mit(basis.clone(), &[("ermaessigung_einmal_genutzt_partner", json!(true), true)]), false),
        ("Einzelveranlagung", mit(basis.clone(), &[("veranlagung", json!("einzel"), true)]), false),
        ("Veranlagung nur vorlaeufig", mit(basis.clone(), &[("veranlagung", json!("zusammen"), false)]), false),
        ("ueber 5 Millionen", partner_mit_antrag("gewerbe", 5_000_001), false),
        ("genau 5 Millionen", partner_mit_antrag("gewerbe", 5_000_000), true),
        ("Gewinn null", partner_mit_antrag("gewerbe", 0), false),
    ];
    for (name, ev, soll) in faelle {
        let l = lauf(&ev);
        assert_eq!(l.ring_partner, soll, "{name}: Ring-Wert");
        assert_eq!(l.person_b.contains_key("E0801602"), soll, "{name}: Antragszeile in Person B");
        assert!(!l.person_a.contains_key("E0801602"), "{name}: Person A traegt die Antragszeile");
    }
}

/// Person A und der Partner haben je ihre Zeile: A beantragt fuer den eigenen Gewinn (Betriebsart selbstaendig), der Partner
/// fuer seinen (Gewerbe). Jede Zeile liegt im eigenen Eimer mit der Kennzahl der EIGENEN Betriebsart. Die Sperre, die diesen
/// Fall in der Berechnung abfaengt (`abs3_partner_gewinn_offen`), liegt in `deklaration/sperre.rs`; hier zaehlt nur, dass
/// die Zeilen sich nicht gegenseitig ueberschreiben.
#[test]
fn die_zeilen_von_a_und_vom_partner_ueberschreiben_sich_nicht() {
    let mut ev = partner_mit_antrag("gewerbe", 500_000);
    ev.extend([
        ("rentner_veraeusserungsgewinn", json!(20_000_000), true),
        ("rentner_veraeusserungs_betriebsart", json!("selbstaendig"), true),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true), true),
        ("rentner_freibetrag_erstmalig", json!(true), true),
        ("antrag_ermaessigter_satz", json!(true), true),
        ("geburtsjahr", json!(1970), true),
        ("dauernd_berufsunfaehig", json!(false), true),
        ("ermaessigung_einmal_genutzt", json!(false), true),
    ]);
    let l = lauf(&ev);
    assert!(l.ring_a && l.ring_partner);
    assert_eq!(l.person_a.get("E0805003"), Some(&json!(200_000)), "A: Antragszeile selbstaendig");
    assert_eq!(l.person_b.get("E0801602"), Some(&json!(500_000)), "Partner: Antragszeile Gewerbe");
    assert!(!l.person_a.contains_key("E0801602"), "A traegt die Zeile des Partners");
    assert!(!l.person_b.contains_key("E0805003"), "Partner traegt die Zeile von A");
}

// ---------------------------------------------------------------- das amtliche XML

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json")
}

fn antwort(feld_id: &str, wert: Value) -> NeuesEventRoh {
    NeuesEventRoh {
        feld_id: feld_id.to_owned(),
        wert: wert.into(),
        zustand: Zustand::Bestaetigt,
        herkunft: domain::HerkunftVektor::Voll(Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        }),
        schreiber: "ui:laie".to_owned(),
        signal: Signal {
            signal_1: Some(None),
            signal_2: Some(format!("ok@{feld_id}")),
            signal_2_fehlt: false,
        },
        signal_2_fremd: None,
        ersetzt: None,
        ts: None,
    }
}

/// Schreibweg wie der Dienst (mit Bindung): ersetzt das aktive Ereignis des Feldes, falls es eines gibt.
fn setze(s: &mut Store, feld_id: &str, wert: Value) {
    let alt = s
        .aktive()
        .find(|(fid, _)| *fid == feld_id)
        .map(|(_, e)| e.event_id.to_string());
    let mut e = antwort(feld_id, wert);
    e.ersetzt = alt;
    s.append_roh(&e, None, BindungNachschlag::neu(index()))
        .unwrap_or_else(|f| panic!("{feld_id}: {f:?}"));
}

/// `tests/_kegel.py::standardwert`: der Abwesenheitswert, nie der illustrative Beispielwert.
fn standardwert(b: &Bindung) -> Value {
    if let Some(w) = &b.abwesenheitswert {
        return w.clone();
    }
    match b.typ {
        Feldtyp::Bool => json!(b.feld_id.starts_with("kein_") || b.feld_id.starts_with("keine_")),
        Feldtyp::Cent | Feldtyp::Int => json!(0),
        _ => b.beispielwert.clone(),
    }
}

/// Die E2E-Akte `gesamt` (Einzelveranlagung), auf Zusammenveranlagung umgestellt, mit dem vollen Pflicht-Kegel der Scheibe
/// (Abwesenheitswerte) und den Stammdaten des Partners; dazu der Partner-Gewinn `art` mit Antrag.
fn zusammen_akte(art: &'static str, gewinn_euro: i64) -> Store {
    let mut s = Store::aus_datei(store::lade(&fixture()).unwrap());
    setze(&mut s, "veranlagung", json!("zusammen"));
    let aktive: HashSet<String> = s.aktive().map(|(f, _)| f.to_owned()).collect();
    for f in Cfg::fuer(Scheibe::Gesamt).kegel_roh().unwrap() {
        if !aktive.contains(*f) {
            setze(&mut s, f, standardwert(index()[*f]));
        }
    }
    for (feld, wert) in [
        ("stammdaten_nachname_partner", json!("Muster")),
        ("stammdaten_vorname_partner", json!("Max")),
        ("stammdaten_geburtsdatum_partner", json!("01.02.1960")),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("kein_gewinn", json!(false)),
    ] {
        setze(&mut s, feld, wert);
    }
    for (feld, wert, _) in partner_mit_antrag(art, gewinn_euro).into_iter().filter(|(f, ..)| *f != "veranlagung") {
        setze(&mut s, feld, wert);
    }
    s
}

/// Wie oft ein Element im XML steht und wem sein Anlage-Abschnitt gehoert: je Treffer der Text von `<Person>` davor
/// (`PersonA` / `PersonB`). Der Abschnitt jeder Anlage beginnt mit `<Person>`; ein Treffer ohne dieses Element davor gaebe
/// `None`.
fn personen_von(xml: &str, kz: &str) -> Vec<Option<String>> {
    let offen = format!("<{kz}>");
    xml.match_indices(&offen)
        .map(|(i, _)| {
            let davor = &xml[..i];
            davor.rfind("<Person>").map(|p| {
                davor[p + "<Person>".len()..]
                    .split('<')
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            })
        })
        .collect()
}

/// Das amtliche XML einer Zusammenveranlagung: der Partner mit Antrag traegt die Basiszeile UND die Antragszeile im
/// Abschnitt `PersonB` seiner Anlage, je Betriebsart unter der Kennzahl der Betriebsart, und das Ganze ist gegen das Schema
/// `E10-2025` gueltig. Person A traegt keine der beiden Zeilen. Das Schema liegt nur in der lokalen ERiC-Auslieferung
/// (`~/02_Software/eric`), nie im Repo; in der CI (`TAXGRAPH_OHNE_XSD=1`) baut der Writer kein XML, und der Test meldet den
/// Verzicht auf stderr, statt ihn zu verschweigen.
#[test]
fn das_xml_traegt_die_antragszeile_im_abschnitt_des_partners_und_ist_schemagueltig() {
    for (art, basis, antrag) in ARTEN {
        let r = einreichungs_xml(&zusammen_akte(art, 500_000), index(), params(), "BY", Some("74931".to_owned()));
        if !schemas_da(2025) {
            assert!(
                matches!(r, Err(EinreichFehler::XmlNichtBaubar(_))),
                "{art}: ohne Schema baut der Writer nicht: {:?}",
                r.map(|e| e.xml.len())
            );
            eprintln!("p34-Partner-XML ({art}): kein lokales Schema, Antragszeile im XML nicht geprueft");
            continue;
        }
        let xml = r.unwrap_or_else(|f| panic!("{art}: {f}")).xml;
        for kz in [basis, antrag] {
            assert_eq!(
                personen_von(&xml, kz),
                vec![Some("PersonB".to_owned())],
                "{art}: {kz} muss genau einmal und in PersonB stehen"
            );
        }
        let (ok, meldung) = elster::validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
        assert!(ok, "{art}: das XML ist nicht schemagueltig: {meldung}");
    }
}

/// Gegenprobe im selben Fall: ohne Antrag steht die Basiszeile in `PersonB` und die Antragszeile nirgends.
#[test]
fn ohne_antrag_steht_nur_die_basiszeile_im_xml() {
    if !schemas_da(2025) {
        eprintln!("p34-Partner-XML: kein lokales Schema, ohne Antrag nicht geprueft");
        return;
    }
    let mut s = zusammen_akte("gewerbe", 500_000);
    setze(&mut s, "antrag_ermaessigter_satz_partner", json!(false));
    let xml = einreichungs_xml(&s, index(), params(), "BY", Some("74931".to_owned()))
        .unwrap_or_else(|f| panic!("{f}"))
        .xml;
    assert_eq!(personen_von(&xml, "E0801301"), vec![Some("PersonB".to_owned())]);
    assert!(personen_von(&xml, "E0801602").is_empty(), "Antragszeile ohne Antrag im XML");
}
