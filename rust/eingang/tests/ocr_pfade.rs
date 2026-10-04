//! PDF-Text und OCR (`eingang::ocr`) gegen die Antworten des PYTHON-Orakels, hermetisch: `PATH` zeigt je
//! Fall auf ein Verzeichnis mit drei Shell-Skripten (`pdftotext`, `pdftoppm`, `tesseract`, nur
//! Builtins), die dieselben Skripte laufen unter Python (`tools/parity/extract_eingang_orakel.py`) und
//! hier. Die Faelle stehen in `rust/fixtures/eingang_orakel.json`, Abschnitt `ocr`.
//!
//! Sie decken die Entscheidungsstellen des Lesers: Seitendeckel 40/41 (Voll-Scan und gemischt),
//! Plausibilitaet ab 20 Zeichen nach `strip`, Seitennummer und Zeilenversatz im gemischten PDF,
//! Konfidenz je Zeile (Minimum, 0, unlesbar, leer), pdftotext-Exit 0/3/sonst, BEL im Textlayer,
//! `.txt` in jeder Schreibweise, Zeilenenden, `OMP_THREAD_LIMIT` nur fuer tesseract.
//!
//! `PATH` und `OMP_THREAD_LIMIT` gelten fuer den ganzen Prozess, darum steht alles in EINEM Test (kein
//! zweiter Test in dieser Datei, der parallel liefe).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use eingang::ocr::{
    lies_beleg_text, lies_kontoauszug_pdf, ConfMap, OcrFehler, OCR_SEITEN_HOECHSTZAHL,
    PDFTOPPM_ZEITLIMIT, PDFTOTEXT_ZEITLIMIT, TESSERACT_ZEITLIMIT,
};
use serde_json::{json, Map, Value};

const FIXTURE: &str = include_str!("../../fixtures/eingang_orakel.json");

/// Die Antwort des Lesers in der Form des Orakels: Fehler auf Klasse und Text, die Konfidenz je
/// Zeilenindex mit dem Index als Text (JSON-Schluessel).
fn ocr_json(r: Result<(String, ConfMap), OcrFehler>) -> Value {
    match r {
        Ok((text, conf)) => {
            let conf: Map<String, Value> = conf
                .into_iter()
                .map(|(i, c)| (i.to_string(), json!(c)))
                .collect();
            json!({"ok": [text, conf]})
        }
        Err(e) => {
            let klasse = match &e {
                OcrFehler::ZuAufwendig(_) => "OcrZuAufwendig",
                OcrFehler::NichtLesbar | OcrFehler::BildUmwandlung => "PdfNichtLesbar",
                OcrFehler::OcrNichtVerfuegbar => "OcrNichtVerfuegbar",
                // Python wirft `IndexError` beim Zugriff auf das erste Bild und hat keinen Text dazu.
                OcrFehler::KeinBild => return json!({"err": "IndexError"}),
                andere => panic!("unerwarteter Fehler {andere:?}"),
            };
            json!({"err": klasse, "msg": e.to_string()})
        }
    }
}

/// Wie `ocr_json`, fuer den Orakelwert: `IndexError` traegt dort Pythons Meldung, die niemand nachbaut.
fn py_norm(mut v: Value) -> Value {
    if v["err"] == "IndexError" {
        v.as_object_mut().unwrap().remove("msg");
    }
    v
}

fn stubs_anlegen(verz: &Path, skripte: &[Value]) {
    std::fs::create_dir_all(verz).unwrap();
    for (name, rumpf) in ["pdftotext", "pdftoppm", "tesseract"].iter().zip(skripte) {
        let datei = verz.join(name);
        std::fs::write(&datei, format!("#!/bin/sh\n{}\n", rumpf.as_str().unwrap())).unwrap();
        std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn pdf_und_ocr_lesen_wie_python_mit_denselben_hilfsprogrammen() {
    let fix: Value = serde_json::from_str(FIXTURE).unwrap();

    // Die Zahlen sind Pythons (`kontoauszug_writer.py:44-58`, `beleg_writer.py:43-53`): gleich in beiden.
    let k = &fix["ocr_konstanten"];
    assert_eq!(k["kontoauszug"], k["beleg"]);
    assert_eq!(
        k["kontoauszug"],
        json!([
            PDFTOTEXT_ZEITLIMIT.as_secs(),
            PDFTOPPM_ZEITLIMIT.as_secs(),
            TESSERACT_ZEITLIMIT.as_secs(),
            OCR_SEITEN_HOECHSTZAHL
        ])
    );
    assert_eq!(PDFTOTEXT_ZEITLIMIT, Duration::from_secs(30));

    let faelle = fix["ocr"].as_array().unwrap();
    assert!(faelle.len() >= 60, "Orakel hat nur {} Faelle", faelle.len());

    let tmp = tempfile::tempdir().unwrap();
    let pdf = tmp.path().join("x.pdf").to_string_lossy().into_owned();
    let alt_path = std::env::var_os("PATH");
    let alt_omp = std::env::var_os("OMP_THREAD_LIMIT");
    // tesseract setzt 1, alle anderen Programme erben 7.
    std::env::set_var("OMP_THREAD_LIMIT", "7");

    let mut abweichungen = Vec::new();
    let mut verglichen = 0;
    for (i, fall) in faelle.iter().enumerate() {
        let name = fall["name"].as_str().unwrap();
        let verz = tmp.path().join(format!("bin{i}"));
        stubs_anlegen(&verz, fall["stubs"].as_array().unwrap());
        std::env::set_var("PATH", &verz);

        let (beleg_pfad, nur_beleg) = match fall["datei"].as_array() {
            Some(d) => {
                let dir = tmp.path().join(format!("datei{i}"));
                std::fs::create_dir(&dir).unwrap();
                let p = dir.join(d[0].as_str().unwrap());
                std::fs::write(&p, d[1].as_str().unwrap()).unwrap();
                (p.to_string_lossy().into_owned(), true)
            }
            None => (pdf.clone(), false),
        };
        let mut vergleiche = vec![(
            "beleg",
            ocr_json(lies_beleg_text(&beleg_pfad)),
            &fall["py"]["beleg"],
        )];
        if !nur_beleg {
            vergleiche.push((
                "kontoauszug",
                ocr_json(lies_kontoauszug_pdf(&pdf)),
                &fall["py"]["konto"],
            ));
        }
        for (leser, rust, py) in vergleiche {
            verglichen += 1;
            if rust != py_norm(py.clone()) {
                abweichungen.push(format!("{name} [{leser}]\n  rust: {rust}\n  py:   {py}"));
            }
        }
    }

    match alt_path {
        Some(p) => std::env::set_var("PATH", p),
        None => std::env::remove_var("PATH"),
    }
    match alt_omp {
        Some(v) => std::env::set_var("OMP_THREAD_LIMIT", v),
        None => std::env::remove_var("OMP_THREAD_LIMIT"),
    }
    assert!(verglichen >= 100, "nur {verglichen} Vergleiche");
    assert!(
        abweichungen.is_empty(),
        "{} von {verglichen} weichen von Python ab:\n{}",
        abweichungen.len(),
        abweichungen.join("\n")
    );
}
