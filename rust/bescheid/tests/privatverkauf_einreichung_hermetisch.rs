//! Privatverkauf (Abweichung Nr. 54): was die Erklaerung traegt und wann die Abgabe sperrt. Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Der Bescheid rechnet einen privaten Verkauf (`api/tests/p23_befreiungen_hermetisch.rs`). Die Erklaerung traegt
//! nur steuerpflichtige Verkaeufe in die Anlage SO: Bezeichnung, beide Daten und Gewinn unter den Kz der Art (Grundstueck
//! E0306111/E0306201/E0306202/E0306801, anderes Wirtschaftsgut E0307101/E0307201/E0307202/E0307701). Ein Verkauf mit abgelaufener
//! Frist und ein gebrauchter Gegenstand des taeglichen Gebrauchs steht NICHT drin: die Anleitung zur Anlage SO nimmt beide aus den
//! Zeilen heraus (`sources/bfinv/anleitungen/anl_so_2025.txt`), die Abgabe sperrt nicht. Ein selbst bewohnter Verkauf steht noch nicht
//! drin: die Abgabe sperrt mit Klartext, die Rechnung bleibt richtig.
//!
//! **Warum es zaehlt.** Ohne den Filter truege der Schreiber jeden Verkauf der Akte ein, also auch den, den das Gesetz nicht erfasst,
//! und das Finanzamt saehe einen Gewinn, den es nicht besteuern darf.
//!
//! Der Test geht ueber `einreichungs_xml` und nicht ueber `POST /einreichen` (der HTTP-Weg ruft danach `ERiC`). Die Akte ist
//! `rust/fixtures/e2e/gesamt.json` (Scheibe `gesamt`, vollstaendig, `kein_sonstige` = ja) mit dem Kreuz `kein_p23_verkauf` = nein und
//! den Verkaeufen dieses Tests. Erwartete Werte von Hand aus dem Gesetz: Gewinn = Preis - (Anschaffungskosten - `AfA`) - Werbungskosten,
//! in vollen Euro abgerundet (`checkESt` rechnet den Gewinn nach und lehnt Abweichungen mit rc=610001002 ab, Spike AK0).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use bescheid::deklaration::{einreichungs_xml, EinreichFehler, Einreichung};
use bescheid::privatverkauf::p23_steuerfreier_verkauf;
use bescheid::testhilfe::{index, params};
use bescheid::Instanzquelle;
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Sperrgrund, Vz, Zustand};
use elster::testhilfe::schemas_da;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

/// Ein bestaetigter Wert (zwei Signale).
fn setze(s: &mut Store, feld: &str, wert: Value) {
    setze_mit(s, feld, wert, Zustand::Bestaetigt);
}

/// Ein Wert mit dem Zustand `bestaetigt` (zwei Signale) oder `vorlaeufig` (ohne zweites Signal).
fn setze_mit(s: &mut Store, feld: &str, wert: Value, zustand: Zustand) {
    let bestaetigt = zustand == Zustand::Bestaetigt;
    let leer = HashMap::new();
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

fn name(basis: &str, n: u8) -> String {
    if n == 1 {
        basis.to_owned()
    } else {
        format!("{basis}__{n}")
    }
}

/// Ein vollstaendiger Verkauf der Instanz `n` (Daten TT.MM.JJJJ, Betraege in Cent, Werbungskosten und `AfA` 0). Beim Grundstueck "keine
/// Eigennutzung, kein Gebaeude der letzten zehn Jahre", beim anderen Wirtschaftsgut "kein taeglicher Gebrauch, keine Einkunftsquelle".
fn verkauf(n: u8, typ: &str, a: &str, v: &str, preis: i64, ak: i64) -> Vec<(String, Value)> {
    let mut k = vec![
        (name("p23_veraeusserungs_typ", n), json!(typ)),
        (name("p23_bezeichnung", n), json!("Testobjekt")),
        (name("p23_anschaffungsdatum", n), json!(a)),
        (name("p23_veraeusserungsdatum", n), json!(v)),
        (name("p23_veraeusserungspreis", n), json!(preis)),
        (name("p23_anschaffung_herstellungskosten", n), json!(ak)),
        (name("p23_werbungskosten", n), json!(0)),
        (name("p23_afa", n), json!(0)),
    ];
    if typ == "anderes_wg" {
        k.push((name("p23_taeglicher_gebrauch", n), json!(false)));
        k.push((name("p23_einkunftsquelle", n), json!(false)));
    } else {
        k.push((name("p23_eigennutzung", n), json!("nein")));
        k.push((name("p23_gebaeude_in_frist", n), json!(false)));
    }
    k
}

fn mit(mut k: Vec<(String, Value)>, n: u8, basis: &str, wert: Value) -> Vec<(String, Value)> {
    let ziel = name(basis, n);
    match k.iter_mut().find(|(f, _)| *f == ziel) {
        Some(e) => e.1 = wert,
        None => k.push((ziel, wert)),
    }
    k
}

/// Steuerpflichtig: gekauft 15.06.2020, verkauft 10.03.2025, Gewinn 50.000 EUR.
fn pflichtig(n: u8) -> Vec<(String, Value)> {
    verkauf(n, "grundstueck", "15.06.2020", "10.03.2025", 10_000_000, 5_000_000)
}

/// Die vollstaendige Akte mit dem Kreuz "ich habe verkauft" und den gegebenen Verkaeufen.
fn akte(verkaeufe: Vec<(String, Value)>) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    setze(&mut s, "kein_p23_verkauf", json!(false));
    for (feld, wert) in verkaeufe {
        setze(&mut s, &feld, wert);
    }
    s
}

fn lauf(s: &Store) -> Result<Einreichung, EinreichFehler> {
    einreichungs_xml(s, index(), params(), "BY", Some("74931".to_owned()))
}

/// Die Eintraege der Abgabe-Sperre als `(feld_id, grund)`; leer, wenn `lauf` nicht mit `DeklarationUnvollstaendig` endet.
fn sperre(r: &Result<Einreichung, EinreichFehler>) -> Vec<(String, String)> {
    match r {
        Err(EinreichFehler::DeklarationUnvollstaendig(e)) => e
            .iter()
            .map(|x| (x.feld_id.clone(), x.grund.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Die Kz der Instanz `index` (1-basiert) der Gruppe `p23_veraeusserung` der Erklaerung.
fn kz_der_instanz(e: &Einreichung, index: u64) -> BTreeMap<String, Value> {
    e.deklaration
        .anlage_instanzen
        .iter()
        .filter(|(g, _)| g == "p23_veraeusserung")
        .flat_map(|(_, ii)| ii)
        .find(|i| i.index == index)
        .map(|i| i.felder.clone())
        .unwrap_or_default()
}

fn erwarte_einreichbar(name: &str, r: &Result<Einreichung, EinreichFehler>) {
    assert!(sperre(r).is_empty(), "{name}: sperrt schon: {r:?}");
    if schemas_da(2025) {
        assert!(r.is_ok(), "{name}: {r:?}");
    }
}

/// Kontrolle zuerst: ein steuerpflichtiger Verkauf sperrt die Abgabe nicht und steht mit allen vier Kz in der Erklaerung. Ohne sie
/// bewiese die Sperre unten nichts (ein Fall, der aus anderem Grund sperrt, liesse sie gruen).
#[test]
fn steuerpflichtiges_grundstueck_steht_mit_vier_kz_in_der_erklaerung() {
    let r = lauf(&akte(pflichtig(1)));
    erwarte_einreichbar("Grundstueck", &r);
    if let Ok(e) = &r {
        assert_eq!(
            kz_der_instanz(e, 1),
            BTreeMap::from([
                ("E0306111".to_owned(), json!("Testobjekt")),
                ("E0306201".to_owned(), json!("15.06.2020")),
                ("E0306202".to_owned(), json!("10.03.2025")),
                ("E0306801".to_owned(), json!(50_000)),
            ])
        );
        if schemas_da(2025) {
            assert!(e.xml.contains("E0306801") && e.xml.contains("Testobjekt"), "XML ohne den Verkauf");
        }
    }
}

#[test]
fn steuerpflichtiges_wirtschaftsgut_steht_mit_seinen_kz_in_der_erklaerung() {
    let v = verkauf(1, "anderes_wg", "15.02.2025", "20.08.2025", 10_000_000, 5_000_000);
    let r = lauf(&akte(v));
    erwarte_einreichbar("Wirtschaftsgut", &r);
    if let Ok(e) = &r {
        assert_eq!(
            kz_der_instanz(e, 1),
            BTreeMap::from([
                ("E0307101".to_owned(), json!("Testobjekt")),
                ("E0307201".to_owned(), json!("15.02.2025")),
                ("E0307202".to_owned(), json!("20.08.2025")),
                ("E0307701".to_owned(), json!(50_000)),
            ])
        );
    }
}

/// Die Abschreibung mindert die Anschaffungskosten: 100.000 - (80.000 - 10.000) - 5.000 Werbungskosten = 25.000 EUR. `checkESt`
/// rechnet den Gewinn aus den Einzelbetraegen nach (Spike AK0); ein anderer Wert wuerde mit rc=610001002 abgewiesen.
#[test]
fn der_gewinn_in_der_erklaerung_traegt_die_abschreibung() {
    let v = verkauf(1, "grundstueck", "15.06.2020", "10.03.2025", 10_000_000, 8_000_000);
    let v = mit(mit(v, 1, "p23_afa", json!(1_000_000)), 1, "p23_werbungskosten", json!(500_000));
    let r = lauf(&akte(v));
    erwarte_einreichbar("AfA", &r);
    if let Ok(e) = &r {
        assert_eq!(kz_der_instanz(e, 1).get("E0306801"), Some(&json!(25_000)));
    }
}

/// Ein Verlust steht als negativer Gewinn in der Erklaerung (-10.000 EUR) und sperrt nichts.
#[test]
fn ein_verlust_steht_negativ_in_der_erklaerung() {
    let v = verkauf(1, "grundstueck", "15.06.2020", "10.03.2025", 4_000_000, 5_000_000);
    let r = lauf(&akte(v));
    erwarte_einreichbar("Verlust", &r);
    if let Ok(e) = &r {
        assert_eq!(kz_der_instanz(e, 1).get("E0306801"), Some(&json!(-10_000)));
    }
}

/// Ein Gewinn von 0 steht als 0 in der Erklaerung: das Kz ist beim Grundstueck Pflicht (Spike AK0), das Schema erlaubt die 0.
#[test]
fn ein_gewinn_von_null_steht_als_null_in_der_erklaerung() {
    let v = verkauf(1, "grundstueck", "15.06.2020", "10.03.2025", 5_000_000, 5_000_000);
    let r = lauf(&akte(v));
    erwarte_einreichbar("Gewinn 0", &r);
    if let Ok(e) = &r {
        assert_eq!(kz_der_instanz(e, 1).get("E0306801"), Some(&json!(0)));
    }
}

/// Ein steuerpflichtiger Verkauf unter der Freigrenze (600 EUR) steht trotzdem in der Erklaerung: die Freigrenze wendet das
/// Finanzamt auf die Summe an, die Software meldet den Verkauf.
#[test]
fn verkauf_unter_der_freigrenze_steht_in_der_erklaerung() {
    let v = verkauf(1, "grundstueck", "15.06.2020", "10.03.2025", 1_000_000, 940_000);
    let r = lauf(&akte(v));
    erwarte_einreichbar("Freigrenze", &r);
    if let Ok(e) = &r {
        assert_eq!(kz_der_instanz(e, 1).get("E0306801"), Some(&json!(600)));
    }
}

/// Ein selbst bewohnter Verkauf sperrt die Abgabe mit EIGENEM Grund (409 `deklaration_unvollstaendig`): beide Arten der Eigennutzung.
/// Die Liste nennt genau den Verkauf, der Grund die Anlage SO und die Sperre. (Frist abgelaufen und taeglicher Gebrauch sperren nicht:
/// sie stehen gar nicht in der Erklaerung, siehe `frist_abgelaufen_und_taeglicher_gebrauch_stehen_nicht_in_der_erklaerung`.)
#[test]
fn selbst_bewohnter_verkauf_sperrt_die_abgabe_mit_eigenem_grund() {
    let bewohnt = mit(pflichtig(1), 1, "p23_eigennutzung", json!("ausschliesslich"));
    let drei_jahre = mit(pflichtig(1), 1, "p23_eigennutzung", json!("letzte_drei_jahre"));
    for (name, v) in [("selbst bewohnt", bewohnt), ("drei Jahre bewohnt", drei_jahre)] {
        let r = lauf(&akte(v));
        let e = sperre(&r);
        assert_eq!(
            e.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
            ["p23_veraeusserungspreis"],
            "{name}: die Sperre nennt genau diesen Verkauf, erhalten {r:?}"
        );
        let grund = &e.first().unwrap().1;
        for teil in ["steuerfrei", "Anlage SO", "gesperrt"] {
            assert!(grund.contains(teil), "{name}: Grund ohne `{teil}`: {grund}");
        }
    }
}

/// Der zweite Verkauf ist selbst bewohnt, der erste steuerpflichtig: die Sperre nennt den zweiten, mit dem Suffix seiner Instanz.
#[test]
fn die_sperre_nennt_den_selbst_bewohnten_verkauf_unter_seiner_instanz() {
    let mut v = pflichtig(1);
    v.extend(mit(pflichtig(2), 2, "p23_eigennutzung", json!("ausschliesslich")));
    let r = lauf(&akte(v));
    let e = sperre(&r);
    assert_eq!(
        e.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
        ["p23_veraeusserungspreis__2"],
        "{r:?}"
    );
}

/// Was die Software nicht rechnet, sperrt schon VOR der Erklaerung mit dem Grund des Verkaufs (409 `gesperrt`).
#[test]
fn kryptowert_und_unvollstaendige_angaben_sperren_vor_der_erklaerung() {
    let krypto = mit(pflichtig(1), 1, "p23_veraeusserungs_typ", json!("kryptowert"));
    match lauf(&akte(krypto)) {
        Err(EinreichFehler::Gesperrt(g)) => assert_eq!(g, Sperrgrund::P23VerkaufNichtGerechnet),
        anders => panic!("Kryptowert: {anders:?}"),
    }
    let ohne_datum: Vec<_> = pflichtig(1)
        .into_iter()
        .filter(|(f, _)| f != "p23_veraeusserungsdatum")
        .collect();
    match lauf(&akte(ohne_datum)) {
        Err(EinreichFehler::Gesperrt(g)) => assert_eq!(g, Sperrgrund::P23AngabenOffen),
        anders => panic!("Datum fehlt: {anders:?}"),
    }
}

/// Eine negative Abschreibung (hier −50.000 EUR) ist keine Angabe: sie liesse die Anschaffungskosten steigen und den Gewinn sinken
/// (100.000 − (50.000 + 50.000) = 0 statt 50.000). Sie sperrt mit `p23_angaben_offen`, ehe eine Zahl entsteht. Der Store weist ein
/// Minus beim Schreiben ab (`nicht_negativ` an `p23_afa`); diese Akte umgeht ihn (`append_roh` ohne Bindung), wie eine geladene
/// Altakte, und misst darum den Guard selbst. Gegenprobe: eine Abschreibung in Hoehe der Kosten (50.000 EUR) sperrt nicht.
#[test]
fn negative_abschreibung_sperrt_mit_angaben_offen_und_senkt_den_gewinn_nicht() {
    let negativ = mit(pflichtig(1), 1, "p23_afa", json!(-5_000_000));
    match lauf(&akte(negativ)) {
        Err(EinreichFehler::Gesperrt(g)) => assert_eq!(g, Sperrgrund::P23AngabenOffen),
        anders => panic!("negative AfA muss mit p23_angaben_offen sperren, erhalten {anders:?}"),
    }
    let gleich_den_kosten = mit(pflichtig(1), 1, "p23_afa", json!(5_000_000));
    let r = lauf(&akte(gleich_den_kosten));
    erwarte_einreichbar("AfA gleich den Kosten", &r);
    if let Ok(e) = &r {
        // 100.000 − (50.000 − 50.000) = 100.000 EUR
        assert_eq!(kz_der_instanz(e, 1).get("E0306801"), Some(&json!(100_000)));
    }
}

/// Ein Minus in einem Betragsfeld ist keine Angabe. Jede dieser Akten umgeht den Store (`append_roh` ohne Bindung), wie eine geladene
/// Altakte, und misst darum den Guard selbst; das Speichern weist ein Minus schon ab (`nicht_negativ`, `api/tests/p23_befreiungen_hermetisch.rs`).
/// Der Guard sperrt mit `p23_angaben_offen`, ehe eine Zahl entsteht. Gegenprobe: derselbe Betrag mit positivem Vorzeichen sperrt nicht.
fn erwarte_offen_bei_minus(feld: &str, minus: i64) {
    match lauf(&akte(mit(pflichtig(1), 1, feld, json!(minus)))) {
        Err(EinreichFehler::Gesperrt(g)) => assert_eq!(g, Sperrgrund::P23AngabenOffen, "{feld}"),
        anders => panic!("{feld} = {minus} muss mit p23_angaben_offen sperren, erhalten {anders:?}"),
    }
    let r = lauf(&akte(mit(pflichtig(1), 1, feld, json!(-minus))));
    assert!(!matches!(r, Err(EinreichFehler::Gesperrt(_))), "Gegenprobe {feld} = {}: sperrt schon: {r:?}", -minus);
}

/// Preis −100.000 EUR liesse den Gewinn um 200.000 EUR sinken (−100.000 − 50.000 statt +50.000): ein Verlust, der keiner ist.
#[test]
fn ein_negativer_veraeusserungspreis_sperrt_mit_angaben_offen() {
    erwarte_offen_bei_minus("p23_veraeusserungspreis", -10_000_000);
}

/// Anschaffungskosten −50.000 EUR liessen den Gewinn auf 150.000 EUR steigen. Der Guard ist `afa > kosten`: die Abschreibung ist nie
/// negativ (eigener Guard), also liegt sie bei negativen Kosten immer darueber. Eine eigene Pruefung `kosten < 0` waere nie erreichbar.
#[test]
fn negative_anschaffungskosten_sperren_mit_angaben_offen() {
    erwarte_offen_bei_minus("p23_anschaffung_herstellungskosten", -5_000_000);
}

/// Werbungskosten −5.000 EUR liessen den Gewinn um 5.000 EUR steigen (Gewinn = Preis − Kosten − Werbungskosten).
#[test]
fn negative_werbungskosten_sperren_mit_angaben_offen() {
    erwarte_offen_bei_minus("p23_werbungskosten", -500_000);
}

/// Ein Grundstueck, dessen Frist abgelaufen ist (gekauft 2014), kann ein Gebaeude, einen Ausbau oder Anbau tragen, der in den letzten
/// zehn Jahren entstand (§ 23 Abs. 1 Nr. 1 Satz 2 `EStG`; Anleitung Anlage SO, Zeilen 34 bis 42). Dieser Teil ist steuerpflichtig. Die
/// Software rechnet die Aufteilung zwischen Boden und Gebaeude nicht und sperrt mit `p23_verkauf_nicht_gerechnet`, statt den ganzen
/// Verkauf als steuerfrei zu fuehren. Gegenprobe: dasselbe Grundstueck ohne Gebaeude sperrt den Guard nicht.
#[test]
fn ein_altes_grundstueck_mit_neuem_gebaeude_sperrt_als_nicht_gerechnet() {
    let alt = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    let r = lauf(&akte(alt.clone()));
    assert!(!matches!(r, Err(EinreichFehler::Gesperrt(_))), "Gegenprobe: ohne Gebaeude sperrt der Guard nicht: {r:?}");
    match lauf(&akte(mit(alt, 1, "p23_gebaeude_in_frist", json!(true)))) {
        Err(EinreichFehler::Gesperrt(g)) => assert_eq!(g, Sperrgrund::P23VerkaufNichtGerechnet),
        anders => panic!("altes Grundstueck mit neuem Gebaeude muss mit p23_verkauf_nicht_gerechnet sperren, erhalten {anders:?}"),
    }
}

/// Das Gebaeude sperrt nur, wo es etwas aendert: laeuft die Frist noch, ist das ganze Grundstueck steuerpflichtig; ist es selbst
/// bewohnt, sind Gebaeude und Boden frei; ein anderes Wirtschaftsgut hat keine Gebaeudefrage und liest das Feld nicht.
#[test]
fn das_gebaeude_sperrt_nur_wo_es_den_steuerstatus_aendert() {
    let laufend = mit(pflichtig(1), 1, "p23_gebaeude_in_frist", json!(true));
    let r = lauf(&akte(laufend));
    erwarte_einreichbar("Frist laeuft, Gebaeude ja", &r);
    if let Ok(e) = &r {
        assert_eq!(kz_der_instanz(e, 1).get("E0306801"), Some(&json!(50_000)), "das ganze Grundstueck zaehlt");
    }
    let alt = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    let bewohnt = mit(mit(alt, 1, "p23_eigennutzung", json!("ausschliesslich")), 1, "p23_gebaeude_in_frist", json!(true));
    let r = lauf(&akte(bewohnt));
    assert!(!matches!(r, Err(EinreichFehler::Gesperrt(_))), "selbst bewohnt: Gebaeude ja sperrt nicht: {r:?}");
    let wg = verkauf(1, "anderes_wg", "10.01.2024", "11.01.2025", 10_000_000, 5_000_000);
    let r = lauf(&akte(mit(wg, 1, "p23_gebaeude_in_frist", json!(true))));
    assert!(!matches!(r, Err(EinreichFehler::Gesperrt(_))), "anderes Wirtschaftsgut liest das Feld nicht: {r:?}");
}

/// Ein Grundstueck ohne Antwort zum Gebaeude ist eine offene Angabe, keine Antwort "nein".
#[test]
fn grundstueck_ohne_antwort_zum_gebaeude_sperrt_mit_angaben_offen() {
    let ohne: Vec<_> = pflichtig(1).into_iter().filter(|(f, _)| f != "p23_gebaeude_in_frist").collect();
    match lauf(&akte(ohne)) {
        Err(EinreichFehler::Gesperrt(g)) => assert_eq!(g, Sperrgrund::P23AngabenOffen),
        anders => panic!("fehlende Gebaeude-Antwort muss mit p23_angaben_offen sperren, erhalten {anders:?}"),
    }
}

/// Frist abgelaufen und Gegenstand des taeglichen Gebrauchs sind keine privaten Veraeusserungsgeschaefte der Anlage SO: die
/// Anleitung nimmt Verkaeufe nach Ablauf der Frist aus den Zeilen 34 bis 57 heraus ("Private Veraeusserungsgeschaefte sind" nur Verkaeufe
/// innerhalb der Frist) und sagt zum taeglichen Gebrauch: "muessen Sie hier nicht angeben". Die Abgabe sperrt darum nicht, und der
/// Verkauf steht mit keinem Kz im XML. Gegenprobe: `steuerpflichtiges_grundstueck_steht_mit_vier_kz_in_der_erklaerung`.
#[test]
fn frist_abgelaufen_und_taeglicher_gebrauch_stehen_nicht_in_der_erklaerung() {
    let frist = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    let gebrauch = mit(
        verkauf(1, "anderes_wg", "15.02.2025", "20.08.2025", 10_000_000, 5_000_000),
        1,
        "p23_taeglicher_gebrauch",
        json!(true),
    );
    for (name, v) in [("Frist abgelaufen", frist), ("taeglicher Gebrauch", gebrauch)] {
        let r = lauf(&akte(mit(v, 1, "p23_bezeichnung", json!("Freier Verkauf"))));
        erwarte_einreichbar(name, &r);
        if let Ok(e) = &r {
            assert!(kz_der_instanz(e, 1).is_empty(), "{name}: steht mit {:?} in der Erklaerung", kz_der_instanz(e, 1));
            if schemas_da(2025) {
                for kz in ["Freier Verkauf", "E0306801", "E0307701"] {
                    assert!(!e.xml.contains(kz), "{name}: `{kz}` steht im XML");
                }
            }
        }
    }
}

/// Ein Verkauf ausserhalb der Erklaerung nimmt nur sich selbst mit: die anderen Verkaeufe stehen unter ihrem Index, und eine andere
/// Instanz-Gruppe mit demselben Index (`hh_handwerker_betrag__2` in der Akte) bleibt unberuehrt.
#[test]
fn der_verkauf_ausserhalb_der_erklaerung_laesst_die_anderen_stehen() {
    let frei = |n: u8| mit(verkauf(n, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000), n, "p23_bezeichnung", json!("Freier Verkauf"));
    let handwerker_2 = |e: &Einreichung| {
        e.deklaration
            .anlage_instanzen
            .iter()
            .any(|(g, ii)| g == "hh_handwerker" && ii.iter().any(|i| i.index == 2))
    };
    // Der erste Verkauf ist frei, der zweite steuerpflichtig.
    let mut v = frei(1);
    v.extend(pflichtig(2));
    let r = lauf(&akte(v));
    erwarte_einreichbar("erster frei", &r);
    if let Ok(e) = &r {
        assert!(kz_der_instanz(e, 1).is_empty(), "der freie Verkauf steht in der Erklaerung: {:?}", kz_der_instanz(e, 1));
        assert_eq!(kz_der_instanz(e, 2).get("E0306801"), Some(&json!(50_000)), "der steuerpflichtige Verkauf fehlt");
        assert!(handwerker_2(e), "die Instanz 2 einer anderen Gruppe ging mit verloren");
    }
    // Umgekehrt: der erste steuerpflichtig, der zweite frei.
    let mut v = pflichtig(1);
    v.extend(frei(2));
    let r = lauf(&akte(v));
    erwarte_einreichbar("zweiter frei", &r);
    if let Ok(e) = &r {
        assert_eq!(kz_der_instanz(e, 1).get("E0306801"), Some(&json!(50_000)), "der steuerpflichtige Verkauf fehlt");
        assert!(kz_der_instanz(e, 2).is_empty(), "der freie Verkauf steht in der Erklaerung: {:?}", kz_der_instanz(e, 2));
        assert!(handwerker_2(e), "die Instanz 2 einer anderen Gruppe ging mit verloren");
    }
}

/// `p23_steuerfreier_verkauf` nennt nur BESTAETIGTE steuerfreie Verkaeufe: ein Verkauf mit vorlaeufigem Preis ist offen, nicht fertig, und
/// die Abgabe sperrt dann mit dem Eintrag der Vorlaeufigkeit. Ueber `einreichungs_xml` ist der Zweig nicht zu erreichen, weil der Guard
/// davor jede vorlaeufige Instanz mit `p23_angaben_offen` sperrt; darum ruft der Test die Funktion selbst. Gegenprobe: derselbe Verkauf
/// bestaetigt wird genannt.
#[test]
fn der_steuerfreie_verkauf_zaehlt_nur_bestaetigt() {
    let frist = mit(pflichtig(1), 1, "p23_eigennutzung", json!("ausschliesslich"));
    let nennt = |vorlaeufig: Option<&str>| {
        let mut s = akte(Vec::new());
        for (feld, wert) in &frist {
            let zustand = if vorlaeufig == Some(feld.as_str()) { Zustand::Vorlaeufig } else { Zustand::Bestaetigt };
            setze_mit(&mut s, feld, wert.clone(), zustand);
        }
        let q = Instanzquelle { store: Some(&s), bindung: Some(index()), nur_bestaetigt: false };
        p23_steuerfreier_verkauf(&q, Vz::Vz2025).unwrap()
    };
    assert_eq!(nennt(None).as_deref(), Some("p23_veraeusserungspreis"), "Gegenprobe: bestaetigt wird genannt");
    assert_eq!(nennt(Some("p23_veraeusserungspreis")), None, "ein vorlaeufiger Preis macht den Verkauf offen");
    assert_eq!(nennt(Some("p23_anschaffungsdatum")), None, "ein vorlaeufiges Datum auch");
}
