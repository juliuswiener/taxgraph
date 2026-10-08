//! § 34c Abs. 2 `EStG`, Abzug statt Anrechnung (Abweichung Nr. 41): die Folgefrage `dba_fiktive_steuer_vorhanden` ueber die
//! HTTP-Naht. Ohne `PARITY=1`, ohne Python, ohne Netz, ohne `ERiC`.
//!
//! **Worum es geht.** Der Abzug gilt nur fuer die tatsaechlich gezahlte Steuer; eine Steuer, die ein Abkommen als gezahlt
//! ansieht (fiktiv), laesst sich nur anrechnen. Wer den Abzug waehlt, bekommt darum die Ja/Nein-Frage nach einer fiktiven
//! Steuer. Ein "ja" sperrt die Berechnung (`dba_abzug_offen`, Test `p34c_abzug_sperre.rs`).
//!
//! **Warum es zaehlt.** Die Frage darf weder jeden Nutzer mit Auslandsangaben noch jeden Anrechnungs-Fall belaestigen, und
//! sie darf nicht fehlen, wo der Abzug gewaehlt ist: sonst rechnet die Software einen Abzug, den das Gesetz verbietet.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_p34c_gesamt.yaml` (Feld, `feld_bedingung` mit `und`), die Regel
//! `p34c_2_abzug_statt_anrechnung` verbirgt die Frage ohne Auslandseinkuenfte (`bindung_regel_bedingungen.yaml`).
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

const FELD: &str = "dba_fiktive_steuer_vorhanden";
const WAHL: &str = "dba_abzug_statt_anrechnung";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";

struct Dienst {
    zustand: Zustand,
    token: String,
    /// Haelt das Verzeichnis des Falls am Leben, solange der Dienst laeuft.
    _tmp: tempfile::TempDir,
}

fn dienst() -> Dienst {
    let tmp = tempfile::tempdir().unwrap();
    let konfig = Konfig {
        wurzel: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        faelle: tmp.path().join("faelle"),
        audit_dir: tmp.path().join("faelle"),
    };
    std::fs::create_dir_all(&konfig.faelle).unwrap();
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

/// Legt den Fall `fs` der Scheibe `gesamt` (VZ 2025) an und schreibt jedes Paar ueber die echte Route `POST /event`.
async fn fall(d: &Dienst, paare: &[(&str, Value)]) {
    let kopf = json!({"fall_id": "fs", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (status, antwort) =
            sende(d, "POST", "/fall/fs/event", Some(&ereignis(feld, wert))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
}

/// Der Fall mit Auslandsangaben: die Eingangsfrage bejaht (`keine_auslandseinkuenfte` = `false`), Einzelveranlagung.
fn auslandsfall(mehr: &[(&'static str, Value)]) -> Paare {
    let mut p: Paare = vec![
        ("veranlagung", json!("einzel")),
        ("keine_auslandseinkuenfte", json!(false)),
    ];
    p.extend(mehr.iter().cloned());
    p
}

async fn ids(paare: &[(&str, Value)]) -> Vec<String> {
    let d = dienst();
    fall(&d, paare).await;
    let (status, fragen) = sende(&d, "GET", "/fall/fs/fragen", None).await;
    assert_eq!(status, 200, "{fragen}");
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|q| q["feld_id"].as_str().map(str::to_owned))
        .collect()
}

fn steht(ids: &[String], feld: &str) -> bool {
    ids.iter().any(|f| f == feld)
}

/// Die Frage steht nur beim Abzug mit gezahlter Steuer: beim Abzug und bei Schweigen auf die Abzugsfrage (fail-closed,
/// nur ein bestaetigtes "nein" streicht sie), nie ohne bestaetigte Steuer ueber 0.
#[tokio::test]
async fn die_frage_steht_nur_beim_abzug_mit_gezahlter_steuer() {
    let abzug = ids(&auslandsfall(&[
        (STEUER, json!(200_000)),
        (WAHL, json!(true)),
    ]))
    .await;
    assert!(steht(&abzug, FELD), "Abzug gewaehlt: {abzug:?}");

    let stumm = ids(&auslandsfall(&[(STEUER, json!(200_000))])).await;
    assert!(
        steht(&stumm, FELD),
        "Abzugsfrage unbeantwortet: die Folgefrage darf nicht still wegfallen: {stumm:?}"
    );

    let anrechnung = ids(&auslandsfall(&[
        (STEUER, json!(200_000)),
        (WAHL, json!(false)),
    ]))
    .await;
    assert!(
        !anrechnung.is_empty(),
        "KONTROLLE: der Katalog ist nicht leer, die Frage fehlt nicht nur, weil nichts mehr gefragt wird"
    );
    assert!(
        !steht(&anrechnung, FELD),
        "Abzug abgewaehlt, und die Frage steht doch: {anrechnung:?}"
    );

    let ohne_steuer = ids(&auslandsfall(&[(WAHL, json!(true))])).await;
    assert!(
        !steht(&ohne_steuer, FELD),
        "keine bestaetigte Steuer, und die Frage steht doch: {ohne_steuer:?}"
    );
    let null_steuer = ids(&auslandsfall(&[(STEUER, json!(0)), (WAHL, json!(true))])).await;
    assert!(
        !steht(&null_steuer, FELD),
        "Steuer 0, und die Frage steht doch: {null_steuer:?}"
    );
}

/// Ohne Auslandseinkuenfte (Eingangsfrage bestaetigt `true` = keine) fragt die Regel nichts: auch die Folgefrage nicht.
#[tokio::test]
async fn ohne_auslandseinkuenfte_steht_die_frage_nicht() {
    let p: Paare = vec![
        ("veranlagung", json!("einzel")),
        ("keine_auslandseinkuenfte", json!(true)),
        (STEUER, json!(200_000)),
        (WAHL, json!(true)),
    ];
    let keine = ids(&p).await;
    assert!(!keine.is_empty(), "KONTROLLE: der Katalog ist nicht leer");
    assert!(!steht(&keine, FELD), "{keine:?}");
}

/// Fragetext und Hilfetext sind fuer Laien geschrieben: der Fragetext nennt keinen Paragrafen, der Hilfetext sagt, was ein
/// "ja" bewirkt (Sperre, Anrechnung waehlen).
#[tokio::test]
async fn die_texte_erklaeren_fiktiv_und_nennen_den_ausweg() {
    let d = dienst();
    fall(
        &d,
        &auslandsfall(&[(STEUER, json!(200_000)), (WAHL, json!(true))]),
    )
    .await;
    let (status, fragen) = sende(&d, "GET", "/fall/fs/fragen", None).await;
    assert_eq!(status, 200, "{fragen}");
    let frage = fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["feld_id"] == FELD)
        .unwrap_or_else(|| panic!("die Frage fehlt: {fragen}"));
    let text = frage["fragetext_laie"].as_str().unwrap();
    assert!(text.contains("fiktiv"), "{text}");
    assert!(!text.contains('§'), "{text}");
    let hilfe = frage["hilfe_kurz"].as_str().unwrap();
    for teil in ["abziehen", "sperrt die Berechnung", "Anrechnung", "nein"] {
        assert!(hilfe.contains(teil), "Hilfetext ohne {teil:?}: {hilfe}");
    }
}
