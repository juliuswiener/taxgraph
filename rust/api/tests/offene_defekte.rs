//! Offene Defekte der HTTP-Naht: je Python-`xfail(strict=True)`-Fall ein Gegenstueck, das das
//! RICHTIGE Verhalten verlangt. Aufbau wie `stand_naht.rs`: ein Dienst im Wegwerf-Verzeichnis,
//! Requests durch `app()`.
//!
//! Die Faelle hier brauchen `GET /fall/{id}/deklaration`, `POST /fall/{id}/einreichen` und
//! `POST /fall/{id}/kontoauszug`. Alle drei sind portiert (`api/src/deklaration.rs`,
//! `api/src/einreichen.rs`, `api/src/kontoauszug.rs`). Ein Test ist `#[ignore]`, solange der
//! Handler den Defekt von Python teilt (`einreichen` liest `pflichtfelder_luecken` nicht). Das ist
//! der Defekt, den der Python-Test pinnt; der Ignore-Text nennt ihn, damit das Rot nicht als „schon
//! erledigt" gelesen wird. Die Tests von `deklaration` und `kontoauszug` laufen ohne `#[ignore]`.
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
    // Seit der Portierung 200; ein 501 oder 404/401/403 erreichte den Fall nicht.
    assert_eq!(
        status, 200,
        "deklaration erreicht den Fall nicht: {status} {text}"
    );
    assert!(json["deklaration"].is_object(), "{text}");
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
    // Der Fall ist ohne `bruttoarbeitslohn`: das Abgabe-Gate des XML-Writers nennt die Luecke
    // (422 `xml_nicht_baubar`), noch vor jeder Frage an ERiC.
    assert_eq!(
        (status, json["grund"].as_str()),
        (422, Some("xml_nicht_baubar")),
        "einreichen erreicht den Fall nicht: {status} {text}"
    );
    assert!(
        json["detail"].as_str().is_some_and(|d| d.contains("bruttoarbeitslohn")),
        "{text}"
    );
}

/// `test_pflichtfelder_luecken_ohne_leser.py::test_abgabegate_nennt_die_pflichtfeldluecke_selbst`:
/// das Abgabe-Gate muss `pflichtfelder_luecken` befragen.
///
/// Der Fall ohne `bruttoarbeitslohn` bleibt `eingaben_konsistent` (die vorhandenen Angaben sind
/// stimmig); `pflichtfelder_luecken` kennt die Luecke, `unvollstaendig` nicht. Weil `einreichen`
/// in Python nur `eingaben_konsistent` liest (`api.py:731`), laeuft der Fall bis `ERiC` durch und
/// der Nutzer bekommt eine Fremdmeldung statt unseres Feldnamens.
#[tokio::test]
#[ignore = "POST /einreichen (api/src/einreichen.rs) bildet Python ab und liest pflichtfelder_luecken nicht (elster::Deklaration hat den Accessor, kein Handler ruft ihn): heute 422 'xml_nicht_baubar' aus dem Abgabe-Gate des Writers, wie in Python. Erwartet 409 'deklaration_unvollstaendig' mit bruttoarbeitslohn. Python: test_pflichtfelder_luecken_ohne_leser.py::test_abgabegate_nennt_die_pflichtfeldluecke_selbst. Vault: tickets/zwei-vollstaendigkeitsbegriffe-einer-davon-gelesen.md. Rot sehen: --ignored"]
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

/// `test_deklaration_umgeht_waechter_gepinnt.py::test_deklaration_sperrt_bei_kapital_semantik_offen`:
/// `/deklaration` fragt den Sperrgrund wie `/ergebnis` und `/einreichen` und sperrt mit 409.
///
/// Derselbe Fall: Aggregat (E1900701) UND Aktien-Topf (E1900901) gleichzeitig bestaetigt. Die
/// Deklaration darf ihn nicht als vollstaendig melden und beide Kz zeigen; sie antwortet 409 mit
/// `grund` und dem `klartext`, den `/ergebnis` fuer denselben Grund zeigt
/// (`decisions/deklaration-darf-verweigern.md`, entschieden 2026-10-03).
///
/// Kein `#[ignore]` mehr: der Aufruf von `an_gesamt_sperrgrund` in `api/src/deklaration.rs` ist die
/// Sperre, und dieser Test wird rot, sobald er fehlt.
#[tokio::test]
async fn deklaration_umgeht_den_waechter_nicht() {
    let d = dienst();
    let token = fall_anlegen(&d, "waechter", "gesamt").await;
    setze_felder(&d, "waechter", &kapital_semantik_offen());
    let kopf = [("authorization", token.as_str())];
    let (status, json, text) = sende(&d, "GET", "/fall/waechter/deklaration", &kopf, None).await;
    assert_eq!(status, 409, "erwartet 409, erhalten {status}: {text}");
    assert_eq!(json["grund"], "kapital_semantik_offen", "{text}");
    // Der Koerper ist genau `fall_id`, `grund`, `klartext` — nichts von der Deklaration steht darin.
    let mut schluessel: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    schluessel.sort_unstable();
    assert_eq!(schluessel, ["fall_id", "grund", "klartext"], "{text}");
    // Der Satz ist der von `/ergebnis` fuer denselben Grund, kein eigener Text.
    let (st_erg, erg, text_erg) = sende(&d, "GET", "/fall/waechter/ergebnis", &kopf, None).await;
    assert_eq!(st_erg, 200, "{text_erg}");
    assert_eq!(erg["grund"], "kapital_semantik_offen", "{text_erg}");
    assert!(
        erg["klartext"].as_str().is_some_and(|k| !k.is_empty()),
        "/ergebnis liefert keinen Klartext: {text_erg}"
    );
    assert_eq!(json["klartext"], erg["klartext"], "{text} gegen {text_erg}");
}

/// CSV mit einer lesbaren Ruerup-Zeile und zwei unlesbaren Betraegen (`abc`, `1,2,3`) — dieselben
/// Zeilen wie `test_kontoauszug_csv_unlesbarer_betrag.py`.
const KONTOAUSZUG_CSV: &str = "datum;betrag;verwendungszweck\n\
    15.03.2025;-1200,00;Ruerup-Rente Jahresbeitrag Basisrente\n\
    16.03.2025;abc;Ruerup-Rente Nachzahlung Basisrente\n\
    17.03.2025;1,2,3;Ruerup-Rente Sonderzahlung Basisrente\n";

async fn kontoauszug_hochladen(d: &Dienst, fall_id: &str) -> (u16, Value, String) {
    let token = fall_anlegen(d, fall_id, "an_gesamt").await;
    let rumpf = json!({"format": "csv", "inhalt": KONTOAUSZUG_CSV}).to_string();
    sende(
        d,
        "POST",
        &format!("/fall/{fall_id}/kontoauszug"),
        &[
            ("authorization", &token),
            ("content-type", "application/json"),
            ("content-length", &rumpf.len().to_string()),
        ],
        Some(&rumpf),
    )
    .await
}

/// GRUENE KONTROLLZEILE fuer den Kontoauszug-Test: die Route erreicht den Fall.
#[tokio::test]
async fn kontrolle_der_kontoauszug_erreicht_den_fall() {
    let d = dienst();
    let (status, json, text) = kontoauszug_hochladen(&d, "konto").await;
    assert_eq!(
        status, 200,
        "kontoauszug erreicht den Fall nicht: {status} {text}"
    );
    assert!(json["uebernommen"].is_number(), "{text}");
}

/// `test_kontoauszug_csv_unlesbarer_betrag.py::test_unlesbarer_betrag_steht_in_verworfen_mit_grund`:
/// eine CSV-Zeile mit unlesbarem Betrag muss in `verworfen` zaehlen und im `hinweis` stehen.
///
/// `eingang::kontoauszug::parse_csv` liefert die Zahl der verworfenen Zeilen als zweiten Wert
/// (Regel getestet in `eingang` `tests::csv_unlesbarer_betrag_zaehlt_in_verworfen`); die Route
/// muss sie nach `verworfen` und in den `hinweis` reichen. Den Wortlaut des Grundes legt der Test
/// nicht fest, nur dass er den Betrag nennt.
///
/// Kein `#[ignore]` mehr: die Route ist portiert (`api/src/kontoauszug.rs`), und dieser Test wird rot,
/// sobald die Zahl aus `parse_csv` nicht mehr nach `verworfen` oder in den `hinweis` gelangt.
#[tokio::test]
async fn kontoauszug_unlesbarer_betrag_steht_in_verworfen() {
    let d = dienst();
    let (status, json, text) = kontoauszug_hochladen(&d, "unlesbar").await;
    assert_eq!(status, 200, "erwartet 200, erhalten {status}: {text}");
    assert_eq!(json["uebernommen"], 1, "{json}");
    assert_eq!(
        json["verworfen"], 2,
        "abc und 1,2,3 fehlen in verworfen: {json}"
    );
    assert!(
        json["hinweis"]
            .as_str()
            .is_some_and(|h| h.contains("Betrag")),
        "der Hinweis nennt den Betrag nicht als Grund: {json}"
    );
}
