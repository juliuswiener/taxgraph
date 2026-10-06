//! GATE fuer die Naht `GET /fall/{id}/stand`: der Endpunkt rechnet (`stand::stand`), er antwortet
//! nicht mehr `501 nicht_portiert`. Er war die Probe vor dem Port und blieb als Gestalt-Test.
//!
//! Die Zahlen prueft der Differenz-Harness (`rust/parity/tests/api_http_paritaet.rs`) gegen Python;
//! dieser Test haelt die Gestalt aus `api.stand` (`api.py:490`) ohne Python fest.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;

use api::konfig::Konfig;
use api::{app, ApiFehler, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use bescheid::deklaration::{Cfg, ScheibenFehler};
use domain::Scheibe;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

struct Dienst {
    zustand: Zustand,
    _tmp: tempfile::TempDir,
}

fn dienst() -> Dienst {
    dienst_in(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn dienst_in(wurzel: &Path) -> Dienst {
    let tmp = tempfile::tempdir().unwrap();
    let konfig = Konfig {
        wurzel: wurzel.to_path_buf(),
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

/// Der Fall der Sonde: Scheibe `ep` — sechs Felder, kein `guard`.
async fn fall_anlegen(d: &Dienst) -> String {
    let token = format!("Bearer {}", d.zustand.auth.stelle_aus("alice").unwrap());
    let rumpf = r#"{"fall_id": "sonde", "scheibe": "ep", "veranlagungszeitraum": 2025}"#;
    let (status, _, text) = sende(
        d,
        "POST",
        "/fall",
        &[
            ("authorization", &token),
            ("content-type", "application/json"),
            ("content-length", &rumpf.len().to_string()),
        ],
        Some(rumpf),
    )
    .await;
    assert_eq!(status, 201, "POST /fall: {text}");
    token
}

/// Eine Akte ohne Events; `None` heisst: der Schluessel `scheibe` fehlt.
fn akte(scheibe: Option<&str>) -> store::Store {
    let mut d = serde_json::json!({"version": 1, "veranlagungszeitraum": 2025, "events": []});
    if let Some(s) = scheibe {
        d["scheibe"] = s.into();
    }
    store::Store::aus_datei(serde_json::from_value(d).unwrap())
}

/// SCHRITT 0c: `_cfg` und `_scheibe_bindung` (`api.py:171-195`) im `api`-Crate; `stand` ruft beide
/// zuerst (`api.py:454-455`). Die Zahlen sind `len(api._scheibe_bindung({"scheibe": s}))`, gemessen
/// 2026-10-02 (`gesamt` 350 -> 351, `rentner_gesamt` 247 -> 248 mit dem Zwilling `p34_abs3_antragsbetrag`,
/// gemessen 2026-10-03; beide +1 mit `stammdaten_hausnummerzusatz`: 352 und 249, gemessen 2026-10-03;
/// beide +2 mit den Satz-3-Feldern `kind_anderer_elternteil_tod_am` und `..._ausland_zeitraum`:
/// 354 und 251, C2 am 2026-10-06).
/// `n_vor_gwg` liest die YAML: 0 statt 69 waere der stille Rueckfall auf "leer".
#[test]
fn scheibe_bindung_wie_python() {
    let d = dienst();
    for (scheibe, n) in [
        ("ep", 6),
        ("n_vor_gwg", 69),
        ("an_gesamt", 84),
        ("gesamt", 354),
        ("rentner_gesamt", 251),
    ] {
        let sb = d.zustand.scheibe_bindung(&akte(Some(scheibe))).unwrap();
        assert_eq!(sb.cfg.scheibe().to_string(), scheibe);
        assert_eq!(sb.index.len(), n, "{scheibe}");
    }
}

/// `_cfg`: 400 mit Pythons `repr` der Scheibe (`api.py:174`).
#[test]
fn unbekannte_scheibe_ist_400_wie_python() {
    let d = dienst();
    for (scheibe, meldung) in [
        (Some("xyz"), "unbekannte Scheibe 'xyz'"),
        (Some("it's"), r#"unbekannte Scheibe "it's""#),
        (None, "unbekannte Scheibe None"),
    ] {
        match d.zustand.scheibe_bindung(&akte(scheibe)) {
            Err(ApiFehler::Status(400, m)) => assert_eq!(m, meldung),
            anders => panic!("{scheibe:?}: {anders:?}"),
        }
    }
}

/// `_scheibe_bindung`: 500 mit Pythons `repr` der Liste (`api.py:194`). Die echte Bindung ist
/// vollstaendig; die Abweisung selbst prueft der Doctest von `bescheid::deklaration::scheibe_bindung`.
#[test]
fn unvollstaendige_bindung_ist_500_wie_python() {
    let e = ApiFehler::from(ScheibenFehler::BindungUnvollstaendig(vec![
        "a".into(),
        "b'c".into(),
    ]));
    match e {
        ApiFehler::Status(500, m) => assert_eq!(
            m,
            r#"Bindungstabelle unvollständig für Scheibe: ['a', "b'c"]"#
        ),
        anders => panic!("{anders:?}"),
    }
}

/// Eine Wurzel, deren `rust/bindung/daten/` leer ist: `ep` findet keine seiner sechs Felder (500 mit
/// der Liste in Scheibenreihenfolge), `n_vor_gwg` findet seine Datei nicht — `FileNotFoundError`,
/// nie eine Scheibe mit null Feldern, die „vollstaendig" aussaehe.
#[test]
fn leere_bindung_ist_ein_fehler_nie_eine_leere_scheibe() {
    let wurzel = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(wurzel.path().join("rust/bindung/daten")).unwrap();
    let d = dienst_in(wurzel.path());

    let ep = Cfg::fuer(Scheibe::Ep).felder_roh().unwrap();
    let liste = ep.iter().map(|f| format!("'{f}'")).collect::<Vec<_>>();
    match d.zustand.scheibe_bindung(&akte(Some("ep"))) {
        Err(ApiFehler::Status(500, m)) => assert_eq!(
            m,
            format!(
                "Bindungstabelle unvollständig für Scheibe: [{}]",
                liste.join(", ")
            )
        ),
        anders => panic!("ep: {anders:?}"),
    }

    match d.zustand.scheibe_bindung(&akte(Some("n_vor_gwg"))) {
        Err(ApiFehler::Unerwartet { typ, meldung }) => {
            assert_eq!(typ, "FileNotFoundError");
            assert!(meldung.contains("bindung_n_vor_gwg.yaml"), "{meldung}");
        }
        anders => panic!("n_vor_gwg: {anders:?}"),
    }
}

/// GRUENE KONTROLLZEILE: der Fall existiert, die Route erreicht ihn. Ohne sie fiele der rote
/// Test unten auch dann, wenn schon `POST /fall` kaputt waere — und „irgendwas geht nicht"
/// ist kein Befund.
#[tokio::test]
async fn kontrolle_der_fall_wird_erreicht() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let (status, json, text) = sende(
        &d,
        "GET",
        "/fall/sonde/stand",
        &[("authorization", &token)],
        None,
    )
    .await;
    // 200 beweist, dass die Route den Fall erreicht; 404/401/403 taeten es nicht.
    assert_eq!(
        status, 200,
        "stand erreicht den Fall nicht: {status} {text}"
    );
    assert!(json.is_object(), "{text}");
}

/// DIE NAHT: `stand` rechnet, statt 501 zu liefern (vorher `#[ignore]`, rot mit `--ignored`).
///
/// Geprueft wird die Gestalt aus `api.stand` (`api.py:489`), nicht ein Zahlwert — ein Zahlwert
/// haengt am Korpus. `engine` ist einer von vier Werten; der leere Fall hat einen Ring, also
/// ist `catala` die erwartete ehrliche Antwort (kein Fake-Gruen).
#[tokio::test]
async fn stand_rechnet_statt_501() {
    let d = dienst();
    let token = fall_anlegen(&d).await;
    let (status, json, text) = sende(
        &d,
        "GET",
        "/fall/sonde/stand",
        &[("authorization", &token)],
        None,
    )
    .await;
    assert_eq!(status, 200, "stand lieferte {status}: {text}");
    for schluessel in [
        "fall_id",
        "snapshot_id",
        "engine",
        "felder",
        "relevanz",
        "teil_ringe",
        "ring_gesperrt",
        "ring_gesperrt_klartext",
    ] {
        assert!(
            json.get(schluessel).is_some(),
            "Schluessel {schluessel} fehlt in {text}"
        );
    }
    assert_eq!(json["fall_id"], "sonde");
    assert!(
        ["unavailable", "gesperrt", "catala", "catala_teilweise"]
            .contains(&json["engine"].as_str().unwrap_or("")),
        "unerwartete engine: {}",
        json["engine"]
    );
}
