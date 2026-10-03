//! Paritaet bei Rueckgabecodes von `pdftoppm` und `tesseract` (Vault `decisions/ein-hilfsprogramm-mit-
//! fehlercode-bricht-den-upload-ab`): derselbe Stub-Fall ergibt in `rust/eingang` und in
//! `produkt/eingang/kontoauszug_writer.py` dieselbe Art Fehler mit demselben Text (`PdfNichtLesbar` fuer
//! `pdftoppm`, `OcrNichtVerfuegbar` fuer `tesseract`) — oder dasselbe Ergebnis bei Exit 0, auch auf einer
//! weissen Seite.
//!
//! WARUM EINE EIGENE DATEI: `PATH` gilt fuer den ganzen Prozess. `eingang_paritaet.rs` laeuft mit mehreren
//! Faeden und liest in `pdf_ocr` die ECHTEN Werkzeuge; ein Stub-Verzeichnis auf `PATH` wuerde dort
//! mitten im Lauf die falschen Programme finden. Hier steht darum alles in EINEM Test und der Prozess hat
//! nur die Stubs. Das Orakel (Python) erbt `PATH` und `STUB_MODUS` beim Start und sieht dieselben Stubs;
//! welches Verhalten sie zeigen, steht je Fall in der Datei `STUB_MODUS` (`pdftotext pdftoppm tesseract`).
//!
//!   `PARITY`=1 `cargo` test -p parity --test `eingang_werkzeug_paritaet`
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use eingang::ocr::{lies_kontoauszug_pdf, ConfMap, OcrFehler};
use parity::Oracle;
use serde_json::{json, Value};

/// Die drei Skripte lesen ihr Verhalten aus `$STUB_MODUS` (eine Zeile, drei Woerter).
/// pdftotext: `leer` = kein Text (Voll-Scan), `seite` = eine zu kurze Seite (Einzelseite).
const PDFTOTEXT: &str = r#"read t p s < "$STUB_MODUS"
case "$t" in seite) printf 'x\f';; esac
exit 0"#;
/// pdftoppm: `ok` = legt `<Praefix>-1.png` ab, `fehler` = Exit 1 ohne Bild, `fehler_mit_bild` = Bild und Exit 1.
const PDFTOPPM: &str = r#"read t p s < "$STUB_MODUS"
for a; do last=$a; done
case "$p" in
  ok) : > "$last-1.png";;
  fehler_mit_bild) : > "$last-1.png"; exit 1;;
  fehler) exit 1;;
esac
exit 0"#;
/// tesseract: `weiss` = nur die TSV-Kopfzeile, `text` = ein Wort, `fehler` = Exit 1 mit leerem stdout.
const TESSERACT: &str = r#"read t p s < "$STUB_MODUS"
case "$s" in
  weiss) printf 'level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n';;
  text) printf 'level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n5\t1\t1\t1\t1\t1\t0\t0\t10\t10\t96\tMiete\n';;
  fehler) exit 1;;
esac
exit 0"#;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn ocr_json(r: &Result<(String, ConfMap), OcrFehler>) -> Value {
    match r {
        Ok((t, c)) => json!({"ok": [t, c]}),
        Err(e @ (OcrFehler::NichtLesbar | OcrFehler::BildUmwandlung)) => {
            json!({"err": "PdfNichtLesbar", "msg": e.to_string()})
        }
        Err(e @ OcrFehler::OcrNichtVerfuegbar) => {
            json!({"err": "OcrNichtVerfuegbar", "msg": e.to_string()})
        }
        Err(e) => json!({"err": format!("{e:?}")}),
    }
}

fn stubs_anlegen(verz: &Path) {
    std::fs::create_dir_all(verz).unwrap();
    for (name, rumpf) in [
        ("pdftotext", PDFTOTEXT),
        ("pdftoppm", PDFTOPPM),
        ("tesseract", TESSERACT),
    ] {
        let datei = verz.join(name);
        std::fs::write(&datei, format!("#!/bin/sh\n{rumpf}\n")).unwrap();
        std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// (Name, pdftotext, pdftoppm, tesseract, erwartete Art: `Some(Fehlername)` oder `None` = Ergebnis)
type Fall = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
);

const FAELLE: &[Fall] = &[
    (
        "pdftoppm Exit 1, Voll-Scan",
        "leer",
        "fehler",
        "weiss",
        Some("PdfNichtLesbar"),
    ),
    (
        "pdftoppm Exit 1 mit Bild, Voll-Scan",
        "leer",
        "fehler_mit_bild",
        "weiss",
        Some("PdfNichtLesbar"),
    ),
    (
        "pdftoppm Exit 1, Einzelseite",
        "seite",
        "fehler",
        "weiss",
        Some("PdfNichtLesbar"),
    ),
    (
        "pdftoppm Exit 1 mit Bild, Einzelseite",
        "seite",
        "fehler_mit_bild",
        "weiss",
        Some("PdfNichtLesbar"),
    ),
    (
        "tesseract Exit 1, Voll-Scan",
        "leer",
        "ok",
        "fehler",
        Some("OcrNichtVerfuegbar"),
    ),
    (
        "tesseract Exit 1, Einzelseite",
        "seite",
        "ok",
        "fehler",
        Some("OcrNichtVerfuegbar"),
    ),
    ("weisse Seite, Voll-Scan", "leer", "ok", "weiss", None),
    ("weisse Seite, Einzelseite", "seite", "ok", "weiss", None),
    ("Exit 0 mit Wort, Voll-Scan", "leer", "ok", "text", None),
    ("Exit 0 mit Wort, Einzelseite", "seite", "ok", "text", None),
];

#[test]
fn rueckgabecodes_beider_sprachen_stimmen_ueberein() {
    if skip() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    stubs_anlegen(&bin);
    let modus = tmp.path().join("modus");
    // Die Stubs vor die echten Programme; der Rest von `PATH` bleibt, damit `python3` startet.
    let alt = std::env::var_os("PATH").unwrap_or_default();
    let neu = std::env::join_paths(std::iter::once(bin.clone()).chain(std::env::split_paths(&alt)))
        .unwrap();
    std::env::set_var("PATH", neu);
    std::env::set_var("STUB_MODUS", &modus);
    // Erst NACH dem Setzen starten: das Orakel erbt beide Variablen.
    let mut orakel = Oracle::spawn(&repo_root()).expect("oracle.py startet");

    let pdf = tmp.path().join("x.pdf").to_string_lossy().into_owned();
    let mut abweichungen = Vec::new();
    for (name, t, p, s, art) in FAELLE {
        std::fs::write(&modus, format!("{t} {p} {s}\n")).unwrap();
        let rust = ocr_json(&lies_kontoauszug_pdf(&pdf));
        let antwort = orakel
            .call_json(&json!({"fn": "schritt8.eingang.pdf", "pfad": pdf}))
            .expect("orakel antwortet");
        let python = &antwort["ok"]["konto"];
        if rust != *python {
            abweichungen.push(format!("{name}\n  rust: {rust}\n  py:   {python}"));
        }
        // Gegen "beide gleich falsch": die erwartete Art muss tatsaechlich eingetreten sein.
        let ist = python.get("err").and_then(Value::as_str);
        assert_eq!(
            ist, *art,
            "{name}: Python lieferte nicht die erwartete Art: {python}"
        );
    }
    assert!(
        abweichungen.is_empty(),
        "{} von {} Faellen weichen ab:\n{}",
        abweichungen.len(),
        FAELLE.len(),
        abweichungen.join("\n")
    );
    println!(
        "eingang_werkzeug_paritaet: {} Faelle, 0 Abweichungen",
        FAELLE.len()
    );
}
