//! HTTP-Verhalten des Dienstes ohne Netz: Requests gehen durch `app()` (Dispatcher + Router).
//! Der Vergleich mit Python steht in `rust/parity/tests/api_http_paritaet.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;

use api::konfig::Konfig;
use api::routen::EINTRAEGE;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
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
    let zustand = Zustand::neu(konfig, auth);
    Dienst { zustand, _tmp: tmp }
}

struct Antwort {
    status: u16,
    kopf: axum::http::HeaderMap,
    text: String,
}

impl Antwort {
    fn json(&self) -> Value {
        serde_json::from_str(&self.text).unwrap_or_else(|e| panic!("kein JSON: {e}: {}", self.text))
    }
}

async fn sende(
    d: &Dienst,
    methode: &str,
    pfad: &str,
    kopf: &[(&str, &str)],
    body: Option<&str>,
) -> Antwort {
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
    Antwort {
        status: teile.status.as_u16(),
        kopf: teile.headers,
        text: String::from_utf8(bytes.to_vec()).unwrap(),
    }
}

fn bearer(d: &Dienst, nutzer: &str) -> String {
    format!("Bearer {}", d.zustand.auth.stelle_aus(nutzer).unwrap())
}

#[tokio::test]
async fn health_traegt_python_wortlaut_und_header() {
    let d = dienst();
    let a = sende(&d, "GET", "/health", &[], None).await;
    assert_eq!(a.status, 200);
    assert_eq!(a.text, r#"{"flow": false, "status": "ok"}"#);
    assert_eq!(a.kopf["content-type"], "application/json; charset=utf-8");
    assert_eq!(a.kopf["x-content-type-options"], "nosniff");
    assert_eq!(a.kopf["referrer-policy"], "no-referrer");
    assert!(a.kopf["content-security-policy"]
        .to_str()
        .unwrap()
        .starts_with("default-src 'none'"));
}

#[tokio::test]
async fn unbekannter_pfad_und_falsche_methode_sind_404_json() {
    let d = dienst();
    let a = sende(&d, "GET", "/gibt-es-nicht?x=1", &[], None).await;
    assert_eq!(a.status, 404);
    assert_eq!(
        a.json(),
        json!({"fehler": "route_not_found", "methode": "GET", "pfad": "/gibt-es-nicht"})
    );
    // Pfad existiert (nur für POST) — Python meldet trotzdem 404, nicht 405.
    let a = sende(&d, "GET", "/fall", &[], None).await;
    assert_eq!(a.json()["fehler"], "route_not_found");
    // Fall-Kennung mit Punkt passt nicht auf `_ID`.
    let a = sende(
        &d,
        "GET",
        "/fall/a.b/stand",
        &[("authorization", &bearer(&d, "alice"))],
        None,
    )
    .await;
    assert_eq!(a.json()["fehler"], "route_not_found");
}

#[tokio::test]
async fn fuehrende_doppelte_schraegstriche_werden_wie_in_python_zu_einem() {
    let d = dienst();
    let a = sende(&d, "GET", "//health?x=1", &[], None).await;
    assert_eq!((a.status, a.json()["status"].clone()), (200, json!("ok")));
    assert_eq!(sende(&d, "GET", "///ready", &[], None).await.status, 503);
}

#[tokio::test]
async fn andere_methoden_sind_501_html() {
    let d = dienst();
    for m in ["PUT", "PATCH", "HEAD", "OPTIONS"] {
        let a = sende(&d, m, "/health", &[], None).await;
        assert_eq!(a.status, 501, "{m}");
        assert!(a.kopf["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html"));
    }
}

#[tokio::test]
async fn host_origin_und_rumpfgrenzen() {
    let d = dienst();
    let a = sende(&d, "GET", "/health", &[("host", "evil.example")], None).await;
    assert_eq!(
        (a.status, a.json()["fehler"].clone()),
        (421, json!("unerwarteter_host"))
    );
    let a = sende(
        &d,
        "POST",
        "/auth/login",
        &[("origin", "http://evil.example")],
        None,
    )
    .await;
    assert_eq!(
        (a.status, a.json()["fehler"].clone()),
        (403, json!("cross_origin_verboten"))
    );
    // Origin zählt nur bei POST/DELETE.
    assert_eq!(
        sende(
            &d,
            "GET",
            "/health",
            &[("origin", "http://evil.example")],
            None
        )
        .await
        .status,
        200
    );
    let a = sende(
        &d,
        "POST",
        "/auth/login",
        &[("content-length", "33554433")],
        None,
    )
    .await;
    assert_eq!(a.status, 413);
    let a = sende(
        &d,
        "POST",
        "/auth/login",
        &[("content-length", "2"), ("content-type", "text/plain")],
        Some("{}"),
    )
    .await;
    assert_eq!(a.status, 415);
    let json_kopf = [("content-type", "application/json")];
    let a = sende(
        &d,
        "POST",
        "/auth/login",
        &[json_kopf[0], ("content-length", "3")],
        Some("{x}"),
    )
    .await;
    assert_eq!(
        (a.status, a.json()["fehler"].clone()),
        (400, json!("ungültiges JSON im Body"))
    );
}

#[tokio::test]
async fn jede_route_hat_einen_handler() {
    let d = dienst();
    for e in EINTRAEGE {
        let pfad = e.axum_pfad.replace("{id}", "x").replace("{fid}", "f");
        let a = sende(&d, e.methode, &pfad, &[], None).await;
        assert_ne!(
            a.json().get("fehler"),
            Some(&json!("route_not_found")),
            "{} {pfad}",
            e.methode
        );
    }
    assert_eq!(EINTRAEGE.len(), 24);
}

#[tokio::test]
async fn owner_check_trennt_nutzer_und_die_ampel_ist_offen() {
    let d = dienst();
    let alice = bearer(&d, "alice");
    let bob = bearer(&d, "bob");
    let anlegen = r#"{"fall_id": "f1", "scheibe": "ep", "veranlagungszeitraum": 2025}"#;
    let a = sende(
        &d,
        "POST",
        "/fall",
        &[
            ("authorization", &alice),
            ("content-type", "application/json"),
            ("content-length", &anlegen.len().to_string()),
        ],
        Some(anlegen),
    )
    .await;
    assert_eq!(
        (a.status, a.json()),
        (
            201,
            json!({"fall_id": "f1", "scheibe": "ep", "veranlagungszeitraum": 2025})
        )
    );
    // Eigener Fall: Stub (`deklaration` ist noch nicht portiert; `stand` und `fragen` rechnen).
    let a = sende(
        &d,
        "GET",
        "/fall/f1/deklaration",
        &[("authorization", &alice)],
        None,
    )
    .await;
    assert_eq!(
        (a.status, a.json()["fehler"].clone()),
        (501, json!("nicht_portiert"))
    );
    // Fremder Fall, kein Token, fehlender Fall.
    assert_eq!(
        sende(
            &d,
            "GET",
            "/fall/f1/stand",
            &[("authorization", &bob)],
            None
        )
        .await
        .status,
        403
    );
    assert_eq!(
        sende(&d, "GET", "/fall/f1/stand", &[], None).await.status,
        401
    );
    assert_eq!(
        sende(
            &d,
            "GET",
            "/fall/nix/stand",
            &[("authorization", &alice)],
            None
        )
        .await
        .status,
        404
    );
    assert_eq!(
        sende(&d, "DELETE", "/fall/f1", &[("authorization", &bob)], None)
            .await
            .status,
        403
    );
    // PARITÄT: P4 — die Ampel antwortet jedem, auch für fremde und fehlende Fälle.
    for kopf in [vec![], vec![("authorization", bob.as_str())]] {
        let a = sende(&d, "POST", "/fall/f1/elster-ampel", &kopf, None).await;
        assert_eq!(
            (a.status, a.json()["fehler"].clone()),
            (503, json!("unavailable"))
        );
    }
    let a = sende(&d, "DELETE", "/fall/f1", &[("authorization", &alice)], None).await;
    assert_eq!(
        a.json(),
        json!({"geloescht": true, "fall_id": "f1", "scheibe": "ep", "veranlagungszeitraum": 2025})
    );
    assert_eq!(
        sende(&d, "DELETE", "/fall/f1", &[("authorization", &alice)], None)
            .await
            .status,
        404
    );
}

/// 9c/0b: Der Owner-Check gibt dem Handler `FallId` und `Username`, keinen rohen String.
#[tokio::test]
async fn eigener_fall_traegt_fall_id_und_username() {
    let d = dienst();
    let rumpf = r#"{"fall_id": "f1", "scheibe": "ep", "veranlagungszeitraum": 2025}"#;
    let laenge = rumpf.len().to_string();
    let alice = bearer(&d, "alice");
    let kopf = [
        ("authorization", alice.as_str()),
        ("content-type", "application/json"),
        ("content-length", laenge.as_str()),
    ];
    assert_eq!(
        sende(&d, "POST", "/fall", &kopf, Some(rumpf)).await.status,
        201
    );
    let id = domain::FallId::new("f1").unwrap();
    let fall =
        api::EigenerFall::pruefe(&d.zustand, &api::Nutzer(Some("alice".into())), &id).unwrap();
    let (fid, nutzer): (&domain::FallId, Option<&auth::Username>) = (fall.id(), fall.nutzer());
    assert_eq!(
        (fid.as_str(), nutzer.map(auth::Username::as_str)),
        ("f1", Some("alice"))
    );
}

/// PARITÄT-Grenze (9c/0b): Ein gültig signiertes Token, dessen `sub` `_USER_RE` verfehlt, zählt
/// wie kein Token — 401. Python nimmt den Namen als uid an. Ein solches Token entsteht nur aus
/// einer von Hand geänderten `users.json` (die Registrierung prüft das Muster, der Login nicht).
#[tokio::test]
async fn token_mit_ungueltigem_namen_ist_401() {
    let d = dienst();
    let rumpf = r#"{"fall_id": "f1", "scheibe": "ep", "veranlagungszeitraum": 2025}"#;
    let laenge = rumpf.len().to_string();
    for name in ["ab", "1abc", "a b", ""] {
        let t = bearer(&d, name);
        let kopf = [
            ("authorization", t.as_str()),
            ("content-type", "application/json"),
            ("content-length", laenge.as_str()),
        ];
        for (methode, ziel, k, body) in [
            ("POST", "/fall", &kopf[..], Some(rumpf)),
            ("DELETE", "/fall/nix", &kopf[..1], None),
            ("GET", "/fall/nix/stand", &kopf[..1], None),
        ] {
            let a = sende(&d, methode, ziel, k, body).await;
            assert_eq!(
                (a.status, a.json()["fehler"].clone()),
                (401, json!("Authentifizierung erforderlich")),
                "{name:?} {methode} {ziel}"
            );
        }
    }
    assert!(!d.zustand.konfig.faelle.join("f1.json").exists());
}

/// Eine Akte, die `store::lade` abweist, bleibt byte-gleich (Vault
/// `backlog/taxgraph/falldatei-mit-nan-liest-rust-als-text`, AK2): `DELETE` und `POST /event`
/// scheitern am Owner-Check mit 500, `POST /fall` mit 409. Kein Weg faellt auf eine leere Akte
/// zurueck und schreibt.
#[tokio::test]
async fn abgewiesene_akte_bleibt_byte_gleich() {
    let d = dienst();
    let alice = bearer(&d, "alice");
    let pfad = d.zustand.konfig.faelle.join("f1.json");
    let akte = concat!(
        r#"{"version":1,"veranlagungszeitraum":2025,"fall_id":"f1","user_id":"alice","#,
        r#""events":[NaN],"snapshots":[]}"#
    );
    std::fs::create_dir_all(&d.zustand.konfig.faelle).unwrap();
    std::fs::write(&pfad, akte).unwrap();
    let rumpf = r#"{"fall_id": "f1", "scheibe": "ep", "veranlagungszeitraum": 2025}"#;
    let laenge = rumpf.len().to_string();
    let kopf = [
        ("authorization", alice.as_str()),
        ("content-type", "application/json"),
        ("content-length", laenge.as_str()),
    ];
    let mut status = Vec::new();
    for (methode, ziel, k, body) in [
        ("DELETE", "/fall/f1", &kopf[..1], None),
        ("POST", "/fall/f1/event", &kopf[..], Some(rumpf)),
        ("POST", "/fall", &kopf[..], Some(rumpf)),
    ] {
        status.push(sende(&d, methode, ziel, k, body).await.status);
    }
    assert_eq!(status, [500, 500, 409]);
    assert_eq!(std::fs::read(&pfad).unwrap(), akte.as_bytes());
}

#[tokio::test]
async fn openapi_nennt_alle_fertigen_routen() {
    use utoipa::OpenApi;
    let doc = serde_json::to_value(api::openapi::ApiDoc::openapi()).unwrap();
    let pfade = doc["paths"].as_object().unwrap();
    for p in [
        "/health",
        "/ready",
        "/auth/register",
        "/auth/login",
        "/auth/logout",
        "/auth/session",
        "/fall",
        "/fall/{id}",
        "/fall/{id}/elster-ampel",
    ] {
        assert!(pfade.contains_key(p), "{p}");
    }
}
