//! `Konfiguration::aus_env` liest `$LLM_API_BASE`, `$LLM_MODEL`, `$LLM_API_KEY` wie `llm_client._base/_model/_key`
//! (Python: `strip()`, bei der Basis zusaetzlich `rstrip("/")`; erst Basis und Modell, dann der Schluessel). Eine
//! eigene Testdatei mit EINEM Test, weil die Umgebung prozessweit gilt. Die Meldungen und der Aufruf im letzten
//! Fall sind Laeufe der Python-Funktion `llm_client.complete` mit denselben Umgebungswerten.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod stub;

use llm::{Chat, HttpChat, Konfiguration, LlmFehler};
use serde_json::Value;
use stub::{antwort, Aktion, Stub};

const KEINE_BASIS: &str = "LLM_API_BASE/LLM_MODEL nicht gesetzt — Provider nicht konfiguriert.";
const KEIN_SCHLUESSEL: &str =
    "kein LLM_API_KEY in der Umgebung — Chat bleibt reine Erklär-Grenze ($0).";

fn setze(basis: Option<&str>, modell: Option<&str>, schluessel: Option<&str>) {
    for (name, wert) in [
        ("LLM_API_BASE", basis),
        ("LLM_MODEL", modell),
        ("LLM_API_KEY", schluessel),
    ] {
        match wert {
            Some(w) => std::env::set_var(name, w),
            None => std::env::remove_var(name),
        }
    }
}

fn fehler(basis: Option<&str>, modell: Option<&str>, schluessel: Option<&str>) -> String {
    setze(basis, modell, schluessel);
    match Konfiguration::aus_env() {
        Err(LlmFehler::Endgueltig { detail, .. }) => detail,
        Err(e) => panic!("falsche Fehlerklasse: {e:?}"),
        Ok(_) => "ok".into(),
    }
}

#[test]
fn die_umgebung_bestimmt_basis_modell_und_schluessel_wie_python() {
    // Basis und Modell zuerst: jede der beiden Angaben fehlt oder ist leer -> dieselbe Meldung, auch ohne Schluessel.
    assert_eq!(fehler(None, None, None), KEINE_BASIS);
    assert_eq!(fehler(Some("http://x"), None, None), KEINE_BASIS);
    assert_eq!(fehler(None, Some("m"), None), KEINE_BASIS);
    assert_eq!(fehler(Some("  "), Some("m"), None), KEINE_BASIS);
    assert_eq!(fehler(Some("http://x"), Some(" "), None), KEINE_BASIS);
    assert_eq!(fehler(None, None, Some("k")), KEINE_BASIS);
    assert_eq!(fehler(Some("/"), Some("m"), Some("k")), KEINE_BASIS);
    // Dann der Schluessel: fehlt oder besteht aus Leerraum.
    assert_eq!(fehler(Some("http://x"), Some("m"), None), KEIN_SCHLUESSEL);
    assert_eq!(
        fehler(Some("http://x"), Some("m"), Some("  \n")),
        KEIN_SCHLUESSEL
    );

    // Alles gesetzt, mit Leerraum und Schraegstrichen: Basis ohne beides, Modell und Schluessel gestrippt.
    let ok = Stub::starte(vec![Aktion::Roh(antwort(
        200,
        r#"{"choices": [{"message": {"content": "{}"}, "finish_reason": "stop"}], "provider": "p"}"#,
    ))]);
    let basis = format!(" {}// ", ok.basis("/v1"));
    setze(Some(&basis), Some(" m \n"), Some(" sk-env-1 \n"));
    let konfig = Konfiguration::aus_env().unwrap();
    assert_eq!(
        (konfig.basis.as_str(), konfig.modell.as_str()),
        (ok.basis("/v1").as_str(), "m")
    );
    HttpChat {
        konfiguration: konfig,
    }
    .complete(&[], None)
    .unwrap();
    let a = &ok.anfragen()[0];
    assert_eq!(a.anfragezeile(), "POST /v1/chat/completions HTTP/1.1");
    assert_eq!(
        a.kopfzeile("Authorization").as_deref(),
        Some("Bearer sk-env-1")
    );
    let nutzlast: Value = serde_json::from_slice(&a.koerper).unwrap();
    assert_eq!(nutzlast["model"], "m");
    setze(None, None, None);
}
