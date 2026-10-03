//! `POST /fall/{id}/kontoauszug` mit einem PDF, wenn ein Hilfsprogramm (`pdftotext`, `pdftoppm`,
//! `tesseract`) fehlt oder mit Fehlercode endet: Python und Rust antworten auf dieselbe Anfrage mit
//! demselben Status und demselben Text (503 bzw. 422; Vault `decisions/fehlendes-hilfsprogramm-
//! antwortet-503`, `decisions/ein-hilfsprogramm-mit-fehlercode-bricht-den-upload-ab`).
//!
//! Bisher stand der Wortlaut in zwei getrennten Tests, die ihn nur per Kommentar „wortgleich“ nannten:
//! `rust/api/tests/kontoauszug_werkzeug.rs` (Rust, in-process) und `tests/test_unterprozess_zeitlimit.py`
//! (Python). Hier antworten BEIDE Server ueber HTTP, und `Paar::anfrage` vergleicht Status, Header, Body,
//! Audit und Fehlerlog.
//!
//! STUB-PATH: Anders als `eingang_werkzeug_paritaet.rs` (ein Test, `PATH` fuer den ganzen Prozess) bekommt
//! hier nur der SERVER einen eigenen `PATH`: `<tmp>/aktiv`, ein Symlink. Der Test legt ihn vor jeder
//! Anfrage auf das Verzeichnis des Falls um (Shell-Skripte, nur Builtins); jeder Aufruf eines Programms
//! sucht `PATH` neu, ein Neustart ist nicht noetig. Was im Verzeichnis fehlt, fehlt auf dem „Rechner“:
//! die echten Programme stehen nicht auf diesem `PATH`. Der Test selbst bleibt unberuehrt und darf
//! neben den anderen Tests dieser Datei laufen. Nur `python3` steht im Startverzeichnis, damit der
//! Python-Server startet.
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::{schreibe_seed, skip, Anfrage, Modus, Paar};

/// `base64("%PDF-1.4\n")`; der Inhalt zaehlt nicht, die Stubs lesen die Datei nie.
const PDF_BASE64: &str = "JVBERi0xLjQK";

/// Der Wortlaut der 503-Antwort bei fehlendem Programm.
macro_rules! fehlt {
    ($programm:literal) => {
        concat!(
            "PDF-Auslesen ist gerade nicht möglich: Das Programm '",
            $programm,
            "' fehlt auf diesem Rechner."
        )
    };
}

/// pdftotext findet keinen Text: Voll-Scan ueber `pdftoppm` und `tesseract`.
const OHNE_TEXT: &str = "exit 0";
/// pdftotext liefert eine Seite, die fuer einen Textlayer zu kurz ist: Einzelseite per OCR.
const EINE_LEERE_SEITE: &str = "printf 'x\\f'";
/// pdftoppm legt `<Praefix>-1.png` ab (das letzte Argument ist das Praefix).
const EIN_BILD: &str = "for a; do p=$a; done; : > \"$p-1.png\"";
/// Exit 1 ohne Ausgabe.
const EXIT_EINS: &str = "exit 1";

/// 422-Antwort bei `pdftoppm` mit Fehlercode.
const BILDER: &str = "Kontoauszug nicht lesbar: Die Seiten der Datei lassen sich nicht in Bilder umwandeln (beschädigt oder nicht unterstützt).";
/// 503-Antwort bei `tesseract` mit Fehlercode.
const TEXTERKENNUNG: &str = "PDF-Auslesen ist gerade nicht möglich: Die Texterkennung (tesseract) ist auf diesem Rechner nicht einsatzbereit (sie endete mit einem Fehler, etwa weil die deutschen Sprachdaten fehlen).";

struct Fall {
    name: &'static str,
    /// (Programm, Rumpf des Skripts)
    programme: &'static [(&'static str, &'static str)],
    status: u16,
    /// Muss in `fehler` stehen.
    enthaelt: &'static str,
}

const FAELLE: &[Fall] = &[
    Fall {
        name: "pdftotext fehlt",
        programme: &[],
        status: 503,
        enthaelt: fehlt!("pdftotext"),
    },
    Fall {
        name: "pdftoppm fehlt (Voll-Scan)",
        programme: &[("pdftotext", OHNE_TEXT)],
        status: 503,
        enthaelt: fehlt!("pdftoppm"),
    },
    Fall {
        name: "tesseract fehlt (Voll-Scan)",
        programme: &[("pdftotext", OHNE_TEXT), ("pdftoppm", EIN_BILD)],
        status: 503,
        enthaelt: fehlt!("tesseract"),
    },
    Fall {
        name: "pdftoppm fehlt (Einzelseite)",
        programme: &[("pdftotext", EINE_LEERE_SEITE)],
        status: 503,
        enthaelt: fehlt!("pdftoppm"),
    },
    Fall {
        name: "tesseract fehlt (Einzelseite)",
        programme: &[("pdftotext", EINE_LEERE_SEITE), ("pdftoppm", EIN_BILD)],
        status: 503,
        enthaelt: fehlt!("tesseract"),
    },
    Fall {
        name: "pdftoppm Exit 1 (Voll-Scan)",
        programme: &[("pdftotext", OHNE_TEXT), ("pdftoppm", EXIT_EINS)],
        status: 422,
        enthaelt: BILDER,
    },
    Fall {
        name: "pdftoppm Exit 1 (Einzelseite)",
        programme: &[("pdftotext", EINE_LEERE_SEITE), ("pdftoppm", EXIT_EINS)],
        status: 422,
        enthaelt: BILDER,
    },
    Fall {
        name: "tesseract Exit 1 (Voll-Scan)",
        programme: &[
            ("pdftotext", OHNE_TEXT),
            ("pdftoppm", EIN_BILD),
            ("tesseract", EXIT_EINS),
        ],
        status: 503,
        enthaelt: TEXTERKENNUNG,
    },
    Fall {
        name: "tesseract Exit 1 (Einzelseite)",
        programme: &[
            ("pdftotext", EINE_LEERE_SEITE),
            ("pdftoppm", EIN_BILD),
            ("tesseract", EXIT_EINS),
        ],
        status: 503,
        enthaelt: TEXTERKENNUNG,
    },
];

fn programme_anlegen(verz: &Path, programme: &[(&str, &str)]) {
    std::fs::create_dir_all(verz).unwrap();
    for (name, rumpf) in programme {
        let datei = verz.join(name);
        std::fs::write(&datei, format!("#!/bin/sh\n{rumpf}\n")).unwrap();
        std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// `aktiv` zeigt danach auf `ziel`: neuer Symlink, dann `rename`, nie ein Augenblick ohne `aktiv`.
fn umlegen(aktiv: &Path, ziel: &Path) {
    let neu = aktiv.with_extension("neu");
    let _ = std::fs::remove_file(&neu);
    std::os::unix::fs::symlink(ziel, &neu).unwrap();
    std::fs::rename(&neu, aktiv).unwrap();
}

/// Der erste `python3` auf dem `PATH` dieses Tests.
fn python3() -> PathBuf {
    let pfad = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&pfad)
        .map(|d| d.join("python3"))
        .find(|p| p.is_file())
        .expect("python3 nicht auf PATH")
}

/// Beide Server mit `PATH=<tmp>/aktiv`; je Fall zeigt `aktiv` auf ein anderes Programmverzeichnis.
#[test]
fn werkzeug_texte_beider_server_stimmen_ueberein() {
    if skip() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let seed = tmp.path().join("seed");
    schreibe_seed(&seed);
    let start = tmp.path().join("start");
    std::fs::create_dir_all(&start).unwrap();
    std::os::unix::fs::symlink(python3(), start.join("python3")).unwrap();
    let aktiv = tmp.path().join("aktiv");
    umlegen(&aktiv, &start);
    let pfad = aktiv.to_string_lossy().into_owned();
    let extra = [("PATH", pfad.as_str())];
    let mut p = Paar::neu_mit(tmp.path(), true, false, &seed, &extra, &extra);

    for (i, f) in FAELLE.iter().enumerate() {
        let verz = tmp.path().join(format!("bin{i}"));
        programme_anlegen(&verz, f.programme);
        umlegen(&aktiv, &verz);
        let id = format!("w{i}");
        let neu = Anfrage::neu("anlegen", "POST", "/fall")
            .json(&json!({"fall_id": id, "scheibe": "gesamt", "veranlagungszeitraum": 2025}));
        p.anfrage(&neu, Modus::Voll);
        assert_eq!(p.stat.letzter, 201, "{}: Fall anlegen", f.name);

        let a = Anfrage::neu(f.name, "POST", &format!("/fall/{id}/kontoauszug"))
            .json(&json!({"format": "pdf", "inhalt": PDF_BASE64}));
        let body = p.anfrage(&a, Modus::Voll);
        let meldung = body
            .as_ref()
            .and_then(|b| b["fehler"].as_str())
            .unwrap_or_default()
            .to_owned();
        // Gegen „beide gleich falsch“: der Soll-Ausgang muss eingetreten sein. Rust steht in
        // `stat.letzter`, Python ist der Body; stimmen beide ueberein (Vergleich unten), gilt es fuer beide.
        if p.stat.letzter != f.status {
            let d = format!(
                "{}: Status {}, erwartet {}",
                f.name, p.stat.letzter, f.status
            );
            p.stat.abweichungen.push(d);
        }
        if !meldung.contains(f.enthaelt) {
            let d = format!(
                "{}: Python-Meldung enthaelt nicht {:?}: {meldung:?}",
                f.name, f.enthaelt
            );
            p.stat.abweichungen.push(d);
        }
    }
    p.zustand_vergleichen("nach den Werkzeug-Faellen");
    p.bericht("extern/werkzeug");
    println!("EXTERN extern/werkzeug: {} Faelle, Stub-PATH", FAELLE.len());
    assert!(p.stat.abweichungen.is_empty(), "{:#?}", p.stat.abweichungen);
}
