//! Der Deckel der KI-Aufrufe je Kontoauszug (50) MIT einem Klassifikator: was `kontoauszug_naht.rs` bis zum Bankimport-Fehler
//! „Deckel-Hinweis ohne KI“ (Vault `backlog/taxgraph/bankimport-echte-bankdatei-zeigt-depotblock-deckelhinweis-und-umlaut-fehler`,
//! AK2) ohne Anbieter pruefte, steht hier gegen ein Attrappen-Modell (`llm/tests/stub`). Ohne `$LLM_*` gibt es keinen Deckel
//! (Test `echtdatei_ohne_ki_kein_deckelhinweis` in `kontoauszug_naht.rs`); mit Anbieter gilt er wie in Python.
//!
//! Die Zahlen und Texte der drei Auszuege (56 Buchungen ohne Stichwort + eine Spende + ein unlesbarer Betrag; genau 50; 51) sind
//! die des frueheren Tests in `kontoauszug_naht.rs`, die ein Lauf von `api.kontoauszug` (`CPython` 3.14) OHNE Schluessel lieferte.
//! Mit Anbieter und der Antwort „keine Kategorie“ zaehlt der Deckel dort gleich (derselbe Zaehler); Python selbst lief mit
//! Anbieter nicht (ungemessen). Neu ist hier nur, dass der Anbieter wirklich da ist und die Aufrufe gezaehlt werden.
//!
//! Die Umgebung (`$LLM_API_BASE` u. a.) gilt fuer den ganzen Prozess; darum steht alles in EINEM Test, und die Datei hat ein
//! eigenes Test-Ziel.
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

#[path = "../../llm/tests/stub/mod.rs"]
mod stub;
use stub::{antwort, Aktion, Stub};

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

async fn post(d: &Dienst, pfad: &str, rumpf: &Value) -> (u16, Value) {
    let text = rumpf.to_string();
    let req = Request::builder()
        .method("POST")
        .uri(pfad)
        .header("authorization", format!("Bearer {}", d.token))
        .header("content-type", "application/json")
        .header("content-length", text.len().to_string())
        .body(Body::from(text))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (
        teile.status.as_u16(),
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Ein Auszug an einen frischen Fall `id` der Scheibe `gesamt`.
async fn auszug(d: &Dienst, id: &str, inhalt: Vec<Value>) -> (u16, Value) {
    let (s, a) = post(
        d,
        "/fall",
        &json!({"fall_id": id, "scheibe": "gesamt", "veranlagungszeitraum": 2025}),
    )
    .await;
    assert_eq!(s, 201, "POST /fall: {a}");
    post(
        d,
        &format!("/fall/{id}/kontoauszug"),
        // Das Modell sieht einen Buchungstext nur mit dem Haken (d-59).
        &json!({"format": "json", "inhalt": inhalt, "ki_buchungstext": true}),
    )
    .await
}

fn einkaeufe(n: i64) -> Vec<Value> {
    (0..n)
        .map(|i| json!({"datum": "01.03.2025", "betrag": -100 - i, "verwendungszweck": format!("Einkauf {i}")}))
        .collect()
}

/// Die Antwort des Modells: keine Kategorie.
fn keine_kategorie() -> Aktion {
    Aktion::Roh(antwort(
        200,
        &json!({
            "provider": "StubAnbieter",
            "choices": [{"finish_reason": "stop", "message": {"content": json!({"kategorie": null}).to_string()}}]
        })
        .to_string(),
    ))
}

#[tokio::test]
async fn mit_ki_gilt_der_deckel_von_50_aufrufen_und_der_hinweis_nennt_ihn() {
    // Genug Antworten fuer drei Auszuege zu je hoechstens 50 Aufrufen und fuer die fuenf der Kontrolle am Ende.
    let stub = Stub::starte((0..160).map(|_| keine_kategorie()).collect());
    std::env::set_var("LLM_API_BASE", stub.basis(""));
    std::env::set_var("LLM_MODEL", "stub/modell");
    std::env::set_var("LLM_API_KEY", "SYNTHETISCH-LLM-SCHLUESSEL");
    std::env::remove_var("TAXGRAPH_FLOW");
    let d = dienst();

    // 56 Buchungen ohne Stichwort, eine Spende (Stichwort, kein Aufruf), ein unlesbarer Betrag: 50 Aufrufe, 6 uebersprungen.
    let mut tx = einkaeufe(56);
    tx.push(json!({"datum": "01.03.2025", "betrag": -5000, "verwendungszweck": "Spende"}));
    tx.push(json!({"datum": "01.03.2025", "betrag": "x", "verwendungszweck": "Maler"}));
    let (status, antwort) = auszug(&d, "kd", tx).await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(
        antwort,
        json!({"uebernommen": 1, "transaktionen": 57, "verworfen": 1, "llm_uebersprungen": 6,
            "hinweis": "1 Zeile(n) mit unlesbarem Betrag (keine Zahl oder ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen. 6 Buchung(en) wurden NICHT automatisch eingeordnet — die Grenze von 50 Klassifikationen je Auszug war erreicht. Bitte diese Buchungen selbst zuordnen oder den Auszug in kleineren Zeiträumen hochladen."})
    );
    assert_eq!(stub.anfragen().len(), 50, "der Deckel: genau 50 Aufrufe");

    // Genau am Deckel: nichts wird uebersprungen, und der Schluessel fehlt.
    let (status, antwort) = auszug(&d, "kd2", einkaeufe(50)).await;
    assert_eq!(
        (status, antwort),
        (
            200,
            json!({"uebernommen": 0, "transaktionen": 50, "verworfen": 0})
        )
    );
    assert_eq!(stub.anfragen().len(), 100);

    // EINE uebersprungene Buchung ist schon ein Hinweis wert (Python: `> 0`; Mutant K40).
    let (status, antwort) = auszug(&d, "kd3", einkaeufe(51)).await;
    assert_eq!(
        (status, antwort),
        (
            200,
            json!({"uebernommen": 0, "transaktionen": 51, "verworfen": 0, "llm_uebersprungen": 1,
                "hinweis": "1 Buchung(en) wurden NICHT automatisch eingeordnet — die Grenze von 50 Klassifikationen je Auszug war erreicht. Bitte diese Buchungen selbst zuordnen oder den Auszug in kleineren Zeiträumen hochladen."})
        )
    );
    assert_eq!(stub.anfragen().len(), 150);

    // Bankimport S3: die Vorschau ruft das Modell NIE, auch nicht mit dem Haken (zwei Aufrufe, die zweite Einordnung koennte von der
    // gezeigten abweichen; die Firmenerkennung braucht spaeter eine Vorschau mit KI-Treffern und eine Zwischenablage je Fall). Sie
    // nennt die Zahl der Buchungen ohne Stichwort; das Uebernehmen mit demselben Haken fragt das Modell fuer jede.
    let (status, vorschau) = post(
        &d,
        "/fall/kd/bank/vorschau",
        &json!({"format": "json", "inhalt": einkaeufe(5), "ki_buchungstext": true}),
    )
    .await;
    assert_eq!(status, 200, "{vorschau}");
    assert_eq!(vorschau["ohne_stichwort"], json!(5), "{vorschau}");
    assert_eq!(vorschau["haken"]["ki_buchungstext"], json!(true), "{vorschau}");
    assert_eq!(stub.anfragen().len(), 150, "die Vorschau hat das Modell gefragt");
    let (status, antwort) = post(
        &d,
        "/fall/kd/kontoauszug",
        &json!({"format": "json", "inhalt": einkaeufe(5), "ki_buchungstext": true}),
    )
    .await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(stub.anfragen().len(), 155, "Kontrolle: das Uebernehmen fragt das Modell fuer jede Buchung ohne Stichwort");
}
