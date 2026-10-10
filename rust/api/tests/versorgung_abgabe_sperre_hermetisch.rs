//! Versorgungsbezuege sperren die Abgabe (Abweichung Nr. 42): die HTTP-Naht. Ohne `PARITY=1`, ohne Python, ohne Netz, ohne
//! `ERiC`. Python laesst die Abgabe zu (Nr. 33: "stehen im Bescheid, aber nicht im XML").
//!
//! **Worum es geht.** Wer einen Versorgungsbezug (Pension) angibt, bekommt ihn im Bescheid verrechnet. Die ELSTER-Erklaerung
//! traegt ihn nicht: `versorgung_jahresrente` (Person A) und `versorgung_jahresrente_partner` (Ehegatte) haben kein
//! Kennzeichen. Vorher stand der Betrag nur in `nicht_deklariert`, und die Abgabe ging durch.
//!
//! **Warum es zaehlt.** Das Finanzamt haette den Bezug nie gesehen. Jetzt sperrt `GET /deklaration` (`unvollstaendig`) und
//! `POST /einreichen` (409 `deklaration_unvollstaendig`) bei einem bestaetigten Betrag ueber 0; die Zahl im Bescheid bleibt.
//!
//! **Wo es sitzt.** `rust/elster/src/deklaration.rs::Bau::versorgung`. Die Sperre selbst und ihre Grenzen prueft
//! `rust/bescheid/tests/versorgung_abgabe_sperre.rs`; hier stehen die Scheiben `gesamt` und `rentner_gesamt` auf dem Weg
//! `POST /event`, `GET /deklaration`, `GET /ergebnis`, `POST /einreichen`.
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

const A: &str = "versorgung_jahresrente";
const B: &str = "versorgung_jahresrente_partner";

struct Dienst {
    zustand: Zustand,
    token: String,
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

/// Ein Event ueber die echte Route `POST /event` (Nutzer-Klick); `bestaetigt = false` schreibt es vorlaeufig, ohne zweites
/// Signal.
fn ereignis(feld: &str, wert: &Value, bestaetigt: bool) -> Value {
    json!({
        "feld_id": feld, "wert": wert,
        "zustand": if bestaetigt { "bestaetigt" } else { "vorlaeufig" },
        "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": bestaetigt.then(|| format!("ok@{feld}"))},
        "ts": "2026-01-01T00:00:00+00:00",
    })
}

/// Ein Fall der Scheibe mit allen `paare`, jedes ueber `POST /event` (muss 201 sein).
async fn fall_mit(scheibe: &str, paare: &[(&'static str, Value)]) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "vs", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (status, antwort) = sende(&d, "POST", "/fall/vs/event", Some(&ereignis(feld, wert, true))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    d
}

/// Ein vorlaeufiges Event auf einen bestehenden Fall.
async fn vorlaeufig(d: &Dienst, feld: &str, wert: &Value) {
    let (status, antwort) = sende(d, "POST", "/fall/vs/event", Some(&ereignis(feld, wert, false))).await;
    assert_eq!(status, 201, "POST /event {feld} (vorlaeufig): {antwort}");
}

fn feld_ids(dekl: &Value, schluessel: &str) -> Vec<String> {
    dekl[schluessel]
        .as_array()
        .map(|a| a.iter().map(|e| e["feld_id"].as_str().unwrap().to_owned()).collect())
        .unwrap_or_default()
}

/// Was `GET /deklaration` und `GET /ergebnis` des Falls sagen: `(unvollstaendig, nicht_deklariert, ergebnis)`, die Listen
/// auf Felder `versorgung_*` gekuerzt.
async fn bericht(d: &Dienst) -> (Vec<String>, Vec<String>, Value) {
    let (status, dekl) = sende(d, "GET", "/fall/vs/deklaration", None).await;
    assert_eq!(status, 200, "{dekl}");
    let (_, erg) = sende(d, "GET", "/fall/vs/ergebnis", None).await;
    let nur = |v: Vec<String>| -> Vec<String> { v.into_iter().filter(|f| f.starts_with("versorgung_")).collect() };
    (
        nur(feld_ids(&dekl, "unvollstaendig")),
        nur(feld_ids(&dekl, "nicht_deklariert")),
        erg,
    )
}

/// Pflicht-Kegel `rentner_gesamt` (28 Felder) samt `kein_p23_verkauf = true`: Rente `rente_cent` ab 2025, `veranlagung` wie
/// angegeben. Zusammen: dazu die fuenf Kapital-Felder des Partner-Kegels auf null.
fn rentner(rente_cent: i64, veranlagung: &'static str) -> Paare {
    let mut p: Paare = vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(rente_cent)),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_grad_der_behinderung", json!(0)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(0)),
        ("rentner_gepflegter_hilflos", json!(false)),
        ("rentner_hinterbliebenenbezuege", json!(false)),
        ("veranlagung", json!(veranlagung)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
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
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
        ("kein_p23_verkauf", json!(true)),
    ];
    if veranlagung == "zusammen" {
        p.extend(partner_kap());
    }
    p
}

fn partner_kap() -> Paare {
    vec![
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ]
}

fn mit(mut basis: Paare, mehr: Paare) -> Paare {
    basis.extend(mehr);
    basis
}

/// Pflicht-Kegel `gesamt` bei Zusammenveranlagung, ohne Lohn und ohne Arbeitsweg: der Arbeitnehmer mit Versorgung.
fn gesamt_zusammen() -> Paare {
    let mut p: Paare = vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(100)),
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(0)),
        ("bruttoarbeitslohn_partner", json!(0)),
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
        ("versicherungsart_partner", json!("gesetzlich_an")),
    ];
    p.extend(partner_kap());
    p
}

/// Bezug von `euro`, Bemessungsgrundlage gleich dem Bezug, Beginn 2025, beamtenrechtlich.
fn bezug_a(euro: i64) -> Paare {
    vec![
        (A, json!(euro * 100)),
        ("versorgung_bemessungsgrundlage", json!(euro * 100)),
        ("versorgung_beginn_jahr", json!(2025)),
        ("versorgung_art", json!("beamtenrechtlich")),
    ]
}

fn bezug_b(euro: i64) -> Paare {
    vec![
        (B, json!(euro * 100)),
        ("versorgung_bemessungsgrundlage_partner", json!(euro * 100)),
        ("versorgung_beginn_jahr_partner", json!(2025)),
        ("versorgung_art_partner", json!("beamtenrechtlich")),
    ]
}

/// Der Bruttoarbeitslohn (Nr. 3 der Lohnsteuerbescheinigung) enthaelt den Bezug (Abweichung Nr. 50): ein Bezug von `euro`
/// braucht einen Lohn von mindestens `euro`, sonst sperrt der Bescheid mit `versorgung_ueber_lohn`.
fn lohn_a(euro: i64) -> Paare {
    vec![("bruttoarbeitslohn", json!(euro * 100))]
}

fn lohn_b(euro: i64) -> Paare {
    vec![("bruttoarbeitslohn_partner", json!(euro * 100))]
}

/// KONTROLLE: ohne Bezug sperrt nichts, und es gibt eine bestaetigte Zahl.
#[tokio::test]
async fn kontrolle_ohne_bezug_sperrt_nichts() {
    let d = fall_mit("rentner_gesamt", &rentner(2_000_000, "zusammen")).await;
    let (offen, _, erg) = bericht(&d).await;
    assert!(offen.is_empty(), "{offen:?}");
    assert_eq!(erg["grund"], json!("bestaetigt"), "{erg}");
}

/// KONTROLLE: ein Bezug von 0 sperrt nicht (A und B).
#[tokio::test]
async fn kontrolle_bezug_null_sperrt_nichts() {
    let p = mit(mit(rentner(2_000_000, "zusammen"), bezug_a(0)), bezug_b(0));
    let (offen, nicht, _) = bericht(&fall_mit("rentner_gesamt", &p).await).await;
    assert!(offen.is_empty(), "{offen:?}");
    assert!(nicht.contains(&A.to_owned()) && nicht.contains(&B.to_owned()), "{nicht:?}");
}

/// AK1, Rentner-Scheibe: der Bezug von Person A RECHNET (bestaetigte Zahl, hoeher als ohne Bezug) und sperrt die Abgabe; der
/// Betrag steht seit Abweichung Nr. 50 in Zeile 11 und nicht mehr in `nicht_deklariert`. Ohne die Kontrolle "rechnet" belegte das
/// Rot nichts.
#[tokio::test]
async fn person_a_rechnet_und_sperrt_auf_rentner_gesamt() {
    let ohne = bericht(&fall_mit("rentner_gesamt", &rentner(2_000_000, "zusammen")).await).await.2;
    let p = mit(mit(rentner(2_000_000, "zusammen"), lohn_a(30_000)), bezug_a(30_000));
    let (offen, nicht, erg) = bericht(&fall_mit("rentner_gesamt", &p).await).await;
    assert_eq!(erg["grund"], json!("bestaetigt"), "der Bezug rechnet: {erg}");
    assert_ne!(erg["zahl_cent"], ohne["zahl_cent"], "der Bezug aendert die Zahl nicht: {erg}");
    assert_eq!(offen, [A], "unvollstaendig: {offen:?}");
    assert!(!nicht.contains(&A.to_owned()), "der Betrag steht in Zeile 11, nicht in nicht_deklariert: {nicht:?}");
}

/// AK1, Scheibe `gesamt` (Arbeitnehmer mit Versorgung): nur Person A ist dort beantwortbar, und sie sperrt.
#[tokio::test]
async fn person_a_sperrt_auf_gesamt() {
    // Der Kegel traegt `bruttoarbeitslohn` = 0; ein zweites Event auf dasselbe Feld waere ein Fehler, also ersetzt der Lohn
    // des Bezugs den Eintrag.
    let mut kegel = gesamt_zusammen();
    kegel.retain(|(f, _)| *f != "bruttoarbeitslohn");
    let p = mit(mit(kegel, lohn_a(30_000)), bezug_a(30_000));
    let (offen, _, erg) = bericht(&fall_mit("gesamt", &p).await).await;
    assert_eq!(erg["grund"], json!("bestaetigt"), "der Bezug rechnet: {erg}");
    assert_eq!(offen, [A], "unvollstaendig: {offen:?}");
}

/// AK2: der Ehegatte sperrt bei bestaetigter Zusammenveranlagung, und sein Bezug rechnet (bestaetigte Zahl).
#[tokio::test]
async fn der_ehegatte_sperrt_bei_zusammenveranlagung() {
    let p = mit(mit(rentner(2_000_000, "zusammen"), lohn_b(30_000)), bezug_b(30_000));
    let (offen, nicht, erg) = bericht(&fall_mit("rentner_gesamt", &p).await).await;
    assert_eq!(erg["grund"], json!("bestaetigt"), "der Bezug rechnet: {erg}");
    assert_eq!(offen, [B], "unvollstaendig: {offen:?}");
    assert!(!nicht.contains(&B.to_owned()), "der Betrag steht in Zeile 11 der zweiten Anlage N: {nicht:?}");
}

/// AK2, Gegenprobe: bei Einzelveranlagung zaehlt der Bezug des Ehegatten im Bescheid nicht (gleiche Zahl mit und ohne), also
/// sperrt er die Abgabe auch nicht.
#[tokio::test]
async fn der_ehegatte_sperrt_bei_einzelveranlagung_nicht() {
    let ohne = bericht(&fall_mit("rentner_gesamt", &rentner(2_000_000, "einzel")).await).await.2;
    let p = mit(rentner(2_000_000, "einzel"), bezug_b(30_000));
    let (offen, nicht, erg) = bericht(&fall_mit("rentner_gesamt", &p).await).await;
    assert_eq!(erg["zahl_cent"], ohne["zahl_cent"], "der Bezug des Ehegatten zaehlt bei Einzel");
    assert!(offen.is_empty(), "{offen:?}");
    assert!(nicht.contains(&B.to_owned()), "{nicht:?}");
}

/// AK3: ein vorlaeufiger Betrag behaelt den Grund "Pflicht-Bestaetigung fehlt" (`GET /deklaration`); die Meldung der neuen
/// Sperre erscheint erst mit der Bestaetigung. Beginnjahr, Bemessungsgrundlage und Art stehen bestaetigt da: sonst sperrt schon
/// der Bescheid (`versorgungsfreibetrag_offen`, 409) und die Deklaration kommt nicht zustande.
#[tokio::test]
async fn ein_vorlaeufiger_betrag_behaelt_den_alten_grund() {
    let rest: Paare = bezug_a(30_000).into_iter().filter(|(f, _)| *f != A).collect();
    let d = fall_mit("rentner_gesamt", &mit(mit(rentner(2_000_000, "zusammen"), lohn_a(30_000)), rest)).await;
    vorlaeufig(&d, A, &json!(3_000_000)).await;
    let (status, dekl) = sende(&d, "GET", "/fall/vs/deklaration", None).await;
    assert_eq!(status, 200, "{dekl}");
    let eintraege: Vec<&Value> = dekl["unvollstaendig"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["feld_id"] == json!(A))
        .collect();
    assert_eq!(eintraege.len(), 1, "{dekl}");
    let grund = eintraege[0]["grund"].as_str().unwrap();
    assert!(grund.contains("Pflicht-Bestätigung"), "{grund}");
    assert!(!grund.contains("Versorgungsbezüge"), "{grund}");
}

/// AK1, `POST /einreichen`: eine vollstaendige Akte (`rust/fixtures/e2e/gesamt.json`) mit bestaetigtem Bezug wird mit 409
/// `deklaration_unvollstaendig` abgewiesen, und die Liste nennt genau dieses Feld mit einem Grund, der die Sperre und das
/// Formular nennt. Die Sperre liegt VOR dem Writer und vor `ERiC`: der Test setzt weder `ERiC` noch ein Schema voraus.
#[tokio::test]
async fn einreichen_mit_versorgungsbezug_ist_409_mit_eigenem_grund() {
    let d = dienst();
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let mut akte: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    akte["fall_id"] = json!("vs");
    akte["user_id"] = json!("alice");
    std::fs::write(
        d.zustand.konfig.faelle.join("vs.json"),
        serde_json::to_vec(&akte).unwrap(),
    )
    .unwrap();
    for (feld, wert) in bezug_a(30_000) {
        let (status, antwort) = sende(&d, "POST", "/fall/vs/event", Some(&ereignis(feld, &wert, true))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, a) = sende(&d, "POST", "/fall/vs/einreichen", Some(&json!({}))).await;
    assert_eq!(status, 409, "erwartet 409, erhalten {status}: {a}");
    assert_eq!(a["grund"], "deklaration_unvollstaendig", "{a}");
    let eintraege = a["unvollstaendig"].as_array().unwrap();
    assert_eq!(
        eintraege.iter().filter_map(|e| e["feld_id"].as_str()).collect::<Vec<_>>(),
        [A],
        "{a}"
    );
    let grund = eintraege[0]["grund"].as_str().unwrap();
    assert!(
        grund.contains("gesperrt") && grund.contains("Formular"),
        "der Grund nennt die Sperre nicht: {grund}"
    );
}
