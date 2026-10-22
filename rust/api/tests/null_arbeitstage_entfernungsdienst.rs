//! Fragen-Reduktion Teil 2, Phase 3 (Entscheidungen d-55 und d-56): Was tut der Entfernungsdienst (`POST /fall/{id}/entfernung`), wenn
//! die Entfernung schon aus bestaetigten 0 Arbeitstagen abgeleitet ist? Gegen einen lokalen Attrappen-Dienst, ohne Netz und ohne Schluessel
//! (Muster: `entfernung_ors_attrappe_hermetisch.rs`). Eigenes Binary, weil die Umgebung fuer den ganzen Prozess gilt.
//!
//! Der Dienst schreibt ein VORLAEUFIGES Event `berechnet:maps` mit `ersetzt` auf das aktive Ereignis des Feldes. Auf dem abgeleiteten (bestaetigten)
//! Wert heisst das: der Neutralwert weicht dem Vorschlag, die Frage nach der Entfernung kommt zurueck, der Kegel verlangt die Bestaetigung.
//! Kein 500, nie.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

/// Startet den Attrappen-Dienst: JEDE Anfrage bekommt Geocoding und eine Strecke von 30 400 m. Gibt die Basis-URL zurueck.
fn attrappen_dienst() -> String {
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
            let json = json!({
                "features": [{"geometry": {"coordinates": [11.5, 48.1]}}],
                "routes": [{"summary": {"distance": 30_400.0}}],
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
    let auth = Auth::neu("testgeheimnis".into(), tmp.path().join("users.json"), Some(konfig.audit_pfad()));
    let zustand = Zustand::neu(konfig, auth);
    let token = zustand.auth.stelle_aus("alice").unwrap();
    Dienst { zustand, token, _tmp: tmp }
}

async fn sende(d: &Dienst, methode: &str, pfad: &str, rumpf: Option<&Value>) -> (u16, Value) {
    let text = rumpf.map(ToString::to_string);
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

async fn stand_km(d: &Dienst) -> (Value, Value) {
    let (s, stand) = sende(d, "GET", "/fall/nz/stand", None).await;
    assert_eq!(s, 200, "GET /stand: {stand}");
    (stand["felder"]["ep_entfernung_km"]["wert"].clone(), stand["felder"]["ep_entfernung_km"]["zustand"].clone())
}

/// Der Dienst auf einer abgeleiteten Entfernung: 200, ein Vorschlag (30 km) ersetzt den Neutralwert, die Frage nach der Entfernung
/// steht wieder in der Liste, der Kegel sperrt, `GET /stand` und `GET /fragen` antworten 200. Die Antwort des Nutzers (mit `ersetzt` auf
/// den Vorschlag) bestaetigt ihn; die bestaetigten 0 Tage bleiben, auch dann ist die Steuer dieselbe wie ohne Dienst.
#[tokio::test]
async fn der_entfernungsdienst_auf_einer_abgeleiteten_entfernung_gibt_einen_vorschlag_und_nie_500() {
    let basis = attrappen_dienst();
    std::env::set_var("ORS_API_KEY", "SYNTHETISCH-ORS-SCHLUESSEL");
    std::env::set_var("ORS_API_BASE", &basis);
    let d = dienst();
    let (s, a) = sende(&d, "POST", "/fall", Some(&json!({"fall_id": "nz", "scheibe": "an_gesamt", "veranlagungszeitraum": 2025}))).await;
    assert_eq!(s, 201, "{a}");
    let (s, a) = sende(&d, "POST", "/fall/nz/event", Some(&ereignis("ep_arbeitstage", &json!(0)))).await;
    assert_eq!(s, 201, "{a}");
    assert_eq!(stand_km(&d).await, (json!(0), json!("bestaetigt")), "Kontrolle, die Entfernung ist abgeleitet");

    let rumpf = json!({"von": "Musterstr. 1, 80331 München", "nach": "Beispielweg 2, 10115 Berlin"});
    let (s, a) = sende(&d, "POST", "/fall/nz/entfernung", Some(&rumpf)).await;
    assert!(s < 500, "Entfernungsdienst auf abgeleiteter Entfernung: {s} {a}");
    println!("MESSUNG ii Entfernungsdienst auf abgeleiteter Entfernung: {s} {a}");
    assert_eq!(s, 200, "{a}");
    assert_eq!(stand_km(&d).await, (json!(30), json!("vorlaeufig")), "der Vorschlag ersetzt den Neutralwert");
    for pfad in ["stand", "fragen", "ergebnis", "deklaration"] {
        let (s, a) = sende(&d, "GET", &format!("/fall/nz/{pfad}"), None).await;
        assert_eq!(s, 200, "GET /{pfad} nach dem Entfernungsdienst: {a}");
    }
    let (_, fragen) = sende(&d, "GET", "/fall/nz/fragen", None).await;
    let ids: Vec<&str> = fragen["fragen"].as_array().unwrap().iter().map(|q| q["feld_id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"ep_entfernung_km"), "die Frage nach der Entfernung kommt zurueck: {ids:?}");

    // Der Nutzer bestaetigt den Vorschlag wie die Oberflaeche: mit `ersetzt` auf den aktiven Vorschlag.
    let (_, stand) = sende(&d, "GET", "/fall/nz/stand", None).await;
    let alt = stand["felder"]["ep_entfernung_km"]["event_id"].clone();
    let mut e = ereignis("ep_entfernung_km", &json!(30));
    e["ersetzt"] = alt;
    let (s, a) = sende(&d, "POST", "/fall/nz/event", Some(&e)).await;
    assert!(s == 200 || s == 201, "{s} {a}");
    assert_eq!(stand_km(&d).await, (json!(30), json!("bestaetigt")));
}
