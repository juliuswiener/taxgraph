//! Schulgeld: die Frage nach dem Anteil am Hoechstbetrag steht nur, wenn das Schulgeld eines Kindes ueber 0 liegt
//! (Abweichung Nr. 34 in `rust/fixtures/README.md`). Ohne `PARITY=1`, ohne Python, ohne Netz. Python kennt das Feld nicht.
//!
//! **Worum es geht.** Seit Abweichung Nr. 26 fragt die Software bei Einzelveranlagung je Kind, welchen Anteil am Schulgeld-
//! Hoechstbetrag der Nutzer traegt. Sie fragte das bei jedem Elternteil mit Kind, auch wenn er gar kein Schulgeld zahlt.
//!
//! **Warum es zaehlt.** Es rechnete nichts falsch (0 EUR), verlaengerte aber das Interview fuer alle ohne Schulgeld. Umgekehrt
//! darf die Bedingung die Frage nie dort verstecken, wo sie gebraucht wird: bei Schulgeld ueber 0 in Einzelveranlagung.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_p10_1_9_schulgeld_gesamt.yaml` (`feld_bedingung` mit `und`: Einzelveranlagung UND
//! `schulgeld` `groesser_als` 0), `rust/interview/src/fragen.rs::feld_ausgeschlossen` (die Auswertung), `rust/bindung/src/
//! bindung_datei.rs` (die Bedingungsart). Die Rechnung bleibt, wie sie war (`schulgeld_anteil_hermetisch.rs`).
//!
//! **Belegt wird (positiver Beleg).** Anders als `wert`, `wert_nicht` und `alter_im_vz` schliesst `groesser_als` auch bei
//! SCHWEIGEN aus: die Frage steht erst, wenn ein BESTAETIGTES Schulgeld ueber 0 vorliegt. Das ist sicher, weil eine fehlende
//! Antwort auf den Anteil "je zur Haelfte" heisst; und die Eingangsfrage `schulgeld` steht vorn in der Queue.
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

const ANTEIL: &str = "kind_schulgeld_aufteilung_prozent";
const SCHULGELD: &str = "schulgeld";

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

/// Ein Ereignis, bestaetigt (zwei Signale) oder vorlaeufig (nur das erste, `signal_2` leer).
fn ereignis(feld_id: &str, wert: &Value, bestaetigt: bool) -> Value {
    let signal_2 = if bestaetigt {
        json!(format!("ok@{feld_id}"))
    } else {
        Value::Null
    };
    json!({
        "feld_id": feld_id,
        "herkunft": {"haftung": "nutzer", "herkunft": "laie", "pruef_tiefe": "ungeprueft"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": null, "signal_2": signal_2},
        "wert": wert,
        "zustand": if bestaetigt { "bestaetigt" } else { "vorlaeufig" },
        "ts": "2026-01-01T00:00:00+00:00",
    })
}

/// Legt den Fall `sb` der Scheibe `gesamt` (VZ 2025) an und schreibt jedes Paar ueber die echte Route `POST /event`.
async fn fall(d: &Dienst, paare: &[(String, Value, bool)]) {
    let kopf = json!({"fall_id": "sb", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert, bestaetigt) in paare {
        let (status, antwort) =
            sende(d, "POST", "/fall/sb/event", Some(&ereignis(feld, wert, *bestaetigt))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
}

fn frage<'a>(fragen: &'a Value, feld: &str) -> Option<&'a Value> {
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["feld_id"] == feld)
}

/// Eine bestaetigte Eingabe `(Feld, Wert, bestaetigt)`.
fn ein(feld: &str, wert: &Value) -> (String, Value, bool) {
    (feld.to_owned(), wert.clone(), true)
}

/// Die Frage `ANTEIL` steht in `GET /fragen` genau dann, wenn die Eingaben es verlangen. `kinder` ist die Zahl der Kinder,
/// `veranlagung` die Antwort (oder keine), `schulgeld` die Antworten je Kind `(Feld, Wert, bestaetigt)`.
async fn steht(
    kinder: i64,
    veranlagung: Option<&str>,
    schulgeld: &[(&str, Value, bool)],
) -> (bool, Value) {
    let d = dienst();
    let mut paare = vec![ein("fam_anzahl_kinder", &json!(kinder))];
    if let Some(v) = veranlagung {
        paare.push(ein("veranlagung", &json!(v)));
    }
    for (feld, wert, bestaetigt) in schulgeld {
        paare.push(((*feld).to_owned(), wert.clone(), *bestaetigt));
    }
    fall(&d, &paare).await;
    let (status, fragen) = sende(&d, "GET", "/fall/sb/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    (frage(&fragen, ANTEIL).is_some(), fragen)
}

/// AK1: Ohne Schulgeld und bei Schulgeld 0 steht die Anteilsfrage nicht; bei Schulgeld ueber 0 steht sie, schon bei 1 Cent.
/// Die Eingangsfrage `schulgeld` selbst bleibt in jedem Fall in der Queue: sie ist es, die die Anteilsfrage freischaltet.
#[tokio::test]
async fn die_anteilsfrage_steht_erst_bei_bestaetigtem_schulgeld_ueber_0() {
    for (name, schulgeld, erwartet) in [
        ("ohne Schulgeld", vec![], false),
        ("Schulgeld 0", vec![(SCHULGELD, json!(0), true)], false),
        ("Schulgeld 1 Cent", vec![(SCHULGELD, json!(1), true)], true),
        (
            "Schulgeld 3.000 EUR",
            vec![(SCHULGELD, json!(300_000), true)],
            true,
        ),
        (
            "Schulgeld ueber 0, nur vorlaeufig",
            vec![(SCHULGELD, json!(300_000), false)],
            false,
        ),
    ] {
        let (da, fragen) = steht(1, Some("einzel"), &schulgeld).await;
        assert_eq!(da, erwartet, "{name}: Anteilsfrage {da}, erwartet {erwartet}");
        if erwartet {
            let q = frage(&fragen, ANTEIL).unwrap();
            assert_eq!(q["instanz_anzahl"], json!(1), "{name}");
        }
        if schulgeld.is_empty() {
            assert!(
                frage(&fragen, SCHULGELD).is_some(),
                "{name}: die Eingangsfrage steht in der Queue und schaltet die Anteilsfrage frei"
            );
        }
    }
}

/// AK1: Die Veranlagung bleibt Teil der Bedingung (`und`): bei Zusammenveranlagung steht die Anteilsfrage auch mit Schulgeld
/// nicht, bei unbeantworteter Veranlagung steht sie (Schweigen schliesst die Veranlagungs-Bedingung nie aus).
#[tokio::test]
async fn die_veranlagung_bleibt_teil_der_bedingung() {
    let mit_schulgeld = [(SCHULGELD, json!(300_000), true)];
    for (veranlagung, erwartet) in [(Some("einzel"), true), (None, true), (Some("zusammen"), false)] {
        let (da, _) = steht(1, veranlagung, &mit_schulgeld).await;
        assert_eq!(da, erwartet, "Veranlagung {veranlagung:?} mit Schulgeld");
    }
    // Ohne Schulgeld steht sie nirgends, auch nicht bei unbeantworteter Veranlagung.
    for veranlagung in [Some("einzel"), None, Some("zusammen")] {
        let (da, _) = steht(1, veranlagung, &[]).await;
        assert!(!da, "Veranlagung {veranlagung:?} ohne Schulgeld");
    }
}

/// AK1: Mit zwei Kindern genuegt EIN Kind mit bestaetigtem Schulgeld ueber 0, gleich welches (Kind 2 heisst `schulgeld__2`).
/// Die Queue fuehrt Felder, nicht Kinder: die Frage steht dann fuer beide Kinder (`instanz_anzahl` 2).
#[tokio::test]
async fn bei_zwei_kindern_genuegt_ein_kind_mit_schulgeld() {
    let zweites = "schulgeld__2";
    for (name, schulgeld, erwartet) in [
        ("beide 0", vec![(SCHULGELD, json!(0), true), (zweites, json!(0), true)], false),
        ("nur Kind 1 mit Schulgeld", vec![(SCHULGELD, json!(300_000), true), (zweites, json!(0), true)], true),
        ("nur Kind 2 mit Schulgeld", vec![(SCHULGELD, json!(0), true), (zweites, json!(300_000), true)], true),
        ("Kind 2 mit Schulgeld, Kind 1 offen", vec![(zweites, json!(300_000), true)], true),
        ("Kind 1 0, Kind 2 offen", vec![(SCHULGELD, json!(0), true)], false),
        (
            "Kind 1 vorlaeufig, Kind 2 0",
            vec![(SCHULGELD, json!(300_000), false), (zweites, json!(0), true)],
            false,
        ),
    ] {
        let (da, fragen) = steht(2, Some("einzel"), &schulgeld).await;
        assert_eq!(da, erwartet, "{name}");
        if erwartet {
            assert_eq!(frage(&fragen, ANTEIL).unwrap()["instanz_anzahl"], json!(2), "{name}");
        }
    }
}

/// Die Bedingung steuert nur die Queue, nicht den Schreibweg: den Anteil nimmt `POST /event` auch an, wenn die Frage gerade
/// nicht steht (ein Fall aus der Zeit vor Nr. 34, oder ein Beleg-Import). Die Rechnung liest ihn weiter
/// (`schulgeld_anteil_hermetisch.rs::der_anteil_aendert_die_steuer_und_die_fehlende_antwort_sperrt_nie`).
#[tokio::test]
async fn der_schreibweg_nimmt_den_anteil_auch_ohne_schulgeld_an() {
    let d = dienst();
    fall(
        &d,
        &[ein("fam_anzahl_kinder", &json!(1)), ein("veranlagung", &json!("einzel"))],
    )
    .await;
    let (status, antwort) =
        sende(&d, "POST", "/fall/sb/event", Some(&ereignis(ANTEIL, &json!(100), true))).await;
    assert_eq!(status, 201, "{antwort}");
    let (_, fragen) = sende(&d, "GET", "/fall/sb/fragen", None).await;
    assert!(
        frage(&fragen, ANTEIL).is_none(),
        "ohne Schulgeld steht die Frage nicht, auch nicht mit einer Antwort: {fragen}"
    );
}
