//! Abweichung Nr. 46: die Rentner-Scheibe fragt zuerst, ob neben der Rente Arbeitslohn oder eine Pension da ist
//! (`kein_lohn_pension` fuer Person A, `kein_lohn_pension_partner` fuer den Ehegatten), und verbirgt bei "nein" die
//! Detailfragen dazu: acht Felder von Person A, zehn des Ehegatten.
//!
//! Polaritaet wie `kein_gewinn`: `true` heisst "nein, weder Lohn noch Pension" und verbirgt. `false` und Schweigen lassen
//! alles stehen (fail-closed fuer die Fragenliste). Der Bescheid liest gespeicherte Werte wie bisher, auch wenn ihre Frage
//! verborgen ist; das Kreuz aendert nur die Liste (Entscheidung 3, ohne `FLAG_NEGIERT`).
//!
//! Gemessen wird ueber die HTTP-Schnittstelle (`GET /fall/reh/fragen`, `GET /fall/reh/ergebnis`), so wie die Oberflaeche
//! es sieht. Ein Test prueft jedes Feld einzeln: eine entfernte Bedingung faellt an ihrem Feldnamen auf, nicht an einer
//! Sammelzahl.
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

/// Die acht Felder von Person A hinter dem Kreuz. `geburtsjahr` gehoert nicht dazu: es traegt auch die 55-Jahre-Frage.
const A_HINTER_DEM_KREUZ: [&str; 8] = [
    "bruttoarbeitslohn",
    "steuerklasse",
    "p36_lohnsteuer",
    "versorgung_jahresrente",
    "versorgung_bemessungsgrundlage",
    "versorgung_beginn_jahr",
    "versorgung_art",
    "versorgung_alter_bei_beginn",
];

/// Die zehn Felder des Ehegatten hinter dem Kreuz (die gleichen acht plus die zwei Rentenversicherungsanteile aus seiner
/// Lohnsteuerbescheinigung). `geburtsjahr_partner` gehoert nicht dazu.
const P_HINTER_DEM_KREUZ: [&str; 10] = [
    "bruttoarbeitslohn_partner",
    "steuerklasse_partner",
    "p36_lohnsteuer_partner",
    "vor_an_anteil_rv_partner",
    "vor_ag_anteil_rv_partner",
    "versorgung_jahresrente_partner",
    "versorgung_bemessungsgrundlage_partner",
    "versorgung_beginn_jahr_partner",
    "versorgung_art_partner",
    "versorgung_alter_bei_beginn_partner",
];

/// Die fuenf Felder des Ehegatten, die schon eine Bedingung tragen. Das Kreuz haengt als `und`-Glied dran; die alte
/// Bedingung gilt weiter.
const P_MIT_ALTER_BEDINGUNG: [&str; 5] = [
    "steuerklasse_partner",
    "p36_lohnsteuer_partner",
    "vor_an_anteil_rv_partner",
    "vor_ag_anteil_rv_partner",
    "versorgung_alter_bei_beginn_partner",
];

/// Die Pflichtfelder im Kegel: ohne Bedingung, sonst bleibt die Zahl bei "nein" fuer immer gesperrt.
const KEGEL_A: [&str; 2] = ["vor_an_anteil_rv", "vor_ag_anteil_rv"];

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

async fn event_post(d: &Dienst, feld: &str, wert: &Value) -> (u16, Value) {
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    sende(d, "POST", "/fall/reh/event", Some(&rumpf)).await
}

/// Antwortet auf ein Feld und verlangt 201.
async fn antworte(d: &Dienst, feld: &str, wert: Value) {
    let (status, antwort) = event_post(d, feld, &wert).await;
    assert_eq!(status, 201, "POST {feld}: {antwort}");
}

async fn neuer_fall() -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "reh", "scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    d
}

async fn fall_mit(paare: &[(&'static str, Value)]) -> Dienst {
    let d = neuer_fall().await;
    for (feld, wert) in paare {
        antworte(&d, feld, wert.clone()).await;
    }
    d
}

async fn fragen(d: &Dienst) -> Vec<Value> {
    let (status, antwort) = sende(d, "GET", "/fall/reh/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {antwort}");
    antwort["fragen"].as_array().unwrap().clone()
}

async fn fragen_ids(d: &Dienst) -> Vec<String> {
    fragen(d)
        .await
        .iter()
        .map(|q| q["feld_id"].as_str().unwrap().to_owned())
        .collect()
}

async fn ergebnis(d: &Dienst) -> Value {
    let (status, antwort) = sende(d, "GET", "/fall/reh/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// Rentner mit allen Pflichtangaben des Kegels. Das Kreuz und die Detailfragen zu Lohn und Pension sind offen.
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
        p.extend(vec![
            ("kap_kapitalertraege_partner", json!(0)),
            ("kap_gewinn_aktien_partner", json!(0)),
            ("kap_gewinn_sonstige_partner", json!(0)),
            ("kap_verlust_aktien_partner", json!(0)),
            ("kap_verlust_sonstige_partner", json!(0)),
        ]);
    }
    p
}

fn mit(mut basis: Paare, mehr: Paare) -> Paare {
    basis.extend(mehr);
    basis
}

/// Die Felder aus `felder`, die in der Liste fehlen.
fn fehlende<'a>(ids: &[String], felder: &[&'a str]) -> Vec<&'a str> {
    felder
        .iter()
        .copied()
        .filter(|f| !ids.iter().any(|i| i == f))
        .collect()
}

/// Die Felder aus `felder`, die in der Liste stehen.
fn vorhandene<'a>(ids: &[String], felder: &[&'a str]) -> Vec<&'a str> {
    felder
        .iter()
        .copied()
        .filter(|f| ids.iter().any(|i| i == f))
        .collect()
}

fn pos(ids: &[String], feld: &str) -> Option<usize> {
    ids.iter().position(|i| i == feld)
}

// ---------------------------------------------------------------- die Kreuze ------------------------------------------

/// AK1/AK2/AK7: beide Kreuze sind invertierte Ankreuzfragen, die Lohn und Pension beim Namen nennen. Die Texte tragen
/// die Begriffe, an denen ein Rentner sein "ja" erkennt (Minijob, Nebenjob, Ruhegehalt, Betriebsrente, Witwen- und
/// Waisengeld) und sagen, dass die gesetzliche Rente allein "nein" heisst.
#[tokio::test]
async fn die_kreuze_nennen_lohn_und_pension_mit_den_begriffen_des_rentners() {
    let d = fall_mit(&rentner(2_000_000, "zusammen")).await;
    let liste = fragen(&d).await;
    for kreuz in ["kein_lohn_pension", "kein_lohn_pension_partner"] {
        let q = liste
            .iter()
            .find(|q| q["feld_id"] == json!(kreuz))
            .unwrap_or_else(|| panic!("{kreuz} fehlt in der Fragenliste"));
        assert_eq!(q["typ"], json!("bool"), "{kreuz}");
        assert_eq!(q["frage_invertiert"], json!(true), "{kreuz}: das Feld nennt die Abwesenheit");
        assert_eq!(q["screening"], json!(false), "{kreuz}: Ueberspringen darf kein stilles 'nein' sein");
        let text = format!("{} {}", q["fragetext_laie"].as_str().unwrap(), q["hilfe_kurz"].as_str().unwrap());
        for wort in ["Arbeitslohn", "Pension", "Minijob", "Nebenjob", "Ruhegehalt", "Betriebsrente", "Witwen", "Waisen"] {
            assert!(text.contains(wort), "{kreuz}: das Wort {wort} fehlt im Text: {text}");
        }
        assert!(text.contains("gesetzliche Rente"), "{kreuz}: der Text sagt nicht, dass die gesetzliche Rente allein 'nein' heisst: {text}");
    }
}

/// AK1: Das Kreuz von Person A ist Frage 1 im leeren Fall, vor `veranlagung` und vor allen Detailfragen.
#[tokio::test]
async fn das_kreuz_von_person_a_ist_die_erste_frage() {
    let ids = fragen_ids(&neuer_fall().await).await;
    assert_eq!(ids.first().map(String::as_str), Some("kein_lohn_pension"), "erste Fragen: {:?}", &ids[..5.min(ids.len())]);
    assert!(pos(&ids, "kein_lohn_pension") < pos(&ids, "veranlagung"));
}

/// AK1: Mit allen Pflichtangaben steht das Kreuz von Person A vor jedem Feld, das es abschaltet.
#[tokio::test]
async fn das_kreuz_von_person_a_steht_vor_den_feldern_die_es_abschaltet() {
    let ids = fragen_ids(&fall_mit(&rentner(2_000_000, "einzel")).await).await;
    let kreuz = pos(&ids, "kein_lohn_pension").expect("kein_lohn_pension fehlt in der Liste");
    for f in A_HINTER_DEM_KREUZ {
        let p = pos(&ids, f).unwrap_or_else(|| panic!("{f} fehlt ohne Antwort auf das Kreuz"));
        assert!(kreuz < p, "kein_lohn_pension@{kreuz} steht nicht vor {f}@{p}");
    }
}

/// AK2: Das Kreuz des Ehegatten steht vor jedem Feld, das es abschaltet (Zusammenveranlagung).
#[tokio::test]
async fn das_kreuz_des_ehegatten_steht_vor_den_feldern_die_es_abschaltet() {
    let ids = fragen_ids(&fall_mit(&rentner(2_000_000, "zusammen")).await).await;
    let kreuz = pos(&ids, "kein_lohn_pension_partner").expect("kein_lohn_pension_partner fehlt in der Liste");
    for f in P_HINTER_DEM_KREUZ {
        let p = pos(&ids, f).unwrap_or_else(|| panic!("{f} fehlt ohne Antwort auf das Kreuz"));
        assert!(kreuz < p, "kein_lohn_pension_partner@{kreuz} steht nicht vor {f}@{p}");
    }
}

/// AK2: Das Kreuz des Ehegatten erscheint nur bei Zusammenveranlagung (und solange `veranlagung` offen ist).
#[tokio::test]
async fn das_kreuz_des_ehegatten_erscheint_nur_bei_zusammenveranlagung() {
    let einzel = fragen_ids(&fall_mit(&rentner(2_000_000, "einzel")).await).await;
    assert!(pos(&einzel, "kein_lohn_pension_partner").is_none(), "Kreuz des Ehegatten bei Einzelveranlagung");
    assert!(pos(&einzel, "kein_lohn_pension").is_some());
    let zusammen = fragen_ids(&fall_mit(&rentner(2_000_000, "zusammen")).await).await;
    assert!(pos(&zusammen, "kein_lohn_pension_partner").is_some(), "Kreuz des Ehegatten fehlt bei Zusammenveranlagung");
}

// ---------------------------------------------------------------- Person A ------------------------------------------------

/// AK1: "Nein" (`kein_lohn_pension = true`) verbirgt jedes der acht Felder von Person A; `geburtsjahr` bleibt, ebenso das
/// Kreuz des Ehegatten und alles andere.
#[tokio::test]
async fn nein_verbirgt_die_acht_felder_von_person_a() {
    let d = fall_mit(&rentner(2_000_000, "einzel")).await;
    let vorher = fragen_ids(&d).await;
    assert!(fehlende(&vorher, &A_HINTER_DEM_KREUZ).is_empty(), "vor der Antwort fehlen: {:?}", fehlende(&vorher, &A_HINTER_DEM_KREUZ));
    antworte(&d, "kein_lohn_pension", json!(true)).await;
    let nachher = fragen_ids(&d).await;
    assert_eq!(vorhandene(&nachher, &A_HINTER_DEM_KREUZ), Vec::<&str>::new(), "trotz 'nein' gefragt");
    assert!(pos(&nachher, "geburtsjahr").is_some(), "geburtsjahr darf nicht hinter dem Kreuz stehen");
    assert!(pos(&nachher, "kein_lohn_pension").is_none(), "die beantwortete Eingangsfrage steht noch in der Liste");
}

/// AK1: "Ja" (`false`) und Schweigen lassen alle acht Felder stehen, wie heute.
#[tokio::test]
async fn ja_und_schweigen_lassen_die_felder_von_person_a_stehen() {
    let schweigen = fragen_ids(&fall_mit(&rentner(2_000_000, "einzel")).await).await;
    assert!(fehlende(&schweigen, &A_HINTER_DEM_KREUZ).is_empty(), "ohne Antwort fehlen: {:?}", fehlende(&schweigen, &A_HINTER_DEM_KREUZ));
    let d = fall_mit(&rentner(2_000_000, "einzel")).await;
    antworte(&d, "kein_lohn_pension", json!(false)).await;
    let ja = fragen_ids(&d).await;
    assert!(fehlende(&ja, &A_HINTER_DEM_KREUZ).is_empty(), "nach 'ja' fehlen: {:?}", fehlende(&ja, &A_HINTER_DEM_KREUZ));
}

/// AK1: Das Kreuz von Person A verbirgt nichts vom Ehegatten.
#[tokio::test]
async fn das_kreuz_von_person_a_laesst_den_ehegatten_unberuehrt() {
    let d = fall_mit(&rentner(2_000_000, "zusammen")).await;
    antworte(&d, "kein_lohn_pension", json!(true)).await;
    let ids = fragen_ids(&d).await;
    assert!(fehlende(&ids, &P_HINTER_DEM_KREUZ).is_empty(), "Felder des Ehegatten verschwunden: {:?}", fehlende(&ids, &P_HINTER_DEM_KREUZ));
    assert!(pos(&ids, "kein_lohn_pension_partner").is_some());
}

/// Wechsel von "nein" auf "ja": die Felder stehen wieder da.
#[tokio::test]
async fn der_wechsel_von_nein_auf_ja_zeigt_die_felder_wieder() {
    let d = fall_mit(&rentner(2_000_000, "zusammen")).await;
    antworte(&d, "kein_lohn_pension", json!(true)).await;
    antworte(&d, "kein_lohn_pension_partner", json!(true)).await;
    let nein = fragen_ids(&d).await;
    assert!(vorhandene(&nein, &A_HINTER_DEM_KREUZ).is_empty() && vorhandene(&nein, &P_HINTER_DEM_KREUZ).is_empty());
    antworte(&d, "kein_lohn_pension", json!(false)).await;
    antworte(&d, "kein_lohn_pension_partner", json!(false)).await;
    let ja = fragen_ids(&d).await;
    assert!(fehlende(&ja, &A_HINTER_DEM_KREUZ).is_empty(), "A fehlt nach dem Wechsel: {:?}", fehlende(&ja, &A_HINTER_DEM_KREUZ));
    assert!(fehlende(&ja, &P_HINTER_DEM_KREUZ).is_empty(), "Ehegatte fehlt nach dem Wechsel: {:?}", fehlende(&ja, &P_HINTER_DEM_KREUZ));
}

// ---------------------------------------------------------------- Ehegatte ------------------------------------------------

/// AK2: "Nein" des Ehegatten verbirgt jedes der zehn Felder; Person A und `geburtsjahr_partner` bleiben.
#[tokio::test]
async fn nein_verbirgt_die_zehn_felder_des_ehegatten() {
    let d = fall_mit(&rentner(2_000_000, "zusammen")).await;
    let vorher = fragen_ids(&d).await;
    assert!(fehlende(&vorher, &P_HINTER_DEM_KREUZ).is_empty(), "vor der Antwort fehlen: {:?}", fehlende(&vorher, &P_HINTER_DEM_KREUZ));
    antworte(&d, "kein_lohn_pension_partner", json!(true)).await;
    let nachher = fragen_ids(&d).await;
    assert_eq!(vorhandene(&nachher, &P_HINTER_DEM_KREUZ), Vec::<&str>::new(), "trotz 'nein' gefragt");
    assert!(pos(&nachher, "geburtsjahr_partner").is_some(), "geburtsjahr_partner darf nicht hinter dem Kreuz stehen");
    assert!(fehlende(&nachher, &A_HINTER_DEM_KREUZ).is_empty(), "das Kreuz des Ehegatten hat Felder von Person A verborgen");
}

/// AK2: "Ja" des Ehegatten laesst alle zehn stehen.
#[tokio::test]
async fn ja_des_ehegatten_laesst_die_zehn_felder_stehen() {
    let d = fall_mit(&rentner(2_000_000, "zusammen")).await;
    antworte(&d, "kein_lohn_pension_partner", json!(false)).await;
    let ids = fragen_ids(&d).await;
    assert!(fehlende(&ids, &P_HINTER_DEM_KREUZ).is_empty(), "nach 'ja' fehlen: {:?}", fehlende(&ids, &P_HINTER_DEM_KREUZ));
}

/// AK2: Die alten Bedingungen gelten bei "ja" weiter: bestaetigter Lohn 0 verbirgt Steuerklasse, Lohnsteuer und die zwei
/// Rentenversicherungsanteile; eine beamtenrechtliche Versorgung verbirgt die Altersfrage.
#[tokio::test]
async fn bei_ja_gelten_die_alten_bedingungen_des_ehegatten_weiter() {
    let d = fall_mit(&rentner(2_000_000, "zusammen")).await;
    antworte(&d, "kein_lohn_pension_partner", json!(false)).await;
    antworte(&d, "bruttoarbeitslohn_partner", json!(0)).await;
    antworte(&d, "versorgung_art_partner", json!("beamtenrechtlich")).await;
    let ids = fragen_ids(&d).await;
    assert_eq!(
        vorhandene(&ids, &P_MIT_ALTER_BEDINGUNG),
        Vec::<&str>::new(),
        "eine alte Bedingung wirkt nicht mehr"
    );
}

/// AK2: Das Kreuz haengt als `und`-Glied an jedem der fuenf Felder mit alter Bedingung. Hier laesst die alte Bedingung das
/// Feld stehen (Lohn ungleich 0, Art = altersgrenze_sonstige); nur das Kreuz verbirgt es.
#[tokio::test]
async fn das_kreuz_haengt_als_und_glied_an_den_fuenf_feldern_mit_alter_bedingung() {
    let d = fall_mit(&mit(
        rentner(2_000_000, "zusammen"),
        vec![
            ("bruttoarbeitslohn_partner", json!(1_000_000)),
            ("versorgung_art_partner", json!("altersgrenze_sonstige")),
        ],
    ))
    .await;
    let vorher = fragen_ids(&d).await;
    assert!(fehlende(&vorher, &P_MIT_ALTER_BEDINGUNG).is_empty(), "die alte Bedingung versteckt schon vorher: {:?}", fehlende(&vorher, &P_MIT_ALTER_BEDINGUNG));
    antworte(&d, "kein_lohn_pension_partner", json!(true)).await;
    let nachher = fragen_ids(&d).await;
    assert_eq!(vorhandene(&nachher, &P_MIT_ALTER_BEDINGUNG), Vec::<&str>::new(), "das Kreuz verbirgt nicht");
}

// ---------------------------------------------------------------- keine Sackgasse (Kegel) --------------------------------

/// AK3: Die zwei Pflichtfelder von Person A im Kegel bleiben ohne Bedingung: bei "nein" stehen sie weiter in der Liste.
/// Mit einer Bedingung am Kreuz verschwaenden sie aus der Liste, blieben aber Pflicht (`input_kegel_nicht_bestaetigt`),
/// und die Zahl bliebe fuer immer gesperrt.
#[tokio::test]
async fn die_pflichtfelder_im_kegel_bleiben_bei_nein_in_der_liste() {
    let ohne_kegel: Paare = rentner(2_000_000, "einzel")
        .into_iter()
        .filter(|(f, _)| !KEGEL_A.contains(f))
        .collect();
    let d = fall_mit(&ohne_kegel).await;
    antworte(&d, "kein_lohn_pension", json!(true)).await;
    let ids = fragen_ids(&d).await;
    assert!(fehlende(&ids, &KEGEL_A).is_empty(), "Kegel-Felder hinter dem Kreuz: {:?}", fehlende(&ids, &KEGEL_A));
    let e = ergebnis(&d).await;
    assert_eq!(e["zahl_cent"], Value::Null, "Zahl ohne Kegel-Antworten: {e}");
    // Mit den Antworten gibt es eine Zahl: keine Sackgasse.
    antworte(&d, "vor_an_anteil_rv", json!(0)).await;
    antworte(&d, "vor_ag_anteil_rv", json!(0)).await;
    let e = ergebnis(&d).await;
    assert!(e["zahl_cent"].is_i64(), "keine Zahl nach 'nein' und beantwortetem Kegel: {e}");
    assert_eq!(e["grund"], json!("bestaetigt"), "{e}");
}

/// AK3: Mit "nein/nein" und beantwortetem Kegel gibt es bei Zusammenveranlagung eine bestaetigte Zahl.
#[tokio::test]
async fn nein_und_nein_geben_bei_zusammenveranlagung_eine_zahl() {
    let d = fall_mit(&rentner(2_000_000, "zusammen")).await;
    antworte(&d, "kein_lohn_pension", json!(true)).await;
    antworte(&d, "kein_lohn_pension_partner", json!(true)).await;
    let e = ergebnis(&d).await;
    assert!(e["zahl_cent"].is_i64(), "{e}");
    assert_eq!(e["grund"], json!("bestaetigt"), "{e}");
}

// ---------------------------------------------------------------- gespeicherte Werte (AK4) -------------------------------

async fn zahl(d: &Dienst) -> i64 {
    ergebnis(d).await["zahl_cent"].as_i64().expect("keine Zahl")
}

/// AK4: Ein gespeicherter Wert zaehlt weiter, auch wenn das Kreuz auf "nein" steht (Entscheidung 3, ohne `FLAG_NEGIERT`):
/// Das Kreuz aendert nur die Fragenliste, nie die Rechnung. Zu viel Steuer, nie zu wenig; "ja" hebt es auf.
#[tokio::test]
async fn ein_gespeicherter_wert_zaehlt_trotz_nein_weiter() {
    let ohne_lohn = zahl(&fall_mit(&rentner(4_000_000, "zusammen")).await).await;
    let mit_lohn = mit(rentner(4_000_000, "zusammen"), vec![("bruttoarbeitslohn_partner", json!(6_000_000))]);
    let offen = zahl(&fall_mit(&mit_lohn).await).await;
    let d = fall_mit(&mit_lohn).await;
    antworte(&d, "kein_lohn_pension_partner", json!(true)).await;
    let nein = zahl(&d).await;
    assert!(ohne_lohn < offen, "der Lohn des Ehegatten zaehlt nicht: {ohne_lohn} gegen {offen}");
    assert_eq!(nein, offen, "das Kreuz hat die Rechnung geaendert");
    // Person A, gleiches Verhalten.
    let a_lohn = mit(rentner(4_000_000, "einzel"), vec![("bruttoarbeitslohn", json!(3_000_000)), ("steuerklasse", json!("1"))]);
    let a_offen = zahl(&fall_mit(&a_lohn).await).await;
    let d = fall_mit(&a_lohn).await;
    antworte(&d, "kein_lohn_pension", json!(true)).await;
    assert_eq!(zahl(&d).await, a_offen, "das Kreuz von Person A hat die Rechnung geaendert");
}

// ---------------------------------------------------------------- Fragenzahl (AK5) ----------------------------------------

/// AK5: Die Fragenzahl vor und nach den Antworten. Die Kreuze selbst verlassen die Liste, sobald sie beantwortet sind.
/// "Nein" spart 8 Fragen je Person (Ehegatte: 10); "ja" kostet nichts gegenueber heute (147 und 196).
#[tokio::test]
async fn die_fragenzahl_vor_und_nach_den_antworten() {
    let faelle: [(&str, &'static str, Option<bool>, Option<bool>, usize); 9] = [
        ("einzel offen", "einzel", None, None, 148),
        ("einzel nein", "einzel", Some(true), None, 139),
        ("einzel ja", "einzel", Some(false), None, 147),
        ("zusammen offen", "zusammen", None, None, 198),
        ("zusammen nein/nein", "zusammen", Some(true), Some(true), 178),
        ("zusammen nein/ja", "zusammen", Some(true), Some(false), 188),
        ("zusammen ja/nein", "zusammen", Some(false), Some(true), 186),
        ("zusammen ja/ja", "zusammen", Some(false), Some(false), 196),
        ("zusammen ja/offen", "zusammen", Some(false), None, 197),
    ];
    let mut abweichungen = Vec::new();
    for (name, veranlagung, a, b, soll) in faelle {
        let d = fall_mit(&rentner(2_000_000, veranlagung)).await;
        if let Some(x) = a {
            antworte(&d, "kein_lohn_pension", json!(x)).await;
        }
        if let Some(x) = b {
            antworte(&d, "kein_lohn_pension_partner", json!(x)).await;
        }
        let ist = fragen_ids(&d).await.len();
        if ist != soll {
            abweichungen.push(format!("{name}: {ist} statt {soll}"));
        }
    }
    assert!(abweichungen.is_empty(), "Fragenzahl weicht ab: {abweichungen:?}");
}
