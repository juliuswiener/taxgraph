//! E2E: eine Fall-Datei bis zum ELSTER-XML, Byte fuer Byte wie Python.
//!
//! Unter `rust/fixtures/e2e/` liegen je Fall die Fall-Datei `<fall>.json` und `<fall>.xml`, das
//! XML, das `api.einreichen` fuer denselben Fall in die ERiC-Pruefung gaebe. Beides schrieb ein Python-Skript
//! ueber den Nutzerpfad (`fall_anlegen`, `event`, `einreichen`); die Dateien sind eingefroren, der Erzeuger
//! ist geloescht (Verlauf: `git show 2dd056a6:tools/parity/e2e_faelle.py`). Der Test laedt die Fall-Datei wie der Server (`store::lade`), ruft [`einreichungs_xml`] und verlangt:
//!
//! 1. das XML gleich der Python-Datei, Byte fuer Byte,
//! 2. das XML gueltig gegen das amtliche Schema (`elster11_E10_<vz>_extern.xsd`, xmllint).
//!
//! # Ohne Schema rot
//!
//! Beide Schemas liegen nur in der lokalen ERiC-Auslieferung (`$ERIC_DIR`, sonst
//! `~/02_Software/eric`), nie im Repo: `E10-<vz>.xsd` braucht schon der Writer, das
//! `extern`-XSD braucht xmllint. Fehlt eines, scheitert jeder Fall und nennt die Datei. Nur
//! `TAXGRAPH_OHNE_XSD=1` erlaubt das Fehlen (die CI hat keine ERiC-Auslieferung). Dann meldet jeder
//! Fall den Verzicht auf stderr, auch ohne `--nocapture`, und prueft nur, dass die Komposition erst
//! am Writer haelt: Guard und Deklaration liefen durch, das XML ist ungeprueft. Liegen die Schemas,
//! prueft der Test voll, gleich ob die Variable gesetzt ist.
//!
//! Die Dateien werden nicht neu erzeugt. Aendert ein Rust-eigenes Feld das XML, steht die Abweichung mit
//! Grund in der Abweichungsliste (`rust/fixtures/README.md`); die `.xml` wird von Hand gepflegt und lokal
//! gegen die XSD geprueft.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use bescheid::deklaration::{einreichungs_xml, EinreichFehler};
use bescheid::testhilfe::{index, params};
use elster::testhilfe::schemas_da;

fn fixture(datei: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/e2e")
        .join(datei)
}

fn pruefe(fall: &str) {
    let st = store::Store::aus_datei(store::lade(&fixture(&format!("{fall}.json"))).unwrap());
    let ergebnis = einreichungs_xml(&st, index(), params(), "BY", Some("74931".to_owned()));
    if !schemas_da(st.veranlagungszeitraum()) {
        assert!(
            matches!(&ergebnis, Err(EinreichFehler::XmlNichtBaubar(f)) if f.0.contains("nicht gefunden")),
            "{fall}: {ergebnis:?}"
        );
        return;
    }
    let e = ergebnis.unwrap_or_else(|f| panic!("{fall}: {f}"));
    let python = std::fs::read_to_string(fixture(&format!("{fall}.xml"))).unwrap();
    if e.xml != python {
        let zeile = e
            .xml
            .lines()
            .zip(python.lines())
            .enumerate()
            .find(|(_, (r, p))| r != p);
        panic!(
            "{fall}: Rust-XML ({} Bytes) weicht vom Python-XML ({} Bytes) ab, erste Zeile: {zeile:?}",
            e.xml.len(),
            python.len()
        );
    }
    let (ok, meldung) = elster::validiere_xsd_text(e.xml.as_bytes(), e.vz);
    assert!(ok, "{fall}: {meldung}");
}

#[test]
fn arbeitnehmer() {
    pruefe("arbeitnehmer");
}

#[test]
fn rentner() {
    pruefe("rentner");
}

/// Traegt drei Ring-Werte: Verpflegungskuerzung, Summenzeilen Anlage V, Handwerker aus zwei
/// Instanzen.
#[test]
fn gesamt() {
    pruefe("gesamt");
}

/// Die Angestellte mit Hausnummerzusatz "a" (Vault `hausnummer-zusatz-bekommt-ein-bindungsfeld`):
/// `E0101207` steht neben `E0101206`, und der Zusatz haengt an `<AbsStr>`. Das Rust-XML ist gleich
/// dem Python-XML (`pruefe`); die Zeilen darunter halten fest, dass die Python-Datei den Zusatz
/// ueberhaupt traegt -- sonst verglichen beide Seiten einen Fall ohne ihn.
#[test]
fn hausnummer_zusatz() {
    pruefe("hausnummer_zusatz");
    let python = std::fs::read_to_string(fixture("hausnummer_zusatz.xml")).unwrap();
    let hausnummer = python.find("<E0101206>7</E0101206>");
    let zusatz = python.find("<E0101207>a</E0101207>");
    assert!(
        hausnummer.is_some() && zusatz > hausnummer,
        "E0101207 steht nicht neben E0101206 im Python-XML"
    );
    assert!(
        python.contains("<AbsStr>Musterstraße 7a</AbsStr>"),
        "der Zusatz erreicht <AbsStr> nicht"
    );
}

/// Zwei Vermietungsobjekte und zwei Kinder (Vault `bescheid-elster-mutation`, Mutanten X02 und X17): die zweite Anlage V
/// traegt `Laufende_Nummer_V` 2 (aus dem Index der Instanz; die XSD verlangt eindeutige Nummern), und der Kinderzahl-Waechter
/// zaehlt das erste Kind (`kind_anlagen`) zu den weiteren Kind-Instanzen (`1 + n`). Das Rust-XML ist gleich dem Python-XML
/// (`pruefe`); die Zeilen darunter halten fest, dass die Python-Datei beides ueberhaupt traegt.
#[test]
fn instanzen() {
    pruefe("instanzen");
    let python = std::fs::read_to_string(fixture("instanzen.xml")).unwrap();
    for nr in ["1", "2"] {
        let kz = format!("<Laufende_Nummer_V>{nr}</Laufende_Nummer_V>");
        assert_eq!(python.matches(&kz).count(), 1, "{kz} im Python-XML");
    }
    assert_eq!(python.matches("<Kind>").count(), 2, "zwei Kind-Container");
}
