//! Rente mit Leistungsmitteilung (Abweichung Nr. 55, Anlage R-AV/bAV): der Waechter der Scheiben `gesamt` und `rentner_gesamt`
//! im Standardlauf (ohne `PARITY=1`, ohne Python).
//!
//! **Worum es geht.** Wer das Kreuz "Ich habe eine Rente mit Leistungsmitteilung" bejaht, bekommt eine Zahl nur, wenn jede Mitteilung
//! vollstaendig ist und nur Nummern traegt, die die Software rechnet (Nr. 1, 4 und 5). Alles andere sperrt VOR jeder Zahl mit Klartext:
//! `rav_bav_angaben_offen` (der Nutzer kann nachtragen) oder `rav_bav_nicht_gerechnet` (die Software rechnet den Fall nicht).
//!
//! **Warum es zaehlt.** Ohne den Waechter lieferte die Software eine Zahl ohne die weiteren Nummern, ohne die Basisrente und ohne die
//! Rente des Ehegatten. Die Zahl waere zu niedrig, die Erklaerung abgabefaehig.
//!
//! **Wo es sitzt.** `rav_bav_sperrgrund` (`rust/bescheid/src/rav_bav.rs`), aufgerufen in `sperre/gesamt.rs::gesamt_guard`. Python
//! kennt weder die Felder noch die zwei Gruende (Abweichung Nr. 55); die Erwartungen sind die Regeln aus dem Vault-Eintrag
//! `renten-mit-leistungsmitteilung-bauen-oder-hart-sperren`, AK4. Die Zahlen R0/Z0/G0 sind die Positivkontrolle: dieselben
//! Akten ohne Mitteilung wie in `p23_waechter_hermetisch.rs` (dort aus dem Python-Server gelesen, hier unveraendert, weil das Kreuz
//! "keine Rente mit Leistungsmitteilung" nichts aendert). Ohne sie waere "gesperrt" nicht von "Kegel offen" zu unterscheiden.
//! Die Rechnung einer vollstaendigen Mitteilung misst `bescheid/tests/rav_bav_ring_hermetisch.rs`; hier genuegt: es kommt eine Zahl,
//! und sie liegt ueber der Baseline.
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

/// Baseline Rentner einzel (Rente 20.000 EUR, Kreuz "keine Mitteilung"): 59.170,00 EUR festzusetzende `ESt`.
const R0: i64 = 5_917_000;
/// Baseline Rentner zusammen (Person A: Rente 20.000 EUR, Partner nur KAP-Nullen).
const Z0: i64 = 4_824_200;
/// Baseline `gesamt` (AN 200.000 EUR, `kein_sonstige` auf "ja").
const G0: i64 = 7_255_600;

/// Die Angabe fehlt oder passt nicht; der Nutzer kann nachtragen.
const OFFEN: &str = "rav_bav_angaben_offen";
/// Die Software rechnet den Fall nicht.
const NICHT_GERECHNET: &str = "rav_bav_nicht_gerechnet";

type Paare = Vec<(String, Value)>;

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

/// Legt einen Fall der Scheibe an, schreibt jedes Paar ueber die echte Route `POST /event` (als Nutzer-Klick, bestaetigt; ein Feld
/// aus `vorlaeufig` ohne zweites Signal) und liefert `GET /ergebnis`.
async fn ergebnis_mit(scheibe: &str, paare: &[(String, Value)], vorlaeufig: &[&str]) -> Value {
    antwort_mit(scheibe, paare, vorlaeufig, "/fall/rav/ergebnis").await
}

/// Wie [`ergebnis_mit`], nur die Antwort von `GET pfad` (zum Beispiel `/fall/rav/fragen`).
async fn antwort_mit(
    scheibe: &str,
    paare: &[(String, Value)],
    vorlaeufig: &[&str],
    pfad: &str,
) -> Value {
    let d = dienst();
    let kopf = json!({"fall_id": "rav", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let bestaetigt = !vorlaeufig.contains(&feld.as_str());
        let rumpf = json!({
            "feld_id": feld, "wert": wert,
            "zustand": if bestaetigt { "bestaetigt" } else { "vorlaeufig" },
            "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": bestaetigt.then(|| format!("ok@{feld}"))},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (status, antwort) = sende(&d, "POST", "/fall/rav/event", Some(&rumpf)).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, antwort) = sende(&d, "GET", pfad, None).await;
    assert_eq!(status, 200, "GET {pfad}: {antwort}");
    antwort
}

async fn ergebnis(scheibe: &str, paare: &[(String, Value)]) -> Value {
    ergebnis_mit(scheibe, paare, &[]).await
}

fn p(feld: &str, wert: Value) -> (String, Value) {
    (feld.to_owned(), wert)
}

/// Pflicht-Kegel `rentner_gesamt` (ohne die Kreuze der Zusatzfragen), Rente 20.000 EUR ab 2025, geboren am 05.05.1955.
fn kegel_rentner(veranlagung: &str) -> Paare {
    let mut k: Paare = [
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(20_000_000)),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_grad_der_behinderung", json!(0)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(0)),
        ("rentner_gepflegter_hilflos", json!(false)),
        ("rentner_hinterbliebenenbezuege", json!(false)),
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
    .into_iter()
    .map(|(f, w)| p(f, w))
    .collect();
    k.push(p("veranlagung", json!(veranlagung)));
    if veranlagung == "zusammen" {
        for f in [
            "kap_kapitalertraege_partner",
            "kap_gewinn_aktien_partner",
            "kap_gewinn_sonstige_partner",
            "kap_verlust_aktien_partner",
            "kap_verlust_sonstige_partner",
        ] {
            k.push(p(f, json!(0)));
        }
    }
    k
}

/// Pflicht-Kegel `gesamt`, AN 200.000 EUR, `kein_sonstige = true`.
fn kegel_gesamt() -> Paare {
    [
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
    .into_iter()
    .map(|(f, w)| p(f, w))
    .collect()
}

/// Das Geburtsdatum, das die Nr. 5 fuer das Alter bei Rentenbeginn braucht. Es steht nicht im Kegel: die Baselines R0/Z0/G0 gelten
/// ohne Geburtsdatum (mit ihm bekaeme `gesamt` den Altersentlastungsbetrag).
fn geboren() -> Paare {
    vec![p("stammdaten_geburtsdatum", json!("05.05.1955"))]
}

/// Der Name des Feldes in der Zaehl-Instanz `n` (Instanz 1 traegt den Basisnamen).
fn name(basis: &str, n: u8) -> String {
    if n == 1 {
        basis.to_owned()
    } else {
        format!("{basis}__{n}")
    }
}

/// Eine vollstaendige Mitteilung der Instanz `n`: Nr. 1 3.000 EUR, Nr. 4 0, Nr. 5 0, keine weiteren Nummern, keine Basisrente.
fn mitteilung(n: u8) -> Paare {
    vec![
        p(&name("rav_bav_nr1_betrag", n), json!(300_000)),
        p(&name("rav_bav_nr4_betrag", n), json!(0)),
        p(&name("rav_bav_nr5_betrag", n), json!(0)),
        p(&name("rav_bav_andere_nummern", n), json!(false)),
        p(&name("rav_bav_basisrente", n), json!(false)),
    ]
}

/// Setzt oder ergaenzt `feld` in `k`.
fn mit(mut k: Paare, feld: &str, wert: Value) -> Paare {
    match k.iter_mut().find(|(f, _)| f == feld) {
        Some(e) => e.1 = wert,
        None => k.push(p(feld, wert)),
    }
    k
}

/// Entfernt `feld` aus `k`.
fn ohne(mut k: Paare, feld: &str) -> Paare {
    k.retain(|(f, _)| f != feld);
    k
}

/// Der Kegel mit dem Kreuz `kein_rav_bav` (`Some(w)`; `None`: nie gefragt) und den Feldern der Mitteilungen.
fn akte(kegel: Paare, kreuz: Option<bool>, mitteilungen: Paare) -> Paare {
    let mut a = kegel;
    if let Some(w) = kreuz {
        a.push(p("kein_rav_bav", json!(w)));
    }
    a.extend(mitteilungen);
    a
}

/// Die Baseline muss eine Zahl sein, sonst ist "gesperrt" nicht von "Kegel offen" zu unterscheiden.
fn erwarte_zahl(name: &str, a: &Value, zahl: i64) {
    assert_eq!(
        (a["grund"].as_str(), a["zahl_cent"].as_i64()),
        (Some("bestaetigt"), Some(zahl)),
        "{name}: Positivkontrolle -- erwartet bestaetigt mit {zahl} Cent. Antwort: {a}"
    );
}

/// Der Fall muss gesperrt sein, ohne Zahl.
fn erwarte_sperre(name: &str, a: &Value, grund: &str) {
    let zahl = a["zahl_cent"].as_i64();
    assert!(
        zahl.is_none() && a["grund"] == grund,
        "{name}: erwartet Sperre {grund:?} ohne Zahl; erhalten grund={} zahl_cent={zahl:?}. Antwort: {a}",
        a["grund"]
    );
}

/// Eine vollstaendige Mitteilung rechnet: es kommt eine Zahl, und sie liegt ueber der Baseline (3.000 EUR mehr Einkuenfte).
fn erwarte_mehr_als(name: &str, a: &Value, baseline: i64) {
    let zahl = a["zahl_cent"].as_i64();
    assert!(
        a["grund"] == "bestaetigt" && zahl.is_some_and(|z| z > baseline),
        "{name}: erwartet eine Zahl ueber {baseline} Cent; erhalten grund={} zahl_cent={zahl:?}. Antwort: {a}",
        a["grund"]
    );
}

// ---- Positivkontrolle -----------------------------------------------------------------------------

/// Das Kreuz "keine Mitteilung" aendert die Baselines nicht.
#[tokio::test]
async fn p0_ohne_mitteilung_rechnen_die_baselines() {
    erwarte_zahl(
        "R0",
        &ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("einzel"), Some(true), vec![]),
        )
        .await,
        R0,
    );
    erwarte_zahl(
        "Z0",
        &ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("zusammen"), Some(true), vec![]),
        )
        .await,
        Z0,
    );
    erwarte_zahl(
        "G0",
        &ergebnis("gesamt", &akte(kegel_gesamt(), Some(true), vec![])).await,
        G0,
    );
}

/// Eine vollstaendige Mitteilung (Nr. 1 3.000 EUR) rechnet auf beiden Scheiben, auch eine zweite Zaehl-Instanz.
#[tokio::test]
async fn v1_vollstaendige_mitteilung_rechnet() {
    let eine = || mitteilung(1);
    let zwei = || [mitteilung(1), mitteilung(2)].concat();
    erwarte_mehr_als(
        "R einzel",
        &ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("einzel"), Some(false), eine()),
        )
        .await,
        R0,
    );
    erwarte_mehr_als(
        "G einzel",
        &ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), eine())).await,
        G0,
    );
    let a1 = ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), eine())).await;
    let a2 = ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), zwei())).await;
    assert!(
        a2["zahl_cent"].as_i64() > a1["zahl_cent"].as_i64(),
        "die zweite Mitteilung erhoeht die Zahl: {a1} gegen {a2}"
    );
}

// ---- Offene Angaben -------------------------------------------------------------------------------

/// Kreuz bejaht, keine einzige Mitteilung: eine offene Angabe, kein "keine Rente".
#[tokio::test]
async fn o1_kreuz_bejaht_ohne_mitteilung_ist_offen() {
    erwarte_sperre(
        "R",
        &ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("einzel"), Some(false), vec![]),
        )
        .await,
        OFFEN,
    );
    erwarte_sperre(
        "G",
        &ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), vec![])).await,
        OFFEN,
    );
}

/// Jede Pflichtangabe der Mitteilung fehlt einzeln: Betrag, Antwort zu weiteren Nummern, Antwort zur Basisrente.
#[tokio::test]
async fn o2_eine_fehlende_pflichtangabe_ist_offen() {
    for fehlt in [
        "rav_bav_nr1_betrag",
        "rav_bav_nr4_betrag",
        "rav_bav_nr5_betrag",
        "rav_bav_andere_nummern",
        "rav_bav_basisrente",
    ] {
        let m = ohne(mitteilung(1), fehlt);
        erwarte_sperre(
            &format!("R ohne {fehlt}"),
            &ergebnis(
                "rentner_gesamt",
                &akte(kegel_rentner("einzel"), Some(false), m.clone()),
            )
            .await,
            OFFEN,
        );
        erwarte_sperre(
            &format!("G ohne {fehlt}"),
            &ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), m)).await,
            OFFEN,
        );
    }
}

/// Zu einem Betrag ueber 0 gehoert ein Beginn im Steuerjahr oder davor; bei Nr. 5 auch das Geburtsdatum (Alter bei Rentenbeginn).
#[tokio::test]
async fn o3_beginn_fehlt_oder_liegt_nach_dem_steuerjahr_ist_offen() {
    let nr4 = |beginn: Option<&str>| {
        let mut m = mit(mitteilung(1), "rav_bav_nr4_betrag", json!(200_000));
        if let Some(b) = beginn {
            m.push(p("rav_bav_nr4_beginn", json!(b)));
        }
        m
    };
    let nr5 = |beginn: Option<&str>| {
        let mut m = [
            mit(mitteilung(1), "rav_bav_nr5_betrag", json!(1_000_000)),
            geboren(),
        ]
        .concat();
        if let Some(b) = beginn {
            m.push(p("rav_bav_nr5_beginn", json!(b)));
        }
        m
    };
    for (was, m) in [
        ("Nr. 4 ohne Beginn", nr4(None)),
        ("Nr. 4 Beginn nach dem Steuerjahr", nr4(Some("01.01.2026"))),
        ("Nr. 5 ohne Beginn", nr5(None)),
        ("Nr. 5 Beginn nach dem Steuerjahr", nr5(Some("01.01.2026"))),
        ("Nr. 5 Beginn vor der Geburt", nr5(Some("01.01.1950"))),
        // Ohne Geburtsdatum laesst sich das Alter bei Rentenbeginn nicht bestimmen.
        (
            "Nr. 5 ohne Geburtsdatum",
            ohne(nr5(Some("05.05.2020")), "stammdaten_geburtsdatum"),
        ),
    ] {
        erwarte_sperre(
            &format!("R {was}"),
            &ergebnis(
                "rentner_gesamt",
                &akte(kegel_rentner("einzel"), Some(false), m.clone()),
            )
            .await,
            OFFEN,
        );
        erwarte_sperre(
            &format!("G {was}"),
            &ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), m)).await,
            OFFEN,
        );
    }
    // Gegenprobe: dieselben Mitteilungen mit gueltigem Beginn rechnen.
    erwarte_mehr_als(
        "R Nr. 4 mit Beginn",
        &ergebnis(
            "rentner_gesamt",
            &akte(
                kegel_rentner("einzel"),
                Some(false),
                nr4(Some("01.04.2022")),
            ),
        )
        .await,
        R0,
    );
    let mit_geburt = |mut k: Paare| {
        k.extend(geboren());
        k
    };
    let g_ohne = ergebnis(
        "gesamt",
        &akte(mit_geburt(kegel_gesamt()), Some(true), vec![]),
    )
    .await;
    let r_ohne = ergebnis(
        "rentner_gesamt",
        &akte(mit_geburt(kegel_rentner("einzel")), Some(true), vec![]),
    )
    .await;
    let (g_ohne, r_ohne) = (
        g_ohne["zahl_cent"].as_i64().unwrap(),
        r_ohne["zahl_cent"].as_i64().unwrap(),
    );
    erwarte_mehr_als(
        "G Nr. 5 mit Beginn",
        &ergebnis(
            "gesamt",
            &akte(kegel_gesamt(), Some(false), nr5(Some("05.05.2020"))),
        )
        .await,
        g_ohne,
    );
    erwarte_mehr_als(
        "R Nr. 5 mit Beginn",
        &ergebnis(
            "rentner_gesamt",
            &akte(
                kegel_rentner("einzel"),
                Some(false),
                nr5(Some("05.05.2020")),
            ),
        )
        .await,
        r_ohne,
    );
}

/// Die Zaehl-Instanz `__2` zaehlt wie die erste: ist sie unvollstaendig, sperrt die Akte.
#[tokio::test]
async fn o4_eine_unvollstaendige_zweite_mitteilung_ist_offen() {
    let zweite = ohne(mitteilung(2), "rav_bav_nr5_betrag__2");
    let m = [mitteilung(1), zweite].concat();
    erwarte_sperre(
        "R",
        &ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("einzel"), Some(false), m.clone()),
        )
        .await,
        OFFEN,
    );
    erwarte_sperre(
        "G",
        &ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), m)).await,
        OFFEN,
    );
}

/// Eine Mitteilung mit einem nur vorlaeufigen Betrag ist keine Antwort: gesperrt, nicht gerechnet und nicht weggelassen.
#[tokio::test]
async fn o5_eine_vorlaeufige_mitteilung_ist_offen() {
    let vorlaeufig = ["rav_bav_nr1_betrag"];
    let a = ergebnis_mit(
        "rentner_gesamt",
        &akte(kegel_rentner("einzel"), Some(false), mitteilung(1)),
        &vorlaeufig,
    )
    .await;
    erwarte_sperre("R", &a, OFFEN);
    let a = ergebnis_mit(
        "gesamt",
        &akte(kegel_gesamt(), Some(false), mitteilung(1)),
        &vorlaeufig,
    )
    .await;
    erwarte_sperre("G", &a, OFFEN);
}

// ---- Nicht gerechnet ------------------------------------------------------------------------------

/// "Weitere Nummern" und "Basisrente" mit Ja: die Software rechnet den Fall nicht.
#[tokio::test]
async fn n1_weitere_nummern_und_basisrente_sperren() {
    for feld in ["rav_bav_andere_nummern", "rav_bav_basisrente"] {
        let m = mit(mitteilung(1), feld, json!(true));
        erwarte_sperre(
            &format!("R {feld}"),
            &ergebnis(
                "rentner_gesamt",
                &akte(kegel_rentner("einzel"), Some(false), m.clone()),
            )
            .await,
            NICHT_GERECHNET,
        );
        erwarte_sperre(
            &format!("G {feld}"),
            &ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), m)).await,
            NICHT_GERECHNET,
        );
    }
}

/// Dasselbe in der zweiten Zaehl-Instanz.
#[tokio::test]
async fn n2_die_zweite_mitteilung_mit_weiteren_nummern_sperrt() {
    let m = [
        mitteilung(1),
        mit(mitteilung(2), "rav_bav_andere_nummern__2", json!(true)),
    ]
    .concat();
    erwarte_sperre(
        "R",
        &ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("einzel"), Some(false), m.clone()),
        )
        .await,
        NICHT_GERECHNET,
    );
    erwarte_sperre(
        "G",
        &ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), m)).await,
        NICHT_GERECHNET,
    );
}

/// Was die Software nie rechnet, geht vor dem, was der Nutzer nachtragen kann: Instanz 1 unvollstaendig, Instanz 2 Basisrente.
#[tokio::test]
async fn n3_nicht_gerechnet_geht_vor_offen() {
    let m = [
        ohne(mitteilung(1), "rav_bav_nr1_betrag"),
        mit(mitteilung(2), "rav_bav_basisrente__2", json!(true)),
    ]
    .concat();
    erwarte_sperre(
        "R",
        &ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("einzel"), Some(false), m.clone()),
        )
        .await,
        NICHT_GERECHNET,
    );
    erwarte_sperre(
        "G",
        &ergebnis("gesamt", &akte(kegel_gesamt(), Some(false), m)).await,
        NICHT_GERECHNET,
    );
}

/// Zusammenveranlagung: keine Angabe sagt, wem die Mitteilung gehoert, und die Rente des Ehegatten ist nicht gebaut (Person B,
/// Stufe 2). Ein bejahtes Kreuz sperrt, auch mit vollstaendiger Mitteilung; ohne bejahtes Kreuz bleibt die Baseline.
#[tokio::test]
async fn n4_zusammenveranlagung_mit_mitteilung_sperrt() {
    erwarte_sperre(
        "Z",
        &ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("zusammen"), Some(false), mitteilung(1)),
        )
        .await,
        NICHT_GERECHNET,
    );
    erwarte_zahl(
        "Z0",
        &ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("zusammen"), Some(true), vec![]),
        )
        .await,
        Z0,
    );
}

// ---- Widerspruch zum Kreuz ------------------------------------------------------------------------

/// Kreuz "keine Mitteilung" (oder nie beantwortet) trotz eingetragener Betraege: der Widerspruch sperrt, die Betraege fallen nicht
/// still weg. Ein Betrag 0 ist kein Widerspruch.
#[tokio::test]
async fn w1_betrag_trotz_kreuz_keine_mitteilung_sperrt() {
    for kreuz in [Some(true), None] {
        erwarte_sperre(
            &format!("R {kreuz:?}"),
            &ergebnis(
                "rentner_gesamt",
                &akte(kegel_rentner("einzel"), kreuz, mitteilung(1)),
            )
            .await,
            OFFEN,
        );
        erwarte_sperre(
            &format!("G {kreuz:?}"),
            &ergebnis("gesamt", &akte(kegel_gesamt(), kreuz, mitteilung(1))).await,
            OFFEN,
        );
    }
    let nullen = mit(mitteilung(1), "rav_bav_nr1_betrag", json!(0));
    erwarte_zahl(
        "R mit Nullen",
        &ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("einzel"), Some(true), nullen.clone()),
        )
        .await,
        R0,
    );
    erwarte_zahl(
        "G mit Nullen",
        &ergebnis("gesamt", &akte(kegel_gesamt(), Some(true), nullen)).await,
        G0,
    );
}

/// Ein nur vorlaeufiger Betrag zu "keine Mitteilung" (oder ohne Kreuz) ist noch keine Angabe: er sperrt nicht, der Fall rechnet wie
/// ohne ihn (Mutant N21 der Nachmessung Nr. 55). Erst der bestaetigte Betrag ist der Widerspruch, den `w1` pinnt.
#[tokio::test]
async fn w2_ein_vorlaeufiger_betrag_trotz_kreuz_keine_mitteilung_sperrt_nicht() {
    let vorlaeufig = ["rav_bav_nr1_betrag"];
    for kreuz in [Some(true), None] {
        let r = ergebnis_mit(
            "rentner_gesamt",
            &akte(kegel_rentner("einzel"), kreuz, mitteilung(1)),
            &vorlaeufig,
        )
        .await;
        let r_ohne = ergebnis(
            "rentner_gesamt",
            &akte(kegel_rentner("einzel"), kreuz, vec![]),
        )
        .await;
        assert_eq!(r["grund"], r_ohne["grund"], "R {kreuz:?}: {r}");
        assert_eq!(r["zahl_cent"], r_ohne["zahl_cent"], "R {kreuz:?}: {r}");
        let g = ergebnis_mit(
            "gesamt",
            &akte(kegel_gesamt(), kreuz, mitteilung(1)),
            &vorlaeufig,
        )
        .await;
        let g_ohne = ergebnis("gesamt", &akte(kegel_gesamt(), kreuz, vec![])).await;
        assert_eq!(g["grund"], g_ohne["grund"], "G {kreuz:?}: {g}");
        assert_eq!(g["zahl_cent"], g_ohne["zahl_cent"], "G {kreuz:?}: {g}");
    }
    erwarte_zahl(
        "R Kreuz keine Mitteilung",
        &ergebnis_mit(
            "rentner_gesamt",
            &akte(kegel_rentner("einzel"), Some(true), mitteilung(1)),
            &vorlaeufig,
        )
        .await,
        R0,
    );
}

// ---- Fragenfluss ----------------------------------------------------------------------------------

/// Die sieben Felder einer Mitteilung (Gruppe `rente_av_bav`).
const MITTEILUNG_FELDER: [&str; 7] = [
    "rav_bav_nr1_betrag",
    "rav_bav_nr4_betrag",
    "rav_bav_nr4_beginn",
    "rav_bav_nr5_betrag",
    "rav_bav_nr5_beginn",
    "rav_bav_andere_nummern",
    "rav_bav_basisrente",
];

/// Die Fragen der Mitteilung stehen nach "ja, ich habe eine Rente mit Leistungsmitteilung" (`kein_rav_bav` = nein) und solange das
/// Kreuz offen ist, nach "nein, keine" nicht (Regel `p22_nr5_rav_bav_leistung` in `bindung_regel_bedingungen.yaml`; Mutant N55 der
/// Nachmessung Nr. 55 drehte den Kreuzwert).
#[tokio::test]
async fn f1_die_fragen_der_mitteilung_stehen_nur_nach_ja_und_solange_das_kreuz_offen_ist() {
    for (scheibe, kegel) in [
        ("rentner_gesamt", kegel_rentner("einzel")),
        ("gesamt", kegel_gesamt()),
    ] {
        for (kreuz, erwartet) in [(Some(false), true), (None, true), (Some(true), false)] {
            let fragen = antwort_mit(
                scheibe,
                &akte(kegel.clone(), kreuz, vec![]),
                &[],
                "/fall/rav/fragen",
            )
            .await;
            let ids: Vec<&str> = fragen["fragen"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|q| q["feld_id"].as_str())
                .collect();
            for feld in MITTEILUNG_FELDER {
                assert_eq!(
                    ids.contains(&feld),
                    erwartet,
                    "{scheibe}, Kreuz {kreuz:?}: {feld}"
                );
            }
        }
    }
}
