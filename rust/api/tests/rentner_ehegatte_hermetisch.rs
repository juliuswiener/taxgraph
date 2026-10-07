//! Rentner-Scheibe (`rentner_gesamt`) bei Zusammenveranlagung ueber HTTP: Lohn, Steuerklasse und die fuenf
//! Versorgungsfelder des Ehegatten sind beantwortbar, werden gefragt, und sie aendern die Steuer. Im Standardlauf, ohne
//! `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Die Scheibe fragte Lohn und Versorgung nur fuer Person A (Abweichung Nr. 25). Fuer den Ehegatten fragte
//! sie nichts, und der Ring zaehlte beides als 0.
//!
//! **Warum es zaehlt.** Hat der Ehegatte Lohn oder Pension, fehlte der Betrag in der Steuer. Die Felder allein freizuschalten
//! waere schlimmer gewesen: die Erklaerung waere abgabefaehig gewesen, und 60.000 Euro Lohn haetten die Steuer um 0 Euro
//! erhoeht. Darum steht der Ring zuerst (`bescheid/tests/rentner_ehegatte_hermetisch.rs`) und diese Datei misst die Scheibe
//! auf dem echten Weg `POST /event`, `GET /fragen`, `GET /ergebnis`.
//!
//! **Wo es sitzt.** `SCHEIBEN_RENTNER_GESAMT_FELDER` (`bescheid/src/deklaration/scheiben_tabellen.rs`); der Kegel
//! (28 Pflichtfelder) bleibt, die Angaben zum Ehegatten sind keine neuen Pflichtfragen.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Zahlen sind Vergleiche, keine Konstanten. Der Anker ist die Scheibe `an_gesamt`: eine
//! Rentnerin ohne eigene Einkuenfte, deren Mann einen Lohn hat, zahlt dieselbe Steuer wie der Arbeitnehmer mit demselben Lohn
//! und demselben Ehegatten (zwei Ringe, ein Ergebnis). Die Euro-Betraege von Hand gerechnet stehen in
//! `bescheid/tests/rentner_ehegatte_hermetisch.rs`.
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

/// Die sieben Felder des Ehegatten, die diese Abweichung der Scheibe gibt.
const EHEGATTEN_FELDER: [&str; 7] = [
    "bruttoarbeitslohn_partner",
    "steuerklasse_partner",
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
    sende(d, "POST", "/fall/reh/event", Some(&rumpf)).await
}

/// Ein leerer Fall der Scheibe.
async fn neuer_fall(scheibe: &str) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "reh", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    d
}

/// Ein Fall der Scheibe mit allen `paare`, jedes ueber `POST /event` (muss 201 sein).
async fn fall_mit(scheibe: &str, paare: &[(&'static str, Value)]) -> Dienst {
    let d = neuer_fall(scheibe).await;
    for (feld, wert) in paare {
        let (status, antwort) = event_post(&d, feld, wert).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    d
}

/// Die Antwort von `GET /ergebnis` des Falls `rentner_gesamt` mit allen `paare`.
async fn ergebnis(paare: &[(&'static str, Value)]) -> Value {
    ergebnis_der("rentner_gesamt", paare).await
}

async fn ergebnis_der(scheibe: &str, paare: &[(&'static str, Value)]) -> Value {
    let d = fall_mit(scheibe, paare).await;
    let (status, antwort) = sende(&d, "GET", "/fall/reh/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// Die Feld-IDs der Fragen, die der Fall noch stellt (`GET /fragen`).
async fn fragen_ids(d: &Dienst) -> Vec<String> {
    let (status, fragen) = sende(d, "GET", "/fall/reh/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["feld_id"].as_str().unwrap().to_owned())
        .collect()
}

/// Pflicht-Kegel `rentner_gesamt` (28 Felder) samt `kein_p23_verkauf = true`: Rente `rente_cent` ab 2025,
/// `kein_sonstige = false` (die eigene Rente ist die ehrliche Antwort), `veranlagung` wie angegeben. Zusammen: dazu die
/// fuenf Kapital-Felder des Partner-Kegels auf null.
fn rentner(rente_cent: i64, veranlagung: &'static str) -> Paare {
    let mut p: Paare = vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(rente_cent)),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_grad_der_behinderung", json!(0)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(0)),
        ("rentner_gepflegter_hilflos", json!(false)),
        ("rentner_hinterbliebenenbezuege", json!(false)),
        ("veranlagung", json!(veranlagung)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
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
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
        ("kein_p23_verkauf", json!(true)),
    ];
    if veranlagung == "zusammen" {
        p.extend(partner_kap());
    }
    p
}

fn partner_kap() -> Paare {
    vec![
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ]
}

fn paar(rente_cent: i64) -> Paare {
    rentner(rente_cent, "zusammen")
}

fn mit(mut basis: Paare, mehr: Paare) -> Paare {
    basis.extend(mehr);
    basis
}

fn lohn_partner(euro: i64) -> Paare {
    vec![("bruttoarbeitslohn_partner", json!(euro * 100))]
}

/// Versorgungsbezug des Ehegatten von `euro`, Bemessungsgrundlage gleich dem Bezug, Beginn 2025, beamtenrechtlich.
fn versorgung_partner(euro: i64) -> Paare {
    vec![
        ("versorgung_jahresrente_partner", json!(euro * 100)),
        ("versorgung_bemessungsgrundlage_partner", json!(euro * 100)),
        ("versorgung_beginn_jahr_partner", json!(2025)),
        ("versorgung_art_partner", json!("beamtenrechtlich")),
    ]
}

fn zahl(a: &Value) -> i64 {
    assert_eq!(a["grund"], json!("bestaetigt"), "keine Zahl: {a}");
    a["zahl_cent"]
        .as_i64()
        .unwrap_or_else(|| panic!("keine Zahl: {a}"))
}

/// Die sieben Felder sind auf der Scheibe beantwortbar: `POST /event` antwortet 201. Heute (vorher): 400
/// „nicht in dieser Scheibe".
#[tokio::test]
async fn lohn_steuerklasse_und_die_fuenf_versorgungsfelder_des_ehegatten_sind_beantwortbar() {
    let felder: Paare = vec![
        ("bruttoarbeitslohn_partner", json!(6_000_000)),
        ("steuerklasse_partner", json!("1")),
        ("versorgung_jahresrente_partner", json!(3_000_000)),
        ("versorgung_bemessungsgrundlage_partner", json!(3_000_000)),
        ("versorgung_beginn_jahr_partner", json!(2025)),
        ("versorgung_art_partner", json!("altersgrenze_sonstige")),
        ("versorgung_alter_bei_beginn_partner", json!(63)),
    ];
    assert_eq!(felder.len(), EHEGATTEN_FELDER.len());
    let d = neuer_fall("rentner_gesamt").await;
    for (feld, wert) in &felder {
        let (status, antwort) = event_post(&d, feld, wert).await;
        assert_eq!(
            status, 201,
            "POST /event {feld} auf rentner_gesamt: {antwort}"
        );
    }
}

/// Die Fragen stehen bei Zusammenveranlagung und fehlen bei Einzelveranlagung. Die Alters-Frage steht nur, solange die Art
/// nicht bestaetigt eine andere ist; die Steuerklasse entfaellt bei einem bestaetigten Lohn von 0 (kein Arbeitgeber, keine
/// Bescheinigung).
#[tokio::test]
async fn die_fragen_zum_ehegatten_stehen_nur_bei_zusammenveranlagung() {
    let d = fall_mit("rentner_gesamt", &paar(2_000_000)).await;
    let ids = fragen_ids(&d).await;
    for feld in EHEGATTEN_FELDER {
        assert!(ids.iter().any(|i| i == feld), "{feld} fehlt bei Zusammenveranlagung: {ids:?}");
    }
    let d = fall_mit("rentner_gesamt", &rentner(2_000_000, "einzel")).await;
    let ids = fragen_ids(&d).await;
    for feld in EHEGATTEN_FELDER {
        assert!(!ids.iter().any(|i| i == feld), "{feld} steht bei Einzelveranlagung: {ids:?}");
    }
    // Veranlagung offen: die Frage bleibt (fail-closed).
    let d = neuer_fall("rentner_gesamt").await;
    let ids = fragen_ids(&d).await;
    assert!(ids.iter().any(|i| i == "bruttoarbeitslohn_partner"), "Veranlagung offen: {ids:?}");
}

/// Die Alters-Frage des Ehegatten haengt an seiner Versorgungsart: bei `beamtenrechtlich` entfaellt sie, bei
/// `altersgrenze_sonstige` steht sie. Die Steuerklasse entfaellt bei bestaetigtem Lohn 0 und steht bei Lohn ueber 0.
#[tokio::test]
async fn die_folgefragen_zum_ehegatten_folgen_ihren_bedingungen() {
    let art = |wert: &'static str| mit(paar(2_000_000), vec![("versorgung_art_partner", json!(wert))]);
    let ids = fragen_ids(&fall_mit("rentner_gesamt", &art("beamtenrechtlich")).await).await;
    assert!(!ids.iter().any(|i| i == "versorgung_alter_bei_beginn_partner"), "beamtenrechtlich: {ids:?}");
    let ids = fragen_ids(&fall_mit("rentner_gesamt", &art("altersgrenze_sonstige")).await).await;
    assert!(ids.iter().any(|i| i == "versorgung_alter_bei_beginn_partner"), "altersgrenze_sonstige: {ids:?}");
    let ids = fragen_ids(&fall_mit("rentner_gesamt", &mit(paar(2_000_000), lohn_partner(0))).await).await;
    assert!(!ids.iter().any(|i| i == "steuerklasse_partner"), "Lohn 0: {ids:?}");
    let ids = fragen_ids(&fall_mit("rentner_gesamt", &mit(paar(2_000_000), lohn_partner(6_000))).await).await;
    assert!(ids.iter().any(|i| i == "steuerklasse_partner"), "Lohn 6.000: {ids:?}");
}

/// 6.000, 60.000 und 600.000 Euro Lohn des Ehegatten auf 40.000 Euro Rente: vier verschiedene, steigende Zahlen (heute
/// dieselbe). Bei 20.000 Euro Rente laege der erste Schritt noch unter dem doppelten Grundfreibetrag der Zusammen-
/// veranlagung und die Steuer bei 0.
#[tokio::test]
async fn der_lohn_des_ehegatten_aendert_die_steuer_ueber_http() {
    let ohne = zahl(&ergebnis(&paar(4_000_000)).await);
    let mut steuer = vec![ohne];
    for euro in [6_000, 60_000, 600_000] {
        steuer.push(zahl(&ergebnis(&mit(paar(4_000_000), lohn_partner(euro))).await));
    }
    assert!(
        steuer.windows(2).all(|w| w[0] < w[1]),
        "die Steuer steigt mit dem Lohn des Ehegatten (0, 6.000, 60.000, 600.000 Euro): {steuer:?}"
    );
}

/// 0, 30.000 und 60.000 Euro Versorgung des Ehegatten: 0 Euro aendern nichts, danach steigt die Steuer (heute dreimal
/// dieselbe).
#[tokio::test]
async fn die_versorgung_des_ehegatten_aendert_die_steuer_ueber_http() {
    let ohne = zahl(&ergebnis(&paar(2_000_000)).await);
    let null = zahl(&ergebnis(&mit(paar(2_000_000), versorgung_partner(0))).await);
    let dreissig = zahl(&ergebnis(&mit(paar(2_000_000), versorgung_partner(30_000))).await);
    let sechzig = zahl(&ergebnis(&mit(paar(2_000_000), versorgung_partner(60_000))).await);
    assert_eq!(null, ohne, "0 Euro Versorgung aendern nichts");
    assert!(
        ohne < dreissig && dreissig < sechzig,
        "die Steuer steigt mit der Versorgung des Ehegatten: {ohne} < {dreissig} < {sechzig}"
    );
}

/// Pflicht-Kegel `an_gesamt` (33 Felder) fuer einen Arbeitnehmer mit `euro` Lohn, ZUSAMMEN veranlagt, mit dem Lohn des
/// Ehegatten (`bruttoarbeitslohn_partner`, der Partner-Kegel dieser Scheibe), ohne Pendelstrecke, ohne Kinder, alle vier
/// Kreuze "nein".
fn arbeitnehmer_zusammen(euro: i64, euro_partner: i64) -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(euro * 100)),
        ("bruttoarbeitslohn_partner", json!(euro_partner * 100)),
        ("veranlagung", json!("zusammen")),
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

/// Anker: eine Rentnerin ohne Rente, zusammen veranlagt mit einem Ehegatten mit 40.000 Euro Lohn, zahlt dieselbe Steuer wie
/// das Paar auf der Scheibe `an_gesamt` (Person A ohne Lohn, Ehegatte 40.000 Euro). Die zwei Scheiben rechnen den Lohn des
/// Ehegatten in zwei Ringen; die Zahl ist nicht aus dem Rentner-Ring abgelesen.
#[tokio::test]
async fn der_lohn_des_ehegatten_kostet_dieselbe_steuer_wie_auf_der_arbeitnehmer_scheibe() {
    let rentnerin = ergebnis(&mit(paar(0), lohn_partner(40_000))).await;
    let arbeitnehmer = ergebnis_der("an_gesamt", &arbeitnehmer_zusammen(0, 40_000)).await;
    assert_eq!(zahl(&rentnerin), zahl(&arbeitnehmer), "Rentnerin: {rentnerin}");
    assert!(zahl(&rentnerin) > zahl(&ergebnis(&paar(0)).await), "der Lohn kostet Steuer");
}

/// AK4: Zusammenveranlagung ohne Angabe zum Ehegatten sperrt nicht. Es gibt eine Zahl, und sie ist dieselbe wie mit
/// bestaetigten Nullen.
#[tokio::test]
async fn ohne_angaben_zum_ehegatten_bleibt_die_erklaerung_abgabefaehig() {
    let ohne = ergebnis(&paar(2_000_000)).await;
    let mut nullen = lohn_partner(0);
    nullen.extend(versorgung_partner(0));
    let mit_nullen = ergebnis(&mit(paar(2_000_000), nullen)).await;
    assert_eq!(zahl(&ohne), zahl(&mit_nullen), "fehlende Antwort rechnet wie bestaetigte Null");
}

/// Die Deklaration (`GET /deklaration`, schreibt nichts, reicht nichts ein): Lohn und Steuerklasse des Ehegatten stehen in
/// Anlage N der Person B (E0200201, E0200002). Die Versorgung des Ehegatten hat KEIN Kz, wie die von Person A: ihre fuenf
/// Felder stehen mit Grund in `nicht_deklariert`, nichts verschwindet unsichtbar. Dass der Betrag im Bescheid steht und im
/// XML fehlt, sperrt die Abgabe NICHT (bekannte Luecke, fuer A seit Nr. 25 dieselbe; hier nicht geschlossen, siehe Bericht).
#[tokio::test]
async fn die_deklaration_traegt_lohn_und_steuerklasse_und_nennt_die_versorgung_als_nicht_deklariert() {
    let mut p = mit(paar(2_000_000), lohn_partner(40_000));
    p.push(("steuerklasse_partner", json!("1")));
    p.extend(versorgung_partner(30_000));
    let d = fall_mit("rentner_gesamt", &p).await;
    let (status, dekl) = sende(&d, "GET", "/fall/reh/deklaration", None).await;
    assert_eq!(status, 200, "{dekl}");
    assert_eq!(dekl["person_b"]["E0200201"], json!(40_000), "Lohn des Ehegatten: {}", dekl["person_b"]);
    assert_eq!(dekl["person_b"]["E0200002"], json!("1"), "Steuerklasse des Ehegatten: {}", dekl["person_b"]);
    let nicht: Vec<&str> = dekl["nicht_deklariert"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["feld_id"].as_str().unwrap())
        .collect();
    // Die vier beantworteten Versorgungsfelder (die Alters-Frage entfaellt bei `beamtenrechtlich`).
    for feld in &EHEGATTEN_FELDER[2..6] {
        assert!(nicht.contains(feld), "{feld} fehlt in nicht_deklariert: {nicht:?}");
    }
}

/// Ein Versorgungsbezug des Ehegatten ohne Beginnjahr und Bemessungsgrundlage sperrt mit dem bestehenden Grund
/// `versorgungsfreibetrag_offen` (kein neuer Sperrgrund); vollstaendig gibt es eine Zahl.
#[tokio::test]
async fn ein_unvollstaendiger_versorgungsbezug_des_ehegatten_sperrt_mit_dem_bestehenden_grund() {
    let nur_bezug = mit(paar(2_000_000), vec![("versorgung_jahresrente_partner", json!(3_000_000))]);
    let a = ergebnis(&nur_bezug).await;
    assert_eq!(a["grund"], json!("versorgungsfreibetrag_offen"), "{a}");
    assert!(a["zahl_cent"].is_null(), "gesperrt heisst keine Zahl: {a}");
}
