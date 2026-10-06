//! Gegenstueck zu `tests/test_datenwurzel_ausserhalb_repo.py::test_faelle_und_daten_wurzel_laufen_nicht_auseinander`
//! — dort `xfail(strict=True)`, hier ein NORMALER gruener Test: das verlangte Verhalten ist in
//! Rust schon da.
//!
//! Python haelt dieselbe Sache in ZWEI Repraesentationen: `api_constants.FAELLE` wird beim Import
//! gebunden, `_daten_wurzel()` rechnet bei jedem Aufruf. Wer `$TAXGRAPH_DATEN` nach dem Import
//! setzt, bekommt zwei Orte fuer dieselben Steuerdaten. Der Fix waere der Umbau von 143
//! Teststellen — eine Entscheidung, keine Reparatur, deshalb xfail statt Fix.
//!
//! Rust kennt nur EINE: [`Konfig::aus_env`] ist im Produktcode der einzige Konstruktor (gerufen
//! `zustand.rs:39`, `main.rs:54`) und der einzige Leser von `TAXGRAPH_DATEN`/`XDG_DATA_HOME`; es
//! gibt kein Import-Zeit-Global und keinen zweiten Namen fuer denselben Pfad.
//! `api::Konfig` wird zudem ueberall per Parameter gereicht. Der Test misst genau die Eigenschaft,
//! an der Python scheitert: eine Aenderung der Umgebungsvariable WIRKT auf den naechsten Aufruf.
//!
//! Eigene Testdatei, weil dieser Test die Umgebung setzt — in einem gemeinsamen Binaer liefe er
//! gegen jeden Nachbartest, der `aus_env()` liest.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Mutex, MutexGuard, PoisonError};

use api::konfig::Konfig;

/// Beide Tests setzen die Prozess-Umgebung; ohne diese Sperre liefen sie als Threads desselben
/// Binaers ineinander (gemessen: 1 von 6 Laeufen rot, `faelle` aus dem Nachbartest).
static UMGEBUNG: Mutex<()> = Mutex::new(());

/// Setzt Variablen und stellt den vorherigen Wert wieder her — auch beim Panic. Haelt die Sperre
/// bis nach dem Zuruecksetzen (Felder fallen nach `Drop::drop`).
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

/// Der eine Satz, der in Python rot ist: die Umgebungsvariable wirkt auf den NAECHSTEN Aufruf,
/// und beide Aufrufe meinen denselben Ort.
#[test]
fn eine_wurzel_gilt_sofort_und_fuer_jeden_aufruf() {
    let probe = tempfile::tempdir().unwrap();
    let a = probe.path().join("a");
    let b = probe.path().join("b");
    let _u = Umgebung::neue(&[
        ("TAXGRAPH_DATEN", Some(a.to_str().unwrap())),
        ("XDG_DATA_HOME", None),
    ]);

    let erst = Konfig::aus_env();
    assert_eq!(
        erst.faelle,
        a.join("faelle"),
        "TAXGRAPH_DATEN ersetzt die GESAMTE Wurzel, kein zusaetzliches 'taxgraph'-Segment"
    );

    // DIESER Schritt ist der Defekt aus Python: dort bliebe der beim Import gebundene Wert
    // stehen. Hier muss der zweite Aufruf den neuen Wert sehen.
    std::env::set_var("TAXGRAPH_DATEN", &b);
    let zweit = Konfig::aus_env();
    assert_eq!(
        zweit.faelle,
        b.join("faelle"),
        "die Umgebungsvariable wirkt nicht auf den naechsten Aufruf — es gibt eine zweite, \
         zwischengespeicherte Repraesentation (genau der Python-Defekt)"
    );
    assert_ne!(
        erst.faelle, zweit.faelle,
        "zwei Aufrufe nach einer Aenderung liefern denselben Ort — dann wird nicht bei jedem \
         Aufruf gerechnet"
    );
}

/// Dieselbe Funktion, ohne Umlenkung: `XDG_DATA_HOME` bekommt das `taxgraph`-Segment, `/faelle`
/// haengt in jedem Fall am Ende an. Und ein nur-Leerzeichen-Wert zaehlt wie „nicht gesetzt".
#[test]
fn xdg_konvention_und_leerzeichen_zaehlen_wie_ungesetzt() {
    let probe = tempfile::tempdir().unwrap();
    let xdg = probe.path().join("xdg");
    let _u = Umgebung::neue(&[
        ("TAXGRAPH_DATEN", Some("   ")),
        ("XDG_DATA_HOME", Some(xdg.to_str().unwrap())),
    ]);
    assert_eq!(
        Konfig::aus_env().faelle,
        xdg.join("taxgraph").join("faelle"),
        "ein nur-Leerzeichen-TAXGRAPH_DATEN muss wie 'nicht gesetzt' wirken (getrimmt, nicht-leer)"
    );
}

/// `strip()` in `api_constants._daten_wurzel` (`api_constants.py:32`) nimmt auch U+001C..U+001F
/// weg, `str::trim` nicht: `"\x1c/pfad\x1f"` ist dort `"/pfad"`, `"\x1c\x1f"` leer (gemessen
/// 2026-10-02 mit `python3 -c`).
#[test]
fn steuerzeichen_am_rand_zaehlen_wie_leerzeichen() {
    let probe = tempfile::tempdir().unwrap();
    let a = probe.path().join("a");
    let xdg = probe.path().join("xdg");
    let umrandet = format!("\u{1c}{}\u{1f}", a.to_str().unwrap());
    let erste = Umgebung::neue(&[("TAXGRAPH_DATEN", Some(&umrandet)), ("XDG_DATA_HOME", None)]);
    assert_eq!(Konfig::aus_env().faelle, a.join("faelle"));
    drop(erste);
    let _zweite = Umgebung::neue(&[
        ("TAXGRAPH_DATEN", Some("\u{1c}\u{1f}")),
        ("XDG_DATA_HOME", Some(xdg.to_str().unwrap())),
    ]);
    assert_eq!(
        Konfig::aus_env().faelle,
        xdg.join("taxgraph").join("faelle"),
        "nur Steuerzeichen in TAXGRAPH_DATEN zaehlen wie 'nicht gesetzt'"
    );
}

// ---------------------------------------------------------------------------------------------
// Das Protokoll folgt den Akten (Gegenstueck zu `tests/test_audit_folgt_fallverzeichnis.py`)
//
// Anlass, gemessen 2026-10-01: ein Messlauf lenkte die Fallakten um und glaubte sich isoliert; das
// Protokoll lag weiter neben den echten Faellen und bekam 1145 Zeilen (alle Nutzer `dev`). In Rust
// gilt: `Konfig::aus_env` rechnet `audit_dir` aus `faelle`, solange `TAXGRAPH_AUDIT_DIR` fehlt oder
// leer ist. Faellt dieser Rueckfall (zum Beispiel auf `~/.local/share/taxgraph`), schreibt ein
// Messlauf mit `TAXGRAPH_DATEN=<Wegwerf>` ins Protokoll des Nutzers.
// ---------------------------------------------------------------------------------------------

/// Der Rueckfall selbst: ohne `TAXGRAPH_AUDIT_DIR` liegen `audit.jsonl` und `fehler.log` im
/// Fallverzeichnis, bei `TAXGRAPH_DATEN` wie bei `XDG_DATA_HOME`.
#[test]
fn ohne_audit_dir_liegt_das_protokoll_neben_den_akten() {
    let probe = tempfile::tempdir().unwrap();
    let a = probe.path().join("a");
    let xdg = probe.path().join("xdg");
    let erste = Umgebung::neue(&[
        ("TAXGRAPH_DATEN", Some(a.to_str().unwrap())),
        ("XDG_DATA_HOME", None),
        ("TAXGRAPH_AUDIT_DIR", None),
    ]);
    let k = Konfig::aus_env();
    assert_eq!(k.audit_dir, a.join("faelle"), "audit_dir folgt nicht den Faellen");
    assert_eq!(k.audit_pfad(), a.join("faelle").join("audit.jsonl"));
    assert_eq!(k.fehler_pfad(), a.join("faelle").join("fehler.log"));
    drop(erste);

    let _zweite = Umgebung::neue(&[
        ("TAXGRAPH_DATEN", None),
        ("XDG_DATA_HOME", Some(xdg.to_str().unwrap())),
        ("TAXGRAPH_AUDIT_DIR", None),
    ]);
    let k = Konfig::aus_env();
    assert_eq!(k.faelle, xdg.join("taxgraph").join("faelle"));
    assert_eq!(k.audit_dir, k.faelle, "mit XDG_DATA_HOME folgt audit_dir den Faellen nicht");
}

/// Ein gesetztes `TAXGRAPH_AUDIT_DIR` gewinnt und bewegt die Akten nicht; ein LEERES zaehlt wie
/// ungesetzt (`export TAXGRAPH_AUDIT_DIR=` im Start-Skript darf das Protokoll nicht ins aktuelle
/// Verzeichnis legen).
#[test]
fn gesetztes_audit_dir_gewinnt_und_leeres_zaehlt_wie_ungesetzt() {
    let probe = tempfile::tempdir().unwrap();
    let a = probe.path().join("a");
    let b = probe.path().join("b");
    let erste = Umgebung::neue(&[
        ("TAXGRAPH_DATEN", Some(a.to_str().unwrap())),
        ("XDG_DATA_HOME", None),
        ("TAXGRAPH_AUDIT_DIR", Some(b.to_str().unwrap())),
    ]);
    let k = Konfig::aus_env();
    assert_eq!(k.audit_dir, b, "ein gesetztes TAXGRAPH_AUDIT_DIR gewinnt nicht");
    assert_eq!(k.faelle, a.join("faelle"), "TAXGRAPH_AUDIT_DIR bewegt die Akten");
    drop(erste);

    let _zweite = Umgebung::neue(&[
        ("TAXGRAPH_DATEN", Some(a.to_str().unwrap())),
        ("XDG_DATA_HOME", None),
        ("TAXGRAPH_AUDIT_DIR", Some("")),
    ]);
    let k = Konfig::aus_env();
    assert_eq!(
        k.audit_dir,
        a.join("faelle"),
        "ein leeres TAXGRAPH_AUDIT_DIR muss wie ungesetzt wirken"
    );
}

/// Derselbe Weg am echten Aufrufort: der Dienst aus `Konfig::aus_env`, EIN `POST /fall`. Akte und
/// Protokollzeile liegen unter `TAXGRAPH_DATEN`; das (umgelenkte) Heimatverzeichnis bleibt leer.
/// Ohne den Rueckfall schriebe das Protokoll dorthin (`~/.local/share/taxgraph/faelle`).
#[test]
fn der_dienst_legt_akte_und_protokoll_an_denselben_ort() {
    use auth::Auth;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let probe = tempfile::tempdir().unwrap();
    let daten = probe.path().join("daten");
    let heim = probe.path().join("heim");
    std::fs::create_dir_all(&heim).unwrap();
    let _u = Umgebung::neue(&[
        ("TAXGRAPH_DATEN", Some(daten.to_str().unwrap())),
        ("XDG_DATA_HOME", None),
        ("TAXGRAPH_AUDIT_DIR", None),
        ("TAXGRAPH_NO_AUTH", None),
        ("HOME", Some(heim.to_str().unwrap())),
    ]);

    let konfig = Konfig::aus_env();
    let auth = Auth::neu(
        "testgeheimnis".into(),
        probe.path().join("users.json"),
        Some(konfig.audit_pfad()),
    );
    let token = auth.stelle_aus("alice").unwrap();
    let zustand = api::Zustand::neu(konfig, auth);

    let rumpf = r#"{"fall_id": "dw1", "scheibe": "ep", "veranlagungszeitraum": 2025}"#;
    let laenge = rumpf.len().to_string();
    let req = Request::builder()
        .method("POST")
        .uri("/fall")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("content-length", laenge)
        .body(Body::from(rumpf))
        .unwrap();
    let laufzeit = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (status, text) = laufzeit.block_on(async {
        let r = api::app(zustand).oneshot(req).await.unwrap();
        let (teile, body) = r.into_parts();
        let bytes = body.collect().await.unwrap().to_bytes();
        (teile.status.as_u16(), String::from_utf8(bytes.to_vec()).unwrap())
    });
    assert_eq!(status, 201, "{text}");

    let faelle = daten.join("faelle");
    let protokoll = std::fs::read_to_string(faelle.join("audit.jsonl"))
        .expect("kein Protokoll neben den Akten");
    assert!(
        protokoll
            .lines()
            .any(|z| z.contains("\"fall_angelegt\"") && z.contains("\"dw1\"") && z.contains("\"alice\"")),
        "keine Zeile fall_angelegt/dw1/alice im Protokoll: {protokoll}"
    );
    let namen: Vec<String> = std::fs::read_dir(&faelle)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert!(
        namen.iter().any(|n| n.starts_with("dw1")),
        "keine Akte dw1 neben dem Protokoll: {namen:?}"
    );
    assert_eq!(
        std::fs::read_dir(&heim).unwrap().count(),
        0,
        "das Heimatverzeichnis wurde beschrieben (das echte Nutzerprotokoll laege dort)"
    );
}
