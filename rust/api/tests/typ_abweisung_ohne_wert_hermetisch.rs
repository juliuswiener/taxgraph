//! Die Typ-Abweisung (`fail-closed (Typ)`) nennt den eingegebenen Wert nicht (Abweichung Nr. 30, Julius 2026-10-07,
//! Vault `decisions/fehlertexte-nennen-bei-typ-und-format-den-wert-nicht-mehr`). Standardlauf, ohne Python, ohne `PARITY=1`.
//!
//! Was der Test festhaelt: `POST /fall/{id}/event` weist einen Wert vom falschen Typ mit 422 ab. Der Text nennt Feld und
//! Bindungstyp, nie den Wert, und der Grund der Zeile `abgewiesen` im Ablaufprotokoll (`flow.jsonl`, nur mit
//! `TAXGRAPH_FLOW=1`) ebenso. Das Feld `wert` derselben Protokollzeile bleibt (Absicht, Abweichung Nr. 24, nicht Teil
//! dieser Entscheidung).
//!
//! Grenze: Der Test prueft den Typ-Text. Das Format-Gegenstueck steht in `kind_idnr_schreibweg_hermetisch.rs`; die Texte fuer
//! Groesse, Vorzeichen und Bereich nennen den Betrag weiter (`event_naht.rs`).
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
    token: String,
    tmp: tempfile::TempDir,
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
    Dienst { zustand, token, tmp }
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

/// Legt Fall `id` der Scheibe `gesamt` an (VZ 2025) und schreibt `wert` an `feld_id`, bestaetigt, als Nutzer-Klick.
async fn schreibe(d: &Dienst, id: &str, feld_id: &str, wert: &Value) -> (u16, Value) {
    let kopf = json!({"fall_id": id, "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall {id}: {antwort}");
    let rumpf = json!({
        "feld_id": feld_id, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld_id}")},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    sende(d, "POST", &format!("/fall/{id}/event"), Some(&rumpf)).await
}

/// (Name, Feld, Bindungstyp, falscher Wert, die Schreibweise des Wertes im Text von Python). Die Werte sind so gewaehlt, dass
/// ihre Schreibweise nirgends sonst im Text steht: eine Personenkennung als Zahl, ein Text im Cent-Feld, eine Kommazahl im
/// Ganzzahl-Feld, ein Wahrheitswert im Cent-Feld.
fn faelle() -> Vec<(&'static str, &'static str, &'static str, Value, &'static str)> {
    vec![
        ("IdNr als Zahl", "kind_idnr", "text", json!(13_579_246_007_i64), "13579246007"),
        ("Text im Cent-Feld", "bruttoarbeitslohn", "cent", json!("GEHEIM-123"), "GEHEIM-123"),
        ("Kommazahl im Ganzzahl-Feld", "ep_arbeitstage", "int", json!(1.5), "1.5"),
        ("Wahrheitswert im Cent-Feld", "bruttoarbeitslohn", "cent", json!(true), "True"),
    ]
}

/// Jeder Fall: 422, der Text nennt Feld und Typ, nicht den Wert.
#[tokio::test]
async fn typ_text_ohne_wert() {
    let d = dienst();
    for (n, (name, feld, typ, wert, schreibweise)) in faelle().into_iter().enumerate() {
        let (status, antwort) = schreibe(&d, &format!("typ-422-{n}"), feld, &wert).await;
        assert_eq!(status, 422, "{name}: erwartet 422: {antwort}");
        let text = antwort["fehler"].as_str().unwrap();
        assert!(
            text.starts_with(&format!(
                "fail-closed (Typ): {feld} passt nicht zum Bindungstyp '{typ}' — "
            )),
            "{name}: nicht der Typ-Text ohne Wert: {text}"
        );
        assert!(
            !text.contains(schreibweise),
            "{name}: der Text nennt den eingegebenen Wert {schreibweise:?}: {text}"
        );
    }
}

/// Der Grund im Ablaufprotokoll ist wertfrei, der Rest der Zeile bleibt (`feld_id`, `wert`: Abweichung Nr. 24).
#[tokio::test]
async fn ablaufprotokoll_grund_ohne_wert() {
    // Der Schalter ist prozessweit; die anderen Tests in dieser Datei schreiben dann nur eigene Zeilen in eigene Ablagen.
    std::env::set_var("TAXGRAPH_FLOW", "1");
    std::env::remove_var("TAXGRAPH_KI_DEBUG");
    let d = dienst();
    let (status, antwort) = schreibe(&d, "typ-flow", "kind_idnr", &json!(13_579_246_007_i64)).await;
    assert_eq!(status, 422, "{antwort}");
    let datei = d.tmp.path().join("faelle").join("flow.jsonl");
    let inhalt = std::fs::read_to_string(&datei).unwrap_or_else(|e| panic!("{datei:?}: {e}"));
    let zeilen: Vec<Value> = inhalt
        .lines()
        .map(|z| serde_json::from_str(z).unwrap())
        .collect();
    let abgewiesen: Vec<&Value> = zeilen.iter().filter(|z| z["art"] == "abgewiesen").collect();
    assert_eq!(abgewiesen.len(), 1, "genau eine Zeile 'abgewiesen': {inhalt}");
    let nutzlast = &abgewiesen[0]["inhalt"];
    let grund = nutzlast["grund"].as_str().unwrap();
    assert!(
        grund.starts_with("fail-closed (Typ): kind_idnr passt nicht zum Bindungstyp 'text'"),
        "{grund}"
    );
    assert!(!grund.contains("13579246007"), "der Grund nennt den Wert: {grund}");
    assert_eq!(nutzlast["feld_id"], "kind_idnr");
    assert_eq!(nutzlast["wert"], 13_579_246_007_i64, "das Feld `wert` bleibt (Abweichung Nr. 24)");
}
