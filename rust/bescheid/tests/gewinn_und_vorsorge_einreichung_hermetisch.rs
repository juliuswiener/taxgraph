//! Abweichung Nr. 61: die Erklaerung nennt, was die Steuerrechnung rechnet, oder die Abgabe sperrt (Entscheid
//! `die-abgabe-sperrt-wo-die-steuerrechnung-einen-wert-rechnet-den-die-erklaerung-nicht-traegt`). Der Test geht ueber
//! `einreichungs_xml`, den Aufrufer der Sperren, und nicht ueber `elster::deklariere` allein: erst hier zeigt sich, dass
//! die Sperre den Weg zum XML wirklich schliesst und dass der Guard der Rechnung den Fall nicht vorher abfaengt.
//!
//! Drei Bereiche, je mit einer Kontrolle ohne Sperre:
//! - **Vorsorge** (AK3): `E2001403`, `E2001503`, `E2001803`, `E2001903`, `E2002003` sind die gemeinsame Summe beider
//!   Ehegatten (Anlage Vorsorgeaufwand, Zeilen `A_B_LP`). Jeder Betrag wird erst aufgerundet, dann addiert.
//! - **Gewinn** (AK1, AK4): Gewerbe und selbstaendige Arbeit tragen den Gewinn unter `E0800302` / `E0803202` aus dem
//!   Direktwert. Rechnet der Bescheid ihn aus der `EUeR` oder aus Mitunternehmer-Komponenten, steht fuer diese Felder kein
//!   Kz im Schema-Pfad, den wir belegen koennen: die Abgabe sperrt mit dem Betrag im Grund, nie mit 0 im XML. Die Fehl-Sperre
//!   `gewinn_quelle_offen` faellt, wo `EUeR` und Direktwert denselben Gewinn nennen.
//! - **Kapital** (AK2): der Topf `kap_gewinn_sonstige` hat keinen eigenen Kz; ein bestaetigter Betrag ueber 0 sperrt.
//!
//! Fehlt das ERiC-Schema (CI, `TAXGRAPH_OHNE_XSD=1`), endet jeder Lauf OHNE Sperre am Writer mit `XmlNichtBaubar`;
//! die Sperren selbst liegen vor dem Writer und brauchen kein Schema.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::path::{Path, PathBuf};

use bescheid::deklaration::{einreichungs_xml, EinreichFehler};
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, PruefTiefe, Sperrgrund, Zustand};
use elster::deklariere;
use elster::testhilfe::schemas_da;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

type Lauf = Result<String, EinreichFehler>;

/// Die fuenf Vorsorge-Summen: (Kz, Feld Person A, Cent A, Feld Partner, Cent Partner, Summe in Euro nach dem Aufrunden).
const SUMMEN: [(&str, &str, i64, &str, i64, i64); 5] = [
    ("E2001403", "vorsorge_arbeitslosenversicherung", 50_001, "vorsorge_arbeitslosenversicherung_partner", 48_000, 981),
    ("E2001503", "vorsorge_erwerbsunfaehigkeit", 30_000, "vorsorge_erwerbsunfaehigkeit_partner", 32_000, 620),
    ("E2001803", "vorsorge_unfall_haftpflicht", 20_000, "vorsorge_unfall_haftpflicht_partner", 21_000, 410),
    ("E2001903", "vorsorge_rv_alt_mit_ueberschuss", 40_000, "vorsorge_rv_alt_mit_ueberschuss_partner", 41_000, 810),
    ("E2002003", "vorsorge_rv_alt_ohne_ueberschuss", 30_000, "vorsorge_rv_alt_ohne_ueberschuss_partner", 29_000, 590),
];

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json")
}

/// Schreibweg wie der Dienst (mit Bindung): ersetzt das aktive Ereignis des Feldes, falls es eines gibt.
fn setze(s: &mut Store, feld_id: &str, wert: Value) {
    let alt = s
        .aktive()
        .find(|(fid, _)| *fid == feld_id)
        .map(|(_, e)| e.event_id.to_string());
    let e = NeuesEventRoh {
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
        ersetzt: alt,
        ts: None,
    };
    s.append_roh(&e, None, BindungNachschlag::neu(index()))
        .unwrap_or_else(|f| panic!("{feld_id}: {f:?}"));
}

/// Die E2E-Akte `gesamt` (einzeln, Arbeitslohn, ohne Gewinn und Kapital; sie traegt den Pflicht-Kegel der Scheibe). Mit
/// `zusammen` steht sie auf Zusammenveranlagung, mit den Stammdaten und dem Arbeitslohn des Ehegatten; `paare` kommen danach.
fn akte(zusammen: bool, paare: &[(&str, Value)]) -> Store {
    let mut s = Store::aus_datei(store::lade(&fixture()).unwrap());
    if zusammen {
        setze(&mut s, "veranlagung", json!("zusammen"));
    }
    if zusammen {
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
        ] {
            setze(&mut s, feld, wert);
        }
    }
    for (feld, wert) in paare {
        setze(&mut s, feld, wert.clone());
    }
    s
}

/// Gewinn der Betriebsart `art` ohne jeden Betrag.
fn gewinn(art: &'static str) -> Vec<(&'static str, Value)> {
    vec![
        ("kein_gewinn", json!(false)),
        ("gewinn_betriebsart", json!(art)),
        ("gewinn_bezeichnung", json!("Testfall")),
    ]
}

/// `EUeR`-Cluster: 50.000 - 10.000 - 5.000 = 35.000 Euro laufender Gewinn.
fn euer(mut paare: Vec<(&'static str, Value)>) -> Vec<(&'static str, Value)> {
    paare.extend([
        ("betriebseinnahmen", json!(5_000_000)),
        ("sonstige_betriebsausgaben", json!(1_000_000)),
        ("afa_jahresbetrag", json!(500_000)),
    ]);
    paare
}

fn lauf(s: &Store) -> Lauf {
    einreichungs_xml(s, index(), params(), "BY", Some("74931".to_owned())).map(|e| e.xml)
}

/// Die Eintraege der Sperre als `(feld_id, grund)`; leer, wenn `lauf` nicht mit `DeklarationUnvollstaendig` endet.
fn sperre(r: &Lauf) -> Vec<(String, String)> {
    match r {
        Err(EinreichFehler::DeklarationUnvollstaendig(e)) => {
            e.iter().map(|x| (x.feld_id.clone(), x.grund.clone())).collect()
        }
        _ => Vec::new(),
    }
}

/// Die Felder der Sperre, sortiert.
fn gesperrte_felder(r: &Lauf) -> Vec<String> {
    let mut f: Vec<String> = sperre(r).into_iter().map(|(feld, _)| feld).collect();
    f.sort();
    f
}

/// Der Grund der Sperre fuer `feld`; `None`, wenn das Feld nicht sperrt.
fn grund_von(r: &Lauf, feld: &str) -> Option<String> {
    sperre(r).into_iter().find(|(f, _)| f == feld).map(|(_, g)| g)
}

/// Der Kz-Wert aus `deklariere` als Text (ohne Anfuehrungszeichen); `None`, wenn der Kz fehlt.
fn kz_wert(s: &Store, kz: &str) -> Option<String> {
    let (felder, _) = s.materialisiere(None).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    d.deklaration
        .get(kz)
        .map(|v| v.to_string().trim_matches('"').to_owned())
}

/// Der Lauf kommt durch alle Sperren: mit Schema endet er im XML, ohne am Writer.
fn ohne_sperre(r: &Lauf, was: &str) {
    assert!(sperre(r).is_empty(), "{was}: sperrt: {r:?}");
    if schemas_da(2025) {
        assert!(r.is_ok(), "{was}: kein XML: {r:?}");
    } else {
        assert!(matches!(r, Err(EinreichFehler::XmlNichtBaubar(_))), "{was}: {r:?}");
    }
}

// ---------------------------------------------------------------- Vorsorge

fn vorsorge(zusammen: bool, mit_partner: bool) -> Store {
    let mut paare: Vec<(&str, Value)> = Vec::new();
    for (_, a, cent_a, b, cent_b, _) in SUMMEN {
        paare.push((a, json!(cent_a)));
        if mit_partner {
            paare.push((b, json!(cent_b)));
        }
    }
    akte(zusammen, &paare)
}

/// AK3: bei Zusammenveranlagung steht unter jedem der fuenf Kz die Summe beider Ehegatten, je Betrag aufgerundet
/// (500,01 + 480,00 -> 501 + 480 = 981). Im XML steht jedes Kz genau einmal (die Zeile hat keine Personenachse).
#[test]
fn die_vorsorge_summe_beider_ehegatten_steht_unter_jedem_kz_und_im_xml() {
    let s = vorsorge(true, true);
    for (kz, _, _, _, _, summe) in SUMMEN {
        assert_eq!(kz_wert(&s, kz), Some(summe.to_string()), "{kz}");
    }
    let r = lauf(&s);
    ohne_sperre(&r, "Zusammenveranlagung mit Vorsorge beider Ehegatten");
    if let Ok(xml) = &r {
        for (kz, _, _, _, _, summe) in SUMMEN {
            let zeile = format!("<{kz}>{summe}</{kz}>");
            assert_eq!(xml.matches(&zeile).count(), 1, "{zeile} im XML");
        }
    }
}

/// KONTROLLE zu AK3: ohne Ehegatten-Betrag oder ohne Zusammenveranlagung steht nur der Betrag von Person A (aufgerundet).
/// Die Betraege des Ehegatten bei Einzelveranlagung gehen nicht in die Summe: die Rechnung nimmt sie dort auch nicht.
#[test]
fn ohne_zusammenveranlagung_oder_ohne_partnerbetrag_steht_nur_person_a() {
    for (zusammen, mit_partner) in [(true, false), (false, true), (false, false)] {
        let s = vorsorge(zusammen, mit_partner);
        for (kz, _, cent_a, _, _, _) in SUMMEN {
            let a_euro = (cent_a + 99) / 100;
            assert_eq!(
                kz_wert(&s, kz),
                Some(a_euro.to_string()),
                "{kz}, zusammen={zusammen}, mit_partner={mit_partner}"
            );
        }
        ohne_sperre(&lauf(&s), "nur Person A");
    }
}

// ---------------------------------------------------------------- Gewinn

/// AK1: die Rechnung nimmt den Gewinn aus der `EUeR` (35.000 Euro), der Direktwert steht auf 0: die Erklaerung nennt den
/// Gewinn nicht, die Abgabe sperrt je `EUeR`-Feld mit dem Betrag im Grund. Gewerbe und selbstaendige Arbeit.
#[test]
fn eine_euer_ohne_direktwert_sperrt_die_abgabe_mit_den_betraegen() {
    for art in ["gewerbe", "selbstaendig"] {
        let mut paare = euer(gewinn(art));
        paare.push(("einkuenfte_gewinn", json!(0)));
        let r = lauf(&akte(false, &paare));
        assert!(matches!(r, Err(EinreichFehler::DeklarationUnvollstaendig(_))), "{art}: {r:?}");
        assert_eq!(
            gesperrte_felder(&r),
            ["afa_jahresbetrag", "betriebseinnahmen", "sonstige_betriebsausgaben"],
            "{art}"
        );
        for (feld, euro) in [
            ("betriebseinnahmen", "50000,00"),
            ("sonstige_betriebsausgaben", "10000,00"),
            ("afa_jahresbetrag", "5000,00"),
        ] {
            let grund = grund_von(&r, feld).unwrap();
            assert!(grund.contains(&format!("{euro} Euro")), "{art}, {feld}: {grund}");
            assert!(grund.contains("gesperrt"), "{art}, {feld}: {grund}");
        }
    }
}

/// AK1 und AK4: nennt der Direktwert denselben Gewinn wie die `EUeR` (35.000 Euro), sperrt nichts: der Bescheid wird nicht
/// mit `gewinn_quelle_offen` abgelehnt, und die Erklaerung traegt den Gewinn unter `E0800302` (Gewerbe) / `E0803202`
/// (selbstaendig). KONTROLLE: nur der Direktwert (ohne `EUeR`) laeuft ebenfalls ohne Sperre durch.
#[test]
fn euer_und_direktwert_mit_demselben_gewinn_sperren_nicht_und_der_gewinn_steht_im_xml() {
    for (art, kz) in [("gewerbe", "E0800302"), ("selbstaendig", "E0803202")] {
        let mut nur_summe = gewinn(art);
        nur_summe.push(("einkuenfte_gewinn", json!(3_500_000)));
        let r = lauf(&akte(false, &nur_summe));
        ohne_sperre(&r, &format!("{art}: nur die Summe"));
        let mut beide = euer(gewinn(art));
        beide.push(("einkuenfte_gewinn", json!(3_500_000)));
        let s = akte(false, &beide);
        let r = lauf(&s);
        ohne_sperre(&r, &format!("{art}: EUeR und passende Summe"));
        assert_eq!(kz_wert(&s, kz).as_deref(), Some("35000"), "{art}: {kz}");
        if let Ok(xml) = &r {
            assert!(xml.contains(&format!("<{kz}>35000</{kz}>")), "{art}: {kz} fehlt im XML");
        }
    }
}

/// AK4, Gegenprobe: nennt der Direktwert einen ANDEREN Gewinn als die `EUeR` (30.000 statt 35.000 Euro), bleibt
/// `gewinn_quelle_offen` bestehen; die Erklaerung wuerde sonst einen der beiden Betraege waehlen.
#[test]
fn ein_abweichender_direktwert_sperrt_weiter_mit_gewinn_quelle_offen() {
    for art in ["gewerbe", "selbstaendig"] {
        let mut paare = euer(gewinn(art));
        paare.push(("einkuenfte_gewinn", json!(3_000_000)));
        let r = lauf(&akte(false, &paare));
        assert!(
            matches!(r, Err(EinreichFehler::Gesperrt(Sperrgrund::GewinnQuelleOffen))),
            "{art}: {r:?}"
        );
    }
}

/// AK1: Mitunternehmer-Komponenten (Person A) haben kein Kz; der Betrag steht im Grund, die Abgabe sperrt. Ein Partner-Betrag
/// sperrt nur bei Zusammenveranlagung, weil die Rechnung ihn nur dort nimmt.
#[test]
fn mitunternehmer_betraege_sperren_mit_dem_betrag_der_partner_nur_bei_zusammenveranlagung() {
    let mut a = gewinn("gewerbe");
    a.push(("gewinnanteil", json!(1_234_567)));
    let r = lauf(&akte(false, &a));
    assert_eq!(gesperrte_felder(&r), ["gewinnanteil"], "{r:?}");
    assert!(grund_von(&r, "gewinnanteil").unwrap().contains("12345,67 Euro"), "{r:?}");

    let mut b = gewinn("gewerbe");
    b.push(("gewinnanteil_partner", json!(500_000)));
    let r = lauf(&akte(true, &b));
    assert_eq!(gesperrte_felder(&r), ["gewinnanteil_partner"], "zusammen: {r:?}");
    assert!(grund_von(&r, "gewinnanteil_partner").unwrap().contains("5000,00 Euro"), "{r:?}");
    let r = lauf(&akte(false, &b));
    assert!(gesperrte_felder(&r).is_empty(), "einzeln: {r:?}");
}

// ---------------------------------------------------------------- Kapital

/// AK2: der Topf `kap_gewinn_sonstige` (Person A) und `kap_gewinn_sonstige_partner` haben keinen eigenen Kz; ein bestaetigter
/// Betrag ueber 0 sperrt mit dem Betrag im Grund, der Partner-Topf nur bei Zusammenveranlagung. KONTROLLE: derselbe Fall mit
/// Topf 0 sperrt nicht.
#[test]
fn der_kap_topf_sonstige_gewinne_sperrt_mit_dem_betrag() {
    let offen = |zusammen: bool, feld: &'static str, cent: i64| {
        let paare: Vec<(&str, Value)> = vec![
            ("kein_kap", json!(false)),
            ("kein_kap_partner", json!(false)),
            (feld, json!(cent)),
        ];
        lauf(&akte(zusammen, &paare))
    };
    ohne_sperre(&offen(false, "kap_gewinn_sonstige", 0), "Topf 0");
    ohne_sperre(&offen(true, "kap_gewinn_sonstige_partner", 0), "Partner-Topf 0");
    let r = offen(false, "kap_gewinn_sonstige", 175_000);
    assert_eq!(gesperrte_felder(&r), ["kap_gewinn_sonstige"], "{r:?}");
    assert!(grund_von(&r, "kap_gewinn_sonstige").unwrap().contains("1750,00 Euro"), "{r:?}");
    let r = offen(true, "kap_gewinn_sonstige_partner", 175_000);
    assert_eq!(gesperrte_felder(&r), ["kap_gewinn_sonstige_partner"], "{r:?}");
    assert!(grund_von(&r, "kap_gewinn_sonstige_partner").unwrap().contains("1750,00 Euro"), "{r:?}");
    let r = offen(false, "kap_gewinn_sonstige_partner", 175_000);
    assert!(gesperrte_felder(&r).is_empty(), "einzeln: {r:?}");
}
