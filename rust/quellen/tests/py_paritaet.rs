//! Die drei Python-Bausteine des Crates (`quellen::py`: `strip`, `laenge`, `repr`) gegen zwei Gegenstücke:
//!
//! 1. `llm::py` (dieselben Hilfen, ein zweites Mal geschrieben, weil `llm` `rustls` und damit einen C-Bau mitzieht; siehe
//!    `src/py.rs`): beide Fassungen müssen für jede Eingabe gleich antworten, damit sie nicht still auseinanderlaufen.
//! 2. Python selbst: `tests/daten/anker_repr.tsv` hat ein einmaliger Python-Lauf erzeugt: die `erwartete_anker`-Zeilen aller echten
//!    Metadaten unter `sources/` (Python hat sie mit `repr(liste)` geschrieben) und 19 erfundene Randfälle mit Pythons `repr`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::explicit_write)]

use std::fs;
use std::path::Path;

use quellen::py;

const PROBEN: &[&str] = &[
    "",
    " ",
    "a",
    "  a  ",
    "\t\n a b \r\n",
    "\x1c\x1d\x1e\x1f x \x1f",
    "\u{a0}x\u{a0}",
    "\u{85}x\u{85}",
    "\u{2028}x\u{2029}",
    "\u{200b}x\u{200b}",
    "\u{feff}x",
    "ä ö ü § – 😀",
    "Tom & Jerry",
    "it's",
    "say \"hi\"",
    "both ' and \"",
    "back\\slash",
    "soft\u{ad}hyphen",
    "line\u{2028}sep",
    "\x01 ctrl \x7f del",
    "tab\tund\nnewline\rcr",
];

#[test]
fn strip_laenge_und_repr_antworten_wie_llm_py() {
    for s in PROBEN {
        assert_eq!(py::strip(s), llm::py::strip(s), "strip({s:?})");
        assert_eq!(py::laenge(s), llm::py::laenge(s), "laenge({s:?})");
        assert_eq!(py::repr_str(s), llm::py::py_repr_str(s), "repr({s:?})");
        assert_eq!(py::ist_leerraum(s.chars().next().unwrap_or('x')), llm::py::ist_leerraum(s.chars().next().unwrap_or('x')));
    }
    for c in ('\0'..='\u{3000}').chain(['\u{feff}', '\u{1f600}']) {
        assert_eq!(py::ist_leerraum(c), llm::py::ist_leerraum(c), "ist_leerraum({c:?})");
    }
}

#[test]
fn die_repr_der_anker_ist_die_von_python() {
    let tsv = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/daten/anker_repr.tsv")).unwrap();
    let mut n = 0;
    for zeile in tsv.lines() {
        let (eintraege, erwartet) = zeile.split_once('\t').unwrap();
        let liste: Vec<String> = if erwartet == "[]" { vec![] } else { eintraege.split('\x1f').map(str::to_owned).collect() };
        assert_eq!(py::repr_liste(&liste), erwartet, "{liste:?}");
        n += 1;
    }
    assert!(n >= 40, "die Datei ist leer oder zu klein: {n} Zeilen");
}

#[test]
fn ein_zeilenumbruch_und_ein_tab_im_anker_werden_wie_in_python_maskiert() {
    assert_eq!(py::repr_str("a\nb\tc\rd"), "'a\\nb\\tc\\rd'");
    assert_eq!(py::repr_liste(&["x\ny".to_owned()]), "['x\\ny']");
}
