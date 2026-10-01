//! E2E: eine Fall-Datei bis zum ELSTER-XML, Byte fuer Byte wie Python.
//!
//! Unter `rust/fixtures/e2e/` liegen je Fall die Fall-Datei `<fall>.json` und `<fall>.xml`, das
//! XML, das `api.einreichen` fuer denselben Fall in die ERiC-Pruefung gaebe. Beides schreibt
//! `tools/parity/e2e_faelle.py` ueber den Nutzerpfad (`fall_anlegen`, `event`, `einreichen`). Der
//! Test laedt die Fall-Datei wie der Server (`store::lade`), ruft [`einreichungs_xml`] und verlangt:
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
//! Neu erzeugen: `python3 tools/parity/e2e_faelle.py` aus der Repo-Wurzel.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::path::{Path, PathBuf};

use bescheid::deklaration::{einreichungs_xml, EinreichFehler};
use bescheid::testhilfe::{index, params};

fn fixture(datei: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/e2e")
        .join(datei)
}

/// Liegen beide Schemas fuer `vz`? Fehlt eines, ist das rot, ausser `TAXGRAPH_OHNE_XSD=1`.
fn schemas_da(vz: i64) -> bool {
    let fehlt: Vec<String> = [
        (
            format!("E10-{vz}.xsd"),
            elster::finde_schema(vz, "E10-{jahr}.xsd"),
        ),
        (
            format!("elster11_E10_{vz}_extern.xsd"),
            elster::finde_xsd_schema(&vz.to_string()),
        ),
    ]
    .into_iter()
    .filter_map(|(name, pfad)| pfad.is_none().then_some(name))
    .collect();
    if fehlt.is_empty() {
        return true;
    }
    assert!(
        std::env::var("TAXGRAPH_OHNE_XSD").as_deref() == Ok("1"),
        "ERiC-Schema fehlt: {fehlt:?}. ERIC_DIR auf die ERiC-Auslieferung setzen; \
         TAXGRAPH_OHNE_XSD=1 nur, wo kein ERiC liegen kann (CI)."
    );
    // Direkt auf stderr: `eprintln!` faengt libtest ein, die CI saehe den Verzicht sonst nie.
    #[allow(clippy::explicit_write)]
    writeln!(
        std::io::stderr(),
        "TAXGRAPH_OHNE_XSD=1: {fehlt:?} fehlt, XML und XSD NICHT geprueft"
    )
    .unwrap();
    false
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
    let (ok, meldung) = elster::validiere_xsd_text(e.xml.as_bytes(), &e.vz.jahr().to_string());
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
