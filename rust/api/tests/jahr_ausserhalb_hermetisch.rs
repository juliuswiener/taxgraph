//! Ein Veranlagungsjahr, das kein `u16` und kein `i64` ist, an der HTTP-Naht (`rust/api`): der
//! Router darf es nie auf ein anderes Jahr wickeln (`as u16`) und nie still zu 0 machen. Standardlauf,
//! ohne Python-Server.
//!
//! Die Mutanten (Bericht h8-hermetisch5, Bestand 0aa91677, `cargo test -p api`, 201 passed / 0 failed): A7 `stand::jahr`
//! (`u16::try_from(j).ok()` -> `Some(j as u16)`), A8 `stand` Guard-Jahr, A9 `fragen` Guard-Jahr, A10 `ergebnis` Guard-Jahr,
//! A11 `deklaration` Ring-Jahr (jeweils `try_from(..).ok()` -> `as u16`), A12 `fall::loeschen` (`json!(vz as f64)` ->
//! `json!(0)` fuer ein Jahr ausserhalb i64), A13 `vorjahr` (`unwrap_or(i64::MAX)` -> `unwrap_or(0)` beim Beleg-Jahr).
//! Alle ueberleben den Bestand: keine Akte des Bestands hat ein Jahr ausserhalb 0..=65535. Mit diesen Tests werden A7, A12 und
//! A13 rot. A8-A11 faengt erst `ueberlauf_klassen_hermetisch.rs`: das
//! Guard-Jahr (A8-A10) ist nur ueber eine jahresabhaengige Sperre sichtbar (Abs. 3), A11 nur mit einem Jahr-abhaengigen Ring-Ueberlauf;
//! die leere Akte hier zeigt den Unterschied nicht (`deklariere` lehnt das Jahr ohnehin ab, `jahr()` lehnt es auf den anderen Routen ab).
//!
//! HERKUNFT DER ERWARTUNGSWERTE (gemessen 2026-10-04, `produkt/haut/api.py` im selben Prozess, `FAELLE`
//! auf ein Wegwerf-Verzeichnis): `deklaration` lehnt 67561 und -63511 mit `ValueError: Veranlagungsjahr
//! <vz> ist kein Steuerjahr (erwartet 2024..2100). ...` ab — das stuetzt A11 am Orakel. `stand`,
//! `fragen` und `ergebnis` liefern bei Python fuer jedes Jahr 200 (die leere Akte braucht keine
//! params/); Rust lehnt ein Jahr ausserhalb 2024..2026 mit 500 `ValueError` ab (Doku `stand::jahr`:
//! `Vz` kennt nur 2024-2026) — strenger als Python, fail-closed, KEINE Orakel-Stuetze; die Faelle
//! tragen "Konvention" im Namen. Der 200 der Wrap-Mutante waere dagegen eine Antwort fuer ein anderes
//! Jahr (67561 -> 2025). `DELETE` meldet das Jahr als JSON-Zahl; fuer 10^38 liefert Python die exakte
//! Ganzzahl, Rust `1e38` als Kommazahl (`fall.rs`, ponytail) — numerisch gleich. `vorjahr`: Python
//! schreibt `signal_1.vz` = 10^38 exakt, Rust sattigt auf `i64::MAX` (`vorjahr.rs:75`, ponytail).
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
    std::fs::create_dir_all(&konfig.faelle).unwrap();
    let auth = Auth::neu(
        "testgeheimnis".into(),
        tmp.path().join("users.json"),
        Some(konfig.audit_pfad()),
    );
    let token = auth.stelle_aus("alice").unwrap();
    Dienst {
        zustand: Zustand::neu(konfig, auth),
        token,
        _tmp: tmp,
    }
}

async fn sende(d: &Dienst, methode: &str, pfad: &str, body: Option<&Value>) -> (u16, Value) {
    let b = Request::builder()
        .method(methode)
        .uri(pfad)
        .header("authorization", format!("Bearer {}", d.token));
    let req = match body {
        Some(v) => {
            let t = v.to_string();
            b.header("content-type", "application/json")
                .header("content-length", t.len().to_string())
                .body(Body::from(t))
                .unwrap()
        }
        None => b.body(Body::empty()).unwrap(),
    };
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let status = r.status().as_u16();
    let bytes = r.collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Eine Akte von Hand (so entsteht ein Jahr ausserhalb 2024..2026: `POST /fall` prueft `params/`);
/// `vz` als Rohtext, damit auch 10^38 exakt in der Datei steht.
fn akte_schreiben(d: &Dienst, id: &str, scheibe: &str, vz: &str) {
    let text = format!(
        r#"{{"version": 1, "scheibe": "{scheibe}", "veranlagungszeitraum": {vz}, "user_id": "alice", "events": []}}"#
    );
    std::fs::write(d.zustand.konfig.faelle.join(format!("{id}.json")), text).unwrap();
}

/// Das Jahr ist bei Python ein `int`; hier die Wrap-Fallen: 67561 = 2025 + 65536, -63511 = 2025 - 65536,
/// 10^38 = sehr viel mehr als `i64`.
/// Ein bestaetigtes Event, wie `POST /fall/{id}/event` es nimmt.
fn ereignis(feld: &str, wert: i64) -> Value {
    json!({"feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:naht",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("klick@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00", "ersetzt": null})
}

const WRAP: [(&str, &str); 2] = [("67561", "67561"), ("-63511", "-63511")];

async fn fehler(d: &Dienst, pfad: &str) -> (u16, String) {
    let (s, a) = sende(d, "GET", pfad, None).await;
    (s, a["fehler"].as_str().unwrap_or("").to_owned())
}

/// A7-A10 (Konvention, kein Orakel): `stand`, `fragen`, `ergebnis` lehnen ein Jahr ausserhalb `u16` mit
/// 500 `ValueError` ab — nicht mit der Antwort fuer 2025. Gegenprobe: 2025 selbst rechnet (200).
#[tokio::test]
async fn ein_jahr_ausserhalb_u16_wird_nie_auf_ein_anderes_jahr_gewickelt_konvention() {
    let d = dienst();
    let mut falsch = Vec::new();
    for (vz, zeigt) in WRAP {
        akte_schreiben(&d, "w", "ep", vz);
        for route in ["stand", "fragen", "ergebnis"] {
            let (s, text) = fehler(&d, &format!("/fall/w/{route}")).await;
            let erwartet = format!("ValueError: kein unterstuetzter Veranlagungszeitraum: {zeigt}");
            if s != 500 || text != erwartet {
                falsch.push(format!(
                    "{route} bei Jahr {vz}: {s} {text:?}, erwartet 500 {erwartet:?}"
                ));
            }
        }
    }
    akte_schreiben(&d, "g", "ep", "2025");
    for route in ["stand", "fragen", "ergebnis"] {
        let (s, _) = sende(&d, "GET", &format!("/fall/g/{route}"), None).await;
        if s != 200 {
            falsch.push(format!(
                "Gegenprobe {route} bei Jahr 2025: {s}, erwartet 200"
            ));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// A11 (am Orakel): `deklaration` lehnt 67561 und -63511 mit dem Wortlaut von Python ab. Das Jahr
/// 2025 rechnet (Gegenprobe), `as u16` machte aus beiden 2025.
#[tokio::test]
async fn deklaration_lehnt_ein_jahr_ausserhalb_u16_wie_python_ab() {
    let d = dienst();
    let mut falsch = Vec::new();
    for (vz, zeigt) in WRAP {
        akte_schreiben(&d, "w", "ep", vz);
        let (s, text) = fehler(&d, "/fall/w/deklaration").await;
        let erwartet = format!(
            "ValueError: Veranlagungsjahr {zeigt} ist kein Steuerjahr (erwartet 2024..2100). "
        );
        if s != 500 || !text.starts_with(&erwartet) {
            falsch.push(format!(
                "Jahr {vz}: {s} {text:?}, erwartet 500 mit {erwartet:?}"
            ));
        }
    }
    akte_schreiben(&d, "g", "ep", "2025");
    let (s, _) = sende(&d, "GET", "/fall/g/deklaration", None).await;
    if s != 200 {
        falsch.push(format!("Gegenprobe Jahr 2025: {s}, erwartet 200"));
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// A12: `DELETE` meldet ein Jahr ausserhalb `i64` als Zahl, nie als 0. Python: der exakte `int` 10^38;
/// Rust: `1e38` (numerisch gleich, `as_f64`). Gegenproben: 2025 und 67561 stehen als Ganzzahl da.
#[tokio::test]
async fn delete_meldet_ein_jahr_ausserhalb_i64_als_zahl_nie_als_null() {
    let d = dienst();
    let mut falsch = Vec::new();
    for (vz, erwartet) in [
        ("100000000000000000000000000000000000000", 1e38),
        ("-100000000000000000000000000000000000000", -1e38),
        ("67561", 67561.0),
        ("2025", 2025.0),
    ] {
        akte_schreiben(&d, "x", "ep", vz);
        let (s, a) = sende(&d, "DELETE", "/fall/x", None).await;
        let ist = a["veranlagungszeitraum"].as_f64();
        if s != 200 || ist != Some(erwartet) {
            falsch.push(format!("Jahr {vz}: {s} {ist:?}, erwartet 200 {erwartet:e}"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// A13: ein Vorjahres-Fall mit einem Jahr ausserhalb `i64` (nur von Hand): der Beleg `signal_1.vz` der
/// uebernommenen Events traegt `i64::MAX` (Python: 10^38 exakt), nie 0.
#[tokio::test]
async fn vorjahr_beleg_mit_jahr_ausserhalb_i64_traegt_i64_max_nie_null() {
    let d = dienst();
    let feld = ereignis("bruttoarbeitslohn", 4_000_000);
    for (id, scheibe, vz) in [("vq", "gesamt", 2024), ("ziel", "gesamt", 2025)] {
        let rumpf = json!({"fall_id": id, "scheibe": scheibe, "veranlagungszeitraum": vz});
        assert_eq!(sende(&d, "POST", "/fall", Some(&rumpf)).await.0, 201);
    }
    assert_eq!(
        sende(&d, "POST", "/fall/vq/event", Some(&feld)).await.0,
        201
    );
    // Das Jahr der Quelle von Hand auf 10^38 setzen (Rohtext: `Value` haelt keine 39-stellige Ganzzahl).
    let pfad = d.zustand.konfig.faelle.join("vq.json");
    let text = std::fs::read_to_string(&pfad).unwrap();
    assert_eq!(
        text.matches("\"veranlagungszeitraum\":2024").count(),
        1,
        "Anker: {text}"
    );
    std::fs::write(
        &pfad,
        text.replace(
            "\"veranlagungszeitraum\":2024",
            "\"veranlagungszeitraum\":100000000000000000000000000000000000000",
        ),
    )
    .unwrap();
    let (s, a) = sende(
        &d,
        "POST",
        "/fall/ziel/vorjahr",
        Some(&json!({"vorjahr_fall_id": "vq"})),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    let akte: Value =
        serde_json::from_slice(&std::fs::read(d.zustand.konfig.faelle.join("ziel.json")).unwrap())
            .unwrap();
    let vz: Vec<&Value> = akte["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| &e["signal"]["signal_1"]["vz"])
        .collect();
    assert!(!vz.is_empty(), "keine uebernommenen Events: {akte}");
    for v in vz {
        assert_eq!(v, &json!(i64::MAX), "Beleg-Jahr");
    }
}
