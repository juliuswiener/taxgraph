//! `POST /fall/{id}/event` ohne Python: Tür, Auflagen und die Kennung eines Events, an einem Fall der
//! Scheibe `ep`. Die Wortlaute und die Kennungen stammen aus einem Lauf von `api.event` /
//! `store.append_event` (`CPython` 3.14); dass beide Server in 243 weiteren Formen gleich antworten,
//! prüft der Differenz-Harness (`rust/parity/tests/api_http_paritaet.rs`, `generatoren`). Dieser Test
//! hält die Gestalt fest und fällt auch dort, wo kein Python läuft.
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

async fn sende(d: &Dienst, pfad: &str, token: &str, body: &str) -> (u16, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(pfad)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("content-length", body.len().to_string())
        .body(Body::from(body.to_owned()))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (
        teile.status.as_u16(),
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Legt Fall `id` (Scheibe `ep`, Nutzer alice) an und liefert das Token.
async fn fall_anlegen(d: &Dienst, id: &str) -> String {
    let token = d.zustand.auth.stelle_aus("alice").unwrap();
    let rumpf = json!({"fall_id": id, "scheibe": "ep", "veranlagungszeitraum": 2025}).to_string();
    let (status, antwort) = sende(d, "/fall", &token, &rumpf).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    token
}

/// Wie viele Events die Akte des Falls trägt.
fn events_in_akte(d: &Dienst, id: &str) -> usize {
    let pfad = d.zustand.konfig.faelle.join(format!("{id}.json"));
    let akte: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    akte["events"].as_array().unwrap().len()
}

/// Ein Event als Rohtext, jeder Teil frei wählbar; `ts` ist fest.
fn rumpf(feld: &str, wert: &str, zustand: &str, schreiber: &str, signal: &str) -> String {
    format!(
        r#"{{"feld_id": {feld}, "wert": {wert}, "zustand": "{zustand}", "schreiber": "{schreiber}",
        "herkunft": {{"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}},
        "signal": {signal}, "ts": "2026-01-01T00:00:00+00:00"}}"#
    )
}

const ARBEITSTAGE: &str = r#""ep_arbeitstage""#;
const KLICK: &str = r#"{"signal_1": null, "signal_2": "klick"}"#;

#[tokio::test]
async fn ein_event_wird_angenommen_und_ein_zweites_auf_dasselbe_feld_abgewiesen() {
    let d = dienst();
    let token = fall_anlegen(&d, "e1").await;
    let b = rumpf(ARBEITSTAGE, "220", "bestaetigt", "ui:naht", KLICK);
    let (status, antwort) = sende(&d, "/fall/e1/event", &token, &b).await;
    assert_eq!(status, 201, "{antwort}");
    assert_eq!(antwort["feld_id"], "ep_arbeitstage");
    assert_eq!(antwort["zustand"], "bestaetigt");
    assert_eq!(antwort["event_id"].as_str().unwrap().len(), 64);
    assert_eq!(events_in_akte(&d, "e1"), 1);
    let (status, antwort) = sende(&d, "/fall/e1/event", &token, &b).await;
    assert_eq!(status, 422);
    assert!(
        antwort["fehler"]
            .as_str()
            .unwrap()
            .starts_with("fail-closed (B): ep_arbeitstage hat schon ein aktives Event"),
        "{antwort}"
    );
    assert_eq!(events_in_akte(&d, "e1"), 1, "eine Abweisung legt nichts ab");
}

/// Die Kennung hängt an jedem Zeichen der kanonischen Schreibweise, auch an der einer Kommazahl:
/// Python schreibt `1e+22` und `1e-07`, `serde_json` `1e22` und `1e-7`. Die Kennungen sind die, die
/// `store.append_event` zu denselben Eingaben liefert.
#[tokio::test]
async fn die_kennung_gleicht_der_von_python() {
    let d = dienst();
    let token = fall_anlegen(&d, "e2").await;
    let signal = r#"{"signal_1": {"z": 1.0, "a": [1e22, 1e-7, 0.1, -0.0, 5e-324, 123456789012345680.0],
        "u": "ä\u0001"}, "signal_2": null}"#;
    let b = rumpf(ARBEITSTAGE, "220", "vorlaeufig", "ui:naht", signal);
    let (status, antwort) = sende(&d, "/fall/e2/event", &token, &b).await;
    assert_eq!(status, 201, "{antwort}");
    assert_eq!(
        antwort["event_id"],
        "f057059d9b5c2d06a6a444f48b60cb0dca7f9911e80217fbcf924793b00c249a"
    );
    // `vorlaeufig` mit `signal_2` ist eine Form, die `NeuesEvent` nicht kennt; Python legt sie ab.
    fall_anlegen(&d, "e3").await;
    let b = rumpf(
        ARBEITSTAGE,
        "220",
        "vorlaeufig",
        "ui:naht",
        r#"{"signal_2": "x", "signal_1": null}"#,
    );
    let (status, antwort) = sende(&d, "/fall/e3/event", &token, &b).await;
    assert_eq!(status, 201, "{antwort}");
    assert_eq!(
        antwort["event_id"],
        "b7be2a125d647323b38e775a6fe4a82fb1de93da27f51d6c2dc62b1fc108e875"
    );
    // Ein `signal` ohne `signal_2`: Python legt es ohne den Schluessel ab, und die Kennung hängt daran.
    for (id, signal, soll) in [
        (
            "e5",
            r#"{"signal_1": 5}"#,
            "1adeb73218c20062c18865e5e086f94c40c3434d104fab67517d816c9fa4cafb",
        ),
        (
            "e6",
            r#"{"signal_1": null}"#,
            "b984dec6ddfa01b6e6ef940f5a0f7a88a7644cc734a1b611607e6c3431e5385e",
        ),
        (
            "e7",
            r#"{"signal_2": "x"}"#,
            "7a364b6528cacb203a1cc8238d1bdf14e3de60b0f90975ebbabe05129dd97493",
        ),
    ] {
        fall_anlegen(&d, id).await;
        let b = rumpf(ARBEITSTAGE, "220", "vorlaeufig", "ui:naht", signal);
        let (status, antwort) = sende(&d, &format!("/fall/{id}/event"), &token, &b).await;
        assert_eq!(status, 201, "{signal}: {antwort}");
        assert_eq!(antwort["event_id"], soll, "{signal}");
    }
}

/// Instanz 1 ist die Basis ohne Suffix, `x__1` ist keine Instanz (Entscheidung 2026-10-03, Zähler
/// `[2-9]|[1-9][0-9]+`): die Route weist es mit 400 ab, und `x`, `x__2`, `x__10` gehen durch. Die
/// Basis `vv_einnahmen` hat eine `instanz_gruppe` und liegt in der Scheibe `gesamt` -- am Fall `ep`
/// käme die 400 auch ohne die Regel (kein Feld dort trägt eine Gruppe). Wortlaut aus `api.event`.
#[tokio::test]
async fn instanz_eins_wird_abgewiesen_und_der_rest_geht_durch() {
    let d = dienst();
    let token = d.zustand.auth.stelle_aus("alice").unwrap();
    let anlegen =
        json!({"fall_id": "e5", "scheibe": "gesamt", "veranlagungszeitraum": 2025}).to_string();
    let (status, antwort) = sende(&d, "/fall", &token, &anlegen).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let ok = |feld: &str| {
        rumpf(
            &format!("\"{feld}\""),
            "1500000",
            "bestaetigt",
            "ui:naht",
            &format!(r#"{{"signal_1": null, "signal_2": "klick@{feld}"}}"#),
        )
    };
    for feld in ["vv_einnahmen", "vv_einnahmen__2", "vv_einnahmen__10"] {
        let (status, antwort) = sende(&d, "/fall/e5/event", &token, &ok(feld)).await;
        assert_eq!(status, 201, "{feld}: {antwort}");
    }
    assert_eq!(events_in_akte(&d, "e5"), 3);
    for feld in ["vv_einnahmen__1", "vv_einnahmen__0", "vv_einnahmen__02"] {
        let (status, antwort) = sende(&d, "/fall/e5/event", &token, &ok(feld)).await;
        assert_eq!(status, 400, "{feld}: {antwort}");
        assert_eq!(
            antwort["fehler"],
            format!("feld_id '{feld}' nicht in dieser Scheibe")
        );
    }
    assert_eq!(events_in_akte(&d, "e5"), 3, "eine Abweisung legt nichts ab");
}

/// Status und Wortlaut je Abweisung, wie `api.event` und `store.append_event` sie liefern.
#[tokio::test]
async fn abweisungen_tragen_den_wortlaut_von_python() {
    let d = dienst();
    let token = fall_anlegen(&d, "e4").await;
    let ok = |feld: &str, wert: &str| rumpf(feld, wert, "bestaetigt", "ui:naht", KLICK);
    let llm = rumpf(
        ARBEITSTAGE,
        "220",
        "bestaetigt",
        "llm:t",
        r#"{"signal_2": "x"}"#,
    );
    let beleg = rumpf(ARBEITSTAGE, "220", "vorlaeufig", "import:beleg2", "null");
    let ts_zahl = ok(ARBEITSTAGE, "220").replace(r#""2026-01-01T00:00:00+00:00""#, "5");
    let ersetzt = ok(ARBEITSTAGE, "1").replace(r#""ts""#, r#""ersetzt": "nope", "ts""#);
    let faelle: [(&str, String, u16, &str); 15] = [
        (
            "feld nicht in der Scheibe",
            ok(r#""gibt_es_nicht""#, "1"),
            400,
            "feld_id 'gibt_es_nicht' nicht in dieser Scheibe",
        ),
        (
            "zustand",
            rumpf(ARBEITSTAGE, "1", "foo", "ui:naht", "null"),
            400,
            // Python druckt die Menge je nach `PYTHONHASHSEED` anders; Rust hat eine feste Reihenfolge.
            "zustand muss {'bestaetigt', 'vorlaeufig'} sein",
        ),
        (
            "schreiber leer",
            rumpf(ARBEITSTAGE, "1", "vorlaeufig", "", "null"),
            400,
            "schreiber ist Pflicht",
        ),
        (
            "ts keine Zeichenkette",
            ts_zahl,
            422,
            "fail-closed (Form): ts muss Text sein oder fehlen, nicht int.",
        ),
        (
            "Vorzeichen",
            ok(r#""ep_oepnv_kosten""#, "-1"),
            422,
            "fail-closed (Vorzeichen): ep_oepnv_kosten=-1 darf nicht negativ sein — das Feld kennt im \
             amtlichen ELSTER-Schema kein Minus, die Erklärung würde dort abgelehnt.",
        ),
        (
            "Bereich",
            ok(ARBEITSTAGE, "367"),
            422,
            "fail-closed (Bereich): ep_arbeitstage=367 liegt ausserhalb des erlaubten Bereichs 0 bis \
             366 der Bindung.",
        ),
        (
            "Typ",
            ok(ARBEITSTAGE, "1.5"),
            422,
            // Abweichung Nr. 30: der Wert fehlt im Text (Python nennt ihn: `ep_arbeitstage=1.5`).
            "fail-closed (Typ): ep_arbeitstage passt nicht zum Bindungstyp 'int' — der Ring läse \
             das sonst still als 0 (Stille-Null-Klasse).",
        ),
        (
            // Auflage Z: das Zeichen und ein Vorschlag, nie der Wert (Wortlaut aus `store.append_event`).
            "Zeichensatz",
            ok(r#""ep_ziel_adresse""#, r#""Wałesa""#),
            422,
            "fail-closed (Zeichensatz): ep_ziel_adresse enthält das Zeichen „ł\" (U+0142), das \
             ELSTER in Textfeldern nicht annimmt — schreibe stattdessen „l\".",
        ),
        (
            "llm bestaetigt",
            llm,
            422,
            "fail-closed (A): llm:-Schreiber muss herkunft=llm_vorschlag, zustand=vorlaeufig, \
             signal_2=null tragen — kein Bestätigen durch die KI.",
        ),
        (
            "Beleg nach Praefix",
            beleg,
            422,
            "fail-closed (A): import:beleg-Schreiber muss herkunft=beleg_import, \
             zustand=vorlaeufig, signal_2=null tragen — ein Beleg-Import bestätigt nie direkt.",
        ),
        (
            "bestaetigt ohne signal_2",
            rumpf(ARBEITSTAGE, "1", "bestaetigt", "ui:naht", "null"),
            422,
            "fail-closed: zustand=bestaetigt braucht ein signal_2 (Zwei-Signal).",
        ),
        (
            "signal_2 Zahl",
            rumpf(
                ARBEITSTAGE,
                "1",
                "vorlaeufig",
                "ui:naht",
                r#"{"signal_2": 5}"#,
            ),
            422,
            "fail-closed: signal_2 muss Text oder null sein, nicht int.",
        ),
        (
            "ersetzt unbekannt",
            ersetzt,
            422,
            "fail-closed (B): ersetzt-Ziel nope existiert nicht.",
        ),
        (
            "Rumpf ist eine Liste",
            "[1]".to_owned(),
            500,
            "AttributeError: 'list' object has no attribute 'get'",
        ),
        (
            "feld_id als Liste",
            ok("[1]", "1"),
            500,
            "TypeError: cannot use 'list' as a dict key (unhashable type: 'list')",
        ),
    ];
    for (name, body, status_soll, text_soll) in faelle {
        let (status, antwort) = sende(&d, "/fall/e4/event", &token, &body).await;
        assert_eq!(status, status_soll, "{name}: {antwort}");
        assert_eq!(antwort["fehler"], text_soll, "{name}");
        assert_eq!(events_in_akte(&d, "e4"), 0, "{name}: die Akte bleibt leer");
    }
}

/// Die gemeinsame Tabelle der Begleitfelder (`rust/fixtures/begleitfelder_formen.json`, Python:
/// `tests/test_begleitfelder_form.py`) gegen die Rust-Route, ohne Python (Vault Backlog
/// `python-schreibt-akte-die-der-rust-leser-sperrt`, AK1 bis AK3). Je Fall `ts`, `herkunft`, `signal`:
/// - `python: angenommen` -> 201, und die Akte besteht den Rundlauf: `store::lade` liest sie, der
///   `event_id` des gelesenen Events stimmt (nichts ging verloren), und `speichere` schreibt dieselben
///   Bytes zurück.
/// - `python: abgewiesen` -> 422 mit `fail-closed (Form)` und eine leere Akte; 400 schon an der Tür
///   (`api.event`), wenn `herkunft` kein Objekt mit dem Schlüssel `herkunft` ist.
#[tokio::test]
async fn begleitfelder_tabelle_bedient_die_route_und_besteht_den_rundlauf() {
    let d = dienst();
    let tabelle: Value =
        serde_json::from_str(include_str!("../../fixtures/begleitfelder_formen.json")).unwrap();
    let faelle = tabelle["faelle"].as_array().unwrap();
    assert!(faelle.len() >= 30, "die Tabelle ist kürzer als erwartet");
    let (mut angenommen, mut abgewiesen) = (0, 0);
    for (i, f) in faelle.iter().enumerate() {
        let (id, name) = (format!("bf{i}"), f["name"].as_str().unwrap());
        let token = fall_anlegen(&d, &id).await;
        let mut body = json!({"feld_id": "ep_arbeitstage", "wert": 1, "zustand": "vorlaeufig",
            "schreiber": "ui:laie"});
        for k in ["ts", "herkunft", "signal"] {
            if let Some(v) = f.get(k) {
                body[k] = v.clone();
            }
        }
        let (status, antwort) =
            sende(&d, &format!("/fall/{id}/event"), &token, &body.to_string()).await;
        if f["python"] == "angenommen" {
            angenommen += 1;
            assert_eq!(status, 201, "{name}: {antwort}");
            rundlauf(&d, &id, name);
        } else {
            abgewiesen += 1;
            let tuer = !f["herkunft"]
                .as_object()
                .is_some_and(|h| h.contains_key("herkunft"));
            assert_eq!(status, if tuer { 400 } else { 422 }, "{name}: {antwort}");
            if !tuer {
                let text = antwort["fehler"].as_str().unwrap();
                assert!(
                    text.starts_with("fail-closed (Form): "),
                    "{name}: {antwort}"
                );
            }
            assert_eq!(
                events_in_akte(&d, &id),
                0,
                "{name}: eine Abweisung legt nichts ab"
            );
        }
    }
    assert!(
        angenommen >= 8 && abgewiesen >= 20,
        "{angenommen} / {abgewiesen}"
    );
}

/// Die Akte des Falls `id` mit einem Event: `store::lade` liest sie, der `event_id` des Events stimmt
/// noch (kein Schlüssel ging beim Lesen verloren), und `speichere` schreibt dieselben Bytes zurück.
fn rundlauf(d: &Dienst, id: &str, name: &str) {
    let pfad = d.zustand.konfig.faelle.join(format!("{id}.json"));
    let datei =
        store::lade(&pfad).unwrap_or_else(|e| panic!("{name}: Rust liest die Akte nicht: {e}"));
    assert_eq!(datei.events.len(), 1, "{name}");
    let event = &datei.events[0];
    assert_eq!(
        event.berechne_event_id().as_ref(),
        Ok(&event.event_id),
        "{name}"
    );
    let kopie = d.zustand.konfig.faelle.join(format!("{id}.kopie"));
    store::speichere(&kopie, &datei).unwrap();
    assert_eq!(
        std::fs::read(&kopie).unwrap(),
        std::fs::read(&pfad).unwrap(),
        "{name}"
    );
}
