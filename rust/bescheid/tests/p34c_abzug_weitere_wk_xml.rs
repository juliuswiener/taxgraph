//! § 34c Abs. 2 `EStG`, Abzug statt Anrechnung, als ZWEITE Zeile "Sonstiges" der Anlage N (Ticket
//! `p34c-abzug-in-weitere-wk-sonst-zweite-zeile`, nach Abweichung Nr. 41 und Nr. 48): der Bescheid kuerzt die Werbungskosten
//! um die gezahlte auslaendische Steuer (`dba_abzug_werbungskosten`), die Erklaerung schreibt diese Kuerzung noch nicht.
//! Sie gehoert in `N/Wk/Weitere_Wk/Sonst` (`E0205405` Bezeichnung, `E0205406` Betrag in volle Euro, aufgerundet); die
//! Unfallkosten (Nr. 48) stehen in derselben Gruppe. Die Summe `E0204803` ist die Summe der GERUNDETEN Zeilen (Vault
//! `aufwand-einzelposten-aufrunden-summe-aus-posten`), nicht die aufgerundete Rohsumme.
//!
//! Die Zeile steht nur, wenn der Bescheid den Abzug auch rechnet (enge Bedingung: Wahl `true`, Steuer und Auslandseinkuenfte
//! ueber 0, keine Freistellung, Arbeitslohn, keine Zusammenveranlagung, keine fiktive Steuer). `checkESt` hat die Zeile
//! angenommen (`checkest_blockmatrix.rs`), Julius gab die Sperre am 2026-10-10 frei: die Sperre in `elster` nutzt die weitere
//! Bedingung (Wahl und Steuer ueber 0) und gilt nur noch, wo der Bescheid den Abzug NICHT rechnet; sonst wichen Erklaerung
//! und Bescheid um den Abzug voneinander ab (`der_abzug_sperrt_nur_dort_wo_der_bescheid_ihn_nicht_rechnet`).
//!
//! Die Deklaration entsteht wie in der Produktion: Ring (`mit_ring_werten`), dann `deklariere`. Der Guard fehlt hier
//! absichtlich: in der Produktion sperrt er jeden Fall, den die Rechnung nicht traegt (`DbaAbzugOffen`), VOR `deklariere`;
//! dieser Test zeigt, dass die Zeile auch ohne den Guard nur bei der engen Bedingung steht.
//!
//! Die Tests lesen die Zeilen aus `deklaration` UND aus `anlage_instanzen` (jede Gruppe) und nennen keine Gruppe: wie der Bau
//! die zweite Zeile traegt, ist seine Wahl. Das erzeugte XML baut `erzeuge_xml` aus der Deklaration der Akte (keine Sperre
//! im getragenen Fall); fehlt das ERiC-Schema (CI), entfaellt dieser Test.
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
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, PyWert, Vz, Zustand};
use elster::testhilfe::schemas_da;
use elster::{deklariere, erzeuge_xml, validiere_xsd_text, Deklaration, Felder, XmlOptionen};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const WAHL: &str = "dba_abzug_statt_anrechnung";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const ART: &str = "dba_einkunftsart";
/// Seit Abweichung Nr. 51 braucht jede Akte mit Auslandseinkuenften einen Staat; Frankreich rechnet wie "kein Staat" (Anrechnung).
const STAAT: &str = "dba_staat";
const FIKTIV: &str = "dba_fiktive_steuer_vorhanden";
const UNFALL: &str = "ep_unfallkosten";

const TEXT_KZ: &str = "E0205405";
const BETRAG_KZ: &str = "E0205406";
const SUMME_KZ: &str = "E0204803";

fn setze(s: &mut Store, feld: &str, wert: Value) {
    setze_mit(s, feld, wert, Zustand::Bestaetigt);
}

/// Ein Wert mit dem Zustand `bestaetigt` (zwei Signale) oder `vorlaeufig` (ohne zweites Signal).
fn setze_mit(s: &mut Store, feld: &str, wert: Value, zustand: Zustand) {
    let leer = HashMap::new();
    let bestaetigt = zustand == Zustand::Bestaetigt;
    s.append_roh(
        &NeuesEventRoh {
            feld_id: feld.to_owned(),
            wert: wert.into(),
            zustand,
            herkunft: HerkunftVektor::Voll(Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }),
            schreiber: "ui:laie".to_owned(),
            signal: Signal {
                signal_1: Some(None),
                signal_2: bestaetigt.then(|| format!("ok@{feld}")),
                signal_2_fehlt: !bestaetigt,
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
        felder.push((STAAT, json!("Frankreich")));
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

/// AK1: nur der Abzug. Aufgerundet wie alle Aufwaende ("zu Ihren Gunsten"): 700,00 -> 700, 700,01 -> 701, 1,00 -> 1, 1,01 -> 2.
/// Die Summe der weiteren Werbungskosten ist diese eine Zeile. ROT, solange die Erklaerung den Abzug nicht schreibt. Unter
/// 1,00 Euro rechnet der Bescheid keinen Abzug (er rundet ab, `dba_abzug_aktiv` verlangt Euro ueber 0): dort steht keine Zeile
/// (Kontrolle `ohne_abzug_im_bescheid_steht_keine_abzugszeile`, Fall "Steuer unter 1 Euro").
#[test]
fn der_abzug_steht_als_zeile_sonstiges_aufgerundet_und_in_der_summe() {
    for (cent, euro) in [(70_000, "700"), (70_001, "701"), (100, "1"), (101, "2")] {
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
/// 1.501 + 701 = 2.202. Die aufgerundete Rohsumme (220.002 Cent) waere 2.201 und `ERiC` weist ab 1 Euro Differenz die ganze
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
    type Fall<'a> = (&'a str, Option<(bool, i64)>, Vec<(&'a str, Value)>);
    let faelle: [Fall<'_>; 7] = [
        ("Wahl nein", Some((false, 70_000)), vec![]),
        ("Steuer 0", Some((true, 0)), vec![]),
        // Der Bescheid rundet die Steuer ab (99 Cent -> 0 Euro) und rechnet keinen Abzug; die Sperre in `elster` zaehlt Cent.
        ("Steuer unter 1 Euro", Some((true, 99)), vec![]),
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

/// KONTROLLE (gruen heute, schuetzt den Bau): eine nur VORLAEUFIGE gezahlte Steuer gibt keine Zeile. Die Sperre und
/// `E0600920` in `elster` lesen nur bestaetigte Werte (`Bau::abzug_gewaehlt`); schriebe der Ring den Betrag aus dem rohen
/// Feld, stuende die Zeile ohne `E0600920` in der Erklaerung.
#[test]
fn ein_vorlaeufiger_betrag_gibt_keine_abzugszeile() {
    for unfall in [None, Some(150_001)] {
        let mut s = akte(unfall, None, &[]);
        setze(&mut s, EINKUENFTE, json!(500_000));
        setze(&mut s, ART, json!("unselbstaendige_arbeit"));
        setze(&mut s, WAHL, json!(true));
        setze_mit(&mut s, STEUER, json!(70_001), Zustand::Vorlaeufig);
        let (mut felder, _) = s.materialisiere(None).unwrap();
        mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).unwrap();
        let d = deklariere(&felder, index(), 2025, None).unwrap();
        let z = zeilen(&d);
        assert_eq!(z.len(), usize::from(unfall.is_some()), "Unfallkosten {unfall:?}: erhalten {z:?}");
        assert_eq!(d.deklaration.get("E0600920"), None, "Unfallkosten {unfall:?}: E0600920 ohne bestaetigte Steuer");
    }
}

/// Die Sperre des Abzugs (Julius gab sie am 2026-10-10 frei, nachdem `checkESt` Zeile und Anlage AUS angenommen hat) gilt nur
/// noch, wo der Bescheid den Abzug NICHT rechnet: dort meldete die Erklaerung `E0600920` ohne die Zeile der Anlage N.
/// Wo der Bescheid ihn rechnet (die Zeile steht), sperrt nichts, und `erzeuge_xml` baut das XML. KONTROLLE: Wahl `false` und
/// Steuer 0 sperren nie. Der Guard fehlt hier absichtlich (siehe oben): in der Produktion sperrt er die Faelle "fiktive Steuer"
/// und "Zinsen" schon vorher mit `DbaAbzugOffen`; dieser Test zeigt, dass auch `deklariere` allein sie nicht durchlaesst.
#[test]
fn der_abzug_sperrt_nur_dort_wo_der_bescheid_ihn_nicht_rechnet() {
    type Fall<'a> = (&'a str, Option<(bool, i64)>, Vec<(&'a str, Value)>);
    // (Name, Akte, Zeile steht, Sperre am Marker)
    let faelle: [(Fall<'_>, bool, bool); 10] = [
        (("getragen 700,00", Some((true, 70_000)), vec![]), true, false),
        (("getragen 700,01", Some((true, 70_001)), vec![]), true, false),
        (("getragen 1,00", Some((true, 100)), vec![]), true, false),
        (("Wahl nein", Some((false, 70_000)), vec![]), false, false),
        (("Steuer 0", Some((true, 0)), vec![]), false, false),
        (("Steuer unter 1 Euro", Some((true, 99)), vec![]), false, true),
        (("keine Auslandseinkuenfte", Some((true, 70_000)), vec![(EINKUENFTE, json!(0))]), false, true),
        (("Freistellung", Some((true, 70_000)), vec![("dba_methode", json!("dba_freistellung"))]), false, true),
        (("fiktive Steuer", Some((true, 70_000)), vec![(FIKTIV, json!(true))]), false, true),
        (("Zinsen statt Arbeitslohn", Some((true, 70_000)), vec![(ART, json!("zinsen"))]), false, true),
    ];
    for ((name, abzug, anders), zeile_steht, sperre) in &faelle {
        let (d, felder) = deklaration(None, *abzug, anders);
        assert_eq!(!zeilen(&d).is_empty(), *zeile_steht, "{name}: Zeile Sonstiges ({:?})", zeilen(&d));
        let luecke = d.unvollstaendig().iter().find(|e| e.feld_id == WAHL).cloned();
        assert_eq!(luecke.is_some(), *sperre, "{name}: Sperre am Marker ({:?})", d.unvollstaendig());
        if let Some(l) = luecke {
            assert!(
                l.grund.contains("gesperrt") && l.grund.contains("rechnet der Bescheid keinen Abzug"),
                "{name}: Sperrgrund `{}`",
                l.grund
            );
            assert!(
                erzeuge_xml(
                    &d,
                    &XmlOptionen {
                        hersteller_id: Some("74931".to_owned()),
                        snapshot: Some(&felder),
                        ..XmlOptionen::default()
                    },
                )
                .is_err_and(|f| f.0.contains(WAHL)),
                "{name}: die Sperre haelt das XML nicht zurueck"
            );
        }
    }
}

/// FEHLERWEG (Mutant R8 der Nachmessung in main): ein bestaetigtes `dba_staat` ohne Text bricht die Rechnung des Abzugs ab
/// (`dba_methode_fuer` ruft `strip()` auf einem Nicht-Text: `AttributeError`, Python-Klasse wie im Original). Der Ring gibt
/// den Fehler weiter und schreibt keine Zeile; er faengt ihn NICHT ab und laesst die Zeile still weg. Die Methode wird vor der
/// Wahl ausgewertet, der Fehler kommt also auch bei Wahl `false`. KONTROLLE: derselbe Fall mit einem Staat als Text laeuft
/// durch und schreibt die Zeile.
#[test]
fn ein_dba_staat_ohne_text_bricht_den_ring_ab_statt_die_zeile_still_wegzulassen() {
    for staat in [json!(5), json!(true), json!(["AT"])] {
        for wahl in [true, false] {
            let s = akte(None, Some((wahl, 70_000)), &[("dba_staat", staat.clone())]);
            let (mut felder, _) = s.materialisiere(None).unwrap();
            let fehler = mit_ring_werten(&mut felder, Some(Vz::Vz2025), params())
                .expect_err(&format!("dba_staat {staat}, Wahl {wahl}: der Ring muss den Fehler weitergeben"));
            assert_eq!(
                fehler.python_klasse(),
                Some("AttributeError"),
                "dba_staat {staat}, Wahl {wahl}: Fehler `{fehler}`"
            );
            assert!(
                !felder.contains_key("dba_abzug_zeile_cent"),
                "dba_staat {staat}, Wahl {wahl}: trotz Fehler steht eine Zeile im Ring"
            );
        }
    }
    let s = akte(None, Some((true, 70_000)), &[("dba_staat", json!("Atlantis"))]);
    let (mut felder, _) = s.materialisiere(None).unwrap();
    mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).expect("ein Staat als Text rechnet");
    assert!(
        matches!(felder.get("dba_abzug_zeile_cent").map(|f| &f.wert), Some(PyWert::Ganz(70_000))),
        "Kontrolle: mit einem Staat als Text steht die Zeile, erhalten {:?}",
        felder.get("dba_abzug_zeile_cent").map(|f| &f.wert)
    );
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

/// Das XML der Akte mit Unfallkosten und Abzug auf dem Weg des Produkts (Ring, `deklariere`, `erzeuge_xml`), ohne Umweg: keine
/// der beiden Zeilen sperrt. Gibt zurueck: das XML und die Zahl der `<N>`-Bloecke der Anrechnung derselben Akte (Vergleich: eine
/// zweite Zeile darf kein zweites `<N>` anlegen).
fn xml_der_akte(unfall: Option<i64>, abzug_cent: i64) -> (String, usize) {
    let (mit, felder) = deklaration(unfall, Some((true, abzug_cent)), &[]);
    assert!(mit.unvollstaendig().is_empty(), "die Akte sperrt: {:?}", mit.unvollstaendig());
    let (anrechnung, _) = deklaration(None, Some((false, abzug_cent)), &[]);
    assert!(anrechnung.unvollstaendig().is_empty(), "die Anrechnung sperrt schon");
    let n_vorher = xml_von(&anrechnung, &felder).matches("<N>").count();
    (xml_von(&mit, &felder), n_vorher)
}

/// AK3: das ERZEUGTE XML traegt beide Zeilen als zwei `<Sonst>` unter EINEM `<Weitere_Wk>`, vor `<Sum>`; kein zweites `<N>`
/// (ein zweiter Block waere Person B); die Summe ist die der gerundeten Zeilen; das Schema nimmt das Dokument. ROT, solange
/// die zweite Zeile fehlt. Ohne ERiC-Schema (CI) entfaellt der Test.
#[test]
fn das_xml_traegt_beide_zeilen_unter_einem_weitere_wk_und_das_schema_nimmt_es() {
    if !schemas_da(2025) {
        return;
    }
    let (xml, n_vorher) = xml_der_akte(Some(150_001), 70_001);
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

/// Anlage AUS im ERZEUGTEN XML: Zeile 10 traegt die aufgerundete Steuer (`E0600920`), Zeile 7 (`E0601401`) steht NACH dem Abzug,
/// um genau diese Zahl gekuerzt (Anleitung AUS Zeile 10; Schema: `E0600920` = "in Zeile 7 abgezogene" Steuern). 5.000 Euro
/// Einkuenfte, 700,01 Euro Steuer: Zeile 10 = 701, Zeile 7 = 4.299. Bei Anrechnung (Wahl `false`) bleibt Zeile 7 bei 5.000 und die
/// Steuer steht unter `E0601901`. ROT, solange Zeile 7 den Abzug nicht abzieht.
#[test]
fn das_xml_traegt_zeile_7_nach_dem_abzug_und_zeile_10_mit_der_steuer() {
    if !schemas_da(2025) {
        return;
    }
    let (xml, _) = xml_der_akte(None, 70_001);
    assert_eq!(tag_text(&xml, "E0600920"), Some("701"), "Zeile 10");
    assert_eq!(tag_text(&xml, "E0601401"), Some("4299"), "Zeile 7 nach dem Abzug");
    assert!(!xml.contains("<E0601901>"), "Anrechnung neben dem Abzug");
    let (anrechnung, felder) = deklaration(None, Some((false, 70_001)), &[]);
    let xml = xml_von(&anrechnung, &felder);
    assert_eq!(tag_text(&xml, "E0601401"), Some("5000"), "Anrechnung: Zeile 7 unveraendert");
    assert_eq!(tag_text(&xml, "E0601901"), Some("700"), "Anrechnung: Steuer");
    assert!(!xml.contains("<E0600920>"), "Abzug neben der Anrechnung");
}
