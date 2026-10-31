//! Plan Nr. 116 (Julius d-67, 2026-10-24): § 35a Abs. 5 S. 4 `EStG` teilt die Hoechstbetraege nur fuer ZWEI ALLEINSTEHENDE im selben Haushalt. Ein
//! zusammenveranlagtes Ehepaar teilt nichts; die Frage `p35a_mitveranlagung` ("leben wir im selben Haushalt mit einem anderen Alleinstehenden")
//! steht ihm nicht mehr.
//!
//! **Was dieser Test haelt.** `GET /fall/{id}/fragen` nennt die Frage, solange `veranlagung` nicht `zusammen` ist (allein veranlagt oder noch
//! unbeantwortet), und laesst sie bei `zusammen` weg (`feld_bedingung {feld: veranlagung, wert_nicht: zusammen}`). Fragetext und Hilfetext
//! nennen den gemeinsamen Haushalt zweier Alleinstehender und nicht mehr die Zusammenveranlagung. Keine Sperre haengt an der Frage: die
//! Zahl bleibt mit und ohne die Frage stehen (die Zahl selbst halten `engine/tests/haushaltsnahe_hermetisch.rs` und
//! `bescheid/tests/steuerermaessigungen_abbildung_hermetisch.rs`).
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

const FRAGE: &str = "p35a_mitveranlagung";

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

/// Ein bestaetigtes Ereignis, wie die Oberflaeche es schreibt.
fn ereignis(feld_id: &str, wert: &Value) -> Value {
    json!({
        "feld_id": feld_id,
        "herkunft": {"haftung": "nutzer", "herkunft": "laie", "pruef_tiefe": "ungeprueft"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld_id}")},
        "wert": wert,
        "zustand": "bestaetigt",
        "ts": "2026-01-01T00:00:00+00:00",
    })
}

/// Legt den Fall `p35` der Scheibe `gesamt` (VZ 2025) an, schreibt jedes Paar ueber die echte Route und gibt die Fragenliste zurueck.
async fn fragen_nach(paare: &[(&str, Value)]) -> Vec<Value> {
    let d = dienst();
    let kopf = json!({"fall_id": "p35", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (status, antwort) = sende(&d, "POST", "/fall/p35/event", Some(&ereignis(feld, wert))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, fragen) = sende(&d, "GET", "/fall/p35/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    fragen["fragen"].as_array().unwrap().clone()
}

fn frage<'a>(liste: &'a [Value], feld: &str) -> Option<&'a Value> {
    liste.iter().find(|q| q["feld_id"] == json!(feld))
}

/// Die Frage steht bei Einzelveranlagung und solange `veranlagung` fehlt in der Liste, bei Zusammenveranlagung nicht.
/// Kontrolle in jedem Fall: die Ja/Nein-Frage `hh_hat_aufwendungen` ist beantwortet (`ja`), die Frage haengt nicht an ihr.
#[tokio::test]
async fn die_frage_steht_nur_fuer_nicht_zusammen_veranlagte() {
    let hh = ("hh_hat_aufwendungen", json!(true));
    for (name, paare, soll_stehen) in [
        ("allein veranlagt", vec![hh.clone(), ("veranlagung", json!("einzel"))], true),
        ("Veranlagung unbeantwortet", vec![hh.clone()], true),
        ("gemeinsam veranlagt", vec![hh.clone(), ("veranlagung", json!("zusammen"))], false),
    ] {
        let liste = fragen_nach(&paare).await;
        // Kontrolle: die Liste ist nicht leer und kennt die Eingangsfrage `veranlagung` oder hat sie schon beantwortet.
        assert!(!liste.is_empty(), "{name}: leere Fragenliste");
        assert_eq!(
            frage(&liste, FRAGE).is_some(),
            soll_stehen,
            "{name}: {FRAGE} {} in der Liste: {:?}",
            if soll_stehen { "fehlt" } else { "steht" },
            liste.iter().map(|q| q["feld_id"].as_str().unwrap()).take(60).collect::<Vec<_>>()
        );
    }
}

/// Fragetext und Hilfetext nennen den Haushalt zweier Alleinstehender (Wortlaut von § 35a Abs. 5 S. 4), nicht die Zusammenveranlagung.
#[tokio::test]
async fn fragetext_nennt_den_haushalt_zweier_alleinstehender() {
    let liste = fragen_nach(&[("hh_hat_aufwendungen", json!(true)), ("veranlagung", json!("einzel"))]).await;
    let q = frage(&liste, FRAGE).unwrap_or_else(|| panic!("{FRAGE} fehlt bei Einzelveranlagung"));
    let text = q["fragetext_laie"].as_str().unwrap().to_lowercase();
    let hilfe = q["hilfe_kurz"].as_str().unwrap();
    assert!(text.contains("alleinstehend") && text.contains("haushalt"), "Fragetext nennt den Haushalt Alleinstehender nicht: {text}");
    assert!(!hilfe.contains("Zusammenveranlagung"), "Hilfetext nennt noch die Zusammenveranlagung: {hilfe}");
    assert!(hilfe.contains("Alleinstehende") && hilfe.contains("Höchstbeträge"), "Hilfetext nennt den Wortlaut nicht: {hilfe}");
}
