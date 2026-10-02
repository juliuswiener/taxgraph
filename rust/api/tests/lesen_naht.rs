//! `GET /fall/{id}/feld/{fid}/warum`, `graph`, `preflight` und `ergebnis` ohne Python: die Gestalt aus
//! `api.warum` (`api.py:548`), `api.graph` (`api.py:833`), `api.preflight_check` (`api.py:641`) und
//! `api.ergebnis` (`api.py:558`), an einem Fall der Scheibe `ep` (sechs Felder), mit Events aus der
//! Store-Bibliothek.
//!
//! Dass die Antworten Byte fuer Byte denen von Python gleichen, prueft der Differenz-Harness
//! (`rust/parity/tests/api_http_paritaet.rs`, `generatoren`); dieser Test haelt die Form fest und
//! faellt auch dort, wo kein Python laeuft.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashMap;
use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEvent, Store};
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

async fn sende(
    d: &Dienst,
    methode: &str,
    pfad: &str,
    token: &str,
    body: Option<&str>,
) -> (u16, Value) {
    let mut b = Request::builder()
        .method(methode)
        .uri(pfad)
        .header("authorization", format!("Bearer {token}"));
    if let Some(t) = body {
        b = b
            .header("content-type", "application/json")
            .header("content-length", t.len().to_string());
    }
    let req = b
        .body(body.map_or_else(Body::empty, |t| Body::from(t.to_owned())))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (teile.status.as_u16(), json)
}

/// Legt Fall `sonde` (Scheibe `ep`, Nutzer alice) an und liefert das Token.
async fn fall_anlegen(d: &Dienst) -> String {
    let token = d.zustand.auth.stelle_aus("alice").unwrap();
    let rumpf = r#"{"fall_id": "sonde", "scheibe": "ep", "veranlagungszeitraum": 2025}"#;
    let (status, antwort) = sende(d, "POST", "/fall", &token, Some(rumpf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    token
}

/// Haengt ein bestaetigtes Event an die Akte an (so wie `POST /event` es taete) und gibt seine Kennung.
fn event_anhaengen(d: &Dienst, feld: &str, wert: i64) -> String {
    event_mit_wert(d, feld, json!(wert))
}

/// Wie [`event_anhaengen`], mit beliebigem JSON-Wert (ein Bool-Feld).
fn event_mit_wert(d: &Dienst, feld: &str, wert: Value) -> String {
    let pfad = d.zustand.konfig.faelle.join("sonde.json");
    let mut store = Store::aus_datei(store::lade(&pfad).unwrap());
    let neu = NeuesEvent {
        feld_id: feld.to_owned(),
        wert: wert.into(),
        feldzustand: Feldzustand::Bestaetigt {
            signal_2: Signal2::new("klick@naht").unwrap(),
        },
        herkunft: Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: Schreiber::Mensch("ui:naht".to_owned()),
        signal_1: None,
        ersetzt: None,
        ts: Some("2026-01-01T00:00:00+00:00".to_owned()),
    };
    let leer = HashMap::new();
    let id = store
        .append(&neu, None, BindungNachschlag::neu(&leer))
        .unwrap();
    store::speichere(&pfad, store.datei()).unwrap();
    id.to_string()
}

/// Ohne Event 404 mit Pythons Text (`!r` des Felds), kein 200 mit leerer Hülle.
#[tokio::test]
async fn warum_ohne_event_ist_404() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let (status, antwort) = sende(
        &d,
        "GET",
        "/fall/sonde/feld/ep_arbeitstage/warum",
        &token,
        None,
    )
    .await;
    assert_eq!(status, 404, "{antwort}");
    assert_eq!(
        antwort,
        json!({"fehler": "Feld 'ep_arbeitstage' hat (noch) kein Event"})
    );
}

/// Mit Event: das Justification-Objekt aus Event und Bindung, alle zehn Schluessel (`traverser.py:746`).
#[tokio::test]
async fn warum_mit_event_liefert_die_justification() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let event_id = event_anhaengen(&d, "ep_arbeitstage", 220);
    let (status, antwort) = sende(
        &d,
        "GET",
        "/fall/sonde/feld/ep_arbeitstage/warum",
        &token,
        None,
    )
    .await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(antwort["fall_id"], "sonde");
    let j = antwort["justification"].as_object().unwrap();
    let mut schluessel: Vec<&str> = j.keys().map(String::as_str).collect();
    schluessel.sort_unstable();
    assert_eq!(
        schluessel,
        [
            "anker_ref",
            "event_id",
            "feld_id",
            "geltungsbedingung",
            "herkunft",
            "regel_id",
            "signal",
            "signatur_slot",
            "wert",
            "zustand"
        ]
    );
    assert_eq!(j["feld_id"], "ep_arbeitstage");
    assert_eq!(j["wert"], 220);
    assert_eq!(j["zustand"], "bestaetigt");
    assert_eq!(j["event_id"], event_id.as_str());
    assert_eq!(j["herkunft"]["herkunft"], "laie");
    assert_eq!(j["signal"]["signal_2"], "klick@naht");
    assert!(
        j["regel_id"].is_string(),
        "die Bindung nennt die Regel: {j:?}"
    );
    assert!(j["anker_ref"]["zitatanker"].is_string(), "{j:?}");
    // Das Event eines Felds beantwortet nicht die Frage nach einem anderen.
    let pfad = "/fall/sonde/feld/ep_entfernung_km/warum";
    let (status, antwort) = sende(&d, "GET", pfad, &token, None).await;
    assert_eq!(status, 404, "{antwort}");
}

/// `graph`: eine Kante je Feld der Scheibe (`ep`: sechs), nach `feld_id` sortiert, `zustand` `offen`
/// bis ein Event da ist und dann der des Events; Knoten sind die Regeln mit ihrem Status.
#[tokio::test]
async fn graph_der_scheibe_ep() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let (status, vorher) = sende(&d, "GET", "/fall/sonde/graph", &token, None).await;
    assert_eq!(status, 200, "{vorher}");
    assert_eq!(vorher["fall_id"], "sonde");
    assert!(vorher["snapshot_id"]
        .as_str()
        .is_some_and(|s| s.len() == 64));
    let kanten = vorher["kanten"].as_array().unwrap();
    let felder: Vec<&str> = kanten
        .iter()
        .map(|k| k["feld_id"].as_str().unwrap())
        .collect();
    let mut sortiert = felder.clone();
    sortiert.sort_unstable();
    assert_eq!(felder, sortiert, "Kanten nach feld_id sortiert");
    assert_eq!(
        felder,
        [
            "ep_arbeitstage",
            "ep_eigenes_kfz",
            "ep_entfernung_km",
            "ep_oepnv_kosten",
            "ep_ziel_adresse",
            "ep_ziel_des_weges"
        ]
    );
    assert!(kanten.iter().all(|k| k["zustand"] == "offen"));
    // Gemessen 2026-10-02 mit `api.graph` auf demselben Fall: alle sechs Felder speisen die Regel
    // `p09_entfernungspauschale` als Slot, und die Regel ist `relevant`.
    assert!(kanten
        .iter()
        .all(|k| k["rolle"] == "slot" && k["regel_id"] == "p09_entfernungspauschale"));
    assert_eq!(
        vorher["knoten"],
        json!([{"regel_id": "p09_entfernungspauschale", "status": "relevant",
                "gates_offen": [], "annahmen_offen": []}])
    );

    event_anhaengen(&d, "ep_arbeitstage", 220);
    let (_, nachher) = sende(&d, "GET", "/fall/sonde/graph", &token, None).await;
    let k = nachher["kanten"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["feld_id"] == "ep_arbeitstage")
        .unwrap();
    assert_eq!(k["zustand"], "bestaetigt");
    assert_ne!(
        nachher["snapshot_id"], vorher["snapshot_id"],
        "neues Event, neuer Snapshot"
    );
}

/// `preflight`: die Ampel folgt den Events, die Items stehen in Pythons Bereichs-Reihenfolge. Auf
/// `ep` ist `kein_kap` nicht fragbar, ein Kapitalbetrag neben dem nie gestellten Flag ist deshalb
/// kein Widerspruch (auf `gesamt` waere er einer).
#[tokio::test]
async fn preflight_ampel_und_items() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let holen = || sende(&d, "GET", "/fall/sonde/preflight", &token, None);
    let (status, leer) = holen().await;
    assert_eq!(status, 200, "{leer}");
    assert_eq!(
        leer,
        json!({"fall_id": "sonde", "status": "GREEN", "items": []})
    );

    event_anhaengen(&d, "bruttoarbeitslohn", 4_000_000);
    let (_, gelb) = holen().await;
    assert_eq!(gelb["status"], "AMBER");
    assert_eq!(
        gelb["items"],
        json!([{"typ": "hinweis", "bereich": "pauschale",
                "text": "Arbeitslohn vorhanden, aber keine Anzahl an Arbeitstagen für die \
                         Entfernungspauschale angegeben. Möglicherweise wurde die Pauschale vergessen."}])
    );

    event_anhaengen(&d, "ep_arbeitstage", 220);
    let (_, gruen) = holen().await;
    assert_eq!(gruen["status"], "GREEN", "{gruen}");
    assert_eq!(gruen["items"], json!([]));

    // Nur der Hinweis auf den Sparer-Pauschbetrag, kein `flag`-Widerspruch: `kein_kap` ist auf `ep`
    // nicht fragbar.
    event_anhaengen(&d, "kap_kapitalertraege", 10_000);
    let sparer = json!({"typ": "hinweis", "bereich": "pauschale",
        "text": "Kapitaleinkünfte vorhanden, aber der Sparer-Pauschbetrag (1.000/2.000 €) ist nur mit \
                 Angabe der Veranlagungsart korrekt bestimmbar."});
    let (_, gelb2) = holen().await;
    assert_eq!(gelb2["status"], "AMBER", "{gelb2}");
    assert_eq!(gelb2["items"], json!([sparer]));

    // Widersprueche stehen vor den Hinweisen.
    event_anhaengen(&d, "p36_lohnsteuer", 5_000_000);
    let (_, rot) = holen().await;
    assert_eq!(rot["status"], "RED");
    assert_eq!(
        rot["items"],
        json!([{"typ": "widerspruch", "bereich": "plausibilitaet",
                "text": "Bei einem Bruttoarbeitslohn von 40.000 € kann dein Arbeitgeber nicht 50.000 € \
                         Lohnsteuer einbehalten haben — die Lohnsteuer wird vom Lohn abgezogen und ist \
                         deshalb immer kleiner als der Lohn. Bitte prüfe, welche der beiden Zahlen stimmt."},
              sparer])
    );
}

/// `ergebnis`: ohne Angaben die offenen Kegel-Felder (sortiert) und ein Klartext, mit allen vier die
/// Zahl der Entfernungspauschale samt Trace. Gemessen 2026-10-02 mit `api.ergebnis` auf demselben
/// Fall: 215.600 Cent.
#[tokio::test]
async fn ergebnis_offen_dann_zahl() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let holen = || sende(&d, "GET", "/fall/sonde/ergebnis", &token, None);
    let (status, offen) = holen().await;
    assert_eq!(status, 200, "{offen}");
    assert_eq!(offen["grund"], "input_kegel_nicht_bestaetigt");
    assert_eq!(
        offen["offen"],
        json!([
            "ep_arbeitstage",
            "ep_eigenes_kfz",
            "ep_entfernung_km",
            "ep_oepnv_kosten"
        ])
    );
    assert_eq!(offen["zahl_cent"], Value::Null);
    assert!(offen["klartext"].as_str().is_some_and(|k| !k.is_empty()));

    event_anhaengen(&d, "ep_arbeitstage", 220);
    event_anhaengen(&d, "ep_entfernung_km", 30);
    event_anhaengen(&d, "ep_oepnv_kosten", 0);
    event_mit_wert(&d, "ep_eigenes_kfz", json!(true));
    let (_, zahl) = holen().await;
    assert_eq!(zahl["grund"], "bestaetigt", "{zahl}");
    assert_eq!(zahl["zahl_cent"], 215_600);
    assert_eq!(zahl["offen"], json!([]));
    assert!(
        zahl.get("klartext").is_none(),
        "kein Klartext bei einer Zahl"
    );
    assert_eq!(zahl["kette"], Value::Null);
    assert!(zahl["trace"]["regeln"]["p09_entfernungspauschale"].is_array());
}

/// `frage`: auch ein beantwortetes Feld bekommt seine Frage; `feld_id__n` loest sich auf das
/// Basisfeld auf; ein Feld ausserhalb der Scheibe ist 404 mit Pythons Text (`!r` der ganzen Kennung).
#[tokio::test]
async fn frage_einzeln_loest_die_instanz_auf() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    event_anhaengen(&d, "ep_arbeitstage", 220);
    let (status, basis) = sende(
        &d,
        "GET",
        "/fall/sonde/feld/ep_arbeitstage/frage",
        &token,
        None,
    )
    .await;
    assert_eq!(status, 200, "{basis}");
    assert_eq!(basis["fall_id"], "sonde");
    let frage = basis["frage"].as_object().unwrap();
    let mut schluessel: Vec<&str> = frage.keys().map(String::as_str).collect();
    schluessel.sort_unstable();
    assert_eq!(
        schluessel,
        [
            "anker_ref",
            "beispielwert",
            "bereich",
            "einheit",
            "enum_labels",
            "enum_werte",
            "feld_id",
            "frage_invertiert",
            "fragetext_laie",
            "hilfe_kurz",
            "instanz_anzahl",
            "instanz_etikett",
            "muster",
            "regel_id",
            "screening",
            "standardwert",
            "typ",
            "vorjahr_kategorie",
        ]
    );
    assert_eq!(frage["feld_id"], "ep_arbeitstage");
    assert_eq!(
        (&frage["instanz_anzahl"], &frage["instanz_etikett"]),
        (&json!(1), &json!(""))
    );
    // Mit Instanz-Suffix dieselbe Frage zum Basisfeld, nicht zur Kennung mit Suffix.
    let (status, mit_suffix) = sende(
        &d,
        "GET",
        "/fall/sonde/feld/ep_arbeitstage__2/frage",
        &token,
        None,
    )
    .await;
    assert_eq!(status, 200, "{mit_suffix}");
    assert_eq!(mit_suffix, basis);
    for fid in ["nicht_da__2", "bruttoarbeitslohn"] {
        let (status, fehler) = sende(
            &d,
            "GET",
            &format!("/fall/sonde/feld/{fid}/frage"),
            &token,
            None,
        )
        .await;
        assert_eq!(status, 404, "{fehler}");
        assert_eq!(
            fehler,
            json!({"fehler": format!("Feld '{fid}' nicht in dieser Scheibe")})
        );
    }
}
