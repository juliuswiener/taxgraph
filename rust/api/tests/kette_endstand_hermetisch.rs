//! Rechenweg-Kette aus dem ENDSTAND (p24a): die letzte Stufe von `GET /ergebnis` -> `kette` ist die
//! gezahlte Steuer, auch dort, wo eine Korrektur AUSSERHALB der Engine sitzt (§ 32d-Abgeltungsteuer,
//! § 32b-Zuschlag) -- im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! Vor p24a speiste `rahmen` die Kette aus dem Basis-`g` (Vor-Korrektur-Stand); `setze_kette` verwarf sie
//! dann still, `kette` war `null` (bei 30.000 EUR Kapitalertraegen endete sie bei 13.924 statt 21.174 EUR,
//! -7.250 EUR). Die Paritaets-Suiten hielten das nur mit `PARITY=1`; die CI faehrt Parity nicht.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl unten ist die Ausgabe des Python-Servers (HEAD 0aa91677 +
//! p24a-Fix, 2026-10-04, `GET /fall/{id}/ergebnis` mit denselben Events ueber `POST /event`; die
//! Ereignislisten stammen aus `tests/_kegel.py::kegel_fuer`, erzeugt von `rechenweg/gen_kette_rs.py`).
//! Kein Wert ist aus dem Rust-Code abgelesen.
//!
//! ponytail: die Erwartungswerte sind eingefroren. Aendert sich der Tarif oder der Kegel, rechnet man
//! sie am Python-Server nach und zieht die Konstanten nach.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::path::Path;

use api::konfig::Konfig;
use api::{Zustand, app};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

type Paare = Vec<(&'static str, Value)>;

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

/// Legt den Fall der Scheibe an, schreibt jedes Paar ueber die echte Route `POST /event` (als Nutzer-Klick,
/// bestaetigt) und liefert `GET /ergebnis`.
async fn ergebnis(scheibe: &str, paare: &[(&'static str, Value)]) -> Value {
    let d = dienst();
    let kopf = json!({"fall_id": "kette", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let rumpf = json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (status, antwort) = sende(&d, "POST", "/fall/kette/event", Some(&rumpf)).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, antwort) = sende(&d, "GET", "/fall/kette/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// Basis plus Aenderungen: ein Feld, das die Basis schon traegt, wird ersetzt (EIN Event je Feld, kein
/// Ueberschreiben), ein neues angehaengt -- wie `tests/_kegel.py::kegel_fuer` mit dem gesetzt-Dict.
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

/// Die vier Stufen einer Kette, EURO, in der Reihenfolge der Oberflaeche.
fn stufen(a: &Value) -> [i64; 4] {
    let k = &a["kette"];
    [
        "gesamtbetrag_der_einkuenfte",
        "zu_versteuerndes_einkommen",
        "tarifliche_est",
        "festzusetzende_est",
    ]
    .map(|s| k[s].as_i64().unwrap_or(i64::MIN))
}

/// Die Kette steht da, ihre letzte Stufe IST die gezahlte Steuer, und die Stufen stimmen mit dem Python-Lauf.
fn erwarte_kette(name: &str, a: &Value, zahl_cent: i64, soll: [i64; 4]) {
    assert_eq!(
        a["grund"], "bestaetigt",
        "{name}: Positivkontrolle -- Kegel vollstaendig? {a}"
    );
    assert_eq!(
        a["zahl_cent"].as_i64(),
        Some(zahl_cent),
        "{name}: Zahl -- {a}"
    );
    assert!(
        !a["kette"].is_null(),
        "{name}: kette fehlt (null) -- `setze_kette` hat die Vor-Korrektur-Kette verworfen; die Kette \
         muss aus dem Endstand kommen und bei {zahl_cent} ct enden. Antwort: {a}"
    );
    assert_eq!(
        stufen(a),
        soll,
        "{name}: Stufen GdE/zvE/tariflich/festzusetzende -- {a}"
    );
    assert_eq!(
        soll[3] * 100,
        zahl_cent,
        "{name}: letzte Stufe muss die gezahlte Steuer sein"
    );
}

// ---- Basisfaelle (volle Kegel, aus tests/_kegel.py::kegel_fuer) ------------------------------------

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

fn kegel_rentner() -> Paare {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(2_000_000)),
        ("rentner_renten_beginn_jahr", json!(2_025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_grad_der_behinderung", json!(0)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(0)),
        ("rentner_gepflegter_hilflos", json!(false)),
        ("rentner_hinterbliebenenbezuege", json!(false)),
        ("veranlagung", json!("einzel")),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(false)),
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
        ("rentner_rentenfreibetrag", json!(0)),
        ("kap_kapitalertraege", json!(3_000_000)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
    ]
}

fn g1_p32d_kapital_aenderungen() -> Paare {
    vec![
        ("kap_kapitalertraege", json!(3_000_000)),
        ("kein_kap", json!(false)),
    ]
}

fn g2_p35_gewerbesteuer_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(5_000_000)),
        ("gewinn_betriebsart", json!("gewerbe")),
        ("gewst_messbetrag", json!(150_000)),
        ("gewst_hebesatz", json!(400)),
    ]
}

fn g3_kind_kindergeld_aenderungen() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(2_000_000)),
        ("fam_anzahl_kinder", json!(1)),
    ]
}

fn g4_kind_freibetrag_p32d_aenderungen() -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(30_000_000)),
        ("kap_kapitalertraege", json!(3_000_000)),
        ("kein_kap", json!(false)),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("fam_anzahl_kinder", json!(1)),
    ]
}

fn r2_rentner_kind_p32d_aenderungen() -> Paare {
    vec![("fam_anzahl_kinder", json!(1))]
}

// ---- Tests -----------------------------------------------------------------------------------------

/// Gegenprobe ohne Sonderregel: die Kette lief schon vor p24a und muss unveraendert laufen.
#[tokio::test]
async fn g0_gegenprobe_lohn_60k_kette_endet_bei_der_zahl() {
    let a = ergebnis("gesamt", &kegel_gesamt()).await;
    erwarte_kette("g0", &a, 1_392_400, [58_770, 58_734, 13_924, 13_924]);
    assert!(a["kette"]["p31"].is_null(), "kinderlos: kein p31");
}

/// § 32d, 30.000 EUR Kapitalertraege: die Abgeltungsteuer (+7.250 EUR) sitzt AUSSERHALB der Engine
/// (`result = est_raw + kap_st_k`). Tarifliche Stufe bleibt 13.924 EUR (Kapital laeuft nicht in den
/// tariflichen zvE, § 2 Abs. 5b), nur die letzte Stufe traegt den Zuschlag: 21.174 statt 13.924 EUR.
#[tokio::test]
async fn g1_p32d_kapital_kette_traegt_die_abgeltungsteuer() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g1_p32d_kapital_aenderungen()),
    )
    .await;
    erwarte_kette("g1", &a, 2_117_400, [58_770, 58_734, 13_924, 21_174]);
}

/// § 35 `EStG`: 50.000 EUR Gewerbegewinn, Messbetrag 1.500, Hebesatz 400 -- die Anrechnung steckt in
/// `steuerermaessigungen` des finalen `g2`; die Kette muss sie sehen (Stufe `festzusetzende` 28.756 EUR).
#[tokio::test]
async fn g2_p35_gewerbesteuer_kette_endet_bei_der_zahl() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g2_p35_gewerbesteuer_aenderungen()),
    )
    .await;
    erwarte_kette("g2", &a, 2_875_600, [108_770, 108_734, 34_756, 28_756]);
}

/// § 31 mit einem Kind, Kindergeld gewinnt: der fb=0-Lauf bestimmt die Steuer und muss seinen eigenen
/// Endstand hinterlassen -- `SolzInfo` traegt nur den Kinderfreibetrag-Lauf. `p31` nennt den Sieger.
#[tokio::test]
async fn g3_kind_kindergeld_siegt_kette_kommt_aus_dem_ohne_freibetrag_lauf() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g3_kind_kindergeld_aenderungen()),
    )
    .await;
    erwarte_kette("g3", &a, 132_700, [18_770, 18_734, 1_327, 1_327]);
    assert_eq!(a["kette"]["p31"]["guenstiger"], "kindergeld", "{a}");
    assert_eq!(a["kette"]["p31"]["kindergeld"].as_i64(), Some(3060), "{a}");
}

/// § 31 mit einem Kind, der Kinderfreibetrag gewinnt (300.000 EUR, Zusammenveranlagung) UND § 32d: der
/// Kinderfreibetrag-Lauf hinterlaesst seinen Endstand mit der Abgeltungsteuer (+7.000 EUR), das Kindergeld
/// ist darin hinzugerechnet. Vor p24a verwarf `setze_kette` die Kette hier still (`null`).
#[tokio::test]
async fn g4_kind_freibetrag_p32d_kette_kommt_aus_dem_freibetrag_lauf() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g4_kind_freibetrag_p32d_aenderungen()),
    )
    .await;
    erwarte_kette("g4", &a, 10_965_600, [298_770, 289_098, 99_596, 109_656]);
    assert_eq!(a["kette"]["p31"]["guenstiger"], "freibetraege", "{a}");
    assert_eq!(a["kette"]["p31"]["kindergeld"].as_i64(), Some(3060), "{a}");
}

/// Rentner-Zweig, 20.000 EUR Rente + 30.000 EUR Kapitalertraege: 1:1 gesamt-Praezedenz, eigener
/// Schliesser in `rentner_tarif::festzusetzende` (Kapital VOR § 32b, ein einziger Rueckgabepunkt).
#[tokio::test]
async fn r1_rentner_p32d_kette_traegt_die_abgeltungsteuer() {
    let a = ergebnis("rentner_gesamt", &kegel_rentner()).await;
    erwarte_kette("r1", &a, 806_100, [16_598, 16_562, 811, 8_061]);
}

/// Rentner-Zweig mit Kind (Kindergeld gewinnt) UND § 32d: der Kinderzweig des Rentner-Zweigs speist die
/// Kette ebenfalls aus dem Endstand des fb=0-Laufs, nicht aus dem Rentner-`g`. `p31` nennt den Sieger.
#[tokio::test]
async fn r2_rentner_kind_p32d_kette_kommt_aus_dem_endstand() {
    let a = ergebnis(
        "rentner_gesamt",
        &mit(kegel_rentner(), r2_rentner_kind_p32d_aenderungen()),
    )
    .await;
    erwarte_kette("r2", &a, 806_100, [16_598, 16_562, 811, 8_061]);
    assert_eq!(a["kette"]["p31"]["guenstiger"], "kindergeld", "{a}");
    assert_eq!(a["kette"]["p31"]["kindergeld"].as_i64(), Some(3060), "{a}");
}
