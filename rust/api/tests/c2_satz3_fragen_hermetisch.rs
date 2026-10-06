//! C2 (§ 32 Abs. 6 Satz 3 Nr. 1 `EStG`, gebaut 2026-10-06): wer die beiden neuen Fragen zum anderen
//! Elternteil sieht und was der Schreibweg mit den Antworten tut. Ohne `PARITY=1`, ohne Python, ohne Netz.
//! Python kennt die beiden Felder nicht (Abweichung Nr. 22 in `rust/fixtures/README.md`).
//!
//! **Worum es geht.** Die Software fragt je Kind, ob der andere Elternteil verstorben ist und ob er im
//! Ausland lebte. Beides zaehlt nur, wenn man nicht zusammen mit dem anderen Elternteil veranlagt wird.
//! Wer zusammen veranlagt wird, soll die beiden Fragen nicht sehen.
//!
//! **Warum es zaehlt.** Eine Frage, die niemand beantworten kann (Ehepaar, Kind gehoert beiden), laesst
//! die Akte offen oder lockt eine Angabe heraus, die nichts bewirkt. Fehlen die Fragen dagegen bei
//! Einzelveranlagung, bekommt ein verwitweter oder getrennt lebender Elternteil den doppelten
//! Freibetrag nie, ohne es zu merken.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_kap_vv_familie.yaml`, `feld_bedingung` auf `veranlagung`
//! (`rust/interview/src/fragen.rs::feld_ausgeschlossen`): die Frage entfaellt, sobald `veranlagung`
//! bestaetigt etwas anderes als `einzel` ist. Solange sie unbeantwortet ist, bleibt sie stehen
//! (fail-closed wie bei allen Fragen mit dieser Bedingung).
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

const TOD: &str = "kind_anderer_elternteil_tod_am";
const AUSLAND: &str = "kind_anderer_elternteil_ausland_zeitraum";

/// Legt einen Fall `sf` (Scheibe `gesamt`, zwei Kinder) an; `veranlagung` ist `None` (unbeantwortet), `"einzel"`
/// oder `"zusammen"`.
async fn fall_mit(d: &Dienst, veranlagung: Option<&str>) {
    let kopf = json!({"fall_id": "sf", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let mut eingaben = vec![ereignis("fam_anzahl_kinder", &json!(2))];
    if let Some(v) = veranlagung {
        eingaben.push(ereignis("veranlagung", &json!(v)));
    }
    for e in eingaben {
        let (status, antwort) = sende(d, "POST", "/fall/sf/event", Some(&e)).await;
        assert_eq!(status, 201, "POST /event {}: {antwort}", e["feld_id"]);
    }
}

fn frage<'a>(fragen: &'a Value, feld: &str) -> Option<&'a Value> {
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["feld_id"] == feld)
}

/// Einzelveranlagung und unbeantwortete Veranlagung zeigen die beiden Fragen (mit zwei Kindern: je Kind);
/// Zusammenveranlagung zeigt sie nicht.
#[tokio::test]
async fn die_fragen_stehen_nur_bei_einzelveranlagung_und_solange_sie_offen_ist() {
    for (veranlagung, erwartet) in [
        (Some("einzel"), true),
        (None, true),
        (Some("zusammen"), false),
    ] {
        let d = dienst();
        fall_mit(&d, veranlagung).await;
        let (status, fragen) = sende(&d, "GET", "/fall/sf/fragen", None).await;
        assert_eq!(status, 200, "{veranlagung:?}: {fragen}");
        for feld in [TOD, AUSLAND] {
            let q = frage(&fragen, feld);
            assert_eq!(q.is_some(), erwartet, "{veranlagung:?}: {feld}: {q:?}");
            if let Some(q) = q {
                assert_eq!(q["instanz_anzahl"], json!(2), "{veranlagung:?}: {feld} je Kind");
                assert_eq!(q["instanz_etikett"], json!("Kind"), "{feld}");
            }
        }
    }
}

/// Der Schreibweg (`POST /event`): ein Todestag und ein Ausland-Zeitraum je Kind werden angenommen
/// (Kind 2 ueber die Instanz-Kennung `..__2`), leerer Text und `null` nicht (Auflage T und M), ein
/// kalenderrisch unmoeglicher Tag schon (die Rechnung sperrt, `bescheid/tests/c2_weit_ang_xml_hermetisch.rs`).
#[tokio::test]
async fn der_schreibweg_nimmt_beide_antworten_je_kind_an_und_weist_leeres_ab() {
    let d = dienst();
    fall_mit(&d, Some("einzel")).await;
    let angenommen = [
        (TOD, json!("15.07.2025")),
        ("kind_anderer_elternteil_tod_am__2", json!("31.12.2024")),
        (AUSLAND, json!("01.01-30.06")),
        ("kind_anderer_elternteil_ausland_zeitraum__2", json!("01.07-31.12")),
        (TOD, json!("31.02.2025")),
    ];
    for (feld, wert) in &angenommen {
        // `ersetzt` nur beim zweiten Todestag derselben Instanz noetig; die letzte Zeile ersetzt den ersten Todestag.
        let mut e = ereignis(feld, wert);
        if *feld == TOD && wert == &json!("31.02.2025") {
            let (_, stand) = sende(&d, "GET", "/fall/sf/stand", None).await;
            e["ersetzt"] = stand["felder"][TOD]["event_id"].clone();
        }
        let (status, antwort) = sende(&d, "POST", "/fall/sf/event", Some(&e)).await;
        assert_eq!(status, 201, "{feld} {wert}: {antwort}");
    }
    for (feld, wert) in [
        (TOD, json!("")),
        (TOD, Value::Null),
        (AUSLAND, json!("")),
        (AUSLAND, Value::Null),
        (AUSLAND, json!("01.13-31.12")),
        (AUSLAND, json!("2025")),
        (TOD, json!("2025-07-15")),
        (TOD, json!("15.7.2025")),
    ] {
        let e = ereignis(feld, &wert);
        let (status, antwort) = sende(&d, "POST", "/fall/sf/event", Some(&e)).await;
        assert!(
            (400..500).contains(&status),
            "{feld} {wert}: Status {status}, erwartet eine Abweisung: {antwort}"
        );
    }
}
