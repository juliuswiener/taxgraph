//! Unfallkosten auf dem Arbeitsweg (`ep_unfallkosten`, gebaut 2026-10-07): die HTTP-Naht. Ohne `PARITY=1`, ohne Python, ohne
//! Netz, ohne `ERiC`. Python kennt das Feld nicht (Abweichung Nr. 28 in `rust/fixtures/README.md`).
//!
//! **Worum es geht.** Wer auf dem Weg zur Arbeit einen Unfall hatte, darf die Kosten ZUSAETZLICH zur Entfernungspauschale
//! abziehen (BMF-Schreiben vom 18.11.2021, Rz. 30). Der Bundesfinanzhof sieht das fuer Fahrzeug- und Wegschaeden anders
//! (VI R 8/18); `TaxGraph` folgt der Finanzverwaltung und sagt es im Hilfetext.
//!
//! **Warum es zaehlt.** Vor dem Bau fehlte die Frage: wer 1.500 EUR Unfallkosten hatte, gab sie nirgends an und zahlte etwa
//! 500 EUR Steuer zu viel. Umgekehrt darf der Betrag das ELSTER-Formular nie verfehlen, ohne dass der Nutzer es erfaehrt: das
//! Kennzeichen ist ungeprueft, also sperrt die Abgabe ab einem Betrag ueber 0 (409), die Zahl im Bescheid bleibt.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_n_vor_gwg.yaml` (Feld, Bedingung, Hilfetext), `rust/bescheid/src/zweige/wk.rs::
//! mit_unfallkosten` (Rechnung), `rust/elster/src/deklaration.rs::unfallkosten` (Sperre). Die Rechnung im Einzelnen pruefen
//! `rust/bescheid/tests/unfallkosten_neben_entfernungspauschale.rs`, die Sperre `unfallkosten_einreichung_hermetisch.rs`; hier
//! stehen Frage, Hilfetext, Schreibweg, Ergebnis und die 409-Antwort.
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

type Paare = Vec<(&'static str, Value)>;

const FELD: &str = "ep_unfallkosten";

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
    std::fs::create_dir_all(&konfig.faelle).unwrap();
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
        "ts": "2026-01-01T00:00:00+00:00",
    })
}

/// Legt den Fall `uk` der Scheibe `gesamt` (VZ 2025) an und schreibt jedes Paar ueber die echte Route `POST /event`.
async fn fall(d: &Dienst, paare: &[(&str, Value)]) {
    let kopf = json!({"fall_id": "uk", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (status, antwort) = sende(d, "POST", "/fall/uk/event", Some(&ereignis(feld, wert))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
}

/// Pflicht-Kegel `gesamt` (35 Felder), Einzelveranlagung, 60.000 EUR Arbeitslohn, EUeR-Weg offen (`kein_gewinn` nein, drei
/// Nullen), damit ein Abzug aus der `EUeR` dieselbe Zahl gibt wie der Abzug in der Anlage N. Wie `gwg_folgefrage_hermetisch.rs::
/// kegel`, aber mit einem Arbeitsweg: `km` Kilometer, 220 Tage, eigenes Kfz. `km` = `None` laesst die Entfernung unbeantwortet.
fn kegel(km: Option<i64>) -> Paare {
    let mut p: Paare = vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(0)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("ep_arbeitstage", json!(if km == Some(0) { 0 } else { 220 })),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(km != Some(0))),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!("gesetzlich_an")),
        ("basis_kv", json!(0)),
        ("basis_pv", json!(0)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(false)),
        ("kap_kapitalertraege", json!(0)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
        ("kein_gewinn", json!(false)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
        ("betriebseinnahmen", json!(0)),
        ("afa_jahresbetrag", json!(0)),
        ("sonstige_betriebsausgaben", json!(0)),
    ];
    if let Some(km) = km {
        p.push(("ep_entfernung_km", json!(km)));
    }
    p
}

/// Der Kegel mit `cent` als Unfallkosten.
fn mit_unfall(km: i64, cent: i64) -> Paare {
    let mut p = kegel(Some(km));
    p.push((FELD, json!(cent)));
    p
}

/// Derselbe Kegel, aber `cent` als sonstige Betriebsausgabe: die Referenz, die dieselbe Summe der Einkuenfte senkt.
fn mit_betriebsausgabe(km: i64, cent: i64) -> Paare {
    kegel(Some(km))
        .into_iter()
        .map(|(f, w)| {
            if f == "sonstige_betriebsausgaben" {
                (f, json!(cent))
            } else {
                (f, w)
            }
        })
        .collect()
}

fn fragen_ids(fragen: &Value) -> Vec<&str> {
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|q| q["feld_id"].as_str())
        .collect()
}

async fn ids(paare: &[(&str, Value)]) -> Vec<String> {
    let d = dienst();
    fall(&d, paare).await;
    let (status, fragen) = sende(&d, "GET", "/fall/uk/fragen", None).await;
    assert_eq!(status, 200, "{fragen}");
    fragen_ids(&fragen).into_iter().map(str::to_owned).collect()
}

/// AK2: die Frage steht nur zu einem Arbeitsweg. Bei bestaetigten 0 km faellt sie weg; bei 30 km und bei einer
/// unbeantworteten Entfernung steht sie (fail-closed: Schweigen schliesst sie nicht aus).
#[tokio::test]
async fn die_frage_steht_nur_mit_einer_entfernungspauschale() {
    let mit_weg = ids(&kegel(Some(30))).await;
    assert!(mit_weg.iter().any(|f| f == FELD), "30 km: {mit_weg:?}");
    let ohne_weg = ids(&kegel(Some(0))).await;
    assert!(
        !ohne_weg.is_empty(),
        "KONTROLLE: der Katalog ist nicht leer, die Frage fehlt nicht nur, weil nichts mehr gefragt wird"
    );
    assert!(
        !ohne_weg.iter().any(|f| f == FELD),
        "0 km bestaetigt, und die Frage steht doch: {ohne_weg:?}"
    );
    let stumm = ids(&kegel(None)).await;
    assert!(
        stumm.iter().any(|f| f == FELD),
        "Entfernung unbeantwortet: die Frage darf nicht still wegfallen: {stumm:?}"
    );
}

/// AK4: der Hilfetext nennt beide Ansichten (BMF gegen BFH VI R 8/18), sagt, welcher `TaxGraph` folgt, und warnt vor der Sperre
/// der Abgabe. Der Fragetext des Laien nennt keinen Paragrafen.
#[tokio::test]
async fn der_hilfetext_nennt_bmf_gegen_bfh_und_die_sperre() {
    let d = dienst();
    fall(&d, &kegel(Some(30))).await;
    let (status, fragen) = sende(&d, "GET", "/fall/uk/fragen", None).await;
    assert_eq!(status, 200, "{fragen}");
    let q = fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["feld_id"] == FELD)
        .unwrap_or_else(|| panic!("die Frage steht nicht: {:?}", fragen_ids(&fragen)));
    let hilfe = q["hilfe_kurz"].as_str().unwrap();
    for teil in [
        "Finanzverwaltung",
        "Bundesfinanzhof",
        "VI R 8/18",
        "TaxGraph folgt der Finanzverwaltung",
        "Abgabe",
        "gesperrt",
        "zusätzlich zur Pauschale",
    ] {
        assert!(hilfe.contains(teil), "Hilfetext ohne `{teil}`: {hilfe}");
    }
    assert!(!q["fragetext_laie"].as_str().unwrap().contains('§'), "{q}");
    assert_eq!(
        (q["typ"].as_str(), q["einheit"].as_str()),
        (Some("cent"), Some("EUR")),
        "{q}"
    );
    assert!(
        q["anker_ref"]["datei"]
            .as_str()
            .is_some_and(|f| f.starts_with("sources/bmf/")),
        "der Anker zeigt nicht auf das eingefrorene BMF-Schreiben: {q}"
    );
}

/// Der Schreibweg (`POST /event`) nimmt Betraege ab 0 an und weist negative, Text-, Dezimal- und leere Werte mit 422 ab.
#[tokio::test]
async fn der_schreibweg_nimmt_betraege_an_und_weist_den_rest_ab() {
    // Ein frischer Fall je Wert: ein zweites Event auf dasselbe Feld verlangt `ersetzt` und wiese jeden Wert ab.
    for wert in [json!(0), json!(150_000)] {
        let d = dienst();
        fall(&d, &kegel(Some(30))).await;
        let (status, antwort) = sende(&d, "POST", "/fall/uk/event", Some(&ereignis(FELD, &wert))).await;
        assert_eq!(status, 201, "{wert}: {antwort}");
    }
    for wert in [json!(-1), json!("viel"), json!(1.5), Value::Null] {
        let d = dienst();
        fall(&d, &kegel(Some(30))).await;
        let (status, antwort) = sende(&d, "POST", "/fall/uk/event", Some(&ereignis(FELD, &wert))).await;
        assert_eq!(status, 422, "{wert}: Status {status}, erwartet 422: {antwort}");
        let text = antwort["fehler"].as_str().unwrap_or_default();
        assert!(
            !text.contains("aktives Event"),
            "{wert}: abgewiesen aus dem falschen Grund: {antwort}"
        );
    }
}

/// `GET /ergebnis`: `(grund, zahl_cent)`.
async fn ergebnis(paare: &[(&str, Value)]) -> (String, Option<i64>) {
    let d = dienst();
    fall(&d, paare).await;
    let (status, a) = sende(&d, "GET", "/fall/uk/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {a}");
    (
        a["grund"].as_str().unwrap_or("").to_owned(),
        a["zahl_cent"].as_i64(),
    )
}

/// AK3/AK4 Ende zu Ende: 1.500 EUR Unfallkosten neben der Pauschale (30 km, 220 Tage) senken die Steuer um GENAU so viel wie
/// 1.500 EUR sonstige Betriebsausgaben, die die Einkuenfte um denselben Betrag senken (unabhaengige Referenz ueber die `EUeR`).
/// Leer und 0 geben die Zahl ohne Feld; ein Fall ohne Arbeitsweg bleibt von der Antwort unberuehrt (die Frage steht dort nicht).
#[tokio::test]
async fn unfallkosten_senken_die_steuer_wie_ein_gleich_hoher_abzug() {
    let (g_ohne, ohne) = ergebnis(&kegel(Some(30))).await;
    let (g_ref, referenz) = ergebnis(&mit_betriebsausgabe(30, 150_000)).await;
    assert_eq!(
        (g_ohne.as_str(), g_ref.as_str()),
        ("bestaetigt", "bestaetigt"),
        "KONTROLLE: die Referenzen rechnen"
    );
    let (ohne, referenz) = (ohne.unwrap(), referenz.unwrap());
    assert!(
        referenz < ohne,
        "KONTROLLE: 1.500 EUR Abzug senken die Steuer: {referenz} gegen {ohne} Cent"
    );
    assert_eq!(
        ergebnis(&mit_unfall(30, 150_000)).await,
        ("bestaetigt".to_owned(), Some(referenz)),
        "Unfallkosten 1.500 EUR neben der Pauschale"
    );
    assert_eq!(
        ergebnis(&mit_unfall(30, 0)).await,
        ("bestaetigt".to_owned(), Some(ohne)),
        "Unfallkosten 0 aendern die Zahl"
    );
}

/// AK5 ueber die echte Route: eine vollstaendige Akte (`rust/fixtures/e2e/gesamt.json`, mit Stammdaten) mit Arbeitsweg und
/// einem Betrag ueber 0 wird beim Einreichen mit 409 `deklaration_unvollstaendig` abgewiesen, und die Liste nennt genau
/// dieses Feld mit dem Grund. Die Sperre liegt VOR dem Writer und vor `ERiC`: der Test setzt weder `ERiC` noch ein Schema voraus.
#[tokio::test]
async fn einreichen_mit_unfallkosten_ist_409_mit_eigenem_grund() {
    let d = dienst();
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let mut akte: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    for e in akte["events"].as_array_mut().unwrap() {
        match e["feld_id"].as_str() {
            Some("ep_entfernung_km") => e["wert"] = json!(30),
            Some("ep_arbeitstage") => e["wert"] = json!(220),
            Some("ep_eigenes_kfz") => e["wert"] = json!(true),
            _ => {}
        }
    }
    akte["fall_id"] = json!("uk");
    akte["user_id"] = json!("alice");
    std::fs::write(
        d.zustand.konfig.faelle.join("uk.json"),
        serde_json::to_vec(&akte).unwrap(),
    )
    .unwrap();
    let (status, antwort) = sende(&d, "POST", "/fall/uk/event", Some(&ereignis(FELD, &json!(150_000)))).await;
    assert_eq!(status, 201, "POST /event: {antwort}");

    let (status, a) = sende(&d, "POST", "/fall/uk/einreichen", Some(&json!({}))).await;
    assert_eq!(status, 409, "erwartet 409, erhalten {status}: {a}");
    assert_eq!(a["grund"], "deklaration_unvollstaendig", "{a}");
    let eintraege = a["unvollstaendig"].as_array().unwrap();
    assert_eq!(
        eintraege.iter().filter_map(|e| e["feld_id"].as_str()).collect::<Vec<_>>(),
        [FELD],
        "{a}"
    );
    let grund = eintraege[0]["grund"].as_str().unwrap();
    assert!(
        grund.contains("gesperrt") && grund.contains("Kennzeichen"),
        "der Grund nennt die Sperre nicht: {grund}"
    );
}
