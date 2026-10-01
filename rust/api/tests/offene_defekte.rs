//! Offene Defekte der HTTP-Naht: je Python-`xfail(strict=True)`-Fall ein Gegenstueck, das das
//! RICHTIGE Verhalten verlangt. Aufbau wie `stand_naht.rs`: ein Dienst im Wegwerf-Verzeichnis,
//! Requests durch `app()`.
//!
//! Die Faelle hier brauchen `GET /fall/{id}/deklaration` und `POST /fall/{id}/einreichen` — beide
//! sind heute 501-Stubs (`routen/lesen.rs:65`, `routen/schreiben.rs:25`). Die Tests sind deshalb
//! `#[ignore]` und werden aus ZWEI Gruenden rot, in dieser Reihenfolge:
//!
//! 1. heute: der Handler antwortet 501 statt 200/409 — die Route ist nicht portiert;
//! 2. nach der Portierung: der Handler liest `pflichtfelder_luecken` bzw. den Sperrgrund nicht.
//!
//! Grund 2 ist der Defekt, den der Python-Test pinnt; Grund 1 ist die fehlende Naht davor. Beide
//! stehen im Ignore-Text, damit das Rot nach der Portierung nicht als „schon erledigt" gelesen wird.
//!
//! Rot sehen:  `cargo test -p api --test offene_defekte -- --ignored`
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
    kopf: &[(&str, &str)],
    body: Option<&str>,
) -> (u16, Value, String) {
    let mut b = Request::builder().method(methode).uri(pfad);
    for (k, v) in kopf {
        b = b.header(*k, *v);
    }
    let req = b
        .body(body.map_or_else(Body::empty, |t| Body::from(t.to_owned())))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let json = serde_json::from_str(&text).unwrap_or(Value::Null);
    (teile.status.as_u16(), json, text)
}

/// `POST /fall` mit Token; liefert das Token zurueck.
async fn fall_anlegen(d: &Dienst, fall_id: &str, scheibe: &str) -> String {
    let token = format!("Bearer {}", d.zustand.auth.stelle_aus("alice").unwrap());
    let rumpf = format!(
        r#"{{"fall_id": "{fall_id}", "scheibe": "{scheibe}", "veranlagungszeitraum": 2025}}"#
    );
    let (status, _, text) = sende(
        d,
        "POST",
        "/fall",
        &[
            ("authorization", &token),
            ("content-type", "application/json"),
            ("content-length", &rumpf.len().to_string()),
        ],
        Some(&rumpf),
    )
    .await;
    assert_eq!(status, 201, "POST /fall: {text}");
    token
}

/// Die Ereignisse eines Falls direkt in die Store-Datei schreiben — `POST /event` ist 501-Stub,
/// der Fall muss aber beantwortet sein, bevor `/deklaration` etwas messen kann. Derselbe Weg wie
/// `bescheid::testhilfe::store`, nur auf der Platte statt im Speicher.
fn setze_felder(d: &Dienst, fall_id: &str, paare: &[(&str, Value)]) {
    let pfad = d.zustand.konfig.faelle.join(format!("{fall_id}.json"));
    let mut datei = store::lade(&pfad).expect("Fall-Datei lesbar");
    for (i, (fid, wert)) in paare.iter().enumerate() {
        let mut e = json!({
            "ts": format!("2026-01-01T00:00:{i:02}+00:00"), "feld_id": fid, "wert": wert,
            "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": null, "signal_2": "ok"},
        });
        e["event_id"] = json!(store::EventId::von_json(&e).to_string());
        datei.events.push(serde_json::from_value(e).expect("Event"));
    }
    store::speichere(&pfad, &datei).expect("Fall-Datei schreibbar");
}

/// `_VOLL` aus `test_pflichtfelder_luecken_ohne_leser.py`, ohne `bruttoarbeitslohn`.
fn ohne_bruttoarbeitslohn() -> Vec<(&'static str, Value)> {
    vec![
        ("stammdaten_nachname", json!("Maier")),
        ("stammdaten_vorname", json!("Hans")),
        ("stammdaten_geburtsdatum", json!("05.05.1955")),
        ("stammdaten_strasse", json!("Musterstr.")),
        ("stammdaten_hausnummer", json!("55")),
        ("stammdaten_plz", json!("55555")),
        ("stammdaten_wohnort", json!("Musterort")),
        ("stammdaten_steuernummer", json!("9181081508155")),
        ("kist_konfession", json!("keine")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_art_est_erklaerung", json!(true)),
        ("veranlagung", json!("einzel")),
        ("steuerklasse", json!("1")),
        ("p36_lohnsteuer", json!(1_200_000)),
        ("vor_an_anteil_rv", json!(4_200_000)),
        ("vor_ag_anteil_rv", json!(1_200_000)),
    ]
}

/// Der Fall aus `test_deklaration_umgeht_waechter_gepinnt.py`: reiner Person-A-Kegel, Aggregat
/// (E1900701) UND Aktien-Topf (E1900901) gleichzeitig bestaetigt.
fn kapital_semantik_offen() -> Vec<(&'static str, Value)> {
    let mut paare = vec![
        ("stammdaten_nachname", json!("Meier")),
        ("stammdaten_vorname", json!("Klaus")),
        ("stammdaten_geburtsdatum", json!("01.01.1970")),
        ("stammdaten_strasse", json!("Teststr.")),
        ("stammdaten_hausnummer", json!("1")),
        ("stammdaten_plz", json!("10115")),
        ("stammdaten_wohnort", json!("Berlin")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_art_est_erklaerung", json!(true)),
        ("kist_konfession", json!("keine")),
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("vor_an_anteil_rv", json!(4_200_000)),
        ("vor_ag_anteil_rv", json!(1_200_000)),
        ("steuerklasse", json!("1")),
        ("p36_lohnsteuer", json!(1_200_000)),
        ("veranlagung", json!("einzel")),
        ("kein_vuv", json!(true)),
        ("kein_kap", json!(false)),
    ];
    paare.extend([
        ("kap_kapitalertraege", json!(500_000)),
        ("kap_gewinn_aktien", json!(300_000)),
    ]);
    paare
}

/// GRUENE KONTROLLZEILE: `GET /fall/{id}/deklaration` erreicht den Fall. Ohne sie fiele der rote
/// Test unten auch dann, wenn schon `POST /fall` kaputt waere — und „irgendwas geht nicht" ist
/// kein Befund.
#[tokio::test]
async fn kontrolle_die_deklaration_erreicht_den_fall() {
    let d = dienst();
    let token = fall_anlegen(&d, "sonde", "gesamt").await;
    setze_felder(&d, "sonde", &ohne_bruttoarbeitslohn());
    let (status, json, text) = sende(
        &d,
        "GET",
        "/fall/sonde/deklaration",
        &[("authorization", &token)],
        None,
    )
    .await;
    // 501 heute, 200 nach der Naht — beides beweist, dass die Route den Fall erreicht.
    // 404/401/403 taeten es nicht.
    assert!(
        status == 501 || status == 200,
        "deklaration erreicht den Fall nicht: {status} {text}"
    );
    if status == 501 {
        assert_eq!(json["fehler"], "nicht_portiert");
    }
}

/// GRUENE KONTROLLZEILE fuer den Abgabegate-Test: derselbe Fall, dieselbe Route.
#[tokio::test]
async fn kontrolle_das_einreichen_erreicht_den_fall() {
    let d = dienst();
    let token = fall_anlegen(&d, "sonde2", "gesamt").await;
    setze_felder(&d, "sonde2", &ohne_bruttoarbeitslohn());
    let (status, json, text) = sende(
        &d,
        "POST",
        "/fall/sonde2/einreichen",
        &[
            ("authorization", &token),
            ("content-type", "application/json"),
            ("content-length", "2"),
        ],
        Some("{}"),
    )
    .await;
    assert!(
        status == 501 || status == 200 || status == 409 || status == 422,
        "einreichen erreicht den Fall nicht: {status} {text}"
    );
    if status == 501 {
        assert_eq!(json["fehler"], "nicht_portiert");
    }
}

/// `test_pflichtfelder_luecken_ohne_leser.py::test_abgabegate_nennt_die_pflichtfeldluecke_selbst`:
/// das Abgabe-Gate muss `pflichtfelder_luecken` befragen.
///
/// Der Fall ohne `bruttoarbeitslohn` bleibt `eingaben_konsistent` (die vorhandenen Angaben sind
/// stimmig); `pflichtfelder_luecken` kennt die Luecke, `unvollstaendig` nicht. Weil `einreichen`
/// in Python nur `eingaben_konsistent` liest (`api.py:731`), laeuft der Fall bis `ERiC` durch und
/// der Nutzer bekommt eine Fremdmeldung statt unseres Feldnamens.
#[tokio::test]
#[ignore = "POST /einreichen ist 501-Stub (api/src/routen/schreiben.rs:25); nach der Portierung fehlt der Leser von pflichtfelder_luecken (elster::Deklaration hat den Accessor, kein Handler ruft ihn). Erwartet 409 'deklaration_unvollstaendig' mit bruttoarbeitslohn. Python: test_pflichtfelder_luecken_ohne_leser.py::test_abgabegate_nennt_die_pflichtfeldluecke_selbst. Vault: tickets/zwei-vollstaendigkeitsbegriffe-einer-davon-gelesen.md. Rot sehen: --ignored"]
async fn abgabegate_nennt_die_pflichtfeldluecke_selbst() {
    let d = dienst();
    let token = fall_anlegen(&d, "gate", "gesamt").await;
    setze_felder(&d, "gate", &ohne_bruttoarbeitslohn());
    let (status, json, text) = sende(
        &d,
        "POST",
        "/fall/gate/einreichen",
        &[
            ("authorization", &token),
            ("content-type", "application/json"),
            ("content-length", "2"),
        ],
        Some("{}"),
    )
    .await;
    assert_eq!(status, 409, "erwartet 409, erhalten {status}: {text}");
    assert_eq!(
        json["grund"], "deklaration_unvollstaendig",
        "der Grund muss die Pflichtfeldluecke nennen, nicht eine ERiC-Fremdmeldung: {json}"
    );
    let luecken: Vec<&str> = json["unvollstaendig"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e["feld_id"].as_str())
        .collect();
    assert_eq!(
        luecken,
        ["bruttoarbeitslohn"],
        "der Grund muss genau die Luecke nennen, die pflichtfelder_luecken kennt: {json}"
    );
}

/// `test_deklaration_umgeht_waechter_gepinnt.py::test_deklaration_erkennt_widerspruch_NICHT_bug_gepinnt`:
/// `/deklaration` fragt den Sperrgrund nie, `/ergebnis` und `/einreichen` fragen ihn.
///
/// Derselbe Fall: Aggregat (E1900701) UND Aktien-Topf (E1900901) gleichzeitig bestaetigt. Die
/// Deklaration darf ihn nicht als vollstaendig melden und beide Kz gleichzeitig zeigen.
///
/// Ob `/deklaration` sperren SOLL oder nur warnen soll, ist offen — `decisions/deklaration-darf-
/// verweigern.md` empfiehlt Sperren und hat es ausdruecklich nicht entschieden. Der Test verlangt
/// deshalb nur das, was in JEDER der beiden Optionen gilt: keine Antwort, die den Widerspruch
/// gleichzeitig als vollstaendig ausgibt.
#[tokio::test]
#[ignore = "GET /deklaration ist 501-Stub (api/src/routen/lesen.rs:65); nach der Portierung fehlt der Aufruf von an_gesamt_sperrgrund — anders als /ergebnis und /einreichen (api.py:580 bzw. 725). Offen, ob Sperren oder Warnen: decisions/deklaration-darf-verweigern.md. Python: test_deklaration_umgeht_waechter_gepinnt.py::test_deklaration_erkennt_widerspruch_NICHT_bug_gepinnt. Rot sehen: --ignored"]
async fn deklaration_umgeht_den_waechter_nicht() {
    let d = dienst();
    let token = fall_anlegen(&d, "waechter", "gesamt").await;
    setze_felder(&d, "waechter", &kapital_semantik_offen());
    let (status, json, text) = sende(
        &d,
        "GET",
        "/fall/waechter/deklaration",
        &[("authorization", &token)],
        None,
    )
    .await;
    // Gesperrt (409) oder warnend (200) — beides ist eine ehrliche Antwort. Entscheidend ist,
    // dass nicht BEIDES zugleich gemeldet wird.
    assert!(
        status == 200 || status == 409,
        "unerwartet {status}: {text}"
    );
    if status == 409 {
        return;
    }
    let aggregat = json["deklaration"].get("E1900701");
    let topf = json["deklaration"].get("E1900901");
    assert!(
        !(json["vollstaendig"] == Value::Bool(true)
            && aggregat.is_some_and(|w| w != &Value::Null)
            && topf.is_some_and(|w| w != &Value::Null)),
        "DEFEKT: Deklaration zeigt Aggregat({aggregat:?}) UND Topf({topf:?}) gleichzeitig bei \
         vollstaendig={}, ohne den Widerspruch zu nennen — derselbe Fall, den /ergebnis und \
         /einreichen sperren: {json}",
        json["vollstaendig"]
    );
}
