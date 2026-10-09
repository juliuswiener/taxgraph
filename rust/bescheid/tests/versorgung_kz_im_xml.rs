//! Versorgungsbezug in den Zeilen 11 bis 13 der Anlage N (Abweichung Nr. 50, Backlog
//! `versorgungsbezuege-kennzeichen-fehlt-die-sperre-faellt-erst-mit-ihm`, Entscheid
//! `versorgungsbezug-steckt-im-bruttoarbeitslohn-bescheid-und-zeile-5-folgen-dem-vordruck`).
//!
//! Der Bruttoarbeitslohn (Zeile 5, `E0200201`) enthaelt den Bezug und bleibt, was der Nutzer eingibt. Die Zeilen 11 bis 13
//! (`E0200801` Betrag, `E0200902` Bemessungsgrundlage, `E0201307` Beginnjahr) schreibt `Bau::versorgung` nur, wenn der Ring
//! den Wert `versorgung_zeile` setzt: Bezug und Bemessungsgrundlage auf volle Euro mindestens 1, Beginnjahr gesetzt, Alters-Gate
//! erfuellt. Person B steht in der zweiten Anlage N (`person_b`). Die Kz stehen NICHT in der Bindung (Weg 2): ein Kz dort
//! liesse die Sperre von selbst fallen.
//!
//! Die Sperre (Abweichung Nr. 42) bleibt. Das erzeugte XML baut darum `xml_ohne_sperre`: dieselbe Akte ohne Bezug hat keine
//! Sperre, die Inhalte der echten Deklaration (`deklaration`, `person_b`, `anlage_instanzen`) kommen hinein. Was `deklariere`
//! berechnet hat, kommt aus dem Store, nicht aus dem Test. Fehlt das ERiC-Schema (CI), entfaellt der XML-Test.
//!
//! `checkest_nimmt_die_zeilen_11_bis_13_an` ist `#[ignore]`: er braucht ERiC und die registrierte Hersteller-ID
//! (`ELSTER_HERSTELLER_ID`, nie im Repo, nie in einer Meldung). Er prueft nur offline (`ERIC_VALIDIERE`), nichts geht raus.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::deklaration::mit_ring_werten;
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Vz, Zustand};
use elster::testhilfe::schemas_da;
use elster::{deklariere, erzeuge_xml, validiere_xsd_text, Deklaration, Felder, XmlOptionen};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const BETRAG_KZ: &str = "E0200801";
const BMG_KZ: &str = "E0200902";
const BEGINN_KZ: &str = "E0201307";
const LOHN_KZ: &str = "E0200201";
const DREI: [&str; 3] = [BETRAG_KZ, BMG_KZ, BEGINN_KZ];

type Paare = Vec<(&'static str, Value)>;

fn setze(s: &mut Store, feld: &str, wert: Value) {
    let leer = HashMap::new();
    s.append_roh(
        &NeuesEventRoh {
            feld_id: feld.to_owned(),
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: HerkunftVektor::Voll(Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }),
            schreiber: "ui:laie".to_owned(),
            signal: Signal {
                signal_1: Some(None),
                signal_2: Some(format!("ok@{feld}")),
                signal_2_fehlt: false,
            },
            signal_2_fremd: None,
            ersetzt: None,
            ts: Some("2026-09-01T00:00:00+00:00".to_owned()),
        },
        None,
        BindungNachschlag::neu(&leer),
    )
    .unwrap();
}

/// Die vollstaendige Akte `rentner.json` (Rentner-Scheibe, einzeln veranlagt, `checkESt` rc=0 ohne Zusatz) mit `mehr` obendrauf.
/// Ein Feld aus `mehr`, das die Akte schon fuehrt (`veranlagung`), ersetzt den Eintrag der Akte: der Store lehnt ein zweites
/// aktives Event auf dasselbe Feld ab.
fn akte(mehr: &[(&'static str, Value)]) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/rentner.json");
    let mut roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    roh["events"]
        .as_array_mut()
        .unwrap()
        .retain(|e| !e["feld_id"].as_str().is_some_and(|f| mehr.iter().any(|(m, _)| *m == f)));
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    for (feld, wert) in mehr {
        setze(&mut s, feld, wert.clone());
    }
    s
}

/// Der Lohnteil einer Person: Zeile 5 (Nr. 3 der Bescheinigung, enthaelt den Bezug), Steuerklasse und Lohnsteuer.
fn lohn(euro: i64) -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(euro * 100)),
        ("steuerklasse", json!(1)),
        ("p36_lohnsteuer", json!(300_000)),
    ]
}

/// Der Bezug Person A in CENT (Nr. 8, Nr. 29, Nr. 30 der Bescheinigung).
fn bezug(jahresrente: i64, bemessungsgrundlage: i64, beginn: i64, art: &str) -> Paare {
    vec![
        ("versorgung_jahresrente", json!(jahresrente)),
        ("versorgung_bemessungsgrundlage", json!(bemessungsgrundlage)),
        ("versorgung_beginn_jahr", json!(beginn)),
        ("versorgung_art", json!(art)),
    ]
}

/// Der Ehegatte als Person B: Zusammenveranlagung, Stammdaten, Kapital-Nullen, Lohnteil und optional ein Bezug.
fn ehegatte(lohn_euro: i64, bezug_partner: Option<[i64; 3]>) -> Paare {
    let mut p: Paare = vec![
        ("stammdaten_nachname_partner", json!("Muster")),
        ("stammdaten_vorname_partner", json!("Erika")),
        ("stammdaten_geburtsdatum_partner", json!("01.01.1960")),
        ("geburtsjahr_partner", json!(1960)),
        ("kist_konfession_partner", json!("keine")),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn_partner", json!(lohn_euro * 100)),
        ("steuerklasse_partner", json!(1)),
        ("p36_lohnsteuer_partner", json!(300_000)),
    ];
    if let Some([rente, bmg, beginn]) = bezug_partner {
        p.extend([
            ("versorgung_jahresrente_partner", json!(rente)),
            ("versorgung_bemessungsgrundlage_partner", json!(bmg)),
            ("versorgung_beginn_jahr_partner", json!(beginn)),
            ("versorgung_art_partner", json!("beamtenrechtlich")),
        ]);
    }
    p
}

/// Wie die Produktion (`einreichungs_xml`, `api::deklaration`): Ring, dann `deklariere`; der Guard fehlt absichtlich.
fn deklaration(mehr: &[(&'static str, Value)]) -> (Deklaration, Felder) {
    let (mut felder, _) = akte(mehr).materialisiere(None).unwrap();
    mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    (d, felder)
}

fn text(v: &Value) -> String {
    v.to_string().trim_matches('"').to_owned()
}

/// Die drei Zeilen aus einer Kz-Tabelle (Person A: `deklaration`, Person B: `person_b`): `(Betrag, Bemessungsgrundlage, Beginn)`.
fn trio(kz: &std::collections::BTreeMap<String, Value>) -> Option<(String, String, String)> {
    let hole = |k: &str| kz.get(k).map(text);
    match (hole(BETRAG_KZ), hole(BMG_KZ), hole(BEGINN_KZ)) {
        (None, None, None) => None,
        (Some(a), Some(b), Some(c)) => Some((a, b, c)),
        teil => panic!("das Trio kommt ganz oder gar nicht, erhalten {teil:?}"),
    }
}

fn tag_text<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let auf = format!("<{tag}>");
    let zu = format!("</{tag}>");
    let von = xml.find(&auf)? + auf.len();
    let bis = xml[von..].find(&zu)? + von;
    Some(&xml[von..bis])
}

/// Alle `<tag>...</tag>` in der Reihenfolge des Dokuments.
fn alle_tags<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let mut rest = xml;
    let mut gefunden = Vec::new();
    while let Some(t) = tag_text(rest, tag) {
        gefunden.push(t);
        let ende = rest.find(&format!("</{tag}>")).unwrap() + tag.len() + 3;
        rest = &rest[ende..];
    }
    gefunden
}

fn xml_von(d: &Deklaration, felder: &Felder, hersteller: Option<&str>) -> String {
    erzeuge_xml(
        d,
        &XmlOptionen {
            hersteller_id: hersteller.map(str::to_owned),
            snapshot: Some(felder),
            ..XmlOptionen::default()
        },
    )
    .unwrap()
}

/// Das XML, das die Deklaration mit Bezug ergaebe, waere die Sperre nicht da. `erzeuge_xml` lehnt jede Deklaration mit offenem
/// Eintrag ab. Der Test nimmt darum die Deklaration derselben Akte OHNE die Bezug-Felder (keine Sperre) und setzt die Inhalte
/// der echten Deklaration hinein. `hersteller` `None` liest `ELSTER_HERSTELLER_ID` (nur der `#[ignore]`-Test).
fn xml_ohne_sperre(mehr: &[(&'static str, Value)], hersteller: Option<&str>) -> String {
    xml_ohne_sperre_und_kz(mehr, hersteller, &[])
}

/// Wie [`xml_ohne_sperre`], aber ohne die Kz in `weg` (die Kontrolle K1 des `#[ignore]`-Tests streicht Zeile 5).
fn xml_ohne_sperre_und_kz(mehr: &[(&'static str, Value)], hersteller: Option<&str>, weg: &[&str]) -> String {
    let (mit, felder) = deklaration(mehr);
    let ohne: Paare = mehr
        .iter()
        .filter(|(f, _)| !f.starts_with("versorgung_"))
        .cloned()
        .collect();
    let (mut sauber, _) = deklaration(&ohne);
    assert!(sauber.unvollstaendig().is_empty(), "die Akte ohne Bezug sperrt schon: {:?}", sauber.unvollstaendig());
    sauber.deklaration.clone_from(&mit.deklaration);
    sauber.person_b.clone_from(&mit.person_b);
    sauber.anlage_instanzen.clone_from(&mit.anlage_instanzen);
    for kz in weg {
        assert!(sauber.deklaration.remove(*kz).is_some(), "{kz} steht nicht in der Deklaration");
    }
    xml_von(&sauber, &felder, hersteller)
}

fn mit(mut a: Paare, b: Paare) -> Paare {
    a.extend(b);
    a
}

/// AK3 und AK4: Lohn 50.000 (Zeile 5, enthaelt den Bezug), Bezug 30.000, Bemessungsgrundlage 28.000, Beginn 2020. Zeile 5 bleibt
/// 50.000, die drei Zeilen tragen Betrag, Bemessungsgrundlage und Beginn; Person B bekommt nichts. ROT, solange `Bau` sie nicht schreibt.
#[test]
fn der_bezug_steht_in_den_zeilen_11_bis_13_und_zeile_5_bleibt_der_lohn() {
    let (d, _) = deklaration(&mit(lohn(50_000), bezug(3_000_000, 2_800_000, 2020, "beamtenrechtlich")));
    assert_eq!(d.deklaration.get(LOHN_KZ).map(text).as_deref(), Some("50000"), "Zeile 5 bleibt der eingegebene Lohn");
    assert_eq!(
        trio(&d.deklaration),
        Some(("30000".to_owned(), "28000".to_owned(), "2020".to_owned())),
        "Zeilen 11 bis 13 Person A"
    );
    assert_eq!(trio(&d.person_b), None, "Person B hat keinen Bezug");
}

/// Beide Betraege auf volle Euro ABgerundet, wie der Bescheid (`cent_zu_euro`): 30.000,99 -> 30000, 28.000,50 -> 28000.
#[test]
fn die_zeilen_runden_ab_wie_der_bescheid() {
    let (d, _) = deklaration(&mit(lohn(50_000), bezug(3_000_099, 2_800_050, 2020, "beamtenrechtlich")));
    assert_eq!(trio(&d.deklaration), Some(("30000".to_owned(), "28000".to_owned(), "2020".to_owned())));
}

/// AK5, Gate: `altersgrenze_sonstige` mit 63 Jahren bei Beginn ist steuerbeguenstigt und gibt die Zeilen; mit 62 nicht (der Bezug
/// ist normaler Arbeitslohn und steckt in Zeile 5). ROT fuer 63, solange `Bau` nichts schreibt. Die 62 sind die Kontrolle.
#[test]
fn das_alters_gate_entscheidet_ob_die_zeilen_stehen() {
    let gate = |alter: i64| {
        let mut b = bezug(3_000_000, 3_000_000, 2020, "altersgrenze_sonstige");
        b.push(("versorgung_alter_bei_beginn", json!(alter)));
        let (d, _) = deklaration(&mit(lohn(50_000), b));
        trio(&d.deklaration)
    };
    assert_eq!(gate(63), Some(("30000".to_owned(), "30000".to_owned(), "2020".to_owned())), "63 Jahre: Gate erfuellt");
    assert_eq!(gate(62), None, "62 Jahre: kein steuerbegunstigter Bezug, keine Zeilen");
}

/// KONTROLLE (gruen heute, schuetzt den Bau): kein Bezug, Bezug 0 (die Bemessungsgrundlage und das Beginnjahr koennen aus dem
/// Vorjahr stehen, ohne Betrag lehnt `checkESt` sie ab: Regeln `Arbeitslohn_100200010` und `Arbeitslohn_ab08_9`) und ein Bezug
/// unter 1 Euro geben keine Zeile, auch kein Teil des Trios.
#[test]
fn ohne_bezug_oder_ohne_betrag_stehen_keine_zeilen() {
    let faelle: [(&str, Paare); 4] = [
        ("kein Bezug", lohn(50_000)),
        ("Bezug 0", mit(lohn(50_000), bezug(0, 2_800_000, 2020, "beamtenrechtlich"))),
        ("Bemessungsgrundlage 0", mit(lohn(50_000), bezug(3_000_000, 0, 2020, "beamtenrechtlich"))),
        ("Bezug unter 1 Euro", mit(lohn(50_000), bezug(99, 99, 2020, "beamtenrechtlich"))),
    ];
    for (name, mehr) in faelle {
        let (d, _) = deklaration(&mehr);
        assert_eq!(trio(&d.deklaration), None, "{name}: Person A");
        assert_eq!(trio(&d.person_b), None, "{name}: Person B");
    }
}

/// Person B: die Zeilen stehen in der zweiten Anlage N (`person_b`), nicht in der von Person A. ROT, solange `Bau` sie nicht schreibt.
/// Person A hat dabei einen eigenen Bezug mit anderen Zahlen: keine Vermischung.
#[test]
fn der_bezug_des_ehegatten_steht_in_der_zweiten_anlage_n() {
    let nur_b = mit(lohn(30_000), ehegatte(20_000, Some([2_000_000, 1_900_000, 2018])));
    let (d, _) = deklaration(&nur_b);
    assert_eq!(trio(&d.deklaration), None, "Person A hat keinen Bezug");
    assert_eq!(trio(&d.person_b), Some(("20000".to_owned(), "19000".to_owned(), "2018".to_owned())), "Person B");

    let beide = mit(
        mit(lohn(50_000), bezug(3_000_000, 2_800_000, 2020, "beamtenrechtlich")),
        ehegatte(20_000, Some([2_000_000, 1_900_000, 2018])),
    );
    let (d, _) = deklaration(&beide);
    assert_eq!(trio(&d.deklaration), Some(("30000".to_owned(), "28000".to_owned(), "2020".to_owned())), "Person A");
    assert_eq!(trio(&d.person_b), Some(("20000".to_owned(), "19000".to_owned(), "2018".to_owned())), "Person B");
}

/// KONTROLLE (gruen heute): ohne Zusammenveranlagung zaehlt der Bezug des Ehegatten nicht (Python und Bescheid lesen die
/// Partner-Angaben dann nicht): keine Zeilen in `person_b`.
#[test]
fn bei_einzelveranlagung_steht_der_bezug_des_ehegatten_nicht_in_der_erklaerung() {
    let mut mehr = mit(lohn(30_000), ehegatte(20_000, Some([2_000_000, 1_900_000, 2018])));
    for (feld, wert) in &mut mehr {
        if *feld == "veranlagung" {
            *wert = json!("einzel");
        }
    }
    let (d, _) = deklaration(&mehr);
    assert_eq!(trio(&d.person_b), None);
    assert_eq!(trio(&d.deklaration), None);
}

/// Die Sperre bleibt (Abweichung Nr. 42, Julius): mit Bezug haelt `erzeuge_xml` die Deklaration zurueck und nennt das Feld. Gruen
/// heute und danach; das Gegenstueck zum Umweg `xml_ohne_sperre`.
#[test]
fn die_sperre_bleibt_und_haelt_das_xml_zurueck() {
    let (d, felder) = deklaration(&mit(lohn(50_000), bezug(3_000_000, 2_800_000, 2020, "beamtenrechtlich")));
    assert!(
        d.unvollstaendig().iter().any(|e| e.feld_id == "versorgung_jahresrente"),
        "die Sperre ist weg: {:?}",
        d.unvollstaendig()
    );
    let fehler = erzeuge_xml(
        &d,
        &XmlOptionen {
            hersteller_id: Some("74931".to_owned()),
            snapshot: Some(&felder),
            ..XmlOptionen::default()
        },
    )
    .expect_err("die Sperre haelt das XML zurueck");
    assert!(fehler.0.contains("versorgung_jahresrente"), "XmlFehler `{}`", fehler.0);
}

/// Die `<N>`-Bloecke des XML in der Reihenfolge des Dokuments: Person A zuerst, Person B danach.
fn anlagen_n(xml: &str) -> Vec<&str> {
    alle_tags(xml, "N")
}

/// AK3: das ERZEUGTE XML traegt die drei Kz in der Anlage N der Person, ein `<N>` fuer Person A, ein zweites fuer Person B; das
/// Schema nimmt das Dokument. ROT, solange `Bau` sie nicht schreibt. Ohne ERiC-Schema (CI) entfaellt der Test.
#[test]
fn das_xml_traegt_die_zeilen_in_der_anlage_n_der_person_und_das_schema_nimmt_es() {
    if !schemas_da(2025) {
        return;
    }
    let nur_a = mit(lohn(50_000), bezug(3_000_000, 2_800_000, 2020, "beamtenrechtlich"));
    let xml = xml_ohne_sperre(&nur_a, Some("74931"));
    let n = anlagen_n(&xml);
    assert_eq!(n.len(), 1, "Person A allein: ein <N>");
    assert_eq!(tag_text(n[0], BETRAG_KZ), Some("30000"), "Betrag");
    assert_eq!(tag_text(n[0], BMG_KZ), Some("28000"), "Bemessungsgrundlage");
    assert_eq!(tag_text(n[0], BEGINN_KZ), Some("2020"), "Beginnjahr");
    assert_eq!(tag_text(n[0], LOHN_KZ), Some("50000"), "Zeile 5 bleibt der Lohn");
    let (ok, meldung) = validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
    assert!(ok, "xmllint gegen E10-2025.xsd (Person A): {meldung}");

    let nur_b = mit(lohn(30_000), ehegatte(20_000, Some([2_000_000, 1_900_000, 2018])));
    let xml = xml_ohne_sperre(&nur_b, Some("74931"));
    let n = anlagen_n(&xml);
    assert_eq!(n.len(), 2, "Zusammenveranlagung: zwei <N>");
    assert!(DREI.iter().all(|k| tag_text(n[0], k).is_none()), "Person A hat keinen Bezug: {:?}", n[0].len());
    assert_eq!(tag_text(n[1], BETRAG_KZ), Some("20000"), "Betrag Person B");
    assert_eq!(tag_text(n[1], BMG_KZ), Some("19000"), "Bemessungsgrundlage Person B");
    assert_eq!(tag_text(n[1], BEGINN_KZ), Some("2018"), "Beginnjahr Person B");
    let (ok, meldung) = validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
    assert!(ok, "xmllint gegen E10-2025.xsd (Person B): {meldung}");
}

/// Die Fehler, die `checkESt` nennt, als `Regelname`: nie der Rumpf der Antwort, nie die Hersteller-ID.
fn regeln(antwort: &str) -> Vec<String> {
    let mut rest = antwort;
    let mut regeln = Vec::new();
    while let Some(i) = rest.find("<RegelName>") {
        rest = &rest[i + "<RegelName>".len()..];
        regeln.push(rest.split('<').next().unwrap_or("").to_owned());
    }
    regeln
}

/// `checkESt` (offline, `ERIC_VALIDIERE`) nimmt die Zeilen an: ein reiner Pensionaer (Zeile 5 = Bezug), Lohn und Bezug, und der
/// Ehegatte in der zweiten Anlage N, je `rc=0`. KONTROLLEN, dass der Lauf urteilt: die Akte ohne Bezug ist `rc=0`; fehlt die
/// Zeile 5, lehnt `Arbeitslohn_2` ab ("Versorgungsbezuege angegeben, aber kein Arbeitslohn"); steht ein Beginnjahr nach dem
/// Veranlagungsjahr, lehnt `Arbeitslohn_ab08_5` ab. Die Meldungen nennen rc und Regelnamen, nie die ID, nie den Rumpf.
#[test]
#[ignore = "braucht die ERiC-Bibliothek und die registrierte Hersteller-ID (Umgebungs-Gate, Entscheid Julius 2026-09-12): ELSTER_HERSTELLER_ID aus der .env setzen, dann `cargo test -p bescheid --test versorgung_kz_im_xml -- --ignored`"]
fn checkest_nimmt_die_zeilen_11_bis_13_an() {
    assert!(elster::find_eric_lib().is_some(), "Bibliothek fehlt: libericapi.so liegt weder unter $ERIC_DIR noch unter ~/02_Software/eric");
    assert!(schemas_da(2025), "Schema fehlt: ohne E10-2025.xsd ist das kein Lauf");
    assert!(
        std::env::var("ELSTER_HERSTELLER_ID").is_ok_and(|v| !v.trim().is_empty()),
        "ID fehlt: ELSTER_HERSTELLER_ID ist nicht gesetzt"
    );
    let pruefe = |name: &str, xml: &str| -> (i32, Vec<String>) {
        let (rc, antwort) = elster::validiere(xml.as_bytes(), "ESt_2025").unwrap_or_else(|e| panic!("{name}: ERiC nicht erreichbar: {e:?}"));
        let klasse = elster::klassifiziere_rc(i64::from(rc));
        assert!(
            !elster::nicht_geprueft(klasse),
            "{name}: rc={rc} klasse={klasse:?}: kein Plausibilitaetsurteil (ID gesperrt oder Pruefung vor dem Urteil abgebrochen)"
        );
        (rc, regeln(&antwort))
    };
    let rein = mit(lohn(20_000), bezug(2_000_000, 2_000_000, 2020, "beamtenrechtlich"));
    let gemischt = mit(lohn(68_500), bezug(2_000_000, 2_000_000, 2020, "beamtenrechtlich"));
    let ehe = mit(lohn(20_000), ehegatte(20_000, Some([2_000_000, 2_000_000, 2020])));
    let faelle = [("reiner Pensionaer", rein.clone()), ("Lohn und Bezug", gemischt), ("Ehegatte", ehe)];
    // Kontrolle K0: die Akte ohne Bezug ist sauber.
    let (rc, r) = pruefe("K0 ohne Bezug", &xml_ohne_sperre(&lohn(20_000), None));
    assert_eq!((rc, &r[..]), (0, &[][..]), "K0: die Basis ist nicht sauber: rc={rc} {r:?}");
    for (name, mehr) in &faelle {
        let (rc, r) = pruefe(name, &xml_ohne_sperre(mehr, None));
        assert_eq!((rc, &r[..]), (0, &[][..]), "{name}: rc={rc} Regeln {r:?}");
    }
    // Kontrolle K1: ohne Zeile 5 lehnt checkESt den Bezug ab. Zeile 5 faellt per Hand aus der Deklaration.
    let (rc, r) = pruefe("K1 Bezug ohne Zeile 5", &xml_ohne_sperre_und_kz(&rein, None, &[LOHN_KZ]));
    assert!(rc != 0 && r.iter().any(|x| x.contains("Arbeitslohn_2")), "K1: erwartet Arbeitslohn_2, erhalten rc={rc} {r:?}");
    // Kontrolle K2: Beginnjahr nach dem Veranlagungsjahr.
    let spaet = mit(lohn(20_000), bezug(2_000_000, 2_000_000, 2030, "beamtenrechtlich"));
    let (rc, r) = pruefe("K2 Beginn 2030", &xml_ohne_sperre(&spaet, None));
    assert!(rc != 0 && r.iter().any(|x| x.contains("Arbeitslohn_ab08_5")), "K2: erwartet Arbeitslohn_ab08_5, erhalten rc={rc} {r:?}");
}
