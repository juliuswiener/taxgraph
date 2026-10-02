//! Gemeinsame Test- und Doctest-Hilfen: die Regel „ERiC-Schema fehlt → rot".
//!
//! `pub` und versteckt wie `bescheid::testhilfe`: ein Doctest ist eine eigene Crate und sieht kein
//! `cfg(test)`. Ein Feature (wie `domain::testhilfe`) schaltete `cargo test -p elster` nicht ein,
//! die Doctests saehen den Helfer dann nicht. Nicht Teil der stabilen API.
use std::io::Write;

/// Liegen beide Schemas fuer `vz`? Fehlt eines, ist das rot, ausser `TAXGRAPH_OHNE_XSD=1` —
/// dieselbe Regel wie `tests/conftest.py`. Mit dem Flag steht der Verzicht auf stderr, und die
/// Antwort ist `false`: der Aufrufer laesst weg, was das Schema braucht.
///
/// ponytail: verlangt auch das `extern`-XSD, wo ein Test nur `E10-<vz>.xsd` liest (die
/// Auslieferung bringt beide). Aus einem bestandenen Doctest zeigt rustdoc kein stderr, auch mit
/// `--no-capture` nicht: dort nennt nur `ci.yml` den Verzicht. Muss er ins Log, den Doctest
/// `no_run` setzen und die Pruefung in einen echten Test legen.
///
/// # Panics
/// Wenn ein Schema fehlt und `TAXGRAPH_OHNE_XSD` nicht genau `1` ist. Das ist der rote Test.
///
/// ```
/// if elster::testhilfe::schemas_da(2025) {
///     assert!(elster::finde_schema(2025, "E10-{jahr}.xsd").is_some());
/// }
/// ```
#[must_use]
pub fn schemas_da(vz: i64) -> bool {
    let fehlt: Vec<String> = [
        (
            format!("E10-{vz}.xsd"),
            crate::finde_schema(vz, "E10-{jahr}.xsd"),
        ),
        (
            format!("elster11_E10_{vz}_extern.xsd"),
            crate::finde_xsd_schema(&vz.to_string()),
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
