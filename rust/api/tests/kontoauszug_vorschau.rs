//! Bankimport S3 (Vault `backlog/taxgraph/bankimport-s3-oberflaeche-vorschlaege-pruefen-mit-haken-und-kreuz-abwahl`):
//! `POST /fall/{id}/bank/vorschau` rechnet einen Auszug auf einer KOPIE der Akte und schreibt nichts; `POST /fall/{id}/kontoauszug`
//! (Uebernehmen) nimmt dazu das Feld `abgewaehlt` und nennt das Eingangskreuz `keine_spenden`, das die Seite abwaehlen darf.
//!
//! Die Kennung einer Buchung ist ihre Stelle in der gelesenen Liste (Datei und Format allein, unabhaengig von Haken und Regeln).
//! Der Server schreibt das Kreuz NIE: das Kreuz ist die Antwort der Nutzerin, die Seite schreibt es als `ui:laie` nach dem
//! Uebernehmen (Browser-Test `bank_vorschlaege_pruefen`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Handrechnung (Cent). Die Buchungstexte sind erfunden. Die Zahl der Fragen eines Falls misst der
//! Test als Unterschied vor und nach dem Ereignis (die Messung des Tickets: 370 -> 369), nicht als feste Zahl.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::path::PathBuf;

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
    fremd: String,
    _tmp: tempfile::TempDir,
}

fn dienst() -> Dienst {
    let tmp = tempfile::tempdir().unwrap();
    let konfig = Konfig {
        wurzel: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        faelle: tmp.path().join("faelle"),
        audit_dir: tmp.path().join("faelle"),
    };
    let auth = Auth::neu("testgeheimnis".into(), tmp.path().join("users.json"), Some(konfig.audit_pfad()));
    let zustand = Zustand::neu(konfig, auth);
    let token = zustand.auth.stelle_aus("alice").unwrap();
    let fremd = zustand.auth.stelle_aus("bob").unwrap();
    Dienst { zustand, token, fremd, _tmp: tmp }
}

/// Ein Aufruf mit diesem Token (`None`: ohne Anmeldung) und diesen zusaetzlichen Koepfen.
async fn anfrage(d: &Dienst, methode: &str, pfad: &str, token: Option<&str>, extra: &[(&str, String)], body: &Value) -> (u16, Value) {
    let text = if methode == "GET" { String::new() } else { body.to_string() };
    let mut b = Request::builder().method(methode).uri(pfad);
    if let Some(t) = token {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    if methode != "GET" {
        b = b.header("content-type", "application/json");
        if !extra.iter().any(|(k, _)| *k == "content-length") {
            b = b.header("content-length", text.len().to_string());
        }
    }
    for (k, v) in extra {
        b = b.header(*k, v);
    }
    let r = app(d.zustand.clone()).oneshot(b.body(Body::from(text)).unwrap()).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (teile.status.as_u16(), serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn sende(d: &Dienst, pfad: &str, body: &Value) -> (u16, Value) {
    anfrage(d, "POST", pfad, Some(&d.token), &[], body).await
}

async fn fall_anlegen(d: &Dienst, id: &str) {
    let rumpf = json!({"fall_id": id, "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "/fall", &rumpf).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
}

fn csv(zeilen: &[&str]) -> String {
    format!("datum;betrag;verwendungszweck\n{}\n", zeilen.join("\n"))
}

fn rumpf(zeilen: &[&str], extra: &Value) -> Value {
    let mut b = json!({"format": "csv", "inhalt": csv(zeilen)});
    for (k, v) in extra.as_object().unwrap() {
        b[k] = v.clone();
    }
    b
}

/// Stellen 0 bis 6: zwei Spenden, ein Handwerker (Nachfrage), eine Einnahme, eine Ausgabe ohne Stichwort, die Krankenversicherung
/// (nur mit dem Haken `art9`), eine Rentenversicherung (Vorsorge).
const AUSZUG: [&str; 7] = [
    "01.03.2025;-11,00;Spende Tierheim",
    "02.03.2025;-23,00;Spende Unicef",
    "03.03.2025;-73,00;Maler Huber",
    "04.03.2025;900,00;Gehalt Muster GmbH",
    "05.03.2025;-5,00;Miete Muster",
    "06.03.2025;-37,00;Krankenversicherung Muster Kasse",
    "07.03.2025;-24,00;Rentenversicherung Ruerup",
];

fn akte_pfad(d: &Dienst, id: &str) -> PathBuf {
    d.zustand.konfig.faelle.join(format!("{id}.json"))
}

fn events(d: &Dienst, id: &str) -> Vec<Value> {
    let akte: Value = serde_json::from_slice(&std::fs::read(akte_pfad(d, id)).unwrap()).unwrap();
    akte["events"].as_array().unwrap().clone()
}

fn felder_der_events(d: &Dienst, id: &str) -> Vec<String> {
    events(d, id).iter().map(|e| e["feld_id"].as_str().unwrap().to_owned()).collect()
}

fn dateien(d: &Dienst) -> Vec<String> {
    let mut namen: Vec<String> = std::fs::read_dir(&d.zustand.konfig.faelle)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    namen.sort();
    namen
}

fn audit_zeilen(d: &Dienst) -> Vec<String> {
    std::fs::read_to_string(d.zustand.konfig.audit_pfad())
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn posten_eines_felds<'a>(a: &'a Value, feld: &str) -> &'a Vec<Value> {
    a["felder"].as_array().unwrap().iter().find(|f| f["feld_id"] == feld).unwrap_or_else(|| panic!("{feld} fehlt: {a}"))["posten"]
        .as_array()
        .unwrap()
}

fn feld_der_antwort<'a>(a: &'a Value, feld: &str) -> &'a Value {
    a["felder"].as_array().unwrap().iter().find(|f| f["feld_id"] == feld).unwrap_or_else(|| panic!("{feld} fehlt: {a}"))
}

/// AK1: die Vorschau schreibt nichts. Die Akte hat danach dieselben Bytes, das Verzeichnis dieselben Dateien, und im Protokoll steht
/// genau eine neue Zeile, die des Dispatchers; keine Zeile `bank_import`, keine Zeile `llm_call`. Kontrolle: derselbe Auszug ueber
/// `kontoauszug` schreibt (die Akte aendert sich), und die Antwort der Vorschau nennt die Felder, die er geschrieben haette.
#[tokio::test]
async fn bank_vorschau_schreibt_nichts_akte_verzeichnis_und_protokoll_bleiben() {
    let d = dienst();
    fall_anlegen(&d, "v1").await;
    // Ein erster Import schreibt (der Fall ist nicht leer); die Vorschau trifft dann ein Feld mit eigenem Vorschlag.
    let (s, a) = sende(&d, "/fall/v1/kontoauszug", &rumpf(&["01.03.2025;-4,00;Spende Tierheim"], &json!({}))).await;
    assert_eq!(s, 200, "{a}");
    let akte_vorher = std::fs::read(akte_pfad(&d, "v1")).unwrap();
    let dateien_vorher = dateien(&d);
    let audit_vorher = audit_zeilen(&d);

    let (s, a) = sende(&d, "/fall/v1/bank/vorschau", &rumpf(&AUSZUG, &json!({}))).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["vorschau"], json!(true), "{a}");
    assert!(a["felder"].as_array().is_some_and(|f| !f.is_empty()), "die Vorschau nennt keine Felder: {a}");

    assert_eq!(std::fs::read(akte_pfad(&d, "v1")).unwrap(), akte_vorher, "die Vorschau hat die Akte veraendert");
    assert_eq!(dateien(&d), dateien_vorher, "die Vorschau hat eine Datei angelegt oder entfernt");
    let audit_nachher = audit_zeilen(&d);
    assert_eq!(audit_nachher.len(), audit_vorher.len() + 1, "genau die Zeile des Dispatchers: {audit_nachher:?}");
    assert_eq!(audit_nachher[..audit_vorher.len()], audit_vorher[..], "alte Protokollzeilen veraendert");
    let neu = audit_nachher.last().unwrap();
    assert!(neu.contains("fall_vorschau") && neu.contains("status=200"), "{neu}");
    assert!(!neu.contains("bank_import") && !neu.contains("llm_call"), "{neu}");

    // Kontrolle: der Auszug haette geschrieben (Vorsorge ist frei), und derselbe Aufruf ueber `kontoauszug` tut es.
    assert!(!felder_der_events(&d, "v1").contains(&"vor_rv_ausserhalb_lstb".to_owned()));
    let (s, a) = sende(&d, "/fall/v1/kontoauszug", &rumpf(&AUSZUG, &json!({}))).await;
    assert_eq!(s, 200, "{a}");
    assert!(felder_der_events(&d, "v1").contains(&"vor_rv_ausserhalb_lstb".to_owned()), "Kontrolle: der Import schreibt");
}

/// Die Antwort: Felder mit Summe, Zahl und Folge, die Posten mit Kennung, Datum, Betrag (Vorzeichen der Buchung), maskiertem Zweck,
/// Regel und Quelle; dazu `nachfragen` (§ 35a), die Haken und die Zahl der Buchungen ohne Stichwort. Ohne die Haken sind Einnahme und
/// Krankenversicherung keine Zeile.
#[tokio::test]
async fn bank_vorschau_antwort_nennt_felder_posten_nachfragen_und_haken() {
    let d = dienst();
    fall_anlegen(&d, "v2").await;
    let (s, a) = sende(&d, "/fall/v2/bank/vorschau", &rumpf(&AUSZUG, &json!({}))).await;
    assert_eq!(s, 200, "{a}");
    let spenden = feld_der_antwort(&a, "spenden_betrag");
    assert_eq!(
        (spenden["summe_cent"].as_i64(), spenden["anzahl"].as_i64(), spenden["folge"].as_str()),
        (Some(34_00), Some(2), Some("geschrieben")),
        "{spenden}"
    );
    let p = posten_eines_felds(&a, "spenden_betrag");
    assert_eq!(p.len(), 2);
    assert_eq!(
        (p[0]["index"].as_i64(), p[0]["datum"].as_str(), p[0]["betrag_cent"].as_i64(), p[0]["regel_id"].as_str(), p[0]["quelle"].as_str(), p[0]["abgewaehlt"].as_bool()),
        (Some(0), Some("2025-03-01"), Some(-11_00), Some("spende"), Some("stichwort"), Some(false)),
        "{}",
        p[0]
    );
    assert!(p[0]["zweck_maskiert"].as_str().unwrap().contains("Tierheim"), "{}", p[0]);
    assert_eq!((p[1]["index"].as_i64(), p[1]["betrag_cent"].as_i64()), (Some(1), Some(-23_00)));
    let vorsorge = posten_eines_felds(&a, "vor_rv_ausserhalb_lstb");
    assert_eq!((vorsorge[0]["index"].as_i64(), vorsorge[0]["betrag_cent"].as_i64()), (Some(6), Some(-24_00)));
    // Ohne den Haken `art9` gibt es keine Zeile fuer die Krankenversicherung (Stelle 5).
    assert!(a["felder"].as_array().unwrap().iter().all(|f| f["feld_id"] != "basis_kv"), "{a}");
    // § 35a: eine Nachfrage, kein Feld.
    let n = a["nachfragen"].as_array().unwrap();
    assert_eq!((n.len(), n[0]["feld_id"].as_str(), n[0]["betrag_cent"].as_i64()), (1, Some("hh_handwerker_betrag"), Some(-73_00)), "{a}");
    assert!(a["felder"].as_array().unwrap().iter().all(|f| f["feld_id"] != "hh_handwerker_betrag"), "{a}");
    assert_eq!(a["haken"], json!({"ersetzen": false, "art9": false, "ki_buchungstext": false}), "{a}");
    // Eine Ausgabe ohne Stichwort (Stelle 4): die Seite nennt ihre Zahl neben dem KI-Haken. Die Krankenversicherung ohne Haken
    // zaehlt als gesperrt, nicht als ohne Stichwort.
    assert_eq!(a["ohne_stichwort"], json!(1), "{a}");
    assert_eq!(a["transaktionen"], json!(7), "{a}");
    assert!(a["hinweis"].as_str().is_some_and(|h| h.contains("§ 35a")), "der Satz zu den Nachfragen fehlt: {a}");
}

/// Die Kennung haengt nicht von den Haken ab: mit `art9` kommt die Krankenversicherung als Zeile dazu (Stelle 5, ohne Zweck), die
/// Stellen der uebrigen Zeilen bleiben, wie sie waren. Ein Wechsel des Hakens `ki_buchungstext` aendert die Vorschau nicht (sie ruft
/// die KI nie), nur den genannten Haken.
#[tokio::test]
async fn bank_vorschau_index_ist_bei_haken_an_und_aus_derselbe() {
    let d = dienst();
    fall_anlegen(&d, "v3").await;
    let (_, aus) = sende(&d, "/fall/v3/bank/vorschau", &rumpf(&AUSZUG, &json!({}))).await;
    let (s, an) = sende(&d, "/fall/v3/bank/vorschau", &rumpf(&AUSZUG, &json!({"art9": true, "ki_buchungstext": true}))).await;
    assert_eq!(s, 200, "{an}");
    assert_eq!(an["haken"], json!({"ersetzen": false, "art9": true, "ki_buchungstext": true}));
    for feld in ["spenden_betrag", "vor_rv_ausserhalb_lstb"] {
        let stellen = |a: &Value| posten_eines_felds(a, feld).iter().map(|p| p["index"].as_i64().unwrap()).collect::<Vec<_>>();
        assert_eq!(stellen(&aus), stellen(&an), "{feld}");
    }
    let kv = posten_eines_felds(&an, "basis_kv");
    assert_eq!((kv[0]["index"].as_i64(), kv[0]["betrag_cent"].as_i64(), kv[0]["zweck_maskiert"].as_str()), (Some(5), Some(-37_00), Some("")), "{}", kv[0]);
    assert_eq!(an["ohne_stichwort"], aus["ohne_stichwort"], "der KI-Haken aendert die Vorschau nicht");
    assert!(
        !an["hinweis"].as_str().unwrap_or_default().contains("nicht eingerichtet"),
        "die Vorschau ruft die KI nie, also fehlt ihr nichts: {an}"
    );
}

/// Der Server rechnet die Summe neu: eine abgewaehlte Buchung geht nicht ein, bleibt aber mit `abgewaehlt: true` in der Liste;
/// sind alle abgewaehlt, steht das Feld mit Summe 0 und der Folge `abgewaehlt` da, damit die Seite die Posten wieder anbietet.
#[tokio::test]
async fn bank_vorschau_abgewaehlt_rechnet_der_server_neu() {
    let d = dienst();
    fall_anlegen(&d, "v4").await;
    let (s, a) = sende(&d, "/fall/v4/bank/vorschau", &rumpf(&AUSZUG, &json!({"abgewaehlt": [1]}))).await;
    assert_eq!(s, 200, "{a}");
    let f = feld_der_antwort(&a, "spenden_betrag");
    assert_eq!((f["summe_cent"].as_i64(), f["anzahl"].as_i64(), f["folge"].as_str()), (Some(11_00), Some(1), Some("geschrieben")), "{f}");
    let p = posten_eines_felds(&a, "spenden_betrag");
    assert_eq!(p.iter().map(|p| p["abgewaehlt"].as_bool().unwrap()).collect::<Vec<_>>(), [false, true]);
    assert_eq!(p[1]["index"], json!(1), "die abgewaehlte Zeile bleibt in der Liste");

    // Die erste und die letzte Stelle eines Felds.
    let (_, a) = sende(&d, "/fall/v4/bank/vorschau", &rumpf(&AUSZUG, &json!({"abgewaehlt": [0]}))).await;
    assert_eq!(feld_der_antwort(&a, "spenden_betrag")["summe_cent"], json!(23_00));
    let (_, a) = sende(&d, "/fall/v4/bank/vorschau", &rumpf(&AUSZUG, &json!({"abgewaehlt": [6]}))).await;
    assert_eq!(feld_der_antwort(&a, "vor_rv_ausserhalb_lstb")["folge"], json!("abgewaehlt"), "{a}");

    // Alle Posten eines Felds abgewaehlt.
    let (_, a) = sende(&d, "/fall/v4/bank/vorschau", &rumpf(&AUSZUG, &json!({"abgewaehlt": [0, 1]}))).await;
    let f = feld_der_antwort(&a, "spenden_betrag");
    assert_eq!((f["summe_cent"].as_i64(), f["anzahl"].as_i64(), f["folge"].as_str()), (Some(0), Some(0), Some("abgewaehlt")), "{f}");
    assert_eq!(posten_eines_felds(&a, "spenden_betrag").len(), 2);
    assert_eq!(a["uebernommen"], json!(1), "nur die Vorsorge ginge ein: {a}");
}

/// `abgewaehlt` ist eine Liste von Stellen: jede andere Form, eine Stelle jenseits der Liste und eine doppelte Stelle sind 400, in
/// der Vorschau wie beim Uebernehmen, und nichts wird geschrieben.
#[tokio::test]
async fn bank_abgewaehlt_ungueltig_ist_400_und_schreibt_nichts() {
    let d = dienst();
    fall_anlegen(&d, "v5").await;
    let vorher = std::fs::read(akte_pfad(&d, "v5")).unwrap();
    for (name, wert) in [
        ("null", Value::Null),
        ("kein Array", json!("1")),
        ("Wahrheitswert", json!(true)),
        ("Objekt", json!({"a": 1})),
        ("Wahrheitswert in der Liste", json!([true])),
        ("Kommazahl", json!([1.5])),
        ("negativ", json!([-1])),
        ("Text in der Liste", json!(["1"])),
        ("doppelt", json!([1, 1])),
        ("jenseits der Liste", json!([7])),
        ("weit jenseits", json!([4_000_000_000_u64])),
    ] {
        for pfad in ["/fall/v5/bank/vorschau", "/fall/v5/kontoauszug"] {
            let (s, a) = sende(&d, pfad, &rumpf(&AUSZUG, &json!({"abgewaehlt": wert}))).await;
            assert_eq!(s, 400, "{name} {pfad}: {a}");
            assert!(a["fehler"].as_str().is_some_and(|t| t.contains("abgewaehlt")), "{name} {pfad}: {a}");
        }
    }
    assert_eq!(std::fs::read(akte_pfad(&d, "v5")).unwrap(), vorher, "ein 400 darf nichts schreiben");
    // Kontrolle: die letzte gueltige Stelle (6) und die leere Liste gehen durch.
    for gueltig in [json!([]), json!([6]), json!([0, 6])] {
        let (s, a) = sende(&d, "/fall/v5/bank/vorschau", &rumpf(&AUSZUG, &json!({"abgewaehlt": gueltig}))).await;
        assert_eq!(s, 200, "{gueltig}: {a}");
    }
}

/// Uebernehmen mit `abgewaehlt`: der Server schreibt die Summe ohne die abgewaehlte Buchung; der Posten steht nicht in der Akte; das
/// Protokoll nennt die Zahl der abgewaehlten (nur die Zahl).
#[tokio::test]
async fn bank_uebernehmen_abgewaehlt_schreibt_die_summe_ohne_den_posten() {
    let d = dienst();
    fall_anlegen(&d, "u1").await;
    let (s, a) = sende(&d, "/fall/u1/kontoauszug", &rumpf(&AUSZUG, &json!({"abgewaehlt": [0, 5]}))).await;
    assert_eq!(s, 200, "{a}");
    // Die Spende an Stelle 0 ist abgewaehlt; die Krankenversicherung (Stelle 5) ist ohne den Haken keine Zeile: ohne Wirkung.
    assert_eq!(a["uebernommen"], json!(2), "{a}");
    let ev = events(&d, "u1");
    let spenden = ev.iter().find(|e| e["feld_id"] == "spenden_betrag").unwrap();
    assert_eq!(spenden["wert"], json!(23_00));
    assert_eq!(spenden["signal"]["signal_1"]["posten"].as_array().unwrap().len(), 1);
    assert_eq!(spenden["signal"]["signal_1"]["posten"][0]["betrag_cent"], json!(-23_00));
    let eintraege = store::audit::lies(&d.zustand.konfig.audit_pfad()).unwrap();
    let import = eintraege.iter().find(|e| e.action.als_str() == "bank_import").unwrap();
    let detail = import.detail.as_deref().unwrap();
    assert!(detail.contains("abgewaehlt=1 "), "nur die Spende zaehlt als abgewaehlt: {detail}");
    for geheim in ["Tierheim", "1100", "11,00"] {
        assert!(!detail.contains(geheim), "{geheim:?} steht im Protokoll: {detail}");
    }
    // Ohne das Feld bleibt die Antwort, wie sie war: kein `kreuz_abwahl`, und das Protokoll zaehlt 0.
    fall_anlegen(&d, "u2").await;
    let (_, a) = sende(&d, "/fall/u2/kontoauszug", &rumpf(&AUSZUG, &json!({}))).await;
    assert!(a.get("kreuz_abwahl").is_none(), "{a}");
    let eintraege = store::audit::lies(&d.zustand.konfig.audit_pfad()).unwrap();
    let u2 = eintraege.iter().find(|e| e.action.als_str() == "bank_import" && e.fall_id.as_deref() == Some("u2")).unwrap();
    assert!(u2.detail.as_deref().unwrap().contains("abgewaehlt=0 "), "{u2:?}");
}

/// Die Vorschau hat dieselbe Anmeldung, denselben Besitzer-Check und dieselbe Groessenbegrenzung wie `kontoauszug`: ohne Token 401,
/// ein fremder Nutzer und ein unbekannter Fall bekommen denselben Status wie dort, ein zu grosser Rumpf 413, und nichts wird geschrieben.
#[tokio::test]
async fn bank_vorschau_hat_anmeldung_besitzer_check_und_groessenbegrenzung_wie_kontoauszug() {
    let d = dienst();
    fall_anlegen(&d, "z1").await;
    let akte = std::fs::read(akte_pfad(&d, "z1")).unwrap();
    let body = rumpf(&AUSZUG, &json!({}));
    for (pfad_vorschau, pfad_import) in [
        ("/fall/z1/bank/vorschau", "/fall/z1/kontoauszug"),
        ("/fall/gibt-es-nicht/bank/vorschau", "/fall/gibt-es-nicht/kontoauszug"),
    ] {
        let (s_v, a_v) = anfrage(&d, "POST", pfad_vorschau, None, &[], &body).await;
        let (s_i, a_i) = anfrage(&d, "POST", pfad_import, None, &[], &body).await;
        assert_eq!((s_v, &a_v), (s_i, &a_i), "ohne Token: {pfad_vorschau}");
        assert_eq!(s_v, 401, "{a_v}");
    }
    // Ein fremder Nutzer: derselbe Status wie beim Import, und der ist kein Erfolg.
    let (s_v, a_v) = anfrage(&d, "POST", "/fall/z1/bank/vorschau", Some(&d.fremd), &[], &body).await;
    let (s_i, a_i) = anfrage(&d, "POST", "/fall/z1/kontoauszug", Some(&d.fremd), &[], &body).await;
    assert_eq!((s_v, &a_v), (s_i, &a_i), "fremder Nutzer");
    assert!(s_v == 403 || s_v == 404, "{s_v}: {a_v}");
    assert!(a_v.get("felder").is_none(), "die Vorschau gibt einem Fremden nichts: {a_v}");
    // Ein unbekannter Fall mit eigenem Token.
    let (s_v, a_v) = sende(&d, "/fall/gibt-es-nicht/bank/vorschau", &body).await;
    let (s_i, a_i) = sende(&d, "/fall/gibt-es-nicht/kontoauszug", &body).await;
    assert_eq!((s_v, &a_v), (s_i, &a_i), "unbekannter Fall");
    assert!(s_v >= 400, "{s_v}");
    // Zu grosser Rumpf: die Laenge steht im Kopf (32 MiB + 1), der Rumpf ist klein; der Dispatcher weist vor dem Handler ab.
    let gross = [("content-length", (32 * 1024 * 1024 + 1).to_string())];
    let (s_v, a_v) = anfrage(&d, "POST", "/fall/z1/bank/vorschau", Some(&d.token), &gross, &body).await;
    let (s_i, a_i) = anfrage(&d, "POST", "/fall/z1/kontoauszug", Some(&d.token), &gross, &body).await;
    assert_eq!((s_v, &a_v), (s_i, &a_i), "zu grosser Rumpf");
    assert_eq!(s_v, 413, "{a_v}");
    assert_eq!(std::fs::read(akte_pfad(&d, "z1")).unwrap(), akte, "kein Aufruf darf die Akte aendern");
}

/// Die Vorschau schreibt nicht, weil sie es nicht kann: der Handler bekommt eine Kopie und hat keinen Aufruf von `speichere`, keinen
/// Zugriff auf `store_mut` und keinen Protokollschreiber. Ein Wachter am Quelltext des Moduls (Mutant: ein `speichere`-Aufruf).
#[test]
fn bank_vorschau_modul_hat_keinen_schreibweg() {
    let quelle = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bank.rs"))
        .expect("rust/api/src/bank.rs fehlt");
    // Kommentare nennen die Namen, der Code nicht.
    let code: String = quelle.lines().filter(|z| !z.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
    assert!(code.contains(".clone()"), "Kontrolle: das Modul rechnet auf einer Kopie des Stores");
    for verboten in ["speichere", "store_mut", "anhaengen", "std::fs", "File::create", "write("] {
        assert!(!code.contains(verboten), "rust/api/src/bank.rs nennt {verboten:?}");
    }
}

// ---------------------------------------------------------------------------------------------------------------------------
// Das Eingangskreuz `keine_spenden` (Bankimport S3, Entscheidung von main zu Q2)
// ---------------------------------------------------------------------------------------------------------------------------

const SPENDE_NEU: &str = "01.03.2025;-11,00;Spende Tierheim";

/// Das Kreuz als Antwort der Nutzerin, wie die Seite es schreibt (`screeningWeiter`): `false` heisst „ja, gespendet“.
async fn kreuz_setzen(d: &Dienst, id: &str, wert: bool) {
    let b = json!({"feld_id": "keine_spenden", "wert": wert, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie", "signal": {"signal_1": null, "signal_2": "screening@keine_spenden"}});
    let (s, a) = sende(d, &format!("/fall/{id}/event"), &b).await;
    assert_eq!(s, 201, "{a}");
}

fn kreuz(a: &Value) -> Option<&Value> {
    a.get("kreuz_abwahl").and_then(Value::as_array).and_then(|l| l.first())
}

/// Die Antwort von „Uebernehmen“ nennt das Kreuz NUR, wenn in diesem Aufruf wirklich ein Ereignis auf `spenden_betrag` geschrieben
/// wurde und die Seite die Abwahl verlangte (das Feld `abgewaehlt` steht im Rumpf).
#[tokio::test]
async fn bank_kreuz_abwahl_antwort_nennt_das_kreuz_nur_bei_geschriebenem_spenden_ereignis() {
    let d = dienst();
    // 1. Spende, kein Kreuz bisher: das Kreuz steht mit `aktuell: null` in der Antwort.
    fall_anlegen(&d, "k1").await;
    let (s, a) = sende(&d, "/fall/k1/kontoauszug", &rumpf(&[SPENDE_NEU], &json!({"abgewaehlt": []}))).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["kreuz_abwahl"], json!([{"feld_id": "keine_spenden", "aktuell": null, "grund_feld": "spenden_betrag"}]), "{a}");
    // Der Server schreibt das Kreuz nie.
    assert!(!felder_der_events(&d, "k1").contains(&"keine_spenden".to_owned()), "der Import hat das Kreuz geschrieben");

    // 2. Ohne das Feld `abgewaehlt` bleibt die Antwort, wie sie war.
    fall_anlegen(&d, "k2").await;
    let (_, a) = sende(&d, "/fall/k2/kontoauszug", &rumpf(&[SPENDE_NEU], &json!({}))).await;
    assert!(a.get("kreuz_abwahl").is_none(), "{a}");

    // 3. Kein Spenden-Ereignis: nur ein Handwerker, nur Vorsorge, nur eine Einnahme.
    for (id, zeile) in [("k3a", "01.03.2025;-73,00;Maler Huber"), ("k3b", "01.03.2025;-24,00;Rentenversicherung Ruerup"), ("k3c", "01.03.2025;50,00;Gehalt")] {
        fall_anlegen(&d, id).await;
        let (_, a) = sende(&d, &format!("/fall/{id}/kontoauszug"), &rumpf(&[zeile], &json!({"abgewaehlt": []}))).await;
        assert!(a.get("kreuz_abwahl").is_none(), "{id}: {a}");
    }

    // 4. Alle Spenden-Posten abgewaehlt: kein Ereignis, kein Kreuz. Nur einer abgewaehlt: Ereignis, Kreuz.
    fall_anlegen(&d, "k4").await;
    let zwei = ["01.03.2025;-11,00;Spende Tierheim", "02.03.2025;-23,00;Spende Unicef"];
    let (_, a) = sende(&d, "/fall/k4/kontoauszug", &rumpf(&zwei, &json!({"abgewaehlt": [0, 1]}))).await;
    assert!(a.get("kreuz_abwahl").is_none(), "{a}");
    assert!(!felder_der_events(&d, "k4").contains(&"spenden_betrag".to_owned()));
    let (_, a) = sende(&d, "/fall/k4/kontoauszug", &rumpf(&zwei, &json!({"abgewaehlt": [1]}))).await;
    assert!(kreuz(&a).is_some(), "{a}");

    // 5. Ein Feld mit eigenem Vorschlag aus einem frueheren Auszug: ohne Haken `ersetzen` keine Schreibung, kein Kreuz; mit Haken
    // schreibt der Import, und das Kreuz steht in der Antwort.
    fall_anlegen(&d, "k5").await;
    let (_, a) = sende(&d, "/fall/k5/kontoauszug", &rumpf(&[zwei[0]], &json!({}))).await;
    assert_eq!(a["uebernommen"], json!(1), "{a}");
    let (_, a) = sende(&d, "/fall/k5/kontoauszug", &rumpf(&zwei, &json!({"abgewaehlt": []}))).await;
    assert!(a["ersetzbar"].as_array().is_some_and(|l| l.len() == 1), "Kontrolle: die Rueckfrage steht aus: {a}");
    assert!(a.get("kreuz_abwahl").is_none(), "nichts geschrieben, also kein Kreuz: {a}");
    let (_, a) = sende(&d, "/fall/k5/kontoauszug", &rumpf(&zwei, &json!({"abgewaehlt": [], "ersetzen": true}))).await;
    assert!(kreuz(&a).is_some(), "{a}");

    // 6. Ein Feld mit Beleg: der Import schreibt nichts, kein Kreuz.
    fall_anlegen(&d, "k6").await;
    let beleg = json!({"feld_id": "spenden_betrag", "wert": 40_00, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}, "schreiber": "ui:laie",
        "signal": {"signal_1": null, "signal_2": null}});
    let (s, a) = sende(&d, "/fall/k6/event", &beleg).await;
    assert_eq!(s, 201, "{a}");
    let (_, a) = sende(&d, "/fall/k6/kontoauszug", &rumpf(&[SPENDE_NEU], &json!({"abgewaehlt": []}))).await;
    assert_eq!(a["uebernommen"], json!(0), "{a}");
    assert!(a.get("kreuz_abwahl").is_none(), "{a}");
}

/// Der Stand des Kreuzes im Fall: `false` („ja, gespendet“) braucht keine Abwahl, `true` („keine Spenden“) nennt der Server mit
/// `aktuell: true`, damit die Seite das Kaestchen AUS laesst und den Widerspruch nennt. Die Vorschau nennt dieselbe Zeile wie das
/// Uebernehmen.
#[tokio::test]
async fn bank_kreuz_abwahl_stand_des_kreuzes_und_vorschau_gleich_uebernehmen() {
    let d = dienst();
    for (id, stand, soll) in [
        ("s1", None, Some(json!([{"feld_id": "keine_spenden", "aktuell": null, "grund_feld": "spenden_betrag"}]))),
        ("s2", Some(true), Some(json!([{"feld_id": "keine_spenden", "aktuell": true, "grund_feld": "spenden_betrag"}]))),
        ("s3", Some(false), None),
    ] {
        fall_anlegen(&d, id).await;
        if let Some(wert) = stand {
            kreuz_setzen(&d, id, wert).await;
        }
        let body = rumpf(&[SPENDE_NEU], &json!({"abgewaehlt": []}));
        let (s, v) = sende(&d, &format!("/fall/{id}/bank/vorschau"), &body).await;
        assert_eq!(s, 200, "{v}");
        assert_eq!(v.get("kreuz_abwahl"), soll.as_ref(), "Vorschau {id}: {v}");
        let (s, u) = sende(&d, &format!("/fall/{id}/kontoauszug"), &body).await;
        assert_eq!(s, 200, "{u}");
        assert_eq!(u.get("kreuz_abwahl"), soll.as_ref(), "Uebernehmen {id}: {u}");
    }
    // Die Vorschau nennt das Kreuz auch ohne das Feld `abgewaehlt` (die Seite fragt es nicht eigens an).
    fall_anlegen(&d, "s4").await;
    let (_, v) = sende(&d, "/fall/s4/bank/vorschau", &rumpf(&[SPENDE_NEU], &json!({}))).await;
    assert!(kreuz(&v).is_some(), "{v}");
    // Keine Spende in der Vorschau: kein Kreuz; alle Spenden abgewaehlt: kein Kreuz.
    let (_, v) = sende(&d, "/fall/s4/bank/vorschau", &rumpf(&["01.03.2025;-73,00;Maler Huber"], &json!({}))).await;
    assert!(v.get("kreuz_abwahl").is_none(), "{v}");
    let (_, v) = sende(&d, "/fall/s4/bank/vorschau", &rumpf(&[SPENDE_NEU], &json!({"abgewaehlt": [0]}))).await;
    assert!(v.get("kreuz_abwahl").is_none(), "{v}");
}

/// Das Ereignis der Seite spart genau eine Frage (Messung des Tickets: 370 -> 369, nicht der Spendenblock): die Frage
/// `keine_spenden` steht vorher in der Liste und nachher nicht mehr; sonst aendert sich die Zahl nicht.
#[tokio::test]
async fn bank_kreuz_abwahl_das_ereignis_der_seite_spart_genau_eine_frage() {
    let d = dienst();
    fall_anlegen(&d, "f1").await;
    let fragen = |a: &Value| -> Vec<String> {
        a["fragen"].as_array().unwrap().iter().map(|q| q["feld_id"].as_str().unwrap().to_owned()).collect()
    };
    let (s, vorher) = anfrage(&d, "GET", "/fall/f1/fragen", Some(&d.token), &[], &json!({})).await;
    assert_eq!(s, 200, "{vorher}");
    let vorher = fragen(&vorher);
    assert!(vorher.contains(&"keine_spenden".to_owned()), "Kontrolle: die Kreuzfrage steht in der Liste");
    // Ein vorlaeufiger Vorschlag (der Import) aendert die Zahl der Fragen nicht und nimmt das Kreuz nicht weg.
    let (s, _) = sende(&d, "/fall/f1/kontoauszug", &rumpf(&[SPENDE_NEU], &json!({"abgewaehlt": []}))).await;
    assert_eq!(s, 200);
    let (_, nach_import) = anfrage(&d, "GET", "/fall/f1/fragen", Some(&d.token), &[], &json!({})).await;
    assert_eq!(fragen(&nach_import).len(), vorher.len(), "der Import allein aendert die Zahl der Fragen nicht");
    kreuz_setzen(&d, "f1", false).await;
    let (_, nachher) = anfrage(&d, "GET", "/fall/f1/fragen", Some(&d.token), &[], &json!({})).await;
    let nachher = fragen(&nachher);
    assert_eq!(nachher.len(), vorher.len() - 1, "genau die Kreuzfrage faellt weg");
    assert!(!nachher.contains(&"keine_spenden".to_owned()));
}
