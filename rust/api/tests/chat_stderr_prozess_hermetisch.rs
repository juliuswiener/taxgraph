//! `POST /fall/{id}/chat` im ECHTEN Server-Prozess (`taxgraph-api`), um die Beobachtung auf stderr zu
//! prüfen, ohne Netz und ohne Python. `$LLM_API_BASE` zeigt auf einen lokalen Attrappen-Dienst.
//!
//! Auftrag 8 (Mutanten D020, D022, K021 in `chat.rs::beobachte`): die zwei Zeilen
//! `[haut.chat] LLM-Vorschläge …` gehen nur an stderr, und ein Test im selben Prozess kann stderr nicht
//! mitlesen. Der Prozess läuft darum für sich; der Test liest, was er auf stderr schrieb. Sicherheits-
//! Beobachtung (`api.py:1228`): nur Feld-IDs und eine Zahl, nie ein Wert und nie der Freitext.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};

use serde_json::{json, Value};

const TEXT: &str = "Ich habe 6000 Euro Unterhalt gezahlt und 100 Euro Lohn bekommen.";

/// Antwort des Attrappen-Dienstes je Stufe (Schemaname der Anfrage). `abweisungen`: der dritte Schritt
/// schlaegt Felder vor, die der Server ablehnt, oder genau ein gueltiges.
fn inhalt(stufe: &str, abweisungen: bool) -> Value {
    match stufe {
        "aussagen" => json!({"aussagen": [
            {"text": "Der Nutzer zahlte 6000 Euro Unterhalt", "beleg": "6000 Euro Unterhalt"}]}),
        "zuordnung" => json!({"zuordnungen": []}),
        _ if !abweisungen => json!({"vorschlaege": [
            {"feld_id": "realsplitting_unterhaltsleistungen", "wert": 600_000, "beleg": "6000 Euro Unterhalt",
             "begruendung": "x", "aussage": 0, "rechenweg": null}],
            "rueckfragen": [], "antwort": "", "unsicher": false}),
        _ => json!({"vorschlaege": [
            {"feld_id": "zzz_unbekannt", "wert": 1, "beleg": "100 Euro Lohn",
             "begruendung": "x", "aussage": 0, "rechenweg": null},
            {"feld_id": "gibt_es_nicht", "wert": 2, "beleg": "100 Euro Lohn",
             "begruendung": "y", "aussage": 0, "rechenweg": null},
            {"feld_id": "gibt_es_nicht", "wert": 3, "beleg": "100 Euro Lohn",
             "begruendung": "z", "aussage": 0, "rechenweg": null},
            {"feld_id": "", "wert": 4, "beleg": "100 Euro Lohn",
             "begruendung": "ohne Feld", "aussage": 0, "rechenweg": null},
            {"feld_id": "", "wert": 5, "beleg": "100 Euro Lohn",
             "begruendung": "noch eins ohne Feld", "aussage": 0, "rechenweg": null}],
            "rueckfragen": [], "antwort": "", "unsicher": false}),
    }
}

/// Liest eine HTTP-Nachricht bis zum Ende des Rumpfs (`Content-Length`); `(Kopf, Rumpf)`.
fn lies_nachricht(s: &mut TcpStream) -> (String, Vec<u8>) {
    let mut puffer = Vec::new();
    let mut b = [0u8; 8192];
    loop {
        let n = s.read(&mut b).unwrap();
        if n == 0 {
            break;
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
                return (text[..i].to_owned(), puffer[i + 4..i + 4 + laenge].to_vec());
            }
        }
    }
    (String::new(), Vec::new())
}

fn attrappen_dienst(abweisungen: bool) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let basis = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
    std::thread::spawn(move || {
        for s in l.incoming() {
            let mut s = s.unwrap();
            let (_, rumpf) = lies_nachricht(&mut s);
            let anfrage: Value = serde_json::from_slice(&rumpf).unwrap_or(Value::Null);
            let stufe = anfrage["response_format"]["json_schema"]["name"]
                .as_str()
                .unwrap_or("?");
            let body = json!({
                "provider": "StubAnbieter",
                "choices": [{"finish_reason": "stop", "message": {"content": inhalt(stufe, abweisungen).to_string()}}]
            })
            .to_string();
            let antwort = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(antwort.as_bytes());
        }
    });
    basis
}

/// Ein Request an den Server-Prozess; `(Status, Rumpf als JSON)`.
fn sende(port: u16, pfad: &str, rumpf: &Value) -> (u16, Value) {
    let text = rumpf.to_string();
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "POST {pfad} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
        text.len()
    )
    .unwrap();
    let (kopf, rumpf) = lies_nachricht(&mut s);
    let status = kopf
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse::<u16>()
        .unwrap();
    (
        status,
        serde_json::from_slice(&rumpf).unwrap_or(Value::Null),
    )
}

struct Prozess {
    kind: Child,
    port: u16,
    /// Haelt das Rohr offen, damit der Server beim Schreiben nicht auf ein geschlossenes Rohr trifft.
    _stdout: BufReader<std::process::ChildStdout>,
}

impl Drop for Prozess {
    fn drop(&mut self) {
        let _ = self.kind.kill();
        let _ = self.kind.wait();
    }
}

fn starte(llm_basis: &str, daten: &std::path::Path) -> Prozess {
    let mut kind = Command::new(env!("CARGO_BIN_EXE_taxgraph-api"))
        .arg("0")
        .env("TAXGRAPH_NO_AUTH", "1")
        .env("TAXGRAPH_DATEN", daten)
        .env("TAXGRAPH_AUDIT_DIR", daten)
        .env("TAXGRAPH_USER_STORE", daten.join("users.json"))
        .env("TAXGRAPH_FLOW", "0")
        .env("LLM_API_BASE", llm_basis)
        .env("LLM_MODEL", "stub/modell")
        .env("LLM_API_KEY", "SYNTHETISCH-LLM-SCHLUESSEL")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut aus = BufReader::new(kind.stdout.take().unwrap());
    let mut zeile = String::new();
    aus.read_line(&mut zeile).unwrap();
    let port = zeile
        .trim()
        .rsplit(':')
        .next()
        .and_then(|p| p.split_whitespace().next())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or_else(|| panic!("kein Port in {zeile:?}"));
    Prozess {
        kind,
        port,
        _stdout: aus,
    }
}

#[test]
fn beobachtung_nennt_feld_ids_und_zahlen_nie_den_text() {
    let tmp = tempfile::tempdir().unwrap();
    let basis = attrappen_dienst(true);
    let mut p = starte(&basis, tmp.path());
    let (s, a) = sende(
        p.port,
        "/fall",
        &json!({"fall_id": "pc1", "scheibe": "gesamt", "veranlagungszeitraum": 2025}),
    );
    assert_eq!(s, 201, "{a}");
    let (s, a) = sende(p.port, "/fall/pc1/chat", &json!({"text": TEXT}));
    assert_eq!(s, 200, "{a}");
    assert_eq!(
        a["abgelehnt"],
        json!(["zzz_unbekannt", "gibt_es_nicht", "gibt_es_nicht"]),
        "{a}"
    );
    // Alles, was der Prozess auf stderr schrieb, bis er endet.
    let _ = p.kind.kill();
    let mut err = String::new();
    p.kind
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    let zeilen: Vec<&str> = err
        .lines()
        .filter(|z| z.starts_with("[haut.chat]"))
        .collect();
    assert_eq!(
        zeilen,
        [
            "[haut.chat] LLM-Vorschläge außerhalb Katalog abgelehnt: ['gibt_es_nicht', 'zzz_unbekannt']",
            "[haut.chat] LLM-Vorschläge mit fehlender/leerer feld_id abgelehnt: 2",
        ],
        "{err}"
    );
    assert!(
        !err.contains("Unterhalt") && !err.contains("SYNTHETISCH"),
        "{err}"
    );
}

#[test]
fn ohne_abweisung_schreibt_der_prozess_keine_beobachtung() {
    // Kein Vorschlag, also nichts abzulehnen: keine der beiden Zeilen (Mutanten D020, D022, K021).
    let tmp = tempfile::tempdir().unwrap();
    let basis = attrappen_dienst(false);
    let mut p = starte(&basis, tmp.path());
    let (s, _) = sende(
        p.port,
        "/fall",
        &json!({"fall_id": "pc2", "scheibe": "gesamt", "veranlagungszeitraum": 2025}),
    );
    assert_eq!(s, 201);
    let (s, a) = sende(p.port, "/fall/pc2/chat", &json!({"text": TEXT}));
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["abgelehnt"], json!([]), "{a}");
    let _ = p.kind.kill();
    let mut err = String::new();
    p.kind
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    assert!(!err.contains("[haut.chat]"), "{err}");
}
