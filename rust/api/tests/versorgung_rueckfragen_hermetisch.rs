//! Die Rueckfragen zum Versorgungsbezug (Abweichung Nr. 66, Backlog `versorgung-rueckfrage-zweiter-bezug-sterbegeld-mehrjahr` AK3 und
//! AK4): die HTTP-Naht. Ohne `PARITY=1`, ohne Python, ohne Netz, ohne `ERiC`.
//!
//! **Worum es geht.** Vier Faelle verraten die Eingaben nicht: ein zweiter Versorgungsbezug, Sterbegeld und Verwandtes (Zeile 15),
//! Bezuege fuer mehrere Kalenderjahre (Zeile 16) und ein Bezug, der nicht das ganze Jahr lief (Zeile 14). Die Erklaerung traegt diese
//! Zeilen nicht. Darum fragt sie je Person danach; ein bestaetigtes Ja sperrt die Abgabe mit dem Text des Falls, Nein und Schweigen
//! aendern nichts.
//!
//! **Hier geprueft.** (1) Die Fragen stehen NUR bei einem bestaetigten Bezug ueber 0, die des Ehegatten NUR bei Zusammenveranlagung und
//! bestaetigtem Bezug des Ehegatten. (2) Der ganze Weg: Bezug angeben, die Frage sehen, Ja antworten, `POST /einreichen` antwortet 409
//! `deklaration_unvollstaendig` mit dem Grund unter dem Feld der Frage, auf Nein aendern, die Sperre ist weg. Die Sperre selbst und
//! ihre Grenzen pruefen `rust/elster` (Unit-Tests) und `rust/bescheid/tests/versorgung_sperre_faelle.rs`.
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

const A: &str = "versorgung_jahresrente";
const B: &str = "versorgung_jahresrente_partner";
/// Die vier Fragen von Person A und die des Ehegatten, in der Reihenfolge der Faelle.
const FRAGEN_A: [&str; 4] = ["versorgung_zweiter_bezug", "versorgung_sterbegeld", "versorgung_mehrere_jahre", "versorgung_nicht_ganzjaehrig"];
const FRAGEN_B: [&str; 4] = [
    "versorgung_zweiter_bezug_partner",
    "versorgung_sterbegeld_partner",
    "versorgung_mehrere_jahre_partner",
    "versorgung_nicht_ganzjaehrig_partner",
];

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

/// Ein Event ueber die echte Route `POST /event` (Nutzer-Klick); `bestaetigt = false` schreibt es vorlaeufig, ohne zweites Signal.
/// `ersetzt` ist die `event_id` des Events, das es ersetzt (eine geaenderte Antwort).
fn ereignis(feld: &str, wert: &Value, bestaetigt: bool, ersetzt: Option<&Value>) -> Value {
    json!({
        "feld_id": feld, "wert": wert,
        "zustand": if bestaetigt { "bestaetigt" } else { "vorlaeufig" },
        "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": bestaetigt.then(|| format!("ok@{feld}"))},
        "ts": "2026-01-01T00:00:00+00:00",
        "ersetzt": ersetzt.cloned().unwrap_or(Value::Null),
    })
}

/// Legt die vollstaendige Akte `rust/fixtures/e2e/<datei>.json` als Fall `vs` an. `ersetze`: ein Feld der Akte bekommt den Wert,
/// `null` entfernt es aus der Akte. Dann schreibt jedes Paar von `mehr` ueber `POST /event` (muss 201 sein).
async fn akte(datei: &str, ersetze: &[(&str, Value)], mehr: &[(&'static str, Value, bool)]) -> Dienst {
    let d = dienst();
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e").join(datei);
    let mut roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let ereignisse = roh["events"].as_array_mut().unwrap();
    for (feld, wert) in ersetze {
        if wert.is_null() {
            ereignisse.retain(|e| e["feld_id"] != json!(feld));
        } else {
            for e in ereignisse.iter_mut().filter(|e| e["feld_id"] == json!(feld)) {
                e["wert"] = wert.clone();
            }
        }
    }
    roh["fall_id"] = json!("vs");
    roh["user_id"] = json!("alice");
    std::fs::write(d.zustand.konfig.faelle.join("vs.json"), serde_json::to_vec(&roh).unwrap()).unwrap();
    for (feld, wert, bestaetigt) in mehr {
        let (status, antwort) = sende(&d, "POST", "/fall/vs/event", Some(&ereignis(feld, wert, *bestaetigt, None))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    d
}

/// Aendert eine bestehende Antwort: der Store verlangt die `event_id` des aktiven Events als `ersetzt`.
async fn aendere(d: &Dienst, feld: &str, wert: &Value) {
    let (status, stand) = sende(d, "GET", "/fall/vs/stand", None).await;
    assert_eq!(status, 200, "GET /stand: {stand}");
    let id = stand["felder"][feld]["event_id"].clone();
    let (status, antwort) = sende(d, "POST", "/fall/vs/event", Some(&ereignis(feld, wert, true, Some(&id)))).await;
    assert_eq!(status, 201, "POST (aendern) {feld}: {antwort}");
}

async fn fragen_ids(d: &Dienst) -> Vec<String> {
    let (status, antwort) = sende(d, "GET", "/fall/vs/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {antwort}");
    antwort["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|q| q["feld_id"].as_str().map(str::to_owned))
        .collect()
}

/// Welche der Rueckfragen `FRAGEN_A` und `FRAGEN_B` in der Fragenliste stehen: `(A, B)`.
async fn rueckfragen(d: &Dienst) -> (Vec<&'static str>, Vec<&'static str>) {
    let ids = fragen_ids(d).await;
    assert!(ids.len() > 8, "KONTROLLE: die Fragenliste ist nicht leer ({ids:?})");
    let da = FRAGEN_A.iter().copied().filter(|f| ids.iter().any(|i| i == f)).collect();
    let db = FRAGEN_B.iter().copied().filter(|f| ids.iter().any(|i| i == f)).collect();
    (da, db)
}

/// Der Bezug von Person A: `euro` Jahresbetrag und Bemessungsgrundlage, Beginn 2020, beamtenrechtlich (alle bestaetigt).
fn bezug_a(euro: i64, bestaetigt: bool) -> Vec<(&'static str, Value, bool)> {
    vec![
        (A, json!(euro * 100), bestaetigt),
        ("versorgung_bemessungsgrundlage", json!(euro * 100), true),
        ("versorgung_beginn_jahr", json!(2020), true),
        ("versorgung_art", json!("beamtenrechtlich"), true),
    ]
}

/// Der Bruttoarbeitslohn (Nr. 3 der Lohnsteuerbescheinigung) enthaelt den Bezug: ohne ihn sperrt der Bescheid mit `versorgung_ueber_lohn`
/// (`GET /fragen` antwortet dann 200 mit der Sperre, s. unten). Nur fuer die Akte `rentner.json`; `gesamt.json` fuehrt einen Lohn.
fn lohn_a(euro: i64) -> Vec<(&'static str, Value, bool)> {
    vec![("bruttoarbeitslohn", json!(euro * 100), true)]
}

fn lohn_b(euro: i64) -> Vec<(&'static str, Value, bool)> {
    vec![("bruttoarbeitslohn_partner", json!(euro * 100), true)]
}

fn bezug_b(euro: i64) -> Vec<(&'static str, Value, bool)> {
    vec![
        (B, json!(euro * 100), true),
        ("versorgung_bemessungsgrundlage_partner", json!(euro * 100), true),
        ("versorgung_beginn_jahr_partner", json!(2020), true),
        ("versorgung_art_partner", json!("beamtenrechtlich"), true),
    ]
}

/// Die Kapital-Nullen des Ehegatten, die die Rentner-Scheibe bei Zusammenveranlagung zusaetzlich verlangt.
fn partner_kap() -> Vec<(&'static str, Value, bool)> {
    vec![
        ("kap_kapitalertraege_partner", json!(0), true),
        ("kap_gewinn_aktien_partner", json!(0), true),
        ("kap_gewinn_sonstige_partner", json!(0), true),
        ("kap_verlust_aktien_partner", json!(0), true),
        ("kap_verlust_sonstige_partner", json!(0), true),
    ]
}

fn mit(mut a: Vec<(&'static str, Value, bool)>, b: Vec<(&'static str, Value, bool)>) -> Vec<(&'static str, Value, bool)> {
    a.extend(b);
    a
}

/// AK3, Person A: die vier Fragen stehen NUR bei einem bestaetigten Bezug ueber 0 (positiver Beweis). Ohne Bezug, bei Bezug 0 und bei
/// einem nur vorlaeufigen Bezug steht keine. Der Ehegatte-Satz steht bei Einzelveranlagung nie, auch mit gespeichertem Bezug des Ehegatten.
#[tokio::test]
async fn die_fragen_von_person_a_stehen_nur_mit_einem_bestaetigten_bezug_ueber_0() {
    let einzel = [("veranlagung", json!("einzel"))];
    // Kontrolle: die Akte ohne Bezug fragt keine der acht.
    let d = akte("rentner.json", &einzel, &[]).await;
    assert_eq!(rueckfragen(&d).await, (vec![], vec![]), "ohne Bezug");

    let d = akte("rentner.json", &einzel, &mit(lohn_a(30_000), bezug_a(30_000, true))).await;
    assert_eq!(rueckfragen(&d).await, (FRAGEN_A.to_vec(), vec![]), "Bezug 30.000 EUR: alle vier von Person A, keine des Ehegatten");

    let d = akte("rentner.json", &einzel, &mit(lohn_a(30_000), bezug_a(0, true))).await;
    assert_eq!(rueckfragen(&d).await, (vec![], vec![]), "Bezug 0: die Fragen sind verborgen");

    let d = akte("rentner.json", &einzel, &mit(lohn_a(30_000), bezug_a(30_000, false))).await;
    assert_eq!(rueckfragen(&d).await, (vec![], vec![]), "vorlaeufiger Bezug: Schweigen und Vorlaeufiges schliessen die Frage aus");

    // Einzelveranlagung: ein gespeicherter Bezug des Ehegatten fragt nichts.
    let d = akte("rentner.json", &einzel, &mit(mit(lohn_a(30_000), lohn_b(20_000)), mit(bezug_a(30_000, true), bezug_b(20_000)))).await;
    assert_eq!(rueckfragen(&d).await, (FRAGEN_A.to_vec(), vec![]), "Einzelveranlagung: keine Frage des Ehegatten");
}

/// AK3, Ehegatte: die vier Fragen stehen NUR bei Zusammenveranlagung UND bestaetigtem Bezug des Ehegatten, und sie gehoeren dem
/// Ehegatten: der Bezug von Person A fragt die des Ehegatten nicht.
#[tokio::test]
async fn die_fragen_des_ehegatten_stehen_nur_bei_zusammenveranlagung_mit_bezug() {
    let zusammen = [("veranlagung", json!("zusammen"))];
    let d = akte("rentner.json", &zusammen, &partner_kap()).await;
    assert_eq!(rueckfragen(&d).await, (vec![], vec![]), "Zusammenveranlagung ohne Bezug");

    let d = akte("rentner.json", &zusammen, &mit(mit(partner_kap(), lohn_b(20_000)), bezug_b(20_000))).await;
    assert_eq!(rueckfragen(&d).await, (vec![], FRAGEN_B.to_vec()), "nur der Ehegatte hat einen Bezug");

    let d = akte("rentner.json", &zusammen, &mit(mit(partner_kap(), lohn_a(30_000)), bezug_a(30_000, true))).await;
    assert_eq!(rueckfragen(&d).await, (FRAGEN_A.to_vec(), vec![]), "nur Person A hat einen Bezug");

    let d = akte("rentner.json", &zusammen, &mit(mit(mit(partner_kap(), mit(lohn_a(30_000), lohn_b(20_000))), bezug_a(30_000, true)), bezug_b(20_000))).await;
    assert_eq!(rueckfragen(&d).await, (FRAGEN_A.to_vec(), FRAGEN_B.to_vec()), "beide haben einen Bezug");

    let d = akte("rentner.json", &zusammen, &mit(mit(partner_kap(), lohn_b(20_000)), bezug_b(0))).await;
    assert_eq!(rueckfragen(&d).await, (vec![], vec![]), "Bezug des Ehegatten 0");
}

/// AK3, der Wortlaut: die Frage des Ehegatten nennt den Ehe-/Lebenspartner, die von Person A spricht den Nutzer an. Ohne das beantwortet
/// der Nutzer die Frage fuer die falsche Person, und die Sperre trifft die andere (das Ja gilt nur unter dem Feld seiner Frage).
#[tokio::test]
async fn die_fragetexte_nennen_die_person_um_die_es_geht() {
    let zusammen = [("veranlagung", json!("zusammen"))];
    let d = akte("rentner.json", &zusammen, &mit(mit(mit(partner_kap(), mit(lohn_a(30_000), lohn_b(20_000))), bezug_a(30_000, true)), bezug_b(20_000))).await;
    let (status, antwort) = sende(&d, "GET", "/fall/vs/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {antwort}");
    let text = |feld: &str| {
        antwort["fragen"]
            .as_array()
            .unwrap()
            .iter()
            .find(|q| q["feld_id"] == json!(feld))
            .unwrap_or_else(|| panic!("{feld}: die Frage steht nicht in der Fragenliste"))["fragetext_laie"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    for feld in FRAGEN_A {
        let t = text(feld);
        assert!(t.contains(" du ") || t.contains("deine"),"{feld}: spricht den Nutzer nicht an: {t}");
        assert!(!t.contains("Partner"), "{feld}: nennt den Partner: {t}");
    }
    for feld in FRAGEN_B {
        let t = text(feld);
        assert!(t.contains("Ehe-/Lebenspartner"), "{feld}: nennt den Ehe-/Lebenspartner nicht: {t}");
    }
}

/// AK4, Person A, der ganze Weg auf der Akte `gesamt.json`: Bezug angeben (30.000 EUR, Beginn 2020: die Zeilen 11 bis 13 tragen ihn), die
/// Frage sehen. Abweichung Nr. 79: ohne Antwort sperrt `POST /einreichen` mit 409 `deklaration_unvollstaendig`, die Liste nennt alle vier
/// Fragen, und jeder Grund sagt, dass die Antwort fehlt. Sind drei Fragen mit Nein beantwortet, nennt die Liste genau die vierte. Mit Ja
/// auf diese Frage bleibt es 409 mit genau diesem Feld, und der Grund nennt Zeile, Sperre und Formular (nicht mehr die fehlende Antwort).
/// Auf Nein geaendert, ist die Sperre weg. Wie weit die Abgabe danach kommt, haengt von Schema, Hersteller-ID und `ERiC` der Umgebung ab
/// (der Test setzt nichts davon voraus).
#[tokio::test]
async fn ja_sperrt_die_abgabe_und_nein_gibt_sie_frei() {
    let faelle = [
        ("versorgung_zweiter_bezug", "zwei verschiedene Versorgungsbezüge"),
        ("versorgung_sterbegeld", "Zeile 15"),
        ("versorgung_mehrere_jahre", "Zeile 16"),
        ("versorgung_nicht_ganzjaehrig", "nicht in allen zwölf Monaten"),
    ];
    for (frage, merkmal) in faelle {
        let d = akte("gesamt.json", &[], &bezug_a(30_000, true)).await;
        assert!(fragen_ids(&d).await.iter().any(|f| f == frage), "{frage}: die Frage steht nicht in der Fragenliste");

        // Ohne Antwort (Nr. 79): 409, alle vier Fragen stehen in der Liste, jeder Grund nennt die fehlende Antwort und die Sperre.
        let (status, a) = sende(&d, "POST", "/fall/vs/einreichen", Some(&json!({}))).await;
        assert_eq!(status, 409, "{frage} unbeantwortet: erwartet 409, erhalten {status}: {a}");
        assert_eq!(a["grund"], "deklaration_unvollstaendig", "{a}");
        let eintraege = a["unvollstaendig"].as_array().unwrap();
        assert_eq!(eintraege.iter().filter_map(|e| e["feld_id"].as_str()).collect::<Vec<_>>(), FRAGEN_A, "{a}");
        for e in eintraege {
            let grund = e["grund"].as_str().unwrap();
            assert!(
                grund.contains("nicht mit Ja oder Nein beantwortet") && grund.contains("gesperrt"),
                "{}: der Grund nennt die fehlende Antwort und die Sperre nicht: {grund}",
                e["feld_id"]
            );
        }

        // Die drei anderen mit Nein: genau diese Frage bleibt in der Liste.
        for andere in FRAGEN_A.iter().filter(|f| **f != frage) {
            let (status, antwort) = sende(&d, "POST", "/fall/vs/event", Some(&ereignis(andere, &json!(false), true, None))).await;
            assert_eq!(status, 201, "POST /event {andere}: {antwort}");
        }
        let (status, a) = sende(&d, "POST", "/fall/vs/einreichen", Some(&json!({}))).await;
        assert_eq!(status, 409, "{frage} als einzige unbeantwortet: {status} {a}");
        assert_eq!(
            a["unvollstaendig"].as_array().unwrap().iter().filter_map(|e| e["feld_id"].as_str()).collect::<Vec<_>>(),
            [frage],
            "{a}"
        );

        // Ja: 409, genau diese Frage steht in der Liste, mit dem Text des Falls.
        let (status, antwort) = sende(&d, "POST", "/fall/vs/event", Some(&ereignis(frage, &json!(true), true, None))).await;
        assert_eq!(status, 201, "POST /event {frage}: {antwort}");
        let (status, a) = sende(&d, "POST", "/fall/vs/einreichen", Some(&json!({}))).await;
        assert_eq!(status, 409, "{frage} = Ja: erwartet 409, erhalten {status}: {a}");
        assert_eq!(a["grund"], "deklaration_unvollstaendig", "{a}");
        let eintraege = a["unvollstaendig"].as_array().unwrap();
        assert_eq!(eintraege.iter().filter_map(|e| e["feld_id"].as_str()).collect::<Vec<_>>(), [frage], "{a}");
        let grund = eintraege[0]["grund"].as_str().unwrap();
        assert!(
            grund.contains(merkmal) && grund.contains("gesperrt") && grund.contains("Formular"),
            "{frage}: der Grund nennt Merkmal, Sperre und Formular nicht: {grund}"
        );

        // Nein: die Sperre ist weg, und kein Eintrag der Versorgungsfelder bleibt.
        aendere(&d, frage, &json!(false)).await;
        let (status, a) = sende(&d, "POST", "/fall/vs/einreichen", Some(&json!({}))).await;
        assert_ne!(a["grund"], json!("deklaration_unvollstaendig"), "{frage} = Nein: die Sperre greift noch: {status} {a}");
        assert!(
            a["unvollstaendig"]
                .as_array()
                .is_none_or(|e| e.iter().all(|x| !x["feld_id"].as_str().unwrap_or("").starts_with("versorgung_"))),
            "{frage} = Nein: ein Eintrag der Versorgungsfelder bleibt: {a}"
        );
    }
}

/// Die Felder `versorgung_*` in `unvollstaendig` von `GET /deklaration`.
async fn offen(d: &Dienst) -> Vec<String> {
    let (status, dekl) = sende(d, "GET", "/fall/vs/deklaration", None).await;
    assert_eq!(status, 200, "{dekl}");
    dekl["unvollstaendig"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["feld_id"].as_str())
        .filter(|f| f.starts_with("versorgung_"))
        .map(str::to_owned)
        .collect()
}

/// Antwort `ja` auf jede Frage in `felder` (bestaetigt).
fn antworten(felder: &[&'static str], ja: bool) -> Vec<(&'static str, Value, bool)> {
    felder.iter().map(|f| (*f, json!(ja), true)).collect()
}

/// AK4, Ehegatte: das Ja des Ehegatten sperrt (`GET /deklaration`, `unvollstaendig` unter dem Feld seiner Frage), das Ja von Person A
/// sperrt den Ehegatten nicht, und das Ja des Ehegatten zaehlt nicht bei Einzelveranlagung. Abweichung Nr. 79: dasselbe gilt fuer die
/// fehlende Antwort; sie sperrt alle vier Fragen des Ehegatten, bei Zusammenveranlagung und bestaetigtem Bezug des Ehegatten.
#[tokio::test]
async fn das_ja_des_ehegatten_sperrt_nur_bei_zusammenveranlagung() {
    let zusammen = [("veranlagung", json!("zusammen"))];
    let basis = || mit(mit(partner_kap(), lohn_b(30_000)), bezug_b(20_000));
    let b_nein = || antworten(&FRAGEN_B, false);

    // Ohne Antwort (Nr. 79): alle vier Fragen des Ehegatten sperren, keine von Person A (sie hat keinen Bezug).
    let d = akte("rentner.json", &zusammen, &basis()).await;
    assert_eq!(offen(&d).await, FRAGEN_B, "ohne Antwort");

    // Kontrolle: viermal Nein sperrt nichts.
    let d = akte("rentner.json", &zusammen, &mit(basis(), b_nein())).await;
    assert!(offen(&d).await.is_empty(), "viermal Nein");

    // Das Ja des Ehegatten sperrt unter dem Feld seiner Frage; auf Nein geaendert, ist die Sperre weg.
    let ja = mit(
        basis(),
        mit(antworten(&["versorgung_zweiter_bezug_partner", "versorgung_mehrere_jahre_partner", "versorgung_nicht_ganzjaehrig_partner"], false), vec![("versorgung_sterbegeld_partner", json!(true), true)]),
    );
    let d = akte("rentner.json", &zusammen, &ja).await;
    assert_eq!(offen(&d).await, ["versorgung_sterbegeld_partner"]);
    aendere(&d, "versorgung_sterbegeld_partner", &json!(false)).await;
    assert!(offen(&d).await.is_empty(), "Nein");

    // Das Ja von Person A (ohne eigenen Bezug) sperrt den Ehegatten nicht, und ihre fehlenden Antworten auch nicht.
    let d = akte("rentner.json", &zusammen, &mit(mit(basis(), b_nein()), vec![("versorgung_sterbegeld", json!(true), true)])).await;
    assert!(offen(&d).await.is_empty(), "das Ja von Person A gilt nicht fuer den Ehegatten");

    // Einzelveranlagung: der Ehegatte zaehlt nicht, weder sein Ja noch seine fehlende Antwort sperrt.
    let einzel = [("veranlagung", json!("einzel"))];
    let d = akte("rentner.json", &einzel, &mit(mit(lohn_b(30_000), bezug_b(20_000)), vec![("versorgung_sterbegeld_partner", json!(true), true)])).await;
    assert!(offen(&d).await.is_empty(), "Einzelveranlagung, Ja");
    let d = akte("rentner.json", &einzel, &mit(lohn_b(30_000), bezug_b(20_000))).await;
    assert!(offen(&d).await.is_empty(), "Einzelveranlagung, ohne Antwort");
}

/// Abweichung Nr. 79, die Naht zwischen Frage und Sperre: eine unbeantwortete Rueckfrage sperrt GENAU DANN, wenn sie in der Fragenliste
/// steht. Sperrte sie ohne sichtbare Frage, koennte der Nutzer die Abgabe nie freigeben; stuende sie ohne Sperre da, waere sie keine
/// Pflicht. Gemessen an den Lagen der Sichtbarkeitstests (Bezug von Person A, Bezug 0, vorlaeufiger Bezug, Ehegatte bei Einzel- und
/// Zusammenveranlagung): die Menge der gesperrten Fragen ist die Menge der sichtbaren. Danach Nein auf alle sichtbaren: nichts sperrt.
#[tokio::test]
async fn eine_unbeantwortete_frage_sperrt_genau_dann_wenn_sie_in_der_fragenliste_steht() {
    let einzel = [("veranlagung", json!("einzel"))];
    let zusammen = [("veranlagung", json!("zusammen"))];
    let lagen: Vec<(&str, &str, Vec<(&str, Value)>, Vec<(&'static str, Value, bool)>)> = vec![
        ("kein Bezug", "rentner.json", einzel.to_vec(), vec![]),
        ("Bezug A 30.000", "rentner.json", einzel.to_vec(), mit(lohn_a(30_000), bezug_a(30_000, true))),
        ("Bezug A 0", "rentner.json", einzel.to_vec(), mit(lohn_a(30_000), bezug_a(0, true))),
        ("Bezug A vorlaeufig", "rentner.json", einzel.to_vec(), mit(lohn_a(30_000), bezug_a(30_000, false))),
        ("einzel, Bezug des Ehegatten gespeichert", "rentner.json", einzel.to_vec(), mit(mit(lohn_a(30_000), lohn_b(20_000)), mit(bezug_a(30_000, true), bezug_b(20_000)))),
        ("zusammen, nur Ehegatte", "rentner.json", zusammen.to_vec(), mit(mit(partner_kap(), lohn_b(20_000)), bezug_b(20_000))),
        ("zusammen, nur Person A", "rentner.json", zusammen.to_vec(), mit(mit(partner_kap(), lohn_a(30_000)), bezug_a(30_000, true))),
        (
            "zusammen, beide",
            "rentner.json",
            zusammen.to_vec(),
            mit(mit(mit(partner_kap(), mit(lohn_a(30_000), lohn_b(20_000))), bezug_a(30_000, true)), bezug_b(20_000)),
        ),
        ("zusammen, Ehegatte 0", "rentner.json", zusammen.to_vec(), mit(mit(partner_kap(), lohn_b(20_000)), bezug_b(0))),
        ("gesamt, Bezug A", "gesamt.json", vec![], bezug_a(30_000, true)),
    ];
    for (name, datei, ersetze, mehr) in lagen {
        let d = akte(datei, &ersetze, &mehr).await;
        let (a, b) = rueckfragen(&d).await;
        let mut sichtbar: Vec<String> = a.iter().chain(b.iter()).map(|f| (*f).to_owned()).collect();
        sichtbar.sort();
        let mut gesperrt: Vec<String> = offen(&d).await.into_iter().filter(|f| FRAGEN_A.contains(&f.as_str()) || FRAGEN_B.contains(&f.as_str())).collect();
        gesperrt.sort();
        assert_eq!(gesperrt, sichtbar, "{name}: gesperrt ist nicht gleich sichtbar");
        // Nein auf jede sichtbare Frage: nichts sperrt mehr, und die Fragenliste fuehrt sie nicht mehr.
        for f in &sichtbar {
            let (status, antwort) = sende(&d, "POST", "/fall/vs/event", Some(&ereignis(f, &json!(false), true, None))).await;
            assert_eq!(status, 201, "{name}: POST /event {f}: {antwort}");
        }
        let rest: Vec<String> = offen(&d).await.into_iter().filter(|f| FRAGEN_A.contains(&f.as_str()) || FRAGEN_B.contains(&f.as_str())).collect();
        assert!(rest.is_empty(), "{name}: nach Nein auf alle sichtbaren Fragen sperrt noch {rest:?}");
    }
    // KONTROLLE: mindestens eine Lage hat sichtbare Fragen (sonst belegt die Gleichheit der leeren Mengen nichts).
    let d = akte("rentner.json", &einzel, &mit(lohn_a(30_000), bezug_a(30_000, true))).await;
    assert_eq!(rueckfragen(&d).await.0, FRAGEN_A.to_vec(), "KONTROLLE: Bezug A zeigt vier Fragen");
}

type Fall = (&'static str, &'static str, Vec<(&'static str, Value)>, Vec<(&'static str, Value, bool)>);

/// Ein Bezug ueber dem Lohn sperrt den Bescheid (`versorgung_ueber_lohn`, Nr. 50). `GET /fragen` zeigt die Sperre wie `/stand` und
/// `/ergebnis` (200, `ring_gesperrt`, Klartext) und bleibt vollstaendig. Vorher: 500 `OverflowError: ... der Guard haette sperren
/// muessen` (Beobachtung in Nr. 66 Punkt 5), weil `gesamt_beitrag` den Ring auch dann rechnete, wenn der Kern die Sperre als Fehler
/// meldet. KONTROLLE im selben Lauf: Lohn gleich Bezug sperrt nicht; die Fragenliste mit Lohn 10.000 (Sperre) nennt dieselben Fragen
/// wie die mit Lohn 30.000 (keine Sperre), nur die Reihenfolge darf abweichen.
#[tokio::test]
async fn ein_bezug_ueber_dem_lohn_laesst_die_fragenliste_mit_der_sperre_antworten() {
    let einzel = [("veranlagung", json!("einzel"))];
    let zusammen = [("veranlagung", json!("zusammen"))];
    let d = akte("rentner.json", &einzel, &mit(lohn_a(30_000), bezug_a(30_000, true))).await;
    let (status, antwort) = sende(&d, "GET", "/fall/vs/fragen", None).await;
    assert_eq!(status, 200, "KONTROLLE Lohn gleich Bezug: {antwort}");
    assert_eq!(antwort["ring_gesperrt"], Value::Null, "KONTROLLE: {antwort}");
    let mut ohne_sperre = fragen_ids(&d).await;
    ohne_sperre.sort();

    let faelle: Vec<Fall> = vec![
        ("rentner, Bezug 30.000 ohne Lohn", "rentner.json", einzel.to_vec(), bezug_a(30_000, true)),
        ("rentner, Lohn 0", "rentner.json", einzel.to_vec(), mit(lohn_a(0), bezug_a(30_000, true))),
        ("rentner, Lohn 10.000", "rentner.json", einzel.to_vec(), mit(lohn_a(10_000), bezug_a(30_000, true))),
        ("gesamt, Lohn 0", "gesamt.json", vec![("veranlagung", json!("einzel")), ("bruttoarbeitslohn", json!(0))], bezug_a(30_000, true)),
        (
            "rentner zusammen, Ehegatte Lohn 0 und Bezug 20.000",
            "rentner.json",
            zusammen.to_vec(),
            mit(mit(partner_kap(), mit(lohn_a(30_000), lohn_b(0))), mit(bezug_a(30_000, true), bezug_b(20_000))),
        ),
    ];
    for (name, datei, ersetze, mehr) in faelle {
        let d = akte(datei, &ersetze, &mehr).await;
        let (status, antwort) = sende(&d, "GET", "/fall/vs/fragen", None).await;
        assert_eq!(status, 200, "{name}: {antwort}");
        assert_eq!(antwort["ring_gesperrt"], json!("versorgung_ueber_lohn"), "{name}: {antwort}");
        assert!(
            antwort["ring_gesperrt_klartext"].as_str().is_some_and(|t| t.contains("Versorgungsbezug")),
            "{name}: der Klartext fehlt: {antwort}"
        );
        assert!(antwort["fragen"].as_array().is_some_and(|f| f.len() > 8), "{name}: die Liste ist zu kurz: {antwort}");
    }
    let d = akte("rentner.json", &einzel, &mit(lohn_a(10_000), bezug_a(30_000, true))).await;
    let mut mit_sperre = fragen_ids(&d).await;
    mit_sperre.sort();
    assert_eq!(mit_sperre, ohne_sperre, "die Sperre darf keine Frage aus der Liste nehmen");
}

/// Dieselbe Ursache bei den Kindern: ein Zeitraum, den es im Veranlagungsjahr nicht gibt (29.02. 2025),
/// sperrt den Bescheid (`kind_zeitraum_unlesbar`); `GET /fragen` antwortete 500 `... der Guard haette sperren muessen`
/// (`BescheidFehler::KindFreibetragGesperrt`). KONTROLLE: derselbe Fall mit ganzjaehrigem Zeitraum sperrt nicht.
#[tokio::test]
async fn ein_unlesbarer_kind_zeitraum_laesst_die_fragenliste_mit_der_sperre_antworten() {
    let einzel = [("veranlagung", json!("einzel"))];
    let kind = |zeitraum: &'static str| -> Vec<(&'static str, Value, bool)> {
        vec![
            ("fam_anzahl_kinder", json!(1), true),
            ("kind_kindschaftsverhaeltnis_a", json!("1"), true),
            ("kind_kindschaftsverh_zeitraum_a", json!(zeitraum), true),
            ("kind_kindschaftsverhaeltnis_b", json!("1"), true),
            ("kind_kindschaftsverh_zeitraum_b", json!("01.01-31.12"), true),
        ]
    };
    let d = akte("gesamt.json", &einzel, &kind("01.01-31.12")).await;
    let (status, antwort) = sende(&d, "GET", "/fall/vs/fragen", None).await;
    assert_eq!(status, 200, "KONTROLLE ganzjaehrig: {antwort}");
    assert_eq!(antwort["ring_gesperrt"], Value::Null, "KONTROLLE: {antwort}");

    let d = akte("gesamt.json", &einzel, &kind("29.02-31.12")).await;
    let (status, antwort) = sende(&d, "GET", "/fall/vs/fragen", None).await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(antwort["ring_gesperrt"], json!("kind_zeitraum_unlesbar"), "{antwort}");
    assert!(antwort["fragen"].as_array().is_some_and(|f| f.len() > 8), "die Liste ist zu kurz: {antwort}");
}
