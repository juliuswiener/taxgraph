//! Gesamtlauf (`gesamt`) bei Zusammenveranlagung ueber HTTP: die fuenf Versorgungsfelder des Ehegatten sind beantwortbar, werden
//! gefragt und aendern die Steuer. Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Wer verheiratet ist und gemeinsam veranlagt wird, konnte im Gesamtlauf keinen Versorgungsbezug (Werkspension,
//! Beamtenpension) fuer den Ehegatten eintragen: `POST /event` auf `versorgung_jahresrente_partner` antwortete 400 „nicht in dieser
//! Scheibe". Ein Bezug des Ehegatten steckte nur in dessen Bruttoarbeitslohn, und ein Versorgungsfreibetrag fehlte.
//!
//! **Warum es zaehlt.** Der Rechenring (`bescheid/src/zweige/gesamt.rs`) liest die Felder von Person B schon (Abweichung Nr. 33,
//! Rentner-Scheibe), nur die Scheibe `gesamt` nahm sie nicht an. Die Steuer war zu hoch (Richtung aus dem Code abgeleitet): der Bezug
//! zaehlte als voller Arbeitslohn mit Arbeitnehmer-Pauschbetrag statt als Versorgungsbezug mit Freibetrag, Zuschlag und Pauschbetrag.
//!
//! **Wo es sitzt.** `SCHEIBEN_GESAMT_FELDER` (`bescheid/src/deklaration/scheiben_tabellen.rs`), Abweichung Nr. 87. Der Kegel
//! (35 Pflichtfelder) bleibt, die Angaben zum Ehegatten sind keine neuen Pflichtfragen. Person B bekommt ihre Altersgrenze mit
//! `rentner_grad_der_behinderung_partner`; Nr. 84 (verneinte Behinderung gilt als Grad 0) gilt nur fuer Person A.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus `params/` und dem Gesetzestext gerechnet, nie aus dem Rust-Code gelesen
//! (Skript `handrechnung.py` im Auftragsordner, Rechenweg im Kopf von `bescheid/tests/gesamt_ehegatte_versorgung_hermetisch.rs`).
//! Die Bezuege sind 30.001 und 60.001 Euro, damit das zvE gerade ist und die Haelfte beim Splitting keine Rundungsfrage stellt.
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

/// Die fuenf Felder des Ehegatten, die diese Abweichung der Scheibe `gesamt` gibt.
const EHEGATTEN_VERSORGUNG: [&str; 5] = [
    "versorgung_jahresrente_partner",
    "versorgung_bemessungsgrundlage_partner",
    "versorgung_beginn_jahr_partner",
    "versorgung_art_partner",
    "versorgung_alter_bei_beginn_partner",
];

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
    let auth = Auth::neu(
        "testgeheimnis".into(),
        tmp.path().join("users.json"),
        Some(konfig.audit_pfad()),
    );
    let zustand = Zustand::neu(konfig, auth);
    let token = zustand.auth.stelle_aus("alice").unwrap();
    Dienst {
        zustand,
        token,
        _tmp: tmp,
    }
}

async fn sende(d: &Dienst, methode: &str, pfad: &str, body: Option<&Value>) -> (u16, Value) {
    let text = body.map(ToString::to_string);
    let mut b = Request::builder()
        .method(methode)
        .uri(pfad)
        .header("authorization", format!("Bearer {}", d.token));
    if let Some(t) = &text {
        b = b
            .header("content-type", "application/json")
            .header("content-length", t.len().to_string());
    }
    let req = b.body(text.map_or_else(Body::empty, Body::from)).unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (
        teile.status.as_u16(),
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Ein Event ueber die echte Route `POST /event` (Nutzer-Klick, bestaetigt); liefert Status und Antwort.
async fn event_post(d: &Dienst, feld: &str, wert: &Value) -> (u16, Value) {
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    sende(d, "POST", "/fall/gev/event", Some(&rumpf)).await
}

/// Ein leerer Fall der Scheibe `gesamt`.
async fn neuer_fall() -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "gev", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    d
}

/// Ein Fall `gesamt` mit allen `paare`, jedes ueber `POST /event` (muss 201 sein).
async fn fall_mit(paare: &[(&'static str, Value)]) -> Dienst {
    let d = neuer_fall().await;
    for (feld, wert) in paare {
        let (status, antwort) = event_post(&d, feld, wert).await;
        assert_eq!(status, 201, "POST /event {feld} auf gesamt: {antwort}");
    }
    d
}

async fn ergebnis(paare: &[(&'static str, Value)]) -> Value {
    let d = fall_mit(paare).await;
    let (status, antwort) = sende(&d, "GET", "/fall/gev/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// Die Feld-IDs der Fragen, die der Fall noch stellt (`GET /fragen`).
async fn fragen_ids(d: &Dienst) -> Vec<String> {
    let (status, fragen) = sende(d, "GET", "/fall/gev/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["feld_id"].as_str().unwrap().to_owned())
        .collect()
}

fn zahl(a: &Value) -> i64 {
    assert_eq!(a["grund"], json!("bestaetigt"), "keine Zahl: {a}");
    a["zahl_cent"]
        .as_i64()
        .unwrap_or_else(|| panic!("keine Zahl: {a}"))
}

/// Pflicht-Kegel `gesamt` (35 Felder) fuer ein Paar, ZUSAMMEN veranlagt, ohne jede Einkunft ausser dem Lohn: `lohn_a` Euro fuer
/// Person A (`bruttoarbeitslohn`), `lohn_b` Euro fuer den Ehegatten (`bruttoarbeitslohn_partner`), alle Kreuze „nein"/„kein".
/// `veranlagung` wie angegeben; der Partner-Kegel (Versicherungsart und fuenf Kapital-Felder) steht nur bei „zusammen".
fn paar(veranlagung: &'static str, lohn_a: i64, lohn_b: i64) -> Paare {
    let mut p: Paare = vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(100)),
        ("veranlagung", json!(veranlagung)),
        ("bruttoarbeitslohn", json!(lohn_a * 100)),
        ("bruttoarbeitslohn_partner", json!(lohn_b * 100)),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!("gesetzlich_an")),
        ("basis_kv", json!(0)),
        ("basis_pv", json!(0)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(false)),
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
    ];
    if veranlagung == "zusammen" {
        p.extend([
            ("versicherungsart_partner", json!("gesetzlich_an")),
            ("kap_kapitalertraege_partner", json!(0)),
            ("kap_gewinn_aktien_partner", json!(0)),
            ("kap_gewinn_sonstige_partner", json!(0)),
            ("kap_verlust_aktien_partner", json!(0)),
            ("kap_verlust_sonstige_partner", json!(0)),
        ]);
    }
    p
}

/// Die vier Versorgungsfelder des Ehegatten fuer einen beamtenrechtlichen Bezug von `euro` mit der Bemessungsgrundlage `bemessung`
/// und dem Beginnjahr `beginn`.
fn versorgung_b(euro: i64, bemessung: i64, beginn: i64) -> Paare {
    vec![
        ("versorgung_jahresrente_partner", json!(euro * 100)),
        ("versorgung_bemessungsgrundlage_partner", json!(bemessung * 100)),
        ("versorgung_beginn_jahr_partner", json!(beginn)),
        ("versorgung_art_partner", json!("beamtenrechtlich")),
    ]
}

/// Dieselben vier Felder fuer Person A.
fn versorgung_a(euro: i64, bemessung: i64, beginn: i64) -> Paare {
    vec![
        ("versorgung_jahresrente", json!(euro * 100)),
        ("versorgung_bemessungsgrundlage", json!(bemessung * 100)),
        ("versorgung_beginn_jahr", json!(beginn)),
        ("versorgung_art", json!("beamtenrechtlich")),
    ]
}

fn mit(mut basis: Paare, mehr: Paare) -> Paare {
    basis.extend(mehr);
    basis
}

/// AK1 (a): die fuenf Felder sind auf `gesamt` beantwortbar, `POST /event` antwortet 201. Vorher: 400 „nicht in dieser Scheibe".
#[tokio::test]
async fn die_fuenf_versorgungsfelder_des_ehegatten_sind_auf_gesamt_beantwortbar() {
    let felder: Paare = vec![
        ("versorgung_jahresrente_partner", json!(3_000_000)),
        ("versorgung_bemessungsgrundlage_partner", json!(3_000_000)),
        ("versorgung_beginn_jahr_partner", json!(2025)),
        ("versorgung_art_partner", json!("altersgrenze_sonstige")),
        ("versorgung_alter_bei_beginn_partner", json!(63)),
    ];
    assert_eq!(felder.len(), EHEGATTEN_VERSORGUNG.len());
    let d = neuer_fall().await;
    for (feld, wert) in &felder {
        let (status, antwort) = event_post(&d, feld, wert).await;
        assert_eq!(status, 201, "POST /event {feld} auf gesamt: {antwort}");
    }
}

/// Die Fragen stehen bei Zusammenveranlagung und fehlen bei Einzelveranlagung. Die Alters-Frage steht nur, solange die Art nicht
/// bestaetigt eine andere ist (bei `beamtenrechtlich` entfaellt sie, bei `altersgrenze_sonstige` steht sie).
#[tokio::test]
async fn die_fragen_zum_versorgungsbezug_des_ehegatten_stehen_nur_bei_zusammenveranlagung() {
    let ohne_alter = || EHEGATTEN_VERSORGUNG.iter().filter(|f| **f != "versorgung_alter_bei_beginn_partner").copied();
    let ids = fragen_ids(&fall_mit(&paar("zusammen", 0, 0)).await).await;
    for feld in ohne_alter() {
        assert!(ids.iter().any(|i| i == feld), "{feld} fehlt bei Zusammenveranlagung: {ids:?}");
    }
    let ids = fragen_ids(&fall_mit(&paar("einzel", 0, 0)).await).await;
    for feld in EHEGATTEN_VERSORGUNG {
        assert!(!ids.iter().any(|i| i == feld), "{feld} steht bei Einzelveranlagung: {ids:?}");
    }
    let art = |wert: &'static str| mit(paar("zusammen", 0, 0), vec![("versorgung_art_partner", json!(wert))]);
    let ids = fragen_ids(&fall_mit(&art("beamtenrechtlich")).await).await;
    assert!(!ids.iter().any(|i| i == "versorgung_alter_bei_beginn_partner"), "beamtenrechtlich: {ids:?}");
    let ids = fragen_ids(&fall_mit(&art("altersgrenze_sonstige")).await).await;
    assert!(ids.iter().any(|i| i == "versorgung_alter_bei_beginn_partner"), "altersgrenze_sonstige: {ids:?}");
}

/// AK1 (b): der Bezug des Ehegatten geht mit Freibetrag, Zuschlag und Pauschbetrag in die Steuer. Beginn 2025 (13,2 %, hoechstens
/// 990, Zuschlag 297). Handrechnung (`handrechnung.py`): 30.001 − 990 − 297 − 102 = 28.612, zvE 28.540, Steuer 696 Euro;
/// 60.001 − 1.389 = 58.612, zvE 58.540, Steuer 8.194 Euro. Der Lohn (Nr. 3) enthaelt den Bezug (Nr. 8), also ist der Lohn gleich dem Bezug.
/// Vorher: der Bezug war nicht eintragbar, der Test scheiterte schon beim Eintragen.
#[tokio::test]
async fn der_bezug_des_ehegatten_geht_nach_freibetrag_zuschlag_und_pauschbetrag_in_die_steuer() {
    let ohne = zahl(&ergebnis(&paar("zusammen", 0, 0)).await);
    assert_eq!(ohne, 0, "ohne jede Einkunft keine Steuer");
    for (euro, steuer) in [(30_001, 696), (60_001, 8_194)] {
        let a = ergebnis(&mit(paar("zusammen", 0, euro), versorgung_b(euro, euro, 2025))).await;
        assert_eq!(zahl(&a), steuer * 100, "Bezug {euro} Euro: {a}");
    }
}

/// AK1 (b), Beginnjahr und Bemessungsgrundlage des EHEGATTEN: Beginn 2005 (Kohorte 2005: 40,0 %, hoechstens 3.000, Zuschlag 900)
/// bei 30.000 Euro: 30.000 − 3.900 − 102 = 25.998, zvE 25.926, Steuer 256 Euro. Bemessungsgrundlage 5.000 Euro bei Beginn 2025:
/// 13,2 % von 5.000 = 660, Zuschlag 297: 30.001 − 660 − 297 − 102 = 28.942, zvE 28.870, Steuer 756 Euro.
#[tokio::test]
async fn der_freibetrag_folgt_beginnjahr_und_bemessung_des_ehegatten() {
    let beginn = ergebnis(&mit(paar("zusammen", 0, 30_000), versorgung_b(30_000, 30_000, 2005))).await;
    assert_eq!(zahl(&beginn), 256 * 100, "Beginn 2005: {beginn}");
    let bemessung = ergebnis(&mit(paar("zusammen", 0, 30_001), versorgung_b(30_001, 5_000, 2025))).await;
    assert_eq!(zahl(&bemessung), 756 * 100, "Bemessung 5.000: {bemessung}");
}

/// AK1 (c): Person A und der Ehegatte haben je eine eigene Versorgung. Nur A, nur B: dieselbe Steuer (696 Euro bei 30.001).
/// A mit 30.001 und B mit 60.001: gdb 28.612 + 58.612 = 87.224, zvE 87.152, Steuer 16.970 Euro, und die Vertauschung (A 60.001,
/// B 30.001) rechnet dasselbe: je Person ein Bezug, kein Bezug zaehlt doppelt oder entfaellt.
#[tokio::test]
async fn die_versorgung_beider_ehegatten_zaehlt_je_fuer_sich() {
    let nur_b = ergebnis(&mit(paar("zusammen", 0, 30_001), versorgung_b(30_001, 30_001, 2025))).await;
    let nur_a = ergebnis(&mit(paar("zusammen", 30_001, 0), versorgung_a(30_001, 30_001, 2025))).await;
    assert_eq!(zahl(&nur_b), 696 * 100, "nur der Ehegatte: {nur_b}");
    assert_eq!(zahl(&nur_a), 696 * 100, "nur Person A: {nur_a}");
    let a_klein = mit(
        mit(paar("zusammen", 30_001, 60_001), versorgung_a(30_001, 30_001, 2025)),
        versorgung_b(60_001, 60_001, 2025),
    );
    let a_gross = mit(
        mit(paar("zusammen", 60_001, 30_001), versorgung_a(60_001, 60_001, 2025)),
        versorgung_b(30_001, 30_001, 2025),
    );
    let (klein, gross) = (ergebnis(&a_klein).await, ergebnis(&a_gross).await);
    assert_eq!(zahl(&klein), 16_970 * 100, "A 30.001, B 60.001: {klein}");
    assert_eq!(zahl(&gross), 16_970 * 100, "A 60.001, B 30.001: {gross}");
}

/// Bei Einzelveranlagung zaehlen die Angaben zum Ehegatten nicht: ein bestaetigter Bezug im Feld des Ehegatten aendert die Steuer
/// nicht (Person A ohne Einkunft: Steuer 0).
#[tokio::test]
async fn bei_einzelveranlagung_zaehlt_der_bezug_des_ehegatten_nicht() {
    let mut paare = paar("einzel", 0, 0);
    paare.extend(versorgung_b(30_001, 30_001, 2025));
    paare.push(("bruttoarbeitslohn_partner", json!(30_001 * 100)));
    let a = ergebnis(&paare).await;
    assert_eq!(zahl(&a), 0, "Einzelveranlagung: {a}");
}
