//! Kirchensteuer: die Frage nach dem Bundesland steht, wenn Person A ODER Person B einer Kirche angehoert (Entscheidung d-43,
//! Vault `decisions/julius-entscheidet-bundesland-frage-mit-oder-bedingung-2026-10-20`). Ohne `PARITY=1`, ohne Python, ohne Netz.
//!
//! **Worum es geht.** Seit Abweichung Nr. 94 rechnet die Software die Kirchensteuer bei Zusammenveranlagung nach Landesrecht und
//! je Person. Den Hebesatz des Bundeslandes (8 % in Bayern und Baden-Wuerttemberg, sonst 9 %) fragte sie aber nur, wenn Person A
//! einer Kirche angehoert. Gehoert nur der Ehegatte an, fehlte die Frage, und die Software rechnete mit 9 %.
//!
//! **Warum es zaehlt.** In Bayern und Baden-Wuerttemberg war die Kirchensteuer des Ehegatten um ein Neuntel zu hoch (Schaetzung aus
//! 9 zu 8). Umgekehrt darf die neue Bedingung die Frage nicht dort einblenden, wo kein Mitglied ist, und nicht bei Einzelveranlagung,
//! wo die Frage nach dem Ehegatten nie gestellt wird.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_p51a_kirchensteuer.yaml` (`kist_bundesland`: `feld_bedingung` mit `oder`),
//! `rust/bindung/src/bindung_datei.rs` (die Bedingungsart), `rust/interview/src/fragen.rs::feld_ausgeschlossen` (die Auswertung).
//!
//! **Belegt wird (positiver Beleg fuer die Alternative).** Die Bedingung auf A bleibt wie sie war (eine offene Antwort schliesst
//! nicht aus). Die Alternative (Ehegatte) zaehlt nur mit einer BESTAETIGTEN Mitgliedschaft; Schweigen, "keine" und ein Feld, das die
//! Scheibe oder die Veranlagung nicht fragt, halten die Frage nicht offen. Das kostet nichts: ohne bestaetigtes Mitglied gibt es
//! keine Kirchensteuer und keinen Hebesatz.
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

const BUNDESLAND: &str = "kist_bundesland";
const KONFESSION_A: &str = "kist_konfession";
const KONFESSION_B: &str = "kist_konfession_partner";

struct Dienst {
    zustand: Zustand,
    token: String,
    /// Haelt das Verzeichnis des Falls am Leben, solange der Dienst laeuft.
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

/// Ein bestaetigtes Ereignis (zwei Signale).
fn ereignis(feld_id: &str, wert: &Value) -> Value {
    json!({
        "feld_id": feld_id,
        "herkunft": {"haftung": "nutzer", "herkunft": "laie", "pruef_tiefe": "ungeprueft"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld_id}")},
        "wert": wert,
        "zustand": "bestaetigt",
        "ts": "2026-01-01T00:00:00+00:00",
    })
}

/// Legt den Fall `sb` der Scheibe `scheibe` (VZ 2025) an und schreibt jedes Paar ueber die echte Route `POST /event`.
async fn fall(d: &Dienst, scheibe: &str, paare: &[(&str, Value)]) {
    let kopf = json!({"fall_id": "sb", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (status, antwort) =
            sende(d, "POST", "/fall/sb/event", Some(&ereignis(feld, wert))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
}

/// Steht `kist_bundesland` in `GET /fragen`, wenn der Fall der Scheibe `scheibe` genau diese Antworten traegt?
async fn steht(scheibe: &str, paare: &[(&str, Value)]) -> bool {
    let d = dienst();
    fall(&d, scheibe, paare).await;
    let (status, fragen) = sende(&d, "GET", "/fall/sb/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .any(|q| q["feld_id"] == BUNDESLAND)
}

/// Zusammenveranlagung mit den Konfessionen von A und B (jeweils `None` = nicht beantwortet).
fn zusammen(a: Option<&str>, b: Option<&str>) -> Vec<(&'static str, Value)> {
    let mut paare = vec![("veranlagung", json!("zusammen"))];
    if let Some(a) = a {
        paare.push((KONFESSION_A, json!(a)));
    }
    if let Some(b) = b {
        paare.push((KONFESSION_B, json!(b)));
    }
    paare
}

/// Der Fehler: A ohne Kirche, B evangelisch. Die Frage nach dem Bundesland fehlte; der Hebesatz blieb bei 9 %.
#[tokio::test]
async fn nur_der_ehegatte_gehoert_einer_kirche_an_die_frage_steht() {
    for scheibe in ["gesamt", "rentner_gesamt", "an_gesamt"] {
        for b in ["evangelisch", "roemisch-katholisch"] {
            assert!(
                steht(scheibe, &zusammen(Some("keine"), Some(b))).await,
                "{scheibe}: A keine, B {b}: die Frage nach dem Bundesland fehlt"
            );
        }
    }
}

/// Gegenprobe 1: weder A noch B gehoert einer Kirche an. Kein Mitglied, kein Hebesatz, keine Frage.
#[tokio::test]
async fn weder_a_noch_b_gehoert_einer_kirche_an_die_frage_fehlt() {
    for scheibe in ["gesamt", "rentner_gesamt", "an_gesamt"] {
        assert!(
            !steht(scheibe, &zusammen(Some("keine"), Some("keine"))).await,
            "{scheibe}: A keine, B keine: die Frage steht"
        );
    }
}

/// Gegenprobe 2: A gehoert einer Kirche an. Die Frage stand schon vor der Aenderung und steht weiter, gleich was B antwortet.
#[tokio::test]
async fn gehoert_a_einer_kirche_an_steht_die_frage_wie_vorher() {
    for scheibe in ["gesamt", "rentner_gesamt", "an_gesamt"] {
        for b in [None, Some("keine"), Some("evangelisch")] {
            assert!(
                steht(scheibe, &zusammen(Some("evangelisch"), b)).await,
                "{scheibe}: A evangelisch, B {b:?}: die Frage fehlt"
            );
        }
    }
}

/// Gegenprobe 3 (wie vorher): A bestaetigt "keine", B unbeantwortet. Die Alternative braucht eine bestaetigte Mitgliedschaft; ohne sie
/// bleibt die Frage weg, so wie sie vor der Aenderung wegblieb (keine Zusatzfrage vor der Antwort auf B).
#[tokio::test]
async fn ist_b_unbeantwortet_bleibt_die_frage_weg() {
    for scheibe in ["gesamt", "rentner_gesamt", "an_gesamt"] {
        assert!(
            !steht(scheibe, &zusammen(Some("keine"), None)).await,
            "{scheibe}: A keine, B offen: die Frage steht"
        );
    }
}

/// Die Falle "Bedingungsfeld selbst versteckt": bei Einzelveranlagung fragt die Software den Ehegatten nie. Eine alte Antwort auf
/// `kist_konfession_partner` (der Nutzer wechselte von "zusammen" zu "einzel") darf die Frage nicht oeffnen: sie gilt dort nicht.
#[tokio::test]
async fn bei_einzelveranlagung_zaehlt_eine_alte_antwort_des_ehegatten_nicht() {
    let einzel = [
        ("veranlagung", json!("einzel")),
        (KONFESSION_A, json!("keine")),
        (KONFESSION_B, json!("evangelisch")),
    ];
    assert!(
        !steht("gesamt", &einzel).await,
        "A keine, B evangelisch, aber Einzelveranlagung: die Frage steht"
    );
}
