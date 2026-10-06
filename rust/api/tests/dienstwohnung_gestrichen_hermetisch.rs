//! Die Frage `dhf_keine_pflicht_dienstwohnung` wird nicht mehr gestellt (2026-10-06, Entscheidung Julius:
//! "erstmal streichen"; sie kommt mit dem Paket VZ 2026 zurueck, wenn ueberhaupt). Das Feld bleibt mit
//! `askable: false` in der Bindung (Muster `spenden_vermoegensstock`), damit eine schon gegebene Antwort
//! keine Akte sperrt. Abweichung von Python Nr. 21 in `rust/fixtures/README.md`: `make serve-python`
//! fragt sie weiter.
//!
//! Was die Streichung fuer den Nutzer aendert, steht fest (gemessen vor und nach der Streichung, gleiche
//! Eingaben, ohne `PARITY=1`):
//!
//! - `/fragen`: eine Frage weniger (zuvor 189, danach 188 in der Messung), im Inland und im Ausland.
//! - `/stand`, `relevanz` der Regel `p9_1_3_nr5_doppelte_haushaltsfuehrung`: `annahmen_offen` bleibt
//!   `["keine_verpflichtende_dienst_oder_werkswohnung"]`. Die Annahme steckt weiter in der Rechnung (der
//!   Scope kennt die Wohnungsart nicht), darum nennt `/stand` sie weiter.
//! - Die Scheibe rechnet im Inland unveraendert (Spanne 9.976,00 EUR), im Ausland weiter nicht
//!   (`ausland_dhf_nicht_ring_faehig`). Die Antwort hat nie ein Ergebnis bewegt: bool, `gate: false`,
//!   kein Kz, kein Slot.
//! - Ist-Zustand, nicht beauftragt: `POST /event` fuer das Feld antwortet weiter 201 und
//!   `GET /feld/{fid}/frage` weiter 200 (`fragetext_laie: null`). Die Oberflaeche ruft beides nicht mehr auf.
//!
//! Ein Fall, in dem der Nutzer die Frage VOR der Streichung beantwortet hat, bleibt vollstaendig: das
//! Ereignis steht weiter im Store, `/deklaration` nennt das Feld unter `nicht_deklariert`, und keine
//! Route aendert ihr Ergebnis. Ohne das Feld in der Bindung meldete `/deklaration` "Feld nicht in der
//! Bindungstabelle", `vollstaendig: false` (gemessen 2026-10-06, Variante "Feld gestrichen").
//!
//! Dieser Test haelt den Ist-Zustand fest. Kommt die Frage zurueck, wird er rot; wer sie zurueckbringt,
//! streicht den Eintrag Nr. 21 und schreibt diese Datei um.
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
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Zustand as EventZustand};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};
use tower::ServiceExt;

const FELD: &str = "dhf_keine_pflicht_dienstwohnung";
const REGEL: &str = "p9_1_3_nr5_doppelte_haushaltsfuehrung";
const ANNAHME: &str = "keine_verpflichtende_dienst_oder_werkswohnung";

struct Dienst {
    zustand: Zustand,
    token: String,
    tmp: tempfile::TempDir,
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
        tmp,
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

fn ereignis(feld_id: &str, wert: &Value) -> Value {
    json!({
        "feld_id": feld_id,
        "herkunft": {"haftung": "nutzer", "herkunft": "laie", "pruef_tiefe": "ungeprueft"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld_id}")},
        "wert": wert,
        "zustand": "bestaetigt",
    })
}

/// Die Eingaben des Falls `gesamt_kegel` aus `rust/fixtures/api_stand_fragen_orakel.json` (Lohn, Vorsorge,
/// Einkunftsarten: damit die Scheibe eine Spanne rechnet) und dazu alle zwoelf Antworten der doppelten
/// Haushaltsfuehrung, im Inland oder im Ausland.
fn eingaben(im_inland: bool) -> Vec<Value> {
    let datei =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/api_stand_fragen_orakel.json");
    let gm: Value = serde_json::from_slice(&std::fs::read(datei).unwrap()).unwrap();
    let mut alle = gm["faelle"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "gesamt_kegel")
        .unwrap()["events"]
        .as_array()
        .unwrap()
        .clone();
    assert!(alle
        .iter()
        .all(|e| !e["feld_id"].as_str().unwrap().starts_with("dhf_")));
    alle.extend(dhf_antworten(im_inland));
    alle
}

fn dhf_antworten(im_inland: bool) -> Vec<Value> {
    vec![
        ereignis("dhf_unterkunftskosten_monat", &json!(100_000)),
        ereignis("dhf_monate", &json!(12)),
        ereignis("dhf_im_inland", &json!(im_inland)),
        ereignis("dhf_beruflich_veranlasst", &json!(true)),
        ereignis("dhf_eigener_hausstand", &json!(true)),
        ereignis("dhf_finanzielle_beteiligung", &json!(true)),
        ereignis("dhf_beschaeftigungsort", &json!("Muenchen")),
        ereignis("dhf_grund", &json!("Versetzung")),
        ereignis("dhf_begruendet_am", &json!("01.03.2025")),
        ereignis("dhf_bestanden_bis", &json!("31.12.")),
        ereignis("dhf_hausstand_plz_ort", &json!("20095 Hamburg")),
        ereignis("dhf_hausstand_seit", &json!("01.01.2015")),
    ]
}

/// Legt einen Fall `sf` der Scheibe `gesamt` an und spielt die Eingaben durch die echte Route ein.
async fn fall_mit(d: &Dienst, im_inland: bool) {
    let kopf = json!({"fall_id": "sf", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for e in eingaben(im_inland) {
        let (status, antwort) = sende(d, "POST", "/fall/sf/event", Some(&e)).await;
        assert_eq!(status, 201, "POST /event {}: {antwort}", e["feld_id"]);
    }
}

fn fragen_ids(fragen: &Value) -> Vec<String> {
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["feld_id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn die_frage_wird_nicht_mehr_gestellt_und_das_ergebnis_bleibt() {
    for (im_inland, spanne_cent, gesperrt) in [
        (true, Some(997_600), Value::Null),
        (false, None, json!("ausland_dhf_nicht_ring_faehig")),
    ] {
        let d = dienst();
        fall_mit(&d, im_inland).await;
        let name = if im_inland { "Inland" } else { "Ausland" };

        // /fragen: keine Frage zum Feld.
        let (status, fragen) = sende(&d, "GET", "/fall/sf/fragen", None).await;
        assert_eq!(status, 200, "{name}: {fragen}");
        assert!(
            !fragen_ids(&fragen).iter().any(|f| f == FELD),
            "{name}: /fragen fragt {FELD}"
        );

        // /stand: das Feld ist unbeantwortet, die Annahme bleibt genannt, Spanne und Sperre sind wie vorher.
        let (status, stand) = sende(&d, "GET", "/fall/sf/stand", None).await;
        assert_eq!(status, 200, "{name}: {stand}");
        assert!(stand["felder"].get(FELD).is_none(), "{name}");
        assert_eq!(
            stand["relevanz"][REGEL]["annahmen_offen"],
            json!([ANNAHME]),
            "{name}"
        );
        assert_eq!(
            stand["relevanz"][REGEL]["status"],
            json!("relevant"),
            "{name}"
        );
        assert_eq!(stand["ring_gesperrt"], gesperrt, "{name}");
        match spanne_cent {
            Some(cent) => {
                assert_eq!(stand["intervall"]["min_cent"], json!(cent), "{name}");
                assert_eq!(stand["intervall"]["max_cent"], json!(cent), "{name}");
            }
            None => assert_eq!(stand["intervall"], Value::Null, "{name}"),
        }

        // Ist-Zustand der Einzelfrage und des Schreibwegs (siehe Kopf): keine Frage, aber kein Fehler.
        let (status, antwort) =
            sende(&d, "GET", &format!("/fall/sf/feld/{FELD}/frage"), None).await;
        assert_eq!(status, 200, "{name}: {antwort}");
        assert_eq!(antwort["frage"]["fragetext_laie"], Value::Null, "{name}");
        let (status, antwort) = sende(
            &d,
            "POST",
            "/fall/sf/event",
            Some(&ereignis(FELD, &json!(false))),
        )
        .await;
        assert_eq!(status, 201, "{name}: {antwort}");
        let (_, nachher) = sende(&d, "GET", "/fall/sf/stand", None).await;
        assert_eq!(
            nachher["intervall"], stand["intervall"],
            "{name}: Spanne nach der Antwort"
        );
        assert_eq!(nachher["ring_gesperrt"], stand["ring_gesperrt"], "{name}");
    }
}

/// `body` ohne die Werte, die je Lauf oder je Ereignis anders sind: die Kennung eines Ereignisses hat eine
/// Uhrzeit im Hash, und die Kennung des Stands (`snapshot_id`, `basis_snapshot`) haengt an allen Ereignissen.
fn ohne_event_id(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.remove("event_id");
            m.remove("snapshot_id");
            m.remove("basis_snapshot");
            m.values_mut().for_each(ohne_event_id);
        }
        Value::Array(a) => a.iter_mut().for_each(ohne_event_id),
        _ => {}
    }
}

/// Die ersten Stellen, an denen zwei Antworten auseinandergehen (Pfad, links, rechts), fuer die Meldung.
fn unterschiede(pfad: &str, a: &Value, b: &Value, aus: &mut Vec<String>) {
    if a == b || aus.len() >= 8 {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                unterschiede(
                    &format!("{pfad}/{k}"),
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                    aus,
                );
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (u, v)) in x.iter().zip(y).enumerate() {
                unterschiede(&format!("{pfad}/{i}"), u, v, aus);
            }
        }
        _ => aus.push(format!("{pfad}: {a} gegen {b}")),
    }
}

#[tokio::test]
async fn eine_alte_antwort_bleibt_im_store_und_sperrt_die_akte_nicht() {
    const ROUTEN: [&str; 7] = [
        "stand",
        "fragen",
        "ergebnis",
        "preflight",
        "deklaration",
        "graph",
        "elster-ampel",
    ];
    let d = dienst();
    fall_mit(&d, true).await;
    let mut vorher = Vec::new();
    for r in ROUTEN {
        let (status, mut body) = sende(&d, "GET", &format!("/fall/sf/{r}"), None).await;
        ohne_event_id(&mut body);
        vorher.push((r, status, body));
    }

    // Die Antwort, die ein Nutzer VOR der Streichung gegeben hat: ins Ereignis-Journal des Falls
    // geschrieben, wie es der Dienst damals tat.
    let pfad = d.tmp.path().join("faelle/sf.json");
    let mut s = Store::aus_datei(store::lade(&pfad).unwrap());
    let herkunft = Herkunft {
        herkunft: Achsenwert::new("laie").unwrap(),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: Achsenwert::new("nutzer").unwrap(),
    };
    let vor = s.datei().events.len();
    let leer = std::collections::HashMap::new();
    s.append_roh(
        &NeuesEventRoh {
            feld_id: FELD.to_owned(),
            wert: json!(true).into(),
            zustand: EventZustand::Bestaetigt,
            herkunft: HerkunftVektor::Voll(herkunft),
            schreiber: "ui:laie".to_owned(),
            signal: Signal {
                signal_1: Some(None),
                signal_2: Some(format!("ok@{FELD}")),
                signal_2_fehlt: false,
            },
            signal_2_fremd: None,
            ersetzt: None,
            ts: Some("2026-09-01T00:00:00+00:00".to_owned()),
        },
        None,
        BindungNachschlag::neu(&leer),
    )
    .unwrap();
    assert_eq!(s.datei().events.len(), vor + 1);
    store::speichere(&pfad, s.datei()).unwrap();

    for (r, status0, body0) in vorher {
        let (status, mut body) = sende(&d, "GET", &format!("/fall/sf/{r}"), None).await;
        ohne_event_id(&mut body);
        match r {
            // Das Feld steht unter `felder`, ohne Frage (die Oberflaeche zeigt es unter "Schon beantwortet").
            "stand" => {
                let eintrag = body["felder"]
                    .as_object_mut()
                    .unwrap()
                    .remove(FELD)
                    .unwrap();
                assert_eq!(
                    (&eintrag["wert"], &eintrag["zustand"], &eintrag["frage"]),
                    (&json!(true), &json!("bestaetigt"), &Value::Null)
                );
            }
            // Die Spur der Regel nennt das Feld unter ihren Eingaben; die Zahlen bleiben gleich.
            "ergebnis" => {
                let liste = body["trace"]["regeln"][REGEL].as_array_mut().unwrap();
                let i = liste.iter().position(|e| e["feld_id"] == FELD).unwrap();
                assert_eq!(liste.remove(i)["geltungsbedingung"], json!(ANNAHME));
            }
            // Die Kante des Felds ist "bestaetigt" statt "offen".
            "graph" => {
                let kante = body["kanten"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|k| k["feld_id"] == FELD)
                    .unwrap();
                assert_eq!(kante["zustand"], json!("bestaetigt"));
                kante["zustand"] = json!("offen");
            }
            // Das Feld steht unter `nicht_deklariert`, mit einem Grund, der die Streichung nennt, und die
            // Deklaration bleibt vollstaendig (`einreichungs_xml` bricht nur bei `eingaben_konsistent` ab).
            "deklaration" => {
                assert_eq!(body["vollstaendig"], json!(true), "deklaration");
                assert_eq!(body["eingaben_konsistent"], json!(true), "deklaration");
                assert_eq!(body["unvollstaendig"], json!([]), "deklaration");
                let liste = body["nicht_deklariert"].as_array_mut().unwrap();
                let i = liste.iter().position(|e| e["feld_id"] == FELD).unwrap();
                let grund = liste.remove(i)["grund"].as_str().unwrap().to_owned();
                assert!(
                    grund.starts_with("Frage gestrichen 2026-10-06")
                        && grund.contains("bewegt kein Ergebnis"),
                    "{grund}"
                );
            }
            _ => {}
        }
        assert_eq!(status, status0, "{r}: Status mit alter Antwort");
        let mut aus = Vec::new();
        unterschiede("", &body0, &body, &mut aus);
        assert!(
            aus.is_empty(),
            "{r}: Antwort mit alter Antwort im Store: {aus:#?}"
        );
    }
}
