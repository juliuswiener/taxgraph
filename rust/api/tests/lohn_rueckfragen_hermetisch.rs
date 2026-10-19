//! Die Rueckfragen zum Lohn (Abweichung Nr. 85, Julius 2026-10-19, d-29): die HTTP-Naht. Ohne `PARITY=1`, ohne Python, ohne Netz, ohne `ERiC`.
//!
//! **Worum es geht.** Zwei Faelle verraten die Eingaben nicht: Arbeitslohn fuer mehrere Jahre und Entschaedigungen, z. B. Abfindungen
//! (Zeile 17 der Anlage N), und steuerfreier Arbeitslohn nach dem Auslandstaetigkeitserlass (Zeile 22, Anlage N-AUS). Die Erklaerung
//! traegt diese Zeilen nicht. Darum fragt sie je Person mit Lohn danach; ein bestaetigtes Ja sperrt die Abgabe mit dem Text des Falls,
//! eine fehlende Antwort mit eigenem Text, nur ein bestaetigtes Nein sperrt nichts.
//!
//! **Hier geprueft.** (1) Die Fragen stehen NUR bei einem bestaetigten Bruttoarbeitslohn ueber 0, die des Ehegatten NUR bei
//! Zusammenveranlagung und bestaetigtem Lohn des Ehegatten. (2) Der ganze Weg: ohne Antwort 409 `deklaration_unvollstaendig`, Ja 409 mit
//! dem Grund unter dem Feld der Frage, auf Nein geaendert ist die Sperre weg. (3) Die Naht zwischen Frage und Sperre: eine
//! unbeantwortete Frage sperrt GENAU DANN, wenn sie in der Fragenliste steht, in jeder Scheibe, die einen Bruttoarbeitslohn fuehrt. Die
//! Sperre selbst und ihre Grenzen pruefen die Unit-Tests in `rust/elster`.
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

/// Die zwei Fragen von Person A und die des Ehegatten, in der Reihenfolge der Faelle.
const FRAGEN_A: [&str; 2] = ["lohn_mehrere_jahre", "lohn_auslandstaetigkeit"];
const FRAGEN_B: [&str; 2] = ["lohn_mehrere_jahre_partner", "lohn_auslandstaetigkeit_partner"];

type Paare = Vec<(&'static str, Value, bool)>;

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

/// Legt die vollstaendige Akte `rust/fixtures/e2e/<datei>.json` als Fall `lo` an. `ersetze`: ein Feld der Akte bekommt den Wert, `null`
/// entfernt es aus der Akte. Dann schreibt jedes Paar von `mehr` ueber `POST /event` (muss 201 sein).
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
    roh["fall_id"] = json!("lo");
    roh["user_id"] = json!("alice");
    std::fs::write(d.zustand.konfig.faelle.join("lo.json"), serde_json::to_vec(&roh).unwrap()).unwrap();
    for (feld, wert, bestaetigt) in mehr {
        let (status, antwort) = sende(&d, "POST", "/fall/lo/event", Some(&ereignis(feld, wert, *bestaetigt, None))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    d
}

/// Aendert eine bestehende Antwort: der Store verlangt die `event_id` des aktiven Events als `ersetzt`.
async fn aendere(d: &Dienst, feld: &str, wert: &Value) {
    let (status, stand) = sende(d, "GET", "/fall/lo/stand", None).await;
    assert_eq!(status, 200, "GET /stand: {stand}");
    let id = stand["felder"][feld]["event_id"].clone();
    let (status, antwort) = sende(d, "POST", "/fall/lo/event", Some(&ereignis(feld, wert, true, Some(&id)))).await;
    assert_eq!(status, 201, "POST (aendern) {feld}: {antwort}");
}

/// Die Akten `gesamt.json` und `rentner.json` tragen ohne Zutun keine Antwort auf die zwei Fragen des Lohns (`rentner.json` hat keinen Lohn;
/// `gesamt.json` hat die Antworten Nein, die diese Tests entfernen, wo sie die fehlende Antwort messen).
fn ohne_antworten() -> Vec<(&'static str, Value)> {
    FRAGEN_A.iter().map(|f| (*f, Value::Null)).collect()
}

fn lohn_a(euro: i64) -> Paare {
    vec![("bruttoarbeitslohn", json!(euro * 100), true)]
}

fn lohn_b(euro: i64) -> Paare {
    vec![("bruttoarbeitslohn_partner", json!(euro * 100), true)]
}

/// Die Kapital-Nullen des Ehegatten, die eine Zusammenveranlagung zusaetzlich verlangt.
fn partner_kap() -> Paare {
    vec![
        ("kap_kapitalertraege_partner", json!(0), true),
        ("kap_gewinn_aktien_partner", json!(0), true),
        ("kap_gewinn_sonstige_partner", json!(0), true),
        ("kap_verlust_aktien_partner", json!(0), true),
        ("kap_verlust_sonstige_partner", json!(0), true),
    ]
}

fn mit(mut a: Paare, b: Paare) -> Paare {
    a.extend(b);
    a
}

/// Welche der Fragen `FRAGEN_A` und `FRAGEN_B` in der Fragenliste stehen: `(A, B)`.
async fn sichtbar(d: &Dienst) -> (Vec<&'static str>, Vec<&'static str>) {
    let (status, antwort) = sende(d, "GET", "/fall/lo/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {antwort}");
    let ids: Vec<&str> = antwort["fragen"].as_array().unwrap().iter().filter_map(|q| q["feld_id"].as_str()).collect();
    assert!(ids.len() > 8, "KONTROLLE: die Fragenliste ist nicht leer ({ids:?})");
    let waehle = |felder: &[&'static str]| felder.iter().copied().filter(|f| ids.contains(f)).collect();
    (waehle(&FRAGEN_A), waehle(&FRAGEN_B))
}

/// Die Felder `lohn_*` in `unvollstaendig` von `GET /deklaration`.
async fn gesperrt(d: &Dienst) -> Vec<String> {
    let (status, dekl) = sende(d, "GET", "/fall/lo/deklaration", None).await;
    assert_eq!(status, 200, "{dekl}");
    dekl["unvollstaendig"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["feld_id"].as_str())
        .filter(|f| f.starts_with("lohn_"))
        .map(str::to_owned)
        .collect()
}

/// Person A: die zwei Fragen stehen NUR bei einem bestaetigten Bruttoarbeitslohn ueber 0 (positiver Beweis) und solange sie
/// unbeantwortet sind. Ohne Lohn, bei Lohn 0 und bei einem nur vorlaeufigen Lohn steht keine; beantwortet verschwindet sie aus der Liste.
#[tokio::test]
async fn die_fragen_von_person_a_stehen_nur_mit_einem_bestaetigten_lohn_ueber_0() {
    let d = akte("gesamt.json", &ohne_antworten(), &[]).await;
    assert_eq!(sichtbar(&d).await, (FRAGEN_A.to_vec(), vec![]), "Lohn 48.500 EUR, ohne Antwort: beide Fragen von Person A");

    let d = akte("gesamt.json", &[], &[]).await;
    assert_eq!(sichtbar(&d).await, (vec![], vec![]), "beide beantwortet: keine Frage offen");

    let mut f = ohne_antworten();
    f.push(("bruttoarbeitslohn", json!(0)));
    let d = akte("gesamt.json", &f, &[]).await;
    assert_eq!(sichtbar(&d).await, (vec![], vec![]), "Lohn 0: die Fragen sind verborgen");

    let mut f = ohne_antworten();
    f.push(("bruttoarbeitslohn", Value::Null));
    let d = akte("gesamt.json", &f, &[("bruttoarbeitslohn", json!(4_850_000), false)]).await;
    assert_eq!(sichtbar(&d).await, (vec![], vec![]), "vorlaeufiger Lohn: Schweigen und Vorlaeufiges schliessen die Frage aus");

    let mut f = ohne_antworten();
    f.push(("bruttoarbeitslohn", Value::Null));
    let d = akte("gesamt.json", &f, &[]).await;
    assert_eq!(sichtbar(&d).await, (vec![], vec![]), "kein Lohn angegeben: die Fragen sind verborgen");
}

/// Ehegatte: die zwei Fragen stehen NUR bei Zusammenveranlagung UND bestaetigtem Lohn des Ehegatten, und sie gehoeren dem Ehegatten: der
/// Lohn von Person A fragt die des Ehegatten nicht.
#[tokio::test]
async fn die_fragen_des_ehegatten_stehen_nur_bei_zusammenveranlagung_mit_lohn() {
    let zusammen = |mut f: Vec<(&'static str, Value)>| {
        f.push(("veranlagung", json!("zusammen")));
        f
    };
    let d = akte("gesamt.json", &zusammen(ohne_antworten()), &partner_kap()).await;
    assert_eq!(sichtbar(&d).await, (FRAGEN_A.to_vec(), vec![]), "Zusammenveranlagung ohne Lohn des Ehegatten");

    let d = akte("gesamt.json", &zusammen(ohne_antworten()), &mit(partner_kap(), lohn_b(20_000))).await;
    assert_eq!(sichtbar(&d).await, (FRAGEN_A.to_vec(), FRAGEN_B.to_vec()), "beide haben Lohn");

    let d = akte("gesamt.json", &zusammen(ohne_antworten()), &mit(partner_kap(), lohn_b(0))).await;
    assert_eq!(sichtbar(&d).await, (FRAGEN_A.to_vec(), vec![]), "Lohn des Ehegatten 0");

    // Einzelveranlagung: ein gespeicherter Lohn des Ehegatten fragt nichts.
    let d = akte("gesamt.json", &ohne_antworten(), &mit(partner_kap(), lohn_b(20_000))).await;
    assert_eq!(sichtbar(&d).await, (FRAGEN_A.to_vec(), vec![]), "Einzelveranlagung: keine Frage des Ehegatten");

    // Der Lohn von Person A allein fragt den Ehegatten nicht (Lohn A auf 0, Lohn B > 0: nur der Ehegatte).
    let mut f = zusammen(ohne_antworten());
    f.push(("bruttoarbeitslohn", json!(0)));
    let d = akte("gesamt.json", &f, &mit(partner_kap(), lohn_b(20_000))).await;
    assert_eq!(sichtbar(&d).await, (vec![], FRAGEN_B.to_vec()), "nur der Ehegatte hat Lohn");
}

/// Der Wortlaut: die Frage des Ehegatten nennt den Ehe-/Lebenspartner, die von Person A spricht den Nutzer an. Ohne das beantwortet der
/// Nutzer die Frage fuer die falsche Person, und die Sperre trifft die andere (das Ja gilt nur unter dem Feld seiner Frage).
#[tokio::test]
async fn die_fragetexte_nennen_die_person_um_die_es_geht() {
    let mut f = ohne_antworten();
    f.push(("veranlagung", json!("zusammen")));
    let d = akte("gesamt.json", &f, &mit(partner_kap(), lohn_b(20_000))).await;
    let (status, antwort) = sende(&d, "GET", "/fall/lo/fragen", None).await;
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
        assert!(t.contains("dein ") || t.contains("Hast du"), "{feld}: spricht den Nutzer nicht an: {t}");
        assert!(!t.contains("Partner"), "{feld}: nennt den Partner: {t}");
    }
    for feld in FRAGEN_B {
        let t = text(feld);
        assert!(t.contains("Ehe-/Lebenspartner"), "{feld}: nennt den Ehe-/Lebenspartner nicht: {t}");
    }
}

/// Person A, der ganze Weg auf der Akte `gesamt.json`: ohne Antwort sperrt `POST /einreichen` mit 409 `deklaration_unvollstaendig`, die
/// Liste nennt beide Fragen, und jeder Grund sagt, dass die Antwort fehlt. Ist eine Frage mit Nein beantwortet, nennt die Liste genau die
/// andere. Mit Ja auf diese Frage bleibt es 409 mit genau diesem Feld, und der Grund nennt Merkmal, Sperre und Formular (nicht mehr die
/// fehlende Antwort). Auf Nein geaendert, ist die Sperre weg. Wie weit die Abgabe danach kommt, haengt von Schema, Hersteller-ID und
/// `ERiC` der Umgebung ab (der Test setzt nichts davon voraus).
#[tokio::test]
async fn ja_sperrt_die_abgabe_und_nein_gibt_sie_frei() {
    let faelle = [("lohn_mehrere_jahre", "Zeile 17"), ("lohn_auslandstaetigkeit", "Auslandstätigkeitserlass")];
    for (frage, merkmal) in faelle {
        let d = akte("gesamt.json", &ohne_antworten(), &[]).await;
        assert!(sichtbar(&d).await.0.contains(&frage), "{frage}: die Frage steht nicht in der Fragenliste");

        // Ohne Antwort: 409, beide Fragen stehen in der Liste, jeder Grund nennt die fehlende Antwort und die Sperre.
        let (status, a) = sende(&d, "POST", "/fall/lo/einreichen", Some(&json!({}))).await;
        assert_eq!(status, 409, "{frage} unbeantwortet: erwartet 409, erhalten {status}: {a}");
        assert_eq!(a["grund"], "deklaration_unvollstaendig", "{a}");
        let eintraege = a["unvollstaendig"].as_array().unwrap();
        assert_eq!(eintraege.iter().filter_map(|e| e["feld_id"].as_str()).collect::<Vec<_>>(), FRAGEN_A, "{a}");
        for e in eintraege {
            let grund = e["grund"].as_str().unwrap();
            assert!(
                grund.contains("nicht mit Ja oder Nein beantwortet") && grund.contains("Die Abgabe ist gesperrt, bis du die Frage beantwortest"),
                "{}: der Grund nennt die fehlende Antwort und die Sperre nicht: {grund}",
                e["feld_id"]
            );
        }

        // Die andere mit Nein: genau diese Frage bleibt in der Liste.
        for andere in FRAGEN_A.iter().filter(|f| **f != frage) {
            let (status, antwort) = sende(&d, "POST", "/fall/lo/event", Some(&ereignis(andere, &json!(false), true, None))).await;
            assert_eq!(status, 201, "POST /event {andere}: {antwort}");
        }
        let (status, a) = sende(&d, "POST", "/fall/lo/einreichen", Some(&json!({}))).await;
        assert_eq!(status, 409, "{frage} als einzige unbeantwortet: {status} {a}");
        assert_eq!(a["unvollstaendig"].as_array().unwrap().iter().filter_map(|e| e["feld_id"].as_str()).collect::<Vec<_>>(), [frage], "{a}");

        // Ja: 409, genau diese Frage steht in der Liste, mit dem Text des Falls.
        let (status, antwort) = sende(&d, "POST", "/fall/lo/event", Some(&ereignis(frage, &json!(true), true, None))).await;
        assert_eq!(status, 201, "POST /event {frage}: {antwort}");
        let (status, a) = sende(&d, "POST", "/fall/lo/einreichen", Some(&json!({}))).await;
        assert_eq!(status, 409, "{frage} = Ja: erwartet 409, erhalten {status}: {a}");
        assert_eq!(a["grund"], "deklaration_unvollstaendig", "{a}");
        let eintraege = a["unvollstaendig"].as_array().unwrap();
        assert_eq!(eintraege.iter().filter_map(|e| e["feld_id"].as_str()).collect::<Vec<_>>(), [frage], "{a}");
        let grund = eintraege[0]["grund"].as_str().unwrap();
        assert!(
            grund.contains(merkmal) && grund.contains("gesperrt") && grund.contains("Formular"),
            "{frage}: der Grund nennt Merkmal, Sperre und Formular nicht: {grund}"
        );
        assert!(!grund.contains("nicht mit Ja oder Nein beantwortet"), "{frage}: das Ja liest den Text der fehlenden Antwort: {grund}");

        // Nein: die Sperre ist weg, und kein Eintrag der Lohnfelder bleibt.
        aendere(&d, frage, &json!(false)).await;
        let (status, a) = sende(&d, "POST", "/fall/lo/einreichen", Some(&json!({}))).await;
        assert_ne!(a["grund"], json!("deklaration_unvollstaendig"), "{frage} = Nein: die Sperre greift noch: {status} {a}");
        assert!(
            a["unvollstaendig"].as_array().is_none_or(|e| e.iter().all(|x| !x["feld_id"].as_str().unwrap_or("").starts_with("lohn_"))),
            "{frage} = Nein: ein Eintrag der Lohnfelder bleibt: {a}"
        );
    }
}

/// Antwort auf jede Frage in `felder` (bestaetigt).
fn antworten(felder: &[&'static str], ja: bool) -> Paare {
    felder.iter().map(|f| (*f, json!(ja), true)).collect()
}

/// Ehegatte: das Ja des Ehegatten sperrt (`GET /deklaration`, `unvollstaendig` unter dem Feld seiner Frage), das Ja von Person A sperrt
/// den Ehegatten nicht, und das Ja des Ehegatten zaehlt nicht bei Einzelveranlagung. Dasselbe gilt fuer die fehlende Antwort.
#[tokio::test]
async fn das_ja_des_ehegatten_sperrt_nur_bei_zusammenveranlagung() {
    let mut zusammen = ohne_antworten();
    zusammen.push(("veranlagung", json!("zusammen")));
    let basis = || mit(mit(partner_kap(), lohn_b(30_000)), antworten(&FRAGEN_A, false));

    // Ohne Antwort des Ehegatten: beide Fragen des Ehegatten sperren, keine von Person A (sie hat beantwortet).
    let d = akte("gesamt.json", &zusammen, &basis()).await;
    assert_eq!(gesperrt(&d).await, FRAGEN_B, "ohne Antwort");

    // Kontrolle: zweimal Nein sperrt nichts.
    let d = akte("gesamt.json", &zusammen, &mit(basis(), antworten(&FRAGEN_B, false))).await;
    assert!(gesperrt(&d).await.is_empty(), "zweimal Nein");

    // Das Ja des Ehegatten sperrt unter dem Feld seiner Frage; auf Nein geaendert, ist die Sperre weg.
    let ja = mit(basis(), vec![("lohn_mehrere_jahre_partner", json!(true), true), ("lohn_auslandstaetigkeit_partner", json!(false), true)]);
    let d = akte("gesamt.json", &zusammen, &ja).await;
    assert_eq!(gesperrt(&d).await, ["lohn_mehrere_jahre_partner"]);
    aendere(&d, "lohn_mehrere_jahre_partner", &json!(false)).await;
    assert!(gesperrt(&d).await.is_empty(), "Nein");

    // Das Ja von Person A sperrt den Ehegatten nicht (dessen Antworten sind Nein), und es steht unter dem Feld von Person A. Die Akte
    // bringt die Antworten Nein von Person A mit.
    let d = akte("gesamt.json", &[("veranlagung", json!("zusammen"))], &mit(mit(partner_kap(), lohn_b(30_000)), antworten(&FRAGEN_B, false))).await;
    assert!(gesperrt(&d).await.is_empty(), "Person A beantwortet, der Ehegatte auch");
    aendere(&d, "lohn_mehrere_jahre", &json!(true)).await;
    assert_eq!(gesperrt(&d).await, ["lohn_mehrere_jahre"], "das Ja von Person A sperrt unter ihrem Feld, nicht unter dem des Ehegatten");

    // Einzelveranlagung: der Ehegatte zaehlt nicht, weder sein Ja noch seine fehlende Antwort sperrt.
    let d = akte("gesamt.json", &[], &mit(mit(partner_kap(), lohn_b(30_000)), vec![("lohn_mehrere_jahre_partner", json!(true), true)])).await;
    assert!(gesperrt(&d).await.is_empty(), "Einzelveranlagung, Ja");
    let d = akte("gesamt.json", &[], &mit(partner_kap(), lohn_b(30_000))).await;
    assert!(gesperrt(&d).await.is_empty(), "Einzelveranlagung, ohne Antwort");
}

/// Die Naht zwischen Frage und Sperre: eine unbeantwortete Rueckfrage sperrt GENAU DANN, wenn sie in der Fragenliste steht. Sperrte sie
/// ohne sichtbare Frage, koennte der Nutzer die Abgabe nie freigeben; stuende sie ohne Sperre da, waere sie keine Pflicht. Gemessen an den
/// Lagen der Sichtbarkeitstests in den zwei Scheiben, die einen Bruttoarbeitslohn mit Deklaration fuehren (`gesamt` und `rentner_gesamt`;
/// `an_gesamt` hat keine eigene Akte, ihre Feldliste prueft `scheiben_tabellen_konsistenz`): die Menge der gesperrten Fragen ist die
/// Menge der sichtbaren. Danach Nein auf alle sichtbaren: nichts sperrt.
#[tokio::test]
async fn eine_unbeantwortete_frage_sperrt_genau_dann_wenn_sie_in_der_fragenliste_steht() {
    type Lage = (&'static str, &'static str, Vec<(&'static str, Value)>, Paare);
    let zusammen = |mut f: Vec<(&'static str, Value)>| {
        f.push(("veranlagung", json!("zusammen")));
        f
    };
    let lagen: Vec<Lage> = vec![
        ("gesamt, Lohn A", "gesamt.json", ohne_antworten(), vec![]),
        ("gesamt, Lohn A 0", "gesamt.json", mit_lohn_null(ohne_antworten()), vec![]),
        ("gesamt, nur Ehegatte", "gesamt.json", zusammen(mit_lohn_null(ohne_antworten())), mit(partner_kap(), lohn_b(20_000))),
        ("gesamt, beide", "gesamt.json", zusammen(ohne_antworten()), mit(partner_kap(), lohn_b(20_000))),
        ("gesamt, einzel, Lohn des Ehegatten gespeichert", "gesamt.json", ohne_antworten(), mit(partner_kap(), lohn_b(20_000))),
        ("gesamt, Ehegatte 0", "gesamt.json", zusammen(ohne_antworten()), mit(partner_kap(), lohn_b(0))),
        ("rentner, ohne Lohn", "rentner.json", vec![], vec![]),
        ("rentner, Lohn A", "rentner.json", vec![], lohn_a(30_000)),
        ("rentner, Lohn A, kein_lohn_pension nein", "rentner.json", vec![], mit(lohn_a(30_000), vec![("kein_lohn_pension", json!(false), true)])),
        ("rentner, zusammen, beide", "rentner.json", zusammen(vec![]), mit(mit(partner_kap(), lohn_a(30_000)), lohn_b(20_000))),
        ("rentner, zusammen, nur Ehegatte", "rentner.json", zusammen(vec![]), mit(partner_kap(), lohn_b(20_000))),
    ];
    let mut mit_fragen = 0;
    for (name, datei, ersetze, mehr) in lagen {
        let d = akte(datei, &ersetze, &mehr).await;
        let (a, b) = sichtbar(&d).await;
        let mut offen: Vec<String> = a.iter().chain(b.iter()).map(|f| (*f).to_owned()).collect();
        offen.sort();
        let mut sperren = gesperrt(&d).await;
        sperren.sort();
        assert_eq!(sperren, offen, "{name}: gesperrt ist nicht gleich sichtbar");
        mit_fragen += usize::from(!offen.is_empty());
        // Nein auf jede sichtbare Frage: nichts sperrt mehr.
        for f in &offen {
            let (status, antwort) = sende(&d, "POST", "/fall/lo/event", Some(&ereignis(f, &json!(false), true, None))).await;
            assert_eq!(status, 201, "{name}: POST /event {f}: {antwort}");
        }
        let rest = gesperrt(&d).await;
        assert!(rest.is_empty(), "{name}: nach Nein auf alle sichtbaren Fragen sperrt noch {rest:?}");
    }
    // KONTROLLE: die Mehrzahl der Lagen hat sichtbare Fragen (sonst belegt die Gleichheit der leeren Mengen nichts).
    assert!(mit_fragen >= 7, "KONTROLLE: nur {mit_fragen} Lagen mit sichtbaren Fragen");
}

/// Die Akte mit Lohn von Person A auf 0 (ersetzt den Wert in der Akte).
fn mit_lohn_null(mut f: Vec<(&'static str, Value)>) -> Vec<(&'static str, Value)> {
    f.push(("bruttoarbeitslohn", json!(0)));
    f
}
