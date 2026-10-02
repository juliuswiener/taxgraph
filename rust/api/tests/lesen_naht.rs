//! `GET /fall/{id}/feld/{fid}/warum` ohne Python: die Gestalt aus `api.warum` (`api.py:548`), an
//! einem Fall der Scheibe `ep`, mit einem Event aus der Store-Bibliothek.
//!
//! Dass die Antworten Byte fuer Byte denen von Python gleichen, prueft der Differenz-Harness
//! (`rust/parity/tests/api_http_paritaet.rs`, `generatoren`); dieser Test haelt die Form fest und
//! faellt auch dort, wo kein Python laeuft.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashMap;
use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEvent, Store};
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
    token: &str,
    body: Option<&str>,
) -> (u16, Value) {
    let mut b = Request::builder()
        .method(methode)
        .uri(pfad)
        .header("authorization", format!("Bearer {token}"));
    if let Some(t) = body {
        b = b
            .header("content-type", "application/json")
            .header("content-length", t.len().to_string());
    }
    let req = b
        .body(body.map_or_else(Body::empty, |t| Body::from(t.to_owned())))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (teile.status.as_u16(), json)
}

/// Legt Fall `sonde` (Scheibe `ep`, Nutzer alice) an und liefert das Token.
async fn fall_anlegen(d: &Dienst) -> String {
    let token = d.zustand.auth.stelle_aus("alice").unwrap();
    let rumpf = r#"{"fall_id": "sonde", "scheibe": "ep", "veranlagungszeitraum": 2025}"#;
    let (status, antwort) = sende(d, "POST", "/fall", &token, Some(rumpf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    token
}

/// Haengt ein bestaetigtes Event an die Akte an (so wie `POST /event` es taete) und gibt seine Kennung.
fn event_anhaengen(d: &Dienst, feld: &str, wert: i64) -> String {
    let pfad = d.zustand.konfig.faelle.join("sonde.json");
    let mut store = Store::aus_datei(store::lade(&pfad).unwrap());
    let neu = NeuesEvent {
        feld_id: feld.to_owned(),
        wert: json!(wert).into(),
        feldzustand: Feldzustand::Bestaetigt {
            signal_2: Signal2::new("klick@naht").unwrap(),
        },
        herkunft: Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: Schreiber::Mensch("ui:naht".to_owned()),
        signal_1: None,
        ersetzt: None,
        ts: Some("2026-01-01T00:00:00+00:00".to_owned()),
    };
    let leer = HashMap::new();
    let id = store
        .append(&neu, None, BindungNachschlag::neu(&leer))
        .unwrap();
    store::speichere(&pfad, store.datei()).unwrap();
    id.to_string()
}

/// Ohne Event 404 mit Pythons Text (`!r` des Felds), kein 200 mit leerer Hülle.
#[tokio::test]
async fn warum_ohne_event_ist_404() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let (status, antwort) = sende(
        &d,
        "GET",
        "/fall/sonde/feld/ep_arbeitstage/warum",
        &token,
        None,
    )
    .await;
    assert_eq!(status, 404, "{antwort}");
    assert_eq!(
        antwort,
        json!({"fehler": "Feld 'ep_arbeitstage' hat (noch) kein Event"})
    );
}

/// Mit Event: das Justification-Objekt aus Event und Bindung, alle zehn Schluessel (`traverser.py:746`).
#[tokio::test]
async fn warum_mit_event_liefert_die_justification() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let event_id = event_anhaengen(&d, "ep_arbeitstage", 220);
    let (status, antwort) = sende(
        &d,
        "GET",
        "/fall/sonde/feld/ep_arbeitstage/warum",
        &token,
        None,
    )
    .await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(antwort["fall_id"], "sonde");
    let j = antwort["justification"].as_object().unwrap();
    let mut schluessel: Vec<&str> = j.keys().map(String::as_str).collect();
    schluessel.sort_unstable();
    assert_eq!(
        schluessel,
        [
            "anker_ref",
            "event_id",
            "feld_id",
            "geltungsbedingung",
            "herkunft",
            "regel_id",
            "signal",
            "signatur_slot",
            "wert",
            "zustand"
        ]
    );
    assert_eq!(j["feld_id"], "ep_arbeitstage");
    assert_eq!(j["wert"], 220);
    assert_eq!(j["zustand"], "bestaetigt");
    assert_eq!(j["event_id"], event_id.as_str());
    assert_eq!(j["herkunft"]["herkunft"], "laie");
    assert_eq!(j["signal"]["signal_2"], "klick@naht");
    assert!(
        j["regel_id"].is_string(),
        "die Bindung nennt die Regel: {j:?}"
    );
    assert!(j["anker_ref"]["zitatanker"].is_string(), "{j:?}");
    // Das Event eines Felds beantwortet nicht die Frage nach einem anderen.
    let pfad = "/fall/sonde/feld/ep_entfernung_km/warum";
    let (status, antwort) = sende(&d, "GET", pfad, &token, None).await;
    assert_eq!(status, 404, "{antwort}");
}
