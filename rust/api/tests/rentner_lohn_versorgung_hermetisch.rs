//! Rentner-Scheibe (`rentner_gesamt`) ueber HTTP: Bruttoarbeitslohn, Steuerklasse und die fuenf Versorgungsfelder sind
//! beantwortbar, und sie aendern die Steuer. Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Die Scheibe fragte die Lohnsteuer, aber nie den Bruttoarbeitslohn und die Steuerklasse, die das
//! amtliche Pruefprogramm dazu verlangt; die Versorgungsbezuege kannte sie gar nicht. Wer sie hatte, konnte nicht
//! abgeben (Lohnsteuer ohne Lohn) oder bekam eine zu niedrige Steuer.
//!
//! **Warum es zaehlt.** Die Felder allein freizuschalten waere schlimmer gewesen: die Erklaerung waere abgabefaehig
//! gewesen, und 60.000 Euro Lohn haetten die Steuer um 0 Euro erhoeht (Entscheidung
//! `rentner-ring-liest-versorgungsbezuege-vor-der-scheibe`). Darum steht der Ring zuerst
//! (`bescheid/tests/rentner_lohn_versorgung_hermetisch.rs`) und diese Datei misst die Scheibe auf dem echten Weg
//! `POST /event`, `GET /ergebnis`.
//!
//! **Wo es sitzt.** `SCHEIBEN_RENTNER_GESAMT_FELDER` (`bescheid/src/deklaration/scheiben_tabellen.rs`); der Kegel
//! (28 Pflichtfelder) bleibt, Lohn und Versorgung sind keine neuen Pflichtfragen.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Zahlen sind Vergleiche, keine Konstanten. Der Anker ist die Scheibe `an_gesamt`:
//! wer als Rentner nur einen Lohn hat, zahlt dieselbe Steuer wie der Arbeitnehmer mit demselben Lohn (zwei Ringe, ein
//! Ergebnis). Die Euro-Betraege von Hand gerechnet stehen in `bescheid/tests/rentner_lohn_versorgung_hermetisch.rs`.
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
    sende(d, "POST", "/fall/rls/event", Some(&rumpf)).await
}

/// Ein leerer Fall der Scheibe.
async fn neuer_fall(scheibe: &str) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "rls", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    d
}

/// Der Fall `rentner_gesamt` mit allen `paare`, jedes ueber `POST /event` (muss 201 sein), dann `GET /ergebnis`.
async fn ergebnis(paare: &[(&'static str, Value)]) -> Value {
    ergebnis_der("rentner_gesamt", paare).await
}

async fn ergebnis_der(scheibe: &str, paare: &[(&'static str, Value)]) -> Value {
    let d = neuer_fall(scheibe).await;
    for (feld, wert) in paare {
        let (status, antwort) = event_post(&d, feld, wert).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, antwort) = sende(&d, "GET", "/fall/rls/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// Pflicht-Kegel `rentner_gesamt` (28 Felder) samt `kein_p23_verkauf = true`: Rente `rente_cent` ab 2025,
/// `kein_sonstige = false` (die eigene Rente ist die ehrliche Antwort), einzeln. Wie `rentner()` in
/// `flag_und_schalter_hermetisch.rs`, nur mit waehlbarer Rente.
fn rentner(rente_cent: i64) -> Paare {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(rente_cent)),
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
    ]
}

fn mit(mut basis: Paare, mehr: Paare) -> Paare {
    basis.extend(mehr);
    basis
}

fn lohn(euro: i64) -> Paare {
    vec![("bruttoarbeitslohn", json!(euro * 100))]
}

/// Versorgungsbezug von `euro`, Bemessungsgrundlage gleich dem Bezug, Beginn 2025, beamtenrechtlich.
fn versorgung(euro: i64) -> Paare {
    vec![
        ("versorgung_jahresrente", json!(euro * 100)),
        ("versorgung_bemessungsgrundlage", json!(euro * 100)),
        ("versorgung_beginn_jahr", json!(2025)),
        ("versorgung_art", json!("beamtenrechtlich")),
    ]
}

fn zahl(a: &Value) -> i64 {
    assert_eq!(a["grund"], json!("bestaetigt"), "keine Zahl: {a}");
    a["zahl_cent"]
        .as_i64()
        .unwrap_or_else(|| panic!("keine Zahl: {a}"))
}

/// Die sieben Felder sind auf der Scheibe beantwortbar: `POST /event` antwortet 201. Heute (vorher): 400
/// „nicht in dieser Scheibe", und die Lohnsteuer allein ist nicht abgabefaehig.
#[tokio::test]
async fn lohn_steuerklasse_und_die_fuenf_versorgungsfelder_sind_beantwortbar() {
    let felder: Paare = vec![
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("steuerklasse", json!("1")),
        ("versorgung_jahresrente", json!(3_000_000)),
        ("versorgung_bemessungsgrundlage", json!(3_000_000)),
        ("versorgung_beginn_jahr", json!(2025)),
        ("versorgung_art", json!("altersgrenze_sonstige")),
        ("versorgung_alter_bei_beginn", json!(63)),
    ];
    let d = neuer_fall("rentner_gesamt").await;
    for (feld, wert) in &felder {
        let (status, antwort) = event_post(&d, feld, wert).await;
        assert_eq!(
            status, 201,
            "POST /event {feld} auf rentner_gesamt: {antwort}"
        );
    }
}

/// 6.000, 60.000 und 600.000 Euro Lohn auf 20.000 Euro Rente: drei verschiedene, steigende Zahlen (heute dreimal
/// dieselbe).
#[tokio::test]
async fn der_lohn_aendert_die_steuer_ueber_http() {
    let ohne = zahl(&ergebnis(&rentner(2_000_000)).await);
    let mut steuer = vec![ohne];
    for euro in [6_000, 60_000, 600_000] {
        steuer.push(zahl(&ergebnis(&mit(rentner(2_000_000), lohn(euro))).await));
    }
    assert!(
        steuer.windows(2).all(|w| w[0] < w[1]),
        "die Steuer steigt mit dem Lohn (0, 6.000, 60.000, 600.000 Euro): {steuer:?}"
    );
}

/// 0, 30.000 und 60.000 Euro Versorgung: 0 Euro aendern nichts, danach steigt die Steuer (heute dreimal dieselbe).
#[tokio::test]
async fn die_versorgung_aendert_die_steuer_ueber_http() {
    let ohne = zahl(&ergebnis(&rentner(2_000_000)).await);
    let null = zahl(&ergebnis(&mit(rentner(2_000_000), versorgung(0))).await);
    let dreissig = zahl(&ergebnis(&mit(rentner(2_000_000), versorgung(30_000))).await);
    let sechzig = zahl(&ergebnis(&mit(rentner(2_000_000), versorgung(60_000))).await);
    assert_eq!(null, ohne, "0 Euro Versorgung aendern nichts");
    assert!(
        ohne < dreissig && dreissig < sechzig,
        "die Steuer steigt mit der Versorgung: {ohne} < {dreissig} < {sechzig}"
    );
}

/// Pflicht-Kegel `an_gesamt` (33 Felder) fuer einen Arbeitnehmer mit `euro` Lohn, einzeln, ohne Pendelstrecke, ohne
/// Kinder, alle vier Kreuze "nein". Wie `an()` in `flag_und_schalter_hermetisch.rs`, nur ohne die 220 Arbeitstage zu
/// 30 km (die Entfernungspauschale ist dort ein Werbungskosten-Abzug, den die Rentner-Scheibe nicht kennt).
fn arbeitnehmer(euro: i64) -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(euro * 100)),
        ("veranlagung", json!("einzel")),
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

/// Anker: eine Rentnerin ohne Rente mit 40.000 Euro Lohn zahlt dieselbe Steuer wie der Arbeitnehmer mit 40.000 Euro
/// Lohn auf der Scheibe `an_gesamt`. Die zwei Scheiben rechnen den Lohn in zwei Ringen; die Zahl ist nicht aus dem
/// Rentner-Ring abgelesen.
#[tokio::test]
async fn der_lohn_einer_rentnerin_kostet_dieselbe_steuer_wie_beim_arbeitnehmer() {
    let rentnerin = ergebnis(&mit(rentner(0), lohn(40_000))).await;
    let arbeitnehmer = ergebnis_der("an_gesamt", &arbeitnehmer(40_000)).await;
    assert_eq!(
        zahl(&rentnerin),
        zahl(&arbeitnehmer),
        "Rentnerin: {rentnerin}"
    );
}
