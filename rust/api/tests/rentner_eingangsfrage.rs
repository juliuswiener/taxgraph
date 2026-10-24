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

/// Aendert eine bestehende Antwort: der Store verlangt die `event_id` des aktiven Events als `ersetzt`.
async fn aendere(d: &Dienst, feld: &str, wert: Value) {
    let (status, stand) = sende(d, "GET", "/fall/reh/stand", None).await;
    assert_eq!(status, 200, "GET /stand: {stand}");
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
        "ts": "2026-01-02T00:00:00+00:00",
        "ersetzt": stand["felder"][feld]["event_id"].clone(),
    });
    let (status, antwort) = sende(d, "POST", "/fall/reh/event", Some(&rumpf)).await;
    assert_eq!(status, 201, "POST (aendern) {feld}: {antwort}");
}

/// Schreibt eine VORLAEUFIGE Antwort (ohne zweites Signal), wie ein Vorjahreswert oder ein KI-Vorschlag. Das Feld hat noch kein Ereignis.
async fn vorlaeufig(d: &Dienst, feld: &str, wert: Value) {
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "vorlaeufig", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": Value::Null},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    let (status, antwort) = sende(d, "POST", "/fall/reh/event", Some(&rumpf)).await;
    assert_eq!(status, 201, "POST (vorlaeufig) {feld}: {antwort}");
}

async fn neuer_fall_in(scheibe: &str) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "reh", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    d
}

async fn neuer_fall() -> Dienst {
    neuer_fall_in("rentner_gesamt").await
}

async fn fall_in(scheibe: &str, paare: &[(&'static str, Value)]) -> Dienst {
    let d = neuer_fall_in(scheibe).await;
    for (feld, wert) in paare {
        antworte(&d, feld, wert.clone()).await;
    }
    d
}

async fn fall_mit(paare: &[(&'static str, Value)]) -> Dienst {
    fall_in("rentner_gesamt", paare).await
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

/// Der Sperrgrund, den `GET /fragen` meldet (`null`, wenn der Guard nicht sperrt).
async fn ring_gesperrt(d: &Dienst) -> Value {
    let (status, antwort) = sende(d, "GET", "/fall/reh/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {antwort}");
    antwort["ring_gesperrt"].clone()
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

/// Wie [`mit`], aber eine Antwort aus `basis` zu demselben Feld entfaellt (der Dienst nimmt je Feld nur ein aktives Ereignis).
fn ueberschreibe(mut basis: Paare, mehr: Paare) -> Paare {
    basis.retain(|(f, _)| !mehr.iter().any(|(g, _)| g == f));
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
/// die Begriffe, an denen ein Rentner sein "ja" erkennt (Minijob, Nebenjob, Ruhegehalt, Werkspension, Witwen- und
/// Waisengeld) und sagen, dass die gesetzliche Rente allein "nein" heisst. Seit Nr. 53 steht dort die Werkspension statt der
/// Betriebsrente: eine Rente aus Pensionskasse, Pensionsfonds oder Direktversicherung ist keine Pension (`renten_leistungsmitteilung_texte.rs`).
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
        for wort in ["Arbeitslohn", "Pension", "Minijob", "Nebenjob", "Ruhegehalt", "Werkspension", "Witwen", "Waisen"] {
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
    aendere(&d, "kein_lohn_pension", json!(false)).await;
    aendere(&d, "kein_lohn_pension_partner", json!(false)).await;
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
/// Feld stehen (Lohn ungleich 0, Art = `altersgrenze_sonstige`); nur das Kreuz verbirgt es.
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
    let (_, stand) = sende(&d, "GET", "/fall/reh/stand", None).await;
    assert_eq!(
        stand["felder"]["bruttoarbeitslohn_partner"]["wert"],
        json!(6_000_000),
        "der gespeicherte Lohn ist nach dem Kreuz nicht mehr im Store: {stand}"
    );
    // Person A, gleiches Verhalten.
    let a_lohn = mit(rentner(4_000_000, "einzel"), vec![("bruttoarbeitslohn", json!(3_000_000)), ("steuerklasse", json!("1"))]);
    let a_ohne = zahl(&fall_mit(&rentner(4_000_000, "einzel")).await).await;
    let a_offen = zahl(&fall_mit(&a_lohn).await).await;
    assert!(a_ohne < a_offen, "der Lohn von Person A zaehlt nicht: {a_ohne} gegen {a_offen}");
    let d = fall_mit(&a_lohn).await;
    antworte(&d, "kein_lohn_pension", json!(true)).await;
    assert_eq!(zahl(&d).await, a_offen, "das Kreuz von Person A hat die Rechnung geaendert");
}

// ---------------------------------------------------------------- Fragenzahl (AK5) ----------------------------------------

/// AK5: Die Fragenzahl vor und nach den Antworten. Die Kreuze selbst verlassen die Liste, sobald sie beantwortet sind.
/// "Nein" spart 8 Fragen je Person (Ehegatte: 10); "ja" kostet nichts gegenueber heute (143 und 192). Seit der Folgefrage
/// `dba_staat_sonstiger` (Abweichung Nr. 58) steht je Fall eine Frage mehr in der Liste (144 und 193).
/// Seit Abweichung Nr. 54 haengen die Fragen zum privaten Verkauf am Kreuz `kein_p23_verkauf` (hier "nein"), nicht mehr an
/// `kein_sonstige`: vier Fragen weniger in jedem Fall (147 -> 143, 196 -> 192).
/// Seit Abweichung Nr. 55 kommen das Kreuz `kein_rav_bav` und die Zaehlfrage `rav_bav_anzahl` mit den sieben Feldern der Gruppe
/// `rente_av_bav` dazu: neun Fragen mehr in jedem Fall (die Frage nach der Rente mit Leistungsmitteilung ist noch offen).
/// Seit Abweichung Nr. 92 stehen die drei Zuschussfragen des Ehegatten in der Liste, solange `mit_anspruch_auf_zuschuss_partner` offen
/// ist: drei Fragen mehr bei Zusammenveranlagung (204 -> 207 und so fort). Die eigenen drei fehlen, weil `rentner()` den Anspruch verneint.
/// Seit Fluss J F2/F3/F8 (2026-10-21) stehen die Kreuze `kein_kind`, `kein_unterhalt` und `keine_auslandseinkuenfte` in der Liste, bis sie
/// beantwortet sind (drei Fragen mehr in jedem Fall), bei Zusammenveranlagung dazu `kein_gewinn_partner` (vier mehr; 207 -> 211, 154 -> 157).
/// Ihre Bloecke ("nein") stehen in `nein_verbirgt_den_block_eines_kreuzes_und_nur_ihn`.
/// Seit Fragen-Reduktion Teil 2, Phase 2 (2026-10-22) stehen die Kreuze `keine_behinderung_pflege`, `keine_energetische_sanierung` und
/// `keine_spenden` in der Liste, bis sie beantwortet sind: drei Fragen mehr in jedem Fall (157 -> 160, 211 -> 214). Ihre Bloecke ("nein")
/// stehen ebenfalls in `nein_verbirgt_den_block_eines_kreuzes_und_nur_ihn`.
/// Seit d-53 (2026-10-22) stehen bei Zusammenveranlagung auch die Zeilen `kein_kap_partner` und `kein_sonstige_partner` in der Liste, bis sie
/// beantwortet sind: zwei Fragen mehr in jedem Fall mit Ehegatten (214 -> 216 und so fort); ihre Bloecke ("nein") stehen in
/// `rentner_ehegatten_kreuze.rs`.
/// Seit Plan Nr. 116 (2026-10-24, Julius d-67) steht die Frage `p35a_mitveranlagung` nur noch Nicht-Zusammenveranlagten
/// (`feld_bedingung veranlagung wert_nicht zusammen`): eine Frage weniger in jedem Fall mit Ehegatten (216 -> 215 und so fort), die
/// drei Einzel-Faelle bleiben gleich.
#[tokio::test]
async fn die_fragenzahl_vor_und_nach_den_antworten() {
    let faelle = [
        ("einzel offen", "einzel", None, None, 160),
        ("einzel nein", "einzel", Some(true), None, 151),
        ("einzel ja", "einzel", Some(false), None, 159),
        ("zusammen offen", "zusammen", None, None, 215),
        ("zusammen nein/nein", "zusammen", Some(true), Some(true), 195),
        ("zusammen nein/ja", "zusammen", Some(true), Some(false), 205),
        ("zusammen ja/nein", "zusammen", Some(false), Some(true), 203),
        ("zusammen ja/ja", "zusammen", Some(false), Some(false), 213),
        ("zusammen ja/offen", "zusammen", Some(false), None, 214),
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

// ---------------------------------------------------------------- Ankreuzliste: Kinder, Unterhalt, Ausland, Gewinn des Ehegatten -------------

// Fluss J F2/F3/F8 (2026-10-21, Vault `julius-entscheidet-vorjahr-sackgasse-rentner-eingangsfragen-und-bic-werden-behoben-2026-10-21`):
// Die Rentner-Scheibe fragt vier Kreuze, die `gesamt` schon fragt: `kein_kind`, `kein_unterhalt`, `keine_auslandseinkuenfte` (Person A) und
// `kein_gewinn_partner` (Ehegatte). "Nein" (Wert `true`) verbirgt den Block dahinter. Wie bei `kein_lohn_pension` aendert das Kreuz nur die
// Liste, nie die Rechnung. `kein_sonstige_partner` und `kein_kap_partner` (beide aus `FLAG_NEGIERT`) fehlten hier bis zum 2026-10-22 (Sackgasse in
// `rente_instanz_offen`, Flag-Sperre); seit d-53 stehen sie in der Liste, ihre Tests (anderer Aufbau wegen der Sperren) in `rentner_ehegatten_kreuze.rs`.

/// Die Felder hinter `kein_kind` (die Bindung gibt sie her; dieser Test haelt sie einzeln fest, damit eine entfernte Bedingung am
/// Feldnamen auffaellt): Familie, Kinder, Betreuung, Schulgeld, Kranken- und Pflegeversicherung der Kinder.
const KIND_BLOCK: [&str; 36] = [
    "fam_alleinstehend",
    "fam_anzahl_kinder",
    "fam_monate_ohne_voraussetzung",
    "p33a_ausbildung_anzahl_kinder",
    "kind_vorname",
    "kind_idnr",
    "kind_kindschaftsverh_zeitraum_a",
    "kind_geburtsdatum",
    "kind_wohnsitz_inland_zeitraum",
    "kind_familienkasse",
    "kind_kindschaftsverh_zeitraum_b",
    "kind_kindschaftsverhaeltnis_a",
    "kind_kindschaftsverhaeltnis_b",
    "kind_anderer_elternteil_tod_am",
    "kind_anderer_elternteil_name",
    "kind_anderer_elternteil_geburtsdatum",
    "kind_anderer_elternteil_kindschaftsverhaeltnis",
    "kind_anderer_elternteil_zeitraum",
    "kind_anderer_elternteil_ausland_zeitraum",
    "kind_behinderten_pb_antrag",
    "kind_pb_nicht_selbst_genutzt",
    "kind_kv",
    "kind_pv",
    "schulgeld",
    "kinderbetreuungskosten",
    "kind_betreuung_rechnung_ueberweisung",
    "kind_betreuung_reine_betreuung",
    "kind_betreuung_kein_gemeinsamer_haushalt_zeitraum",
    "kind_betreuung_haushaltszugehoerigkeit_zeitraum",
    "kind_unter_14_haushaltszugehoerig",
    "kind_betreuung_dienstleister",
    "kind_betreuung_zeitraum",
    "kind_betreuung_einzelbetrag",
    "kind_betreuung_eigenanteil",
    "kind_betreuung_eigenanteil_betrag",
    "kind_betreuung_eigenanteil_zeitraum",
];

/// Die 17 Felder hinter `kein_unterhalt`: Name, Anschrift, Vermoegen und Einkuenfte der unterstuetzten Person (§ 33a Abs. 1 `EStG`).
const UNTERHALT_BLOCK: [&str; 17] = [
    "p33a_andere_einkuenfte_bezuege",
    "p33a_haushalt_anschrift",
    "p33a_unterhalt_aufwendungen",
    "p33a_zahlungszeitraum",
    "p33a_haushalt_personenzahl",
    "p33a_unterstuetzungszeitraum",
    "p33a_person_name",
    "p33a_person_beruf_familienstand",
    "p33a_person_geburtsdatum",
    "p33a_person_idnr",
    "p33a_verwandtschaftsverhaeltnis",
    "p33a_person_im_inlaendischen_haushalt",
    "p33a_kindergeld_anspruch",
    "p33a_person_hat_vermoegen",
    "p33a_person_hat_einkuenfte",
    "p33a_unterhalt_kv_pv",
    "p33a_weitere_person_beteiligt",
];

/// Die acht Felder hinter `keine_auslandseinkuenfte` (§ 34c `EStG`).
const AUSLAND_BLOCK: [&str; 8] = [
    "dba_abzug_statt_anrechnung",
    "dba_einkunftsart",
    "dba_methode",
    "dba_staat",
    "dba_auslaendische_einkuenfte",
    "dba_gezahlte_auslaendische_steuer",
    "dba_mehrere_staaten",
    "dba_staat_sonstiger",
];

/// Die elf Gewinnfelder des Ehegatten hinter `kein_gewinn_partner`.
const GEWINN_PARTNER_BLOCK: [&str; 11] = [
    "gewinn_betriebsart_partner",
    "rentner_veraeusserungs_betriebsart_partner",
    "einkuenfte_gewinn_partner",
    "gewinn_bezeichnung_partner",
    "gewinnanteil_partner",
    "gewst_hebesatz_partner",
    "gewst_messbetrag_partner",
    "rentner_veraeusserungsgewinn_partner",
    "verguetung_darlehen_partner",
    "verguetung_taetigkeit_partner",
    "verguetung_ueberlassung_partner",
];

// Fragen-Reduktion Teil 2, Phase 2 (F10, 2026-10-22): die Rentner-Scheibe fragt auch `keine_behinderung_pflege`, `keine_energetische_sanierung`
// und `keine_spenden`. Die Bloecke sind die Felder, die `rentner()` offen laesst; `rentner_grad_der_behinderung` und `rentner_hilflos_blind_taubblind`
// beantwortet `rentner()` schon (Kegel), sie prueft `nein_auf_behinderung_und_pflege_verbirgt_auch_die_zwei_felder_des_kegels`.

/// Die elf offenen Felder hinter `keine_behinderung_pflege`: die Pflege (§ 33b Abs. 6; die ganze Regel `p33b_pflege_pauschbetrag`), die zwei
/// Fahrtkostenfragen (§ 33 Abs. 2a), der Grad und die Merkzeichen der Kinder (§ 33b Abs. 5) und die zwei Fragen zu den behinderungsbedingten
/// Aufwendungen (§ 33b Abs. 1, `feld_bedingung` am Kreuz).
const BEHINDERUNG_PFLEGE_BLOCK: [&str; 11] = [
    "rentner_gepflegter_wohnsitz_inland",
    "rentner_pflege_durch",
    "rentner_gepflegter_idnr",
    "rentner_gepflegter_angaben",
    "rentner_pflege_weitere_personen",
    "fahrtkosten_pausch_gdb80_oder_70g",
    "fahrtkosten_pausch_ag_bl_tbl_h",
    "kind_grad_der_behinderung",
    "kind_hilflos_blind_taubblind",
    "behinderungsbedingte_aufwendungen",
    "behinderungsbedingte_aufwendungen_wahlrecht_pb",
];

/// Die zwoelf Felder hinter `keine_energetische_sanierung` (§ 35c `EStG`): die Regeln `p35c_sanierung_ermaessigung` und
/// `p35c_energieberater_ermaessigung` ganz. Ein Fragenlauf mit neutralen Antworten (`korrektur_sweep`) fragt nur drei davon
/// (`p35c_bereits_ermaessigung_frueher`, `p35c_keine_doppelfoerderung`, `p35c_energieberater_aufwendungen`), die offene Liste hat alle zwoelf.
const SANIERUNG_BLOCK: [&str; 12] = [
    "p35c_sanierungsaufwendungen",
    "p35c_ist_uebernaechstes_foerderjahr",
    "p35c_keine_doppelfoerderung",
    "p35c_objekt_strasse",
    "p35c_objekt_plz_ort",
    "p35c_gebaeude_herstellungsbeginn",
    "p35c_baubeginn_massnahme",
    "p35c_gesamtflaeche_qm",
    "p35c_eigene_wohnflaeche_qm",
    "p35c_bereits_ermaessigung_frueher",
    "p35c_massnahme_art",
    "p35c_energieberater_aufwendungen",
];

/// Die drei Felder hinter `keine_spenden` (§ 10b `EStG`).
const SPENDEN_BLOCK: [&str; 3] = ["spenden_betrag", "parteispenden_betrag", "waehlervereinigungen_betrag"];

/// Kreuz, Veranlagung des Falls und Block: die sieben neuen Kreuze (vier aus Fluss J, drei aus der Fragen-Reduktion Teil 2).
const NEUE_KREUZE: [(&str, &str, &[&str]); 7] = [
    ("kein_kind", "einzel", &KIND_BLOCK),
    ("kein_unterhalt", "einzel", &UNTERHALT_BLOCK),
    ("keine_auslandseinkuenfte", "einzel", &AUSLAND_BLOCK),
    ("kein_gewinn_partner", "zusammen", &GEWINN_PARTNER_BLOCK),
    ("keine_behinderung_pflege", "einzel", &BEHINDERUNG_PFLEGE_BLOCK),
    ("keine_energetische_sanierung", "einzel", &SANIERUNG_BLOCK),
    ("keine_spenden", "einzel", &SPENDEN_BLOCK),
];

fn menge(ids: &[String]) -> std::collections::BTreeSet<&str> {
    ids.iter().map(String::as_str).collect()
}

/// Die Kreuze stehen in der Fragenliste, so wie sie `gesamt` zeigt: Ja/Nein-Frage, `frage_invertiert` (das Feld nennt die
/// Abwesenheit, die Frage fragt nach der Anwesenheit) und `screening` (die Ankreuzliste am Anfang zeigt sie; wer nichts ankreuzt,
/// antwortet "nein"). Die Polaritaet steht an der Frage, die der Dienst liefert, nicht am Feldnamen.
#[tokio::test]
async fn die_neuen_kreuze_sind_invertierte_ankreuzfragen_der_liste() {
    let liste = fragen(&neuer_fall().await).await;
    for (kreuz, _, _) in NEUE_KREUZE {
        let q = liste
            .iter()
            .find(|q| q["feld_id"] == json!(kreuz))
            .unwrap_or_else(|| panic!("{kreuz} fehlt in der Fragenliste der Rentner-Scheibe"));
        assert_eq!(q["typ"], json!("bool"), "{kreuz}");
        assert_eq!(q["frage_invertiert"], json!(true), "{kreuz}: das Feld nennt die Abwesenheit");
        assert_eq!(q["screening"], json!(true), "{kreuz}: die Ankreuzliste zeigt es");
        assert!(q["fragetext_laie"].as_str().is_some_and(|t| !t.is_empty()), "{kreuz}: ohne Fragetext");
        // Die Zeile des Ehegatten nennt, worauf sie wartet (F5); die sechs Kreuze von Person A tragen den Schluessel nicht.
        let wartet = (kreuz == "kein_gewinn_partner").then(|| json!({"feld": "veranlagung", "wert": "zusammen"}));
        assert_eq!(q.get("wartet_auf"), wartet.as_ref(), "{kreuz}: wartet_auf");
    }
    // Genau die drei Zeilen des Ehegatten der Rentner-Liste warten auf die Veranlagung; die vierte der Scheibe `gesamt`
    // (`keine_behinderung_pflege_partner`) steht hier nicht, und keine andere Frage der Liste traegt den Schluessel.
    let wartende: std::collections::BTreeSet<&str> =
        liste.iter().filter(|q| q.get("wartet_auf").is_some()).filter_map(|q| q["feld_id"].as_str()).collect();
    assert_eq!(wartende, std::collections::BTreeSet::from(["kein_gewinn_partner", "kein_kap_partner", "kein_sonstige_partner"]));
}

/// Die Zeile des Ehegatten steht bei Zusammenveranlagung und solange `veranlagung` offen ist (fail-closed, wie in `gesamt`: die
/// Ankreuzliste erscheint vor der Frage nach der Veranlagung), und fehlt bei Einzelveranlagung. Die drei Kreuze von Person A stehen immer.
#[tokio::test]
async fn die_zeile_des_ehegatten_fehlt_bei_einzelveranlagung() {
    let einzel = fragen_ids(&fall_mit(&rentner(2_000_000, "einzel")).await).await;
    let zusammen = fragen_ids(&fall_mit(&rentner(2_000_000, "zusammen")).await).await;
    let offen = fragen_ids(&neuer_fall().await).await;
    assert!(pos(&einzel, "kein_gewinn_partner").is_none(), "Zeile des Ehegatten bei Einzelveranlagung");
    assert!(pos(&zusammen, "kein_gewinn_partner").is_some(), "Zeile des Ehegatten fehlt bei Zusammenveranlagung");
    assert!(pos(&offen, "kein_gewinn_partner").is_some(), "Zeile des Ehegatten fehlt, solange die Veranlagung offen ist");
    for kreuz in ["kein_kind", "kein_unterhalt", "keine_auslandseinkuenfte", "keine_behinderung_pflege", "keine_energetische_sanierung", "keine_spenden"] {
        for (name, ids) in [("einzel", &einzel), ("zusammen", &zusammen), ("offen", &offen)] {
            assert!(pos(ids, kreuz).is_some(), "{kreuz} fehlt bei {name}");
        }
    }
}

/// "Nein" verbirgt genau den Block des Kreuzes und das Kreuz selbst, sonst nichts. Vorher stehen alle Felder des Blocks in der Liste.
#[tokio::test]
async fn nein_verbirgt_den_block_eines_kreuzes_und_nur_ihn() {
    for (kreuz, veranlagung, block) in NEUE_KREUZE {
        let d = fall_mit(&rentner(2_000_000, veranlagung)).await;
        let vorher = fragen_ids(&d).await;
        assert!(fehlende(&vorher, block).is_empty(), "{kreuz}: vor der Antwort fehlen {:?}", fehlende(&vorher, block));
        assert!(pos(&vorher, kreuz).is_some(), "{kreuz} fehlt vor der Antwort");
        antworte(&d, kreuz, json!(true)).await;
        let nachher = fragen_ids(&d).await;
        assert_eq!(vorhandene(&nachher, block), Vec::<&str>::new(), "{kreuz}: trotz 'nein' gefragt");
        let weg: std::collections::BTreeSet<&str> = menge(&vorher).difference(&menge(&nachher)).copied().collect();
        let soll: std::collections::BTreeSet<&str> = block.iter().copied().chain([kreuz]).collect();
        assert_eq!(weg, soll, "{kreuz}: es verschwindet mehr oder weniger als der Block und das Kreuz");
    }
}

/// "Ja" (`false`) laesst den Block stehen; der Wechsel von "nein" auf "ja" zeigt ihn wieder.
#[tokio::test]
async fn ja_laesst_den_block_stehen_und_der_wechsel_von_nein_auf_ja_zeigt_ihn_wieder() {
    for (kreuz, veranlagung, block) in NEUE_KREUZE {
        let d = fall_mit(&rentner(2_000_000, veranlagung)).await;
        antworte(&d, kreuz, json!(false)).await;
        let ja = fragen_ids(&d).await;
        assert!(fehlende(&ja, block).is_empty(), "{kreuz}: nach 'ja' fehlen {:?}", fehlende(&ja, block));
        let d = fall_mit(&rentner(2_000_000, veranlagung)).await;
        antworte(&d, kreuz, json!(true)).await;
        assert!(vorhandene(&fragen_ids(&d).await, block).is_empty(), "{kreuz}: 'nein' wirkt nicht");
        aendere(&d, kreuz, json!(false)).await;
        let wieder = fragen_ids(&d).await;
        assert!(fehlende(&wieder, block).is_empty(), "{kreuz}: nach dem Wechsel fehlen {:?}", fehlende(&wieder, block));
    }
}

/// Jedes Kreuz steht vor den Feldern, die es abschaltet (sonst beantwortet der Nutzer die Details, bevor er das Kreuz sieht). Ausnahme:
/// die zwei Fragen zu den behinderungsbedingten Aufwendungen ordnet die Warteschlange vor `keine_behinderung_pflege` (Position 17 gegen 21).
/// Die Oberflaeche stoert das nicht: ihre Ankreuzliste sammelt jede Frage mit `screening` und steht vor der ersten Detailfrage
/// (`zeigeScreening` in `app.js`); `die_neuen_kreuze_sind_invertierte_ankreuzfragen_der_liste` haelt `screening` fest.
#[tokio::test]
async fn jedes_neue_kreuz_steht_vor_den_feldern_die_es_abschaltet() {
    const NACH_DEM_KREUZ_ERLAUBT: [&str; 2] = ["behinderungsbedingte_aufwendungen", "behinderungsbedingte_aufwendungen_wahlrecht_pb"];
    for (kreuz, veranlagung, block) in NEUE_KREUZE {
        let ids = fragen_ids(&fall_mit(&rentner(2_000_000, veranlagung)).await).await;
        let k = pos(&ids, kreuz).unwrap_or_else(|| panic!("{kreuz} fehlt in der Liste"));
        for f in block.iter().filter(|f| !NACH_DEM_KREUZ_ERLAUBT.contains(f)) {
            let p = pos(&ids, f).unwrap_or_else(|| panic!("{f} fehlt ohne Antwort auf {kreuz}"));
            assert!(k < p, "{kreuz}@{k} steht nicht vor {f}@{p}");
        }
    }
}

/// Keine Sperre durch die neuen Kreuze: mit "nein" auf alle gibt es eine bestaetigte Zahl, ohne Zusatzfrage und ohne neuen Sperrgrund
/// (`sperrgrund_kreuz_sweep` misst das ueber alle Kontexte).
#[tokio::test]
async fn nein_auf_alle_neuen_kreuze_gibt_eine_bestaetigte_zahl() {
    for veranlagung in ["einzel", "zusammen"] {
        let ohne_kreuze = ergebnis(&fall_mit(&rentner(4_000_000, veranlagung)).await).await;
        let d = fall_mit(&rentner(4_000_000, veranlagung)).await;
        for (kreuz, v, _) in NEUE_KREUZE {
            if v == veranlagung || veranlagung == "zusammen" {
                antworte(&d, kreuz, json!(true)).await;
            }
        }
        let e = ergebnis(&d).await;
        assert!(e["zahl_cent"].is_i64(), "{veranlagung}: keine Zahl nach 'nein': {e}");
        assert_eq!(e["grund"], json!("bestaetigt"), "{veranlagung}: {e}");
        assert_eq!(e["zahl_cent"], ohne_kreuze["zahl_cent"], "{veranlagung}: das Kreuz hat die Rechnung geaendert");
    }
}

/// Der Grad der Behinderung und die Merkzeichen von Person A stehen im Kegel (`rentner()` beantwortet sie). `keine_behinderung_pflege`
/// verbirgt beide: bei "nein" verlassen sie die Liste, und die Zahl steht trotzdem (keine Sackgasse ueber `input_kegel_nicht_bestaetigt`).
/// Sie ist die Zahl mit den Antworten 0 und "nein"; "ja" holt die Fragen zurueck.
#[tokio::test]
async fn nein_auf_behinderung_und_pflege_verbirgt_auch_die_zwei_felder_des_kegels() {
    const KEGEL_BEHINDERUNG: [&str; 2] = ["rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind"];
    let ohne_kegel: Paare = rentner(4_000_000, "einzel").into_iter().filter(|(f, _)| !KEGEL_BEHINDERUNG.contains(f)).collect();
    let d = fall_mit(&ohne_kegel).await;
    let vorher = fragen_ids(&d).await;
    assert!(fehlende(&vorher, &KEGEL_BEHINDERUNG).is_empty(), "vor der Antwort fehlen: {:?}", fehlende(&vorher, &KEGEL_BEHINDERUNG));
    antworte(&d, "keine_behinderung_pflege", json!(true)).await;
    let nachher = fragen_ids(&d).await;
    assert_eq!(vorhandene(&nachher, &KEGEL_BEHINDERUNG), Vec::<&str>::new(), "trotz 'nein' gefragt");
    let e = ergebnis(&d).await;
    assert!(e["zahl_cent"].is_i64(), "keine Zahl nach 'nein' ohne die Kegel-Antworten: {e}");
    assert_eq!(e["grund"], json!("bestaetigt"), "{e}");
    let mit_antworten = ergebnis(&fall_mit(&rentner(4_000_000, "einzel")).await).await;
    assert_eq!(e["zahl_cent"], mit_antworten["zahl_cent"], "die Zahl ohne die Kegel-Antworten weicht von der mit 0 und 'nein' ab");
    aendere(&d, "keine_behinderung_pflege", json!(false)).await;
    let ja = fragen_ids(&d).await;
    assert!(fehlende(&ja, &KEGEL_BEHINDERUNG).is_empty(), "nach 'ja' fehlen: {:?}", fehlende(&ja, &KEGEL_BEHINDERUNG));
}

/// Ein gespeicherter Wert zaehlt weiter, auch wenn das Kreuz auf "nein" steht (wie bei `kein_lohn_pension`): das Kreuz aendert nur die
/// Liste, nie die Rechnung. Je Block ein Wert, der die Steuer bewegt; das Kreuz laesst die Zahl, wie sie ist; ohne den Wert ist sie
/// eine andere (sonst belegte die Gleichheit nichts).
#[tokio::test]
async fn ein_gespeicherter_wert_zaehlt_trotz_nein_weiter_in_jedem_neuen_block() {
    let ausland: Paare = vec![
        ("dba_staat", json!("Frankreich")),
        ("dba_einkunftsart", json!("dividenden")),
        ("dba_methode", json!("kein_dba")),
        ("dba_auslaendische_einkuenfte", json!(500_000)),
        ("dba_gezahlte_auslaendische_steuer", json!(100_000)),
        ("dba_mehrere_staaten", json!(false)),
    ];
    let faelle: [(&str, &str, i64, Paare); 7] = [
        ("kein_kind", "einzel", 2_000_000, vec![("schulgeld", json!(600_000))]),
        ("kein_unterhalt", "einzel", 2_000_000, vec![("p33a_unterhalt_aufwendungen", json!(300_000))]),
        ("keine_auslandseinkuenfte", "einzel", 2_000_000, ausland),
        ("kein_gewinn_partner", "zusammen", 4_000_000, vec![("einkuenfte_gewinn_partner", json!(3_000_000))]),
        // `rentner()` beantwortet den Grad mit 0; der gespeicherte Wert ersetzt diese Antwort.
        ("keine_behinderung_pflege", "einzel", 4_000_000, vec![("rentner_grad_der_behinderung", json!(100))]),
        (
            "keine_energetische_sanierung",
            "einzel",
            4_000_000,
            vec![("p35c_keine_doppelfoerderung", json!(true)), ("p35c_energieberater_aufwendungen", json!(500_000))],
        ),
        ("keine_spenden", "einzel", 4_000_000, vec![("spenden_betrag", json!(300_000))]),
    ];
    for (kreuz, veranlagung, rente, wert) in faelle {
        let ohne = zahl(&fall_mit(&rentner(rente, veranlagung)).await).await;
        let mit_wert = ueberschreibe(rentner(rente, veranlagung), wert.clone());
        let offen = zahl(&fall_mit(&mit_wert).await).await;
        assert_ne!(ohne, offen, "{kreuz}: der Wert bewegt die Steuer nicht ({ohne} gegen {offen}), der Test belegt nichts");
        let d = fall_mit(&mit_wert).await;
        antworte(&d, kreuz, json!(true)).await;
        assert_eq!(zahl(&d).await, offen, "{kreuz}: das Kreuz hat die Rechnung geaendert");
        let (_, stand) = sende(&d, "GET", "/fall/reh/stand", None).await;
        for (feld, w) in &wert {
            assert_eq!(stand["felder"][*feld]["wert"], *w, "{kreuz}: {feld} ist nach dem Kreuz nicht mehr im Store");
        }
    }
}

/// "Ja" auf `keine_behinderung_pflege` ist in der Rentner-Scheibe ein neuer Zustand (das Kreuz stand vorher nicht in der Liste). Die Sperre
/// `fahrtkostenpauschale_offen` verlangt dann die zwei Fahrtkostenfragen; beide stehen in der Liste (keine Sackgasse), und mit ihren
/// Antworten gibt es die bestaetigte Zahl.
#[tokio::test]
async fn ja_auf_behinderung_und_pflege_fragt_die_fahrtkosten_und_gibt_dann_eine_zahl() {
    const FAHRTKOSTEN: [&str; 2] = ["fahrtkosten_pausch_ag_bl_tbl_h", "fahrtkosten_pausch_gdb80_oder_70g"];
    let d = fall_mit(&rentner(4_000_000, "einzel")).await;
    antworte(&d, "keine_behinderung_pflege", json!(false)).await;
    let gesperrt = ergebnis(&d).await;
    assert_eq!(gesperrt["grund"], json!("fahrtkostenpauschale_offen"), "{gesperrt}");
    assert_eq!(gesperrt["zahl_cent"], Value::Null, "{gesperrt}");
    let ids = fragen_ids(&d).await;
    assert!(fehlende(&ids, &FAHRTKOSTEN).is_empty(), "die Sperre verlangt Felder, die nicht in der Liste stehen: {:?}", fehlende(&ids, &FAHRTKOSTEN));
    for f in FAHRTKOSTEN {
        antworte(&d, f, json!(false)).await;
    }
    let e = ergebnis(&d).await;
    assert!(e["zahl_cent"].is_i64(), "keine Zahl nach 'ja' und beantworteten Fahrtkostenfragen: {e}");
    assert_eq!(e["grund"], json!("bestaetigt"), "{e}");
}

// ---------------------------------------------------------------- die Sperrfrage hinter dem Kreuz -------------------------

/// Ein gespeicherter Wert widerspricht dem Kreuz "nein" und loest eine Sperre aus, deren Frage das Kreuz verbirgt. Der Wert gewinnt
/// (Entscheidung vom 2026-10-09, Punkt 3), die Sperre bleibt, und die Frage, die sie verlangt, steht trotzdem in der Fragenliste
/// (Julius, 2026-10-22: "Haelt eine Sperre die Zahl zurueck, muss die zugehoerige Frage sichtbar sein"). Gemessen am Dienst, in
/// `rentner_gesamt` und in `gesamt`: erst ohne Kreuz (die Sperre haelt, die Frage steht in der Liste), dann mit Kreuz (die Sperre haelt,
/// die Frage steht einmal in der Liste), dann nach einer vorlaeufigen Antwort (die Sperre haelt, die Frage bleibt: vorlaeufig ist offen),
/// dann nach der bestaetigten Antwort (die Sperre ist weg, die Frage kommt nicht wieder).
/// `basis` traegt die Antworten des Falls ohne Kreuz; das Kreuz kommt als letztes (der Wert steht VOR dem Kreuz).
async fn sperrfrage_bleibt_trotz_kreuz_nein(scheibe: &str, basis: Paare, kreuz: &str, frage: &str, grund: &str) -> Dienst {
    let d = fall_in(scheibe, &basis).await;
    assert_eq!(ring_gesperrt(&d).await, json!(grund), "{scheibe}: ohne Kreuz sperrt der gespeicherte Wert nicht mit {grund}");
    let vorher = fragen_ids(&d).await.iter().filter(|i| *i == frage).count();
    assert_eq!(vorher, 1, "{scheibe}: ohne Kreuz steht die Frage {frage} {vorher}-mal in der Liste (erwartet: einmal)");
    antworte(&d, kreuz, json!(true)).await;
    assert_eq!(ring_gesperrt(&d).await, json!(grund), "{scheibe}: der gespeicherte Wert gewinnt gegen das Kreuz, die Sperre bleibt");
    let ids = fragen_ids(&d).await;
    assert_eq!(
        ids.iter().filter(|i| *i == frage).count(),
        1,
        "{scheibe}: die Sperre {grund} verlangt {frage}, die Liste stellt sie nicht genau einmal (Kreuz {kreuz} = nein)"
    );
    // Eine vorlaeufige Antwort (Vorjahr, KI-Vorschlag) loest die Sperre nicht und ist keine Antwort: die Frage bleibt sichtbar.
    vorlaeufig(&d, frage, json!(true)).await;
    assert_eq!(ring_gesperrt(&d).await, json!(grund), "{scheibe}: eine vorlaeufige Antwort auf {frage} loest die Sperre {grund}");
    let ids = fragen_ids(&d).await;
    assert_eq!(
        ids.iter().filter(|i| *i == frage).count(),
        1,
        "{scheibe}: nach einer vorlaeufigen Antwort steht {frage} nicht genau einmal in der Liste (Sperre {grund})"
    );
    aendere(&d, frage, json!(true)).await;
    assert_ne!(ring_gesperrt(&d).await, json!(grund), "{scheibe}: die Antwort auf {frage} loest die Sperre {grund} nicht");
    assert!(!fragen_ids(&d).await.iter().any(|i| i == frage), "{scheibe}: {frage} wird nach der Antwort noch einmal gefragt");
    d
}

/// Die bestaetigte Zahl nach der Antwort: in `rentner_gesamt` ist der Kegel beantwortet, dort gibt es sie.
async fn zahl_ist_bestaetigt(d: &Dienst) {
    let e = ergebnis(d).await;
    assert!(e["zahl_cent"].is_i64(), "keine Zahl nach der Antwort auf die Sperrfrage: {e}");
    assert_eq!(e["grund"], json!("bestaetigt"), "{e}");
}

/// AK1 (S1, `behinderungsbedingte_aufwendungen_wahlrecht_offen`): ein gespeicherter Grad der Behinderung ab 20 mit behinderungsbedingten
/// Aufwendungen verlangt die Wahl zwischen Pauschbetrag und Einzelnachweis; das Kreuz `keine_behinderung_pflege` = nein verbirgt die Frage.
#[tokio::test]
async fn sperrfrage_bleibt_trotz_kreuz_nein_behinderung() {
    const KREUZ: &str = "keine_behinderung_pflege";
    const FRAGE: &str = "behinderungsbedingte_aufwendungen_wahlrecht_pb";
    const GRUND: &str = "behinderungsbedingte_aufwendungen_wahlrecht_offen";
    let werte: Paare = vec![("rentner_grad_der_behinderung", json!(100)), ("behinderungsbedingte_aufwendungen", json!(200_000))];
    let rentner_fall = ueberschreibe(rentner(4_000_000, "einzel"), werte.clone());
    let d = sperrfrage_bleibt_trotz_kreuz_nein("rentner_gesamt", rentner_fall, KREUZ, FRAGE, GRUND).await;
    zahl_ist_bestaetigt(&d).await;
    let gesamt_fall = mit(vec![("veranlagung", json!("einzel"))], werte);
    sperrfrage_bleibt_trotz_kreuz_nein("gesamt", gesamt_fall, KREUZ, FRAGE, GRUND).await;
}

/// AK2 (S2, `p35c_doppelfoerderung_offen`): gespeicherte Energieberater-Aufwendungen ohne Antwort zur Doppelfoerderung; das Kreuz
/// `keine_energetische_sanierung` = nein verbirgt die Frage `p35c_keine_doppelfoerderung`.
#[tokio::test]
async fn sperrfrage_bleibt_trotz_kreuz_nein_energetische_sanierung() {
    const KREUZ: &str = "keine_energetische_sanierung";
    const FRAGE: &str = "p35c_keine_doppelfoerderung";
    const GRUND: &str = "p35c_doppelfoerderung_offen";
    let werte: Paare = vec![("p35c_energieberater_aufwendungen", json!(500_000))];
    let rentner_fall = ueberschreibe(rentner(4_000_000, "einzel"), werte.clone());
    let d = sperrfrage_bleibt_trotz_kreuz_nein("rentner_gesamt", rentner_fall, KREUZ, FRAGE, GRUND).await;
    zahl_ist_bestaetigt(&d).await;
    let gesamt_fall = mit(vec![("veranlagung", json!("einzel"))], werte);
    sperrfrage_bleibt_trotz_kreuz_nein("gesamt", gesamt_fall, KREUZ, FRAGE, GRUND).await;
}
