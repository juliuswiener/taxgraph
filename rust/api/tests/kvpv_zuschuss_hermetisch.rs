//! Steuerfreie Arbeitgeberzuschuesse zur Kranken- und Pflegeversicherung ueber HTTP (Abweichung Nr. 92): die Scheiben nehmen die drei
//! Zuschussfelder je Person an, der Zuschuss mindert die abziehbaren Basisbeitraege, die Erklaerung traegt ihn. Im Standardlauf, ohne
//! `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Wer von seinem Arbeitgeber einen steuerfreien Zuschuss zur Kranken- oder Pflegeversicherung bekommt, darf nur den
//! selbst getragenen Teil der Beitraege als Sonderausgabe abziehen (§ 10 Abs. 2 Satz 1 Nr. 1 `EStG`). Die App fragte den vollen Beitrag
//! und kannte keinen Zuschuss: wer den vollen Beitrag eintrug (freiwillig gesetzlich Versicherte, Privatversicherte, Selbstzahler mit
//! Beitragsbescheinigung), zog zu viel ab, die Steuer fiel zu niedrig aus.
//!
//! **Warum es zaehlt.** Handrechnung des Audits `kv-pv-zuschuesse-werden-nie-abgezogen-handrechnung-2026-10-19`: 549 bis 1.470 Euro zu
//! wenig Steuer in drei Beispielfaellen (alleinstehend, nur Lohn). Die Richtung ist die Nachzahlung.
//!
//! **Wo es sitzt.** Der Basisbeitrag einer Person (`bescheid/src/abzuege.rs`, eine Funktion fuer `an_gesamt`, `gesamt` und
//! `rentner_gesamt`), die Felder in `bindung_n_vor_gwg.yaml` (Person A) und `bindung_an_gesamt.yaml` (Ehegatte), die Kz E2003705,
//! E2003807, E2003907 in der Erklaerung (Anlage Vorsorgeaufwand, `Stfr_AG_Zusch`, je Person).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus `params/` und dem Gesetzestext gerechnet, nie aus dem Rust-Code gelesen
//! (Skript `handrechnung.py` im Auftragsordner, der Rechenweg steht je Test unten). Je Fall: Lohn − Arbeitnehmer-Pauschbetrag 1.230 −
//! (Basisbeitrag KV + PV − Zuschuss, nie unter 0) − Sonderausgaben-Pauschbetrag 36 (Abweichung Nr. 78: neben der Vorsorge) = zvE, darauf
//! der Tarif § 32a (Zusammenveranlagung: Splitting, 2 × Steuer der Haelfte). Die Steuer ist die festzusetzende Einkommensteuer; der
//! Solidaritaetszuschlag steht daneben und zaehlt hier nicht mit.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

type Paare = Vec<(&'static str, Value)>;

/// Die drei Zuschussfelder von Person A (`LStB` Nr. 24 a, 24 b, 24 c) und des Ehegatten.
const ZUSCHUSS_A: [&str; 3] = ["zuschuss_ag_kv_gesetzlich", "zuschuss_ag_kv_privat", "zuschuss_ag_pv"];
const ZUSCHUSS_B: [&str; 3] =
    ["zuschuss_ag_kv_gesetzlich_partner", "zuschuss_ag_kv_privat_partner", "zuschuss_ag_pv_partner"];

struct Dienst {
    zustand: Zustand,
    token: String,
    _tmp: tempfile::TempDir,
}

fn dienst() -> Dienst {
    let tmp = tempfile::tempdir().unwrap();
    let konfig = Konfig {
        wurzel: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        faelle: tmp.path().join("faelle"),
        audit_dir: tmp.path().join("faelle"),
    };
    let auth = Auth::neu("testgeheimnis".into(), tmp.path().join("users.json"), Some(konfig.audit_pfad()));
    let zustand = Zustand::neu(konfig, auth);
    let token = zustand.auth.stelle_aus("alice").unwrap();
    Dienst { zustand, token, _tmp: tmp }
}

async fn sende(d: &Dienst, methode: &str, pfad: &str, body: Option<&Value>) -> (u16, Value) {
    let text = body.map(ToString::to_string);
    let mut b = Request::builder().method(methode).uri(pfad).header("authorization", format!("Bearer {}", d.token));
    if let Some(t) = &text {
        b = b.header("content-type", "application/json").header("content-length", t.len().to_string());
    }
    let req = b.body(text.map_or_else(Body::empty, Body::from)).unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (teile.status.as_u16(), serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// Ein Event ueber die echte Route `POST /event` (Nutzer-Klick, bestaetigt); liefert Status und Antwort.
async fn event_post(d: &Dienst, feld: &str, wert: &Value) -> (u16, Value) {
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    sende(d, "POST", "/fall/kvz/event", Some(&rumpf)).await
}

/// Ein leerer Fall der Scheibe, VZ 2025.
async fn neuer_fall(scheibe: &str) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "kvz", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    d
}

/// Ein Fall der Scheibe mit allen `paare`, jedes ueber `POST /event` (muss 201 sein).
async fn fall_mit(scheibe: &str, paare: &[(&'static str, Value)]) -> Dienst {
    let d = neuer_fall(scheibe).await;
    for (feld, wert) in paare {
        let (status, antwort) = event_post(&d, feld, wert).await;
        assert_eq!(status, 201, "POST /event {feld} auf {scheibe}: {antwort}");
    }
    d
}

async fn ergebnis(scheibe: &str, paare: &[(&'static str, Value)]) -> Value {
    let d = fall_mit(scheibe, paare).await;
    let (status, antwort) = sende(&d, "GET", "/fall/kvz/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// Die Feld-IDs der Fragen, die der Fall noch stellt (`GET /fragen`).
async fn fragen_ids(d: &Dienst) -> Vec<String> {
    let (status, fragen) = sende(d, "GET", "/fall/kvz/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    fragen["fragen"].as_array().unwrap().iter().map(|q| q["feld_id"].as_str().unwrap().to_owned()).collect()
}

fn zahl(a: &Value) -> i64 {
    assert_eq!(a["grund"], json!("bestaetigt"), "keine Zahl: {a}");
    a["zahl_cent"].as_i64().unwrap_or_else(|| panic!("keine Zahl: {a}"))
}

/// Ersetzt den Wert von `feld`; fehlt es, haengt es an.
fn mit(mut paare: Paare, feld: &'static str, wert: Value) -> Paare {
    match paare.iter_mut().find(|(f, _)| *f == feld) {
        Some(p) => p.1 = wert,
        None => paare.push((feld, wert)),
    }
    paare
}

fn plus(mut paare: Paare, mehr: Paare) -> Paare {
    paare.extend(mehr);
    paare
}

/// Pflicht-Kegel `an_gesamt` (33 Felder): Arbeitnehmer ohne Arbeitsweg, einzel, alle vier Kreuze "nein", Lohn und Beitraege wie
/// angegeben (Euro). `mit_anspruch_auf_zuschuss` ist wahr: wer einen Zuschuss eintraegt, hat Anspruch darauf.
fn an(art: &'static str, lohn: i64, kv: i64, pv: i64) -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(lohn * 100)),
        ("veranlagung", json!("einzel")),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!(art)),
        ("basis_kv", json!(kv * 100)),
        ("basis_pv", json!(pv * 100)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(true)),
        ("dhf_unterkunftskosten_monat", json!(0)),
        ("dhf_monate", json!(0)),
        ("dhf_im_inland", json!(false)),
        ("dhf_beruflich_veranlasst", json!(false)),
        ("dhf_eigener_hausstand", json!(false)),
        ("dhf_finanzielle_beteiligung", json!(false)),
        ("tage_24h", json!(0)),
        ("tage_an_abreise", json!(0)),
        ("tage_ueber_8h_eintaegig", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("fam_anzahl_kinder", json!(0)),
        ("verlustvortrag_bestand", json!(0)),
    ]
}

/// Pflicht-Kegel `gesamt` (35 Felder), einzel, alle Kreuze "nein"/"kein"; Lohn und Beitraege wie `an`.
fn gesamt(art: &'static str, lohn: i64, kv: i64, pv: i64) -> Paare {
    vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(100)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(lohn * 100)),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!(art)),
        ("basis_kv", json!(kv * 100)),
        ("basis_pv", json!(pv * 100)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(true)),
        ("kap_kapitalertraege", json!(0)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ]
}

/// Pflicht-Kegel `rentner_gesamt` (28 Felder), einzel: Rente 20.000 Euro ab 2025, Beitraege wie angegeben.
fn rentner(kv: i64, pv: i64) -> Paare {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(20_000_000)),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_grad_der_behinderung", json!(0)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(0)),
        ("rentner_gepflegter_hilflos", json!(false)),
        ("rentner_hinterbliebenenbezuege", json!(false)),
        ("veranlagung", json!("einzel")),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!("gesetzlich_freiwillig")),
        ("basis_kv", json!(kv * 100)),
        ("basis_pv", json!(pv * 100)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(true)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ]
}

/// Der Ehegatte: Lohn und Beitraege (Euro), Versicherungsart gesetzlich als Arbeitnehmer, die fuenf Kapital-Felder des Partner-Kegels
/// von `gesamt` stehen nur dort (`mit_kap`).
fn partner(lohn: i64, kv: i64, pv: i64, mit_kap: bool) -> Paare {
    let mut p: Paare = vec![
        ("bruttoarbeitslohn_partner", json!(lohn * 100)),
        ("versicherungsart_partner", json!("gesetzlich_an")),
        ("basis_kv_partner", json!(kv * 100)),
        ("basis_pv_partner", json!(pv * 100)),
        ("mit_anspruch_auf_zuschuss_partner", json!(true)),
    ];
    if mit_kap {
        p.extend([
            ("kap_kapitalertraege_partner", json!(0)),
            ("kap_gewinn_aktien_partner", json!(0)),
            ("kap_gewinn_sonstige_partner", json!(0)),
            ("kap_verlust_aktien_partner", json!(0)),
            ("kap_verlust_sonstige_partner", json!(0)),
        ]);
    }
    p
}

/// Die drei Zuschussfelder von Person A (`namen` = `ZUSCHUSS_A`) oder des Ehegatten (`ZUSCHUSS_B`) mit den Euro-Betraegen
/// (gesetzliche KV, private KV, Pflege).
fn zuschuss(namen: [&'static str; 3], gkv: i64, pkv: i64, pv: i64) -> Paare {
    vec![(namen[0], json!(gkv * 100)), (namen[1], json!(pkv * 100)), (namen[2], json!(pv * 100))]
}

/// Zusammenveranlagung in `an_gesamt`: Person A und der Ehegatte mit je Lohn 40.001 und Beitrag 3.600 Euro (KV 3.000 + PV 600), dazu die
/// Zuschussfelder `a` (Person A) und `b` (Ehegatte).
fn an_paar(a: Paare, b: Paare) -> Paare {
    let basis = mit(an("gesetzlich_an", 40_001, 3_000, 600), "veranlagung", json!("zusammen"));
    plus(plus(plus(basis, partner(40_001, 3_000, 600, false)), a), b)
}

/// Dasselbe in `gesamt` (dort mit den fuenf Kapital-Feldern des Ehegatten).
fn gesamt_paar(a: Paare, b: Paare) -> Paare {
    let basis = mit(gesamt("gesetzlich_an", 40_001, 3_000, 600), "veranlagung", json!("zusammen"));
    plus(plus(plus(basis, partner(40_001, 3_000, 600, true)), a), b)
}

/// `AK5b` (a): die Zuschussfelder sind auf jeder Scheibe beantwortbar, die einen Basisbeitrag fuehrt, `POST /event` antwortet 201.
/// Vorher: 400 „nicht in dieser Scheibe". Der Ehegatte nur auf den Scheiben mit Zusammenveranlagung.
#[tokio::test]
async fn die_zuschussfelder_sind_auf_jeder_scheibe_mit_basisbeitrag_beantwortbar() {
    for scheibe in ["n_vor_gwg", "an_gesamt", "gesamt", "rentner_gesamt"] {
        let d = neuer_fall(scheibe).await;
        for feld in ZUSCHUSS_A {
            let (status, antwort) = event_post(&d, feld, &json!(10_000)).await;
            assert_eq!(status, 201, "POST /event {feld} auf {scheibe}: {antwort}");
        }
    }
    for scheibe in ["an_gesamt", "gesamt", "rentner_gesamt"] {
        let d = neuer_fall(scheibe).await;
        for feld in ZUSCHUSS_B {
            let (status, antwort) = event_post(&d, feld, &json!(10_000)).await;
            assert_eq!(status, 201, "POST /event {feld} auf {scheibe}: {antwort}");
        }
    }
}

/// `AK5b` (b): die drei Faelle des Audits, `an_gesamt`, alleinstehend, nur Lohn (Handrechnung `handrechnung.py`):
/// F1 freiwillig gesetzlich: Lohn 60.000, KV 5.500 + PV 1.000, Zuschuss Nr. 24 a 2.650 + Nr. 24 c 400 = 3.050: Abzug 3.450,
/// zvE 55.284, Steuer 12.614 Euro (ohne Zuschuss 11.492). F2 gesetzlich: Lohn 45.000, KV 3.000 + PV 600, Zuschuss 1.500 + 200 = 1.700:
/// Abzug 1.900, zvE 41.834, Steuer 7.912 (ohne 7.363). F3 privat: Lohn 100.000, KV 7.000 + PV 1.200, Zuschuss Nr. 24 b 3.100 + 400 = 3.500:
/// Abzug 4.700, zvE 94.034, Steuer 28.582 (ohne 27.112). Die Unterschiede 1.122, 549 und 1.470 Euro sind die des Audits.
///
/// Seit d-46 (4-Prozent-Kuerzung bei `gesetzlich_an`) gilt F2 mit Kuerzung: Beitrag 3.000 · 0,96 + 600 = 3.480, minus Zuschuss 1.700 = 1.780,
/// zvE 41.954, Steuer 7.951 Euro (`rechnung_bau.py`; vorher 7.912, Abzug 1.900 durch den Hoechstbetrag). F1 und F3 sind nicht `gesetzlich_an`.
#[tokio::test]
async fn der_zuschuss_mindert_den_abzug_die_drei_faelle_des_audits_an_gesamt() {
    let f1 = ergebnis("an_gesamt", &plus(an("gesetzlich_freiwillig", 60_000, 5_500, 1_000), zuschuss(ZUSCHUSS_A, 2_650, 0, 400))).await;
    assert_eq!(zahl(&f1), 12_614 * 100, "F1: {f1}");
    let f2 = ergebnis("an_gesamt", &plus(an("gesetzlich_an", 45_000, 3_000, 600), zuschuss(ZUSCHUSS_A, 1_500, 0, 200))).await;
    assert_eq!(zahl(&f2), 7_951 * 100, "F2: {f2}");
    let f3 = ergebnis("an_gesamt", &plus(an("privat", 100_000, 7_000, 1_200), zuschuss(ZUSCHUSS_A, 0, 3_100, 400))).await;
    assert_eq!(zahl(&f3), 28_582 * 100, "F3: {f3}");
}

/// `AK5b` (f): ohne Zuschuss bleibt jede Zahl wie heute. Dieselben drei Faelle ohne die Zuschussfelder: 11.492, 7.363 und 27.112 Euro
/// (Handrechnung `handrechnung.py`, „App-Lesart"). Gegenprobe zu den Zuschuss-Tests: wer nichts eintraegt, rechnet wie vorher.
///
/// Seit d-46 gilt fuer `gesetzlich_an` die 4-Prozent-Kuerzung: 7.402 statt 7.363 Euro (zvE 40.254, `rechnung_bau.py`); die zwei anderen Arten
/// bleiben bei den alten Zahlen.
#[tokio::test]
async fn ohne_zuschuss_bleibt_jede_zahl_wie_heute() {
    for (art, lohn, kv, pv, steuer) in [
        ("gesetzlich_freiwillig", 60_000, 5_500, 1_000, 11_492),
        ("gesetzlich_an", 45_000, 3_000, 600, 7_402),
        ("privat", 100_000, 7_000, 1_200, 27_112),
    ] {
        let a = ergebnis("an_gesamt", &an(art, lohn, kv, pv)).await;
        assert_eq!(zahl(&a), steuer * 100, "{art}, Lohn {lohn}: {a}");
    }
    // Ein Zuschuss von 0 ist kein Zuschuss.
    let null = ergebnis("an_gesamt", &plus(an("gesetzlich_an", 45_000, 3_000, 600), zuschuss(ZUSCHUSS_A, 0, 0, 0))).await;
    assert_eq!(zahl(&null), 7_402 * 100, "Zuschuss 0: {null}");
}

/// `AK5b` (d): ein Zuschuss groesser als der Beitrag zieht nicht ueber 0 ab. Beitrag 3.600, Zuschuss 4.000: Abzug 0, nicht −400; das ist
/// die Steuer ohne jeden Beitrag (zvE 43.734, Steuer 8.538 Euro, Handrechnung).
#[tokio::test]
async fn ein_zuschuss_ueber_dem_beitrag_zieht_nicht_ueber_null_ab() {
    let a = ergebnis("an_gesamt", &plus(an("gesetzlich_freiwillig", 45_000, 3_000, 600), zuschuss(ZUSCHUSS_A, 4_000, 0, 0))).await;
    assert_eq!(zahl(&a), 8_538 * 100, "Zuschuss 4.000 > Beitrag 3.600: {a}");
    let ohne_beitrag = ergebnis("an_gesamt", &an("gesetzlich_freiwillig", 45_000, 0, 0)).await;
    assert_eq!(zahl(&ohne_beitrag), 8_538 * 100, "Gegenprobe ohne Beitrag: {ohne_beitrag}");
}

/// `AK5b` (e), `an_gesamt` und `gesamt` bei Zusammenveranlagung: Person B hat einen eigenen Zuschuss, Person A keinen, und umgekehrt: je
/// dieselbe Steuer (Lohn je 40.001, Beitrag je 3.600, ein Zuschuss von 1.700 bei einer Person: Abzug 5.500, zvE 71.970, Steuer 12.132 Euro).
/// Ohne Zuschuss 11.616 (Abzug 7.200), mit je 1.700 bei beiden 12.654 (Abzug 3.800). Ein vertauschtes oder fehlendes Feld faellt hier auf.
///
/// Seit d-46 sind beide Personen `gesetzlich_an` und gekuerzt (je KV 3.000 · 0,96 + PV 600 = 3.480; `rechnung_bau.py`): keiner zvE 70.510,
/// Steuer 11.690 Euro (Abzug 6.960); ein Zuschuss von 1.700 bei einer Person: Abzug 5.260, zvE 72.210, Steuer 12.206; bei beiden: Abzug 3.560,
/// zvE 73.910, Steuer 12.728.
#[tokio::test]
async fn bei_zusammenveranlagung_zaehlt_der_zuschuss_je_person_fuer_sich() {
    for (scheibe, bauen) in [("an_gesamt", an_paar as fn(Paare, Paare) -> Paare), ("gesamt", gesamt_paar)] {
        let nur_b = ergebnis(scheibe, &bauen(vec![], zuschuss(ZUSCHUSS_B, 1_500, 0, 200))).await;
        let nur_a = ergebnis(scheibe, &bauen(zuschuss(ZUSCHUSS_A, 1_500, 0, 200), vec![])).await;
        let keiner = ergebnis(scheibe, &bauen(vec![], vec![])).await;
        let beide = ergebnis(scheibe, &bauen(zuschuss(ZUSCHUSS_A, 1_500, 0, 200), zuschuss(ZUSCHUSS_B, 1_500, 0, 200))).await;
        assert_eq!(zahl(&nur_b), 12_206 * 100, "{scheibe}, nur der Ehegatte: {nur_b}");
        assert_eq!(zahl(&nur_a), 12_206 * 100, "{scheibe}, nur Person A: {nur_a}");
        assert_eq!(zahl(&keiner), 11_690 * 100, "{scheibe}, keiner: {keiner}");
        assert_eq!(zahl(&beide), 12_728 * 100, "{scheibe}, beide: {beide}");
    }
}

/// `AK5b` (d) bei Zusammenveranlagung, `an_gesamt` und `gesamt`: ein Zuschuss ueber dem Beitrag der EINEN Person zieht bei ihr nur bis 0 ab und
/// mindert den Abzug der anderen Person nicht. Lohn je 40.001, Beitrag je 3.600; Person A mit Zuschuss 4.000 (Abzug 0), der Ehegatte ohne
/// (Abzug 3.600): zvE 73.870, Steuer 12.716 Euro (Handrechnung `handrechnung.py`, Abschnitt g), vertauscht dieselbe Steuer und dieselbe wie bei
/// einer Person ganz ohne Beitrag. Ein Abzug von −400 bei Person A (ungeklemmt) liesse den Abzug auf 3.200 sinken und die Steuer auf 12.840 steigen.
///
/// Seit d-46 ist die andere Person `gesetzlich_an` und gekuerzt (Abzug 3.480 statt 3.600; `rechnung_bau.py`): zvE 73.990, Steuer 12.754 Euro.
#[tokio::test]
async fn ein_zuschuss_ueber_dem_beitrag_mindert_den_abzug_der_anderen_person_nicht() {
    for (scheibe, bauen) in [("an_gesamt", an_paar as fn(Paare, Paare) -> Paare), ("gesamt", gesamt_paar)] {
        let a_ueber = ergebnis(scheibe, &bauen(zuschuss(ZUSCHUSS_A, 4_000, 0, 0), vec![])).await;
        let b_ueber = ergebnis(scheibe, &bauen(vec![], zuschuss(ZUSCHUSS_B, 4_000, 0, 0))).await;
        assert_eq!(zahl(&a_ueber), 12_754 * 100, "{scheibe}, Zuschuss von Person A ueber ihrem Beitrag: {a_ueber}");
        assert_eq!(zahl(&b_ueber), 12_754 * 100, "{scheibe}, Zuschuss des Ehegatten ueber seinem Beitrag: {b_ueber}");
    }
}

/// `AK5b`, `gesamt` (der Ring rechnet ueber `abzuege.rs`, nicht ueber `an_gesamt.rs`): F1 wie oben, 12.614 Euro, ohne Zuschuss 11.492.
#[tokio::test]
async fn der_zuschuss_gilt_auch_im_gesamtlauf() {
    let mit_zuschuss = ergebnis("gesamt", &plus(gesamt("gesetzlich_freiwillig", 60_000, 5_500, 1_000), zuschuss(ZUSCHUSS_A, 2_650, 0, 400))).await;
    assert_eq!(zahl(&mit_zuschuss), 12_614 * 100, "gesamt, F1: {mit_zuschuss}");
    let ohne = ergebnis("gesamt", &gesamt("gesetzlich_freiwillig", 60_000, 5_500, 1_000)).await;
    assert_eq!(zahl(&ohne), 11_492 * 100, "gesamt, F1 ohne Zuschuss: {ohne}");
}

/// `AK5b`, `rentner_gesamt`: der Zuschuss rechnet wie ein um denselben Betrag kleinerer Beitrag (Gleichwertigkeit, keine Handrechnung der
/// Rentensteuer): KV 4.000 + PV 800 mit Zuschuss 1.500 + 300 gibt dieselbe Steuer wie KV 2.500 + PV 500 ohne, und eine andere als der
/// volle Beitrag ohne Zuschuss.
#[tokio::test]
async fn der_zuschuss_gilt_auch_in_der_rentner_scheibe() {
    let mit_zuschuss = ergebnis("rentner_gesamt", &plus(rentner(4_000, 800), zuschuss(ZUSCHUSS_A, 1_500, 0, 300))).await;
    let netto = ergebnis("rentner_gesamt", &rentner(2_500, 500)).await;
    let voll = ergebnis("rentner_gesamt", &rentner(4_000, 800)).await;
    assert_eq!(zahl(&mit_zuschuss), zahl(&netto), "Zuschuss gleich kleinerer Beitrag: {mit_zuschuss} gegen {netto}");
    assert_ne!(zahl(&mit_zuschuss), zahl(&voll), "KONTROLLE: der volle Beitrag rechnet eine andere Steuer: {voll}");
}

/// Die 4-Prozent-Kuerzung fuer den Anspruch auf Krankengeld (§ 10 Abs. 1 Nr. 3 Buchst. a S. 4 `EStG`, Entscheidung d-46 vom 2026-10-20: Regel nach
/// Versicherungsart, keine neue Frage). Die Lohnsteuerbescheinigung Nr. 25 meldet den Beitrag ungekuerzt (BMF-Schreiben vom 05.09.2024,
/// Zeile 59), die App kuerzt den KV-Beitrag pflichtversicherter Arbeitnehmer (`gesetzlich_an`) um 4 Prozent, nie den PV-Beitrag.
///
/// HERKUNFT DER ERWARTUNGSWERTE: Handrechnung `rechnung_bau.py` (Auftragsordner), Tarif aus `params/2025/`, geeicht an den Steuerwerten der
/// Tests oben (6 von 6 Einzeltarif, 2 von 2 Splitting). Fall A: Lohn 45.000, KV 3.000 + PV 600: Abzug 3.000 · 0,96 + 600 = 3.480, zvE
/// 45.000 − 1.230 − 3.480 − 36 = 40.254, Steuer 7.402 Euro (ohne Kuerzung: zvE 40.134, 7.363).
///
/// `an_gesamt` und `gesamt` (die Scheiben mit `versicherungsart`): `gesetzlich_an` zahlt 39 Euro mehr, `gesetzlich_freiwillig` und `privat`
/// bleiben bei 7.363 (Gegenprobe: ohne Art-Unterscheidung wuerden alle drei gleich rechnen).
#[tokio::test]
async fn gesetzlich_an_kuerzt_den_kv_beitrag_um_vier_prozent_die_anderen_arten_nicht() {
    for scheibe in ["an_gesamt", "gesamt"] {
        let bauen = |art: &'static str| if scheibe == "an_gesamt" { an(art, 45_000, 3_000, 600) } else { gesamt(art, 45_000, 3_000, 600) };
        for (art, steuer) in [("gesetzlich_an", 7_402), ("gesetzlich_freiwillig", 7_363), ("privat", 7_363)] {
            let a = ergebnis(scheibe, &bauen(art)).await;
            assert_eq!(zahl(&a), steuer * 100, "{scheibe}, {art}: {a}");
        }
    }
}

/// Die Kuerzung trifft nur den KV-Beitrag: der PV-Beitrag bleibt voll. Lohn 45.000, `gesetzlich_an`: KV 0 und PV 600 geben Abzug 600, zvE 43.134,
/// Steuer 8.339 Euro (kein Fehler bei Beitrag 0, keine Kuerzung des PV-Beitrags, gleich `gesetzlich_freiwillig`); KV 3.000 und PV 0 geben Abzug
/// 2.880, zvE 40.854, Steuer 7.594 Euro (`gesetzlich_freiwillig`: Abzug 3.000, zvE 40.734, 7.556).
#[tokio::test]
async fn die_kuerzung_trifft_nie_den_pv_beitrag_und_nicht_den_beitrag_null() {
    for art in ["gesetzlich_an", "gesetzlich_freiwillig"] {
        let nur_pv = ergebnis("an_gesamt", &an(art, 45_000, 0, 600)).await;
        assert_eq!(zahl(&nur_pv), 8_339 * 100, "{art}: KV 0, PV 600 bleibt voll: {nur_pv}");
    }
    let nur_kv = ergebnis("an_gesamt", &an("gesetzlich_an", 45_000, 3_000, 0)).await;
    assert_eq!(zahl(&nur_kv), 7_594 * 100, "gesetzlich_an, KV 3.000, PV 0: {nur_kv}");
    let nur_kv_frei = ergebnis("an_gesamt", &an("gesetzlich_freiwillig", 45_000, 3_000, 0)).await;
    assert_eq!(zahl(&nur_kv_frei), 7_556 * 100, "Gegenprobe freiwillig, KV 3.000, PV 0: {nur_kv_frei}");
}

/// Zusammenveranlagung, `an_gesamt` und `gesamt`, Lohn je 40.001, Beitrag je KV 3.000 + PV 600: beide `gesetzlich_an` gibt zvE 70.510, Steuer
/// 11.690 Euro (ohne Kuerzung 11.616); nur Person A `gesetzlich_an` (Ehegatte freiwillig) und nur der Ehegatte `gesetzlich_an` (Person A
/// freiwillig) geben je zvE 70.390, Steuer 11.652; beide freiwillig bleiben bei 11.616. Ein Vertauschen der Felder von Person A und Ehegatte
/// faellt hier auf, weil die zwei gemischten Faelle dieselbe Zahl geben, aber nicht dieselbe wie „beide“.
#[tokio::test]
async fn bei_zusammenveranlagung_kuerzt_jede_person_nach_ihrer_eigenen_versicherungsart() {
    let frei = |p: Paare| mit(p, "versicherungsart_partner", json!("gesetzlich_freiwillig"));
    for (scheibe, bauen) in [("an_gesamt", an_paar as fn(Paare, Paare) -> Paare), ("gesamt", gesamt_paar)] {
        let beide = ergebnis(scheibe, &bauen(vec![], vec![])).await;
        let nur_a = ergebnis(scheibe, &frei(bauen(vec![], vec![]))).await;
        let nur_b = ergebnis(scheibe, &mit(bauen(vec![], vec![]), "versicherungsart", json!("gesetzlich_freiwillig"))).await;
        let keiner = ergebnis(scheibe, &frei(mit(bauen(vec![], vec![]), "versicherungsart", json!("gesetzlich_freiwillig")))).await;
        assert_eq!(zahl(&beide), 11_690 * 100, "{scheibe}, beide gesetzlich_an: {beide}");
        assert_eq!(zahl(&nur_a), 11_652 * 100, "{scheibe}, nur Person A gesetzlich_an: {nur_a}");
        assert_eq!(zahl(&nur_b), 11_652 * 100, "{scheibe}, nur der Ehegatte gesetzlich_an: {nur_b}");
        assert_eq!(zahl(&keiner), 11_616 * 100, "{scheibe}, beide freiwillig: {keiner}");
    }
}

/// Zuschuss und Kuerzung zusammen (Nr. 92 und d-46): die Kuerzung rechnet vom vollen KV-Beitrag, der Zuschuss kommt danach ab. Der Anlage-
/// Vorsorgeaufwand-Vordruck 2025 kennt zu den Zuschuss-Zeilen (21, 26, 34 bis 36) keine „davon mit Anspruch“-Zeile, zu den Erstattungen
/// schon (15, 20, 32): der Zuschuss mindert die Basis der 4 Prozent nicht (`derived`, keine Quelle sagt es wortlich). Fall F2 des Audits:
/// Lohn 45.000, KV 3.000 + PV 600, Zuschuss 1.500 + 200: Beitrag 3.480 − 1.700 = 1.780, zvE 41.954, Steuer 7.951 Euro (ohne Kuerzung 7.912).
#[tokio::test]
async fn zuschuss_kommt_nach_der_kuerzung_ab() {
    let f2 = ergebnis("an_gesamt", &plus(an("gesetzlich_an", 45_000, 3_000, 600), zuschuss(ZUSCHUSS_A, 1_500, 0, 200))).await;
    assert_eq!(zahl(&f2), 7_951 * 100, "F2 gesetzlich_an: {f2}");
    let frei = ergebnis("an_gesamt", &plus(an("gesetzlich_freiwillig", 45_000, 3_000, 600), zuschuss(ZUSCHUSS_A, 1_500, 0, 200))).await;
    assert_eq!(zahl(&frei), 7_912 * 100, "F2 freiwillig bleibt: {frei}");
}

/// `rentner_gesamt`: dieselbe Funktion. Wer dort `gesetzlich_an` waehlt, bekommt die Kuerzung wie in den anderen Scheiben (Gleichwertigkeit, keine
/// Handrechnung der Rentensteuer): KV 4.000 + PV 800 mit `gesetzlich_an` rechnet wie KV 3.840 + PV 800 mit `gesetzlich_freiwillig` und anders als
/// KV 4.000 + PV 800 mit `gesetzlich_freiwillig` (die Rentner-Scheibe fragt `gesetzlich_freiwillig` als Regelfall, siehe `rentner`).
#[tokio::test]
async fn in_der_rentner_scheibe_kuerzt_nur_gesetzlich_an() {
    let an_art = ergebnis("rentner_gesamt", &mit(rentner(4_000, 800), "versicherungsart", json!("gesetzlich_an"))).await;
    let gekuerzt = ergebnis("rentner_gesamt", &rentner(3_840, 800)).await;
    let voll = ergebnis("rentner_gesamt", &rentner(4_000, 800)).await;
    assert_eq!(zahl(&an_art), zahl(&gekuerzt), "gesetzlich_an gleich KV um 160 kleiner: {an_art} gegen {gekuerzt}");
    assert_ne!(zahl(&an_art), zahl(&voll), "KONTROLLE: freiwillig mit vollem Beitrag rechnet eine andere Steuer: {voll}");
}

/// Die Erklaerung an das Finanzamt aendert sich nicht: Zeile 11 der Anlage Vorsorgeaufwand traegt den Beitrag so, wie die Lohnsteuerbescheinigung
/// ihn nennt (Nr. 25, ungekuerzt); das Finanzamt kuerzt selbst. Kz E2001203 (gesetzliche KV, Arbeitnehmer) steht bei Person A und beim
/// Ehegatten auf 3.000 Euro, Kz E2001505 (Pflege) auf 600. Die Kuerzung steht nirgends in der Erklaerung.
#[tokio::test]
async fn die_erklaerung_traegt_den_kv_beitrag_ungekuerzt() {
    let paare = plus(mit(gesamt("gesetzlich_an", 40_001, 3_000, 600), "veranlagung", json!("zusammen")), partner(40_001, 3_000, 600, true));
    let d = fall_mit("gesamt", &paare).await;
    let (status, dekl) = sende(&d, "GET", "/fall/kvz/deklaration", None).await;
    assert_eq!(status, 200, "GET /deklaration: {dekl}");
    assert_eq!(dekl["deklaration"]["E2001203"], json!(3_000), "Person A, KV ungekuerzt: {dekl}");
    assert_eq!(dekl["person_b"]["E2001203"], json!(3_000), "Ehegatte, KV ungekuerzt: {dekl}");
    assert_eq!(dekl["deklaration"]["E2001505"], json!(600), "Person A, Pflege: {dekl}");
    assert_eq!(dekl["person_b"]["E2001505"], json!(600), "Ehegatte, Pflege: {dekl}");
}

/// AK5a/AK5c: die sechs Fragen stehen, solange der Anspruch auf einen Zuschuss nicht bestaetigt verneint ist (`feld_bedingung` auf
/// `mit_anspruch_auf_zuschuss`), und fehlen bei einem bestaetigten Nein. Der Ehegatte folgt `mit_anspruch_auf_zuschuss_partner` und steht
/// nur bei Zusammenveranlagung. Keine Pflichtfrage: der Kegel (33 Felder) bleibt, die Zahl steht auch ohne Antwort.
#[tokio::test]
async fn die_zuschussfragen_stehen_nur_ohne_verneinten_anspruch() {
    let ja = fragen_ids(&fall_mit("an_gesamt", &an("gesetzlich_freiwillig", 60_000, 5_500, 1_000)).await).await;
    for feld in ZUSCHUSS_A {
        assert!(ja.iter().any(|i| i == feld), "{feld} fehlt bei Anspruch auf Zuschuss: {ja:?}");
    }
    for feld in ZUSCHUSS_B {
        assert!(!ja.iter().any(|i| i == feld), "{feld} steht bei Einzelveranlagung: {ja:?}");
    }
    let nein = fragen_ids(&fall_mit("an_gesamt", &mit(an("gesetzlich_freiwillig", 60_000, 5_500, 1_000), "mit_anspruch_auf_zuschuss", json!(false))).await).await;
    assert!(!nein.is_empty(), "KONTROLLE: der Katalog ist nicht leer");
    for feld in ZUSCHUSS_A {
        assert!(!nein.iter().any(|i| i == feld), "{feld} steht bei verneintem Anspruch: {nein:?}");
    }
    let zusammen = plus(mit(an("gesetzlich_an", 40_001, 3_000, 600), "veranlagung", json!("zusammen")), partner(40_001, 3_000, 600, false));
    let ids = fragen_ids(&fall_mit("an_gesamt", &zusammen).await).await;
    for feld in ZUSCHUSS_A.into_iter().chain(ZUSCHUSS_B) {
        assert!(ids.iter().any(|i| i == feld), "{feld} fehlt bei Zusammenveranlagung: {ids:?}");
    }
    let ohne_partner_anspruch = mit(zusammen, "mit_anspruch_auf_zuschuss_partner", json!(false));
    let ids = fragen_ids(&fall_mit("an_gesamt", &ohne_partner_anspruch).await).await;
    for feld in ZUSCHUSS_A {
        assert!(ids.iter().any(|i| i == feld), "{feld} (Person A) fehlt, obwohl nur der Ehegatte verneint: {ids:?}");
    }
    for feld in ZUSCHUSS_B {
        assert!(!ids.iter().any(|i| i == feld), "{feld} steht bei verneintem Anspruch des Ehegatten: {ids:?}");
    }
}

/// `AK5b` (Erklaerung): die Anlage Vorsorgeaufwand traegt den Zuschuss je Person (`Stfr_AG_Zusch`: E2003705 gesetzliche KV, E2003807 private
/// KV, E2003907 gesetzliche Pflege, laut Nr. 24 a, b, c der Lohnsteuerbescheinigung; `E10-2025.xsd`). Person A in `deklaration`, der
/// Ehegatte in `person_b`. Ohne Zuschussangabe steht keines der drei Kz in der Erklaerung.
#[tokio::test]
async fn die_erklaerung_traegt_den_zuschuss_je_person() {
    let zusammen = |a: Paare, b: Paare| {
        plus(plus(plus(mit(gesamt("gesetzlich_freiwillig", 40_001, 3_000, 600), "veranlagung", json!("zusammen")), partner(40_001, 3_000, 600, true)), a), b)
    };
    let dekl = |paare: Paare| async move {
        let d = fall_mit("gesamt", &paare).await;
        let (status, dekl) = sende(&d, "GET", "/fall/kvz/deklaration", None).await;
        assert_eq!(status, 200, "GET /deklaration: {dekl}");
        dekl
    };
    let mit_zuschuss = dekl(zusammen(zuschuss(ZUSCHUSS_A, 1_500, 0, 200), zuschuss(ZUSCHUSS_B, 0, 1_100, 90))).await;
    assert_eq!(mit_zuschuss["deklaration"]["E2003705"], json!(1_500), "Person A, gesetzliche KV: {mit_zuschuss}");
    assert_eq!(mit_zuschuss["deklaration"]["E2003907"], json!(200), "Person A, Pflege: {mit_zuschuss}");
    assert_eq!(mit_zuschuss["person_b"]["E2003807"], json!(1_100), "Ehegatte, private KV: {mit_zuschuss}");
    assert_eq!(mit_zuschuss["person_b"]["E2003907"], json!(90), "Ehegatte, Pflege: {mit_zuschuss}");
    let ohne = dekl(zusammen(vec![], vec![])).await;
    for kz in ["E2003705", "E2003807", "E2003907"] {
        assert!(ohne["deklaration"].get(kz).is_none_or(Value::is_null), "{kz} ohne Zuschuss in Person A: {ohne}");
        assert!(ohne["person_b"].get(kz).is_none_or(Value::is_null), "{kz} ohne Zuschuss im Ehegatten: {ohne}");
    }
}
