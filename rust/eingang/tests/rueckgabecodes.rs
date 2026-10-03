//! Rückgabecodes von `pdftoppm` und `tesseract` beim Kontoauszug (Vault `decisions/ein-hilfsprogramm-
//! mit-fehlercode-bricht-den-upload-ab`). Vorher gab der Leser bei Exit != 0 still `("", {})` zurück
//! (Voll-Scan) oder `KeinBild` (Einzelseite); jetzt ist jeder Exit != 0 ein Fehler, in beiden Wegen.
//! Die Gegenstücke in Python stehen in `tests/test_kontoauszug_writer.py`.
//!
//! HERMETISCH: kein echtes `pdftotext`/`pdftoppm`/`tesseract` nötig. Je Fall zeigt `PATH` auf ein
//! Verzeichnis mit Shell-Skripten, die nur Builtins nutzen. `PATH` gilt für den ganzen Prozess, darum
//! steht alles in EINEM Test (kein zweiter Test in dieser Datei, der parallel liefe).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use eingang::ocr::{lies_beleg_text, lies_kontoauszug_pdf, ConfMap, OcrFehler};

/// pdftotext findet keinen Text: Voll-Scan über `pdftoppm` und `tesseract`.
const VOLL: &str = "exit 0";
/// pdftotext liefert eine Seite, die für einen Textlayer zu kurz ist: Einzelseite per OCR.
const EINZEL: &str = "printf 'x\\f'";
/// pdftoppm legt `<Präfix>-1.png` ab (das letzte Argument ist das Präfix).
const BILD: &str = "for a; do p=$a; done; : > \"$p-1.png\"";
/// pdftoppm endet mit Exit 1 und legt KEIN Bild ab (so gemessen).
const PDFTOPPM_FEHLER: &str = "exit 1";
/// pdftoppm legt ein Bild ab und endet trotzdem mit Exit 1: auch das ist ein Fehler.
const PDFTOPPM_FEHLER_MIT_BILD: &str = "for a; do p=$a; done; : > \"$p-1.png\"; exit 1";
/// tesseract auf einer weißen Seite: Exit 0, nur die Kopfzeile.
const WEISSE_SEITE: &str = "printf 'level\\tpage_num\\tblock_num\\tpar_num\\tline_num\\tword_num\\tleft\\ttop\\twidth\\theight\\tconf\\ttext\\n'";
/// tesseract mit Exit 0 und einem erkannten Wort.
const EIN_WORT: &str = "printf 'level\\tpage_num\\tblock_num\\tpar_num\\tline_num\\tword_num\\tleft\\ttop\\twidth\\theight\\tconf\\ttext\\n5\\t1\\t1\\t1\\t1\\t1\\t0\\t0\\t10\\t10\\t96\\tMiete\\n'";
/// tesseract ohne `deu`-Daten: Exit 1, stdout leer (so gemessen).
const TESSERACT_FEHLER: &str = "exit 1";

type Ergebnis = Result<(String, ConfMap), OcrFehler>;

enum Erwartung {
    Text(&'static str),
    BildUmwandlung,
    OcrNichtVerfuegbar,
    KeinBild,
}

struct Fall {
    name: &'static str,
    /// Rümpfe der drei Skripte: pdftotext, pdftoppm, tesseract.
    skripte: [&'static str; 3],
    kontoauszug: Erwartung,
    /// Der Beleg-Leser bleibt, wie er war (Entscheidung Punkt 5); `None` = nicht geprüft.
    beleg: Option<Erwartung>,
}

fn stubs_anlegen(verz: &Path, skripte: &[&str; 3]) {
    std::fs::create_dir_all(verz).unwrap();
    for (name, rumpf) in ["pdftotext", "pdftoppm", "tesseract"].iter().zip(skripte) {
        let datei = verz.join(name);
        std::fs::write(&datei, format!("#!/bin/sh\n{rumpf}\n")).unwrap();
        std::fs::set_permissions(&datei, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn pruefe(name: &str, was: &str, ist: &Ergebnis, soll: &Erwartung) {
    let ok = match (ist, soll) {
        (Ok((text, _)), Erwartung::Text(t)) => text == t,
        (Err(OcrFehler::BildUmwandlung), Erwartung::BildUmwandlung)
        | (Err(OcrFehler::OcrNichtVerfuegbar), Erwartung::OcrNichtVerfuegbar)
        | (Err(OcrFehler::KeinBild), Erwartung::KeinBild) => true,
        _ => false,
    };
    assert!(ok, "{name} ({was}): unerwartetes Ergebnis {ist:?}");
}

#[test]
fn fehlercode_von_pdftoppm_und_tesseract_bricht_den_kontoauszug_ab() {
    use Erwartung::{BildUmwandlung, KeinBild, OcrNichtVerfuegbar, Text};
    let faelle = [
        // --- pdftoppm mit Exit != 0: PdfNichtLesbar-Gegenstück, beide Wege, mit und ohne Bild
        Fall {
            name: "pdftoppm Exit 1, Voll-Scan",
            skripte: [VOLL, PDFTOPPM_FEHLER, WEISSE_SEITE],
            kontoauszug: BildUmwandlung,
            beleg: None,
        },
        Fall {
            name: "pdftoppm Exit 1 mit Bild, Voll-Scan",
            skripte: [VOLL, PDFTOPPM_FEHLER_MIT_BILD, WEISSE_SEITE],
            kontoauszug: BildUmwandlung,
            beleg: None,
        },
        Fall {
            name: "pdftoppm Exit 1, Einzelseite",
            skripte: [EINZEL, PDFTOPPM_FEHLER, WEISSE_SEITE],
            kontoauszug: BildUmwandlung,
            // Der Beleg-Leser prüft den Code nicht: ohne Bild bleibt es beim alten KeinBild.
            beleg: Some(KeinBild),
        },
        Fall {
            name: "pdftoppm Exit 1 mit Bild, Einzelseite",
            skripte: [EINZEL, PDFTOPPM_FEHLER_MIT_BILD, WEISSE_SEITE],
            kontoauszug: BildUmwandlung,
            beleg: None,
        },
        // --- tesseract mit Exit != 0: OcrNichtVerfuegbar-Gegenstück, beide Wege
        Fall {
            name: "tesseract Exit 1, Voll-Scan",
            skripte: [VOLL, BILD, TESSERACT_FEHLER],
            kontoauszug: OcrNichtVerfuegbar,
            beleg: None,
        },
        Fall {
            name: "tesseract Exit 1, Einzelseite",
            skripte: [EINZEL, BILD, TESSERACT_FEHLER],
            kontoauszug: OcrNichtVerfuegbar,
            // Der Beleg-Leser liest den Code nicht: leere Ausgabe ist für ihn ein leeres Ergebnis.
            beleg: Some(Text("")),
        },
        // --- Exit 0 bleibt unverändert
        Fall {
            name: "weiße Seite (tesseract Exit 0, nur Kopfzeile), Voll-Scan",
            skripte: [VOLL, BILD, WEISSE_SEITE],
            kontoauszug: Text(""),
            beleg: None,
        },
        Fall {
            name: "weiße Seite (tesseract Exit 0, nur Kopfzeile), Einzelseite",
            skripte: [EINZEL, BILD, WEISSE_SEITE],
            kontoauszug: Text(""),
            beleg: None,
        },
        Fall {
            name: "Exit 0 mit Wort, Voll-Scan",
            skripte: [VOLL, BILD, EIN_WORT],
            kontoauszug: Text("Miete"),
            beleg: None,
        },
        Fall {
            name: "Exit 0 mit Wort, Einzelseite",
            skripte: [EINZEL, BILD, EIN_WORT],
            kontoauszug: Text("Miete"),
            beleg: None,
        },
    ];

    let tmp = tempfile::tempdir().unwrap();
    // Eine nicht vorhandene Datei: die Skripte lesen sie nie.
    let pdf = tmp.path().join("x.pdf").to_string_lossy().into_owned();
    for (i, fall) in faelle.iter().enumerate() {
        let bin = tmp.path().join(format!("bin{i}"));
        stubs_anlegen(&bin, &fall.skripte);
        std::env::set_var("PATH", &bin);

        pruefe(
            fall.name,
            "Kontoauszug",
            &lies_kontoauszug_pdf(&pdf),
            &fall.kontoauszug,
        );
        if let Some(soll) = &fall.beleg {
            pruefe(fall.name, "Beleg", &lies_beleg_text(&pdf), soll);
        }
    }
}
