//! DBA-Freistellung (Abweichung Nr. 40, § 32b Abs. 1 S. 1 Nr. 3 `EStG`): die Sperre `dba_freistellung_offen` des
//! Gesamt-Guards `an_gesamt_sperrgrund`, Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Einkuenfte aus einem Staat, dessen Abkommen sie in Deutschland freistellt, sind hier steuerfrei,
//! erhoehen aber den Steuersatz auf das uebrige Einkommen (Progressionsvorbehalt). Die Rechnung wendet den Vorbehalt an
//! (`dba_freistellung_progression.rs`). Der Guard sperrt nur noch, was sie nicht aufloest: den Vorbehalt zusammen mit dem
//! ermaessigten Satz fuer einen Betriebsverkauf (§ 34) oder mit der Gewerbesteuer-Anrechnung (§ 35), wie beim Lohnersatz.
//!
//! **Warum es zaehlt.** Ohne diese Sperre rechnete der Post-Engine-Zweig § 32b eine Kombination, deren Verzahnung mit § 34
//! und § 35 nirgends belegt ist (Stufe 1, `einkunft.rs::p32b_koinzidenz`): ein stilles Risiko fuer den Betrag.
//!
//! Die Freistellung folgt aus `dba_methode` ODER aus Staat und Einkunftsart (`einkuenfte::dba_methode_der_akte`); beide
//! Wege zaehlen. Die Reihenfolge im Guard ist festgehalten: mehrere Staaten, Kapital und die Kombination mit Lohnersatz
//! behalten ihren eigenen Grund.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: der Grund `dba_freistellung_offen` ist Rust-eigen (Python kennt ihn nicht, Julius
//! 2026-10-08); die Methode je Staat und Einkunftsart steht in `sources/dba/` und ist in
//! `p34c_abzug_rechnung.rs::bei_freistellung_ueber_staat_und_einkunftsart_gibt_es_keinen_abzug` schon gepinnt.
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

const SCHEIBEN: [Scheibe; 2] = [Scheibe::Gesamt, Scheibe::RentnerGesamt];
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const METHODE: &str = "dba_methode";
const FREI: &str = "dba_freistellung_offen";

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

/// Der Grund auf beiden Scheiben; weichen sie ab, ist der Fall rot.
fn grund_beide(paare: &[(&str, Value)]) -> Option<&'static str> {
    let g: Vec<Option<&str>> = SCHEIBEN.iter().map(|s| grund(*s, paare)).collect();
    assert_eq!(g[0], g[1], "Gesamt und RentnerGesamt weichen ab: {g:?}");
    g[0]
}

/// 20.000 Euro freigestellte Einkuenfte ueber das Feld `dba_methode` (das Beispiel aus dem Ticket).
fn frei() -> Paare {
    vec![
        (EINKUENFTE, json!(cent(20_000))),
        (METHODE, json!("dba_freistellung")),
    ]
}

/// `frei()` plus weitere Paare.
fn frei_mit(mehr: &[(&'static str, Value)]) -> Paare {
    let mut p = frei();
    p.extend(mehr.iter().cloned());
    p
}

/// Die Steuerarten, mit denen der Vorbehalt nicht zusammen gerechnet wird: § 34 (ermaessigter Satz, Veraeusserungsgewinn
/// des Betriebs) und § 35 (Gewerbesteuer-Anrechnung), je mit den Angaben, die den Guard erreichen.
fn nachbarn() -> Vec<(&'static str, Paare)> {
    vec![
        (
            "Veraeusserungsgewinn",
            vec![("rentner_veraeusserungsgewinn", json!(cent(10_000)))],
        ),
        // Der Antrag allein stoesst zuerst an die Berufsunfaehigkeits-Frage (Abs. 3): sie ist beantwortet, damit der Fall den
        // DBA-Zweig des Guards erreicht.
        (
            "Antrag ermaessigter Satz",
            vec![
                ("antrag_ermaessigter_satz", json!(true)),
                ("alter_55_vor_verkauf", json!(true)),
                ("dauernd_berufsunfaehig", json!(false)),
            ],
        ),
        (
            "Gewerbesteuer-Messbetrag",
            vec![
                ("gewst_messbetrag", json!(cent(100))),
                ("gewst_hebesatz", json!(400)),
            ],
        ),
        (
            "Gewerbesteuer-Messbetrag des Ehegatten",
            vec![
                ("veranlagung", json!("zusammen")),
                ("gewst_messbetrag_partner", json!(cent(100))),
                ("gewst_hebesatz_partner", json!(400)),
            ],
        ),
        (
            "Veraeusserungsgewinn des Ehegatten",
            vec![
                ("veranlagung", json!("zusammen")),
                ("rentner_veraeusserungsgewinn_partner", json!(cent(10_000))),
            ],
        ),
    ]
}

/// KONTROLLE zuerst: die Anrechnung (kein Abkommen oder Anrechnungsabkommen) sperrt NICHT, auch mit Auslandseinkuenften
/// und gezahlter Steuer. Sonst belegte die Sperre unten nichts: ein Guard, der bei jedem Auslandsfall sperrt, liesse jeden
/// Test gruen.
#[test]
fn kontrolle_die_anrechnung_sperrt_nicht() {
    let basis = vec![(STEUER, json!(cent(700))), (EINKUENFTE, json!(cent(5_000)))];
    let mit = |mehr: &[(&'static str, Value)]| {
        let mut p = basis.clone();
        p.extend(mehr.iter().cloned());
        p
    };
    let faelle: [(&str, Paare); 5] = [
        ("ohne Auslandsangaben", vec![]),
        ("ohne Abkommen", mit(&[])),
        (
            "Methode Anrechnung",
            mit(&[(METHODE, json!("dba_anrechnung"))]),
        ),
        ("Niederlande pauschal", mit(&[("dba_staat", json!("nl"))])),
        (
            "Polen, Dividenden",
            mit(&[
                ("dba_staat", json!("pl")),
                ("dba_einkunftsart", json!("dividenden")),
            ]),
        ),
    ];
    for (name, paare) in faelle {
        assert_eq!(grund_beide(&paare), None, "{name}");
    }
}

/// Die Freistellung allein sperrt den Bescheid nicht mehr: die Rechnung wendet den Vorbehalt an. Ueber das Feld
/// `dba_methode` und ueber Staat und Einkunftsart (USA und Oesterreich pauschal, Polen nur fuer Ruhegehaelter).
#[test]
fn freistellung_allein_sperrt_den_bescheid_nicht() {
    assert_eq!(grund_beide(&frei()), None, "Methode");
    let faelle: [(&str, Option<&str>); 3] =
        [("us", None), ("at", None), ("pl", Some("ruhegehaelter"))];
    for (staat, art) in faelle {
        let mut paare = vec![
            (EINKUENFTE, json!(cent(20_000))),
            ("dba_staat", json!(staat)),
        ];
        if let Some(a) = art {
            paare.push(("dba_einkunftsart", json!(a)));
        }
        assert_eq!(grund_beide(&paare), None, "{staat} {art:?}");
    }
}

/// Freistellung zusammen mit § 34 oder § 35 sperrt mit `dba_freistellung_offen`, ueber beide Wege zur Methode. Gewerbesteuer
/// und Veraeusserungsgewinn des Ehegatten zaehlen mit (Zusammenveranlagung).
#[test]
fn freistellung_mit_betriebsverkauf_oder_gewerbesteuer_sperrt() {
    for (name, mehr) in nachbarn() {
        assert_eq!(grund_beide(&frei_mit(&mehr)), Some(FREI), "{name}, Methode");
        let mut ueber_staat = vec![
            (EINKUENFTE, json!(cent(20_000))),
            ("dba_staat", json!("us")),
        ];
        ueber_staat.extend(mehr.iter().cloned());
        assert_eq!(grund_beide(&ueber_staat), Some(FREI), "{name}, Staat");
    }
}

/// KONTROLLE zur Kombination: dieselben Nachbarn ohne Freistellung (Anrechnung) lösen `dba_freistellung_offen` NICHT aus.
/// Sonst sperrte der Guard schon an den Nachbarn allein.
#[test]
fn die_nachbarn_allein_loesen_den_grund_nicht_aus() {
    for (name, mehr) in nachbarn() {
        // Je Scheibe einzeln: die Scheibe gesamt sperrt den Ehegatten-Fall schon an seinem Kegel (`partner_kegel_offen`).
        let mut anrechnung = vec![
            (EINKUENFTE, json!(cent(20_000))),
            (METHODE, json!("dba_anrechnung")),
        ];
        anrechnung.extend(mehr.iter().cloned());
        for s in SCHEIBEN {
            assert_ne!(grund(s, &mehr), Some(FREI), "{name} [{s}]");
            assert_ne!(
                grund(s, &anrechnung),
                Some(FREI),
                "{name}, Anrechnung [{s}]"
            );
        }
    }
}

/// Ohne Auslandseinkuenfte ueber 0 gibt es nichts, was den Satz anhebt: weder ohne das Feld noch mit 0 noch mit nur
/// gezahlter Steuer sperrt der Guard, auch nicht neben einem Nachbarn.
#[test]
fn ohne_freigestellte_einkuenfte_sperrt_der_bescheid_nicht() {
    let nachbar = vec![("antrag_ermaessigter_satz", json!(true))];
    let faelle: [(&str, Paare); 4] = [
        (
            "nur die Methode",
            vec![(METHODE, json!("dba_freistellung"))],
        ),
        (
            "Einkuenfte 0",
            vec![(EINKUENFTE, json!(0)), (METHODE, json!("dba_freistellung"))],
        ),
        (
            "nur gezahlte Steuer",
            vec![
                (STEUER, json!(cent(700))),
                (METHODE, json!("dba_freistellung")),
            ],
        ),
        (
            "Nachbar ohne Einkuenfte",
            [vec![(METHODE, json!("dba_freistellung"))], nachbar].concat(),
        ),
    ];
    for (name, paare) in faelle {
        assert_ne!(grund_beide(&paare), Some(FREI), "{name}");
    }
}

/// Die Nachbarn zaehlen nur, wenn sie wirklich den DBA-Zweig des Guards erreichen: der Antrag allein in `nachbarn` stoesst
/// zuerst an die Berufsunfaehigkeits-Frage und belegte darum nie, dass 0 Euro Auslandseinkuenfte nicht sperren (Mutant
/// `>= 0` ueberlebte). Hier steht jeder Nachbar neben Methode Freistellung ohne Einkuenfte ueber 0.
#[test]
fn ohne_freigestellte_einkuenfte_sperrt_auch_kein_nachbar_der_den_guard_erreicht() {
    let methode = (METHODE, json!("dba_freistellung"));
    let basen: [(&str, Paare); 3] = [
        ("nur die Methode", vec![methode.clone()]),
        (
            "Einkuenfte 0",
            vec![(EINKUENFTE, json!(0)), methode.clone()],
        ),
        (
            "nur gezahlte Steuer",
            vec![(STEUER, json!(cent(700))), methode],
        ),
    ];
    for (nachbar, mehr) in nachbarn() {
        for (basis, paare) in &basen {
            let mut p = paare.clone();
            p.extend(mehr.iter().cloned());
            for s in SCHEIBEN {
                assert_ne!(grund(s, &p), Some(FREI), "{nachbar} neben {basis} [{s}]");
            }
        }
    }
}

/// Gewerbesteuer und Veraeusserungsgewinn des Ehegatten sind nur bei Zusammenveranlagung ein Nachbar: bei
/// Einzelveranlagung gehoeren sie nicht zur Rechnung der Person. Kontrolle im selben Fall: mit Zusammenveranlagung sperrt
/// dieselbe Akte.
#[test]
fn die_nachbarn_des_ehegatten_zaehlen_nur_bei_zusammenveranlagung() {
    let faelle: [(&str, Paare); 2] = [
        (
            "Gewerbesteuer des Ehegatten",
            vec![
                ("gewst_messbetrag_partner", json!(cent(100))),
                ("gewst_hebesatz_partner", json!(400)),
            ],
        ),
        (
            "Veraeusserungsgewinn des Ehegatten",
            vec![("rentner_veraeusserungsgewinn_partner", json!(cent(10_000)))],
        ),
    ];
    for (name, mehr) in faelle {
        let zusammen = frei_mit(&[&[("veranlagung", json!("zusammen"))], &mehr[..]].concat());
        assert_eq!(grund_beide(&zusammen), Some(FREI), "{name}, zusammen");
        let einzel = frei_mit(&[&[("veranlagung", json!("einzel"))], &mehr[..]].concat());
        for s in SCHEIBEN {
            assert_ne!(grund(s, &einzel), Some(FREI), "{name}, einzeln [{s}]");
        }
    }
}

/// Die Koinzidenz des Lohnersatzes zaehlt auch die gezahlte Auslandssteuer allein (ohne Auslandseinkuenfte, ohne Nachbar):
/// § 34c-Anrechnung neben § 32b bleibt unaufgeloest und sperrt mit `p32b_kombi_offen`. Kontrolle: der Lohnersatz allein
/// sperrt nicht.
#[test]
fn lohnersatz_neben_nur_gezahlter_steuer_sperrt_als_kombination() {
    let lohnersatz = ("p32b_progressionseinkuenfte", json!(cent(5_000)));
    let steuer = (STEUER, json!(cent(700)));
    assert_eq!(
        grund_beide(std::slice::from_ref(&lohnersatz)),
        None,
        "Lohnersatz allein"
    );
    assert_eq!(
        grund_beide(&[lohnersatz.clone(), steuer.clone()]),
        Some("p32b_kombi_offen"),
        "mit gezahlter Steuer"
    );
    assert_eq!(
        grund_beide(&[lohnersatz, steuer, (METHODE, json!("dba_freistellung"))]),
        Some("p32b_kombi_offen"),
        "mit gezahlter Steuer und Methode Freistellung"
    );
}

/// Die Reihenfolge im Guard: mehrere Staaten, Kapital zusammen mit Auslandseinkuenften und Lohnersatz zusammen mit
/// Auslandseinkuenften behalten ihren eigenen Grund, auch bei Freistellung.
#[test]
fn die_anderen_dba_gruende_gehen_vor() {
    assert_eq!(
        grund_beide(&frei_mit(&[("dba_mehrere_staaten", json!(true))])),
        Some("dba_multi_country_offen")
    );
    assert_eq!(
        grund_beide(&frei_mit(&[("kap_kapitalertraege", json!(cent(1_000)))])),
        Some("dba_kapital_offen")
    );
    assert_eq!(
        grund_beide(&frei_mit(&[(
            "p32b_progressionseinkuenfte",
            json!(cent(5_000))
        )])),
        Some("p32b_kombi_offen")
    );
    // Freistellung, Nachbar UND Lohnersatz: der aeltere Grund (Lohnersatz) steht vor dem neuen.
    assert_eq!(
        grund_beide(&frei_mit(&[
            ("p32b_progressionseinkuenfte", json!(cent(5_000))),
            ("rentner_veraeusserungsgewinn", json!(cent(10_000))),
        ])),
        Some("p32b_kombi_offen")
    );
}
