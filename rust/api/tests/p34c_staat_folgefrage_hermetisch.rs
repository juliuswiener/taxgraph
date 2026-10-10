//! Anlage AUS, Staat der Auslandseinkuenfte (Abweichung Nr. 55): die Folgefrage `dba_staat_sonstiger` ueber die HTTP-Naht. Ohne
//! `PARITY=1`, ohne Python, ohne Netz, ohne `ERiC`.
//!
//! **Worum es geht.** Wer bei "Aus welchem Staat?" "Anderer Staat" waehlt, nennt den Staat in einer Folgefrage mit der amtlichen
//! Laenderliste. Der Name steht dann in der Erklaerung (Test `dba_staat_kz.rs`). Wer einen benannten Staat waehlt, bekommt die
//! Folgefrage nicht.
//!
//! **Warum es zaehlt.** Die Frage darf nicht jeden mit Auslandsangaben belaestigen, und sie darf nicht fehlen, wo "Anderer Staat"
//! gewaehlt ist oder die Staatsfrage noch offen ist: sonst sperrt die Abgabe, und der Nutzer sieht keinen Weg heraus.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_p34c_gesamt.yaml` (Feld, `feld_bedingung` auf `dba_staat`), die Regel
//! `p34c_1_anrechnung_hoechstbetrag` verbirgt die Frage ohne Auslandseinkuenfte (`bindung_regel_bedingungen.yaml`).
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

const FELD: &str = "dba_staat_sonstiger";
const STAAT: &str = "dba_staat";

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

/// Die Folgefrage steht nur nach "Anderer Staat" und bei Schweigen auf die Staatsfrage (fail-closed: nur ein bestaetigter
/// benannter Staat streicht sie).
#[tokio::test]
async fn die_folgefrage_steht_nur_nach_anderer_staat() {
    let anderer = ids(&auslandsfall(&[(STAAT, json!("sonstiger_staat"))])).await;
    assert!(steht(&anderer, FELD), "Anderer Staat gewaehlt: {anderer:?}");

    let stumm = ids(&auslandsfall(&[])).await;
    assert!(
        steht(&stumm, FELD),
        "Staatsfrage unbeantwortet: die Folgefrage darf nicht still wegfallen: {stumm:?}"
    );

    let benannt = ids(&auslandsfall(&[(STAAT, json!("Frankreich"))])).await;
    assert!(
        !benannt.is_empty() && !steht(&benannt, STAAT),
        "KONTROLLE: der Katalog ist nicht leer und die Staatsfrage ist beantwortet: {benannt:?}"
    );
    assert!(!steht(&benannt, FELD), "benannter Staat, und die Folgefrage steht doch: {benannt:?}");
}

/// Ohne Auslandseinkuenfte (Eingangsfrage bestaetigt `true` = keine) fragt die Regel nichts: auch die Folgefrage nicht.
#[tokio::test]
async fn ohne_auslandseinkuenfte_steht_die_folgefrage_nicht() {
    let p: Paare = vec![
        ("veranlagung", json!("einzel")),
        ("keine_auslandseinkuenfte", json!(true)),
        (STAAT, json!("sonstiger_staat")),
    ];
    let keine = ids(&p).await;
    assert!(!keine.is_empty(), "KONTROLLE: der Katalog ist nicht leer");
    assert!(!steht(&keine, FELD), "{keine:?}");
}

/// Die Frage bietet die 182 Namen der amtlichen Liste an, mit Anzeigetext, und der Text nennt keinen Paragrafen.
#[tokio::test]
async fn die_folgefrage_bietet_die_amtlichen_namen_an() {
    let d = dienst();
    fall(&d, &auslandsfall(&[(STAAT, json!("sonstiger_staat"))])).await;
    let (status, fragen) = sende(&d, "GET", "/fall/fs/fragen", None).await;
    assert_eq!(status, 200, "{fragen}");
    let frage = fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["feld_id"] == FELD)
        .unwrap_or_else(|| panic!("die Frage fehlt: {fragen}"));
    let text = frage["fragetext_laie"].as_str().unwrap();
    assert!(text.contains("Staat"), "{text}");
    assert!(!text.contains('§'), "{text}");
    let labels = frage["enum_labels"].as_object().unwrap_or_else(|| panic!("keine Anzeigetexte: {frage}"));
    assert_eq!(labels.len(), 182, "Zahl der angebotenen Namen");
    for name in ["Portugal", "Korea, Republik", "Cote d'Ivoire", "Äthiopien"] {
        assert_eq!(labels.get(name).and_then(Value::as_str), Some(name), "Anzeigetext zu {name}: {labels:?}");
    }
}
