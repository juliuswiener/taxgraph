//! `GET /fall/{id}/stand`, `GET /fall/{id}/fragen` und `GET /fall/{id}/feld/{fid}/frage` gegen feste
//! Antworten des Python-Servers, ohne `PARITY=1`, ohne Python, ohne Netz.
//!
//! Auftrag 8 (Mutationsmessung der duennen `api`-Dateien): fuer `stand.rs` und `fragen.rs` hielt nur
//! der Vergleich mit Python die Gestalt der Antwort (Engine-Name, Spanne, Sperrgrund, Teil-Ringe,
//! Metadaten jeder Frage). Hier stehen die Antworten fest in `rust/fixtures/api_stand_fragen_orakel.json`;
//! sie sind eingefroren; der Erzeuger ist geloescht (Verlauf:
//! `git show 2dd056a6:tools/parity/extract_stand_fragen_orakel.py`: `api.stand`, `api.fragen`,
//! `api.frage_einzeln` im selben Prozess, mit denselben Ereignissen).
//!
//! Die Ereignisse spielt der Test ueber die echte Route `POST /event` ein. Nur `event_id` jedes Felds
//! in `/stand` hat eine Uhrzeit im Hash: der Test prueft nur die Form (64 Hex-Zeichen), das Fixture
//! fuehrt `<event_id>`.
//!
//! Der Mitschnitt (`TAXGRAPH_FLOW=1`) liest die Umgebung des Prozesses; darum steht alles in EINEM Test.
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

fn orakel() -> Value {
    let pfad =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/api_stand_fragen_orakel.json");
    serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap()
}

/// `event_id` hat eine Uhrzeit im Hash: Form pruefen, dann auf einen festen Wert setzen.
fn ohne_event_id(stand: &mut Value) {
    for (fid, feld) in stand["felder"].as_object_mut().unwrap() {
        let id = feld["event_id"]
            .as_str()
            .unwrap_or_else(|| panic!("{fid}: keine event_id"));
        assert!(
            id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()),
            "{fid}: {id}"
        );
        feld["event_id"] = json!("<event_id>");
    }
}

fn gleich(fall: &str, was: &str, ist: &Value, soll: &Value) {
    assert_eq!(ist, soll, "{fall}: {was}");
}

#[tokio::test]
async fn stand_und_fragen_wie_python() {
    std::env::remove_var("TAXGRAPH_FLOW");
    let o = orakel();
    let faelle = o["faelle"].as_array().unwrap();
    assert_eq!(faelle.len(), 20);
    for f in faelle {
        let name = f["name"].as_str().unwrap();
        let d = dienst();
        let kopf = json!({"fall_id": "sf", "scheibe": f["scheibe"], "veranlagungszeitraum": 2025});
        let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
        assert_eq!(status, 201, "{name}: POST /fall: {antwort}");
        for e in f["events"].as_array().unwrap() {
            let (status, antwort) = sende(&d, "POST", "/fall/sf/event", Some(e)).await;
            assert_eq!(
                status, 201,
                "{name}: POST /event {}: {antwort}",
                e["feld_id"]
            );
        }

        // --- /stand: Feld fuer Feld, dann der Rest.
        let (status, mut ist) = sende(&d, "GET", "/fall/sf/stand", None).await;
        assert_eq!(status, 200, "{name}: GET /stand: {ist}");
        ohne_event_id(&mut ist);
        let mut soll = f["stand"].clone();
        soll["fall_id"] = json!("sf");
        let (ist_felder, soll_felder) = (
            ist["felder"].as_object().unwrap(),
            soll["felder"].as_object().unwrap(),
        );
        assert_eq!(
            ist_felder.keys().collect::<Vec<_>>().len(),
            soll_felder.len(),
            "{name}: Zahl der Felder in /stand"
        );
        for (fid, s) in soll_felder {
            gleich(name, &format!("stand.felder.{fid}"), &ist_felder[fid], s);
        }
        for schluessel in [
            "fall_id",
            "snapshot_id",
            "engine",
            "intervall",
            "teil_ringe",
            "ring_gesperrt",
            "ring_gesperrt_klartext",
            "relevanz",
        ] {
            gleich(
                name,
                &format!("stand.{schluessel}"),
                &ist[schluessel],
                &soll[schluessel],
            );
        }
        assert_eq!(
            ist.as_object().unwrap().len(),
            soll.as_object().unwrap().len(),
            "{name}: Schluessel von /stand"
        );

        // --- /fragen
        let (status, ist) = sende(&d, "GET", "/fall/sf/fragen", None).await;
        assert_eq!(status, 200, "{name}: GET /fragen: {ist}");
        let ist_fragen = ist["fragen"].as_array().unwrap();
        if let Some(voll) = f.get("fragen") {
            gleich(
                name,
                "fragen.snapshot_id",
                &ist["snapshot_id"],
                &voll["snapshot_id"],
            );
            gleich(
                name,
                "fragen.ring_gesperrt",
                &ist["ring_gesperrt"],
                &voll["ring_gesperrt"],
            );
            gleich(
                name,
                "fragen.ring_gesperrt_klartext",
                &ist["ring_gesperrt_klartext"],
                &voll["ring_gesperrt_klartext"],
            );
            let soll_fragen = voll["fragen"].as_array().unwrap();
            assert_eq!(
                ist_fragen
                    .iter()
                    .map(|q| q["feld_id"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                soll_fragen
                    .iter()
                    .map(|q| q["feld_id"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                "{name}: Reihenfolge der Queue"
            );
            for (i, s) in soll_fragen.iter().enumerate() {
                gleich(
                    name,
                    &format!("fragen[{i}] {}", s["feld_id"]),
                    &ist_fragen[i],
                    s,
                );
            }
        } else {
            assert_eq!(
                ist_fragen
                    .iter()
                    .map(|q| q["feld_id"].clone())
                    .collect::<Vec<_>>(),
                f["fragen_ids"].as_array().unwrap().clone(),
                "{name}: Reihenfolge der Queue"
            );
            let soll = &f["fragen_ring_gesperrt"];
            gleich(
                name,
                "fragen.ring_gesperrt",
                &ist["ring_gesperrt"],
                &soll[0],
            );
            gleich(
                name,
                "fragen.ring_gesperrt_klartext",
                &ist["ring_gesperrt_klartext"],
                &soll[1],
            );
            gleich(name, "fragen.snapshot_id", &ist["snapshot_id"], &soll[2]);
        }
        assert_eq!(ist["fall_id"], json!("sf"), "{name}");

        // --- /feld/{fid}/frage
        if let Some(einzeln) = f.get("einzeln") {
            for (fid, soll) in einzeln.as_object().unwrap() {
                let (status, ist) =
                    sende(&d, "GET", &format!("/fall/sf/feld/{fid}/frage"), None).await;
                assert_eq!(json!(status), soll["status"], "{name}: frage {fid}: {ist}");
                let mut soll_body = soll["body"].clone();
                if soll["status"] == json!(200) {
                    soll_body["fall_id"] = json!("sf");
                }
                gleich(name, &format!("frage {fid}"), &ist, &soll_body);
            }
        }

        // --- Mitschnitt von /fragen: Anzahl und Kopf der Queue (einmal, am ersten Fall).
        if name == "gesamt_kegel" {
            std::env::set_var("TAXGRAPH_FLOW", "1");
            let (status, _) = sende(&d, "GET", "/fall/sf/fragen", None).await;
            std::env::remove_var("TAXGRAPH_FLOW");
            assert_eq!(status, 200);
            let fluss =
                std::fs::read_to_string(d.zustand.konfig.audit_dir.join("flow.jsonl")).unwrap();
            let zeilen: Vec<Value> = fluss
                .lines()
                .map(|z| serde_json::from_str(z).unwrap())
                .collect();
            assert_eq!(zeilen.len(), 1, "{fluss}");
            assert_eq!(
                (zeilen[0]["art"].as_str(), zeilen[0]["fall"].as_str()),
                (Some("fragen"), Some("sf"))
            );
            gleich(name, "flow.inhalt", &zeilen[0]["inhalt"], &f["kopf"]);
        }
    }
}
