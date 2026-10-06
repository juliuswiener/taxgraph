//! § 34 Abs. 3 `EStG` fuer den Ehegatten (B Option 1, 2026-10-06): wer die drei neuen Fragen sieht, was der Schreibweg mit den
//! Antworten tut und was `GET /ergebnis` daraus macht. Ohne `PARITY=1`, ohne Python, ohne Netz. Python kennt die drei Felder
//! nicht (Abweichung Nr. 23 in `rust/fixtures/README.md`).
//!
//! **Worum es geht.** Wer seinen Betrieb verkauft, kann einmal im Leben einen ermaessigten Steuersatz beantragen. Die Software
//! fragt das jetzt auch fuer den Ehe- oder Lebenspartner: ob er den Antrag stellen will, ob er dauernd berufsunfaehig ist und
//! ob er den Satz schon einmal genutzt hat. Das gilt nur bei Zusammenveranlagung; wer allein veranlagt wird, hat keinen
//! Partner und soll die Fragen nicht sehen.
//!
//! **Warum es zaehlt.** Eine Frage nach dem Partner, die ein Alleinstehender sieht, kann er nicht beantworten. Fehlt sie dem
//! Ehepaar, kann der Partner den Antrag nicht stellen und zahlt zu viel Steuer.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_an_gesamt.yaml` (die vier Felder, Regel `p2_festzusetzung_zusammen`) und
//! `rust/bindung/daten/bindung_regel_bedingungen.yaml` (diese Regel faellt bei bestaetigter Einzelveranlagung ganz weg;
//! solange die Veranlagung offen ist, bleiben die Fragen stehen, fail-closed). Die Rechnung steht in
//! `rust/bescheid/src/zweige/tarif.rs`; die Erwartungswerte sind die Handrechnung aus
//! `rust/bescheid/tests/p34_partner_hermetisch.rs`.
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

type Paare = Vec<(&'static str, Value)>;

const ANTRAG: &str = "antrag_ermaessigter_satz_partner";
const BERUFSUNFAEHIG: &str = "dauernd_berufsunfaehig_partner";
const EINMAL: &str = "ermaessigung_einmal_genutzt_partner";

fn frage<'a>(fragen: &'a Value, feld: &str) -> Option<&'a Value> {
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["feld_id"] == feld)
}

/// Legt den Fall `pp` (Scheibe `gesamt`) an und schreibt die Paare als bestaetigte Ereignisse ueber `POST /event`.
async fn fall_mit(d: &Dienst, paare: &[(&'static str, Value)]) {
    let kopf = json!({"fall_id": "pp", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (status, antwort) = sende(d, "POST", "/fall/pp/event", Some(&ereignis(feld, wert))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
}

/// Die drei Fragen stehen bei Zusammenveranlagung und solange die Veranlagung offen ist, bei Einzelveranlagung nicht.
#[tokio::test]
async fn die_fragen_stehen_nur_bei_zusammenveranlagung_und_solange_sie_offen_ist() {
    for (veranlagung, erwartet) in [
        (Some("zusammen"), true),
        (None, true),
        (Some("einzel"), false),
    ] {
        let d = dienst();
        let paare: Paare = veranlagung.map_or_else(Vec::new, |v| vec![("veranlagung", json!(v))]);
        fall_mit(&d, &paare).await;
        let (status, fragen) = sende(&d, "GET", "/fall/pp/fragen", None).await;
        assert_eq!(status, 200, "{veranlagung:?}: {fragen}");
        for feld in [ANTRAG, BERUFSUNFAEHIG, EINMAL] {
            let q = frage(&fragen, feld);
            assert_eq!(q.is_some(), erwartet, "{veranlagung:?}: {feld}: {q:?}");
            if let Some(q) = q {
                assert_eq!(q["typ"], "bool", "{feld}");
                assert!(
                    q["fragetext_laie"].as_str().is_some_and(|t| t.contains("partner")),
                    "{feld}: Fragetext nennt den Partner nicht: {q}"
                );
            }
        }
    }
}

/// Der Schreibweg (`POST /event`): die drei Angaben sind Ja/Nein und nehmen nur Ja/Nein an (Auflage T), `null` und Text nicht.
#[tokio::test]
async fn der_schreibweg_nimmt_nur_ja_nein_an() {
    let d = dienst();
    fall_mit(&d, &[("veranlagung", json!("zusammen"))]).await;
    for feld in [ANTRAG, BERUFSUNFAEHIG, EINMAL] {
        let (status, antwort) = sende(&d, "POST", "/fall/pp/event", Some(&ereignis(feld, &json!(true)))).await;
        assert_eq!(status, 201, "{feld}: {antwort}");
        for schlecht in [json!("ja"), json!(1), Value::Null] {
            let (status, antwort) = sende(&d, "POST", "/fall/pp/event", Some(&ereignis(feld, &schlecht))).await;
            assert!((400..500).contains(&status), "{feld} {schlecht}: Status {status}, erwartet eine Abweisung: {antwort}");
        }
    }
}

/// Der Partner mit 500.000 Euro Gewinn, A mit 60.000 Euro Lohn, Zusammenveranlagung, alles ueber die echten Routen: ohne Antrag
/// die Fuenftelregel (191.188 Euro), mit Antrag und Berechtigung Abs. 3 (114.946 Euro), mit Antrag und einem Gewinn bei A die
/// Sperre mit Klartext. Die Handrechnung steht in `bescheid/tests/p34_partner_hermetisch.rs`.
#[tokio::test]
async fn ueber_die_routen_rechnet_der_antrag_des_partners_und_beide_gewinn_sperrt() {
    let partner: Paare = vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("kein_gewinn", json!(false)),
        ("rentner_veraeusserungsgewinn_partner", json!(50_000_000)),
        ("rentner_veraeusserungs_betriebsart_partner", json!("gewerbe")),
        ("rentner_alter_55_oder_berufsunfaehig_partner", json!(true)),
        ("rentner_freibetrag_erstmalig_partner", json!(true)),
    ];
    let antrag: Paare = vec![
        (ANTRAG, json!(true)),
        ("geburtsjahr_partner", json!(1960)),
        (BERUFSUNFAEHIG, json!(false)),
        (EINMAL, json!(false)),
    ];
    let a_gewinn: Paare = vec![
        ("rentner_veraeusserungsgewinn", json!(20_000_000)),
        ("rentner_veraeusserungs_betriebsart", json!("gewerbe")),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
    ];
    let lauf = |zusatz: Vec<Paare>| async move {
        let mut e = kegel_gesamt();
        for z in zusatz {
            e = mit(e, z);
        }
        let d = dienst();
        fall_mit(&d, &e).await;
        let (status, a) = sende(&d, "GET", "/fall/pp/ergebnis", None).await;
        assert_eq!(status, 200, "GET /ergebnis: {a}");
        a
    };
    let ohne = lauf(vec![partner.clone()]).await;
    assert_eq!(ohne["grund"], "bestaetigt", "{ohne}");
    assert_eq!(ohne["zahl_cent"].as_i64(), Some(19_118_800), "Fuenftelregel: {ohne}");
    let mit_antrag = lauf(vec![partner.clone(), antrag.clone()]).await;
    assert_eq!(mit_antrag["grund"], "bestaetigt", "{mit_antrag}");
    assert_eq!(mit_antrag["zahl_cent"].as_i64(), Some(11_494_600), "Abs. 3: {mit_antrag}");
    assert_eq!(mit_antrag["kette"]["tarifliche_est"].as_i64(), Some(114_946), "{mit_antrag}");
    let beide = lauf(vec![partner, antrag, a_gewinn]).await;
    assert_eq!(beide["grund"], "abs3_partner_antrag_gewinn_offen", "{beide}");
    assert!(beide["zahl_cent"].is_null(), "{beide}");
    assert!(
        beide["klartext"].as_str().is_some_and(|t| t.contains("Ehepartner")),
        "Klartext fehlt: {beide}"
    );
}

/// Basis plus Aenderungen: ein Feld, das die Basis schon traegt, wird ersetzt, ein neues angehaengt.
fn mit(basis: Paare, aenderungen: Paare) -> Paare {
    let mut raus = basis;
    for (feld, wert) in aenderungen {
        match raus.iter_mut().find(|(f, _)| *f == feld) {
            Some(p) => p.1 = wert,
            None => raus.push((feld, wert)),
        }
    }
    raus
}

/// Der volle Pflicht-Kegel der Scheibe `gesamt` (Einzelveranlagung, 60.000 Euro Lohn); `tests/_kegel.py::kegel_fuer`.
fn kegel_gesamt() -> Paare {
    vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(0)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
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
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ]
}
