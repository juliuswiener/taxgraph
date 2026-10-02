//! `api::flow` gegen die Prozess-Umgebung und das Dateisystem (`produkt/haut/flow.py`,
//! `tests/test_flow_mitschnitt.py`). Eigene Testdatei, weil die Tests die Umgebung setzen: im
//! gemeinsamen Binaer liefen sie gegen jeden Nachbarn, der `TAXGRAPH_FLOW` liest. Untereinander
//! schliesst sie [`UMGEBUNG`] aus.
//!
//! Die reinen Funktionen (`dumps`, `gekappt`, `zeile`, `kopf_der_queue`) pruefen die Tests in
//! `flow.rs` und Doctests; Pythons Gegenstueck vergleicht `rust/parity/tests/flow_paritaet.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use api::flow::{self, melde_ui, schreibe, DATEI};
use domain::PyWert;
use serde_json::{json, Value};

static UMGEBUNG: Mutex<()> = Mutex::new(());

/// Setzt Variablen und stellt den vorherigen Wert wieder her — auch beim Panic. Haelt die Sperre
/// bis nach dem Zuruecksetzen.
struct Umgebung(
    Vec<(&'static str, Option<std::ffi::OsString>)>,
    #[allow(dead_code)] MutexGuard<'static, ()>,
);

impl Umgebung {
    fn neue(paare: &[(&'static str, Option<&str>)]) -> Self {
        let sperre = UMGEBUNG.lock().unwrap_or_else(PoisonError::into_inner);
        let alt = paare
            .iter()
            .map(|(k, _)| (*k, std::env::var_os(k)))
            .collect();
        for (k, v) in paare {
            match v {
                Some(w) => std::env::set_var(k, w),
                None => std::env::remove_var(k),
            }
        }
        Self(alt, sperre)
    }

    fn an() -> Self {
        Self::neue(&[("TAXGRAPH_FLOW", Some("1")), ("TAXGRAPH_KI_DEBUG", None)])
    }

    fn aus() -> Self {
        Self::neue(&[("TAXGRAPH_FLOW", None), ("TAXGRAPH_KI_DEBUG", None)])
    }
}

impl Drop for Umgebung {
    fn drop(&mut self) {
        for (k, v) in &self.0 {
            match v {
                Some(w) => std::env::set_var(k, w),
                None => std::env::remove_var(k),
            }
        }
    }
}

fn pw(wert: &Value) -> PyWert {
    serde_json::from_str(&wert.to_string()).unwrap()
}

fn zeilen(ablage: &Path) -> Vec<String> {
    std::fs::read_to_string(ablage.join(DATEI))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// `an()` liest zur Aufrufzeit, trimmt wie Pythons `strip()` (auch U+001C..U+001F) und kennt beide
/// Variablen. Dieselbe Tabelle prueft `flow_paritaet.rs` gegen Python.
#[test]
fn schalter_wie_python() {
    let faelle: [(Option<&str>, Option<&str>, bool); 11] = [
        (None, None, false),
        (Some("1"), None, true),
        (Some("0"), None, false),
        (Some(" 1 "), None, true),
        (Some("\u{1c}1\u{1f}"), None, true),
        (Some("true"), None, false),
        (Some("11"), None, false),
        (Some(""), Some(""), false),
        (None, Some("1"), true),
        (Some("0"), Some("1"), true),
        (Some("1"), Some("0"), true),
    ];
    for (flow, debug, erwartet) in faelle {
        let _u = Umgebung::neue(&[("TAXGRAPH_FLOW", flow), ("TAXGRAPH_KI_DEBUG", debug)]);
        assert_eq!(flow::an(), erwartet, "FLOW={flow:?} KI_DEBUG={debug:?}");
    }
}

/// Ohne Schalter entsteht weder Datei noch Verzeichnis, auch fuer einen Rumpf, den `melde_ui` im
/// Schalterfall abwiese (`flow.py:161`: erst der Schalter, dann die Pruefung).
#[test]
fn ohne_schalter_entsteht_nichts() {
    let _u = Umgebung::aus();
    let tmp = tempfile::tempdir().unwrap();
    let ablage = tmp.path().join("nicht").join("da");
    schreibe(&ablage, Some("f1"), "antwort", &PyWert::Null);
    assert!(!tmp.path().join("nicht").exists());
    let a = melde_ui(&ablage, "f1", &pw(&json!([1, 2]))).unwrap();
    assert_eq!((a.status, a.body), (200, json!({"mitgeschrieben": false})));
    assert!(!tmp.path().join("nicht").exists());
}

/// Die Zeile: Verzeichnis wird angelegt, Datei mit 0600, `ts fall art inhalt` in dieser
/// Reihenfolge, die Schluessel des Inhalts in Einfuegereihenfolge, jede Zeile eine eigene.
#[test]
fn zeile_mit_0600_in_der_reihenfolge_des_clients() {
    let _u = Umgebung::an();
    let tmp = tempfile::tempdir().unwrap();
    let ablage = tmp.path().join("a").join("b");
    // Als Text, nicht ueber `json!`: `Value` sortiert die Schluessel, der Client tut es nicht.
    let inhalt: PyWert = serde_json::from_str(r#"{"z": 1, "a": "ä"}"#).unwrap();
    schreibe(&ablage, Some("f1"), "antwort", &inhalt);
    schreibe(&ablage, None, "fragen", &PyWert::Null);
    let datei = ablage.join(DATEI);
    assert_eq!(
        std::fs::metadata(&datei).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let z = zeilen(&ablage);
    assert_eq!(z.len(), 2);
    let (kopf, rest) = z[0].split_once(", \"fall\": ").unwrap();
    let ts = kopf
        .strip_prefix("{\"ts\": \"")
        .and_then(|k| k.strip_suffix("+00:00\""))
        .unwrap();
    // `YYYY-MM-DDTHH:MM:SS`, danach nichts oder genau `.ffffff` (Python laesst 0 Mikrosekunden weg).
    assert!(
        ts.len() == 19 || (ts.len() == 26 && &ts[19..20] == "."),
        "{ts}"
    );
    assert!(ts.starts_with("20") && &ts[10..11] == "T", "{ts}");
    assert_eq!(
        rest,
        "\"f1\", \"art\": \"antwort\", \"inhalt\": {\"z\": 1, \"a\": \"ä\"}}"
    );
    assert!(
        z[1].ends_with("\"fall\": null, \"art\": \"fragen\", \"inhalt\": null}"),
        "{}",
        z[1]
    );
}

/// Eine bestehende Datei bleibt unangetastet: Rechte und alter Inhalt (`flow.py:90`).
#[test]
fn bestehende_datei_behaelt_rechte_und_inhalt() {
    let _u = Umgebung::an();
    let tmp = tempfile::tempdir().unwrap();
    let datei = tmp.path().join(DATEI);
    std::fs::write(&datei, "alt\n").unwrap();
    std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(0o644)).unwrap();
    schreibe(tmp.path(), Some("f1"), "antwort", &PyWert::Ganz(1));
    assert_eq!(
        std::fs::metadata(&datei).unwrap().permissions().mode() & 0o777,
        0o644
    );
    let z = zeilen(tmp.path());
    assert_eq!(z[0], "alt");
    assert_eq!(z.len(), 2);
}

/// `except Exception: pass`: eine Ablage, die ein Verzeichnis sein muesste und eine Datei ist,
/// reisst den Vorgang nicht mit.
#[test]
fn unbeschreibbare_ablage_wirft_nicht() {
    let _u = Umgebung::an();
    let tmp = tempfile::tempdir().unwrap();
    let datei_statt_verzeichnis = tmp.path().join("datei");
    std::fs::write(&datei_statt_verzeichnis, "x").unwrap();
    schreibe(
        &datei_statt_verzeichnis,
        Some("f1"),
        "antwort",
        &PyWert::Null,
    );
    schreibe(
        &datei_statt_verzeichnis.join("unter"),
        Some("f1"),
        "antwort",
        &PyWert::Null,
    );
    let a = melde_ui(
        &datei_statt_verzeichnis,
        "f1",
        &pw(&json!({"art": "weg_gewaehlt"})),
    )
    .unwrap();
    assert_eq!(a.body, json!({"mitgeschrieben": true}));
    assert_eq!(
        std::fs::read_to_string(&datei_statt_verzeichnis).unwrap(),
        "x"
    );
}

/// Die Zweige von `melde_ui` mit Schalter, mit Pythons Wortlaut (`flow.py:163-168`).
#[test]
fn melde_ui_weist_ab_was_nicht_vorgesehen_ist() {
    let _u = Umgebung::an();
    let tmp = tempfile::tempdir().unwrap();
    let fehler = |body: Value| match melde_ui(tmp.path(), "f1", &pw(&body)) {
        Err(api::ApiFehler::Status(400, m)) => m,
        anders => panic!("{body}: {anders:?}"),
    };
    let arten = "art muss eines von ['nachfrage_spaeter', 'nachfragen_gestartet', \
                 'pruefliste_aendern', 'pruefliste_weiter', 'weg_gewaehlt'] sein";
    assert_eq!(fehler(json!([1])), "Rumpf muss ein Objekt sein");
    assert_eq!(fehler(json!(null)), "Rumpf muss ein Objekt sein");
    assert_eq!(fehler(json!({})), arten);
    assert_eq!(fehler(json!({"art": 5})), arten);
    assert_eq!(fehler(json!({"art": ["weg_gewaehlt"]})), arten);
    assert_eq!(fehler(json!({"art": "erfunden"})), arten);
    assert!(
        zeilen(tmp.path()).is_empty(),
        "eine Abweisung schreibt nichts"
    );
}

/// Eine zulaessige Meldung: 200, eine Zeile mit dem Fall der Anfrage, `inhalt` fehlt -> `null`;
/// ein zu grosser `inhalt` wird gekappt und SAGT es (`flow.py:123-134`).
#[test]
fn melde_ui_schreibt_und_kappt() {
    let _u = Umgebung::an();
    let tmp = tempfile::tempdir().unwrap();
    for art in flow::UI_ARTEN {
        let a = melde_ui(tmp.path(), "f1", &pw(&json!({"art": art}))).unwrap();
        assert_eq!(
            (a.status, a.body),
            (200, json!({"mitgeschrieben": true})),
            "{art}"
        );
    }
    let z = zeilen(tmp.path());
    assert_eq!(z.len(), 5);
    assert!(z[0].ends_with("\"fall\": \"f1\", \"art\": \"nachfrage_spaeter\", \"inhalt\": null}"));

    let gross = pw(&json!({"art": "pruefliste_weiter", "inhalt": {"offen": "x".repeat(5000)}}));
    melde_ui(tmp.path(), "f2", &gross).unwrap();
    let letzte: Value = serde_json::from_str(zeilen(tmp.path()).last().unwrap()).unwrap();
    assert_eq!(letzte["fall"], "f2");
    assert_eq!(letzte["inhalt"]["gekappt_bei"], 4000);
    assert!(letzte["inhalt"]["urspruengliche_zeichen"].as_u64().unwrap() > 5000);
    assert_eq!(
        letzte["inhalt"]["anfang"].as_str().unwrap().chars().count(),
        4000
    );

    // Genau an der Grenze bleibt der Inhalt, wie er ist.
    let genau = pw(&json!({"art": "weg_gewaehlt", "inhalt": "y".repeat(3998)}));
    melde_ui(tmp.path(), "f3", &genau).unwrap();
    let letzte: Value = serde_json::from_str(zeilen(tmp.path()).last().unwrap()).unwrap();
    assert_eq!(letzte["inhalt"].as_str().unwrap().len(), 3998);
}

/// `ergebnis_notiert`: `offen_anzahl` ist die Zahl ALLER benannten Felder, `offen` die ersten 12.
#[test]
fn ergebnis_notiert_meldet_wie_viele_offene_felder_es_benennt() {
    let _u = Umgebung::an();
    let tmp = tempfile::tempdir().unwrap();
    let offen: Vec<String> = (0..15).map(|i| format!("f{i}")).collect();
    flow::ergebnis_notiert(
        tmp.path(),
        "f1",
        &pw(&json!({"grund": "offen", "zahl_cent": null, "offen": offen})),
    );
    let z: Value = serde_json::from_str(&zeilen(tmp.path())[0]).unwrap();
    assert_eq!(z["art"], "ergebnis");
    assert_eq!(z["inhalt"]["offen_anzahl"], 15);
    assert_eq!(z["inhalt"]["offen"].as_array().unwrap().len(), 12);
}
