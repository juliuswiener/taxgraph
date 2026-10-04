//! `POST /fall/{id}/entfernung` gegen einen lokalen Attrappen-Dienst fuer `OpenRouteService`, ohne Netz
//! und ohne Schluessel. `$ORS_API_BASE` zeigt auf `127.0.0.1`, `$ORS_API_KEY` ist ein Platzhalter.
//!
//! Auftrag 6 (Mutationsmessung der Crate `api`): der Weg von der Antwort des Dienstes bis zum Event
//! und die 422 bei einer Entfernung ab 10^10 km (`C099`) hatte nur der Vergleich mit Python bewacht.
//! Die Pruefungen vor dem Netz (leere Adresse, Scheibe ohne Feld) stehen in
//! `http_grenzen_hermetisch.rs`.
//!
//! Der Attrappen-Dienst antwortet auf JEDE Anfrage mit demselben Dokument, das Geocoding
//! (`features[0].geometry.coordinates`) und Routing (`routes[0].summary.distance`) zugleich traegt.
//! Die Umgebung gilt fuer den ganzen Prozess; darum steht alles in EINEM Test.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

/// Startet den Dienst; `distanz` bestimmt, was `routes[0].summary.distance` nennt. Gibt die Basis-URL
/// zurueck. Der Thread laeuft bis zum Ende des Prozesses.
fn attrappen_dienst(distanz: Arc<Mutex<Value>>) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let basis = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
    std::thread::spawn(move || {
        for s in l.incoming() {
            let mut s = s.unwrap();
            let mut puffer = Vec::new();
            let mut b = [0u8; 4096];
            loop {
                let n = s.read(&mut b).unwrap();
                if n == 0 {
                    break;
                }
                puffer.extend_from_slice(&b[..n]);
                let text = String::from_utf8_lossy(&puffer).into_owned();
                if let Some(i) = text.find("\r\n\r\n") {
                    let laenge = text[..i]
                        .lines()
                        .find_map(|z| z.strip_prefix("Content-Length: "))
                        .map_or(0, |v| v.trim().parse::<usize>().unwrap());
                    if puffer.len() >= i + 4 + laenge {
                        break;
                    }
                }
            }
            let d = distanz.lock().unwrap().clone();
            let json = json!({
                "features": [{"geometry": {"coordinates": [11.5, 48.1]}}],
                "routes": [{"summary": {"distance": d}}],
            })
            .to_string();
            let antwort = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
                json.len()
            );
            let _ = s.write_all(antwort.as_bytes());
        }
    });
    basis
}

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
    let token = format!("Bearer {}", zustand.auth.stelle_aus("alice").unwrap());
    Dienst {
        zustand,
        token,
        _tmp: tmp,
    }
}

async fn post(d: &Dienst, pfad: &str, rumpf: &Value) -> (u16, Value) {
    let text = rumpf.to_string();
    let req = Request::builder()
        .method("POST")
        .uri(pfad)
        .header("authorization", &d.token)
        .header("content-type", "application/json")
        .header("content-length", text.len().to_string())
        .body(Body::from(text))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("kein JSON: {e}: {}", String::from_utf8_lossy(&bytes)));
    (teile.status.as_u16(), json)
}

fn events(d: &Dienst, fall: &str) -> Vec<Value> {
    let p = d.zustand.konfig.faelle.join(format!("{fall}.json"));
    let akte: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    akte["events"].as_array().unwrap().clone()
}

#[tokio::test]
async fn entfernung_vom_dienst_bis_zum_event_und_die_grenze_bei_zehn_milliarden_km() {
    let distanz = Arc::new(Mutex::new(json!(30_400.0)));
    let basis = attrappen_dienst(distanz.clone());
    std::env::set_var("ORS_API_KEY", "SYNTHETISCH-ORS-SCHLUESSEL");
    std::env::set_var("ORS_API_BASE", &basis);
    let d = dienst();
    let (s, a) = post(
        &d,
        "/fall",
        &json!({"fall_id": "en1", "scheibe": "n_vor_gwg", "veranlagungszeitraum": 2025}),
    )
    .await;
    assert_eq!(s, 201, "{a}");
    let rumpf =
        json!({"von": "Musterstr. 1, 80331 München", "nach": "Beispielweg 2, 10115 Berlin"});

    // 30 400 m sind 30 km: 200, ein vorlaeufiges Event `berechnet:maps`.
    let (s, a) = post(&d, "/fall/en1/entfernung", &rumpf).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["km"], json!(30), "{a}");
    assert_eq!(a["herkunft"], json!("berechnet"), "{a}");
    let ev = events(&d, "en1");
    assert_eq!(ev.len(), 1, "{ev:?}");
    assert_eq!(a["event_id"], ev[0]["event_id"], "{a}");
    assert_eq!(
        (
            ev[0]["feld_id"].as_str(),
            ev[0]["wert"].clone(),
            ev[0]["zustand"].as_str(),
            ev[0]["schreiber"].as_str()
        ),
        (
            Some("ep_entfernung_km"),
            json!(30),
            Some("vorlaeufig"),
            Some("berechnet:maps")
        )
    );
    assert_eq!(
        ev[0]["signal"]["signal_1"],
        json!({"typ": "maps", "dienst": "openrouteservice"})
    );

    // 31 500 m: `round` rundet gerade (32). Das neue Event ersetzt das erste.
    *distanz.lock().unwrap() = json!(31_500.0);
    let (s, a) = post(&d, "/fall/en1/entfernung", &rumpf).await;
    assert_eq!((s, a["km"].clone()), (200, json!(32)), "{a}");
    let ev = events(&d, "en1");
    assert_eq!(ev.len(), 2, "{ev:?}");
    assert_eq!(ev[1]["ersetzt"], ev[0]["event_id"], "{ev:?}");

    // Genau 10^10 km (1e13 m) passt nicht mehr in die Auflage F2: 422 mit der Abweisung, kein Event.
    *distanz.lock().unwrap() = json!(1e13);
    let (s, a) = post(&d, "/fall/en1/entfernung", &rumpf).await;
    assert_eq!(s, 422, "{a}");
    let text = a.to_string();
    assert!(
        text.contains("ep_entfernung_km") && text.contains("10000000000"),
        "{text}"
    );
    assert_eq!(events(&d, "en1").len(), 2);
    // Knapp darunter (9,99e12 m) geht durch.
    *distanz.lock().unwrap() = json!(9.99e12);
    let (s, a) = post(&d, "/fall/en1/entfernung", &rumpf).await;
    assert_eq!((s, a["km"].clone()), (200, json!(9_990_000_000_u64)), "{a}");

    // Ohne Schluessel ist der Dienst nicht verbunden: 503 mit dem Vertrag, kein Event.
    std::env::set_var("ORS_API_KEY", "");
    let vorher = events(&d, "en1").len();
    let (s, a) = post(&d, "/fall/en1/entfernung", &rumpf).await;
    assert_eq!((s, a["fehler"].clone()), (503, json!("unavailable")), "{a}");
    assert!(
        a["vertrag"]
            .as_str()
            .is_some_and(|v| v.starts_with("Der Karten-Dienst ist nicht verbunden")),
        "{a}"
    );
    assert_eq!(events(&d, "en1").len(), vorher);
}
