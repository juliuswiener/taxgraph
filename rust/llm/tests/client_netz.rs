//! Der Chat-Client am Draht gegen die Python-Referenz (`llm_client.py`: `complete`, `_ein_versuch`, `_inhalt`), am
//! Aufrufort von `llm::HttpChat` geprueft (N4, Mutationsmessung `rust/llm`, Teil `client.rs`). Jedes Szenario ist ein
//! Skript fuer einen lokalen Stub (Antworten, Schweigen, Abbruch, Troepfeln); die Erwartung ist der Lauf des
//! Python-Clients gegen DASSELBE Skript: Ergebnis, Fehlerklasse, Grund, Versuche, Text der Meldung, Zahl der Anfragen,
//! `letzter_anbieter`. Wo Rust den Text der Python-Ausnahme nicht kennt (`AttributeError`, `IndexError`,
//! `RemoteDisconnected`, `[Errno 111]`), gilt nur die Klasse. Drei Eingaben, in denen Rust ANDERS endet als Python, steht im Test
//! `weicht_von_python_ab_drei_eingaben_enden_in_rust_anders` (festgehaltenes Rust-Verhalten, kein Soll). Kein Test ruft
//! Python, kein Test geht ins Netz.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod stub;

use std::time::{Duration, Instant};

use llm::pii::filtere;
use llm::prompt::aussagen_prompt;
use llm::schema::DIALOG_SCHEMA;
use llm::{letzter_anbieter, Chat, Completion, HttpChat, Konfiguration, LlmFehler};
use serde_json::{json, Value};
use stub::{antwort, Aktion, Stub};

const SCHLUESSEL: &str = "sk-test-GEHEIM-123";

/// Was Python (und damit Rust) liefert.
enum Erw {
    /// Text, Anbieter, `finish_reason`
    Ok(&'static str, &'static str, &'static str),
    /// Text aus `n` mal `x`
    OkX(usize, &'static str, &'static str),
    /// Klasse (voruebergehend/abgeschnitten/endgueltig), Grund, Versuche, Meldung
    Err(&'static str, &'static str, u32, Msg),
}

/// Wie genau die Meldung geprueft wird.
enum Msg {
    /// nur Klasse, Grund und Versuche
    Egal,
    /// ganzer Text
    Genau(&'static str),
    /// Anfang des Textes (Rust haengt dem Typnamen der Python-Ausnahme einen eigenen Satz an)
    Beginnt(&'static str),
}

/// Die Antwort eines OpenAI-artigen Dienstes.
fn ok_body(inhalt: &str, finish: &str, provider: &str) -> String {
    json!({"choices": [{"message": {"content": inhalt}, "finish_reason": finish}], "provider": provider}).to_string()
}

/// Eine grosse Antwort (ueber 64 KiB), deren letzte `rest` Byte im Abstand `pause_ms` kommen.
fn tropfen_gross(rest: usize, pause_ms: u64) -> Aktion {
    let voll = antwort(200, &ok_body(&"x".repeat(70_000), "stop", "fake"));
    let (vorne, hinten) = voll.split_at(voll.len() - rest);
    Aktion::Tropfen {
        vorne: vorne.to_vec(),
        rest: hinten.to_vec(),
        pause: Duration::from_millis(pause_ms),
    }
}

fn chat(basis: String, (socket, frist, wdh): (u64, u64, u64)) -> HttpChat {
    let mut k = Konfiguration::neu(basis, "m".into(), SCHLUESSEL.into());
    k.socket = Duration::from_secs(socket);
    k.frist = Duration::from_secs(frist);
    k.frist_wiederholung = Duration::from_secs(wdh);
    k.backoff = [Duration::ZERO; 2];
    HttpChat { konfiguration: k }
}

fn klasse(e: &LlmFehler) -> &'static str {
    match e {
        LlmFehler::Voruebergehend { .. } => "voruebergehend",
        LlmFehler::Abgeschnitten { .. } => "abgeschnitten",
        LlmFehler::Endgueltig { .. } => "endgueltig",
    }
}

/// Name, Basis-URL (`{port}` = Port des Stubs; leer = `http://127.0.0.1:{port}/v1`), Skript, Schema, Zeiten, Zahl der Anfragen, Anbieter, Erwartung.
type Fall = (
    &'static str,
    &'static str,
    Option<Vec<Aktion>>,
    bool,
    (u64, u64, u64),
    usize,
    &'static str,
    Erw,
);

fn pruefe(faelle: Vec<Fall>) {
    let nachrichten = aussagen_prompt(&filtere("hallo").0);
    for (name, vorlage, skript, schema, zeiten, anfragen, anbieter, erw) in faelle {
        // `None`: kein Dienst — ein Port, an dem niemand lauscht.
        let stub = skript.map(|s| {
            if vorlage.contains("[::1]") {
                Stub::starte_v6(s)
            } else {
                Stub::starte(s)
            }
        });
        let basis = stub.as_ref().map_or_else(
            || {
                let tot = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
                format!("http://127.0.0.1:{}/v1", tot.local_addr().unwrap().port())
            },
            |s| {
                if vorlage.is_empty() {
                    s.basis("/v1")
                } else {
                    vorlage.replace("{port}", &s.port.to_string())
                }
            },
        );
        let c = chat(basis, zeiten);
        let r: Result<Completion, LlmFehler> =
            c.complete(&nachrichten, schema.then_some(&*DIALOG_SCHEMA));
        let gesehen = stub.as_ref().map_or(0, |s| s.anfragen().len());
        assert_eq!(gesehen, anfragen, "{name}: Zahl der Anfragen");
        assert_eq!(letzter_anbieter(), anbieter, "{name}: letzter Anbieter");
        match (&r, &erw) {
            (Ok(c), Erw::Ok(text, provider, finish)) => {
                assert_eq!(
                    (c.text.as_str(), c.provider.as_str(), c.finish.as_str()),
                    (*text, *provider, *finish),
                    "{name}"
                );
            }
            (Ok(c), Erw::OkX(n, provider, finish)) => {
                assert_eq!(
                    (
                        c.text.len(),
                        c.text.bytes().all(|b| b == b'x'),
                        c.provider.as_str(),
                        c.finish.as_str()
                    ),
                    (*n, true, *provider, *finish),
                    "{name}"
                );
            }
            (Err(e), Erw::Err(kl, grund, versuche, msg)) => {
                assert_eq!(
                    (klasse(e), e.grund(), e.versuche()),
                    (*kl, *grund, *versuche),
                    "{name}: {e}"
                );
                match msg {
                    Msg::Egal => {}
                    Msg::Genau(m) => assert_eq!(e.to_string(), *m, "{name}: Meldung"),
                    Msg::Beginnt(m) => assert!(e.to_string().starts_with(m), "{name}: Meldung {e}"),
                }
                assert!(
                    !e.to_string().contains(SCHLUESSEL),
                    "{name}: der Schluessel steht in der Meldung"
                );
            }
            _ => panic!("{name}: erwartet anderes Ergebnis als {r:?}"),
        }
    }
}

/// Statuscodes: 429/500/502/503/504 drei Versuche, alles andere (auch 300 und 501) endgueltig nach einem; Fehlerkoerper auf 300 Zeichen gekuerzt, Schluessel maskiert.
#[test]
#[rustfmt::skip]
fn statuscodes_voruebergehend_oder_endgueltig_wie_python() {
    pruefe(vec![
        ("status_429_voruebergehend", "", Some(vec![Aktion::Roh(antwort(429, "{\"error\": \"x\"}")), Aktion::Roh(antwort(429, "{\"error\": \"x\"}")), Aktion::Roh(antwort(429, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 429 {\"error\": \"x\"}"))),
        ("status_500_voruebergehend", "", Some(vec![Aktion::Roh(antwort(500, "{\"error\": \"x\"}")), Aktion::Roh(antwort(500, "{\"error\": \"x\"}")), Aktion::Roh(antwort(500, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 500 {\"error\": \"x\"}"))),
        ("status_502_voruebergehend", "", Some(vec![Aktion::Roh(antwort(502, "{\"error\": \"x\"}")), Aktion::Roh(antwort(502, "{\"error\": \"x\"}")), Aktion::Roh(antwort(502, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 502 {\"error\": \"x\"}"))),
        ("status_503_voruebergehend", "", Some(vec![Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(503, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 503 {\"error\": \"x\"}"))),
        ("status_504_voruebergehend", "", Some(vec![Aktion::Roh(antwort(504, "{\"error\": \"x\"}")), Aktion::Roh(antwort(504, "{\"error\": \"x\"}")), Aktion::Roh(antwort(504, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 504 {\"error\": \"x\"}"))),
        ("status_400_endgueltig", "", Some(vec![Aktion::Roh(antwort(400, "{\"error\": \"x\"}")), Aktion::Roh(antwort(400, "{\"error\": \"x\"}")), Aktion::Roh(antwort(400, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 400 {\"error\": \"x\"}"))),
        ("status_401_endgueltig", "", Some(vec![Aktion::Roh(antwort(401, "{\"error\": \"x\"}")), Aktion::Roh(antwort(401, "{\"error\": \"x\"}")), Aktion::Roh(antwort(401, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 401 {\"error\": \"x\"}"))),
        ("status_403_endgueltig", "", Some(vec![Aktion::Roh(antwort(403, "{\"error\": \"x\"}")), Aktion::Roh(antwort(403, "{\"error\": \"x\"}")), Aktion::Roh(antwort(403, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 403 {\"error\": \"x\"}"))),
        ("status_404_endgueltig", "", Some(vec![Aktion::Roh(antwort(404, "{\"error\": \"x\"}")), Aktion::Roh(antwort(404, "{\"error\": \"x\"}")), Aktion::Roh(antwort(404, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 404 {\"error\": \"x\"}"))),
        ("status_501_endgueltig", "", Some(vec![Aktion::Roh(antwort(501, "{\"error\": \"x\"}")), Aktion::Roh(antwort(501, "{\"error\": \"x\"}")), Aktion::Roh(antwort(501, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 501 {\"error\": \"x\"}"))),
        ("status_505_endgueltig", "", Some(vec![Aktion::Roh(antwort(505, "{\"error\": \"x\"}")), Aktion::Roh(antwort(505, "{\"error\": \"x\"}")), Aktion::Roh(antwort(505, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 505 {\"error\": \"x\"}"))),
        ("status_300_endgueltig", "", Some(vec![Aktion::Roh(antwort(300, "{\"error\": \"x\"}")), Aktion::Roh(antwort(300, "{\"error\": \"x\"}")), Aktion::Roh(antwort(300, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 300 {\"error\": \"x\"}"))),
        ("fehlerkoerper_299", "", Some(vec![Aktion::Roh(antwort(400, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 400 AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"))),
        ("fehlerkoerper_300", "", Some(vec![Aktion::Roh(antwort(400, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 400 AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"))),
        ("fehlerkoerper_301", "", Some(vec![Aktion::Roh(antwort(400, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 400 AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"))),
        ("fehlerkoerper_umlaute_301", "", Some(vec![Aktion::Roh(antwort(503, "äääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääää")), Aktion::Roh(antwort(503, "äääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääää")), Aktion::Roh(antwort(503, "äääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääää"))]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 503 ääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääääää"))),
        ("fehlerkoerper_mit_schluessel", "", Some(vec![Aktion::Roh(antwort(400, "invalid key sk-test-GEHEIM-123 given"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 400 invalid key <KEY> given"))),
        ("fehlerkoerper_mit_schluessel_503", "", Some(vec![Aktion::Roh(antwort(503, "invalid key sk-test-GEHEIM-123 given")), Aktion::Roh(antwort(503, "invalid key sk-test-GEHEIM-123 given")), Aktion::Roh(antwort(503, "invalid key sk-test-GEHEIM-123 given"))]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 503 invalid key <KEY> given"))),
    ]);
}

/// Drei Versuche, nie vier; Erfolg nach Wiederholung; 2xx ist Erfolg.
#[test]
#[rustfmt::skip]
fn wiederholungen_und_versuchszahl_wie_python() {
    pruefe(vec![
        ("503_503_200", "", Some(vec![Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 3, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("429_200", "", Some(vec![Aktion::Roh(antwort(429, "{\"error\": \"x\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("503_x4", "", Some(vec![Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(503, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 503 {\"error\": \"x\"}"))),
        ("200_erster_versuch", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("201_ist_erfolg", "", Some(vec![Aktion::Roh(antwort(201, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"fake\"}"))]), false, (1, 3, 1), 1, "fake", Erw::Ok("{}", "fake", "stop")),
    ]);
}

/// Abgeschnittene Antwort: genau eine Wiederholung, getrennt von den drei Versuchen gezaehlt.
#[test]
#[rustfmt::skip]
fn abgeschnitten_bekommt_genau_eine_wiederholung_wie_python() {
    pruefe(vec![
        ("abgeschnitten_zweimal", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Err("abgeschnitten", "abgeschnitten", 2, Msg::Genau("LLM-Aufruf nach 2 Versuch(en) fehlgeschlagen: LLM-Antwort bei 8192 Tokens abgeschnitten — unvollständiges JSON."))),
        ("abgeschnitten_dreimal", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Err("abgeschnitten", "abgeschnitten", 2, Msg::Genau("LLM-Aufruf nach 2 Versuch(en) fehlgeschlagen: LLM-Antwort bei 8192 Tokens abgeschnitten — unvollständiges JSON."))),
        ("abgeschnitten_dann_gut", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("503_abgeschnitten_gut", "", Some(vec![Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 3, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("503_abgeschnitten_abgeschnitten", "", Some(vec![Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake")))]), false, (1, 3, 1), 3, "fake", Erw::Err("abgeschnitten", "abgeschnitten", 2, Msg::Genau("LLM-Aufruf nach 2 Versuch(en) fehlgeschlagen: LLM-Antwort bei 8192 Tokens abgeschnitten — unvollständiges JSON."))),
        ("abgeschnitten_503_503", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake"))), Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(503, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 3, "fake", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 503 {\"error\": \"x\"}"))),
        ("503_503_abgeschnitten", "", Some(vec![Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake")))]), false, (1, 3, 1), 3, "fake", Erw::Err("abgeschnitten", "abgeschnitten", 1, Msg::Genau("LLM-Aufruf nach 1 Versuch(en) fehlgeschlagen: LLM-Antwort bei 8192 Tokens abgeschnitten — unvollständiges JSON."))),
        ("abgeschnitten_vor_leer", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("", "length", "fake"))), Aktion::Roh(antwort(200, &ok_body("", "length", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Err("abgeschnitten", "abgeschnitten", 2, Msg::Genau("LLM-Aufruf nach 2 Versuch(en) fehlgeschlagen: LLM-Antwort bei 8192 Tokens abgeschnitten — unvollständiges JSON."))),
    ]);
}

/// Leerer Inhalt (auch nur Leerraum, 0, false, [], null, {}, fehlend) ist voruebergehend mit Grund `leere_antwort`.
#[test]
#[rustfmt::skip]
fn leere_und_falsche_inhalte_sind_voruebergehend_wie_python() {
    pruefe(vec![
        ("leer_dreimal", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("", "stop", "fake")))]), false, (1, 3, 1), 3, "fake", Erw::Err("voruebergehend", "leere_antwort", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: leerer Inhalt vom Anbieter"))),
        ("leer_dann_gut", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("nur_leerraum_dreimal", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("  \n ", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("  \n ", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("  \n ", "stop", "fake")))]), false, (1, 3, 1), 3, "fake", Erw::Err("voruebergehend", "leere_antwort", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: leerer Inhalt vom Anbieter"))),
        ("inhalt_falsch_0", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": 0}, \"finish_reason\": \"stop\"}], \"provider\": \"p\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("inhalt_falsch_1", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": false}, \"finish_reason\": \"stop\"}], \"provider\": \"p\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("inhalt_falsch_2", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": []}, \"finish_reason\": \"stop\"}], \"provider\": \"p\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("inhalt_falsch_3", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": null}, \"finish_reason\": \"stop\"}], \"provider\": \"p\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("inhalt_falsch_4", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": {}}, \"finish_reason\": \"stop\"}], \"provider\": \"p\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("inhalt_fehlt", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {}, \"finish_reason\": \"stop\"}], \"provider\": \"p\"}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
    ]);
}

/// `finish_reason` und `provider` (falsche Werte werden leer), erste Wahl, Strukturfehler und kaputtes JSON sind endgueltig.
#[test]
#[rustfmt::skip]
fn antwortfelder_und_strukturfehler_wie_python() {
    pruefe(vec![
        ("finish_provider_0", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": false}], \"provider\": \"p\"}"))]), false, (1, 3, 1), 1, "p", Erw::Ok("{}", "p", "")),
        ("finish_provider_1", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": 0}], \"provider\": \"p\"}"))]), false, (1, 3, 1), 1, "p", Erw::Ok("{}", "p", "")),
        ("finish_provider_2", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": null}], \"provider\": \"p\"}"))]), false, (1, 3, 1), 1, "p", Erw::Ok("{}", "p", "")),
        ("finish_provider_3", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": \"\"}], \"provider\": \"p\"}"))]), false, (1, 3, 1), 1, "p", Erw::Ok("{}", "p", "")),
        ("finish_provider_4", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": \"stop\"}], \"provider\": false}"))]), false, (1, 3, 1), 1, "", Erw::Ok("{}", "", "stop")),
        ("finish_provider_5", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": \"stop\"}], \"provider\": 0}"))]), false, (1, 3, 1), 1, "", Erw::Ok("{}", "", "stop")),
        ("finish_provider_6", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": \"stop\"}], \"provider\": null}"))]), false, (1, 3, 1), 1, "", Erw::Ok("{}", "", "stop")),
        ("finish_provider_7", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"\"}"))]), false, (1, 3, 1), 1, "", Erw::Ok("{}", "", "stop")),
        ("finish_provider_8", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"OpenAI\"}"))]), false, (1, 3, 1), 1, "OpenAI", Erw::Ok("{}", "OpenAI", "stop")),
        ("finish_provider_9", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": \"content_filter\"}], \"provider\": \"p\"}"))]), false, (1, 3, 1), 1, "p", Erw::Ok("{}", "p", "content_filter")),
        ("finish_provider_10", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": \"stop\"}], \"provider\": 7}"))]), false, (1, 3, 1), 1, "7", Erw::Ok("{}", "7", "stop")),
        ("zwei_wahlen", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"erste\"}, \"finish_reason\": \"stop\"}, {\"message\": {\"content\": \"zweite\"}, \"finish_reason\": \"stop\"}]}"))]), false, (1, 3, 1), 1, "", Erw::Ok("erste", "", "stop")),
        ("message_kein_objekt", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": \"text\", \"finish_reason\": \"stop\"}]}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Egal)),
        ("ohne_choices", "", Some(vec![Aktion::Roh(antwort(200, "{\"x\": 1}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Egal)),
        ("choices_leer", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": []}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Egal)),
        ("kein_json", "", Some(vec![Aktion::Roh(antwort(200, "<html>")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: JSONDecodeError"))),
    ]);
}

/// Verbindung verweigert und Socket-Timeout sind voruebergehend (drei Versuche), eine Verbindung ohne Antwort ist endgueltig.
#[test]
#[rustfmt::skip]
fn netzfehler_und_socket_zeit_wie_python() {
    pruefe(vec![
        ("netz_verbindung_verweigert", "", None, false, (1, 3, 1), 0, "", Erw::Err("voruebergehend", "", 3, Msg::Egal)),
        ("stille_ueber_socket", "", Some(vec![Aktion::Stille(Duration::from_millis(1500)), Aktion::Stille(Duration::from_millis(1500)), Aktion::Stille(Duration::from_millis(1500))]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: TimeoutError: Zeitüberschreitung nach 1s"))),
        ("schliessen_ohne_antwort", "", Some(vec![Aktion::Schliessen, Aktion::Schliessen, Aktion::Schliessen]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Beginnt("LLM-Aufruf fehlgeschlagen: RemoteDisconnected"))),
    ]);
}

/// Wanduhr-Frist: troepfelnde Antwort ueber die Frist ist endgueltig (Grund `frist_ueberschritten`, kein zweiter Versuch); nach einer abgeschnittenen Antwort gilt die kurze Frist, sonst die lange.
#[test]
#[rustfmt::skip]
fn frist_je_versuch_wie_python() {
    pruefe(vec![
        ("frist_troepfelnd", "", Some(vec![tropfen_gross(60, 80)]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "frist_ueberschritten", 1, Msg::Genau("LLM-Antwort überschritt die Frist"))),
        ("frist_gross_ohne_tropfen", "", Some(vec![Aktion::Roh(antwort(200, &ok_body(&"x".repeat(70_000), "stop", "fake")))]), false, (1, 3, 1), 1, "fake", Erw::OkX(70000, "fake", "stop")),
        ("wiederholung_kurze_frist", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake"))), tropfen_gross(5, 300)]), false, (1, 3, 1), 2, "fake", Erw::Err("endgueltig", "frist_ueberschritten", 1, Msg::Genau("LLM-Antwort überschritt die Frist"))),
        ("erster_versuch_lange_frist", "", Some(vec![tropfen_gross(5, 300)]), false, (1, 3, 1), 1, "fake", Erw::OkX(70000, "fake", "stop")),
        ("frist_troepfelnd_unter_64k", "", Some(vec![Aktion::Tropfen { vorne: "HTTP/1.1 200 X\r\nContent-Length: 106\r\nConnection: close\r\n\r\n{\"choices\": [{\"message\": {\"content\": \"{\\\"aussagen\\\": []}\"}, \"finish_reason\":".as_bytes().to_vec(), rest: " \"stop\"}], \"provider\": \"fake\"}".as_bytes().to_vec(), pause: Duration::from_millis(60) }]), false, (1, 1, 1), 1, "", Erw::Err("endgueltig", "frist_ueberschritten", 1, Msg::Genau("LLM-Antwort überschritt die Frist"))),
        ("frist_vor_socket_timeout_unter_64k", "", Some(vec![Aktion::Stufen(vec![("HTTP/1.1 200 X\r\nContent-Length: 106\r\nConnection: close\r\n\r\n{\"choices\": [{\"messa".as_bytes().to_vec(), Duration::from_millis(0)), ("g".as_bytes().to_vec(), Duration::from_millis(40)), ("e".as_bytes().to_vec(), Duration::from_millis(40)), ("\"".as_bytes().to_vec(), Duration::from_millis(40)), (":".as_bytes().to_vec(), Duration::from_millis(40)), (" ".as_bytes().to_vec(), Duration::from_millis(40)), ("{".as_bytes().to_vec(), Duration::from_millis(40)), ("\"".as_bytes().to_vec(), Duration::from_millis(40)), ("c".as_bytes().to_vec(), Duration::from_millis(40)), ("o".as_bytes().to_vec(), Duration::from_millis(40)), ("n".as_bytes().to_vec(), Duration::from_millis(40)), ("t".as_bytes().to_vec(), Duration::from_millis(40)), ("e".as_bytes().to_vec(), Duration::from_millis(40)), ("n".as_bytes().to_vec(), Duration::from_millis(40)), ("t".as_bytes().to_vec(), Duration::from_millis(40)), ("\"".as_bytes().to_vec(), Duration::from_millis(40)), (":".as_bytes().to_vec(), Duration::from_millis(40)), (" ".as_bytes().to_vec(), Duration::from_millis(40)), ("\"".as_bytes().to_vec(), Duration::from_millis(40)), ("{".as_bytes().to_vec(), Duration::from_millis(40)), ("\\".as_bytes().to_vec(), Duration::from_millis(40)), ("\"".as_bytes().to_vec(), Duration::from_millis(40)), ("a".as_bytes().to_vec(), Duration::from_millis(40)), ("u".as_bytes().to_vec(), Duration::from_millis(40)), ("s".as_bytes().to_vec(), Duration::from_millis(40)), ("s".as_bytes().to_vec(), Duration::from_millis(40)), ("a".as_bytes().to_vec(), Duration::from_millis(40)), ("g".as_bytes().to_vec(), Duration::from_millis(40)), ("e".as_bytes().to_vec(), Duration::from_millis(40)), ("".as_bytes().to_vec(), Duration::from_millis(1500))]), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 1, 1), 2, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
    ]);
}

/// HTTP-Ebene: Kopfnamen gross/klein, Leerraum um Werte, `chunked` (auch gross geschrieben, mit Erweiterung, Einzelbyte-Chunks), Laenge gegen Chunks, Ende ohne Laenge, fremde und kaputte Statuszeile, Stille nach Kopf oder Teilkoerper, 299 und 300 an der Grenze der Erfolge.
#[test]
#[rustfmt::skip]
fn http_rahmung_kopfzeilen_und_statuszeile_wie_python() {
    pruefe(vec![
        ("http_kopf_gross_klein", "", Some(vec![Aktion::Roh("HTTP/1.1 200 OK\r\nCONTENT-length:   106  \r\nConnection: close\r\n\r\n{\"choices\": [{\"message\": {\"content\": \"{\\\"aussagen\\\": []}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"fake\"}".as_bytes().to_vec())]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("http_chunked_gross_geschrieben", "", Some(vec![Aktion::Roh("HTTP/1.1 200 OK\r\nTransfer-Encoding: Chunked\r\nConnection: close\r\n\r\na\r\n{\"choices\"\r\n60\r\n: [{\"message\": {\"content\": \"{\\\"aussagen\\\": []}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"fake\"}\r\n0\r\n\r\n".as_bytes().to_vec())]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("http_chunked_hex_erweiterung_einzelbyte", "", Some(vec![Aktion::Roh("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n1a;x=1\r\n{\"choices\": [{\"message\": {\r\n1;x=1\r\n\"\r\n4f\r\ncontent\": \"{\\\"aussagen\\\": []}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"fake\"}\r\n0\r\n\r\n".as_bytes().to_vec())]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("http_chunked_und_falsche_laenge", "", Some(vec![Aktion::Roh("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Length: 3\r\nConnection: close\r\n\r\nc\r\n{\"choices\": \r\n5e\r\n[{\"message\": {\"content\": \"{\\\"aussagen\\\": []}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"fake\"}\r\n0\r\n\r\n".as_bytes().to_vec())]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("http_ohne_laenge_bis_ende", "", Some(vec![Aktion::Roh("HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n{\"choices\": [{\"message\": {\"content\": \"{\\\"aussagen\\\": []}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"fake\"}".as_bytes().to_vec())]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("http_statuszeile_fremd", "", Some(vec![Aktion::Roh("ICY 200 OK\r\nContent-Length: 106\r\n\r\n{\"choices\": [{\"message\": {\"content\": \"{\\\"aussagen\\\": []}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"fake\"}".as_bytes().to_vec())]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: BadStatusLine"))),
        ("http_statuszeile_ohne_zahl", "", Some(vec![Aktion::Roh("HTTP/1.1 abc OK\r\nContent-Length: 2\r\n\r\n{}".as_bytes().to_vec())]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: BadStatusLine"))),
        ("http_kopf_dann_stille", "", Some(vec![Aktion::Tropfen { vorne: "HTTP/1.1 200 X\r\nContent-Le".as_bytes().to_vec(), rest: "n".as_bytes().to_vec(), pause: Duration::from_millis(1500) }, Aktion::Tropfen { vorne: "HTTP/1.1 200 X\r\nContent-Le".as_bytes().to_vec(), rest: "n".as_bytes().to_vec(), pause: Duration::from_millis(1500) }, Aktion::Tropfen { vorne: "HTTP/1.1 200 X\r\nContent-Le".as_bytes().to_vec(), rest: "n".as_bytes().to_vec(), pause: Duration::from_millis(1500) }]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: TimeoutError: Zeitüberschreitung nach 1s"))),
        ("http_koerper_dann_stille_200", "", Some(vec![Aktion::Tropfen { vorne: "HTTP/1.1 200 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }, Aktion::Tropfen { vorne: "HTTP/1.1 200 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }, Aktion::Tropfen { vorne: "HTTP/1.1 200 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: TimeoutError: Zeitüberschreitung nach 1s"))),
        ("http_koerper_dann_stille_299", "", Some(vec![Aktion::Tropfen { vorne: "HTTP/1.1 299 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }, Aktion::Tropfen { vorne: "HTTP/1.1 299 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }, Aktion::Tropfen { vorne: "HTTP/1.1 299 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: TimeoutError: Zeitüberschreitung nach 1s"))),
        ("http_koerper_dann_stille_300", "", Some(vec![Aktion::Tropfen { vorne: "HTTP/1.1 300 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }, Aktion::Tropfen { vorne: "HTTP/1.1 300 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }, Aktion::Tropfen { vorne: "HTTP/1.1 300 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 300 "))),
        ("http_koerper_dann_stille_400", "", Some(vec![Aktion::Tropfen { vorne: "HTTP/1.1 400 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }, Aktion::Tropfen { vorne: "HTTP/1.1 400 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }, Aktion::Tropfen { vorne: "HTTP/1.1 400 X\r\nContent-Length: 100\r\nConnection: close\r\n\r\nabc".as_bytes().to_vec(), rest: "d".as_bytes().to_vec(), pause: Duration::from_millis(1500) }]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 400 "))),
        ("http_299_ist_erfolg", "", Some(vec![Aktion::Roh(antwort(299, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("http_erfolg_ohne_chunk_ende_im_kopf", "", Some(vec![Aktion::Roh("HTTP/1.1 200 OK\r\nContent-Length: 106\r\nX-Zeile-ohne-Doppelpunkt\r\nConnection: close\r\n\r\n{\"choices\": [{\"message\": {\"content\": \"{\\\"aussagen\\\": []}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"fake\"}".as_bytes().to_vec())]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("http_laenge_erreicht_verbindung_offen", "", Some(vec![Aktion::Stufen(vec![("HTTP/1.1 200 X\r\nContent-Length: 106\r\n\r\n{\"choices\": [{\"message\": {\"content\": \"{\\\"aussagen\\\": []}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"fake\"}".as_bytes().to_vec(), Duration::from_millis(2000))])]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
    ]);
}

/// Basis-URL: IPv6 in Klammern, mehrteiliger Pfad, Port ohne Zahl, `https` gegen einen Klartext-Dienst (kein Rueckfall auf Klartext).
#[test]
#[rustfmt::skip]
fn basis_url_wie_python() {
    pruefe(vec![
        ("url_port_keine_zahl", "http://127.0.0.1:abc/v1", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 0, "", Erw::Err("endgueltig", "", 1, Msg::Egal)),
        ("url_https_gegen_klartext", "https://127.0.0.1:{port}/v1", Some(vec![Aktion::Schliessen, Aktion::Schliessen, Aktion::Schliessen]), false, (1, 3, 1), 0, "", Erw::Err("voruebergehend", "", 3, Msg::Egal)),
        ("url_ipv6", "http://[::1]:{port}/v1", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("url_pfad_mehrteilig", "http://127.0.0.1:{port}/a/b/v1", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
    ]);
}

/// ABWEICHUNG VON PYTHON, keine Absicht (Befund der Messung): drei Eingaben enden in Rust anders als in Python. Der Test haelt das heutige Rust-Verhalten fest, damit eine Aenderung auffaellt; er ist kein Soll. Die Zeile je Eingabe nennt, was Python tut. Die vierte Eingabe, IPv6 ohne Port (`http://[::1]/v1`), ist behoben: `zerlege` nimmt Port 80 oder 443 (`http::tests::url_zerlegen_ipv6_literal`); ein Test gegen den echten Port 80 entfaellt, weil er sich nicht binden laesst.
#[test]
#[rustfmt::skip]
fn weicht_von_python_ab_drei_eingaben_enden_in_rust_anders() {
    pruefe(vec![
        // http_laenge_keine_zahl: Python liest bei `Content-Length: abc` bis zum Ende der Verbindung und liefert die Antwort.
        ("http_laenge_keine_zahl", "", Some(vec![Aktion::Roh("HTTP/1.1 200 OK\r\nContent-Length: abc\r\nConnection: close\r\n\r\n{\"choices\": [{\"message\": {\"content\": \"{\\\"aussagen\\\": []}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"fake\"}".as_bytes().to_vec())]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: Content-Length"))),
        // url_ohne_schema: Python: `URLError: unknown url type`, also voruebergehend mit drei Versuchen.
        ("url_ohne_schema", "127.0.0.1:{port}/v1", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 0, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: unbekanntes URL-Schema"))),
        // url_fremdes_schema: Python versucht FTP und scheitert voruebergehend (drei Versuche, hier nach Socket-Timeout).
        ("url_fremdes_schema", "ftp://127.0.0.1:{port}/v1", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 0, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: unbekanntes URL-Schema"))),
    ]);
}

/// ABWEICHUNG VON PYTHON, gewollt (`client.rs`, `content` keine Zeichenkette): Python ruft `.strip()` auf einer Liste, einer Zahl
/// oder einem Objekt und stuerzt mit `AttributeError` ausserhalb jeder Behandlung (der Dienst antwortet 500). Rust meldet das als
/// endgueltigen Fehler; der Aufruf bricht nicht ab. Eine leere oder falsche Liste/Zahl/Zeichenkette zaehlt dagegen wie in Python als leer.
#[test]
#[rustfmt::skip]
fn weicht_von_python_ab_inhalt_ohne_zeichenkette_ist_endgueltig() {
    pruefe(vec![
        ("inhalt_liste", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": [\"x\"]}, \"finish_reason\": \"stop\"}]}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: AttributeError"))),
        ("inhalt_zahl", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": 5}, \"finish_reason\": \"stop\"}]}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: AttributeError"))),
        ("inhalt_objekt", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": {\"a\": 1}}, \"finish_reason\": \"stop\"}]}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: AttributeError"))),
    ]);
}

/// ABWEICHUNG VON PYTHON, gewollt (`client.rs`, Antwortkoerper des Anbieters): `json.loads` liest `NaN`, `Infinity` und `1e400`
/// (= `inf`) irgendwo im Koerper, Python nimmt die Antwort an. `serde_json` lehnt den Koerper ab; Rust meldet einen
/// endgueltigen Fehler `JSONDecodeError`. Die Antwort enthaelt sonst einen gueltigen Inhalt: die Ablehnung kommt nur vom Zahlwort.
#[test]
#[rustfmt::skip]
fn weicht_von_python_ab_anbieterantwort_mit_nan_ist_endgueltig() {
    let koerper = |zusatz: &str| format!("{{\"choices\": [{{\"message\": {{\"content\": \"{{}}\"}}, \"finish_reason\": \"stop\"}}], \"usage\": {zusatz}}}");
    let fall = |name: &'static str, zusatz: &str| -> Fall {
        (name, "", Some(vec![Aktion::Roh(antwort(200, &koerper(zusatz)))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: JSONDecodeError")))
    };
    pruefe(vec![
        fall("usage_nan", "NaN"),
        fall("usage_infinity", "Infinity"),
        fall("usage_minus_infinity", "-Infinity"),
        fall("usage_ueber_f64", "1e400"),
        // Kontrolle: dieselbe Gestalt mit einer gueltigen Zahl wird gelesen.
        ("usage_gueltig", "", Some(vec![Aktion::Roh(antwort(200, &koerper("1")))]), false, (1, 3, 1), 1, "", Erw::Ok("{}", "", "stop")),
    ]);
}

/// ABWEICHUNG VON PYTHON, gewollt (Sicherheit, `client.rs`, Fehlerkoerper): Python kuerzt den Koerper auf 300 Zeichen und maskiert
/// danach; ein Schluessel, der ueber die Schnittkante ragt, bliebe als Anfang stehen (hier `sk-test-GE`). Rust maskiert zuerst
/// und kuerzt dann. Der Schluessel beginnt bei Zeichen 290 und endet bei 308.
#[test]
fn weicht_von_python_ab_schluessel_an_der_schnittkante_wird_maskiert() {
    let nachrichten = aussagen_prompt(&filtere("hallo").0);
    let koerper = format!("{}{SCHLUESSEL}", "a".repeat(290));
    let stub = Stub::starte(vec![Aktion::Roh(antwort(400, &koerper))]);
    let e = chat(stub.basis("/v1"), (1, 3, 1))
        .complete(&nachrichten, None)
        .unwrap_err();
    assert_eq!(
        e.to_string(),
        format!(
            "LLM-Aufruf fehlgeschlagen: HTTP 400 {}<KEY>",
            "a".repeat(290)
        )
    );
    assert!(
        !e.to_string().contains("sk-"),
        "Anfang des Schluessels: {e}"
    );
}

/// `letzter_anbieter` folgt der letzten gelesenen Antwort und ist leer, wenn der Aufruf vor dem Lesen endet (`_merke`).
#[test]
#[rustfmt::skip]
fn letzter_anbieter_folgt_der_letzten_antwort_wie_python() {
    pruefe(vec![
        ("200_erster_versuch", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "fake", Erw::Ok("{\"aussagen\": []}", "fake", "stop")),
        ("status_400_endgueltig", "", Some(vec![Aktion::Roh(antwort(400, "{\"error\": \"x\"}")), Aktion::Roh(antwort(400, "{\"error\": \"x\"}")), Aktion::Roh(antwort(400, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: HTTP 400 {\"error\": \"x\"}"))),
        ("leer_dreimal", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("", "stop", "fake"))), Aktion::Roh(antwort(200, &ok_body("", "stop", "fake")))]), false, (1, 3, 1), 3, "fake", Erw::Err("voruebergehend", "leere_antwort", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: leerer Inhalt vom Anbieter"))),
        ("kein_json", "", Some(vec![Aktion::Roh(antwort(200, "<html>")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Genau("LLM-Aufruf fehlgeschlagen: JSONDecodeError"))),
        ("abgeschnitten_zweimal", "", Some(vec![Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake"))), Aktion::Roh(antwort(200, &ok_body("{\"aus", "length", "fake")))]), false, (1, 3, 1), 2, "fake", Erw::Err("abgeschnitten", "abgeschnitten", 2, Msg::Genau("LLM-Aufruf nach 2 Versuch(en) fehlgeschlagen: LLM-Antwort bei 8192 Tokens abgeschnitten — unvollständiges JSON."))),
        ("ohne_choices", "", Some(vec![Aktion::Roh(antwort(200, "{\"x\": 1}")), Aktion::Roh(antwort(200, &ok_body("{\"aussagen\": []}", "stop", "fake")))]), false, (1, 3, 1), 1, "", Erw::Err("endgueltig", "", 1, Msg::Egal)),
        ("finish_provider_8", "", Some(vec![Aktion::Roh(antwort(200, "{\"choices\": [{\"message\": {\"content\": \"{}\"}, \"finish_reason\": \"stop\"}], \"provider\": \"OpenAI\"}"))]), false, (1, 3, 1), 1, "OpenAI", Erw::Ok("{}", "OpenAI", "stop")),
        ("status_503_voruebergehend", "", Some(vec![Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(503, "{\"error\": \"x\"}")), Aktion::Roh(antwort(503, "{\"error\": \"x\"}"))]), false, (1, 3, 1), 3, "", Erw::Err("voruebergehend", "", 3, Msg::Genau("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: HTTP 503 {\"error\": \"x\"}"))),
    ]);
}

/// Die Anfrage auf dem Draht: Zeile, Kopfzeilen, JSON-Koerper (ohne und mit Schema) wie bei Python. `Host` traegt den
/// Port, `Content-Length` die Koerperlaenge; der Agent ist Rust-eigen (`taxgraph`, Python sendet `Python-urllib`).
#[test]
fn die_anfrage_auf_dem_draht_ist_wie_bei_python() {
    let nachrichten = aussagen_prompt(&filtere("hallo").0);
    let erwartet = [
        r#"{"model": "m", "messages": [{"role": "system", "content": "<SYSTEM>"}, {"role": "user", "content": "hallo"}], "temperature": 0, "response_format": {"type": "json_object"}, "max_tokens": 8192}"#,
        r#"{"model": "m", "messages": [{"role": "system", "content": "<SYSTEM>"}, {"role": "user", "content": "hallo"}], "temperature": 0, "response_format": {"type": "json_schema", "json_schema": "<DIALOG_SCHEMA>"}, "max_tokens": 8192, "provider": {"require_parameters": true}}"#,
    ];
    for (schema, erwartet) in [false, true].into_iter().zip(erwartet) {
        let stub = Stub::starte(vec![Aktion::Roh(antwort(
            200,
            &ok_body("{}", "stop", "fake"),
        ))]);
        chat(stub.basis("/v1"), (1, 3, 1))
            .complete(&nachrichten, schema.then_some(&*DIALOG_SCHEMA))
            .unwrap();
        let a = &stub.anfragen()[0];
        assert_eq!(a.anfragezeile(), "POST /v1/chat/completions HTTP/1.1");
        assert_eq!(
            a.kopfzeilen_vergleichbar(),
            vec![
                "accept-encoding: identity",
                "authorization: Bearer sk-test-GEHEIM-123",
                "connection: close",
                "content-type: application/json"
            ]
        );
        assert_eq!(
            a.kopfzeile("Host"),
            Some(format!("127.0.0.1:{}", stub.port))
        );
        assert_eq!(a.kopfzeile("User-Agent").as_deref(), Some("taxgraph"));
        assert_eq!(
            a.kopfzeile("Content-Length"),
            Some(a.koerper.len().to_string())
        );
        let mut nutzlast: Value = serde_json::from_slice(&a.koerper).unwrap();
        assert_eq!(nutzlast["messages"][0]["content"], nachrichten[0].inhalt());
        nutzlast["messages"][0]["content"] = json!("<SYSTEM>");
        if schema {
            assert_eq!(nutzlast["response_format"]["json_schema"], *DIALOG_SCHEMA);
            nutzlast["response_format"]["json_schema"] = json!("<DIALOG_SCHEMA>");
        }
        let erwartet: Value = serde_json::from_str(erwartet).unwrap();
        assert_eq!(nutzlast, erwartet, "schema={schema}");
    }
}

/// `maskiere` gilt fuer jede Meldung, die Text des Dienstes oder der Verbindung traegt. Python maskiert nur den Fehlerkoerper
/// (`llm_client.py:307-308`); Rust maskiert zusaetzlich die Netz- und die Kaputt-Meldungen (Rust-eigene Vorsicht). Der Schluessel
/// ist hier ein Wort aus der Meldung selbst. Ein leerer Schluessel maskiert nichts (`if schluessel and ...`, Zeile 307).
#[test]
fn der_schluessel_wird_in_jeder_meldung_maskiert() {
    let nachrichten = aussagen_prompt(&filtere("hallo").0);
    let mit_schluessel = |basis: String, schluessel: &str| {
        let mut k = Konfiguration::neu(basis, "m".into(), schluessel.into());
        k.socket = Duration::from_secs(1);
        k.backoff = [Duration::ZERO; 2];
        HttpChat { konfiguration: k }
            .complete(&nachrichten, None)
            .unwrap_err()
            .to_string()
    };
    // Netz: nichts lauscht am Port, die Meldung nennt `Connection refused`.
    let tot = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let basis = format!("http://127.0.0.1:{}/v1", tot.local_addr().unwrap().port());
    drop(tot);
    let m = mit_schluessel(basis.clone(), "refused");
    assert!(m.starts_with("LLM-Aufruf nach 3 Versuch(en) fehlgeschlagen: URLError: Connection <KEY> (os error 111)"), "{m}");
    assert!(!m.contains("refused"), "{m}");
    // Kaputt: eine fremde Statuszeile ergibt `BadStatusLine`.
    let ok = Stub::starte(vec![Aktion::Roh(
        b"ICY 200 OK\r\nContent-Length: 2\r\n\r\n{}".to_vec(),
    )]);
    assert_eq!(
        mit_schluessel(ok.basis("/v1"), "BadStatusLine"),
        "LLM-Aufruf fehlgeschlagen: <KEY>"
    );
    // Leerer Schluessel: der Fehlerkoerper bleibt, wie er ist.
    let fehler = Stub::starte(vec![Aktion::Roh(antwort(400, "boom"))]);
    assert_eq!(
        mit_schluessel(fehler.basis("/v1"), ""),
        "LLM-Aufruf fehlgeschlagen: HTTP 400 boom"
    );
}

/// Zwischen den Versuchen wird gewartet: `backoff[versuch]` nach dem Versuch mit der Nummer `versuch` (`_BACKOFF_S`).
/// Untergrenze, weil nach oben die Last des Rechners mitspielt.
#[test]
fn zwischen_den_versuchen_wird_gewartet_wie_python() {
    let nachrichten = aussagen_prompt(&filtere("hallo").0);
    let fehler = antwort(503, "{}");
    let gut = antwort(200, &ok_body("{}", "stop", "fake"));
    for (skript, mindestens_ms) in [
        (vec![fehler.clone(), fehler.clone(), fehler.clone()], 600),
        (vec![fehler.clone(), fehler.clone(), gut.clone()], 600),
        (vec![fehler, gut], 200),
    ] {
        let stub = Stub::starte(skript.into_iter().map(Aktion::Roh).collect());
        let mut c = chat(stub.basis("/v1"), (1, 3, 1));
        c.konfiguration.backoff = [Duration::from_millis(200), Duration::from_millis(400)];
        let t0 = Instant::now();
        let _ = c.complete(&nachrichten, None);
        assert!(
            t0.elapsed() >= Duration::from_millis(mindestens_ms),
            "{:?} < {mindestens_ms} ms",
            t0.elapsed()
        );
    }
}
