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
//! **Abweichung Nr. 36.** Zum Lohn des Ehegatten gehoeren seine einbehaltene Lohnsteuer (`p36_lohnsteuer_partner`) und sein
//! Geburtsjahr (`geburtsjahr_partner`, fuer den Altersentlastungsbetrag nach § 24a): beide sind jetzt auf der Scheibe
//! beantwortbar, und der Ring rechnet § 24a je Person wie `gesamt`. Der Ring zuerst: `bescheid/tests/rentner_ehegatte_hermetisch.rs`.
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

/// Ein reiner Pensionaer als Ehegatte (Abweichung Nr. 50): Nr. 3 der Lohnsteuerbescheinigung (Bruttoarbeitslohn) enthaelt den
/// Bezug aus Nr. 8, beide sind `euro`. Ein Bezug ohne so hohen Lohn sperrt (`versorgung_ueber_lohn`).
fn pensionaer_partner(euro: i64) -> Paare {
    [lohn_partner(euro), versorgung_partner(euro)].concat()
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
    let null = zahl(&ergebnis(&mit(paar(2_000_000), pensionaer_partner(0))).await);
    let dreissig = zahl(&ergebnis(&mit(paar(2_000_000), pensionaer_partner(30_000))).await);
    let sechzig = zahl(&ergebnis(&mit(paar(2_000_000), pensionaer_partner(60_000))).await);
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

/// Geburtsjahr des Ehegatten (`geburtsjahr_partner`, Abweichung Nr. 36).
fn geboren_partner(jahr: i64) -> Paare {
    vec![("geburtsjahr_partner", json!(jahr))]
}

/// Abweichung Nr. 36, AK1: Der Altersentlastungsbetrag (§ 24a `EStG`) des Ehegatten senkt die Steuer. Ehegatte mit 20.000 Euro
/// Lohn auf 40.000 Euro Rente, geboren 1955 (VZ 2025: Kohorte 2020, 16,0 %, hoechstens 760 Euro) gegen geboren 1990: die
/// Steuer des Aelteren ist kleiner (heute gleich, der Rentner-Ring rechnet § 24a nur fuer Person A).
#[tokio::test]
async fn der_altersentlastungsbetrag_des_ehegatten_senkt_die_steuer_ueber_http() {
    let fall = |jahr| mit(mit(paar(4_000_000), lohn_partner(20_000)), geboren_partner(jahr));
    let jung = zahl(&ergebnis(&fall(1990)).await);
    let alt = zahl(&ergebnis(&fall(1955)).await);
    assert!(alt < jung, "der Altersentlastungsbetrag des Ehegatten senkt die Steuer: alt {alt} < jung {jung}");
}

/// Pflicht-Kegel `gesamt` (35 Felder) fuer eine Person A ohne jede Einkunft, ZUSAMMEN veranlagt, mit dem Lohn des Ehegatten
/// (`bruttoarbeitslohn_partner`), alle Kreuze "nein"/"kein".
fn gesamt_zusammen(euro_partner: i64) -> Paare {
    let mut p: Paare = vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(100)),
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(0)),
        ("bruttoarbeitslohn_partner", json!(euro_partner * 100)),
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
        ("versicherungsart_partner", json!("gesetzlich_an")),
    ];
    p.extend(partner_kap());
    p
}

/// Anker AK1: eine Rentnerin ohne Rente, Ehegatte 60.000 Euro Lohn, geboren 1955, zahlt dieselbe Steuer wie das Paar auf der
/// Scheibe `gesamt` (Person A ohne Einkunft, Ehegatte 60.000 Euro, geboren 1955). Der Rechenweg von `gesamt` rechnet § 24a je
/// Person schon heute; der Anker ist die Zahl dort, nicht der Rentner-Ring. 60.000 Euro, weil bei 20.000 Euro beide Steuern 0
/// waeren (doppelter Grundfreibetrag) und der Vergleich nichts maesse.
#[tokio::test]
async fn der_altersentlastungsbetrag_des_ehegatten_ist_derselbe_wie_auf_der_scheibe_gesamt() {
    let rentnerin = ergebnis(&mit(mit(paar(0), lohn_partner(60_000)), geboren_partner(1955))).await;
    let gesamt = ergebnis_der("gesamt", &mit(gesamt_zusammen(60_000), geboren_partner(1955))).await;
    assert_eq!(zahl(&rentnerin), zahl(&gesamt), "Rentnerin: {rentnerin}");
    // Kontrolle: der Anker misst etwas. Ohne Geburtsjahr zahlt das Paar auf `gesamt` mehr, und die Steuer ist nicht 0.
    let ohne = ergebnis_der("gesamt", &gesamt_zusammen(60_000)).await;
    assert!(zahl(&gesamt) > 0, "Steuer ueber 0: {gesamt}");
    assert!(zahl(&gesamt) < zahl(&ohne), "der Altersentlastungsbetrag senkt die Steuer auf gesamt: {gesamt} < {ohne}");
}

/// AK2: Die Lohnsteuer des Ehegatten ist beantwortbar und wird nur gefragt, wenn der Ehegatte Lohn hat (Bedingung der
/// Bindung, `feld_bedingung` auf `bruttoarbeitslohn_partner`, nicht gleich 0) und nur bei Zusammenveranlagung. Sie aendert die
/// Steuer (`zahl_cent`) nicht, wohl aber die Abschlusszahlung (Abweichung Nr. 38, Test weiter unten).
#[tokio::test]
async fn die_lohnsteuer_des_ehegatten_wird_nur_bei_seinem_lohn_und_nur_zusammen_gefragt() {
    let ohne_lohn = fragen_ids(&fall_mit("rentner_gesamt", &mit(paar(2_000_000), lohn_partner(0))).await).await;
    assert!(!ohne_lohn.iter().any(|i| i == "p36_lohnsteuer_partner"), "Lohn 0: {ohne_lohn:?}");
    let mit_lohn = fragen_ids(&fall_mit("rentner_gesamt", &mit(paar(2_000_000), lohn_partner(6_000))).await).await;
    assert!(mit_lohn.iter().any(|i| i == "p36_lohnsteuer_partner"), "Lohn 6.000: {mit_lohn:?}");
    let einzel = fragen_ids(&fall_mit("rentner_gesamt", &rentner(2_000_000, "einzel")).await).await;
    assert!(!einzel.iter().any(|i| i == "p36_lohnsteuer_partner"), "Einzelveranlagung: {einzel:?}");
    let d = fall_mit("rentner_gesamt", &mit(paar(2_000_000), lohn_partner(6_000))).await;
    let (status, antwort) = event_post(&d, "p36_lohnsteuer_partner", &json!(50_000)).await;
    assert_eq!(status, 201, "POST /event p36_lohnsteuer_partner: {antwort}");
}

/// AK2: Das Geburtsjahr des Ehegatten ist beantwortbar und steht bei Zusammenveranlagung in den Fragen, bei Einzelveranlagung
/// nicht. Es steht EINMAL: ist das Geburtsdatum des Ehegatten bekannt, leitet der Speicher das Jahr ab und die Frage faellt weg
/// (Ableitung `jahr_aus_datum`, dieselbe wie auf `gesamt`).
#[tokio::test]
async fn das_geburtsjahr_des_ehegatten_wird_einmal_gefragt() {
    let ids = fragen_ids(&fall_mit("rentner_gesamt", &paar(2_000_000)).await).await;
    assert!(ids.iter().any(|i| i == "geburtsjahr_partner"), "Zusammenveranlagung: {ids:?}");
    let ids = fragen_ids(&fall_mit("rentner_gesamt", &rentner(2_000_000, "einzel")).await).await;
    assert!(!ids.iter().any(|i| i == "geburtsjahr_partner"), "Einzelveranlagung: {ids:?}");
    let d = fall_mit(
        "rentner_gesamt",
        &mit(paar(2_000_000), vec![("stammdaten_geburtsdatum_partner", json!("01.01.1955"))]),
    )
    .await;
    let ids = fragen_ids(&d).await;
    assert!(!ids.iter().any(|i| i == "geburtsjahr_partner"), "Geburtsdatum bekannt, Jahr abgeleitet: {ids:?}");
    // Und die Ableitung zaehlt: das abgeleitete Jahr senkt die Steuer wie das direkt genannte.
    let direkt = zahl(&ergebnis(&mit(mit(paar(4_000_000), lohn_partner(20_000)), geboren_partner(1955))).await);
    let abgeleitet = zahl(
        &ergebnis(&mit(
            mit(paar(4_000_000), lohn_partner(20_000)),
            vec![("stammdaten_geburtsdatum_partner", json!("01.01.1955"))],
        ))
        .await,
    );
    assert_eq!(abgeleitet, direkt, "Geburtsdatum 1955 gleich Geburtsjahr 1955");
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
/// Anlage N der Person B (E0200201, E0200002). Der Bezug des Ehegatten steht seit Abweichung Nr. 50 in den Zeilen 11 bis 13
/// der zweiten Anlage N (E0200801, E0200902, E0201307, wie bei Person A); die Art (Weiche des Alters-Gates) ist kein Feld der
/// Erklaerung und bleibt mit Grund in `nicht_deklariert`, nichts verschwindet unsichtbar. Der Bezug des Ehegatten beginnt hier
/// 2025, im Veranlagungsjahr: die Abgabe bleibt gesperrt (Abweichung Nr. 42, seit Nr. 64 nur noch wegen Zeile 14), und
/// `unvollstaendig` nennt den bestaetigten Betrag des Ehegatten bei Zusammenveranlagung (die Sperre selbst prueft
/// `versorgung_abgabe_sperre_hermetisch.rs`).
#[tokio::test]
async fn die_deklaration_traegt_lohn_steuerklasse_und_die_versorgungszeilen_und_nennt_die_art_als_nicht_deklariert() {
    let mut p = mit(paar(2_000_000), lohn_partner(40_000));
    p.push(("steuerklasse_partner", json!("1")));
    p.push(("p36_lohnsteuer_partner", json!(600_000)));
    p.extend(versorgung_partner(30_000));
    let d = fall_mit("rentner_gesamt", &p).await;
    let (status, dekl) = sende(&d, "GET", "/fall/reh/deklaration", None).await;
    assert_eq!(status, 200, "{dekl}");
    assert_eq!(dekl["person_b"]["E0200201"], json!(40_000), "Lohn des Ehegatten: {}", dekl["person_b"]);
    assert_eq!(dekl["person_b"]["E0200002"], json!("1"), "Steuerklasse des Ehegatten: {}", dekl["person_b"]);
    // Abweichung Nr. 36: die einbehaltene Lohnsteuer des Ehegatten (Anlage N, Person B) steht jetzt auch in der Erklaerung.
    assert_eq!(dekl["person_b"]["E0200301"], json!("6000,00"), "Lohnsteuer des Ehegatten: {}", dekl["person_b"]);
    let nicht: Vec<&str> = dekl["nicht_deklariert"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["feld_id"].as_str().unwrap())
        .collect();
    // Die drei Zeilen des Bezugs (Abweichung Nr. 50): Betrag, Bemessungsgrundlage, Beginnjahr; der Ring-Wert ist kein Feld.
    assert_eq!(dekl["person_b"]["E0200801"], json!(30_000), "Bezug des Ehegatten: {}", dekl["person_b"]);
    assert_eq!(dekl["person_b"]["E0200902"], json!(30_000), "Bemessungsgrundlage des Ehegatten: {}", dekl["person_b"]);
    assert_eq!(dekl["person_b"]["E0201307"], json!(2025), "Beginnjahr des Ehegatten: {}", dekl["person_b"]);
    assert!(dekl["deklaration"].get("E0200801").is_none(), "nichts bei Person A: {}", dekl["deklaration"]);
    // Die drei Felder stehen nicht mehr in `nicht_deklariert`, die Art (Weiche) schon; die Alters-Frage entfaellt bei `beamtenrechtlich`.
    for feld in &EHEGATTEN_FELDER[2..5] {
        assert!(!nicht.contains(feld), "{feld} steht in der Erklaerung, nicht in nicht_deklariert: {nicht:?}");
    }
    assert!(nicht.contains(&"versorgung_art_partner"), "die Art ist kein Feld der Erklaerung: {nicht:?}");
    let offen: Vec<&str> = dekl["unvollstaendig"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["feld_id"].as_str().unwrap())
        .collect();
    assert!(
        offen.contains(&"versorgung_jahresrente_partner"),
        "der Bezug des Ehegatten mit Beginn im Veranlagungsjahr sperrt die Abgabe nicht: {offen:?}"
    );
    let grund = dekl["unvollstaendig"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["feld_id"] == json!("versorgung_jahresrente_partner"))
        .map(|e| e["grund"].as_str().unwrap().to_owned())
        .unwrap();
    assert!(grund.contains("Zeile 14"), "die Sperre nennt Zeile 14 nicht: {grund}");
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

/// Abweichung Nr. 37: Fehlt beim Ehegatten das Beginnjahr, die Bemessungsgrundlage oder beides, nennt die Meldung (`klartext`
/// in `GET /ergebnis`) den Ehegatten und sagt, dass die zwei Angaben je Person gelten. Der Grund ist derselbe wie fuer
/// Person A (keine neue Kennung), und die Meldung ist fuer jede der drei Luecken dieselbe.
#[tokio::test]
async fn die_meldung_zu_fehlenden_versorgungsangaben_nennt_den_ehegatten() {
    let ohne = |feld: &str| -> Paare { versorgung_partner(30_000).into_iter().filter(|(f, _)| *f != feld).collect() };
    let nur_bezug: Paare = vec![("versorgung_jahresrente_partner", json!(3_000_000))];
    let luecken = [
        ("Beginnjahr und Bemessung fehlen", nur_bezug),
        ("Bemessung fehlt", ohne("versorgung_bemessungsgrundlage_partner")),
        ("Beginnjahr fehlt", ohne("versorgung_beginn_jahr_partner")),
    ];
    for (name, versorgung) in luecken {
        let a = ergebnis(&mit(paar(2_000_000), versorgung)).await;
        assert_eq!(a["grund"], json!("versorgungsfreibetrag_offen"), "{name}: {a}");
        assert!(a["zahl_cent"].is_null(), "{name}: gesperrt heisst keine Zahl: {a}");
        let klartext = a["klartext"].as_str().unwrap_or_else(|| panic!("{name}: kein Klartext: {a}"));
        assert!(klartext.contains("deines Ehegatten"), "{name}: die Meldung nennt den Ehegatten nicht: {klartext}");
        assert!(klartext.contains("gemeinsamer Veranlagung"), "{name}: {klartext}");
        assert!(klartext.contains("jede Person einzeln"), "{name}: {klartext}");
    }
}

/// Abweichung Nr. 38: Lohnsteuer (5.000 EUR) und Vorauszahlungen (1.000 EUR) von Person A.
fn anrechnung_a() -> Paare {
    vec![("p36_lohnsteuer", json!(500_000)), ("p36_vorauszahlungen", json!(100_000))]
}

/// Die einbehaltene Lohnsteuer des Ehegatten (`p36_lohnsteuer_partner`) in EURO.
fn lohnsteuer_partner(euro: i64) -> Paare {
    vec![("p36_lohnsteuer_partner", json!(euro * 100))]
}

fn abschlusszahlung(a: &Value) -> i64 {
    assert_eq!(a["grund"], json!("bestaetigt"), "keine Zahl: {a}");
    a["abschlusszahlung_cent"].as_i64().unwrap_or_else(|| panic!("keine Abschlusszahlung: {a}"))
}

/// Abweichung Nr. 38, AK4: Bei Zusammenveranlagung zieht `abschlusszahlung_cent` in `GET /ergebnis` die Lohnsteuer des Ehegatten
/// ab, auf der Rentner-Scheibe wie auf `gesamt`. 3.000 EUR Lohnsteuer des Ehegatten senken die Abschlusszahlung um genau 3.000 EUR,
/// die Steuer (`zahl_cent`) bleibt gleich. Kontrolle: ohne den Ehegatten ist die Abschlusszahlung die Steuer minus 6.000 EUR.
#[tokio::test]
async fn die_lohnsteuer_des_ehegatten_senkt_die_abschlusszahlung_auf_rentner_und_gesamt() {
    let rentner_basis = mit(mit(paar(4_000_000), lohn_partner(20_000)), anrechnung_a());
    let gesamt_basis = mit(gesamt_zusammen(60_000), anrechnung_a());
    for (scheibe, basis) in [("rentner_gesamt", rentner_basis), ("gesamt", gesamt_basis)] {
        let ohne = ergebnis_der(scheibe, &basis).await;
        let mit_lst = ergebnis_der(scheibe, &mit(basis.clone(), lohnsteuer_partner(3_000))).await;
        assert_eq!(zahl(&mit_lst), zahl(&ohne), "{scheibe}: die Lohnsteuer des Ehegatten aendert die Steuer nicht");
        assert_eq!(abschlusszahlung(&ohne), zahl(&ohne) - 600_000, "{scheibe}: Kontrolle ohne den Ehegatten: {ohne}");
        assert_eq!(
            abschlusszahlung(&ohne) - abschlusszahlung(&mit_lst),
            300_000,
            "{scheibe}: die Lohnsteuer des Ehegatten fehlt in der Abschlusszahlung. ohne: {ohne} mit: {mit_lst}"
        );
    }
}

/// Abweichung Nr. 38, AK3: Bei Einzelveranlagung zaehlt ein bestaetigter Wert im Feld des Ehegatten nicht. Die Abschlusszahlung
/// ist mit und ohne ihn dieselbe.
#[tokio::test]
async fn bei_einzelveranlagung_zaehlt_die_lohnsteuer_des_ehegatten_nicht() {
    let basis = mit(rentner(4_000_000, "einzel"), anrechnung_a());
    let ohne = ergebnis(&basis).await;
    let mit_lst = ergebnis(&mit(basis, lohnsteuer_partner(3_000))).await;
    assert_eq!(abschlusszahlung(&ohne), zahl(&ohne) - 600_000, "Kontrolle: Person A allein: {ohne}");
    assert_eq!(abschlusszahlung(&mit_lst), abschlusszahlung(&ohne), "einzeln veranlagt: der Wert des Ehegatten zaehlt nicht. {mit_lst}");
}

/// Ein nur vorlaeufiger Wert ueber `POST /event`, als Vorschlag, den der Nutzer noch nicht bestaetigt hat.
async fn event_vorlaeufig(d: &Dienst, feld: &str, wert: &Value) -> (u16, Value) {
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "vorlaeufig", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": null},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    sende(d, "POST", "/fall/reh/event", Some(&rumpf)).await
}

/// Wie `event_vorlaeufig`, mit dem Schreiber und der Herkunft eines Vorschlags (`llm:`, `import:`, `beleg:`).
async fn event_vorschlag_von(d: &Dienst, schreiber: &str, herkunft: &str, feld: &str, wert: &Value) -> (u16, Value) {
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "vorlaeufig", "schreiber": schreiber,
        "herkunft": {"herkunft": herkunft, "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": null},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    sende(d, "POST", "/fall/reh/event", Some(&rumpf)).await
}

/// Die Bestaetigung eines vorlaeufigen Werts: ein bestaetigtes Event, das das vorlaeufige ersetzt.
async fn event_bestaetigt_ersetzt(d: &Dienst, feld: &str, wert: &Value, ersetzt: &str) -> (u16, Value) {
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie", "ersetzt": ersetzt,
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    sende(d, "POST", "/fall/reh/event", Some(&rumpf)).await
}

const KREUZ_PARTNER: &str = "kein_lohn_pension_partner";
const LOHN_PARTNER: &str = "bruttoarbeitslohn_partner";

/// Die Lohnfrage des Ehegatten steht in den Fragen des Falls.
async fn lohnfrage_steht(d: &Dienst) -> bool {
    fragen_ids(d).await.iter().any(|f| f == LOHN_PARTNER)
}

/// Abweichung Nr. 76: Steht im Fall ein nur vorlaeufiger Lohn des Ehegatten (ein Vorschlag, den der Nutzer noch nicht bestaetigt
/// hat), weist der Dienst das bestaetigte Kreuz "Ehegatte ohne Lohn und Pension" mit 400 ab. Das Kreuz verbaergt die Lohnfrage, und
/// ohne die Frage kaeme die Bestaetigung nie: die Zahl bliebe in `ring_betrag_vorlaeufig` gesperrt, ohne dass eine Frage sie loest.
/// Heute (vorher): 201, die Lohnfrage fehlt, `GET /ergebnis` nennt den Lohn als offen.
#[tokio::test]
async fn das_kreuz_ohne_lohn_des_ehegatten_wird_bei_nur_vorlaeufigem_lohn_mit_400_abgewiesen() {
    let lohn = json!(3_800_000);
    let d = fall_mit("rentner_gesamt", &paar(4_000_000)).await;
    let (status, vorschlag) = event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await;
    assert_eq!(status, 201, "Vorschlag fuer den Lohn: {vorschlag}");
    assert!(lohnfrage_steht(&d).await, "KONTROLLE: mit dem Vorschlag steht die Lohnfrage");

    let (status, antwort) = event_post(&d, KREUZ_PARTNER, &json!(true)).await;
    assert_eq!(status, 400, "DEFEKT: das Kreuz neben einem vorlaeufigen Lohn wird angenommen: {antwort}");
    let fehler = antwort["fehler"].as_str().unwrap_or_default();
    assert!(
        fehler.contains(KREUZ_PARTNER) && fehler.contains(LOHN_PARTNER),
        "die Abweisung nennt beide Felder, damit der Nutzer weiss, was zu tun ist: {antwort}"
    );

    // Die Abweisung speichert nichts: die Lohnfrage steht weiter, die Zahl wartet auf die Bestaetigung.
    assert!(lohnfrage_steht(&d).await, "nach der Abweisung fehlt die Lohnfrage");
    let (_, erg) = sende(&d, "GET", "/fall/reh/ergebnis", None).await;
    assert_eq!(erg["grund"], json!("ring_betrag_vorlaeufig"), "{erg}");
    assert_eq!(erg["offen"], json!([LOHN_PARTNER]), "{erg}");

    // Mit dem bestaetigten Lohn geht das Kreuz (Abweichung Nr. 46: ein bestaetigter Lohn zaehlt weiter), die Zahl ist da.
    let ersetzt = vorschlag["event_id"].as_str().unwrap();
    let (status, antwort) = event_bestaetigt_ersetzt(&d, LOHN_PARTNER, &lohn, ersetzt).await;
    assert_eq!(status, 201, "Lohn bestaetigen: {antwort}");
    let (status, antwort) = event_post(&d, KREUZ_PARTNER, &json!(true)).await;
    assert_eq!(status, 201, "Kreuz nach der Bestaetigung: {antwort}");
    let (_, erg) = sende(&d, "GET", "/fall/reh/ergebnis", None).await;
    assert!(zahl(&erg) > 0, "{erg}");
}

/// Die Abweisung greift nur in dem einen Fall. Kontrollen, jede mit einem eigenen Fall: ohne Lohn des Ehegatten, mit verneintem
/// Kreuz, mit nur vorlaeufigem Kreuz (es schliesst nichts aus) und mit bestaetigtem Lohn (Abweichung Nr. 46) nimmt der Dienst das
/// Kreuz an (201).
#[tokio::test]
async fn das_kreuz_ohne_lohn_des_ehegatten_bleibt_sonst_erlaubt() {
    let lohn = json!(3_800_000);
    let kreuz_ja = json!(true);

    // Ohne Lohn des Ehegatten: das Kreuz verbirgt die Lohnfrage, wie es soll.
    let d = fall_mit("rentner_gesamt", &paar(4_000_000)).await;
    assert!(lohnfrage_steht(&d).await, "KONTROLLE: ohne Kreuz steht die Lohnfrage");
    let (status, antwort) = event_post(&d, KREUZ_PARTNER, &kreuz_ja).await;
    assert_eq!(status, 201, "ohne Lohn des Ehegatten: {antwort}");
    assert!(!lohnfrage_steht(&d).await, "das Kreuz verbirgt die Lohnfrage");

    // Mit vorlaeufigem Lohn: das Kreuz "nein" (der Ehegatte hat Lohn) ist die richtige Antwort.
    let d = fall_mit("rentner_gesamt", &paar(4_000_000)).await;
    assert_eq!(event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await.0, 201);
    let (status, antwort) = event_post(&d, KREUZ_PARTNER, &json!(false)).await;
    assert_eq!(status, 201, "Kreuz nein neben vorlaeufigem Lohn: {antwort}");
    assert!(lohnfrage_steht(&d).await);

    // Ein anderes Kreuz (das der Hauptperson) neben dem vorlaeufigen Lohn des Ehegatten geht: die Abweisung gilt nur diesem Kreuz.
    let d = fall_mit("rentner_gesamt", &paar(4_000_000)).await;
    assert_eq!(event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await.0, 201);
    let (status, antwort) = event_post(&d, "kein_lohn_pension", &kreuz_ja).await;
    assert_eq!(status, 201, "anderes Kreuz neben vorlaeufigem Lohn des Ehegatten: {antwort}");
    assert!(lohnfrage_steht(&d).await, "das Kreuz der Hauptperson verbirgt die Lohnfrage des Ehegatten nicht");

    // Ein nur vorlaeufiges Kreuz schliesst die Lohnfrage nicht aus.
    let d = fall_mit("rentner_gesamt", &paar(4_000_000)).await;
    assert_eq!(event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await.0, 201);
    let (status, antwort) = event_vorlaeufig(&d, KREUZ_PARTNER, &kreuz_ja).await;
    assert_eq!(status, 201, "vorlaeufiges Kreuz neben vorlaeufigem Lohn: {antwort}");
    assert!(lohnfrage_steht(&d).await, "ein vorlaeufiges Kreuz verbirgt die Lohnfrage nicht");

    // Mit bestaetigtem Lohn: das Kreuz geht, die Zahl bleibt die ohne Kreuz (Abweichung Nr. 46).
    let ohne = ergebnis(&mit(paar(4_000_000), lohn_partner(38_000))).await;
    let mit_kreuz = ergebnis(&mit(paar(4_000_000), [lohn_partner(38_000), vec![(KREUZ_PARTNER, kreuz_ja)]].concat())).await;
    assert_eq!(zahl(&ohne), zahl(&mit_kreuz), "bestaetigter Lohn neben dem Kreuz: {mit_kreuz}");
}

/// Abweichung Nr. 77 (Julius, Entscheidung d-14, 2026-10-15): die Gegenrichtung zu Nr. 76. Das Kreuz steht schon, dann kommt der
/// Lohn des Ehegatten als Vorschlag (KI, Vorjahr, Beleg) an. Vorher antwortete `POST /event` 201, die Lohnfrage stand nicht in den
/// Fragen, und `GET /ergebnis` blieb in `ring_betrag_vorlaeufig`: dieselbe Sackgasse auf dem anderen Weg (gemessen 2026-10-15 am
/// Stand nach Nr. 76, der alte Test `ein_vorschlag_fuer_den_lohn_nach_dem_kreuz_baut_keine_sackgasse` war rot). Jetzt weist der
/// Dienst den Vorschlag mit 400 ab und nennt beide Felder. Mit dem zurueckgenommenen Kreuz geht er, und die Lohnfrage steht.
#[tokio::test]
async fn ein_vorlaeufiger_lohn_des_ehegatten_wird_bei_gesetztem_kreuz_mit_400_abgewiesen() {
    let lohn = json!(3_800_000);
    let d = fall_mit("rentner_gesamt", &paar(4_000_000)).await;
    let (status, kreuz) = event_post(&d, KREUZ_PARTNER, &json!(true)).await;
    assert_eq!(status, 201, "Kreuz: {kreuz}");
    assert!(!lohnfrage_steht(&d).await, "KONTROLLE: das Kreuz verbirgt die Lohnfrage");
    let (_, vorher) = sende(&d, "GET", "/fall/reh/ergebnis", None).await;
    let zahl_vorher = zahl(&vorher);

    let (status, antwort) = event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await;
    assert_eq!(status, 400, "DEFEKT: der Vorschlag fuer den Lohn neben dem Kreuz wird angenommen: {antwort}");
    let fehler = antwort["fehler"].as_str().unwrap_or_default();
    assert!(
        fehler.contains(KREUZ_PARTNER) && fehler.contains(LOHN_PARTNER),
        "die Abweisung nennt beide Felder, damit der Nutzer weiss, was zu tun ist: {antwort}"
    );

    // Jeder Schreiber eines Vorschlags geht durch dieselbe Tuer: Vorjahr, Beleg, KI.
    for (schreiber, herkunft) in [("import:vorjahr", "vorjahr"), ("import:beleg", "beleg_import"), ("llm:chat", "llm_vorschlag")] {
        let (status, antwort) = event_vorschlag_von(&d, schreiber, herkunft, LOHN_PARTNER, &lohn).await;
        assert_eq!(status, 400, "DEFEKT: der Vorschlag von {schreiber} neben dem Kreuz wird angenommen: {antwort}");
    }

    // Die Abweisung speichert nichts: das Kreuz gilt weiter, die Zahl bleibt die von vorher.
    assert!(!lohnfrage_steht(&d).await, "die Abweisung hat die Lohnfrage veraendert");
    let (_, nachher) = sende(&d, "GET", "/fall/reh/ergebnis", None).await;
    assert_eq!(zahl(&nachher), zahl_vorher, "{nachher}");

    // Der Ausweg, den die Meldung nennt: das Kreuz zuruecknehmen. Dann geht der Vorschlag, und die Frage steht.
    let ersetzt = kreuz["event_id"].as_str().unwrap();
    let (status, antwort) = event_bestaetigt_ersetzt(&d, KREUZ_PARTNER, &json!(false), ersetzt).await;
    assert_eq!(status, 201, "Kreuz zuruecknehmen: {antwort}");
    let (status, antwort) = event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await;
    assert_eq!(status, 201, "Vorschlag nach dem zurueckgenommenen Kreuz: {antwort}");
    assert!(lohnfrage_steht(&d).await, "nach dem zurueckgenommenen Kreuz steht die Lohnfrage");
    let (_, erg) = sende(&d, "GET", "/fall/reh/ergebnis", None).await;
    assert_eq!(erg["grund"], json!("ring_betrag_vorlaeufig"), "{erg}");
    assert_eq!(erg["offen"], json!([LOHN_PARTNER]), "{erg}");
}

/// Die Abweisung der Gegenrichtung greift nur in dem einen Fall. Kontrollen, jede mit einem eigenen Fall: ohne Kreuz, mit
/// verneintem Kreuz, mit nur vorlaeufigem Kreuz (es verbirgt nichts), mit dem Kreuz der Hauptperson und mit bestaetigtem Lohn
/// nach dem Kreuz (Abweichung Nr. 46: er zaehlt weiter) nimmt der Dienst den Lohn an (201). Auf `gesamt` und `an_gesamt`
/// laesst sich das Kreuz nicht setzen (400 an der Tuer); dort gilt die Regel nie, der Vorschlag geht und die Lohnfrage bleibt.
#[tokio::test]
async fn ein_vorlaeufiger_lohn_des_ehegatten_bleibt_sonst_erlaubt() {
    let lohn = json!(3_800_000);

    // Ohne Kreuz: die Lohnfrage steht, der Vorschlag geht.
    let d = fall_mit("rentner_gesamt", &paar(4_000_000)).await;
    assert!(lohnfrage_steht(&d).await, "KONTROLLE: ohne Kreuz steht die Lohnfrage");
    let (status, antwort) = event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await;
    assert_eq!(status, 201, "ohne Kreuz: {antwort}");

    // Verneintes Kreuz (der Ehegatte hat Lohn): die richtige Antwort neben dem Vorschlag.
    let d = fall_mit("rentner_gesamt", &mit(paar(4_000_000), vec![(KREUZ_PARTNER, json!(false))])).await;
    let (status, antwort) = event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await;
    assert_eq!(status, 201, "Kreuz nein: {antwort}");
    assert!(lohnfrage_steht(&d).await);

    // Ein nur vorlaeufiges Kreuz verbirgt die Lohnfrage nicht.
    let d = fall_mit("rentner_gesamt", &paar(4_000_000)).await;
    assert_eq!(event_vorlaeufig(&d, KREUZ_PARTNER, &json!(true)).await.0, 201);
    let (status, antwort) = event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await;
    assert_eq!(status, 201, "vorlaeufiges Kreuz: {antwort}");
    assert!(lohnfrage_steht(&d).await, "ein vorlaeufiges Kreuz verbirgt die Lohnfrage nicht");

    // Das Kreuz der Hauptperson verbirgt die Lohnfrage des Ehegatten nicht.
    let d = fall_mit("rentner_gesamt", &mit(paar(4_000_000), vec![("kein_lohn_pension", json!(true))])).await;
    let (status, antwort) = event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await;
    assert_eq!(status, 201, "anderes Kreuz: {antwort}");
    assert!(lohnfrage_steht(&d).await);

    // Das Kreuz gilt nur dem Lohn: ein anderes vorlaeufiges Feld neben dem Kreuz geht (hier das Kreuz der Hauptperson).
    let d = fall_mit("rentner_gesamt", &mit(paar(4_000_000), vec![(KREUZ_PARTNER, json!(true))])).await;
    let (status, antwort) = event_vorlaeufig(&d, "kein_lohn_pension", &json!(true)).await;
    assert_eq!(status, 201, "anderes Feld neben dem Kreuz: {antwort}");

    // Bestaetigter Lohn nach dem Kreuz (Nr. 46): geht, und die Zahl ist die ohne Kreuz.
    let ohne = ergebnis(&mit(paar(4_000_000), lohn_partner(38_000))).await;
    let d = fall_mit("rentner_gesamt", &mit(paar(4_000_000), vec![(KREUZ_PARTNER, json!(true))])).await;
    let (status, antwort) = event_post(&d, LOHN_PARTNER, &lohn).await;
    assert_eq!(status, 201, "bestaetigter Lohn nach dem Kreuz: {antwort}");
    let (_, mit_kreuz) = sende(&d, "GET", "/fall/reh/ergebnis", None).await;
    assert_eq!(zahl(&ohne), zahl(&mit_kreuz), "bestaetigter Lohn nach dem Kreuz: {mit_kreuz}");

    // `gesamt` und `an_gesamt` fragen das Kreuz nicht: es ist dort ein 400 an der Tuer, der Vorschlag geht.
    for scheibe in ["gesamt", "an_gesamt"] {
        let d = fall_mit(scheibe, &[("veranlagung", json!("zusammen"))]).await;
        let (status, antwort) = event_post(&d, KREUZ_PARTNER, &json!(true)).await;
        assert_eq!(status, 400, "KONTROLLE: das Kreuz ist auf {scheibe} nicht setzbar: {antwort}");
        let (status, antwort) = event_vorlaeufig(&d, LOHN_PARTNER, &lohn).await;
        assert_eq!(status, 201, "{scheibe}: {antwort}");
        assert!(lohnfrage_steht(&d).await, "{scheibe}: die Lohnfrage bleibt");
    }
}
