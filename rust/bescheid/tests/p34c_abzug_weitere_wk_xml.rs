//! § 34c Abs. 2 `EStG`, Abzug statt Anrechnung, als ZWEITE Zeile "Sonstiges" der Anlage N (Ticket
//! `p34c-abzug-in-weitere-wk-sonst-zweite-zeile`, nach Abweichung Nr. 41 und Nr. 48): der Bescheid kuerzt die Werbungskosten
//! um die gezahlte auslaendische Steuer (`dba_abzug_werbungskosten`), die Erklaerung schreibt diese Kuerzung noch nicht.
//! Sie gehoert in `N/Wk/Weitere_Wk/Sonst` (`E0205405` Bezeichnung, `E0205406` Betrag in volle Euro, aufgerundet); die
//! Unfallkosten (Nr. 48) stehen in derselben Gruppe. Die Summe `E0204803` ist die Summe der GERUNDETEN Zeilen (Vault
//! `aufwand-einzelposten-aufrunden-summe-aus-posten`), nicht die aufgerundete Rohsumme.
//!
//! Die Zeile steht nur, wenn der Bescheid den Abzug auch rechnet (enge Bedingung: Wahl `true`, Steuer und Auslandseinkuenfte
//! ueber 0, keine Freistellung, Arbeitslohn, keine Zusammenveranlagung, keine fiktive Steuer). Die Sperre in `elster` nutzt die
//! weitere (Wahl und Steuer ueber 0) und BLEIBT, bis `checkESt` die Zeile einmal angenommen hat (Entscheid von Julius,
//! 2026-10-08, Punkt 8): ohne sie wichen Erklaerung und Bescheid um den Abzug voneinander ab.
//!
//! Die Deklaration entsteht wie in der Produktion: Ring (`mit_ring_werten`), dann `deklariere`. Der Guard fehlt hier
//! absichtlich: in der Produktion sperrt er jeden Fall, den die Rechnung nicht traegt (`DbaAbzugOffen`), VOR `deklariere`;
//! dieser Test zeigt, dass die Zeile auch ohne den Guard nur bei der engen Bedingung steht.
//!
//! Die Tests lesen die Zeilen aus `deklaration` UND aus `anlage_instanzen` (jede Gruppe) und nennen keine Gruppe: wie der Bau
//! die zweite Zeile traegt, ist seine Wahl. Das erzeugte XML baut `xml_ohne_sperre`, weil `erzeuge_xml` jede Deklaration mit
//! offenem Eintrag ablehnt (siehe `unfallkosten_weitere_wk_xml.rs`); fehlt das ERiC-Schema (CI), entfaellt dieser Test.
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

const WAHL: &str = "dba_abzug_statt_anrechnung";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const ART: &str = "dba_einkunftsart";
const FIKTIV: &str = "dba_fiktive_steuer_vorhanden";
const UNFALL: &str = "ep_unfallkosten";

const TEXT_KZ: &str = "E0205405";
const BETRAG_KZ: &str = "E0205406";
const SUMME_KZ: &str = "E0204803";

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

/// Die vollstaendige Akte `gesamt.json` (Arbeitslohn, einzeln veranlagt, Arbeitsweg 30 km, 220 Tage, Kfz). `unfall` in
/// CENT. `abzug` = `(Wahl, gezahlte Steuer in CENT)`: dann stehen auch 5.000 Euro Auslandseinkuenfte aus Arbeitslohn in der
/// Akte; `anders` ersetzt einzelne dieser Auslandsangaben (oder fuegt eine hinzu).
fn akte(unfall: Option<i64>, abzug: Option<(bool, i64)>, anders: &[(&str, Value)]) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let mut roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    for e in roh["events"].as_array_mut().unwrap() {
        match e["feld_id"].as_str() {
            Some("ep_entfernung_km") => e["wert"] = json!(30),
            Some("ep_arbeitstage") => e["wert"] = json!(220),
            Some("ep_eigenes_kfz") => e["wert"] = json!(true),
            _ => {}
        }
    }
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    if let Some(cent) = unfall {
        setze(&mut s, UNFALL, json!(cent));
    }
    let mut felder: Vec<(&str, Value)> = Vec::new();
    if let Some((wahl, steuer)) = abzug {
        felder.push((EINKUENFTE, json!(500_000)));
        felder.push((ART, json!("unselbstaendige_arbeit")));
        felder.push((STEUER, json!(steuer)));
        felder.push((WAHL, json!(wahl)));
    }
    for (name, wert) in anders {
        felder.retain(|(f, _)| f != name);
        felder.push((name, wert.clone()));
    }
    for (name, wert) in felder {
        setze(&mut s, name, wert);
    }
    s
}

/// Wie die Produktion (`einreichungs_xml`, `api::deklaration`): Ring, dann `deklariere`; der Guard fehlt (siehe oben).
fn deklaration(
    unfall: Option<i64>,
    abzug: Option<(bool, i64)>,
    anders: &[(&str, Value)],
) -> (Deklaration, Felder) {
    let s = akte(unfall, abzug, anders);
    let (mut felder, _) = s.materialisiere(None).unwrap();
    mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    (d, felder)
}

fn text(v: &Value) -> String {
    v.to_string().trim_matches('"').to_owned()
}

/// Alle Zeilen "Sonstiges" der Deklaration als `(Bezeichnung, Betrag)`: die in `deklaration` (Person A) und die in jeder
/// Gruppe von `anlage_instanzen`. Der Test nennt keine Gruppe; ein Bau darf die zweite Zeile in jeder tragen.
fn zeilen(d: &Deklaration) -> Vec<(String, String)> {
    let mut z = Vec::new();
    let mut nimm = |felder: &std::collections::BTreeMap<String, Value>| {
        if let Some(betrag) = felder.get(BETRAG_KZ) {
            let bez = felder.get(TEXT_KZ).map(text).unwrap_or_default();
            z.push((bez, text(betrag)));
        }
    };
    nimm(&d.deklaration);
    for (_, instanzen) in &d.anlage_instanzen {
        for inst in instanzen {
            nimm(&inst.felder);
        }
    }
    z
}

fn summe(d: &Deklaration) -> Option<String> {
    d.deklaration.get(SUMME_KZ).map(text)
}

/// Die Zeile des Abzugs: die Zeile, deren Bezeichnung nicht von den Unfallkosten stammt.
fn abzug_zeilen(z: &[(String, String)]) -> Vec<&(String, String)> {
    z.iter().filter(|(bez, _)| !bez.contains("Unfall")).collect()
}

/// AK1: nur der Abzug. Aufgerundet wie alle Aufwaende ("zu Ihren Gunsten"): 700,00 -> 700, 700,01 -> 701, 0,01 -> 1. Die
/// Summe der weiteren Werbungskosten ist diese eine Zeile. ROT, solange die Erklaerung den Abzug nicht schreibt.
#[test]
fn der_abzug_steht_als_zeile_sonstiges_aufgerundet_und_in_der_summe() {
    for (cent, euro) in [(70_000, "700"), (70_001, "701"), (1, "1")] {
        let (d, _) = deklaration(None, Some((true, cent)), &[]);
        let z = zeilen(&d);
        let abzug = abzug_zeilen(&z);
        assert_eq!(abzug.len(), 1, "{cent} Cent: genau eine Zeile fuer den Abzug, erhalten {z:?}");
        let (bez, betrag) = abzug[0];
        assert_eq!(betrag, euro, "{cent} Cent: Betrag der Zeile ({z:?})");
        assert!(!bez.is_empty(), "{cent} Cent: Bezeichnung der Zeile fehlt");
        assert_eq!(z.len(), 1, "{cent} Cent: ohne Unfallkosten keine zweite Zeile ({z:?})");
        assert_eq!(summe(&d).as_deref(), Some(euro), "{cent} Cent: Summe E0204803");
        // Der Abzug steht weiter in der Anlage AUS (Abweichung Nr. 41), die Zeile ersetzt ihn nicht.
        assert_eq!(d.deklaration.get("E0600920").map(text).as_deref(), Some(euro));
    }
}

/// AK2: Unfallkosten UND Abzug. Zwei Zeilen; die Summe ist die Summe der GERUNDETEN Zeilen: 1.500,01 + 700,01 Euro sind
/// 1.501 + 701 = 2.202. Die aufgerundete Rohsumme (220.002 Cent) waere 2.201 und ERiC weist ab 1 Euro Differenz die ganze
/// Erklaerung ab (Vault `aufwand-einzelposten-aufrunden-summe-aus-posten`). ROT, solange nur eine Zeile steht.
#[test]
fn unfallkosten_und_abzug_stehen_in_zwei_zeilen_die_summe_ist_die_der_gerundeten_zeilen() {
    let (d, _) = deklaration(Some(150_001), Some((true, 70_001)), &[]);
    let z = zeilen(&d);
    assert_eq!(z.len(), 2, "zwei Zeilen erwartet, erhalten {z:?}");
    let unfall: Vec<_> = z.iter().filter(|(bez, _)| bez.contains("Unfall")).collect();
    assert_eq!(unfall.len(), 1, "genau eine Zeile Unfallkosten: {z:?}");
    assert_eq!(unfall[0].1, "1501");
    let abzug = abzug_zeilen(&z);
    assert_eq!(abzug.len(), 1, "genau eine Zeile Abzug: {z:?}");
    assert_eq!(abzug[0].1, "701");
    assert_eq!(
        summe(&d).as_deref(),
        Some("2202"),
        "Summe E0204803: Summe der gerundeten Zeilen, nicht die aufgerundete Rohsumme 2201 ({z:?})"
    );
}

/// KONTROLLE (gruen): nur Unfallkosten bleiben eine Zeile mit eigener Summe, mit und ohne Anrechnung der Auslandssteuer
/// (Wahl `false` ist die Anrechnung, die in `E0601901` steht, nicht in der Zeile "Sonstiges").
#[test]
fn nur_unfallkosten_bleiben_eine_zeile() {
    for abzug in [None, Some((false, 70_001))] {
        let (d, _) = deklaration(Some(150_001), abzug, &[]);
        let z = zeilen(&d);
        assert_eq!(z.len(), 1, "{abzug:?}: {z:?}");
        assert!(z[0].0.contains("Unfall"), "{abzug:?}: {z:?}");
        assert_eq!(z[0].1, "1501", "{abzug:?}");
        assert_eq!(summe(&d).as_deref(), Some("1501"), "{abzug:?}");
    }
}

/// KONTROLLE (gruen heute, schuetzt den Bau): die Zeile steht NUR bei der engen Bedingung des Bescheids
/// (`dba_abzug_werbungskosten` > 0). Jeder dieser Faelle hat Wahl und gezahlte Steuer ueber 0 bzw. nahe daran, also die
/// WEITE Bedingung der Sperre in `elster` (`abzug_gewaehlt`), aber der Bescheid zieht nichts ab: dann darf die Erklaerung
/// keine Zeile fuer einen Abzug tragen, den der Bescheid nicht rechnet. Mit und ohne Unfallkosten: die Unfallzeile bleibt
/// die einzige, die Summe ist ihr Betrag.
#[test]
fn ohne_abzug_im_bescheid_steht_keine_abzugszeile() {
    let faelle: [(&str, Option<(bool, i64)>, Vec<(&str, Value)>); 6] = [
        ("Wahl nein", Some((false, 70_000)), vec![]),
        ("Steuer 0", Some((true, 0)), vec![]),
        ("keine Auslandseinkuenfte", Some((true, 70_000)), vec![(EINKUENFTE, json!(0))]),
        (
            "Freistellung",
            Some((true, 70_000)),
            vec![("dba_methode", json!("dba_freistellung"))],
        ),
        ("fiktive Steuer", Some((true, 70_000)), vec![(FIKTIV, json!(true))]),
        ("Zinsen statt Arbeitslohn", Some((true, 70_000)), vec![(ART, json!("zinsen"))]),
    ];
    for (name, abzug, anders) in &faelle {
        for unfall in [None, Some(150_001)] {
            let (d, _) = deklaration(unfall, *abzug, anders);
            let z = zeilen(&d);
            assert_eq!(
                z.len(),
                usize::from(unfall.is_some()),
                "{name}, Unfallkosten {unfall:?}: nur die Unfallzeile darf stehen, erhalten {z:?}"
            );
            assert_eq!(
                summe(&d).as_deref(),
                unfall.map(|_| "1501"),
                "{name}, Unfallkosten {unfall:?}: Summe"
            );
        }
    }
}

/// Die Sperre bleibt (Julius, 2026-10-08, Punkt 8): mit der Zeile im XML sperrt die Abgabe weiter, bis `checkESt` sie
/// einmal angenommen hat. Der Grund nennt die Sperre; `erzeuge_xml` lehnt die Deklaration ab. Gruen heute und danach.
#[test]
fn die_sperre_des_abzugs_bleibt() {
    let (d, felder) = deklaration(None, Some((true, 70_000)), &[]);
    let luecke = d
        .unvollstaendig()
        .iter()
        .find(|e| e.feld_id == WAHL)
        .unwrap_or_else(|| panic!("die Sperre ist weg: {:?}", d.unvollstaendig()));
    assert!(luecke.grund.contains("gesperrt"), "Sperrgrund `{}`", luecke.grund);
    let fehler = erzeuge_xml(
        &d,
        &XmlOptionen {
            hersteller_id: Some("74931".to_owned()),
            snapshot: Some(&felder),
            ..XmlOptionen::default()
        },
    )
    .expect_err("die Sperre haelt das XML zurueck");
    assert!(fehler.0.contains(WAHL), "XmlFehler `{}`", fehler.0);
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

fn xml_von(d: &Deklaration, felder: &Felder) -> String {
    erzeuge_xml(
        d,
        &XmlOptionen {
            hersteller_id: Some("74931".to_owned()),
            snapshot: Some(felder),
            ..XmlOptionen::default()
        },
    )
    .unwrap()
}

/// Das XML, das die Deklaration mit Unfallkosten und Abzug ergaebe, waeren die Sperren nicht da. `erzeuge_xml` lehnt jede
/// Deklaration mit offenem Eintrag ab. Der Test nimmt darum die Deklaration derselben Akte mit der ANRECHNUNG (Wahl `false`,
/// ohne Unfallkosten: keine Sperre) und setzt die Inhalte der echten Deklaration hinein: `deklaration`, `person_b` und
/// `anlage_instanzen`. Was `deklariere` fuer Zeilen und Summe berechnet hat, kommt aus dem Store, nicht aus dem Test.
/// Gibt zurueck: das XML und die Zahl der `<N>`-Bloecke der Anrechnung (Vergleich: eine zweite Zeile darf kein zweites `<N>`
/// anlegen).
fn xml_ohne_sperre(unfall: Option<i64>, abzug_cent: i64) -> (String, usize) {
    let (mit, felder) = deklaration(unfall, Some((true, abzug_cent)), &[]);
    let (mut sauber, _) = deklaration(None, Some((false, abzug_cent)), &[]);
    assert!(sauber.unvollstaendig().is_empty(), "die Anrechnung sperrt schon");
    let n_vorher = xml_von(&sauber, &felder).matches("<N>").count();
    sauber.deklaration = mit.deklaration.clone();
    sauber.person_b = mit.person_b.clone();
    sauber.anlage_instanzen = mit.anlage_instanzen.clone();
    (xml_von(&sauber, &felder), n_vorher)
}

/// AK3: das ERZEUGTE XML traegt beide Zeilen als zwei `<Sonst>` unter EINEM `<Weitere_Wk>`, vor `<Sum>`; kein zweites `<N>`
/// (ein zweiter Block waere Person B); die Summe ist die der gerundeten Zeilen; das Schema nimmt das Dokument. ROT, solange
/// die zweite Zeile fehlt. Ohne ERiC-Schema (CI) entfaellt der Test.
#[test]
fn das_xml_traegt_beide_zeilen_unter_einem_weitere_wk_und_das_schema_nimmt_es() {
    if !schemas_da(2025) {
        return;
    }
    let (xml, n_vorher) = xml_ohne_sperre(Some(150_001), 70_001);
    assert_eq!(xml.matches("<Weitere_Wk>").count(), 1, "genau ein <Weitere_Wk>");
    assert_eq!(xml.matches("<N>").count(), n_vorher, "eine zweite Zeile legt kein zweites <N> an");
    let block = tag_text(&xml, "Weitere_Wk").expect("kein <Weitere_Wk> im XML");
    let sonst = alle_tags(block, "Sonst");
    assert_eq!(sonst.len(), 2, "zwei <Sonst> erwartet, erhalten {}: {block}", sonst.len());
    let betraege: Vec<_> = sonst.iter().map(|s| tag_text(s, BETRAG_KZ)).collect();
    assert!(
        betraege.contains(&Some("1501")) && betraege.contains(&Some("701")),
        "Betraege der Zeilen: {betraege:?} in {block}"
    );
    for s in &sonst {
        assert!(tag_text(s, TEXT_KZ).is_some_and(|t| !t.is_empty()), "Zeile ohne Bezeichnung: {s}");
    }
    assert!(
        block.rfind("<Sonst>") < block.find("<Sum>"),
        "Schema-Reihenfolge: beide Sonst vor Sum: {block}"
    );
    let sum = tag_text(block, "Sum").unwrap_or_else(|| panic!("kein <Sum>: {block}"));
    assert_eq!(tag_text(sum, SUMME_KZ), Some("2202"), "Summe der gerundeten Zeilen ({sum})");
    let (ok, meldung) = validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
    assert!(ok, "xmllint gegen E10-2025.xsd: {meldung}");
}
