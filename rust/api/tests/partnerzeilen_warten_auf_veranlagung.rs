//! Ticket "Einzelnutzer sieht vier Partner-Zeilen vor der Frage allein oder gemeinsam" (Stundenlauf-Qualifikation, 2026-10-24).
//!
//! **Worum es geht.** Die Ankreuzliste "Arbeitnehmer, Vermieter oder Kapital" kommt vor der Frage "allein oder gemeinsam". Sie zeigt darum
//! vier Zeilen, die nur den Ehepartner betreffen, auch jemandem, der allein veranlagt wird. Julius (d-66, 2026-10-23): die Partnerzeilen bleiben
//! sichtbar und werden ausgegraut, wenn jemand Einzelveranlagung angibt. Dafuer muss die Seite wissen, WORAUF eine Zeile wartet.
//!
//! **Was dieser Test haelt.** `GET /fall/{id}/fragen` markiert genau die vier Partnerzeilen mit `wartet_auf` = `{feld, wert}` (aus der
//! `feld_bedingung` der Bindung); keine andere Frage der Liste traegt den Schluessel. Die Zeilen bleiben in der Liste.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::collections::BTreeSet;
use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

/// Die vier Zeilen der Partner-Ankreuzliste (`bindung_screening_partner.yaml`), je mit `feld_bedingung veranlagung = zusammen`.
const PARTNERZEILEN: [&str; 4] = [
    "kein_kap_partner",
    "kein_gewinn_partner",
    "kein_sonstige_partner",
    "keine_behinderung_pflege_partner",
];

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
    let auth = Auth::neu("testgeheimnis".into(), tmp.path().join("users.json"), Some(konfig.audit_pfad()));
    let zustand = Zustand::neu(konfig, auth);
    let token = zustand.auth.stelle_aus("alice").unwrap();
    Dienst { zustand, token, _tmp: tmp }
}

async fn sende(d: &Dienst, methode: &str, pfad: &str, body: Option<&Value>) -> (u16, Value) {
    let text = body.map(ToString::to_string);
    let mut b = Request::builder().method(methode).uri(pfad).header("authorization", format!("Bearer {}", d.token));
    if let Some(t) = &text {
        b = b.header("content-type", "application/json").header("content-length", t.len().to_string());
    }
    let req = b.body(text.map_or_else(Body::empty, Body::from)).unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (teile.status.as_u16(), serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn neuer_fall(scheibe: &str) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "pz", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    d
}

/// Die ganze Antwort von `GET /fall/pz/fragen` (Liste und `ring_gesperrt`).
async fn fragen_voll(d: &Dienst) -> Value {
    let (status, antwort) = sende(d, "GET", "/fall/pz/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {antwort}");
    antwort
}

/// `GET /fragen` auf einem frischen Fall der Scheibe `gesamt`: die Ehegatten-Zeilen warten auf `veranlagung = zusammen`.
///
/// Gebaut in `api/src/fragen.rs` (`frage_metadaten`, Schluessel `wartet_auf`); war rot bis zum Bau. Das Kommando:
/// `cargo nextest run -p api --test it partnerzeilen_warten_auf_veranlagung::die_ehegatten`.
#[tokio::test]
async fn die_ehegatten_zeilen_warten_auf_die_veranlagung() {
    let d = neuer_fall("gesamt").await;
    let antwort = fragen_voll(&d).await;
    let liste = antwort["fragen"].as_array().unwrap();
    // Die Zeilen bleiben sichtbar: Julius will sie ausgegraut sehen, nicht verborgen (d-66).
    for zeile in PARTNERZEILEN {
        let q = liste.iter().find(|q| q["feld_id"] == json!(zeile)).unwrap_or_else(|| panic!("{zeile} fehlt in der Fragenliste"));
        assert_eq!(q["screening"], json!(true), "{zeile}: keine Zeile der Ankreuzliste");
        assert_eq!(
            q["wartet_auf"],
            json!({"feld": "veranlagung", "wert": "zusammen"}),
            "{zeile}: die Zeile nennt nicht, worauf sie wartet: {q}"
        );
    }
    // Genau diese vier: keine andere Frage der Liste (auch keine andere Zeile der Ankreuzliste) traegt den Schluessel.
    let wartende: BTreeSet<&str> = liste
        .iter()
        .filter(|q| q.get("wartet_auf").is_some_and(|w| !w.is_null()))
        .map(|q| q["feld_id"].as_str().unwrap())
        .collect();
    assert_eq!(wartende, PARTNERZEILEN.into_iter().collect::<BTreeSet<_>>(), "wartet_auf steht nicht an genau den vier Partnerzeilen");
    // Die Frage, auf die sie warten, steht mit Werten und Beschriftungen in derselben Liste (nicht als Zeile der Ankreuzliste): daraus baut die Seite
    // die Wahl allein/gemeinsam, ohne `veranlagung` fest zu verdrahten.
    let wahl = liste.iter().find(|q| q["feld_id"] == json!("veranlagung")).expect("veranlagung fehlt in der Fragenliste eines frischen Falls");
    assert_eq!(wahl["screening"], json!(false), "veranlagung ist keine Zeile der Ankreuzliste");
    assert_eq!(wahl["enum_werte"], json!(["einzel", "zusammen"]), "Werte der Wahl: {wahl}");
    for wert in ["einzel", "zusammen"] {
        assert!(wahl["enum_labels"][wert].as_str().is_some_and(|t| !t.is_empty()), "{wert}: ohne Beschriftung: {wahl}");
    }
}

/// Schreibt eine bestaetigte Antwort; hat das Feld schon ein aktives Ereignis, ersetzt die Antwort es (wie die Oberflaeche).
async fn schreibe(d: &Dienst, feld: &str, wert: Value) {
    let (_, stand) = sende(d, "GET", "/fall/pz/stand", None).await;
    let mut rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    if let Some(id) = stand["felder"][feld]["event_id"].as_str() {
        rumpf["ersetzt"] = json!(id);
    }
    let (status, antwort) = sende(d, "POST", "/fall/pz/event", Some(&rumpf)).await;
    assert_eq!(status, 201, "POST {feld}: {antwort}");
}

/// Wechsel von gemeinsam auf allein (Messung 2026-10-24, `gesamt`, VZ 2025; gruen vor dem Bau, muss gruen bleiben): was der Nutzer zu den
/// Partner-Zeilen gespeichert hat, wird unter Einzelveranlagung nie still mitgerechnet. Ein gespeicherter BETRAG des Partners sperrt laut
/// (`partner_konsistenz_offen`, Zahl null, die Zeilen und Felder des Partners stehen nicht mehr in der Liste); ein gespeichertes „ja“ der
/// Zeile ohne Betrag (`kein_sonstige_partner`) wird ignoriert und sperrt nicht.
#[tokio::test]
async fn der_wechsel_auf_allein_laesst_gespeicherte_partnerzeilen_nie_still_mitrechnen() {
    type Fall = (&'static str, Vec<(&'static str, Value)>, Value);
    let faelle: [Fall; 4] = [
        ("ja und Betrag", vec![("kein_kap_partner", json!(false)), ("kap_kapitalertraege_partner", json!(500_000))], json!("partner_konsistenz_offen")),
        ("Zeile offen, Betrag", vec![("kap_kapitalertraege_partner", json!(500_000))], json!("partner_konsistenz_offen")),
        ("nein und Betrag", vec![("kein_kap_partner", json!(true)), ("kap_kapitalertraege_partner", json!(500_000))], json!("partner_konsistenz_offen")),
        ("ja ohne Betrag (sonstige)", vec![("kein_sonstige_partner", json!(false))], Value::Null),
    ];
    for (name, paare, erwartet) in faelle {
        let d = neuer_fall("gesamt").await;
        schreibe(&d, "veranlagung", json!("zusammen")).await;
        for (feld, wert) in paare {
            schreibe(&d, feld, wert).await;
        }
        schreibe(&d, "veranlagung", json!("einzel")).await;
        let antwort = fragen_voll(&d).await;
        assert_eq!(antwort["ring_gesperrt"], erwartet, "{name}: ring_gesperrt nach dem Wechsel auf allein: {antwort}");
        let ids: Vec<&str> = antwort["fragen"].as_array().unwrap().iter().map(|q| q["feld_id"].as_str().unwrap()).collect();
        for zeile in PARTNERZEILEN {
            assert!(!ids.contains(&zeile), "{name}: {zeile} steht unter Einzelveranlagung in der Liste");
        }
        if erwartet.is_string() {
            let (status, e) = sende(&d, "GET", "/fall/pz/ergebnis", None).await;
            assert_eq!(status, 200, "{name}: {e}");
            assert_eq!(e["grund"], erwartet, "{name}: /ergebnis nennt einen anderen Grund als /fragen: {e}");
            assert!(e["zahl_cent"].is_null(), "{name}: trotz Sperre eine Zahl: {e}");
        }
    }
}
