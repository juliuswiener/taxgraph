//! Der Schreibweg der Identifikationsnummer des Kindes (`kind_idnr`) an der HTTP-Naht, mit der ECHTEN Bindung aus
//! `rust/bindung/daten` (Auftrag `IdNr` Stufe 1, Julius 2026-10-06: „die Steueridentifikationsnummer auch in Rust testen“).
//! Standardlauf, ohne Python, ohne `PARITY=1`.
//!
//! Was der Test festhaelt: `POST /fall/{id}/event` nimmt genau 11 Ziffern an (201) und weist alles andere mit 422 und der
//! Meldung „fail-closed (Format)“ ab: 10 Ziffern, 12 Ziffern, 11 Buchstaben, 11 Ziffern mit angehaengtem Zeilenumbruch. Das
//! gilt fuer die erste Kind-Instanz (`kind_idnr`) wie fuer die zweite (`kind_idnr__2`).
//!
//! Warum hier und nicht im Store: Der Store-Test `muster_prueft_den_ganzen_wert_auflage_f` setzt das Muster selbst
//! (`[0-9]{11}`) und pinnt damit den Rahmen `^(?:muster)$`, nicht das Muster der Daten. `bindung/tests/muster_pruefung.rs` prueft,
//! dass das Datenmuster kompiliert und den Beispielwert annimmt, lehnt aber nichts ab. Ohne diesen Test wuerde ein Muster
//! `[0-9]{10,12}` in `bindung_kap_vv_familie.yaml` den Schreibweg lockern, ohne dass ein Rust-Test rot wird (nur der eingefrorene
//! Python-Vergleich unter `PARITY=1`).
//!
//! Grenzen: Der Test prueft Laenge, Zeichenklasse und Zeilenumbruch, KEINE Pruefziffer und keinen Aufbau der `IdNr` (beides steht
//! nicht in `sources/`, Bericht `kind-vorarbeit.md`, Abschnitt 3.1). Er prueft nur den Schreibweg: Der Rechenweg liest `kind_idnr`
//! weicher (>= 11 Zeichen), siehe `rust/bescheid/tests/kind_idnr_ist_zustand_hermetisch.rs`.
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

/// Die Instanzen, an denen das Muster gilt: die erste Kind-Instanz hat die Basis-ID, die zweite `__2`.
const FELDER: [&str; 2] = ["kind_idnr", "kind_idnr__2"];

/// Genau 11 Ziffern gehen durch, an beiden Instanzen. Der Beispielwert der Bindung ist `13579246007`.
#[tokio::test]
async fn elf_ziffern_werden_angenommen() {
    let d = dienst();
    for (i, feld) in FELDER.iter().enumerate() {
        let (status, antwort) =
            schreibe(&d, &format!("idnr-ok-{i}"), feld, &json!("13579246007")).await;
        assert_eq!(
            status, 201,
            "{feld}: 11 Ziffern muessen durchgehen: {antwort}"
        );
    }
}

/// Jede Abweichung vom Muster `11 Ziffern` ist ein 422 mit der Meldung „fail-closed (Format)“, an beiden Instanzen.
/// Die Meldung nennt das Feld und das Muster, nie die eingegebene Nummer (eine Personenkennung, `abweisung.rs`).
#[tokio::test]
async fn jede_abweichung_vom_muster_wird_mit_422_abgewiesen() {
    let d = dienst();
    // (Name, Wert). Die Namen stehen in der Meldung, die Werte nicht.
    let faelle: [(&str, &str); 4] = [
        ("10 Ziffern", "1234567890"),
        ("12 Ziffern", "123456789012"),
        ("11 Buchstaben", "abcdefghijk"),
        ("11 Ziffern mit Zeilenumbruch", "13579246007\n"),
    ];
    let mut n = 0;
    for feld in FELDER {
        for (name, wert) in faelle {
            n += 1;
            let (status, antwort) =
                schreibe(&d, &format!("idnr-422-{n}"), feld, &json!(wert)).await;
            assert_eq!(status, 422, "{feld} {name}: erwartet 422: {antwort}");
            let text = antwort.to_string();
            assert!(
                text.contains("fail-closed (Format)"),
                "{feld} {name}: 422, aber nicht die Format-Abweisung: {text}"
            );
            // `trim_end`: JSON maskiert den Umbruch als `\n`; mit ihm im Suchtext fände die Prüfung nie etwas.
            let nummer = wert.trim_end();
            assert!(
                !text.contains(nummer),
                "{feld} {name}: die Meldung nennt die eingegebene Nummer {nummer:?}: {text}"
            );
            // Gegenprobe gegen eine leere Meldung: Feld und Muster stehen weiter drin.
            assert!(
                text.contains(feld) && text.contains("passt nicht zum Muster '"),
                "{feld} {name}: die Meldung nennt Feld und Muster nicht mehr: {text}"
            );
        }
    }
}

/// Eine Zahl statt eines Textes ist keine `IdNr`: 422 (Typ), auch wenn sie elf Ziffern hat. Sonst las der Ring sie als Zahl.
#[tokio::test]
async fn eine_zahl_statt_text_wird_abgewiesen() {
    let d = dienst();
    let (status, antwort) =
        schreibe(&d, "idnr-zahl", "kind_idnr", &json!(13_579_246_007_i64)).await;
    assert_eq!(status, 422, "Zahl an kind_idnr: erwartet 422: {antwort}");
}
