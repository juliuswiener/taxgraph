//! Privatverkauf (Abweichung Nr. 54): was die Erklaerung traegt und wann die Abgabe sperrt. Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Der Bescheid rechnet einen privaten Verkauf (`api/tests/p23_befreiungen_hermetisch.rs`). Die Erklaerung traegt
//! Verkaeufe innerhalb der Frist in die Anlage SO: Bezeichnung, beide Daten und Gewinn unter den Kz der Art (Grundstueck
//! E0306111/E0306201/E0306202/E0306801, anderes Wirtschaftsgut E0307101/E0307201/E0307202/E0307701). Ein Verkauf mit abgelaufener
//! Frist (auch ein selbst bewohnter) und ein gebrauchter Gegenstand des taeglichen Gebrauchs steht NICHT drin: die Anleitung zur Anlage
//! SO nimmt beide aus den Zeilen heraus (`sources/bfinv/anleitungen/anl_so_2025.txt`), die Abgabe sperrt nicht. Ein selbst bewohntes
//! Grundstueck innerhalb der Frist steht in der Zeile 36 mit Haken, Zeitraum und Flaeche (E0306301 bis E0306303) und dem Gewinn 0;
//! fehlen Flaeche oder Zeitraum, sperrt der Guard mit `p23_angaben_offen`.
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
use bescheid::privatverkauf::p23_ohne_verkaeufe_ausserhalb_der_erklaerung;
use bescheid::testhilfe::{index, params};
use bescheid::Instanzquelle;
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Sperrgrund, Vz, Zustand};
use elster::testhilfe::schemas_da;
use serde_json::{json, Value};
use store::{Abweisung, AbweisungRoh, BindungNachschlag, NeuesEventRoh, Signal, Store};

/// Ein bestaetigter Wert (zwei Signale).
fn setze(s: &mut Store, feld: &str, wert: Value) {
    setze_mit(s, feld, wert, Zustand::Bestaetigt);
}

/// Ein Wert mit dem Zustand `bestaetigt` (zwei Signale) oder `vorlaeufig` (ohne zweites Signal).
fn setze_mit(s: &mut Store, feld: &str, wert: Value, zustand: Zustand) {
    let bestaetigt = zustand == Zustand::Bestaetigt;
    let leer = HashMap::new();
    let mut neu = NeuesEventRoh {
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
    };
    match s.append_roh(&neu, None, BindungNachschlag::neu(&leer)) {
        Ok(_) => {}
        // Nr. 70: ein Feld, das die Akte schon traegt (zum Beispiel `veranlagung`), wird ersetzt statt abgewiesen.
        Err(AbweisungRoh::Store(Abweisung::AktivesEventVorhanden { aktives_event, .. })) => {
            neu.ersetzt = Some(aktives_event.to_string());
            s.append_roh(&neu, None, BindungNachschlag::neu(&leer)).unwrap();
        }
        Err(e) => panic!("{feld}: {e:?}"),
    }
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

/// Nimmt das Feld `basis` der Instanz `n` heraus (nie eingetragen).
fn ohne(mut k: Vec<(String, Value)>, n: u8, basis: &str) -> Vec<(String, Value)> {
    let ziel = name(basis, n);
    k.retain(|(f, _)| *f != ziel);
    k
}

/// Steuerpflichtig: gekauft 15.06.2020, verkauft 10.03.2025, Gewinn 50.000 EUR.
fn pflichtig(n: u8) -> Vec<(String, Value)> {
    verkauf(n, "grundstueck", "15.06.2020", "10.03.2025", 10_000_000, 5_000_000)
}

/// Wie `pflichtig`, aber selbst bewohnt: Eigennutzung `art`, 120 m² Wohnflaeche, bewohnt von `von` bis `bis`.
fn bewohnt(n: u8, art: &str, von: &str, bis: &str) -> Vec<(String, Value)> {
    let k = mit(pflichtig(n), n, "p23_eigennutzung", json!(art));
    let k = mit(k, n, "p23_wohnflaeche", json!(120));
    let k = mit(k, n, "p23_eigennutzung_von", json!(von));
    mit(k, n, "p23_eigennutzung_bis", json!(bis))
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

/// Ein selbst bewohntes Grundstueck innerhalb der Frist steht in der Zeile 36 der Anlage SO: Haken, Zeitraum von-bis und Flaeche in m²,
/// dazu der Gewinn 0 (der Verkauf ist steuerfrei; der volle Gewinn waere 50.000 EUR). Beide Arten der Eigennutzung. `checkESt` nimmt
/// genau diese Zeile an (Fall Gx7, `checkest_blockmatrix.rs`). Gegenprobe: `steuerpflichtiges_grundstueck_steht_mit_vier_kz_in_der_erklaerung`.
#[test]
fn selbst_bewohntes_grundstueck_steht_mit_haken_zeitraum_flaeche_und_gewinn_null_in_der_erklaerung() {
    for (name, art, von) in [
        ("selbst bewohnt", "ausschliesslich", "15.06.2020"),
        ("drei Jahre bewohnt", "letzte_drei_jahre", "01.01.2023"),
    ] {
        let r = lauf(&akte(bewohnt(1, art, von, "10.03.2025")));
        erwarte_einreichbar(name, &r);
        if let Ok(e) = &r {
            assert_eq!(
                kz_der_instanz(e, 1),
                BTreeMap::from([
                    ("E0306111".to_owned(), json!("Testobjekt")),
                    ("E0306201".to_owned(), json!("15.06.2020")),
                    ("E0306202".to_owned(), json!("10.03.2025")),
                    ("E0306301".to_owned(), json!(true)),
                    ("E0306302".to_owned(), json!(format!("{von}-10.03.2025"))),
                    ("E0306303".to_owned(), json!(120)),
                    ("E0306801".to_owned(), json!(0)),
                ]),
                "{name}"
            );
            if schemas_da(2025) {
                for kz in ["E0306301", "E0306302", "E0306303"] {
                    assert!(e.xml.contains(kz), "{name}: `{kz}` fehlt im XML");
                }
            }
        }
    }
}

/// Ein selbst bewohnter Verkauf innerhalb der Frist ohne Flaeche oder ohne lesbaren Zeitraum ist eine offene Angabe: die Zeile 36 waere
/// unvollstaendig, `checkESt` wiese sie ab (Fall Gc1, rc=610001002). Er sperrt mit `p23_angaben_offen`, ehe eine Zahl entsteht. Gegenprobe:
/// die vollstaendige Akte und ein Zeitraum von einem Tag (von = bis) sperren nicht.
#[test]
fn selbst_bewohnter_verkauf_ohne_flaeche_oder_zeitraum_sperrt_mit_angaben_offen() {
    let voll = bewohnt(1, "ausschliesslich", "15.06.2020", "10.03.2025");
    erwarte_einreichbar("vollstaendig", &lauf(&akte(voll.clone())));
    let ein_tag = mit(mit(voll.clone(), 1, "p23_eigennutzung_von", json!("10.03.2025")), 1, "p23_eigennutzung_bis", json!("10.03.2025"));
    erwarte_einreichbar("ein Tag", &lauf(&akte(ein_tag)));
    let faelle = [
        ("ohne Flaeche", ohne(voll.clone(), 1, "p23_wohnflaeche")),
        ("Flaeche 0", mit(voll.clone(), 1, "p23_wohnflaeche", json!(0))),
        ("Flaeche minus", mit(voll.clone(), 1, "p23_wohnflaeche", json!(-5))),
        ("Flaeche als Text", mit(voll.clone(), 1, "p23_wohnflaeche", json!("120"))),
        ("ohne Beginn", ohne(voll.clone(), 1, "p23_eigennutzung_von")),
        ("ohne Ende", ohne(voll.clone(), 1, "p23_eigennutzung_bis")),
        ("Beginn kein Datum", mit(voll.clone(), 1, "p23_eigennutzung_von", json!("31.04.2020"))),
        ("Ende kein Datum", mit(voll.clone(), 1, "p23_eigennutzung_bis", json!("2025-03-10"))),
        ("Ende vor Beginn", mit(voll.clone(), 1, "p23_eigennutzung_von", json!("11.03.2025"))),
    ];
    for (name, v) in faelle {
        match lauf(&akte(v)) {
            Err(EinreichFehler::Gesperrt(g)) => assert_eq!(g, Sperrgrund::P23AngabenOffen, "{name}"),
            anders => panic!("{name}: muss mit p23_angaben_offen sperren, erhalten {anders:?}"),
        }
    }
}

/// Die Frist begrenzt auch den selbst bewohnten Verkauf: die Anlage SO traegt nur Verkaeufe innerhalb von zehn Jahren (Zeilen 34 bis 42).
/// Gekauft am 10.03.2015 und verkauft am 10.03.2025 liegt am letzten Tag der Frist und braucht die Zeile 36. Gekauft am 09.03.2015
/// liegt einen Tag danach: der Verkauf steht nicht in der Erklaerung und braucht weder Flaeche noch Zeitraum.
#[test]
fn selbst_bewohntes_grundstueck_nach_ablauf_der_frist_steht_nicht_in_der_erklaerung() {
    let alt = |kauf: &str| {
        let v = verkauf(1, "grundstueck", kauf, "10.03.2025", 10_000_000, 5_000_000);
        mit(mit(v, 1, "p23_eigennutzung", json!("ausschliesslich")), 1, "p23_bezeichnung", json!("Freier Verkauf"))
    };
    let r = lauf(&akte(alt("09.03.2015")));
    erwarte_einreichbar("einen Tag nach der Frist", &r);
    if let Ok(e) = &r {
        assert!(kz_der_instanz(e, 1).is_empty(), "steht mit {:?} in der Erklaerung", kz_der_instanz(e, 1));
        if schemas_da(2025) {
            for kz in ["Freier Verkauf", "E0306301", "E0306801"] {
                assert!(!e.xml.contains(kz), "`{kz}` steht im XML");
            }
        }
    }
    match lauf(&akte(alt("10.03.2015"))) {
        Err(EinreichFehler::Gesperrt(g)) => assert_eq!(g, Sperrgrund::P23AngabenOffen, "letzter Tag der Frist ohne Zeile 36"),
        anders => panic!("am letzten Tag der Frist fehlt die Zeile 36: erwartet p23_angaben_offen, erhalten {anders:?}"),
    }
    let mit_zeile = mit(mit(mit(alt("10.03.2015"), 1, "p23_wohnflaeche", json!(120)), 1, "p23_eigennutzung_von", json!("10.03.2015")), 1, "p23_eigennutzung_bis", json!("10.03.2025"));
    let r = lauf(&akte(mit_zeile));
    erwarte_einreichbar("letzter Tag der Frist mit Zeile 36", &r);
    if let Ok(e) = &r {
        assert_eq!(kz_der_instanz(e, 1).get("E0306301"), Some(&json!(true)));
    }
}

/// Jede Instanz traegt ihre eigene Zeile 36, und nur die selbst bewohnte: Instanz 1 ist steuerpflichtig (Gewinn voll, kein Haken), Instanz 2
/// selbst bewohnt (Suffix `__2`), Instanz 3 ein anderes Wirtschaftsgut mit liegengebliebener Eigennutzung (die Frage gilt nur fuer
/// Grundstuecke, die Zeile 36 gibt es dort nicht).
#[test]
fn nur_das_selbst_bewohnte_grundstueck_traegt_die_zeile_36_und_jede_instanz_ihre_eigene() {
    let mut v = pflichtig(1);
    v.extend(bewohnt(2, "letzte_drei_jahre", "01.01.2023", "09.03.2025"));
    let wg = verkauf(3, "anderes_wg", "15.02.2025", "20.08.2025", 10_000_000, 5_000_000);
    v.extend(mit(mit(mit(mit(wg, 3, "p23_eigennutzung", json!("ausschliesslich")), 3, "p23_wohnflaeche", json!(80)), 3, "p23_eigennutzung_von", json!("01.01.2025")), 3, "p23_eigennutzung_bis", json!("02.01.2025")));
    let r = lauf(&akte(v));
    erwarte_einreichbar("drei Verkaeufe", &r);
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
        let zwei = kz_der_instanz(e, 2);
        assert_eq!(zwei.get("E0306302"), Some(&json!("01.01.2023-09.03.2025")), "{zwei:?}");
        assert_eq!(zwei.get("E0306801"), Some(&json!(0)), "{zwei:?}");
        assert_eq!(
            kz_der_instanz(e, 3).keys().map(String::as_str).collect::<Vec<_>>(),
            ["E0307101", "E0307201", "E0307202", "E0307701"],
            "das Wirtschaftsgut traegt keine Zeile 36"
        );
    }
}

/// Teilnutzung bleibt gesperrt, auch mit Flaeche und Zeitraum: sie braucht die Aufteilung nach dem BMF-Schreiben vom 17.06.2020, das nicht im
/// Korpus steht. Die Zeile 36 allein wuerde sie als vollstaendig selbst bewohnt melden.
#[test]
fn teilnutzung_bleibt_gesperrt_auch_mit_flaeche_und_zeitraum() {
    match lauf(&akte(bewohnt(1, "teilweise", "15.06.2020", "10.03.2025"))) {
        Err(EinreichFehler::Gesperrt(g)) => assert_eq!(g, Sperrgrund::P23VerkaufNichtGerechnet),
        anders => panic!("Teilnutzung muss mit p23_verkauf_nicht_gerechnet sperren, erhalten {anders:?}"),
    }
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

/// Der Filter nimmt nur BESTAETIGTE Verkaeufe aus dem Snapshot: ein Verkauf mit vorlaeufigem Preis ist offen, nicht fertig, und bleibt
/// stehen. Ueber `einreichungs_xml` ist der Zweig nicht zu erreichen, weil der Guard davor jede vorlaeufige Instanz mit `p23_angaben_offen`
/// sperrt; darum ruft der Test die Funktion selbst. Gegenprobe: derselbe Verkauf bestaetigt faellt heraus.
#[test]
fn der_filter_nimmt_nur_bestaetigte_verkaeufe_aus_dem_snapshot() {
    let frist_abgelaufen = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    let bleibt = |vorlaeufig: Option<&str>| {
        let mut s = akte(Vec::new());
        for (feld, wert) in &frist_abgelaufen {
            let zustand = if vorlaeufig == Some(feld.as_str()) { Zustand::Vorlaeufig } else { Zustand::Bestaetigt };
            setze_mit(&mut s, feld, wert.clone(), zustand);
        }
        let (mut felder, _) = s.materialisiere(None).unwrap();
        let q = Instanzquelle { store: Some(&s), bindung: Some(index()), nur_bestaetigt: false };
        p23_ohne_verkaeufe_ausserhalb_der_erklaerung(&mut felder, &q, Vz::Vz2025).unwrap();
        felder.contains_key("p23_veraeusserungspreis")
    };
    assert!(!bleibt(None), "Gegenprobe: der bestaetigte Verkauf mit abgelaufener Frist faellt heraus");
    assert!(bleibt(Some("p23_veraeusserungspreis")), "ein vorlaeufiger Preis macht den Verkauf offen: er bleibt stehen");
    assert!(bleibt(Some("p23_anschaffungsdatum")), "ein vorlaeufiges Datum auch");
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Mehrere Verkaeufe DERSELBEN Person im XML (Python: `tests/test_p23_mehrfachverkauf_bricht_so_maxoccurs.py`, Gegenversuch Nr. 69 `P1`).
//
// Das Schema erlaubt `<SO>` nur EINMAL (`maxOccurs="1"`, E10-2025.xsd:8298), anders als die Anlagen mit einem Block je Person
// (`maxOccurs="2"`). Mehrere Verkaeufe einer Person stehen als `<Einz>` (`maxOccurs="99"`) in EINEM `<Grdst>`. Ohne den Eintrag
// `("p23_veraeusserung", "Einz")` in `INSTANZ_CONTAINER_TIEFER` (`elster/src/xml.rs`) legte der Schreiber je Verkauf ein eigenes `<SO>` an:
// `xmllint` lehnt das hart ab, ERiC erst in der Plausibilitaetspruefung (rc=610001002, zuGrosseLfdNummer). Die Personenachse (der Verkauf
// des Partners als zweites `<Grdst>`) beruehrt das nicht.

fn zaehle(xml: &str, tag: &str) -> usize {
    xml.matches(&format!("<{tag}>")).count() + xml.matches(&format!("<{tag} ")).count()
}

/// (`<SO>` im Dokument, `<Grdst>` und `<Einz>` zwischen dem ersten `<SO>` und dem letzten `</SO>`): die beiden letzten nur in der Anlage SO,
/// denn `<Einz>` kehrt im Schema an rund hundert anderen Stellen wieder.
fn so_form(xml: &str) -> (usize, usize, usize) {
    let so = zaehle(xml, "SO");
    let (Some(von), Some(bis)) = (xml.find("<SO>"), xml.rfind("</SO>")) else {
        return (so, 0, 0);
    };
    let block = &xml[von..bis];
    (so, zaehle(block, "Grdst"), zaehle(block, "Einz"))
}

/// Drei verschiedene steuerpflichtige Grundstuecke (Gewinn 50.000, 40.000, 20.000 EUR), alle Person A.
fn grundstuecke(n: u8) -> Vec<(String, Value)> {
    let mut v = pflichtig(1);
    if n >= 2 {
        v.extend(verkauf(2, "grundstueck", "15.06.2020", "10.03.2025", 13_000_000, 9_000_000));
    }
    if n >= 3 {
        v.extend(verkauf(3, "grundstueck", "15.06.2020", "10.03.2025", 8_000_000, 6_000_000));
    }
    v
}

#[test]
fn ein_zwei_drei_verkaeufe_einer_person_stehen_in_einem_so_und_einem_grdst_mit_ebenso_vielen_einz() {
    for (n, gewinne) in [(1_u8, &[50_000_i64][..]), (2, &[50_000, 40_000]), (3, &[50_000, 40_000, 20_000])] {
        let r = lauf(&akte(grundstuecke(n)));
        erwarte_einreichbar(&format!("{n} Verkaeufe"), &r);
        let Ok(e) = &r else {
            continue;
        };
        for (i, gewinn) in gewinne.iter().enumerate() {
            let i = u64::try_from(i + 1).unwrap();
            assert_eq!(kz_der_instanz(e, i).get("E0306801"), Some(&json!(gewinn)), "Verkauf {i} von {n}: der Gewinn fehlt in der Erklaerung");
        }
        // Nur mit Schema baut der Schreiber ein XML; ohne (CI) bleibt `r` ein Fehler und die Form ungeprueft (stderr nennt den Verzicht).
        if schemas_da(2025) {
            let nz = usize::from(n);
            assert_eq!(so_form(&e.xml), (1, 1, nz), "{n} Verkaeufe derselben Person: erwartet je ein <SO> und <Grdst> und {n} <Einz>");
            let (ok, meldung) = elster::validiere_xsd_text(e.xml.as_bytes(), Vz::Vz2025);
            assert!(ok, "{n} Verkaeufe derselben Person: XML nicht schema-valide: {meldung}");
        }
    }
}

// ---------------------------------------------------------------- Ehepaar: der Verkauf gehoert keiner Person, also sperrt die Abgabe

/// Die Akte als Zusammenveranlagung: Person B bestaetigt den Kegel der Scheibe `gesamt` (Lohn und Kapital 0), damit der Guard nicht
/// schon an `partner_kegel_offen` endet und allein der Verkauf entscheidet.
fn akte_ehepaar(verkaeufe: Vec<(String, Value)>) -> Store {
    let mut s = akte(verkaeufe);
    setze(&mut s, "veranlagung", json!("zusammen"));
    for feld in [
        "bruttoarbeitslohn_partner",
        "kap_kapitalertraege_partner",
        "kap_gewinn_aktien_partner",
        "kap_gewinn_sonstige_partner",
        "kap_verlust_aktien_partner",
        "kap_verlust_sonstige_partner",
    ] {
        setze(&mut s, feld, json!(0));
    }
    s
}

/// Die Kz der Gruppe `p23_veraeusserung`, die der Schreiber in `person_b` (die zweite Anlage des Ehegatten) legt. Leer heisst: der Verkauf
/// wird nur ueber den Instanzindex geroutet, kein Feld sagt, wessen Verkauf es ist. Ohne Ring-Werte und ohne Guard gelesen, wie
/// `elster::deklariere` den Snapshot sieht.
fn p23_kz_in_person_b(s: &Store) -> (Vec<String>, Vec<String>) {
    let felder = bescheid::testhilfe::felder(s);
    let d = elster::deklariere(&felder, index(), 2025, None).unwrap();
    let p23: Vec<String> = d
        .anlage_instanzen
        .iter()
        .filter(|(g, _)| g == "p23_veraeusserung")
        .flat_map(|(_, ii)| ii)
        .flat_map(|i| i.felder.keys().cloned())
        .collect();
    let in_b = p23
        .iter()
        .filter(|kz| d.person_b.contains_key(kz.as_str()))
        .cloned()
        .collect();
    (p23, in_b)
}

/// `test_p23_ring_sperre_kopplung_an_personen_routing.py` (Nr. 70). Der Python-Waechter sagt: solange der Schreiber einen Verkauf nur nach
/// Instanzindex einer Person zuordnet (kein Personenfeld, kein Kz in `person_b`), muss die Ring-Sperre halten, sonst reicht ein Ehepaar
/// den Verkauf des Partners unter der eigenen Person ein. In Rust ist das die Sperre `p23_verkauf_nicht_gerechnet` fuer jedes Ehepaar mit
/// bejahtem Verkauf (`p23_sperrgrund`, Abweichung Nr. 54; Julius: sie faellt nur auf sein Wort). Gemessen wird am Abgabeweg
/// (`einreichungs_xml`): der Hauptweg `GET /ergebnis` hat schon `z2_rentner_zusammen_verkauf_bejaht_sperrt_p23_verkauf_nicht_gerechnet`,
/// aber nur fuer `rentner_gesamt`; die Scheibe `gesamt` und die Abgabe hatten kein Ehepaar-Gegenstueck.
///
/// Kontrolle zuerst: derselbe Verkauf als Einzelveranlagung ist einreichbar, die Sperre unten ist also die des Ehepaars und nicht die des
/// Verkaufs. Ursache gemessen: der Verkauf steht unter `anlage_instanzen`, kein Kz in `person_b`. Faellt die Sperre, ohne dass das Routing
/// einen Ehegatten kennt, ist der Wert falsch (Geld: der Verkauf des Partners steht bei Person A).
#[test]
fn ein_ehepaar_mit_verkauf_sperrt_die_abgabe_solange_der_verkauf_keiner_person_zugeordnet_wird() {
    erwarte_einreichbar("einzel", &lauf(&akte(pflichtig(1))));

    let ehe = akte_ehepaar(pflichtig(1));
    let (p23, in_b) = p23_kz_in_person_b(&ehe);
    assert!(
        !p23.is_empty(),
        "KONTROLLE: der Verkauf steht mit keinem Kz unter anlage_instanzen['p23_veraeusserung'], die Ursachen-Messung sagt nichts"
    );
    let r = lauf(&ehe);
    let gesperrt = matches!(&r, Err(EinreichFehler::Gesperrt(Sperrgrund::P23VerkaufNichtGerechnet)));
    assert!(
        gesperrt || !in_b.is_empty(),
        "KOPPLUNG GEBROCHEN: ein Ehepaar mit bejahtem Verkauf wird nicht mit p23_verkauf_nicht_gerechnet gesperrt ({r:?}), und der Schreiber \
         legt keinen Verkauf in person_b ({p23:?}): der Verkauf des Partners wuerde unter Person A eingereicht"
    );
}
