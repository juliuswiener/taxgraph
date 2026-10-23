//! Die Hauptfluesse der Oberflaeche im Browser, gegen den echten Rust-Dienst auf Wegwerf-Daten (E9, Nr. 72).
//!
//! Ersetzt, was `tests/test_ui_*.py` (Playwright) pruefen und mit der Loeschung von `tests/` (L2) verloren ginge.
//! Die Zuordnung JEDER der 231 Python-Funktionen zu einem Pruefpunkt hier, einem bestehenden Rust-Test oder einem
//! benannten Verlust steht in `ZUORDNUNG.tsv` neben dieser Datei; `zuordnung.rs` haelt sie gegen den Quelltext.
//! Ein Pruefpunkt ist ein `pp!("<Fluss>.<python_funktion>", bedingung, ...)`: dieselbe Behauptung wie im Python-Test.
//!
//! Diese Tests brauchen Chromium und `chromedriver` (`CHROME_BIN`, `CHROMEDRIVER`, sonst `/usr/bin/chromium`
//! und `/usr/bin/chromedriver`). Fehlt einer, ist der Test ROT. `TAXGRAPH_OHNE_BROWSER=1` erlaubt lokal das Ueberspringen
//! (der Test endet dann gruen, mit der Zeile `UEBERSPRUNGEN`, die der Zaehlschritt der CI nennt); die CI setzt es seit
//! 2026-10-15 NICHT mehr (Julius, d-8): ein Runner ohne Browser macht den Lauf rot, statt die Tests still zu uebergehen.
//! GEMESSEN im Lauf 38459954734 (2026-10-14): der Runner bringt Chrome und `chromedriver` mit, alle 32 Tests liefen
//! in 102,83 s ohne Skip-Zeile; eine gruene CI fuehrt sie also aus. Lokal laufen sie
//! mit `cargo test -p api --test oberflaeche` oder `cargo nextest run -p api -E 'binary(oberflaeche)'`, seriell (ein Browser,
//! ein Dienst zugleich; unter nextest haelt die Gruppe `oberflaeche-browser` in `rust/.config/nextest.toml` das ein, weil
//! jeder Test dort einen eigenen Prozess hat), ca. 0,5 bis 1 GB je Lauf.
//!
//! Keine neue Abhaengigkeit: `browser.rs` ist ein WebDriver-Client aus `std::net` und `serde_json`.
//! Kein ELSTER-Aufruf, keine Hersteller-ID, kein Zugriff auf `~/.local/share/taxgraph`: der Dienst bekommt eine leere
//! Umgebung und Wegwerf-Verzeichnisse (`dienst.rs`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::format_push_string,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::manual_assert,
    clippy::items_after_statements,
    dead_code
)]

/// Ein Pruefpunkt: wie `assert!`, mit der Kennung des Python-Tests vorne. Mit `OBERFLAECHE_PP_LOG=<datei>` haengt er die
/// Kennung vorher an die Datei (Messung: welche Pruefpunkte lief ein Durchlauf wirklich).
macro_rules! pp {
    ($id:literal, $bed:expr, $($msg:tt)+) => {{
        $crate::pp_erfasst($id);
        let ok: bool = $bed;
        assert!(ok, "[{}] {}", $id, format!($($msg)+));
    }};
}

/// Beendet den Test sofort, wenn `TAXGRAPH_OHNE_BROWSER=1` das Ueberspringen erlaubt und kein Browser da ist.
macro_rules! umgebung_oder_ende {
    ($opt:expr, $b:expr, $h:expr) => {
        match $crate::seite::Umgebung::neu(&$opt, $b, $h) {
            Some(u) => u,
            None => return,
        }
    };
}

mod bank_vorschlaege_pruefen;
mod browser;
mod chat;
mod darstellung;
mod daten_dateien;
mod dienst;
mod fluss_a;
mod fluss_b;
mod fluss_c;
mod fluss_d;
mod fluss_e;
mod fluss_f;
mod fluss_g;
mod fluss_h;
mod fluss_i;
mod lohnersatz_liste;
mod fluss_j;
mod konto_ersetzen;
mod konto_zeichensatz;
mod seite;
mod zuordnung;

fn pp_erfasst(id: &str) {
    if let Ok(pfad) = std::env::var("OBERFLAECHE_PP_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(pfad) {
            let _ = writeln!(f, "{id}");
        }
    }
}
