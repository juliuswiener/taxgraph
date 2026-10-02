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
