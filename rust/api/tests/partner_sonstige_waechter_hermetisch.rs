//! Vault-Backlog `ehegatte-sonstige-einkuenfte-auf-gesamt-ohne-waechter` (Abweichung Nr. 52): auf der Scheibe
//! `gesamt` sperrt bei Zusammenveranlagung das Kreuz `kein_sonstige_partner` = nein ("Ja, mein Partner hat Rente oder
//! sonstige Einkuenfte"). Vorher rechnete `TaxGraph` dieselbe Zahl wie bei "ja" (P1/P2 waren rot): die Hauptperson wurde bei
//! `kein_sonstige = false` gesperrt (`fremd_arten` von `gesamt`), fuer den Ehegatten fand der Scope-Fork keinen Waechter, und
//! sein Betragsfeld (`rentner_jahresrente_partner`) gibt es auf `gesamt` nicht -- die Einkuenfte waeren nie eingeflossen.
//!
//! Faelle (alle ueber die echten Routen `POST /fall`, `POST /fall/{id}/event`, `GET /fall/{id}/ergebnis`):
//!
//! - K0 Kontrolle: `gesamt` zusammen, Partner nur Nullen, ohne das Kreuz -> Zahl (Baseline `GZ0`).
//! - K1 Kontrolle: dasselbe, `kein_sonstige_partner = true` -> Zahl `GZ0` (das Kreuz wird angenommen und
//!   sperrt von sich aus nicht).
//! - H1 Vergleich Hauptperson: `kein_sonstige = false` -> Sperre `einkunftsart_nicht_ring_faehig`.
//! - P1: `kein_sonstige_partner = false` -> Sperre `partner_einkunftsart_nicht_ring_faehig`, keine Zahl, Klartext nennt den Partner.
//! - P2: P1 plus Partner-Rente `rentner_jahresrente_partner` -> das Feld ist auf `gesamt` nicht erfassbar (400,
//!   `aw_ksp_gesamt_partnerrente_abgewiesen` in `flag_und_schalter_hermetisch.rs`), die Sperre bleibt.
//! - P3: beide Kreuze bejaht -> der Grund der Hauptperson (Reihenfolge).
//! - P4: Einzelveranlagung mit uebrig gebliebenem Kreuz -> keine Wirkung (`ist_zusammen`).
//! - Q1 (AK3): die anderen Partner-Kreuze (Kapital, Gewinn, Behinderung) sind Gates fuer Felder, die `gesamt` rechnet.
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

/// Rust-eigener Grund (Abweichung Nr. 52): Python rechnet hier eine Zahl ohne die Einkuenfte des Ehegatten.
const SPERRE_PARTNER: &str = "partner_einkunftsart_nicht_ring_faehig";

/// Pruefung fuer P1/P2: Sperre mit dem Partner-Grund, keine Zahl, und ein Klartext, der den Partner nennt (nicht den
/// Text der Hauptperson, der den Nutzer selbst anspricht).
fn erwarte_partner_sperre(name: &str, z: &str, a: &Value) {
    assert!(
        a["zahl_cent"].as_i64().is_none() && a["grund"] == SPERRE_PARTNER,
        "{name}: erwartet Sperre {SPERRE_PARTNER:?} ohne Zahl; der Ehegatte hat 'sonstige Einkuenfte' bejaht. {z}. \
         Differenz gegen GZ0 ({GZ0}): {:?} Cent. Antwort: {a}",
        a["zahl_cent"].as_i64().map(|x| x - GZ0)
    );
    let klartext = a["klartext"].as_str().unwrap_or_default();
    assert!(
        klartext.contains("dein Ehe- oder Lebenspartner") && klartext.contains("zu niedrig"),
        "{name}: der Klartext nennt den Partner nicht: {klartext:?}"
    );
}

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
    let abgewiesen = fall_anlegen(&d, scheibe, paare).await;
    let (status, antwort) = sende(&d, "GET", "/fall/psp/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    (abgewiesen, antwort)
}

/// Legt Fall `psp` an und schreibt jedes Paar; liefert die Felder, die `POST /event` nicht mit 201 angenommen hat.
async fn fall_anlegen(d: &Dienst, scheibe: &str, paare: &[(&'static str, Value)]) -> Vec<(String, u16)> {
    let kopf = json!({"fall_id": "psp", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let mut abgewiesen = Vec::new();
    for (feld, wert) in paare {
        let rumpf = json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (status, _) = sende(d, "POST", "/fall/psp/event", Some(&rumpf)).await;
        if status != 201 {
            abgewiesen.push(((*feld).to_owned(), status));
        }
    }
    abgewiesen
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

/// Rot heisst: der Ehegatte bejaht seine sonstigen Einkuenfte und `TaxGraph` rechnet eine Zahl ohne sie.
#[tokio::test]
async fn p1_partner_kreuz_bejaht_sperrt() {
    let paare = mit(gesamt_zusammen(), "kein_sonstige_partner", json!(false));
    let (ab, a) = lauf("gesamt", &paare).await;
    let z = zeile("P1", &ab, &a);
    assert!(
        ab.is_empty(),
        "P1: das Kreuz selbst wurde nicht angenommen: {ab:?}"
    );
    erwarte_partner_sperre("P1", &z, &a);
}

/// Partner-Rente zusaetzlich: das Feld ist auf `gesamt` nicht erfassbar. Beides gilt: die Abweisung (400) und die
/// Sperre in `GET /ergebnis`.
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
    erwarte_partner_sperre("P2", &z, &a);
}

/// AK5, der Abgabeweg: derselbe Fall wie P1 baut weder die Deklaration (`GET /deklaration`, 409) noch reicht er ein
/// (`POST /einreichen`, 409, `eingereicht: false`); beide nennen den Partner-Grund samt Klartext. Die Sperre greift vor
/// `ERiC` (`EinreichFehler::Gesperrt`), also braucht der Test kein Zertifikat und kein Schema.
#[tokio::test]
async fn p5_abgabeweg_ist_gesperrt() {
    let d = dienst();
    let paare = mit(gesamt_zusammen(), "kein_sonstige_partner", json!(false));
    let ab = fall_anlegen(&d, "gesamt", &paare).await;
    assert!(ab.is_empty(), "P5: Fall nicht angenommen: {ab:?}");
    let (status, dk) = sende(&d, "GET", "/fall/psp/deklaration", None).await;
    assert_eq!(status, 409, "P5: GET /deklaration muss sperren: {dk}");
    assert_eq!(dk["grund"], SPERRE_PARTNER, "P5: GET /deklaration: {dk}");
    assert!(
        dk["klartext"].as_str().is_some_and(|k| k.contains("dein Ehe- oder Lebenspartner")),
        "P5: Klartext nennt den Partner nicht: {dk}"
    );
    let (status, ein) = sende(&d, "POST", "/fall/psp/einreichen", Some(&json!({}))).await;
    assert_eq!(status, 409, "P5: POST /einreichen muss sperren: {ein}");
    assert_eq!(
        (&ein["eingereicht"], &ein["grund"]),
        (&json!(false), &json!(SPERRE_PARTNER)),
        "P5: POST /einreichen: {ein}"
    );
    eprintln!("P5: deklaration=409 {} einreichen=409 {}", dk["grund"], ein["grund"]);
}

/// Beide Kreuze bejaht: die Hauptperson gewinnt, ihr Grund spricht den Nutzer selbst an (Reihenfolge der Pruefungen).
#[tokio::test]
async fn p3_hauptperson_und_partner_bejaht_meldet_den_grund_der_hauptperson() {
    let paare = mit(
        mit(gesamt_zusammen(), "kein_sonstige", json!(false)),
        "kein_sonstige_partner",
        json!(false),
    );
    let (ab, a) = lauf("gesamt", &paare).await;
    let z = zeile("P3", &ab, &a);
    assert!(
        a["zahl_cent"].as_i64().is_none() && a["grund"] == SPERRE_HAUPTPERSON,
        "P3: erwartet {SPERRE_HAUPTPERSON:?} ohne Zahl. {z}. Antwort: {a}"
    );
}

/// `gesamt_zusammen()` als Einzelveranlagung: ohne die Felder des Ehegatten (die brauchen `zusammen`).
fn gesamt_einzeln() -> Paare {
    let mut p: Paare = gesamt_zusammen()
        .into_iter()
        .filter(|(f, _)| !f.ends_with("_partner"))
        .collect();
    p = mit(p, "veranlagung", json!("einzel"));
    p
}

/// Das Kreuz des Ehegatten zaehlt nur bei Zusammenveranlagung (`ist_zusammen`): bei Einzelveranlagung gilt es nicht.
/// Messung der Grenze, kein Urteil: was dort mit einem uebrig gebliebenen Kreuz geschieht, entscheiden andere Waechter.
#[tokio::test]
async fn p4_einzelveranlagung_ignoriert_das_kreuz_des_ehegatten() {
    let (ab0, a0) = lauf("gesamt", &gesamt_einzeln()).await;
    let z0 = zeile("P4-K", &ab0, &a0);
    assert_eq!(
        a0["grund"], "bestaetigt",
        "P4-K: Kontrolle (einzeln ohne Kreuz) muss rechnen. {z0}. Antwort: {a0}"
    );
    let paare = mit(gesamt_einzeln(), "kein_sonstige_partner", json!(false));
    let (ab, a) = lauf("gesamt", &paare).await;
    let z = zeile("P4", &ab, &a);
    assert_eq!(
        (&a["grund"], &a["zahl_cent"]),
        (&a0["grund"], &a0["zahl_cent"]),
        "P4: ein uebrig gebliebenes Kreuz des Ehegatten aendert bei Einzelveranlagung nichts. {z}. Antwort: {a}"
    );
}

/// AK3: die anderen Partner-Kreuze der Scheibe `gesamt` (Kapital, Gewinn, Behinderung) sind Gates fuer Felder, die `gesamt`
/// rechnet: mit Betrag aendert sich die Zahl. Anders als `kein_sonstige_partner`, dessen Betragsfeld (`rentner_jahresrente_partner`)
/// auf `gesamt` fehlt. Prueft die Richtung, keinen Betrag: Kapital und Gewinn erhoehen die Steuer, ein Grad der Behinderung von 50
/// senkt sie (Pauschbetrag).
#[tokio::test]
async fn q1_andere_partner_kreuze_rechnen_ihren_betrag_mit() {
    for (kreuz, feld, betrag, hoeher) in [
        ("kein_kap_partner", "kap_kapitalertraege_partner", 1_000_000_i64, true),
        ("kein_gewinn_partner", "einkuenfte_gewinn_partner", 5_000_000, true),
        ("keine_behinderung_pflege_partner", "rentner_grad_der_behinderung_partner", 50, false),
    ] {
        let paare = mit(mit(gesamt_zusammen(), kreuz, json!(false)), feld, json!(betrag));
        let (ab, a) = lauf("gesamt", &paare).await;
        let z = zeile(&format!("Q1 {kreuz}+{feld}"), &ab, &a);
        let zahl = a["zahl_cent"].as_i64();
        assert!(
            ab.is_empty() && a["grund"] == "bestaetigt" && zahl.is_some_and(|x| (x > GZ0) == hoeher && x != GZ0),
            "Q1: {kreuz} + {feld}={betrag} muss die Zahl {} (GZ0 = {GZ0}). {z}. Antwort: {a}",
            if hoeher { "erhoehen" } else { "senken" }
        );
    }
}
