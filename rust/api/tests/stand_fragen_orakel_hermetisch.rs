//! `GET /fall/{id}/stand`, `GET /fall/{id}/fragen` und `GET /fall/{id}/feld/{fid}/frage` gegen einen
//! Golden-Master, ohne `PARITY=1`, ohne Python, ohne Netz.
//!
//! Auftrag 8 (Mutationsmessung der duennen `api`-Dateien): fuer `stand.rs` und `fragen.rs` hielt nur
//! der Vergleich mit Python die Gestalt der Antwort (Engine-Name, Spanne, Sperrgrund, Teil-Ringe,
//! Metadaten jeder Frage). Die Antworten stehen fest in `rust/fixtures/api_stand_fragen_orakel.json`.
//!
//! GOLDEN-MASTER (Stufe 2, S2.2): Bis `2dd056a6` waren das eingefrorene Antworten des Python-Servers
//! (Verlauf: `git show 2dd056a6:rust/fixtures/api_stand_fragen_orakel.json`). Python liefert fuer ein neues
//! Bindungsfeld keine Antwort mehr, und das erste echte Feld in einer Scheibe aendert Queue und
//! `offen`. Darum gehoert die Datei jetzt Rust. Der Inhalt zum Zeitpunkt der Umstellung war gleich dem
//! der Python-Datei (Beleg im Commit); der Test vergleicht weiter jede Antwort Feld fuer Feld.
//!
//! Neu schreiben: `TAXGRAPH_GOLDEN_NEU=1 cargo test -p api --test stand_fragen_orakel_hermetisch`, danach
//! `git diff rust/fixtures/api_stand_fragen_orakel.json` LESEN. Das Neuschreiben nimmt jede heutige Antwort
//! als richtig; der Diff ist die einzige Pruefung. In der CI (`CI` gesetzt) ist es verboten. Ein neuer
//! Fall braucht in der Datei `name`, `scheibe`, `events` und eine leere Huelle: `"fragen": {}` (ganze
//! Antwort) oder `"fragen_ids": []` (nur Queue und Ring), dazu `"einzeln": {"<feld_id>": {}}` fuer
//! Einzelfragen. Die Zahl [`N_FAELLE`] zieht der Autor nach: sie haelt einen gestrichenen Fall auf.
//!
//! Die Ereignisse spielt der Test ueber die echte Route `POST /event` ein. Nur `event_id` jedes Felds
//! in `/stand` hat eine Uhrzeit im Hash: der Test prueft nur die Form (64 Hex-Zeichen), die Datei
//! fuehrt `<event_id>`.
//!
//! Der Mitschnitt (`TAXGRAPH_FLOW=1`) liest die Umgebung des Prozesses; darum steht alles in EINEM Test.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::path::{Path, PathBuf};

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Map, Value};
use tower::ServiceExt;

/// Zahl der Faelle im Golden-Master. Ein gestrichener Fall faellt im Vergleich nicht auf (die Datei
/// traegt die Eingaben selbst); diese Zahl haelt ihn auf.
const N_FAELLE: usize = 21;

/// Bis zu dieser Tiefe schreibt der Schreiber Objekte und Listen mit Unterstrukturen zeilenweise; darunter
/// steht ein Wert in einer Zeile. Wurzel 0, `faelle` 1, Fall 2, `stand`/`fragen`/`einzeln` 3,
/// `felder`/`fragen` 4, ein Feld oder eine Frage 5: ein Feld, eine Frage je Zeile, damit ein Diff lesbar
/// bleibt. `events` (3) klappt auf, seine Ereignisse (4) bleiben je eine Zeile.
const AUFKLAPPEN_BIS: usize = 4;

/// Kopf der Datei (JSON kennt keine Kommentare); das Neuschreiben setzt ihn jedes Mal.
const KOPF_GOLDEN_MASTER: &str = "Rust-eigener Golden-Master (Stufe 2, S2.2) fuer GET /stand, GET /fragen und GET /feld/{fid}/frage: je Fall Eingaben (name, scheibe, events) und die Antworten des Rust-Dienstes. Der Test stand_und_fragen_wie_python in rust/api/tests/stand_fragen_orakel_hermetisch.rs vergleicht jede Antwort Feld fuer Feld.";
const KOPF_NEU_SCHREIBEN: &str = "TAXGRAPH_GOLDEN_NEU=1 cargo test -p api --test stand_fragen_orakel_hermetisch; danach den Diff dieser Datei LESEN (das Neuschreiben nimmt jede Antwort als richtig; in der CI verboten). Neuer Fall: name, scheibe, events und eine leere Huelle (\"fragen\": {} oder \"fragen_ids\": [], dazu \"einzeln\": {\"<feld_id>\": {}}); dann N_FAELLE im Test anheben.";
const KOPF_PYTHON_STAND: &str = "Bis 2dd056a6 standen hier eingefrorene Antworten des Python-Servers: git show 2dd056a6:rust/fixtures/api_stand_fragen_orakel.json";
const KOPF_QUELLE: &str = "Rust (api::stand, api::fragen); Ursprung der Antworten: produkt/haut/api.py (stand, fragen, frage_einzeln)";

const REIHENFOLGE_WURZEL: [&str; 5] = [
    "golden_master",
    "neu_schreiben",
    "python_stand",
    "quelle",
    "faelle",
];
const REIHENFOLGE_FALL: [&str; 9] = [
    "name",
    "scheibe",
    "events",
    "stand",
    "fragen",
    "fragen_ids",
    "fragen_ring_gesperrt",
    "einzeln",
    "kopf",
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

fn datei() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/api_stand_fragen_orakel.json")
}

fn golden_master() -> Value {
    serde_json::from_slice(&std::fs::read(datei()).unwrap()).unwrap()
}

/// `event_id` hat eine Uhrzeit im Hash: Form pruefen, dann auf einen festen Wert setzen.
fn ohne_event_id(stand: &mut Value) {
    for (fid, feld) in stand["felder"].as_object_mut().unwrap() {
        let id = feld["event_id"]
            .as_str()
            .unwrap_or_else(|| panic!("{fid}: keine event_id"));
        assert!(
            id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()),
            "{fid}: {id}"
        );
        feld["event_id"] = json!("<event_id>");
    }
}

/// Spielt einen Fall durch die echten Routen und gibt zurueck, was der Golden-Master fuer ihn fuehrt.
/// Die Eingaben (`name`, `scheibe`, `events`) und die Huelle (ganze `/fragen`-Antwort oder nur Queue und
/// Ring, welche Felder einzeln) kommen aus dem Fall in der Datei, die Antworten aus Rust.
async fn antworten(f: &Value) -> Value {
    let name = f["name"].as_str().unwrap();
    let d = dienst();
    let kopf = json!({"fall_id": "sf", "scheibe": f["scheibe"], "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "{name}: POST /fall: {antwort}");
    for e in f["events"].as_array().unwrap() {
        let (status, antwort) = sende(&d, "POST", "/fall/sf/event", Some(e)).await;
        assert_eq!(
            status, 201,
            "{name}: POST /event {}: {antwort}",
            e["feld_id"]
        );
    }
    let mut aus = json!({"name": f["name"], "scheibe": f["scheibe"], "events": f["events"]});

    // --- /stand
    let (status, mut stand) = sende(&d, "GET", "/fall/sf/stand", None).await;
    assert_eq!(status, 200, "{name}: GET /stand: {stand}");
    ohne_event_id(&mut stand);
    aus["stand"] = stand;

    // --- /fragen
    let (status, fragen) = sende(&d, "GET", "/fall/sf/fragen", None).await;
    assert_eq!(status, 200, "{name}: GET /fragen: {fragen}");
    assert_eq!(fragen["fall_id"], json!("sf"), "{name}");
    if f.get("fragen").is_some() {
        aus["fragen"] = fragen;
    } else {
        aus["fragen_ids"] = Value::Array(
            fragen["fragen"]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| q["feld_id"].clone())
                .collect(),
        );
        aus["fragen_ring_gesperrt"] = json!([
            fragen["ring_gesperrt"],
            fragen["ring_gesperrt_klartext"],
            fragen["snapshot_id"]
        ]);
    }

    // --- /feld/{fid}/frage
    if let Some(einzeln) = f.get("einzeln") {
        let mut ist = Map::new();
        for fid in einzeln.as_object().unwrap().keys() {
            let (status, body) =
                sende(&d, "GET", &format!("/fall/sf/feld/{fid}/frage"), None).await;
            ist.insert(fid.clone(), json!({"status": status, "body": body}));
        }
        aus["einzeln"] = Value::Object(ist);
    }

    // --- Mitschnitt von /fragen: Anzahl und Kopf der Queue.
    std::env::set_var("TAXGRAPH_FLOW", "1");
    let (status, _) = sende(&d, "GET", "/fall/sf/fragen", None).await;
    std::env::remove_var("TAXGRAPH_FLOW");
    assert_eq!(status, 200, "{name}: GET /fragen mit Mitschnitt");
    let fluss = std::fs::read_to_string(d.zustand.konfig.audit_dir.join("flow.jsonl")).unwrap();
    let zeilen: Vec<Value> = fluss
        .lines()
        .map(|z| serde_json::from_str(z).unwrap())
        .collect();
    assert_eq!(zeilen.len(), 1, "{name}: {fluss}");
    assert_eq!(
        (zeilen[0]["art"].as_str(), zeilen[0]["fall"].as_str()),
        (Some("fragen"), Some("sf")),
        "{name}"
    );
    aus["kopf"] = zeilen[0]["inhalt"].clone();
    aus
}

/// Jede Stelle, an der `ist` und `soll` verschieden sind, als `Pfad: ist …, soll …`.
fn unterschiede(pfad: &str, ist: &Value, soll: &Value, aus: &mut Vec<String>) {
    fn kurz(v: &Value) -> String {
        let t = v.to_string();
        if t.chars().count() > 160 {
            format!("{}…", t.chars().take(160).collect::<String>())
        } else {
            t
        }
    }
    match (ist, soll) {
        (Value::Object(a), Value::Object(b)) => {
            let mut schluessel: Vec<&String> = a.keys().chain(b.keys()).collect();
            schluessel.sort();
            schluessel.dedup();
            for k in schluessel {
                let unter = format!("{pfad}.{k}");
                match (a.get(k), b.get(k)) {
                    (Some(x), Some(y)) => unterschiede(&unter, x, y, aus),
                    (Some(x), None) => aus.push(format!("{unter}: nur ist {}", kurz(x))),
                    (None, Some(y)) => aus.push(format!("{unter}: nur soll {}", kurz(y))),
                    (None, None) => {}
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                aus.push(format!("{pfad}: Laenge ist {}, soll {}", a.len(), b.len()));
            }
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                unterschiede(&format!("{pfad}[{i}]"), x, y, aus);
            }
        }
        _ => {
            if ist != soll {
                aus.push(format!("{pfad}: ist {}, soll {}", kurz(ist), kurz(soll)));
            }
        }
    }
}

/// Schluessel eines Objekts in der Reihenfolge der Datei: die Vorrangliste zuerst, der Rest sortiert.
fn reihenfolge<'a>(vorrang: &[&str], m: &'a Map<String, Value>) -> Vec<&'a String> {
    let mut vorn: Vec<&String> = vorrang
        .iter()
        .filter_map(|k| m.get_key_value(*k))
        .map(|(k, _)| k)
        .collect();
    let mut rest: Vec<&String> = m
        .keys()
        .filter(|k| !vorrang.contains(&k.as_str()))
        .collect();
    rest.sort();
    vorn.append(&mut rest);
    vorn
}

/// Schreibt `v` so, dass ein Diff lesbar bleibt (siehe [`AUFKLAPPEN_BIS`]); gleiche Eingabe, gleiche Bytes.
fn schreibe(v: &Value, tiefe: usize, grenze: usize, aus: &mut String) {
    let aufklappen = tiefe <= grenze
        && match v {
            Value::Object(m) => !m.is_empty(),
            Value::Array(a) => a.iter().any(|x| x.is_object() || x.is_array()),
            _ => false,
        };
    if !aufklappen {
        aus.push_str(&serde_json::to_string(v).unwrap());
        return;
    }
    let (innen, aussen) = (" ".repeat(tiefe + 1), " ".repeat(tiefe));
    match v {
        Value::Object(m) => {
            let vorrang: &[&str] = match tiefe {
                0 => &REIHENFOLGE_WURZEL,
                2 => &REIHENFOLGE_FALL,
                _ => &[],
            };
            aus.push_str("{\n");
            let schluessel = reihenfolge(vorrang, m);
            for (i, k) in schluessel.iter().enumerate() {
                aus.push_str(&innen);
                aus.push_str(&serde_json::to_string(k).unwrap());
                aus.push_str(": ");
                // Die Ereignisse eines Falls: eines je Zeile, nicht je Schluessel.
                let unten = if tiefe == 2 && k.as_str() == "events" {
                    tiefe + 1
                } else {
                    grenze
                };
                schreibe(&m[*k], tiefe + 1, unten, aus);
                aus.push_str(if i + 1 < schluessel.len() {
                    ",\n"
                } else {
                    "\n"
                });
            }
            aus.push_str(&aussen);
            aus.push('}');
        }
        Value::Array(a) => {
            aus.push_str("[\n");
            for (i, x) in a.iter().enumerate() {
                aus.push_str(&innen);
                schreibe(x, tiefe + 1, grenze, aus);
                aus.push_str(if i + 1 < a.len() { ",\n" } else { "\n" });
            }
            aus.push_str(&aussen);
            aus.push(']');
        }
        _ => unreachable!("nur Objekt und Liste klappen auf"),
    }
}

#[tokio::test]
async fn stand_und_fragen_wie_python() {
    std::env::remove_var("TAXGRAPH_FLOW");
    let soll = golden_master();
    let faelle = soll["faelle"].as_array().unwrap();
    assert_eq!(faelle.len(), N_FAELLE, "Zahl der Faelle im Golden-Master");
    let mut ist_faelle = Vec::new();
    for f in faelle {
        ist_faelle.push(antworten(f).await);
    }

    if std::env::var("TAXGRAPH_GOLDEN_NEU").is_ok_and(|v| v == "1") {
        assert!(
            std::env::var_os("CI").is_none(),
            "TAXGRAPH_GOLDEN_NEU=1 ist in der CI verboten: das Neuschreiben nimmt jede Antwort als richtig"
        );
        let mut neu = soll.clone();
        neu["golden_master"] = json!(KOPF_GOLDEN_MASTER);
        neu["neu_schreiben"] = json!(KOPF_NEU_SCHREIBEN);
        neu["python_stand"] = json!(KOPF_PYTHON_STAND);
        neu["quelle"] = json!(KOPF_QUELLE);
        neu["faelle"] = Value::Array(ist_faelle);
        let mut text = String::new();
        schreibe(&neu, 0, AUFKLAPPEN_BIS, &mut text);
        text.push('\n');
        std::fs::write(datei(), text).unwrap();
        eprintln!(
            "Golden-Master neu geschrieben ({N_FAELLE} Faelle): git diff rust/fixtures/api_stand_fragen_orakel.json lesen"
        );
        return;
    }

    let mut fehler = Vec::new();
    for (ist, soll) in ist_faelle.iter().zip(faelle) {
        let mut diff = Vec::new();
        unterschiede("", ist, soll, &mut diff);
        if !diff.is_empty() {
            let name = soll["name"].as_str().unwrap();
            let zeigen = diff
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n    ");
            fehler.push(format!("{name}: {} Abweichungen\n    {zeigen}", diff.len()));
        }
    }
    assert!(
        fehler.is_empty(),
        "Antworten weichen vom Golden-Master ab. Gewollt? Dann `TAXGRAPH_GOLDEN_NEU=1 cargo test -p api \
         --test stand_fragen_orakel_hermetisch` und den Diff der Datei lesen.\n{}",
        fehler.join("\n")
    );
}
