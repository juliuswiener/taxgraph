//! ROTES GATE fuer die Naht `GET /fall/{id}/stand`: der Endpunkt antwortet heute
//! `501 nicht_portiert` (`routen/lesen.rs:24`). Dieser Test fordert die echte Antwort.
//!
//! Er ist die Probe, nicht die Loesung: er haelt fest, WAS fehlt, und faellt, sobald der
//! Stub durch einen rechnenden Handler ersetzt wird. Bis dahin ist er `#[ignore]` mit der
//! Zahl der fehlenden Abhaengigkeiten — ein dauerhaft roter Test zerstoert das Signal
//! „rot heisst, etwas ist kaputtgegangen" (Repo-Konvention, s. `73d4245`).
//!
//! Rot sehen:  `cargo test -p api --test stand_naht -- --ignored`
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

struct Dienst {
    zustand: Zustand,
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
    Dienst {
        zustand: Zustand::neu(konfig, auth),
        _tmp: tmp,
    }
}

async fn sende(
    d: &Dienst,
    methode: &str,
    pfad: &str,
    kopf: &[(&str, &str)],
    body: Option<&str>,
) -> (u16, Value, String) {
    let mut b = Request::builder().method(methode).uri(pfad);
    for (k, v) in kopf {
        b = b.header(*k, *v);
    }
    let req = b
        .body(body.map_or_else(Body::empty, |t| Body::from(t.to_owned())))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let json = serde_json::from_str(&text).unwrap_or(Value::Null);
    (teile.status.as_u16(), json, text)
}

/// Der Fall der Sonde: Scheibe `ep` — sechs Felder, kein `guard`.
async fn fall_anlegen(d: &Dienst) -> String {
    let token = format!("Bearer {}", d.zustand.auth.stelle_aus("alice").unwrap());
    let rumpf = r#"{"fall_id": "sonde", "scheibe": "ep", "veranlagungszeitraum": 2025}"#;
    let (status, _, text) = sende(
        d,
        "POST",
        "/fall",
        &[
            ("authorization", &token),
            ("content-type", "application/json"),
            ("content-length", &rumpf.len().to_string()),
        ],
        Some(rumpf),
    )
    .await;
    assert_eq!(status, 201, "POST /fall: {text}");
    token
}

/// GRUENE KONTROLLZEILE: der Fall existiert, die Route erreicht ihn. Ohne sie fiele der rote
/// Test unten auch dann, wenn schon `POST /fall` kaputt waere — und „irgendwas geht nicht"
/// ist kein Befund.
#[tokio::test]
async fn kontrolle_der_fall_wird_erreicht() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let (status, json, text) = sende(
        &d,
        "GET",
        "/fall/sonde/stand",
        &[("authorization", &token)],
        None,
    )
    .await;
    // 501 heute, 200 nach der Naht — beides beweist, dass die Route den Fall erreicht.
    // 404/401/403 taeten es nicht.
    assert!(
        status == 501 || status == 200,
        "stand erreicht den Fall nicht: {status} {text}"
    );
    if status == 501 {
        assert_eq!(json["fehler"], "nicht_portiert");
    }
}

/// DIE NAHT: `stand` rechnet, statt 501 zu liefern.
///
/// Geprueft wird die Gestalt aus `api.stand` (`api.py:489`), nicht ein Zahlwert — ein Zahlwert
/// haengt am Korpus. `engine` ist einer von vier Werten; der leere Fall hat einen Ring, also
/// ist `catala` die erwartete ehrliche Antwort (kein Fake-Gruen).
#[tokio::test]
#[ignore = "Naht nicht gezogen: GET /stand ist 501. Fehlende Abhaengigkeiten im Bericht startbares-ziel.md: 95 Konstanten / 333 Zeilen SCHEIBEN aus api_constants.py, 4 Crate-Abhaengigkeiten (bindung, interview, intervall, bescheid) und die Cfg-Erweiterung. Rot sehen: --ignored"]
async fn stand_rechnet_statt_501() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let (status, json, text) = sende(
        &d,
        "GET",
        "/fall/sonde/stand",
        &[("authorization", &token)],
        None,
    )
    .await;
    assert_eq!(status, 200, "stand lieferte {status}: {text}");
    for schluessel in [
        "fall_id",
        "snapshot_id",
        "engine",
        "felder",
        "relevanz",
        "teil_ringe",
        "ring_gesperrt",
        "ring_gesperrt_klartext",
    ] {
        assert!(
            json.get(schluessel).is_some(),
            "Schluessel {schluessel} fehlt in {text}"
        );
    }
    assert_eq!(json["fall_id"], "sonde");
    assert!(
        ["unavailable", "gesperrt", "catala", "catala_teilweise"]
            .contains(&json["engine"].as_str().unwrap_or("")),
        "unerwartete engine: {}",
        json["engine"]
    );
}
