//! `GET /fall/{id}/ergebnis`: welche Regeln `feste_zahl` aus dem Kegel nimmt, entscheidet die Relevanz
//! nach der Bindung der SCHEIBE (Pythons `_scheibe_bindung`), nicht nach dem ganzen Graphen.
//!
//! Ein Gate, das die Scheibe nicht führt (hier `vpf_auswaertige_taetigkeit` in `an_gesamt`, `POST
//! /event` weist es dort ab), schließt die Regel § 9 Abs. 4a nicht aus: `fragen` verlangt die drei
//! `tage_*` weiter, also darf `/ergebnis` keine Zahl nennen. Die Akte kommt von Hand (Events über die
//! Store-Bibliothek, ohne Scheiben-Prüfung); der Differenz-Harness (`api_http_paritaet`, Fall `g_an6`)
//! zeigt dasselbe gegen Python, dieser Test fällt auch dort, wo kein Python läuft.
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

/// Der Pflicht-Kegel von `an_gesamt` (33 Felder) samt der Angaben, die den Guard zufriedenstellen,
/// mit neutralen Werten (`api_http_paritaet::kegel_an_voll`).
fn kegel_an_voll() -> Vec<(&'static str, Value)> {
    vec![
        ("bruttoarbeitslohn", json!(4_000_000)),
        ("veranlagung", json!("einzel")),
        ("ep_entfernung_km", json!(30)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(true)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!("gesetzlich_an")),
        ("basis_kv", json!(0)),
        ("basis_pv", json!(0)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(false)),
        ("dhf_unterkunftskosten_monat", json!(0)),
        ("dhf_im_inland", json!(false)),
        ("dhf_beruflich_veranlasst", json!(false)),
        ("dhf_eigener_hausstand", json!(false)),
        ("dhf_finanzielle_beteiligung", json!(false)),
        ("tage_24h", json!(1)),
        ("tage_an_abreise", json!(1)),
        ("tage_ueber_8h_eintaegig", json!(1)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("verlustvortrag_bestand", json!(0)),
        ("ep_arbeitstage", json!(220)),
        ("fam_anzahl_kinder", json!(0)),
        ("dhf_monate", json!(0)),
        ("vpf_monate_am_ort", json!(2)),
        ("vpf_keine_mahlzeitengestellung", json!(true)),
    ]
}

/// Legt Fall `id` der Scheibe `an_gesamt` an und hängt je Feld ein bestätigtes Event an die Akte
/// (so wie `POST /event` es täte, aber ohne dessen Prüfung gegen die Scheibe).
async fn fall_mit(d: &Dienst, id: &str, felder: &[(&str, Value)]) {
    let rumpf = json!({"fall_id": id, "scheibe": "an_gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&rumpf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let pfad = d.zustand.konfig.faelle.join(format!("{id}.json"));
    let mut store = Store::aus_datei(store::lade(&pfad).unwrap());
    let leer = HashMap::new();
    for (feld, wert) in felder {
        let neu = NeuesEvent {
            feld_id: (*feld).to_owned(),
            wert: wert.clone().into(),
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
        store
            .append(&neu, None, BindungNachschlag::neu(&leer))
            .unwrap();
    }
    store::speichere(&pfad, store.datei()).unwrap();
}

#[tokio::test]
async fn gate_ausserhalb_der_scheibe_schliesst_keine_regel_aus() {
    let d = dienst();
    // Kontrolle: der volle Kegel gibt eine Zahl frei.
    fall_mit(&d, "voll", &kegel_an_voll()).await;
    let (status, antwort) = sende(&d, "GET", "/fall/voll/ergebnis", None).await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(antwort["grund"], "bestaetigt", "{antwort}");

    // Ohne die drei `tage_*`, mit verneintem Gate ausserhalb der Scheibe: keine Zahl, die drei
    // Felder bleiben offen (Pythons Relevanz kennt das Gate nicht).
    let mut felder = kegel_an_voll();
    felder.retain(|(f, _)| {
        !matches!(
            *f,
            "tage_24h" | "tage_an_abreise" | "tage_ueber_8h_eintaegig"
        )
    });
    felder.push(("vpf_auswaertige_taetigkeit", json!(false)));
    fall_mit(&d, "gate", &felder).await;
    let (status, antwort) = sende(&d, "GET", "/fall/gate/ergebnis", None).await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(
        antwort["grund"], "input_kegel_nicht_bestaetigt",
        "{antwort}"
    );
    assert_eq!(
        antwort["offen"],
        json!(["tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig"])
    );
    assert!(antwort["zahl_cent"].is_null(), "{antwort}");
}
