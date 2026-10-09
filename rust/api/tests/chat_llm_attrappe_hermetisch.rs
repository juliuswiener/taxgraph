//! `POST /fall/{id}/chat` gegen einen lokalen Attrappen-Dienst fuer das Sprachmodell, ohne Netz und
//! ohne Python. `$LLM_API_BASE` zeigt auf `127.0.0.1`, Modell und Schluessel sind Platzhalter.
//!
//! Die Mutationsmessung der duennen `api`-Dateien (Auftrag 8) fand fuer `chat.rs` nur die Wache des
//! Vergleichs mit Python (`PARITY=1`, `extern_stub`). Dieser Test haelt die Gestalt der Antwort
//! fest, auch wo kein Python laeuft: `verarbeite` (Scheiben-Gate, Konflikt, Store-Abweisung),
//! `beobachte`, `freitext_von`, `offenes_feld`, der Kontext an das Modell und der Mitschnitt.
//! Der Szenario-Satz `drei_stufen` ist der des Vergleichslaufs (`rust/parity/tests/extern_stub`).
//!
//! Der Dienst erkennt die Stufe am Schemanamen der Anfrage (`aussagen`, `zuordnung`, `dialog`) und
//! antwortet nach einem Skript. Die Umgebung gilt fuer den ganzen Prozess; darum steht alles in
//! EINEM Test.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    clippy::panic,
    clippy::too_many_lines
)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const TEXT: &str = "Wir veranlagen gemeinsam mit meiner Frau. Ich habe 6000 Euro Unterhalt an meinen Ex-Mann gezahlt, habe das Merkzeichen G und 100 Euro Lohn.";

struct Schritt {
    stufe: &'static str,
    status: u16,
    body: String,
    oft: bool,
}

/// Eine Modell-Antwort mit `inhalt` als Text der ersten Wahl.
fn modell(stufe: &'static str, inhalt: &Value) -> Schritt {
    Schritt {
        stufe,
        status: 200,
        body: json!({
            "provider": "StubAnbieter",
            "choices": [{"finish_reason": "stop", "message": {"content": inhalt.to_string()}}]
        })
        .to_string(),
        oft: false,
    }
}

/// Wie [`modell`], aber mit dem Text unveraendert, auch wo er kein JSON ist (`NaN`, `1e400`, ein einzelnes
/// Surrogat): der aeussere Koerper bleibt gueltiges JSON, nur der Inhalt der ersten Wahl ist es nicht.
fn modell_roh(stufe: &'static str, text: &str) -> Schritt {
    Schritt {
        stufe,
        status: 200,
        body: json!({
            "provider": "StubAnbieter",
            "choices": [{"finish_reason": "stop", "message": {"content": text}}]
        })
        .to_string(),
        oft: false,
    }
}

/// Ein `rechenweg` des Modells (Schema `dialog`): Basis, Faktor, Erklaerung. Er geht unveraendert in die
/// Antwort (`vorschlaege`, `konflikte`).
fn rechenweg() -> Value {
    json!({"basis": 1_200_000, "faktor": 0.5, "erklaerung": "12000 Euro Unterhalt im Jahr, ein halbes Jahr"})
}

/// Die drei Stufen eines Gespraechs mit fuenf Vorschlaegen: zwei gueltige, ein scheibenfremdes Feld,
/// ein Wert vom falschen Typ und ein Feld, das der Katalog dem Modell nicht freigibt (nicht `askable`).
fn drei_stufen() -> Vec<Schritt> {
    vec![
        aussagen_stufe(),
        modell("zuordnung", &json!({"zuordnungen": []})),
        modell(
            "dialog",
            &json!({"vorschlaege": [
                {"feld_id": "veranlagung", "wert": "zusammen", "beleg": "gemeinsam mit meiner Frau",
                 "begruendung": "b0", "aussage": 0, "rechenweg": null},
                {"feld_id": "realsplitting_unterhaltsleistungen", "wert": 600_000, "beleg": "6000 Euro Unterhalt",
                 "begruendung": "b1", "aussage": 1, "rechenweg": rechenweg()},
                {"feld_id": "gibt_es_nicht", "wert": 1, "beleg": "Merkzeichen G",
                 "begruendung": "b2", "aussage": 1, "rechenweg": null},
                {"feld_id": "fahrtkosten_pausch_gdb80_oder_70g", "wert": "ja", "beleg": "Merkzeichen G",
                 "begruendung": "b3", "aussage": 1, "rechenweg": null},
                {"feld_id": "kap_antrag_guenstigerpruefung", "wert": true, "beleg": "100 Euro Lohn",
                 "begruendung": "b4", "aussage": 1, "rechenweg": null}],
                "rueckfragen": [], "antwort": "Das habe ich verstanden.", "unsicher": false}),
        ),
    ]
}

type Skript = Arc<Mutex<Vec<Schritt>>>;
type Gesehen = Arc<Mutex<Vec<(String, Value)>>>;

/// Startet den Dienst und gibt die Basis-URL zurueck. Der Thread laeuft bis zum Ende des Prozesses.
fn attrappen_dienst(skript: Skript, gesehen: Gesehen) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let basis = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
    std::thread::spawn(move || {
        for s in l.incoming() {
            let mut s = s.unwrap();
            let mut puffer = Vec::new();
            let mut b = [0u8; 8192];
            let (kopf_ende, laenge) = loop {
                let n = s.read(&mut b).unwrap();
                if n == 0 {
                    break (0, 0);
                }
                puffer.extend_from_slice(&b[..n]);
                let text = String::from_utf8_lossy(&puffer).into_owned();
                if let Some(i) = text.find("\r\n\r\n") {
                    let laenge = text[..i]
                        .lines()
                        .find_map(|z| {
                            z.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if puffer.len() >= i + 4 + laenge {
                        break (i + 4, laenge);
                    }
                }
            };
            let anfrage: Value = serde_json::from_slice(&puffer[kopf_ende..kopf_ende + laenge])
                .unwrap_or(Value::Null);
            let stufe = anfrage["response_format"]["json_schema"]["name"]
                .as_str()
                .unwrap_or("?")
                .to_owned();
            gesehen.lock().unwrap().push((stufe.clone(), anfrage));
            let (status, body) = {
                let mut k = skript.lock().unwrap();
                match k.iter().position(|st| st.stufe == stufe) {
                    Some(p) if k[p].oft => (k[p].status, k[p].body.clone()),
                    Some(p) => {
                        let st = k.remove(p);
                        (st.status, st.body)
                    }
                    None => (599, r#"{"error": "ungeplant"}"#.to_owned()),
                }
            };
            let antwort = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(antwort.as_bytes());
        }
    });
    basis
}

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
    let token = format!("Bearer {}", zustand.auth.stelle_aus("alice").unwrap());
    Dienst {
        zustand,
        token,
        _tmp: tmp,
    }
}

async fn post(d: &Dienst, pfad: &str, rumpf: &Value) -> (u16, Value) {
    let text = rumpf.to_string();
    let req = Request::builder()
        .method("POST")
        .uri(pfad)
        .header("authorization", &d.token)
        .header("content-type", "application/json")
        .header("content-length", text.len().to_string())
        .body(Body::from(text))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("kein JSON: {e}: {}", String::from_utf8_lossy(&bytes)));
    (teile.status.as_u16(), json)
}

fn akte(d: &Dienst, fall: &str) -> Value {
    let p = d.zustand.konfig.faelle.join(format!("{fall}.json"));
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

/// Ein bestaetigtes Ereignis direkt in die Akte schreiben (wie `setze_felder` in `offene_defekte.rs`).
fn setze_bestaetigt(d: &Dienst, fall: &str, paare: &[(&str, Value)]) {
    let pfad = d.zustand.konfig.faelle.join(format!("{fall}.json"));
    let mut datei = store::lade(&pfad).unwrap();
    for (i, (fid, wert)) in paare.iter().enumerate() {
        let mut e = json!({
            "ts": format!("2026-01-01T00:00:{i:02}+00:00"), "feld_id": fid, "wert": wert,
            "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": null, "signal_2": "ok"},
        });
        e["event_id"] = json!(store::EventId::von_json(&e).to_string());
        datei.events.push(serde_json::from_value(e).unwrap());
    }
    store::speichere(&pfad, &datei).unwrap();
}

/// Der Inhalt aller Nachrichten einer Anfrage mit der Rolle `rolle`, zusammengefuegt.
fn nachricht(anfrage: &Value, rolle: &str) -> String {
    anfrage["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["role"] == json!(rolle))
        .map(|m| m["content"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

fn dialog_anfrage(gesehen: &Gesehen) -> Value {
    gesehen
        .lock()
        .unwrap()
        .iter()
        .find(|(s, _)| s == "dialog")
        .unwrap_or_else(|| panic!("keine Dialog-Anfrage"))
        .1
        .clone()
}

/// Der `detail` der letzten `llm_call`-Zeile im Audit.
fn letzte_llm_zeile(d: &Dienst) -> String {
    let audit = std::fs::read_to_string(d.zustand.konfig.audit_pfad()).unwrap();
    audit
        .lines()
        .rev()
        .map(|z| serde_json::from_str::<Value>(z).unwrap())
        .find(|z| z["action"] == "llm_call")
        .map_or_else(
            || panic!("keine llm_call-Zeile: {audit}"),
            |z| z["detail"].as_str().unwrap().to_owned(),
        )
}

/// Die erste Stufe der Szenarien `drei_stufen`.
fn aussagen_stufe() -> Schritt {
    modell(
        "aussagen",
        &json!({"aussagen": [
            {"text": "Der Nutzer veranlagt gemeinsam mit seiner Frau", "beleg": "gemeinsam mit meiner Frau"},
            {"text": "Der Nutzer zahlte 6000 Euro Unterhalt", "beleg": "6000 Euro Unterhalt"}]}),
    )
}

/// Auftrag k9-2 (Mutant C46): der Anbieter-Merker (`llm::letzter_anbieter`) liegt je Thread. Ob der Aufruf mit
/// fehlender Umgebung den Anbieter des vorigen Aufrufs sieht, hängt davon ab, ob beide auf demselben Thread des
/// Blocking-Pools laufen; mit dem Pool des `#[tokio::test]` entscheidet das der Zufall (der Mutant überlebte
/// einen Lauf). Eine Laufzeit mit genau EINEM Blocking-Thread macht es zur Regel.
#[test]
fn chat_vom_dienst_bis_zur_akte() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap()
        .block_on(chat_vom_dienst_bis_zur_akte_lauf());
}

async fn chat_vom_dienst_bis_zur_akte_lauf() {
    let skript: Skript = Arc::new(Mutex::new(Vec::new()));
    let gesehen: Gesehen = Arc::new(Mutex::new(Vec::new()));
    let basis = attrappen_dienst(skript.clone(), gesehen.clone());
    std::env::set_var("LLM_API_BASE", &basis);
    std::env::set_var("LLM_MODEL", "stub/modell");
    std::env::set_var("LLM_API_KEY", "SYNTHETISCH-LLM-SCHLUESSEL");
    std::env::remove_var("TAXGRAPH_FLOW");
    let d = dienst();
    let (s, a) = post(
        &d,
        "/fall",
        &json!({"fall_id": "ch1", "scheibe": "gesamt", "veranlagungszeitraum": 2025}),
    )
    .await;
    assert_eq!(s, 201, "{a}");

    // --- Drei Stufen: zwei Vorschlaege geschrieben, drei abgelehnt.
    *skript.lock().unwrap() = drei_stufen();
    let (s, a) = post(&d, "/fall/ch1/chat", &json!({"text": TEXT})).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["antwort"], json!("Das habe ich verstanden."), "{a}");
    assert_eq!(a["unsicher"], json!(false), "{a}");
    assert_eq!(a["konflikte"], json!([]), "{a}");
    assert_eq!(a["rueckfragen"], json!([]), "{a}");
    assert_eq!(a["rueckfragen_zurueckgestellt"], json!(0), "{a}");
    assert_eq!(a["aussagen"].as_array().unwrap().len(), 2, "{a}");
    assert_eq!(
        a["hinweis"],
        json!("Vorschläge erfasst — bitte jeden einzeln bestätigen (die KI setzt nichts)."),
        "{a}"
    );
    let v = a["vorschlaege"].as_array().unwrap();
    assert_eq!(v.len(), 2, "{a}");
    assert_eq!(
        (
            v[0]["feld_id"].as_str(),
            v[0]["wert"].clone(),
            v[0]["beleg"].as_str(),
            v[0]["typ"].as_str()
        ),
        (
            Some("veranlagung"),
            json!("zusammen"),
            Some("gemeinsam mit meiner Frau"),
            Some("enum")
        ),
        "{a}"
    );
    assert_eq!(
        (
            v[1]["feld_id"].as_str(),
            v[1]["wert"].clone(),
            v[1]["einheit"].as_str(),
            v[1]["frage_invertiert"].clone()
        ),
        (
            Some("realsplitting_unterhaltsleistungen"),
            json!(600_000),
            Some("EUR"),
            json!(false)
        ),
        "{a}"
    );
    assert!(
        v[0]["rechenweg"].is_null() && v[0]["enum_labels"].is_object(),
        "{a}"
    );
    // Der Rechenweg des Modells geht unveraendert durch (Mutant H53: `null`).
    assert_eq!(v[1]["rechenweg"], rechenweg(), "{a}");
    // Abgelehnt: in der Reihenfolge der Vorschlaege, mit Klasse und Feld, nie mit dem Wert.
    assert_eq!(
        a["abgelehnt"],
        json!([
            "gibt_es_nicht",
            "fahrtkosten_pausch_gdb80_oder_70g",
            "kap_antrag_guenstigerpruefung"
        ]),
        "{a}"
    );
    let g = &a["abgelehnt_gruende"];
    assert_eq!(
        g["gibt_es_nicht"],
        json!("scheibenfremd: Feld gehört nicht zu dieser Scheibe"),
        "{a}"
    );
    for fid in [
        "fahrtkosten_pausch_gdb80_oder_70g",
        "kap_antrag_guenstigerpruefung",
    ] {
        let grund = g[fid]
            .as_str()
            .unwrap_or_else(|| panic!("kein Grund fuer {fid}: {a}"));
        assert!(
            grund.starts_with("fail-closed") && grund.ends_with(&format!(": {fid}")),
            "{a}"
        );
    }
    // Die Akte: genau zwei Events, vorlaeufig, vom Chat, mit dem Beleg im ersten Signal.
    let datei = akte(&d, "ch1");
    let ev = datei["events"].as_array().unwrap();
    assert_eq!(ev.len(), 2, "{datei}");
    assert_eq!(v[0]["event_id"], ev[0]["event_id"], "{a}");
    assert_eq!(v[1]["event_id"], ev[1]["event_id"], "{a}");
    for (e, fid, wert, beleg, begr) in [
        (
            &ev[0],
            "veranlagung",
            json!("zusammen"),
            "gemeinsam mit meiner Frau",
            "b0",
        ),
        (
            &ev[1],
            "realsplitting_unterhaltsleistungen",
            json!(600_000),
            "6000 Euro Unterhalt",
            "b1",
        ),
    ] {
        assert_eq!(
            (
                e["feld_id"].as_str(),
                e["wert"].clone(),
                e["zustand"].as_str(),
                e["schreiber"].as_str()
            ),
            (Some(fid), wert, Some("vorlaeufig"), Some("llm:chat"))
        );
        assert_eq!(
            e["herkunft"],
            json!({"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"})
        );
        assert_eq!(
            e["signal"]["signal_1"],
            json!({"typ": "llm", "begruendung": begr, "beleg": beleg})
        );
        assert!(e["signal"]["signal_2"].is_null());
    }
    // Der Dienst sah drei Anfragen, in der Reihenfolge der Stufen, mit Modell und Text.
    let sah = gesehen.lock().unwrap().clone();
    assert_eq!(
        sah.iter().map(|(s, _)| s.as_str()).collect::<Vec<_>>(),
        ["aussagen", "zuordnung", "dialog"]
    );
    assert!(sah.iter().all(|(_, b)| b["model"] == json!("stub/modell")));
    assert_eq!(nachricht(&sah[0].1, "user"), TEXT);
    // Der Katalog im Prompt der dritten Stufe: nur LLM-vorschlagbare Felder, mit Typ, Werten, Kurzhilfe.
    let system = nachricht(&sah[2].1, "system");
    assert!(system.contains("- veranlagung: Gibst du die Erklärung allein oder gemeinsam mit Ehe-/Lebenspartner ab? (Typ enum, Werte ['einzel', 'zusammen'])\n    dazu gehört: Gemeinsam = Zusammenveranlagung"), "{system}");
    assert!(
        system.contains("(Typ cent)\n    dazu gehört: Unterhaltsleistungen werden bis 13.805 Euro"),
        "{system}"
    );
    assert!(system.contains("(Typ int, Bereich {'min': 20, 'max': 100, 'grund': 'Spannweite des Grads der Behinderung des Kindes."), "{system}");
    assert!(system.contains("- bruttoarbeitslohn: Wie hoch war dein Bruttoarbeitslohn dieses Jahr? (Nummer 3 der Lohnsteuerbescheinigung) (Typ cent)"), "{system}");
    assert!(
        !system.contains("- kap_antrag_guenstigerpruefung:"),
        "{system}"
    );
    gesehen.lock().unwrap().clear();

    // --- Dasselbe Feld noch einmal mit anderem Wert: ein Konflikt, kein zweites Event; ein Feld, das das
    // Modell nicht vorschlagen darf, ist auch dann kein Konflikt, wenn es schon einen Wert hat.
    setze_bestaetigt(&d, "ch1", &[("kap_antrag_guenstigerpruefung", json!(true))]);
    let mut stufen = drei_stufen();
    stufen[2] = modell(
        "dialog",
        &json!({"vorschlaege": [
            {"feld_id": "veranlagung", "wert": "einzel", "beleg": "gemeinsam mit meiner Frau",
             "begruendung": "neu", "aussage": 0, "rechenweg": rechenweg()},
            {"feld_id": "kap_antrag_guenstigerpruefung", "wert": false, "beleg": "100 Euro Lohn",
             "begruendung": "b4", "aussage": 1, "rechenweg": null},
            {"feld_id": "realsplitting_unterhaltsleistungen", "wert": 700_000, "beleg": "6000 Euro Unterhalt",
             "begruendung": "noch ein Wert", "aussage": 1, "rechenweg": null},
            {"feld_id": "", "wert": 1, "beleg": "100 Euro Lohn",
             "begruendung": "ohne Feld", "aussage": 1, "rechenweg": null}],
            "rueckfragen": [], "antwort": "", "unsicher": true}),
    );
    *skript.lock().unwrap() = stufen;
    let (s, a) = post(&d, "/fall/ch1/chat", &json!({"text": TEXT})).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["vorschlaege"], json!([]), "{a}");
    assert_eq!(
        a["abgelehnt"],
        json!(["kap_antrag_guenstigerpruefung"]),
        "{a}"
    );
    assert_eq!(a["unsicher"], json!(true), "{a}");
    // Ein Vorschlag ohne Feld steht nicht unter `abgelehnt` und hat keinen Grund (Mutant H56).
    assert_eq!(a["abgelehnt_gruende"].as_object().unwrap().len(), 1, "{a}");
    let k = a["konflikte"].as_array().unwrap();
    assert_eq!(k.len(), 2, "{a}");
    assert_eq!(
        (
            k[0]["feld_id"].as_str(),
            k[0]["aktueller_wert"].clone(),
            k[0]["vorschlag_wert"].clone(),
            k[0]["begruendung"].as_str(),
            k[0]["beleg"].as_str()
        ),
        (
            Some("veranlagung"),
            json!("zusammen"),
            json!("einzel"),
            Some("neu"),
            Some("gemeinsam mit meiner Frau")
        )
    );
    assert_eq!(k[0]["aktuelles_event_id"], ev[0]["event_id"], "{a}");
    // `gross`: `veranlagung` steuert selbst andere Regeln (Python: `_ist_struktureller_konflikt` ist wahr),
    // `realsplitting_unterhaltsleistungen` nicht (falsch). Der Rechenweg geht unveraendert durch (H37, H39).
    assert_eq!(
        (k[0]["gross"].clone(), k[0]["rechenweg"].clone()),
        (json!(true), rechenweg()),
        "{a}"
    );
    assert_eq!(
        (
            k[1]["feld_id"].as_str(),
            k[1]["aktueller_wert"].clone(),
            k[1]["vorschlag_wert"].clone(),
            k[1]["gross"].clone(),
            k[1]["rechenweg"].clone()
        ),
        (
            Some("realsplitting_unterhaltsleistungen"),
            json!(600_000),
            json!(700_000),
            json!(false),
            Value::Null
        ),
        "{a}"
    );
    assert_eq!(
        (k[0]["typ"].as_str(), k[0]["frage_invertiert"].clone()),
        (Some("enum"), json!(false)),
        "{a}"
    );
    assert_eq!(
        akte(&d, "ch1")["events"].as_array().unwrap().len(),
        3,
        "{a}"
    );
    gesehen.lock().unwrap().clear();

    // --- Rueckfragen: hoechstens eine je Aussage, die uebrigen zaehlen als zurueckgestellt.
    let frage = |text: &str, feld: &str, aussage: i64| json!({"frage": text, "feld_id": feld, "aussage": aussage});
    *skript.lock().unwrap() = vec![
        aussagen_stufe(),
        modell("zuordnung", &json!({"zuordnungen": []})),
        modell(
            "dialog",
            &json!({"vorschlaege": [], "rueckfragen": [
                frage("Wie hoch war dein Bruttoarbeitslohn?", "bruttoarbeitslohn", 1),
                frage("Und der Lohn deiner Frau?", "", 1),
                frage("Wie lange seid ihr verheiratet?", "", 0)],
                "antwort": "", "unsicher": false}),
        ),
    ];
    let (s, a) = post(&d, "/fall/ch1/chat", &json!({"text": TEXT})).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(
        a["rueckfragen"],
        json!([
            frage(
                "Wie hoch war dein Bruttoarbeitslohn?",
                "bruttoarbeitslohn",
                1
            ),
            frage("Wie lange seid ihr verheiratet?", "", 0)
        ]),
        "{a}"
    );
    assert_eq!(a["rueckfragen_zurueckgestellt"], json!(1), "{a}");
    gesehen.lock().unwrap().clear();

    // --- Die Zuordnung der zweiten Stufe verengt den Katalog der dritten; ein Instanzfeld bringt sein
    // Zaehlfeld mit, ein Feld einer anderen Regel bleibt draussen (Mutant H31: `instanz_gruppe` fehlt).
    *skript.lock().unwrap() = vec![
        aussagen_stufe(),
        modell(
            "zuordnung",
            &json!({"zuordnungen": [{"aussage": 0, "regeln": ["p10_1_3_kv_pv_kind"]}]}),
        ),
        modell(
            "dialog",
            &json!({"vorschlaege": [], "rueckfragen": [], "antwort": "", "unsicher": false}),
        ),
    ];
    let (s, a) = post(&d, "/fall/ch1/chat", &json!({"text": TEXT})).await;
    assert_eq!(s, 200, "{a}");
    let system = nachricht(&dialog_anfrage(&gesehen), "system");
    assert!(system.contains("- kind_kv:"), "{system}");
    assert!(system.contains("- fam_anzahl_kinder:"), "{system}");
    assert!(!system.contains("- veranlagung:"), "{system}");
    gesehen.lock().unwrap().clear();

    // --- Der Kontext an das Modell: das offene Feld mit Kurzhilfe und Gesetzestext, dann die
    // bestaetigten Angaben als Klartext; besondere Kategorien nur gezaehlt.
    setze_bestaetigt(
        &d,
        "ch1",
        &[
            ("bruttoarbeitslohn", json!(4_000_000)),
            ("kein_gewinn", json!(true)),
            ("kein_kap", json!(false)),
            ("steuerklasse", json!("3")),
            ("versorgung_beginn_jahr", json!(2020)),
            ("berufsausbildung_aufwendungen", json!(123_456)),
            ("ep_entfernung_km", json!(30)),
            ("agb_aufwendungen", json!(5)),
            ("kist_konfession", json!("evangelisch")),
        ],
    );
    *skript.lock().unwrap() = drei_stufen();
    let (s, a) = post(
        &d,
        "/fall/ch1/chat",
        &json!({"text": TEXT, "feld_id": "bruttoarbeitslohn"}),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    let system = nachricht(&dialog_anfrage(&gesehen), "system");
    let kopf = "Die Frage, um die es geht: „Wie hoch war dein Bruttoarbeitslohn dieses Jahr? (Nummer 3 der Lohnsteuerbescheinigung)“\n\
Dazu gehört laut Feldbeschreibung: Der Bruttoarbeitslohn einschließlich Sachbezüge steht unter Nummer 3 der Lohnsteuerbescheinigung. Hast du Versorgungsbezüge (Nummer 8), sind sie in diesem Betrag schon enthalten: Trag Nummer 3 unverändert ein.\n\
Wörtlicher Gesetzestext dazu — § 19 Abs. 1 S. 1 Nr. 1 EStG: „Gehälter, Löhne, Gratifikationen, Tantiemen und andere Bezüge und Vorteile für eine Beschäftigung“\n\
Das hat der Nutzer bereits bestätigt:\n";
    let von = system
        .find("Die Frage, um die es geht")
        .unwrap_or_else(|| panic!("kein Kontext: {system}"));
    let kontext = &system[von..];
    assert!(kontext.starts_with(kopf), "{kontext}");
    let block = kontext.split("\n\n").next().unwrap();
    let zeilen: Vec<&str> = block[kopf.len()..].lines().collect();
    let soll = [
        " → ja",
        " → 40000,00 EUR",
        " → nein",
        " → ja",
        " → III — verheiratet, Partner in Klasse V",
        " → 2020 Jahr",
        " → 1234,56 EUR",
        " → 30 km",
    ];
    assert_eq!(zeilen.len(), soll.len() + 1, "{kontext}");
    for (z, ende) in zeilen.iter().zip(soll) {
        assert!(z.starts_with("- ") && z.ends_with(ende), "{z}");
    }
    // Die Bindung fuehrt den Fragetext von `kap_antrag_guenstigerpruefung` ausdruecklich leer: `None`
    // wie `str(None)` in Python (Mutant H23).
    assert_eq!(zeilen[0], "- None → ja", "{kontext}");
    assert!(
        zeilen[soll.len()]
            .starts_with("(2 weitere Angaben liegen vor, dürfen dir aber nicht übermittelt werden"),
        "{kontext}"
    );
    assert!(!block.contains("evangelisch"), "{block}");
    // Ohne Feld im Rumpf steht die Frage-Zeile nicht da, die bestaetigten Angaben schon.
    gesehen.lock().unwrap().clear();
    *skript.lock().unwrap() = drei_stufen();
    let (s, a) = post(&d, "/fall/ch1/chat", &json!({"text": TEXT, "feld_id": ""})).await;
    assert_eq!(s, 200, "{a}");
    let system = nachricht(&dialog_anfrage(&gesehen), "system");
    assert!(!system.contains("Die Frage, um die es geht"), "{system}");
    assert!(
        system.contains("Das hat der Nutzer bereits bestätigt:\n- "),
        "{system}"
    );
    // Ein Feld, das es nicht gibt, ist keine Frage.
    gesehen.lock().unwrap().clear();
    *skript.lock().unwrap() = drei_stufen();
    let (s, a) = post(
        &d,
        "/fall/ch1/chat",
        &json!({"text": TEXT, "feld_id": "gibt_es_nicht"}),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    assert!(!nachricht(&dialog_anfrage(&gesehen), "system").contains("Die Frage, um die es geht"));
    gesehen.lock().unwrap().clear();

    // --- Ein Feld, das kein Text ist, und ein Rumpf ohne Objekt: Pythons Ausnahmen, 500, ohne den
    // Dienst zu fragen.
    let (s, a) = post(
        &d,
        "/fall/ch1/chat",
        &json!({"text": TEXT, "feld_id": ["x"]}),
    )
    .await;
    assert_eq!(
        (s, a["fehler"].as_str()),
        (
            500,
            Some("TypeError: cannot use 'list' as a dict key (unhashable type: 'list')")
        ),
        "{a}"
    );
    let (s, a) = post(
        &d,
        "/fall/ch1/chat",
        &json!({"text": TEXT, "feld_id": {"a": 1}}),
    )
    .await;
    assert_eq!(
        (s, a["fehler"].as_str()),
        (
            500,
            Some("TypeError: cannot use 'dict' as a dict key (unhashable type: 'dict')")
        ),
        "{a}"
    );
    let (s, a) = post(&d, "/fall/ch1/chat", &json!({"text": 5})).await;
    assert_eq!(
        (s, a["fehler"].as_str()),
        (
            500,
            Some("AttributeError: 'int' object has no attribute 'strip'")
        ),
        "{a}"
    );
    let (s, a) = post(&d, "/fall/ch1/chat", &json!([1])).await;
    assert_eq!(
        (s, a["fehler"].as_str()),
        (
            500,
            Some("AttributeError: 'list' object has no attribute 'get'")
        ),
        "{a}"
    );
    assert!(
        gesehen.lock().unwrap().is_empty(),
        "der Dienst wurde gefragt"
    );

    // --- Leerer Text und leere Liste oder Objekt als Feld: 200 mit leerer Antwort, ohne den Dienst zu fragen.
    for rumpf in [
        json!({"text": "   "}),
        json!({"text": null}),
        json!({"text": 0}),
        json!({}),
    ] {
        let (s, a) = post(&d, "/fall/ch1/chat", &rumpf).await;
        assert_eq!(s, 200, "{rumpf}: {a}");
        assert_eq!(
            (
                a["vorschlaege"].clone(),
                a["aussagen"].clone(),
                a["antwort"].clone()
            ),
            (json!([]), json!([]), json!("")),
            "{rumpf}"
        );
    }
    let (s, a) = post(&d, "/fall/ch1/chat", &json!({"text": "  ", "feld_id": []})).await;
    assert_eq!(s, 200, "{a}");
    assert!(
        gesehen.lock().unwrap().is_empty(),
        "der Dienst wurde gefragt"
    );

    // --- Stufe 3 liefert einen Wert, den JSON nicht kennt (`NaN`, `1e400`, `Infinity`), oder ein einzelnes
    // Surrogat (V4 Stapel 1e, P1a/P1c): Rust wertet die GANZE Antwort als unlesbar (fail-closed), auch den
    // gueltigen Vorschlag daneben. Python liest `NaN` als Zahl und schickt ungueltiges JSON weiter
    // (`rust/fixtures/README.md`, Nr. 12). Verlangt: 200 mit gueltigem JSON, kein Vorschlag, beide Aussagen
    // `kein_feld`, die Akte bleibt unberuehrt. Die Kontrolle schreibt dieselbe Gestalt mit gueltigen Werten.
    let gueltig = r#"{"feld_id": "veranlagung", "wert": "zusammen", "beleg": "gemeinsam mit meiner Frau", "begruendung": "b0", "aussage": 0, "rechenweg": null}"#;
    let zweiter = |wert: &str, begruendung: &str, rechenweg: &str| {
        format!(
            r#"{{"feld_id": "realsplitting_unterhaltsleistungen", "wert": {wert}, "beleg": "6000 Euro Unterhalt", "begruendung": "{begruendung}", "aussage": 1, "rechenweg": {rechenweg}}}"#
        )
    };
    let dialog = |zweiter: &str| {
        format!(
            r#"{{"vorschlaege": [{gueltig}, {zweiter}], "rueckfragen": [], "antwort": "ok", "unsicher": false}}"#
        )
    };
    let stufen_mit = |dialog_text: &str| {
        vec![
            aussagen_stufe(),
            modell(
                "zuordnung",
                &json!({"zuordnungen": [
                    {"aussage": 0, "regeln": ["p2_festzusetzung_einzel"]},
                    {"aussage": 1, "regeln": ["p10_1a_realsplitting"]}]}),
            ),
            modell_roh("dialog", dialog_text),
        ]
    };
    let status = |a: &Value| -> Vec<String> {
        a["aussagen"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["status"].as_str().unwrap().to_owned())
            .collect()
    };
    for fall in ["nan0", "nan1"] {
        let (s, a) = post(
            &d,
            "/fall",
            &json!({"fall_id": fall, "scheibe": "gesamt", "veranlagungszeitraum": 2025}),
        )
        .await;
        assert_eq!(s, 201, "{a}");
    }
    *skript.lock().unwrap() = stufen_mit(&dialog(&zweiter("600000", "b1", "null")));
    let (s, a) = post(&d, "/fall/nan0/chat", &json!({"text": TEXT})).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(
        a["vorschlaege"].as_array().unwrap().len(),
        2,
        "KONTROLLE: {a}"
    );
    assert_eq!(status(&a), ["vorschlag", "vorschlag"], "KONTROLLE: {a}");
    assert_eq!(akte(&d, "nan0")["events"].as_array().unwrap().len(), 2);
    let vorher = akte(&d, "nan1");
    for (name, zweiter_text) in [
        ("wert NaN", zweiter("NaN", "b1", "null")),
        ("wert 1e400", zweiter("1e400", "b1", "null")),
        (
            "rechenweg basis Infinity",
            zweiter(
                "600000",
                "b1",
                r#"{"basis": Infinity, "faktor": 1, "erklaerung": "e"}"#,
            ),
        ),
        ("begruendung Surrogat", zweiter("600000", r"\ud800", "null")),
    ] {
        *skript.lock().unwrap() = stufen_mit(&dialog(&zweiter_text));
        let (s, a) = post(&d, "/fall/nan1/chat", &json!({"text": TEXT})).await;
        assert_eq!(s, 200, "{name}: {a}");
        assert_eq!(a["vorschlaege"], json!([]), "{name}: {a}");
        assert_eq!(status(&a), ["kein_feld", "kein_feld"], "{name}: {a}");
        assert_eq!(akte(&d, "nan1"), vorher, "{name}: die Akte wurde beruehrt");
        assert!(!a.to_string().contains("NaN"), "{name}: {a}");
    }
    // Stufe 3 faellt aus (Anbieter 500, zweimal): kein Vorschlag, beide Aussagen `werte_ausgefallen`, die Akte bleibt.
    *skript.lock().unwrap() = vec![
        aussagen_stufe(),
        modell(
            "zuordnung",
            &json!({"zuordnungen": [{"aussage": 0, "regeln": ["p2_festzusetzung_einzel"]}]}),
        ),
        Schritt {
            stufe: "dialog",
            status: 500,
            body: r#"{"error": "x"}"#.to_owned(),
            oft: true,
        },
    ];
    let (s, a) = post(&d, "/fall/nan1/chat", &json!({"text": TEXT})).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["vorschlaege"], json!([]), "{a}");
    assert_eq!(
        status(&a),
        ["werte_ausgefallen", "werte_ausgefallen"],
        "{a}"
    );
    assert_eq!(akte(&d, "nan1"), vorher, "die Akte wurde beruehrt");
    skript.lock().unwrap().clear();
    gesehen.lock().unwrap().clear();

    // --- Stufe 1 faellt aus: 501 mit dem Vertrag, die Akte bleibt, das Fehlerprotokoll warnt.
    let vorher = akte(&d, "ch1");
    *skript.lock().unwrap() = vec![Schritt {
        stufe: "aussagen",
        status: 403,
        body: r#"{"error": "x"}"#.to_owned(),
        oft: true,
    }];
    let (s, a) = post(&d, "/fall/ch1/chat", &json!({"text": TEXT})).await;
    assert_eq!(
        (s, a["fehler"].as_str()),
        (501, Some("not_implemented")),
        "{a}"
    );
    assert!(
        a["vertrag"].as_str().unwrap().contains("schreiber='llm:…'"),
        "{a}"
    );
    assert_eq!(akte(&d, "ch1"), vorher);
    let fehlerlog = std::fs::read_to_string(d.zustand.konfig.fehler_pfad()).unwrap();
    let eintraege: Vec<Value> = fehlerlog
        .lines()
        .map(|z| serde_json::from_str::<Value>(z).unwrap())
        .filter(|e| e["ort"] == json!("api.chat llm"))
        .collect();
    assert_eq!(eintraege.len(), 1, "{fehlerlog}");
    assert_eq!(eintraege[0]["stufe"], json!("WARNING"), "{fehlerlog}");
    assert_eq!(
        eintraege[0]["fall_id"],
        json!("<gesperrt:pii_filter_fehlt>"),
        "{fehlerlog}"
    );
    assert!(
        !fehlerlog.contains("SYNTHETISCH") && !fehlerlog.contains("Unterhalt"),
        "{fehlerlog}"
    );
    skript.lock().unwrap().clear();

    // --- Stufe 1 antwortet abgeschnitten (zweimal): 501, und das Protokoll nennt den Anbieter, der
    // geantwortet hat, obwohl der Aufruf scheiterte.
    *skript.lock().unwrap() = vec![Schritt {
        stufe: "aussagen",
        status: 200,
        body: json!({
            "provider": "StubAnbieter",
            "choices": [{"finish_reason": "length", "message": {"content": "{"}}]
        })
        .to_string(),
        oft: true,
    }];
    let (s, a) = post(&d, "/fall/ch1/chat", &json!({"text": TEXT})).await;
    assert_eq!(
        (s, a["fehler"].as_str()),
        (501, Some("not_implemented")),
        "{a}"
    );
    let zeile = letzte_llm_zeile(&d);
    assert!(
        zeile.contains("stufe=1, ergebnis=kein_ergebnis, grund=abgeschnitten, versuche=2, provider='StubAnbieter'"),
        "{zeile}"
    );
    skript.lock().unwrap().clear();

    // --- Fehlende Umgebung: 501 ohne Anfrage an den Dienst; der Anbieter des vorigen Aufrufs steht
    // nicht im Protokoll (Mutant H06: der Thread haelt `StubAnbieter` vom Aufruf davor).
    gesehen.lock().unwrap().clear();
    std::env::remove_var("LLM_API_KEY");
    let (s, a) = post(&d, "/fall/ch1/chat", &json!({"text": TEXT})).await;
    assert_eq!(
        (s, a["fehler"].as_str()),
        (501, Some("not_implemented")),
        "{a}"
    );
    assert!(
        gesehen.lock().unwrap().is_empty(),
        "der Dienst wurde gefragt"
    );
    let zeile = letzte_llm_zeile(&d);
    assert!(
        zeile.contains(
            "stufe=1, ergebnis=kein_ergebnis, grund=sonstiger_fehler, versuche=1, provider=''"
        ),
        "{zeile}"
    );
    std::env::set_var("LLM_API_KEY", "SYNTHETISCH-LLM-SCHLUESSEL");

    // --- Mitschnitt (`TAXGRAPH_FLOW=1`): der Nutzertext mit dem Feld, die drei Stufen der KI.
    std::env::set_var("TAXGRAPH_FLOW", "1");
    *skript.lock().unwrap() = drei_stufen();
    // Der Text steht gekuerzt (`strip`) im Mitschnitt, auch wenn der Client Leerraum davor und danach schickt.
    let (s, a) = post(
        &d,
        "/fall/ch1/chat",
        &json!({"text": format!(" \t\u{a0}{TEXT}\n  "), "feld_id": "bruttoarbeitslohn"}),
    )
    .await;
    std::env::remove_var("TAXGRAPH_FLOW");
    assert_eq!(s, 200, "{a}");
    let fluss = std::fs::read_to_string(d.zustand.konfig.audit_dir.join("flow.jsonl")).unwrap();
    let zeilen: Vec<Value> = fluss
        .lines()
        .map(|z| serde_json::from_str(z).unwrap())
        .collect();
    let arten: Vec<(&str, &str)> = zeilen
        .iter()
        .map(|z| (z["art"].as_str().unwrap(), z["fall"].as_str().unwrap()))
        .collect();
    assert_eq!(
        arten,
        [
            ("nutzertext", "ch1"),
            ("ki", "ch1"),
            ("ki", "ch1"),
            ("ki", "ch1")
        ],
        "{fluss}"
    );
    assert_eq!(
        zeilen[0]["inhalt"],
        json!({"text": TEXT, "bei_feld": "bruttoarbeitslohn"})
    );
    assert_eq!(
        zeilen[1..]
            .iter()
            .map(|z| (z["inhalt"]["stufe"].clone(), z["inhalt"]["was"].clone()))
            .collect::<Vec<_>>(),
        [
            (json!(1), json!("aussagen")),
            (json!(2), json!("zuordnungen")),
            (json!(3), json!("ergebnis"))
        ]
    );

    // --- Das Protokoll nennt je Stufe die Metadaten und den Nutzer, nie den Text.
    let audit = std::fs::read_to_string(d.zustand.konfig.audit_pfad()).unwrap();
    let llm_zeilen: Vec<Value> = audit
        .lines()
        .map(|z| serde_json::from_str::<Value>(z).unwrap())
        .filter(|z| z["action"] == "llm_call")
        .collect();
    assert!(llm_zeilen.len() >= 3, "{audit}");
    assert!(
        llm_zeilen.iter().all(|z| z["user_id"] == json!("alice")),
        "{audit}"
    );
    assert!(!audit.contains("Unterhalt"), "{audit}");
    let letzte: Vec<&str> = llm_zeilen[llm_zeilen.len() - 3..]
        .iter()
        .map(|z| z["detail"].as_str().unwrap())
        .collect();
    assert!(
        letzte[0].contains("stufe=1, aussagen=2, aussagen_ohne_beleg=0"),
        "{letzte:?}"
    );
    assert!(
        letzte[2].contains("stufe=3, katalog=voll, katalog_felder="),
        "{letzte:?}"
    );
    assert!(
        letzte[2].contains(", vorschlaege=5, ohne_beleg_verworfen=0, rueckfragen=0,"),
        "{letzte:?}"
    );
    assert!(
        letzte
            .iter()
            .all(|z| z.contains("provider='StubAnbieter', finish='stop'")),
        "{letzte:?}"
    );

    // --- Kontoauszug (Auftrag k9-2, Mutant K46): eine Buchung ohne Stichwort geht an das Modell. Der Aufruf
    // hat kein Antwortschema, der Dienst führt ihn unter dem Namen `?`. Das Modell nennt `spende`; die Buchung
    // wird übernommen, als Vorschlag des Modells (`quelle: llm`), und das Modell sah nur den Zweck und den Betrag.
    gesehen.lock().unwrap().clear();
    *skript.lock().unwrap() = vec![Schritt {
        stufe: "?",
        status: 200,
        body: json!({
            "provider": "StubAnbieter",
            "choices": [{"finish_reason": "stop", "message": {"content": json!({"kategorie": "spende"}).to_string()}}]
        })
        .to_string(),
        oft: true,
    }];
    let (s, a) = post(
        &d,
        "/fall",
        &json!({"fall_id": "ko1", "scheibe": "gesamt", "veranlagungszeitraum": 2025}),
    )
    .await;
    assert_eq!(s, 201, "{a}");
    let (s, a) = post(
        &d,
        "/fall/ko1/kontoauszug",
        &json!({"format": "csv", "inhalt": "datum;betrag;verwendungszweck\n01.03.2025;-50,00;Einkauf Wochenmarkt\n"}),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(
        a,
        json!({"uebernommen": 1, "transaktionen": 1, "verworfen": 0}),
        "{a}"
    );
    let ereignisse = akte(&d, "ko1")["events"].as_array().unwrap().clone();
    assert_eq!(ereignisse.len(), 1, "{ereignisse:?}");
    assert_eq!(ereignisse[0]["feld_id"], json!("spenden_betrag"));
    assert_eq!(ereignisse[0]["wert"], json!(5000));
    assert_eq!(ereignisse[0]["zustand"], json!("vorlaeufig"));
    assert_eq!(
        ereignisse[0]["signal"]["signal_1"]["quelle"],
        json!("llm"),
        "{ereignisse:?}"
    );
    assert_eq!(
        ereignisse[0]["signal"]["signal_1"]["kategorie"],
        json!("spende")
    );
    let anfragen = gesehen.lock().unwrap().clone();
    assert_eq!(anfragen.len(), 1, "{anfragen:?}");
    assert_eq!(anfragen[0].0, "?");
    assert_eq!(
        nachricht(&anfragen[0].1, "user"),
        "Zweck: Einkauf Wochenmarkt\nBetrag: -50.00 EUR"
    );
}
