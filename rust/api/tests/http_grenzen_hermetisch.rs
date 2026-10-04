//! Grenzen und Pruefungen des HTTP-Randes, hermetisch (ohne `PARITY=1`, ohne Netz, ohne Python).
//!
//! Auftrag 6 (Mutationsmessung der Crate `api` ohne `PARITY=1`): jeder Test unten bewacht eine
//! Stelle, an der ein Mutant der Messung ueberlebte, weil nur der Vergleich mit Python sie sah.
//! Die Kennungen (`C039` ...) stehen in `berichte/api-mutation.md`.
//!
//! - `C039`: Rumpfgrenze bei genau 32 MiB (`laenge > MAX`, nicht `>=`).
//! - `C041`: ein Rumpf von einem Byte braucht ebenfalls `application/json` (`laenge > 0`).
//! - `X05`: `application/json; charset=utf-8` zaehlt als JSON (`starts_with`, nicht `==`).
//! - `C057`: statische Dateien nur unterhalb von `static/` (Pfad ausserhalb ist 404).
//! - `C214`: `GET /auth/session` ohne Token ist 401.
//! - `C216`: `POST /auth/register` mit einer Liste: vollstaendig 500, unvollstaendig 400.
//! - `C225`, `C234`, `X01`, `X03`: Pruefungen von `POST /fall`.
//! - `X15`, `X08`: eine Akte ohne Besitzer ist fuer jeden angemeldeten Nutzer 403, mit Protokollzeile.
//! - `X06`: die Protokollzeile eines Aufrufs ohne Token nennt den Nutzer `dev`.
//! - `C110`, `C112`: `entfernung` prueft Adressen und Scheibe vor jedem Netzzugriff.
//! - `C070`, `X07`: eine Kennung ausserhalb von `{1,64}` erreicht `fall_kennung` nie (Route).
//! - `C118`: jede Scheibe mit Gesamt-Ring hat einen Accessor; `engine_unavailable` ist unerreichbar.
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
    Dienst {
        zustand: Zustand::neu(konfig, auth),
        _tmp: tmp,
    }
}

struct Antwort {
    status: u16,
    kopf: axum::http::HeaderMap,
    bytes: Vec<u8>,
}

impl Antwort {
    fn text(&self) -> String {
        String::from_utf8(self.bytes.clone()).unwrap()
    }
    fn json(&self) -> Value {
        serde_json::from_slice(&self.bytes)
            .unwrap_or_else(|e| panic!("kein JSON: {e}: {}", self.text()))
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
    Antwort {
        status: teile.status.as_u16(),
        kopf: teile.headers,
        bytes: rumpf.collect().await.unwrap().to_bytes().to_vec(),
    }
}

/// POST mit `Content-Type: application/json` und passender `Content-Length`.
async fn post_json(d: &Dienst, pfad: &str, token: Option<&str>, rumpf: &str) -> Antwort {
    let laenge = rumpf.len().to_string();
    let mut kopf = vec![
        ("content-type", "application/json"),
        ("content-length", laenge.as_str()),
    ];
    if let Some(t) = token {
        kopf.push(("authorization", t));
    }
    sende(d, "POST", pfad, &kopf, Some(rumpf)).await
}

fn bearer(d: &Dienst, nutzer: &str) -> String {
    format!("Bearer {}", d.zustand.auth.stelle_aus(nutzer).unwrap())
}

/// Das Protokoll als JSON-Zeilen; fehlt die Datei, ist es leer.
fn audit_zeilen(d: &Dienst) -> Vec<Value> {
    std::fs::read_to_string(d.zustand.konfig.audit_pfad())
        .unwrap_or_default()
        .lines()
        .map(|z| serde_json::from_str(z).unwrap())
        .collect()
}

/// `C039`, `C041`, `X05`: die Grenzen des Rumpfs, an beiden Seiten.
#[tokio::test]
async fn rumpfgrenze_content_type_und_ein_byte() {
    let d = dienst();
    // 32 MiB genau passieren (der Rumpf ist kuerzer als angegeben; `oneshot` prueft das nicht).
    // `/auth/login` ohne Felder antwortet 400 (Pflichtfelder), also weder 413 noch 415.
    let a = sende(
        &d,
        "POST",
        "/auth/login",
        &[
            ("content-type", "application/json"),
            ("content-length", "33554432"),
        ],
        Some("{}"),
    )
    .await;
    assert_eq!(a.status, 400, "{}", a.text());
    // Ein Byte mehr ist 413, noch vor dem Content-Type.
    let a = sende(
        &d,
        "POST",
        "/auth/login",
        &[("content-length", "33554433")],
        Some("{}"),
    )
    .await;
    assert_eq!(a.status, 413);
    // Auch ein einzelnes Byte braucht application/json.
    let a = sende(
        &d,
        "POST",
        "/auth/login",
        &[("content-length", "1"), ("content-type", "text/plain")],
        Some("x"),
    )
    .await;
    assert_eq!(a.status, 415, "{}", a.text());
    // Ein Zeichensatz hinter dem Medientyp aendert nichts.
    let a = sende(
        &d,
        "POST",
        "/auth/login",
        &[
            ("content-length", "2"),
            ("content-type", "application/json; charset=utf-8"),
        ],
        Some("{}"),
    )
    .await;
    assert_eq!(a.status, 400, "{}", a.text());
}

/// `C057`: `GET /` und `GET /static/...` liefern nur Dateien unterhalb von `produkt/haut/static`.
#[tokio::test]
async fn statische_dateien_nur_unterhalb_von_static() {
    let d = dienst();
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let index = std::fs::read(wurzel.join("produkt/haut/static/index.html")).unwrap();
    let a = sende(&d, "GET", "/", &[], None).await;
    assert_eq!(a.status, 200);
    assert_eq!(a.kopf["content-type"], "text/html; charset=utf-8");
    assert_eq!(a.bytes, index);
    let a = sende(&d, "GET", "/static/app.js", &[], None).await;
    assert_eq!(a.status, 200);
    assert_eq!(
        a.kopf["content-type"],
        "application/javascript; charset=utf-8"
    );
    // Eine Datei, die es gibt, aber ausserhalb von `static/` liegt: relativ ueber `..` und absolut
    // (`os.path.join` ersetzt die Wurzel durch einen absoluten Rest).
    let makefile = wurzel.join("Makefile").canonicalize().unwrap();
    assert!(makefile.is_file());
    let absolut = format!("/static/{}", makefile.display());
    for pfad in ["/static/../../../Makefile", absolut.as_str()] {
        let a = sende(&d, "GET", pfad, &[], None).await;
        assert_eq!(
            (a.status, a.json()),
            (404, json!({"fehler": "not_found", "pfad": pfad})),
            "{pfad}"
        );
    }
    // Ein Verzeichnis ist keine Datei.
    let a = sende(&d, "GET", "/static/", &[], None).await;
    assert_eq!(a.status, 404, "{}", a.text());
}

/// `C214`: `GET /auth/session` ohne und mit Token.
#[tokio::test]
async fn session_ohne_token_ist_401_mit_token_200() {
    let d = dienst();
    let a = sende(&d, "GET", "/auth/session", &[], None).await;
    assert_eq!(a.status, 401, "{}", a.text());
    let alice = bearer(&d, "alice");
    let a = sende(
        &d,
        "GET",
        "/auth/session",
        &[("authorization", &alice)],
        None,
    )
    .await;
    assert_eq!(
        (a.status, a.json()),
        (200, json!({"username": "alice", "authenticated": true}))
    );
}

/// `C216`: Pythons `set(body)` nimmt eine Liste: fehlt ein Feld, ist es 400; sind beide da, scheitert
/// erst `body["username"]` mit `TypeError` (500).
#[tokio::test]
async fn register_mit_einer_liste_unterscheidet_vollstaendig_und_unvollstaendig() {
    let d = dienst();
    let a = post_json(&d, "/auth/register", None, r#"["username"]"#).await;
    assert_eq!(a.status, 400, "{}", a.text());
    let a = post_json(&d, "/auth/register", None, r#"["username", "password"]"#).await;
    assert_eq!(a.status, 500, "{}", a.text());
    assert!(
        a.text()
            .contains("list indices must be integers or slices, not str"),
        "{}",
        a.text()
    );
}

/// `C225`, `C234`, `X01`, `X03`: die Eingabepruefung von `POST /fall` (`api.fall_anlegen`).
#[tokio::test]
async fn fall_anlegen_prueft_jahr_kennung_und_nimmt_die_vorgaben() {
    let d = dienst();
    let alice = bearer(&d, "alice");
    // (Rumpf, Status, Teil der Meldung)
    let lang65 = "a".repeat(65);
    let abgewiesen = [
        (
            r#"{"fall_id": "x1", "veranlagungszeitraum": "abc"}"#.to_owned(),
            "keine Zahl",
        ),
        (
            r#"{"fall_id": "x1", "veranlagungszeitraum": 1999}"#.to_owned(),
            "hat keine Parameter",
        ),
        (
            r#"{"veranlagungszeitraum": 2025}"#.to_owned(),
            "fall_id fehlt oder ungültig",
        ),
        (
            r#"{"fall_id": "", "veranlagungszeitraum": 2025}"#.to_owned(),
            "fall_id fehlt oder ungültig",
        ),
        (
            r#"{"fall_id": "a.b", "veranlagungszeitraum": 2025}"#.to_owned(),
            "fall_id fehlt oder ungültig",
        ),
        (
            format!(r#"{{"fall_id": "{lang65}"}}"#),
            "fall_id fehlt oder ungültig",
        ),
        (
            r#"{"fall_id": "x1", "scheibe": "gibt_es_nicht"}"#.to_owned(),
            "unbekannte Scheibe",
        ),
    ];
    for (rumpf, teil) in &abgewiesen {
        let a = post_json(&d, "/fall", Some(&alice), rumpf).await;
        assert_eq!(a.status, 400, "{rumpf}: {}", a.text());
        assert!(a.text().contains(teil), "{rumpf}: {}", a.text());
    }
    assert!(!d.zustand.konfig.faelle.join("x1.json").exists());
    // Ohne Angaben gelten Scheibe `ep` und Jahr 2025.
    let a = post_json(&d, "/fall", Some(&alice), r#"{"fall_id": "v1"}"#).await;
    assert_eq!(
        (a.status, a.json()),
        (
            201,
            json!({"fall_id": "v1", "scheibe": "ep", "veranlagungszeitraum": 2025})
        )
    );
    // 64 Zeichen sind die laengste Kennung.
    let lang64 = "b".repeat(64);
    let rumpf = format!(r#"{{"fall_id": "{lang64}"}}"#);
    let a = post_json(&d, "/fall", Some(&alice), &rumpf).await;
    assert_eq!(a.status, 201, "{}", a.text());
    let a = sende(
        &d,
        "GET",
        &format!("/fall/{lang64}/stand"),
        &[("authorization", &alice)],
        None,
    )
    .await;
    assert_eq!(a.status, 200, "{}", a.text());
}

/// `C070`, `X07`: `fall_kennung` weist nur ab, was die Route schon abgewiesen hat. Eine Kennung mit
/// 65 Zeichen oder mit Punkt erreicht den Owner-Check nie; sie ist `route_not_found`. Darum ist die
/// Reihenfolge 401 vor 400 und der Status der 400 dort nicht zu sehen (aequivalente Mutanten).
#[tokio::test]
async fn kennung_ausserhalb_der_route_ist_route_not_found_auch_ohne_token() {
    let d = dienst();
    for kennung in ["a".repeat(65), "a.b".to_owned(), "a%20b".to_owned()] {
        let a = sende(&d, "GET", &format!("/fall/{kennung}/stand"), &[], None).await;
        assert_eq!(
            (a.status, a.json()["fehler"].clone()),
            (404, json!("route_not_found")),
            "{kennung}"
        );
    }
}

/// `X15`, `X08`: eine Akte ohne `user_id` gehoert niemandem. Jeder angemeldete Nutzer bekommt 403,
/// und das Protokoll nennt Nutzer, Akte und `owner=None` (Pythons `_fall_owner_check`).
#[tokio::test]
async fn akte_ohne_besitzer_ist_403_und_steht_im_protokoll() {
    let d = dienst();
    std::fs::create_dir_all(&d.zustand.konfig.faelle).unwrap();
    std::fs::write(
        d.zustand.konfig.faelle.join("o1.json"),
        r#"{"version":1,"veranlagungszeitraum":2025,"fall_id":"o1","events":[],"snapshots":[]}"#,
    )
    .unwrap();
    let alice = bearer(&d, "alice");
    let a = sende(
        &d,
        "GET",
        "/fall/o1/stand",
        &[("authorization", &alice)],
        None,
    )
    .await;
    assert_eq!(a.status, 403, "{}", a.text());
    let zeilen = audit_zeilen(&d);
    let z = zeilen
        .iter()
        .find(|z| z["action"] == "zugriff_verweigert")
        .unwrap_or_else(|| panic!("keine Zeile zugriff_verweigert: {zeilen:?}"));
    assert_eq!(
        (
            z["user_id"].as_str(),
            z["fall_id"].as_str(),
            z["detail"].as_str()
        ),
        (Some("alice"), Some("o1"), Some("user=alice, owner=None"))
    );
    // Dieselbe Akte mit Besitzer alice: erreichbar; bob bleibt draussen.
    std::fs::write(
        d.zustand.konfig.faelle.join("o2.json"),
        r#"{"version":1,"veranlagungszeitraum":2025,"fall_id":"o2","user_id":"alice","scheibe":"ep","events":[],"snapshots":[]}"#,
    )
    .unwrap();
    let a = sende(
        &d,
        "GET",
        "/fall/o2/stand",
        &[("authorization", &alice)],
        None,
    )
    .await;
    assert_eq!(a.status, 200, "{}", a.text());
    let bob = bearer(&d, "bob");
    let a = sende(
        &d,
        "GET",
        "/fall/o2/stand",
        &[("authorization", &bob)],
        None,
    )
    .await;
    assert_eq!(a.status, 403, "{}", a.text());
    let z = audit_zeilen(&d)
        .into_iter()
        .rfind(|z| z["action"] == "zugriff_verweigert")
        .unwrap();
    assert_eq!(z["detail"].as_str(), Some("user=bob, owner=alice"));
}

/// `X06`: ohne gueltiges Token nennt die Protokollzeile den Nutzer `dev` (`server.py`:
/// `uid = user or "dev"`), mit Status der Antwort.
#[tokio::test]
async fn protokollzeile_ohne_token_nennt_den_nutzer_dev() {
    let d = dienst();
    let a = post_json(&d, "/fall", None, r#"{"fall_id": "n1"}"#).await;
    assert_eq!(a.status, 401, "{}", a.text());
    let z = audit_zeilen(&d).pop().expect("Protokoll leer");
    assert_eq!(
        (
            z["user_id"].as_str(),
            z["action"].as_str(),
            z["detail"].as_str()
        ),
        (Some("dev"), Some("fall_create"), Some("status=401"))
    );
}

/// `C110`, `C112`: `entfernung` prueft beide Adressen und das Feld der Scheibe, bevor es das Netz
/// fragt. Die Fehler kommen also auch ohne Schluessel und ohne Dienst.
#[tokio::test]
async fn entfernung_prueft_adressen_und_scheibe_vor_dem_netz() {
    let d = dienst();
    let alice = bearer(&d, "alice");
    for (id, scheibe) in [("e1", "n_vor_gwg"), ("e2", "rentner_gesamt")] {
        let rumpf = format!(r#"{{"fall_id": "{id}", "scheibe": "{scheibe}"}}"#);
        let a = post_json(&d, "/fall", Some(&alice), &rumpf).await;
        assert_eq!(a.status, 201, "{}", a.text());
    }
    // Eine leere Adresse genuegt fuer 400 (`von` oder `nach`).
    for rumpf in [
        r#"{"von": "", "nach": "Berlin"}"#,
        r#"{"von": "Berlin", "nach": ""}"#,
        r#"{"von": "Berlin"}"#,
        r"{}",
    ] {
        let a = post_json(&d, "/fall/e1/entfernung", Some(&alice), rumpf).await;
        assert_eq!(a.status, 400, "{rumpf}: {}", a.text());
        assert!(
            a.text().contains("von und nach (Adressen) sind Pflicht"),
            "{}",
            a.text()
        );
    }
    // Beide Adressen da, aber die Scheibe fuehrt kein Feld `ep_entfernung_km`.
    let rumpf = r#"{"von": "Berlin", "nach": "Hamburg"}"#;
    let a = post_json(&d, "/fall/e2/entfernung", Some(&alice), rumpf).await;
    assert_eq!(a.status, 400, "{}", a.text());
    assert!(
        a.text()
            .contains("diese Scheibe hat kein Arbeitsweg-km-Feld"),
        "{}",
        a.text()
    );
}

/// `C118`: `grund_ohne_zahl` kennt `engine_unavailable` (Lage 4: `bescheid_fn` kennt die Quantitaet
/// nicht). Das gilt nur, wo eine Scheibe einen Gesamt-Ring OHNE Accessor hat — und die gibt es nicht:
/// jede der vier Scheiben mit Gesamt-Ring hat einen. Darum ist der Zweig `ohne_accessor && ...` im
/// Dienst unerreichbar, und `&&` gegen `||` ist dort nicht zu unterscheiden (aequivalenter Mutant).
#[test]
fn jede_scheibe_mit_gesamt_ring_hat_einen_accessor() {
    use bescheid::deklaration::Cfg;
    use bescheid::testhilfe::{felder, index, params, store};
    use bescheid::zweige::{bescheid_fn, Umgebung};
    use domain::{Scheibe, Vz};
    let umg = Umgebung {
        achsen: &[],
        index: index(),
        params: params(),
    };
    let leer = felder(&store(&[]));
    let mut mit_ring = 0;
    for scheibe in [
        Scheibe::Ep,
        Scheibe::NVorGwg,
        Scheibe::AnGesamt,
        Scheibe::Gesamt,
        Scheibe::RentnerGesamt,
    ] {
        let Some(q) = Cfg::fuer(scheibe).gesamt_ring() else {
            continue;
        };
        mit_ring += 1;
        assert!(
            bescheid_fn(q, Vz::Vz2025, &umg, Some(&leer), None, true, None, None).is_some(),
            "{scheibe:?}: kein Accessor fuer {q}"
        );
    }
    assert_eq!(mit_ring, 4);
}
