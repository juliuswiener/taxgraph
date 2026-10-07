//! `GET /fall/{id}/fragen` ohne Python: die Reihenfolge der Queue hängt an den Gewichten des
//! Gesamt-Rings (`api.fragen`, `api.py:1099`; `interview::naechste_fragen(…, beitrag)`). Fehlt der
//! Beitrag, wechselt die Reihenfolge; der Differenz-Harness (`api_http_paritaet`, `generatoren`)
//! sieht das nur mit `PARITY=1` und Python, dieser Test auch ohne.
//!
//! Der Fall: `an_gesamt` mit dem Pflicht-Kegel und `ep_arbeitstage` (31 Felder), dazu eine offene
//! Achse mit `bereich` — der Ring rechnet, jede offene Achse bekommt ein Gewicht (Spanne in Cent).
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

async fn sende(
    d: &Dienst,
    methode: &str,
    pfad: &str,
    token: &str,
    body: Option<Value>,
) -> (u16, Value) {
    let mut b = Request::builder()
        .method(methode)
        .uri(pfad)
        .header("authorization", format!("Bearer {token}"));
    let text = body.map(|v| v.to_string());
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

/// Der Pflicht-Kegel von `an_gesamt` mit neutralen Werten (wie `kegel_an` im Harness) und
/// `ep_arbeitstage`; offen bleibt unter anderem `fam_anzahl_kinder` (Achse mit `bereich` 0..20).
fn kegel() -> Vec<(&'static str, Value)> {
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
    ]
}

#[tokio::test]
async fn die_queue_ist_nach_den_gewichten_des_rings_geordnet() {
    let d = dienst();
    let token = d.zustand.auth.stelle_aus("alice").unwrap();
    let fall = json!({"fall_id": "fq", "scheibe": "an_gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", &token, Some(fall)).await;
    assert_eq!(status, 201, "{antwort}");
    for (feld, wert) in kegel() {
        let b = json!({"feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:naht",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": "klick"}, "ts": "2026-01-01T00:00:00+00:00"});
        let (status, antwort) = sende(&d, "POST", "/fall/fq/event", &token, Some(b)).await;
        assert_eq!(status, 201, "{feld}: {antwort}");
    }
    let (status, antwort) = sende(&d, "GET", "/fall/fq/fragen", &token, None).await;
    assert_eq!(status, 200, "{antwort}");
    let ids: Vec<&str> = antwort["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["feld_id"].as_str().unwrap())
        .collect();
    // 31 Fragen wie in Python, dazu die eine nur-Rust-Frage `ep_unfallkosten` (Abweichung Nr. 28): die Entfernung dieses Falls
    // ist unbeantwortet, und Schweigen schliesst die Frage nicht aus (`wert_nicht: 0`, fail-closed).
    assert_eq!(ids.iter().filter(|f| **f != "ep_unfallkosten").count(), 31);
    assert!(ids.contains(&"ep_unfallkosten"), "{ids:?}");
    assert_eq!(ids.len(), 32);
    // Die Python-Reihenfolge gilt fuer die uebrigen 31; die Frage steht, ohne Beitrag, alphabetisch unter den Slots.
    let ids: Vec<&str> = ids.into_iter().filter(|f| *f != "ep_unfallkosten").collect();
    // Gemessen mit Python (`g_an2` im Harness ist derselbe Fall): mit Gewichten steht die Adresse vor
    // dem Ziel des Weges, ohne Beitrag (`None`) kehrt sich das um.
    assert_eq!(
        &ids[..12],
        [
            "ep_ziel_adresse",
            "ep_ziel_des_weges",
            "vpf_frist_nicht_unterbrochen",
            "vpf_keine_mahlzeitengestellung",
            "vpf_abendessen_gestellt_anzahl",
            "vpf_fruehstuecke_gestellt_anzahl",
            "vpf_mahlzeiten_gezahltes_entgelt",
            "vpf_mittagessen_gestellt_anzahl",
            "vpf_monate_am_ort",
            "vpf_steuerfreie_erstattung_betrag",
            "vpf_tage_24h_nach_drei_monaten",
            "vpf_tage_an_abreise_nach_drei_monaten",
        ]
    );
}
