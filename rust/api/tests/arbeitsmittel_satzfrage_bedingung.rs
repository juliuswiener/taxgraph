//! Arbeitsmittel: die Frage nach dem Umsatzsteuersatz der Rechnung steht nur, wenn der Preis ueber 800,00 EUR liegt
//! (Abweichung Nr. 88 in `rust/fixtures/README.md`). Ohne `PARITY=1`, ohne Python, ohne Netz. Python kennt das Feld nicht.
//!
//! **Worum es geht.** Beim Arbeitnehmer gilt die 800-EUR-Grenze fuer den Sofortabzug netto (R 9.12 `LStR`, Regel von Julius
//! 2026-10-19): bei 19 % Umsatzsteuer bis 952,00 EUR brutto, bei 7 % bis 856,00 EUR. Im Band ueber 800,00 EUR entscheidet der Satz
//! der Rechnung, also fragt die Software danach.
//!
//! **Warum es zaehlt.** Ohne Satz wuerde ein Geraet zu 900,00 EUR brutto in die Abschreibung laufen statt in den Sofortabzug (der
//! Abzug im Kaufjahr fiele zu klein aus); mit einem stillen Satz von 19 % stuende die Rechnung eines Kaufs von privat falsch da.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_n_vor_gwg.yaml` (`am_umsatzsteuersatz`: `feld_bedingung` mit `groesser_als` 80.000
//! auf `am_anschaffungskosten`), `rust/interview/src/fragen.rs::feld_ausgeschlossen` (die Auswertung). Die Sperre im Band haelt
//! `rust/bescheid/tests/sperre_scheiben_hermetisch.rs`, die Rechnung `gesamt_mischfaelle_hermetisch.rs`.
//!
//! **Belegt wird (positiver Beleg).** `groesser_als` schliesst auch bei SCHWEIGEN aus: die Frage steht erst, wenn ein BESTAETIGTER
//! Preis ueber 800,00 EUR vorliegt; die Eingangsfrage nach dem Preis steht davor in der Queue. Die Frage steht auch ueber 952,00
//! EUR brutto, wo sie nichts aendert (`feld_bedingung` kennt keine Obergrenze, `ponytail:` in der Bindung).
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

const SATZ: &str = "am_umsatzsteuersatz";
const PREIS: &str = "am_anschaffungskosten";

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

/// Legt den Fall `am` der Scheibe `scheibe` (VZ 2025) an und schreibt jedes Paar ueber die echte Route `POST /event`.
async fn fall(d: &Dienst, scheibe: &str, paare: &[(&str, Value, bool)]) {
    let kopf = json!({"fall_id": "am", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert, bestaetigt) in paare {
        let (status, antwort) =
            sende(d, "POST", "/fall/am/event", Some(&ereignis(feld, wert, *bestaetigt))).await;
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

/// Die Fragen eines Falls mit den Antworten `paare`.
async fn fragen_von(scheibe: &str, paare: &[(&str, Value, bool)]) -> Value {
    let d = dienst();
    fall(&d, scheibe, paare).await;
    let (status, fragen) = sende(&d, "GET", "/fall/am/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    fragen
}

/// AK1: Die Satzfrage steht erst bei einem BESTAETIGTEN Preis ueber 800,00 EUR (80.000 ct), schon bei einem Cent mehr. Bei
/// Schweigen, 0, 800,00 EUR und einem nur vorlaeufigen Preis steht sie nicht; die Eingangsfrage nach dem Preis steht in jedem
/// Fall in der Queue, solange der Preis nicht bestaetigt ist. Beide Scheiben, die Arbeitsmittel fragen (`gesamt`, `an_gesamt`).
#[tokio::test]
async fn die_satzfrage_steht_erst_bei_bestaetigtem_preis_ueber_800_euro() {
    for scheibe in ["gesamt", "an_gesamt"] {
        for (name, preis, erwartet) in [
            ("ohne Preis", None, false),
            ("Preis 0", Some((0, true)), false),
            ("Preis 500,00 EUR", Some((50_000, true)), false),
            ("Preis 800,00 EUR: die Grenze", Some((80_000, true)), false),
            ("Preis 800,01 EUR", Some((80_001, true)), true),
            ("Preis 900,00 EUR", Some((90_000, true)), true),
            ("Preis 952,00 EUR: das obere Ende des Bandes", Some((95_200, true)), true),
            ("Preis 1.200,00 EUR: ueber dem Band, die Frage steht trotzdem", Some((120_000, true)), true),
            ("Preis 900,00 EUR nur vorlaeufig", Some((90_000, false)), false),
        ] {
            let paare: Vec<(&str, Value, bool)> = preis
                .map(|(p, b)| (PREIS, json!(p), b))
                .into_iter()
                .collect();
            let fragen = fragen_von(scheibe, &paare).await;
            let da = frage(&fragen, SATZ).is_some();
            assert_eq!(da, erwartet, "{scheibe}, {name}: Satzfrage {da}, erwartet {erwartet}");
            if !matches!(preis, Some((_, true))) {
                assert!(
                    frage(&fragen, PREIS).is_some(),
                    "{scheibe}, {name}: die Preisfrage steht in der Queue und schaltet die Satzfrage frei"
                );
            }
        }
    }
}

/// Die Bedingung steuert nur die Queue, nicht den Schreibweg: `POST /event` nimmt einen der drei Saetze auch an, wenn die Frage
/// gerade nicht steht (ein Beleg-Import, ein Fall vor der Aenderung). Ein Wert ausserhalb der drei Saetze wird abgewiesen:
/// kein stiller Satz.
#[tokio::test]
async fn der_schreibweg_nimmt_nur_die_drei_saetze_an() {
    // Je Wert ein frischer Fall: ein zweites Ereignis auf dasselbe Feld braucht `ersetzt` und hiesse eine andere Frage.
    for (satz, erwartet) in [
        ("satz_19", 201),
        ("satz_7", 201),
        ("keine_ust", 201),
        ("satz_20", 422),
        ("19", 422),
        ("", 422),
    ] {
        let d = dienst();
        fall(&d, "gesamt", &[]).await;
        let (status, antwort) =
            sende(&d, "POST", "/fall/am/event", Some(&ereignis(SATZ, &json!(satz), true))).await;
        assert_eq!(status, erwartet, "{satz:?}: {antwort}");
    }
}
