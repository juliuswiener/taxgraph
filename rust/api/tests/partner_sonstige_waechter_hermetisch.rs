//! Probe zum Vault-Ticket `ehegatte-sonstige-einkuenfte-auf-gesamt-ohne-waechter`: sperrt auf der Scheibe
//! `gesamt` bei Zusammenveranlagung etwas, wenn der Ehegatte das Kreuz `kein_sonstige_partner` bejaht
//! ("Ja, mein Partner hat Rente oder sonstige Einkuenfte")? Die Hauptperson wird bei `kein_sonstige = false`
//! gesperrt (`fremd_arten` von `gesamt`); fuer den Ehegatten fand der Scope-Fork keinen Waechter
//! (`kein_sonstige_partner` steht weder im Pflicht-Kegel noch in `fremd_arten`).
//!
//! Faelle (alle ueber die echten Routen `POST /fall`, `POST /fall/{id}/event`, `GET /fall/{id}/ergebnis`):
//!
//! - K0 Kontrolle: `gesamt` zusammen, Partner nur Nullen, ohne das Kreuz -> Zahl (Baseline `GZ0`).
//! - K1 Kontrolle: dasselbe, `kein_sonstige_partner = true` -> Zahl `GZ0` (das Kreuz wird angenommen und
//!   sperrt von sich aus nicht).
//! - H1 Vergleich Hauptperson: `kein_sonstige = false` -> Sperre `einkunftsart_nicht_ring_faehig`.
//! - P1 Probe: `kein_sonstige_partner = false` -> Erwartung: Sperre statt Zahl.
//! - P2 Probe: P1 plus Partner-Rente `rentner_jahresrente_partner` -> das Feld ist auf `gesamt` nicht
//!   erfassbar (400, `aw_ksp_gesamt_partnerrente_abgewiesen` in `flag_und_schalter_hermetisch.rs`); gemessen
//!   wird, was `GET /ergebnis` danach liefert.
//!
//! `GZ0` stammt aus `flag_und_schalter_hermetisch.rs` (Python-Orakel 2026-10-03), kein Wert ist aus dem
//! Rust-Code abgelesen.
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

/// `gesamt` zusammen (Partner nur Nullen), AN 200.000 EUR, alle Kreuze "nein".
const GZ0: i64 = 6_162_800;

const SPERRE_HAUPTPERSON: &str = "einkunftsart_nicht_ring_faehig";

type Paare = Vec<(&'static str, Value)>;

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

/// Legt einen Fall der Scheibe an, schreibt jedes Paar ueber `POST /event` (Nutzer-Klick, bestaetigt) und
/// liefert `GET /ergebnis` samt den Feldern, die `POST /event` NICHT mit 201 angenommen hat.
async fn lauf(scheibe: &str, paare: &[(&'static str, Value)]) -> (Vec<(String, u16)>, Value) {
    let d = dienst();
    let kopf = json!({"fall_id": "psp", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let mut abgewiesen = Vec::new();
    for (feld, wert) in paare {
        let rumpf = json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (status, _) = sende(&d, "POST", "/fall/psp/event", Some(&rumpf)).await;
        if status != 201 {
            abgewiesen.push(((*feld).to_owned(), status));
        }
    }
    let (status, antwort) = sende(&d, "GET", "/fall/psp/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    (abgewiesen, antwort)
}

/// Ersetzt den Wert von `feld`; fehlt es, haengt es an.
fn mit(mut paare: Paare, feld: &'static str, wert: Value) -> Paare {
    match paare.iter_mut().find(|(f, _)| *f == feld) {
        Some(p) => p.1 = wert,
        None => paare.push((feld, wert)),
    }
    paare
}

/// Pflicht-Kegel `gesamt` (35 Felder) samt `kein_sonstige = true` und `kein_p23_verkauf = true`, zusammen
/// mit dem Partner-Kegel (`bruttoarbeitslohn_partner`, fuenf KAP-Felder, Nullen). Wie `gesamt_zusammen()` in
/// `flag_und_schalter_hermetisch.rs`.
fn gesamt_zusammen() -> Paare {
    vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(0)),
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(20_000_000)),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
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
        ("kap_kapitalertraege", json!(0)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
        ("kein_p23_verkauf", json!(true)),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ]
}

/// Die Antwort in einer Zeile: Grund und Zahl, damit ein Lauf mit `--nocapture` sie zeigt.
fn zeile(name: &str, abgewiesen: &[(String, u16)], a: &Value) -> String {
    let z = format!(
        "{name}: grund={} zahl_cent={:?} abgewiesen={abgewiesen:?}",
        a["grund"],
        a["zahl_cent"].as_i64()
    );
    eprintln!("{z}");
    z
}

fn erwarte_zahl(name: &str, abgewiesen: &[(String, u16)], a: &Value, zahl: i64) {
    let z = zeile(name, abgewiesen, a);
    assert_eq!(
        (a["grund"].as_str(), a["zahl_cent"].as_i64()),
        (Some("bestaetigt"), Some(zahl)),
        "{name}: Kontrolle -- erwartet bestaetigt mit {zahl} Cent. {z}. Antwort: {a}"
    );
}

#[tokio::test]
async fn k0_gesamt_zusammen_ohne_kreuz_rechnet() {
    let (ab, a) = lauf("gesamt", &gesamt_zusammen()).await;
    assert!(ab.is_empty(), "K0: Kegel nicht angenommen: {ab:?}");
    erwarte_zahl("K0", &ab, &a, GZ0);
}

#[tokio::test]
async fn k1_partner_kreuz_verneint_rechnet() {
    let paare = mit(gesamt_zusammen(), "kein_sonstige_partner", json!(true));
    let (ab, a) = lauf("gesamt", &paare).await;
    assert!(ab.is_empty(), "K1: Kreuz oder Kegel nicht angenommen: {ab:?}");
    erwarte_zahl("K1", &ab, &a, GZ0);
}

#[tokio::test]
async fn h1_hauptperson_kreuz_bejaht_sperrt() {
    let paare = mit(gesamt_zusammen(), "kein_sonstige", json!(false));
    let (ab, a) = lauf("gesamt", &paare).await;
    let z = zeile("H1", &ab, &a);
    assert!(
        a["zahl_cent"].as_i64().is_none() && a["grund"] == SPERRE_HAUPTPERSON,
        "H1: erwartet Sperre {SPERRE_HAUPTPERSON:?} ohne Zahl. {z}. Antwort: {a}"
    );
}

/// DIE PROBE. Rot heisst: der Ehegatte bejaht seine sonstigen Einkuenfte und TaxGraph rechnet eine Zahl.
#[tokio::test]
async fn p1_partner_kreuz_bejaht_sperrt() {
    let paare = mit(gesamt_zusammen(), "kein_sonstige_partner", json!(false));
    let (ab, a) = lauf("gesamt", &paare).await;
    let z = zeile("P1", &ab, &a);
    assert!(
        ab.is_empty(),
        "P1: das Kreuz selbst wurde nicht angenommen: {ab:?}"
    );
    assert!(
        a["zahl_cent"].as_i64().is_none() && a["grund"] != "bestaetigt",
        "P1: erwartet Sperre statt Zahl; der Ehegatte hat 'sonstige Einkuenfte' bejaht. {z}. \
         Differenz gegen GZ0 ({GZ0}): {:?} Cent. Antwort: {a}",
        a["zahl_cent"].as_i64().map(|x| x - GZ0)
    );
}

/// Partner-Rente zusaetzlich: das Feld ist auf `gesamt` nicht erfassbar. Gemessen wird beides: die
/// Abweisung und was `GET /ergebnis` danach liefert.
#[tokio::test]
async fn p2_partner_kreuz_bejaht_und_partner_rente_wird_abgewiesen() {
    let mut paare = mit(gesamt_zusammen(), "kein_sonstige_partner", json!(false));
    paare.push(("rentner_jahresrente_partner", json!(1_500_000)));
    let (ab, a) = lauf("gesamt", &paare).await;
    let z = zeile("P2", &ab, &a);
    assert_eq!(
        ab,
        vec![("rentner_jahresrente_partner".to_owned(), 400)],
        "P2: erwartet genau die Abweisung der Partner-Rente mit 400. {z}"
    );
    assert!(
        a["zahl_cent"].as_i64().is_none() && a["grund"] != "bestaetigt",
        "P2: erwartet Sperre statt Zahl. {z}. Antwort: {a}"
    );
}
