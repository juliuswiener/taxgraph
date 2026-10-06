//! § 23 `EStG`: die zwei Waechter gegen den 21.000-EUR-Fehler im Standardlauf (ohne `PARITY=1`, ohne Python).
//!
//! Ein Rentner mit privatem Grundstuecksverkauf (Gewinn 50.000 EUR) zahlt 21.000,00 EUR zu viel
//! festgesetzte Einkommensteuer, wenn weder `flag_konsistenz_offen` noch `einkunftsart_nicht_ring_faehig`
//! greift (Vault `backlog/taxgraph/p23-sonstige-kreuz-kopplung-rentner-reachable`, Audit
//! `p23-gesperrt-statt-falsch-gerechnet`). Zwei voneinander unabhaengige Waechter fangen den Fall:
//!
//! - `FLAG_NEGIERT["kein_p23_verkauf"]` (`rust/konsistenz/src/flag.rs`): Kreuz nie beantwortet oder
//!   "kein Verkauf" trotz Betraegen -> `flag_konsistenz_offen`;
//! - `fremd_arten` enthaelt `kein_p23_verkauf` (`rust/bescheid/src/deklaration.rs`, Scheiben `gesamt`
//!   und `rentner_gesamt`): Kreuz bejaht -> `einkunftsart_nicht_ring_faehig`.
//!
//! Beide Rust-Waechter sind da und tragen. Ihre Gegenprobe lief bisher nur im Differenz-Harness
//! (`rust/parity/tests/bescheid_deklaration_paritaet.rs`, `api_http_paritaet.rs`) und damit nur mit
//! `PARITY=1`; die CI faehrt Parity nicht. Gemessen am 2026-10-03 auf f80187db (`cargo test --workspace
//! --exclude parity --no-fail-fast`, ohne `PARITY`): alle drei Mutanten lassen die Suite gruen
//! (1185 passed, 0 failed, 21 ignored) -- M1 Schluessel `kein_p23_verkauf` in `FLAG_NEGIERT` umbenannt,
//! M2a `fremd_arten` von `rentner_gesamt` ohne `kein_p23_verkauf`, M2b dasselbe fuer `gesamt`.
//! Mit diesen Tests werden sie rot (Namen: Bericht `p23-rust-waechter`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl und jeder Grund unten ist die Ausgabe des Python-Servers
//! (`server.py`, HEAD 57944f48, 2026-10-03, Sonde `sonde_angepasst.py`, dieselben Events ueber
//! `POST /fall/{id}/event`): R0 5.917.000, Z0 4.824.200, G0 7.255.600 Cent. Kein Wert ist aus dem
//! Rust-Code abgelesen. Dieselben Zahlen stehen im Vault-Eintrag (59.170,00 EUR / 72.556,00 EUR).
//! Ohne Waechter kaeme je Verkaufsfall Baseline + 2.100.000 Cent (R1 8.017.000, G1 9.355.600; Python-
//! Mutationsproben M1/M2a/M2b, gleicher Tag); die Fehlermeldung dieser Tests rechnet diese Differenz aus.
//!
//! Die Baselines R0/Z0/G0 sind die Positivkontrolle: sie zeigen, dass der Kegel vollstaendig ist und die
//! Engine eine Zahl liefert. Ohne sie waere "gesperrt" nicht von "Kegel offen" zu unterscheiden.
//!
//! ZAEHL-INSTANZ `__2`: ein zweiter Verkauf steht nur unter `p23_*__2` (kein Feld unter dem Basisnamen).
//! `FLAG_NEGIERT` findet ihn ueber `instanz_feld_ids_text` (Basis plus `basis__<n>`); sonst liefe ein
//! Verkauf in der zweiten Zaehl-Instanz still durch. R4/R5 (`rentner_gesamt`) sind die Python-Werte aus
//! derselben Sonde (`flag_konsistenz_offen`, beide ohne Zahl). R6 und G5-G7 haben keinen eigenen Python-Lauf:
//! G5/G6 sind G3/G4 der Sonde mit `__2` statt Basisnamen, R6/G7 laufen ueber `fremd_arten`, das nur das
//! Kreuz liest und kein p23-Feld. Gemessen am 2026-10-07 auf 26e9395a: `instanz_feld_ids_text` ohne den
//! Instanz-Zweig und `ist_instanz_suffix` ohne die Ziffer 2 machen genau R4, R5, G5, G6 rot; die zehn
//! Tests mit Basisnamen bleiben gruen. Die Unit-Tests in `konsistenz` fangen beide Mutanten auf
//! Funktionsebene; neu ist der Pin ueber die Route (`GET /ergebnis`).
//!
//! ponytail: die Erwartungswerte sind eingefroren. Faellt Python (Orakel) weg und aendert sich der
//! Tarif oder der Kegel, rechnet man R0/Z0/G0 von Hand nach und zieht die Konstanten nach; die
//! Differenz 2.100.000 Cent (42 % von 50.000 EUR, Zone 4) gilt nur fuer diesen Fall.
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

/// Baseline Rentner einzel (Kreuz "kein Verkauf", kein p23): 59.170,00 EUR festzusetzende `ESt`.
const R0: i64 = 5_917_000;
/// Baseline Rentner zusammen (Person A: Rente 20.000 EUR, Partner nur KAP-Nullen).
const Z0: i64 = 4_824_200;
/// Baseline `gesamt` (AN 200.000 EUR, Kreuz `kein_sonstige` und `kein_p23_verkauf` auf "nein").
const G0: i64 = 7_255_600;

const FLAG_NIE_GEFRAGT: &str = "flag_konsistenz_offen";
const FLAG_BEJAHT: &str = "einkunftsart_nicht_ring_faehig";

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

/// Legt einen Fall der Scheibe an, schreibt jedes Paar ueber die echte Route `POST /event` (als
/// Nutzer-Klick, bestaetigt) und liefert `GET /ergebnis`.
async fn ergebnis(scheibe: &str, paare: &[(&'static str, Value)]) -> Value {
    let d = dienst();
    let kopf = json!({"fall_id": "p23", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let rumpf = json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (status, antwort) = sende(&d, "POST", "/fall/p23/event", Some(&rumpf)).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, antwort) = sende(&d, "GET", "/fall/p23/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// Pflicht-Kegel `rentner_gesamt` (29 Felder ohne das Kreuz `kein_p23_verkauf`), Rente 20.000 EUR ab 2025,
/// `kein_sonstige = false` (die eigene Rente ist die ehrliche Antwort). Aus `tests/_kegel.py::kegel_fuer`
/// (2026-10-03), Abwesenheitswerte.
fn kegel_rentner(veranlagung: &'static str) -> Paare {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(20_000_000)),
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
    ]
}

/// Bei Zusammenveranlagung auf `rentner_gesamt`: die fuenf KAP-Felder des Partners (Nullen); die Scheibe
/// kennt `bruttoarbeitslohn_partner` nicht.
fn partner_kap() -> Paare {
    vec![
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ]
}

/// Pflicht-Kegel `gesamt` (35 Felder ohne das Kreuz), AN 200.000 EUR, `kein_sonstige = true`.
fn kegel_gesamt() -> Paare {
    vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(0)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(20_000_000)),
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

/// Der Verkauf: Grundstueck, Erloes 100.000 EUR, Anschaffung 50.000 EUR -> Gewinn 50.000 EUR. Keine der
/// vier Befreiungen (Frist, Eigennutzung, taeglicher Gebrauch, originaerer Erwerb) wird je erfragt.
fn verkauf() -> Paare {
    vec![
        ("p23_veraeusserungs_typ", json!("grundstueck")),
        ("p23_veraeusserungspreis", json!(10_000_000)),
        ("p23_anschaffung_herstellungskosten", json!(5_000_000)),
        ("p23_werbungskosten", json!(0)),
    ]
}

/// Derselbe Verkauf, aber NUR unter der zweiten Zaehl-Instanz `__2`: kein Feld unter dem Basisnamen.
fn verkauf_zaehl_2() -> Paare {
    vec![
        ("p23_veraeusserungs_typ__2", json!("grundstueck")),
        ("p23_veraeusserungspreis__2", json!(10_000_000)),
        ("p23_anschaffung_herstellungskosten__2", json!(5_000_000)),
        ("p23_werbungskosten__2", json!(0)),
    ]
}

/// Haengt das Kreuz `kein_p23_verkauf` mit `wert` an (`None`: nie gefragt) und danach den Verkauf, wenn
/// `mit_verkauf`.
fn akte(mut kegel: Paare, kreuz: Option<bool>, mit_verkauf: bool) -> Paare {
    if let Some(w) = kreuz {
        kegel.push(("kein_p23_verkauf", json!(w)));
    }
    if mit_verkauf {
        kegel.extend(verkauf());
    }
    kegel
}

/// Wie [`akte`], aber der Verkauf steht nur unter `__2`.
fn akte_zaehl_2(mut kegel: Paare, kreuz: Option<bool>) -> Paare {
    if let Some(w) = kreuz {
        kegel.push(("kein_p23_verkauf", json!(w)));
    }
    kegel.extend(verkauf_zaehl_2());
    kegel
}

/// Die Baseline muss eine Zahl sein, sonst ist "gesperrt" nicht von "Kegel offen" zu unterscheiden.
fn erwarte_zahl(name: &str, a: &Value, zahl: i64) {
    assert_eq!(
        (a["grund"].as_str(), a["zahl_cent"].as_i64()),
        (Some("bestaetigt"), Some(zahl)),
        "{name}: Positivkontrolle -- erwartet bestaetigt mit {zahl} Cent. Antwort: {a}"
    );
}

/// Der Verkaufsfall muss gesperrt sein, ohne Zahl. Die Meldung rechnet aus, was ohne Waechter herauskaeme.
fn erwarte_sperre(name: &str, a: &Value, grund: &str, baseline: i64) {
    let zahl = a["zahl_cent"].as_i64();
    assert!(
        zahl.is_none() && a["grund"] == grund,
        "{name}: erwartet Sperre {grund:?} ohne Zahl; erhalten grund={} zahl_cent={zahl:?} \
         (Differenz gegen die Baseline {baseline}: {:?} Cent -- 2.100.000 Cent sind der \
         ungeprueft besteuerte Verkaufsgewinn von 50.000 EUR). Antwort: {a}",
        a["grund"],
        zahl.map(|z| z - baseline)
    );
}

// ---- Rentner, einzel ------------------------------------------------------------------------------

#[tokio::test]
async fn r0_rentner_ohne_verkauf_rechnet_5_917_000() {
    let a = ergebnis(
        "rentner_gesamt",
        &akte(kegel_rentner("einzel"), Some(true), false),
    )
    .await;
    erwarte_zahl("R0", &a, R0);
}

/// Der Fall des Vault-Eintrags: Verkauf eingetragen, das Kreuz nie beantwortet. Faengt `FLAG_NEGIERT`.
#[tokio::test]
async fn r1_rentner_verkauf_kreuz_nie_gefragt_sperrt_flag_konsistenz_offen() {
    let a = ergebnis("rentner_gesamt", &akte(kegel_rentner("einzel"), None, true)).await;
    erwarte_sperre("R1", &a, FLAG_NIE_GEFRAGT, R0);
}

/// Der ehrliche Verkaeufer: Kreuz bejaht. Faengt `fremd_arten` von `rentner_gesamt`.
#[tokio::test]
async fn r2_rentner_verkauf_bejaht_sperrt_einkunftsart_nicht_ring_faehig() {
    let a = ergebnis(
        "rentner_gesamt",
        &akte(kegel_rentner("einzel"), Some(false), true),
    )
    .await;
    erwarte_sperre("R2", &a, FLAG_BEJAHT, R0);
}

/// Kreuz "kein Verkauf" trotz Betraegen: der Widerspruch. Faengt `FLAG_NEGIERT`.
#[tokio::test]
async fn r3_rentner_kreuz_verneint_trotz_verkauf_sperrt_flag_konsistenz_offen() {
    let a = ergebnis(
        "rentner_gesamt",
        &akte(kegel_rentner("einzel"), Some(true), true),
    )
    .await;
    erwarte_sperre("R3", &a, FLAG_NIE_GEFRAGT, R0);
}

/// Zaehl-Instanz `__2`: Kreuz "kein Verkauf", der Verkauf steht nur unter `p23_*__2`. Python-Fall R4.
/// Faengt `instanz_feld_ids_text`; ohne die Instanz-Suche saehe `FLAG_NEGIERT` keinen Betrag.
#[tokio::test]
async fn r4_rentner_kreuz_verneint_verkauf_nur_unter_zaehl_2_sperrt_flag_konsistenz_offen() {
    let a = ergebnis(
        "rentner_gesamt",
        &akte_zaehl_2(kegel_rentner("einzel"), Some(true)),
    )
    .await;
    erwarte_sperre("R4", &a, FLAG_NIE_GEFRAGT, R0);
}

/// Zaehl-Instanz `__2`, Kreuz nie gefragt. Python-Fall R5.
#[tokio::test]
async fn r5_rentner_kreuz_nie_gefragt_verkauf_nur_unter_zaehl_2_sperrt_flag_konsistenz_offen() {
    let a = ergebnis(
        "rentner_gesamt",
        &akte_zaehl_2(kegel_rentner("einzel"), None),
    )
    .await;
    erwarte_sperre("R5", &a, FLAG_NIE_GEFRAGT, R0);
}

/// Zaehl-Instanz `__2`, Kreuz bejaht: `fremd_arten` liest nur das Kreuz, nicht den Ort des Betrags.
#[tokio::test]
async fn r6_rentner_kreuz_bejaht_verkauf_nur_unter_zaehl_2_sperrt_einkunftsart_nicht_ring_faehig() {
    let a = ergebnis(
        "rentner_gesamt",
        &akte_zaehl_2(kegel_rentner("einzel"), Some(false)),
    )
    .await;
    erwarte_sperre("R6", &a, FLAG_BEJAHT, R0);
}

// ---- Rentner, zusammen ----------------------------------------------------------------------------

fn kegel_rentner_zusammen() -> Paare {
    let mut k = kegel_rentner("zusammen");
    k.extend(partner_kap());
    k
}

#[tokio::test]
async fn z0_rentner_zusammen_ohne_verkauf_rechnet_4_824_200() {
    let a = ergebnis(
        "rentner_gesamt",
        &akte(kegel_rentner_zusammen(), Some(true), false),
    )
    .await;
    erwarte_zahl("Z0", &a, Z0);
}

#[tokio::test]
async fn z1_rentner_zusammen_verkauf_kreuz_nie_gefragt_sperrt_flag_konsistenz_offen() {
    let a = ergebnis(
        "rentner_gesamt",
        &akte(kegel_rentner_zusammen(), None, true),
    )
    .await;
    erwarte_sperre("Z1", &a, FLAG_NIE_GEFRAGT, Z0);
}

#[tokio::test]
async fn z2_rentner_zusammen_verkauf_bejaht_sperrt_einkunftsart_nicht_ring_faehig() {
    let a = ergebnis(
        "rentner_gesamt",
        &akte(kegel_rentner_zusammen(), Some(false), true),
    )
    .await;
    erwarte_sperre("Z2", &a, FLAG_BEJAHT, Z0);
}

// ---- gesamt ---------------------------------------------------------------------------------------

#[tokio::test]
async fn g0_gesamt_ohne_verkauf_rechnet_7_255_600() {
    let a = ergebnis("gesamt", &akte(kegel_gesamt(), Some(true), false)).await;
    erwarte_zahl("G0", &a, G0);
}

/// Ehrliches Ja auf `gesamt`; `kein_sonstige = true` (der Nutzer trennt die Fragen). Nur `kein_p23_verkauf`
/// in `fremd_arten` faengt das -- `kein_sonstige` steht auf "nein" und sperrt nicht.
#[tokio::test]
async fn g1_gesamt_verkauf_bejaht_sperrt_einkunftsart_nicht_ring_faehig() {
    let a = ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), true)).await;
    erwarte_sperre("G1", &a, FLAG_BEJAHT, G0);
}

#[tokio::test]
async fn g3_gesamt_verkauf_kreuz_nie_gefragt_sperrt_flag_konsistenz_offen() {
    let a = ergebnis("gesamt", &akte(kegel_gesamt(), None, true)).await;
    erwarte_sperre("G3", &a, FLAG_NIE_GEFRAGT, G0);
}

/// Zaehl-Instanz `__2` auf `gesamt`, Kreuz nie gefragt (G3 der Sonde mit `__2`).
#[tokio::test]
async fn g5_gesamt_kreuz_nie_gefragt_verkauf_nur_unter_zaehl_2_sperrt_flag_konsistenz_offen() {
    let a = ergebnis("gesamt", &akte_zaehl_2(kegel_gesamt(), None)).await;
    erwarte_sperre("G5", &a, FLAG_NIE_GEFRAGT, G0);
}

/// Zaehl-Instanz `__2` auf `gesamt`, Kreuz "kein Verkauf" (G4 der Sonde mit `__2`).
#[tokio::test]
async fn g6_gesamt_kreuz_verneint_verkauf_nur_unter_zaehl_2_sperrt_flag_konsistenz_offen() {
    let a = ergebnis("gesamt", &akte_zaehl_2(kegel_gesamt(), Some(true))).await;
    erwarte_sperre("G6", &a, FLAG_NIE_GEFRAGT, G0);
}

/// Zaehl-Instanz `__2` auf `gesamt`, Kreuz bejaht: nur `fremd_arten` faengt es (`kein_sonstige` = "nein").
#[tokio::test]
async fn g7_gesamt_kreuz_bejaht_verkauf_nur_unter_zaehl_2_sperrt_einkunftsart_nicht_ring_faehig() {
    let a = ergebnis("gesamt", &akte_zaehl_2(kegel_gesamt(), Some(false))).await;
    erwarte_sperre("G7", &a, FLAG_BEJAHT, G0);
}
