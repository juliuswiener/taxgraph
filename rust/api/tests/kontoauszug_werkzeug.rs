//! `POST /fall/{id}/kontoauszug` mit einem PDF, wenn ein Hilfsprogramm (`pdftotext`, `pdftoppm`,
//! `tesseract`) fehlt: 503 mit Klartext statt 500 mit dem Namen einer Ausnahme (Vault
//! `decisions/fehlendes-hilfsprogramm-antwortet-503`). Die Gegenstücke in Python stehen in
//! `tests/test_unterprozess_zeitlimit.py` (`test_endpunkt_fehlendes_werkzeug_wirft_apierror_503`).
//!
//! HERMETISCH: kein echtes `pdftotext`/`tesseract` nötig. Je Fall zeigt `PATH` auf ein Verzeichnis
//! mit Shell-Skripten, die nur Builtins nutzen. `PATH` und `TMPDIR` gelten für den ganzen Prozess,
//! darum steht alles in EINEM Test (kein zweiter Test in dieser Datei, der parallel liefe).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

/// `base64("%PDF-1.4\n")`; der Inhalt zählt nicht, die Stubs lesen die Datei nie.
const PDF_BASE64: &str = "JVBERi0xLjQK";

/// Der Wortlaut der 503-Antwort; wortgleich mit `tests/test_unterprozess_zeitlimit.py`.
macro_rules! fehlt {
    ($programm:literal) => {
        concat!(
            "PDF-Auslesen ist gerade nicht möglich: Das Programm '",
            $programm,
            "' fehlt auf diesem Rechner."
        )
    };
}

/// pdftotext findet keinen Text: Voll-Scan über `pdftoppm` und `tesseract`.
const OHNE_TEXT: &str = "exit 0";
/// pdftotext liefert eine Seite, die für einen Textlayer zu kurz ist: Einzelseite per OCR.
const EINE_LEERE_SEITE: &str = "printf 'x\\f'";
/// pdftoppm legt `<Präfix>-1.png` ab (das letzte Argument ist das Präfix).
const EIN_BILD: &str = "for a; do p=$a; done; : > \"$p-1.png\"";

struct Dienst {
    zustand: Zustand,
    token: String,
    tmp: tempfile::TempDir,
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
        tmp,
    }
}

async fn sende(d: &Dienst, pfad: &str, body: &Value) -> (u16, Value) {
    let text = body.to_string();
    let req = Request::builder()
        .method("POST")
        .uri(pfad)
        .header("authorization", format!("Bearer {}", d.token))
        .header("content-type", "application/json")
        .header("content-length", text.len().to_string())
        .body(Body::from(text))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (
        teile.status.as_u16(),
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Ein PDF-Auszug an einen frischen Fall `id`.
async fn pdf_auszug(d: &Dienst, id: &str) -> (u16, Value) {
    let rumpf = json!({"fall_id": id, "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "/fall", &rumpf).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    sende(
        d,
        &format!("/fall/{id}/kontoauszug"),
        &json!({"format": "pdf", "inhalt": PDF_BASE64}),
    )
    .await
}

/// Legt die Programme in `verz` ab. `modus` 0o755 = lauffähig, 0o644 = vorhanden, aber ohne Rechte.
fn stubs_anlegen(verz: &Path, stubs: &[(&str, &str, u32)]) {
    std::fs::create_dir_all(verz).unwrap();
    for (name, rumpf, modus) in stubs {
        let datei = verz.join(name);
        std::fs::write(&datei, format!("#!/bin/sh\n{rumpf}\n")).unwrap();
        std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(*modus)).unwrap();
    }
}

struct Fall {
    name: &'static str,
    /// (Programm, Rumpf des Skripts, Dateimodus)
    stubs: &'static [(&'static str, &'static str, u32)],
    status: u16,
    /// Muss in der Meldung stehen.
    enthaelt: &'static str,
}

const FAELLE: &[Fall] = &[
    Fall {
        name: "pdftotext fehlt",
        stubs: &[],
        status: 503,
        enthaelt: fehlt!("pdftotext"),
    },
    Fall {
        name: "pdftoppm fehlt (Voll-Scan)",
        stubs: &[("pdftotext", OHNE_TEXT, 0o755)],
        status: 503,
        enthaelt: fehlt!("pdftoppm"),
    },
    Fall {
        name: "tesseract fehlt (Voll-Scan)",
        stubs: &[
            ("pdftotext", OHNE_TEXT, 0o755),
            ("pdftoppm", EIN_BILD, 0o755),
        ],
        status: 503,
        enthaelt: fehlt!("tesseract"),
    },
    Fall {
        name: "pdftoppm fehlt (Einzelseite)",
        stubs: &[("pdftotext", EINE_LEERE_SEITE, 0o755)],
        status: 503,
        enthaelt: fehlt!("pdftoppm"),
    },
    Fall {
        name: "tesseract fehlt (Einzelseite)",
        stubs: &[
            ("pdftotext", EINE_LEERE_SEITE, 0o755),
            ("pdftoppm", EIN_BILD, 0o755),
        ],
        status: 503,
        enthaelt: fehlt!("tesseract"),
    },
    // Gegenprobe: nur ein FEHLENDES Programm ist ein Betriebsproblem. Eines, das da ist und nicht
    // starten darf (Pythons PermissionError), bleibt ein 500 — kein Catch-all.
    Fall {
        name: "pdftotext ohne Ausfuehrungsrechte",
        stubs: &[("pdftotext", OHNE_TEXT, 0o644)],
        status: 500,
        enthaelt: "OSError",
    },
];

#[tokio::test]
async fn fehlendes_hilfsprogramm_antwortet_503_und_laesst_nichts_liegen() {
    let d = dienst();
    // Temp-Dateien und -Verzeichnisse des Lesepfads landen hier: nach jedem Fall muss es leer sein.
    let spuren = d.tmp.path().join("spuren");
    std::fs::create_dir_all(&spuren).unwrap();
    std::env::set_var("TMPDIR", &spuren);

    for (i, fall) in FAELLE.iter().enumerate() {
        let bin = d.tmp.path().join(format!("bin{i}"));
        stubs_anlegen(&bin, fall.stubs);
        std::env::set_var("PATH", &bin);

        let (status, antwort) = pdf_auszug(&d, &format!("w{i}")).await;
        let meldung = antwort["fehler"].as_str().unwrap_or_default();

        assert_eq!(status, fall.status, "{}: {antwort}", fall.name);
        assert!(
            meldung.contains(fall.enthaelt),
            "{}: Meldung enthält nicht {:?}: {meldung}",
            fall.name,
            fall.enthaelt
        );
        if fall.status == 503 {
            assert!(
                !meldung.contains("FileNotFoundError") && !meldung.contains("os error"),
                "{}: Ausnahme-Typ oder errno in der Antwort an den Nutzer: {meldung}",
                fall.name
            );
        }
        let uebrig: Vec<_> = std::fs::read_dir(&spuren)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(
            uebrig.is_empty(),
            "{}: temporäre Datei(en) liegen geblieben: {uebrig:?}",
            fall.name
        );
    }
}
