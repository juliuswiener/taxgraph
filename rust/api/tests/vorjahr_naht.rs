//! `POST /fall/{id}/vorjahr` ohne Python: Übernahme, Reihenfolge der Events, Altwerte, `vorjahr_referenz`
//! und die zweite Fall-Kennung des Rumpfs. Die Antworten stammen aus einem Lauf von `api.vorjahr`
//! (`CPython` 3.14); dass beide Server in 46 weiteren Formen gleich antworten, prüft der
//! Differenz-Harness (`rust/parity/tests/api_http_paritaet.rs`, `generatoren`). Dieser Test hält die
//! Gestalt fest und fällt auch dort, wo kein Python läuft.
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

async fn sende(d: &Dienst, nutzer: &str, pfad: &str, body: &Value) -> (u16, Value) {
    let token = d.zustand.auth.stelle_aus(nutzer).unwrap();
    let text = body.to_string();
    let req = Request::builder()
        .method("POST")
        .uri(pfad)
        .header("authorization", format!("Bearer {token}"))
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

async fn fall_anlegen(d: &Dienst, nutzer: &str, id: &str, scheibe: &str, vz: i64) {
    let rumpf = json!({"fall_id": id, "scheibe": scheibe, "veranlagungszeitraum": vz});
    let (status, antwort) = sende(d, nutzer, "/fall", &rumpf).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
}

/// Ein bestätigtes Ereignis der Oberfläche.
fn ereignis(feld: &str, wert: &Value) -> Value {
    json!({"feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:naht",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("klick@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00", "ersetzt": null})
}

/// Ein Vorjahres-Fall `id` (Scheibe `gesamt`, 2024) mit bestätigten Feldern.
async fn quelle(d: &Dienst, id: &str, felder: &[(&str, Value)]) {
    fall_anlegen(d, "alice", id, "gesamt", 2024).await;
    for (feld, wert) in felder {
        let (s, a) = sende(
            d,
            "alice",
            &format!("/fall/{id}/event"),
            &ereignis(feld, wert),
        )
        .await;
        assert_eq!(s, 201, "{feld}: {a}");
    }
}

fn akte(d: &Dienst, id: &str) -> Value {
    let pfad = d.zustand.konfig.faelle.join(format!("{id}.json"));
    serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap()
}

fn stamm() -> Vec<(&'static str, Value)> {
    vec![
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(4_000_000)),
        ("geburtsjahr", json!(1980)),
        ("stammdaten_nachname", json!("Müller")),
        ("hh_handwerker_betrag", json!(48_000)),
        ("ep_entfernung_km", json!(30)),
        ("verlustvortrag_bestand", json!(123_456)),
    ]
}

#[tokio::test]
async fn uebernahme_vorlaeufig_in_der_reihenfolge_der_scheibe() {
    let d = dienst();
    quelle(&d, "vq1", &stamm()).await;
    // Ein vorlaeufiger Vorschlag im Vorjahr wird nicht uebernommen.
    let llm = json!({"feld_id": "ep_arbeitstage", "wert": 200, "zustand": "vorlaeufig", "schreiber": "llm:naht",
        "herkunft": {"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": null}, "ts": "2026-01-01T00:00:00+00:00", "ersetzt": null});
    assert_eq!(sende(&d, "alice", "/fall/vq1/event", &llm).await.0, 201);
    fall_anlegen(&d, "alice", "ziel", "gesamt", 2025).await;
    let rumpf = json!({"vorjahr_fall_id": "vq1"});
    let (s, a) = sende(&d, "alice", "/fall/ziel/vorjahr", &rumpf).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(
        a,
        json!({"uebernommen": 6, "uebersprungen": [], "vorjahr_fall_id": "vq1"})
    );
    let ziel = akte(&d, "ziel");
    let events = ziel["events"].as_array().unwrap();
    for e in events {
        assert_eq!(e["schreiber"], "import:vorjahr");
        assert_eq!(e["zustand"], "vorlaeufig");
        assert_eq!(e["herkunft"]["herkunft"], "vorjahr");
        assert!(e["signal"]["signal_2"].is_null());
        assert_eq!(e["signal"]["signal_1"]["vz"], 2024);
        assert_eq!(e["signal"]["signal_1"]["typ"], "vorjahr");
    }
    // Reihenfolge der Events: die der Felder der Scheibe, nicht die nach Namen sortierte.
    let mut felder: Vec<&str> = events
        .iter()
        .map(|e| e["feld_id"].as_str().unwrap())
        .collect();
    let in_scheibenreihenfolge = felder.clone();
    felder.sort_unstable();
    assert_ne!(
        in_scheibenreihenfolge, felder,
        "Reihenfolge der Scheibe fehlt"
    );
    assert_eq!(
        in_scheibenreihenfolge,
        [
            "veranlagung",
            "bruttoarbeitslohn",
            "ep_entfernung_km",
            "hh_handwerker_betrag",
            "geburtsjahr",
            "stammdaten_nachname"
        ]
    );
    // Der Beleg nennt Kategorie und Quellwert.
    let e = &events[1];
    assert_eq!(e["signal"]["signal_1"]["quell_wert"], 4_000_000);
    assert_eq!(e["signal"]["signal_1"]["kategorie"], "vorschlag");
    // Die Vergleichsgroesse steht in der Akte; ein zweiter Lauf findet alles belegt.
    assert_eq!(
        ziel["vorjahr_referenz"],
        json!({"verlustvortrag_bestand": {"wert": 123_456}})
    );
    let (s, a) = sende(&d, "alice", "/fall/ziel/vorjahr", &rumpf).await;
    assert_eq!(
        (s, a),
        (
            200,
            json!({"uebernommen": 0, "uebersprungen": [], "vorjahr_fall_id": "vq1"})
        )
    );
    // Eine Quelle mit neuem Bestand ersetzt die Vergleichsgroesse, eine ohne laesst sie stehen.
    quelle(&d, "vq6", &[("verlustvortrag_bestand", json!(7))]).await;
    quelle(&d, "vq7", &[]).await;
    sende(
        &d,
        "alice",
        "/fall/ziel/vorjahr",
        &json!({"vorjahr_fall_id": "vq6"}),
    )
    .await;
    assert_eq!(
        akte(&d, "ziel")["vorjahr_referenz"]["verlustvortrag_bestand"]["wert"],
        7
    );
    sende(
        &d,
        "alice",
        "/fall/ziel/vorjahr",
        &json!({"vorjahr_fall_id": "vq7"}),
    )
    .await;
    assert_eq!(
        akte(&d, "ziel")["vorjahr_referenz"]["verlustvortrag_bestand"]["wert"],
        7
    );
}

#[tokio::test]
async fn altwert_den_die_pruefung_abweist_wird_uebersprungen() {
    let d = dienst();
    quelle(&d, "vq2", &stamm()).await;
    // Von Hand: Werte, die die heutige Wertpruefung abweist (V, W, T).
    let pfad = d.zustand.konfig.faelle.join("vq2.json");
    let mut a = akte(&d, "vq2");
    for e in a["events"].as_array_mut().unwrap() {
        match e["feld_id"].as_str().unwrap() {
            "hh_handwerker_betrag" => e["wert"] = json!(-5000),
            "geburtsjahr" => e["wert"] = json!(1899),
            "stammdaten_nachname" => e["wert"] = json!("Maier\u{0}"),
            _ => {}
        }
    }
    std::fs::write(pfad, serde_json::to_vec(&a).unwrap()).unwrap();
    fall_anlegen(&d, "alice", "ziel", "gesamt", 2025).await;
    let (s, a) = sende(
        &d,
        "alice",
        "/fall/ziel/vorjahr",
        &json!({"vorjahr_fall_id": "vq2"}),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(
        a,
        json!({"uebernommen": 3, "uebersprungen": ["geburtsjahr", "hh_handwerker_betrag", "stammdaten_nachname"],
            "vorjahr_fall_id": "vq2"})
    );
    assert_eq!(akte(&d, "ziel")["events"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn die_zweite_kennung_geht_durch_dieselbe_pruefung() {
    let d = dienst();
    quelle(&d, "vq1", &stamm()).await;
    fall_anlegen(&d, "bob", "bobs", "gesamt", 2024).await;
    fall_anlegen(&d, "alice", "ziel", "gesamt", 2025).await;
    let fehlt = json!({"fehler": "vorjahr_fall_id fehlt oder ungültig"});
    let post = |wert: Value| {
        let d = &d;
        async move {
            sende(
                d,
                "alice",
                "/fall/ziel/vorjahr",
                &json!({"vorjahr_fall_id": wert}),
            )
            .await
        }
    };
    for wert in [
        json!(null),
        json!(""),
        json!(0),
        json!(false),
        json!([]),
        json!({}),
        json!("a/b"),
        json!("ä"),
        json!("x".repeat(65)),
        json!(" "),
        json!("a b"),
        json!("vq1\n"),
        json!(["vq1"]),
        json!({"a": 1}),
        json!(1.5),
        json!(1e22),
    ] {
        assert_eq!(post(wert.clone()).await, (400, fehlt.clone()), "{wert}");
    }
    let (s, a) = sende(&d, "alice", "/fall/ziel/vorjahr", &json!({})).await;
    assert_eq!((s, a), (400, fehlt));
    assert_eq!(
        post(json!("ziel")).await,
        (
            400,
            json!({"fehler": "vorjahr_fall_id muss ein ANDERER (Vorjahres-)Fall sein"})
        )
    );
    // Ganzzahl, `true` und `1e-05` bestehen die Textpruefung; `lade_fall` wirft `TypeError`.
    for (wert, typ) in [
        (json!(5), "int"),
        (json!(-5), "int"),
        (json!(i64::MAX), "int"),
        (json!(true), "bool"),
        (json!(1e-5), "float"),
    ] {
        assert_eq!(
            post(wert.clone()).await,
            (
                500,
                json!({"fehler": format!("TypeError: expected string or bytes-like object, got '{typ}'")})
            ),
            "{wert}"
        );
    }
    assert_eq!(post(json!("gibtsnicht")).await.0, 404);
    // Der Fall eines anderen Nutzers: 403, und das Ziel bleibt unberuehrt.
    let (s, a) = post(json!("bobs")).await;
    assert_eq!(s, 403, "{a}");
    assert_eq!(a, json!({"fehler": "Zugriff auf Fall 'bobs' verweigert"}));
    assert!(akte(&d, "ziel")["events"].as_array().unwrap().is_empty());
    // Der Rumpf ist kein Objekt: `body.get` scheitert.
    for (body, klasse) in [
        (json!([1]), "list"),
        (json!("vq1"), "str"),
        (json!(null), "NoneType"),
        (json!(5), "int"),
        (json!(true), "bool"),
    ] {
        let (s, a) = sende(&d, "alice", "/fall/ziel/vorjahr", &body).await;
        assert_eq!(
            (s, a),
            (
                500,
                json!({"fehler": format!("AttributeError: '{klasse}' object has no attribute 'get'")})
            ),
            "{body}"
        );
    }
}

#[tokio::test]
async fn ziel_scheibe_bestimmt_die_felder() {
    let d = dienst();
    quelle(&d, "vq1", &stamm()).await;
    fall_anlegen(&d, "alice", "zep", "ep", 2025).await;
    let (s, a) = sende(
        &d,
        "alice",
        "/fall/zep/vorjahr",
        &json!({"vorjahr_fall_id": "vq1"}),
    )
    .await;
    assert_eq!(
        (s, a),
        (
            200,
            json!({"uebernommen": 1, "uebersprungen": [], "vorjahr_fall_id": "vq1"})
        )
    );
    // Was im Ziel schon belegt ist, bleibt: 6 minus die zwei belegten.
    fall_anlegen(&d, "alice", "zbelegt", "gesamt", 2025).await;
    for (f, w) in [
        ("veranlagung", json!("zusammen")),
        ("geburtsjahr", json!(1970)),
    ] {
        sende(&d, "alice", "/fall/zbelegt/event", &ereignis(f, &w)).await;
    }
    let (_, a) = sende(
        &d,
        "alice",
        "/fall/zbelegt/vorjahr",
        &json!({"vorjahr_fall_id": "vq1"}),
    )
    .await;
    assert_eq!(a["uebernommen"], 4);
}

/// Die Scheibe `rentner_gesamt` führt jedes Feld einmal. Stand `geburtsjahr` zweimal in ihrer Liste,
/// brach die Übernahme am zweiten Eintrag mit 422 ab („hat schon ein aktives Event“); Entscheidung
/// `rentner-gesamt-fuehrt-jedes-feld-einmal-das-doppelte-geburtsjahr-faellt-weg`.
#[tokio::test]
async fn vorjahr_nach_rentner_gesamt_uebernimmt_das_geburtsjahr() {
    let d = dienst();
    quelle(&d, "vq1", &stamm()).await;
    fall_anlegen(&d, "alice", "zrg", "rentner_gesamt", 2025).await;
    let (s, a) = sende(
        &d,
        "alice",
        "/fall/zrg/vorjahr",
        &json!({"vorjahr_fall_id": "vq1"}),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    // `bruttoarbeitslohn` und `ep_entfernung_km` kennt die Scheibe nicht, `verlustvortrag_bestand`
    // ist eine Vergleichsgröße: 4 von 7 Feldern der Quelle.
    assert_eq!(
        a,
        json!({"uebernommen": 4, "uebersprungen": [], "vorjahr_fall_id": "vq1"})
    );
    let ziel = akte(&d, "zrg");
    let geburtsjahr: Vec<&Value> = ziel["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["feld_id"] == "geburtsjahr")
        .collect();
    assert_eq!(geburtsjahr.len(), 1, "genau ein Event fuer geburtsjahr");
    assert_eq!(geburtsjahr[0]["wert"], 1980);
    assert_eq!(geburtsjahr[0]["zustand"], "vorlaeufig");
}
