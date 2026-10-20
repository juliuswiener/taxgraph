//! Beitraege zu Berufsverbaenden ueber HTTP (Abweichung Nr. 93): die Scheiben `an_gesamt` und `gesamt` nehmen das Feld je Person an, der Beitrag
//! zaehlt zu den Werbungskosten der Person, die Erklaerung traegt ihn in Zeile 53 der Anlage N. Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Wer Mitglied einer Gewerkschaft oder eines Berufsverbands ist, setzt den Jahresbeitrag als Werbungskosten ab (§ 9 Abs. 1
//! Satz 3 Nr. 3 `EStG`). Die App hatte dafuer kein Feld: wer mit allen Werbungskosten ueber dem Arbeitnehmer-Pauschbetrag lag, zahlte zu viel Steuer.
//!
//! **Warum es zaehlt.** Der Beitrag aendert die Steuer nur, wenn die Werbungskosten der Person zusammen den Arbeitnehmer-Pauschbetrag uebersteigen
//! (1.230 Euro im Jahr 2025). Ein Beitrag von 178 Euro (Beispiel der amtlichen Anleitung zur Anlage N) allein aendert nichts; neben einer
//! Entfernungspauschale von 600 Euro und einem Beitrag von 800 Euro stehen 1.400 Euro und die Steuer sinkt.
//!
//! **Wo es sitzt.** Das Feld in `rust/bindung/daten/bindung_an_gesamt.yaml` (`berufsverband_beitraege`, der Ehegatte mit `_partner`), die
//! Rechnung in `rust/bescheid/src/zweige/wk.rs` (EIN Rechenpfad fuer `an_gesamt`, `gesamt`), die Zeile `N/Wk/Berufsverb/Einz` mit E0204001,
//! E0204003 und der Summe E0204002 in `rust/elster/src/deklaration.rs`.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus `params/` und dem Gesetzestext gerechnet, nie aus dem Rust-Code gelesen (Skript
//! `handrechnung.py` im Auftragsordner, Ausgabe `handrechnung.out`). Je Fall: Lohn − max(Werbungskosten, Arbeitnehmer-Pauschbetrag 1.230) −
//! Basisbeitrag KV + PV − Sonderausgaben-Pauschbetrag 36 = zvE, darauf der Tarif § 32a (Zusammenveranlagung: Splitting, 2 × Steuer der Haelfte).
//! Der Arbeitnehmer-Pauschbetrag gilt je Person. Die Steuer ist die festzusetzende Einkommensteuer.
//!
//! STAND d-46 (2026-10-20): Die Personen sind `gesetzlich_an`, die App kuerzt ihren KV-Beitrag um 4 Prozent (Krankengeld-Anspruch, § 10 Abs. 1
//! Nr. 3 Buchst. a S. 4 `EStG`): der Basisbeitrag 3.600 wird zu 3.480, das zvE steigt je Person um 120 Euro. Die Zahlen unten tragen diese
//! Kuerzung (`rechnung_altfaelle.py` im Auftragsordner, geeicht: jede alte Zahl wird aus dem alten zvE zurueckgerechnet). Ohne sie waeren es
//! zvE 45.134 / Steuer 9.007 (Einzel) und 70.270 / 11.616 (zusammen); die Kommentare an den Tests nennen die alten Zahlen in Klammern.
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

const A: &str = "berufsverband_beitraege";
const B: &str = "berufsverband_beitraege_partner";

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
    let auth = Auth::neu("testgeheimnis".into(), tmp.path().join("users.json"), Some(konfig.audit_pfad()));
    let zustand = Zustand::neu(konfig, auth);
    let token = zustand.auth.stelle_aus("alice").unwrap();
    Dienst { zustand, token, _tmp: tmp }
}

async fn sende(d: &Dienst, methode: &str, pfad: &str, body: Option<&Value>) -> (u16, Value) {
    let text = body.map(ToString::to_string);
    let mut b = Request::builder().method(methode).uri(pfad).header("authorization", format!("Bearer {}", d.token));
    if let Some(t) = &text {
        b = b.header("content-type", "application/json").header("content-length", t.len().to_string());
    }
    let req = b.body(text.map_or_else(Body::empty, Body::from)).unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (teile.status.as_u16(), serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// Ein Event ueber die echte Route `POST /event` (Nutzer-Klick, bestaetigt); liefert Status und Antwort.
async fn event_post(d: &Dienst, feld: &str, wert: &Value) -> (u16, Value) {
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    sende(d, "POST", "/fall/bv/event", Some(&rumpf)).await
}

/// Ein leerer Fall der Scheibe, VZ 2025.
async fn neuer_fall(scheibe: &str) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "bv", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    d
}

/// Ein Fall der Scheibe mit allen `paare`, jedes ueber `POST /event` (muss 201 sein).
async fn fall_mit(scheibe: &str, paare: &[(&'static str, Value)]) -> Dienst {
    let d = neuer_fall(scheibe).await;
    for (feld, wert) in paare {
        let (status, antwort) = event_post(&d, feld, wert).await;
        assert_eq!(status, 201, "POST /event {feld} auf {scheibe}: {antwort}");
    }
    d
}

async fn ergebnis(scheibe: &str, paare: &[(&'static str, Value)]) -> Value {
    let d = fall_mit(scheibe, paare).await;
    let (status, antwort) = sende(&d, "GET", "/fall/bv/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

async fn deklaration(scheibe: &str, paare: &[(&'static str, Value)]) -> Value {
    let d = fall_mit(scheibe, paare).await;
    let (status, dekl) = sende(&d, "GET", "/fall/bv/deklaration", None).await;
    assert_eq!(status, 200, "GET /deklaration: {dekl}");
    dekl
}

/// Die Feld-IDs der Fragen, die der Fall noch stellt (`GET /fragen`).
async fn fragen_ids(d: &Dienst) -> Vec<String> {
    let (status, fragen) = sende(d, "GET", "/fall/bv/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    fragen["fragen"].as_array().unwrap().iter().map(|q| q["feld_id"].as_str().unwrap().to_owned()).collect()
}

fn zahl(a: &Value) -> i64 {
    assert_eq!(a["grund"], json!("bestaetigt"), "keine Zahl: {a}");
    a["zahl_cent"].as_i64().unwrap_or_else(|| panic!("keine Zahl: {a}"))
}

/// Ersetzt den Wert von `feld`; fehlt es, haengt es an.
fn mit(mut paare: Paare, feld: &'static str, wert: Value) -> Paare {
    match paare.iter_mut().find(|(f, _)| *f == feld) {
        Some(p) => p.1 = wert,
        None => paare.push((feld, wert)),
    }
    paare
}

fn plus(mut paare: Paare, mehr: Paare) -> Paare {
    paare.extend(mehr);
    paare
}

/// Pflicht-Kegel `an_gesamt` (33 Felder): Arbeitnehmer ohne Arbeitsweg, einzel, alle vier Kreuze "nein", Lohn und Beitraege wie angegeben
/// (Euro), gesetzlich versichert als Arbeitnehmer.
fn an(lohn: i64, kv: i64, pv: i64) -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(lohn * 100)),
        ("veranlagung", json!("einzel")),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!("gesetzlich_an")),
        ("basis_kv", json!(kv * 100)),
        ("basis_pv", json!(pv * 100)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(true)),
        ("dhf_unterkunftskosten_monat", json!(0)),
        ("dhf_monate", json!(0)),
        ("dhf_im_inland", json!(false)),
        ("dhf_beruflich_veranlasst", json!(false)),
        ("dhf_eigener_hausstand", json!(false)),
        ("dhf_finanzielle_beteiligung", json!(false)),
        ("tage_24h", json!(0)),
        ("tage_an_abreise", json!(0)),
        ("tage_ueber_8h_eintaegig", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("fam_anzahl_kinder", json!(0)),
        ("verlustvortrag_bestand", json!(0)),
    ]
}

/// Pflicht-Kegel `gesamt` (35 Felder), einzel, alle Kreuze "nein"/"kein"; Lohn und Beitraege wie `an`.
fn gesamt(lohn: i64, kv: i64, pv: i64) -> Paare {
    vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(100)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(lohn * 100)),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!("gesetzlich_an")),
        ("basis_kv", json!(kv * 100)),
        ("basis_pv", json!(pv * 100)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(true)),
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

/// Der Ehegatte: Lohn und Beitraege (Euro), gesetzlich als Arbeitnehmer; die fuenf Kapital-Felder des Partner-Kegels von `gesamt` stehen nur
/// dort (`mit_kap`).
fn partner(lohn: i64, kv: i64, pv: i64, mit_kap: bool) -> Paare {
    let mut p: Paare = vec![
        ("bruttoarbeitslohn_partner", json!(lohn * 100)),
        ("versicherungsart_partner", json!("gesetzlich_an")),
        ("basis_kv_partner", json!(kv * 100)),
        ("basis_pv_partner", json!(pv * 100)),
        ("mit_anspruch_auf_zuschuss_partner", json!(true)),
    ];
    if mit_kap {
        p.extend([
            ("kap_kapitalertraege_partner", json!(0)),
            ("kap_gewinn_aktien_partner", json!(0)),
            ("kap_gewinn_sonstige_partner", json!(0)),
            ("kap_verlust_aktien_partner", json!(0)),
            ("kap_verlust_sonstige_partner", json!(0)),
        ]);
    }
    p
}

/// Zusammenveranlagung in `an_gesamt`: Person A und der Ehegatte mit je Lohn 40.001 und Beitrag 3.600 Euro (KV 3.000 + PV 600), dazu die
/// Beitragsfelder `a` (Person A) und `b` (Ehegatte).
fn an_paar(a: Paare, b: Paare) -> Paare {
    let basis = mit(an(40_001, 3_000, 600), "veranlagung", json!("zusammen"));
    plus(plus(plus(basis, partner(40_001, 3_000, 600, false)), a), b)
}

/// Dasselbe in `gesamt` (dort mit den fuenf Kapital-Feldern des Ehegatten).
fn gesamt_paar(a: Paare, b: Paare) -> Paare {
    let basis = mit(gesamt(40_001, 3_000, 600), "veranlagung", json!("zusammen"));
    plus(plus(plus(basis, partner(40_001, 3_000, 600, true)), a), b)
}

/// Die Scheiben, die Werbungskosten rechnen: `(Name, Fall mit Lohn 50.000 und Basisbeitrag 3.600 Euro)`.
fn einzel_faelle() -> [(&'static str, Paare); 2] {
    [("an_gesamt", an(50_000, 3_000, 600)), ("gesamt", gesamt(50_000, 3_000, 600))]
}

/// Das Feld A in Euro und Cent: `(Betrag in Cent)`.
fn beitrag(cent: i64) -> Paare {
    vec![(A, json!(cent))]
}

/// Entfernungspauschale: 200 Arbeitstage zu 10 km, 0,30 Euro je Kilometer und Tag = 600 Euro.
fn pendeln(basis: Paare) -> Paare {
    mit(mit(basis, "ep_arbeitstage", json!(200)), "ep_entfernung_km", json!(10))
}

/// `AK1` (a): beide Felder sind auf `an_gesamt` und `gesamt` beantwortbar, `POST /event` antwortet 201; das des Ehegatten nur dort, wo er
/// vorkommt (nicht `rentner_gesamt`, die keine Werbungskosten fragt, und nicht `n_vor_gwg`, die keinen Gesamtbescheid und keinen Lohn fuehrt).
/// Vorher: 400 „nicht in dieser Scheibe“.
#[tokio::test]
async fn die_beitragsfelder_sind_auf_den_scheiben_mit_werbungskosten_beantwortbar() {
    for scheibe in ["an_gesamt", "gesamt"] {
        let d = neuer_fall(scheibe).await;
        for feld in [A, B] {
            let (status, antwort) = event_post(&d, feld, &json!(17_800)).await;
            assert_eq!(status, 201, "POST /event {feld} auf {scheibe}: {antwort}");
        }
    }
    for scheibe in ["rentner_gesamt", "n_vor_gwg"] {
        let d = neuer_fall(scheibe).await;
        for feld in [A, B] {
            let (status, _) = event_post(&d, feld, &json!(17_800)).await;
            assert_eq!(status, 400, "KONTROLLE: {feld} auf {scheibe} ist dort nicht gefragt");
        }
    }
}

/// `AK1` (i) und (iv), `an_gesamt` und `gesamt`, alleinstehend: ein Beitrag allein unter dem Pauschbetrag aendert die Steuer nicht. Lohn 50.000,
/// Basisbeitrag 3.600 (gekuerzt 3.480): zvE 45.254, Steuer 9.047 Euro (Handrechnung E0; vor d-46 zvE 45.134, 9.007). Ohne Beitrag, mit Beitrag 0,
/// mit 178 Euro (Beispiel der Anleitung zur Anlage N), mit genau 1.230 Euro (= Pauschbetrag), nur Entfernungspauschale 600 und nur Beitrag 800:
/// immer 9.047.
#[tokio::test]
async fn ein_beitrag_unter_dem_pauschbetrag_aendert_die_steuer_nicht() {
    for (scheibe, basis) in einzel_faelle() {
        for (name, paare) in [
            ("ohne Feld", basis.clone()),
            ("Beitrag 0", plus(basis.clone(), beitrag(0))),
            ("Beitrag 178", plus(basis.clone(), beitrag(17_800))),
            ("Beitrag 1.230, genau der Pauschbetrag", plus(basis.clone(), beitrag(123_000))),
            ("nur Entfernungspauschale 600", pendeln(basis.clone())),
            ("nur Beitrag 800", plus(basis.clone(), beitrag(80_000))),
        ] {
            let e = ergebnis(scheibe, &paare).await;
            assert_eq!(zahl(&e), 9_047 * 100, "{scheibe}, {name}: {e}");
        }
    }
}

/// `AK1` (ii), `an_gesamt` und `gesamt`: liegen die Werbungskosten ueber dem Pauschbetrag, sinkt die Steuer um den Beitrag. Beitrag 1.500:
/// zvE 44.984, Steuer 8.956 Euro (Handrechnung E3; vor d-46 zvE 44.864, 8.916). Der Beitrag ZAEHLT ZU der Entfernungspauschale: 600 + 800 =
/// 1.400, zvE 45.084, Steuer 8.990 Euro (E6; vorher 44.964, 8.949), obwohl jeder der beiden Posten allein unter dem Pauschbetrag liegt.
/// Cent-Rest: 1.500,01 Euro rechnen und erklaeren als 1.501 Euro (aufgerundet), zvE 44.983, Steuer 8.956 (E7; vorher 44.863, 8.915); 1.500,00
/// Euro bleibt 8.956 (E8).
#[tokio::test]
async fn ein_beitrag_ueber_dem_pauschbetrag_senkt_die_steuer_und_zaehlt_zur_entfernungspauschale() {
    for (scheibe, basis) in einzel_faelle() {
        for (name, paare, steuer) in [
            ("Beitrag 1.500", plus(basis.clone(), beitrag(150_000)), 8_956),
            ("Entfernungspauschale 600 + Beitrag 800", plus(pendeln(basis.clone()), beitrag(80_000)), 8_990),
            ("Beitrag 1.500,01, aufgerundet 1.501", plus(basis.clone(), beitrag(150_001)), 8_956),
            ("Beitrag 1.500,00", plus(basis.clone(), beitrag(150_000)), 8_956),
        ] {
            let e = ergebnis(scheibe, &paare).await;
            assert_eq!(zahl(&e), steuer * 100, "{scheibe}, {name}: {e}");
        }
    }
}

/// `AK1` (iii), Zusammenveranlagung, `an_gesamt` und `gesamt`: der Arbeitnehmer-Pauschbetrag gilt JE PERSON. Lohn je 40.001, Basisbeitrag je
/// 3.600 (je gekuerzt 3.480). Ohne Beitrag zvE 70.510, Steuer 11.690 Euro (Z0; vor d-46 70.270, 11.616). Beitrag 2.000 nur beim Ehegatten: Abzug
/// 1.230 + 2.000, zvE 69.740, Steuer 11.456 (Z1; vorher 69.500, 11.384); nur bei Person A, vertauscht: dieselbe Steuer (Z2); bei beiden: zvE
/// 68.970, Steuer 11.226 (Z3; vorher 68.730, 11.154); nur 178 beim Ehegatten: wie ohne (Z4).
#[tokio::test]
async fn bei_zusammenveranlagung_zaehlt_der_beitrag_je_person_fuer_sich() {
    for (scheibe, bauen) in [("an_gesamt", an_paar as fn(Paare, Paare) -> Paare), ("gesamt", gesamt_paar)] {
        let keiner = ergebnis(scheibe, &bauen(vec![], vec![])).await;
        let nur_b = ergebnis(scheibe, &bauen(vec![], vec![(B, json!(200_000))])).await;
        let nur_a = ergebnis(scheibe, &bauen(vec![(A, json!(200_000))], vec![])).await;
        let beide = ergebnis(scheibe, &bauen(vec![(A, json!(200_000))], vec![(B, json!(200_000))])).await;
        let b_klein = ergebnis(scheibe, &bauen(vec![], vec![(B, json!(17_800))])).await;
        assert_eq!(zahl(&keiner), 11_690 * 100, "{scheibe}, keiner: {keiner}");
        assert_eq!(zahl(&nur_b), 11_456 * 100, "{scheibe}, nur der Ehegatte: {nur_b}");
        assert_eq!(zahl(&nur_a), 11_456 * 100, "{scheibe}, nur Person A: {nur_a}");
        assert_eq!(zahl(&beide), 11_226 * 100, "{scheibe}, beide: {beide}");
        assert_eq!(zahl(&b_klein), 11_690 * 100, "{scheibe}, nur der Ehegatte mit 178 Euro: {b_klein}");
    }
}

/// Das Feld des Ehegatten gilt nur bei bestaetigter Zusammenveranlagung: bei Einzelveranlagung liest die Rechnung es nicht (wie jedes
/// `_partner`-Feld), die Steuer bleibt die ohne Beitrag.
#[tokio::test]
async fn bei_einzelveranlagung_zaehlt_das_feld_des_ehegatten_nicht() {
    for (scheibe, basis) in einzel_faelle() {
        let e = ergebnis(scheibe, &plus(basis, vec![(B, json!(200_000))])).await;
        assert_eq!(zahl(&e), 9_047 * 100, "{scheibe}: {e}");
    }
}

/// `AK4`: die Frage steht erst bei bestaetigtem Lohn ueber 0 (Person A) und nur bei Zusammenveranlagung mit Lohn des Ehegatten ueber 0 (Ehegatte).
/// Keine Pflichtfrage: die Zahl steht auch ohne Antwort. Positivkontrolle: mit Lohn steht sie.
#[tokio::test]
async fn die_frage_steht_nur_mit_lohn_der_person() {
    let ohne_lohn = fragen_ids(&neuer_fall("an_gesamt").await).await;
    assert!(!ohne_lohn.is_empty(), "KONTROLLE: der Katalog ist nicht leer");
    assert!(!ohne_lohn.iter().any(|i| i == A), "{A} steht ohne Lohn: {ohne_lohn:?}");
    let mit_lohn = fragen_ids(&fall_mit("an_gesamt", &an(50_000, 3_000, 600)).await).await;
    assert!(mit_lohn.iter().any(|i| i == A), "{A} fehlt bei Lohn ueber 0: {mit_lohn:?}");
    assert!(!mit_lohn.iter().any(|i| i == B), "{B} steht bei Einzelveranlagung: {mit_lohn:?}");
    let zusammen = fragen_ids(&fall_mit("an_gesamt", &an_paar(vec![], vec![])).await).await;
    for feld in [A, B] {
        assert!(zusammen.iter().any(|i| i == feld), "{feld} fehlt bei Zusammenveranlagung mit Lohn beider: {zusammen:?}");
    }
    let b_ohne_lohn = mit(an_paar(vec![], vec![]), "bruttoarbeitslohn_partner", json!(0));
    let ids = fragen_ids(&fall_mit("an_gesamt", &b_ohne_lohn).await).await;
    assert!(ids.iter().any(|i| i == A), "{A} fehlt, obwohl nur der Ehegatte ohne Lohn ist: {ids:?}");
    assert!(!ids.iter().any(|i| i == B), "{B} steht ohne Lohn des Ehegatten: {ids:?}");
}

/// Der Schreibweg nimmt nur einen Betrag von 0 bis 99.999 Euro an: die Erklaerung kennt hoechstens fuenf Stellen (`E0204003` und `E0204002`,
/// `E10-2025.xsd`: `GanzzahlOhneFuehrNull_MaxVK5`), ein Minus kennt sie bei der Summe nicht. 99.999,00 Euro gehen durch, 99.999,01 und mehr
/// und jede negative Zahl nicht (422).
#[tokio::test]
async fn der_schreibweg_nimmt_nur_null_bis_99999_euro_an() {
    for (cent, erwartet) in [(0, 201), (17_800, 201), (9_999_900, 201), (9_999_901, 422), (10_000_000, 422), (-1, 422)] {
        let d = neuer_fall("an_gesamt").await;
        let (status, antwort) = event_post(&d, A, &json!(cent)).await;
        assert_eq!(status, erwartet, "{cent} Cent: {antwort}");
    }
}

/// `AK2`: die Erklaerung traegt die Zeile 53 der Anlage N (`N/Wk/Berufsverb`): die Bezeichnung `E0204001`, den Betrag `E0204003` in vollen
/// Euro AUFGERUNDET und die Summe `E0204002` (= Summe der gerundeten Zeilen). Person A in `deklaration`, der Ehegatte in `person_b`. Ohne
/// Beitrag oder mit 0 steht keines der drei Kz in der Erklaerung.
#[tokio::test]
async fn die_erklaerung_traegt_zeile_53_je_person_aufgerundet() {
    let zusammen = |a: Paare, b: Paare| gesamt_paar(a, b);
    let mit_beitrag = deklaration("gesamt", &zusammen(vec![(A, json!(150_001))], vec![(B, json!(17_800))])).await;
    assert_eq!(mit_beitrag["deklaration"]["E0204003"], json!(1_501), "Person A, Betrag aufgerundet: {mit_beitrag}");
    assert_eq!(mit_beitrag["deklaration"]["E0204002"], json!(1_501), "Person A, Summe: {mit_beitrag}");
    assert!(
        mit_beitrag["deklaration"]["E0204001"].as_str().is_some_and(|s| !s.trim().is_empty()),
        "Person A, Bezeichnung: {mit_beitrag}"
    );
    assert_eq!(mit_beitrag["person_b"]["E0204003"], json!(178), "Ehegatte, Betrag: {mit_beitrag}");
    assert_eq!(mit_beitrag["person_b"]["E0204002"], json!(178), "Ehegatte, Summe: {mit_beitrag}");
    assert!(mit_beitrag["person_b"]["E0204001"].as_str().is_some_and(|s| !s.trim().is_empty()), "Ehegatte, Bezeichnung: {mit_beitrag}");
    for ohne in [zusammen(vec![], vec![]), zusammen(vec![(A, json!(0))], vec![(B, json!(0))])] {
        let d = deklaration("gesamt", &ohne).await;
        for kz in ["E0204001", "E0204002", "E0204003"] {
            assert!(d["deklaration"].get(kz).is_none_or(Value::is_null), "{kz} ohne Beitrag in Person A: {d}");
            assert!(d["person_b"].get(kz).is_none_or(Value::is_null), "{kz} ohne Beitrag im Ehegatten: {d}");
        }
    }
}

/// `AK4`: Frage- und Hilfetext nennen, was zaehlt (Gewerkschaft, Berufsverband), was nicht (politische Parteien) und dass der Beitrag zu den
/// uebrigen Werbungskosten zaehlt; ein Satz zu den Kammern (`Kammerbeitrag/Wert`, Rechtslage ungeklaert) steht nicht darin. Beide Felder.
#[test]
fn die_texte_nennen_was_zaehlt_und_was_nicht() {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let reg = bindung::lade_registry_der_wurzel(&wurzel).unwrap();
    let alle: Vec<&bindung::Bindung> = reg.dateien.iter().flat_map(|(_, d)| d.bindungen.iter()).collect();
    assert!(alle.len() > 300, "KONTROLLE: nur {} Felder gelesen", alle.len());
    for fid in [A, B] {
        let b = alle.iter().find(|b| b.feld_id == fid).unwrap_or_else(|| panic!("Feld {fid} fehlt in der Registry"));
        let frage = b.fragetext_laie.clone().unwrap_or_default();
        assert!(frage.contains("Gewerkschaft") && frage.contains("Berufsverband"), "{fid}: Fragetext: {frage}");
        for muss in ["Parteien", "Werbungskosten", "Pauschbetrag", "Zeile 53"] {
            assert!(b.hilfe_kurz.contains(muss), "{fid}: Hilfetext nennt {muss:?} nicht: {}", b.hilfe_kurz);
        }
        let alles = format!("{frage} {}", b.hilfe_kurz);
        for darf_nicht in ["Kammer", "Arbeitnehmerkammer"] {
            assert!(!alles.contains(darf_nicht), "{fid}: der Text sagt etwas zu {darf_nicht:?}, die Rechtslage ist ungeklaert");
        }
    }
}

/// `AK3`: keines der zwei Felder bietet einen Vorschlag aus dem Beleg an. Die Lohnsteuerbescheinigung und der Abruf der Finanzverwaltung
/// kennen keinen Beitrag zu einem Berufsverband (`Kammerbeitrag/Wert` bleibt in `NICHT_UNTERSUCHT`, `api/tests/vast_naht_bindung.rs`):
/// ein `vorschlagbar_von` liesse die Oberflaeche einen Wert anbieten, den keine Quelle liefert.
#[test]
fn die_felder_bieten_keinen_beleg_vorschlag_an() {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let reg = bindung::lade_registry_der_wurzel(&wurzel).unwrap();
    let alle: Vec<&bindung::Bindung> = reg.dateien.iter().flat_map(|(_, d)| d.bindungen.iter()).collect();
    assert!(alle.len() > 300, "KONTROLLE: nur {} Felder gelesen", alle.len());
    assert!(
        alle.iter().any(|b| !b.vorschlagbar_von.is_empty()),
        "KONTROLLE: kein Feld der Registry bietet einen Vorschlag an, die Pruefung unten sah nichts"
    );
    for fid in [A, B] {
        let b = alle.iter().find(|b| b.feld_id == fid).unwrap_or_else(|| panic!("Feld {fid} fehlt in der Registry"));
        assert_eq!(b.vorschlagbar_von.len(), 0, "{fid}: bietet einen Vorschlag aus dem Beleg an: {:?}", b.vorschlagbar_von);
    }
}
