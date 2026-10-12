//! Die Verdrahtung der drei Tests gegen das ECHTE `checkESt`, die kein eigenes Ziel in `blockmatrix-rust` oder
//! `abgabeweg-freigabe-rust` haben (Vault: `drei-echte-checkest-tests-bekommen-ein-make-ziel-2026-10-12`).
//!
//! WARUM ES DIESE DATEI GIBT. Die drei Tests tragen `#[ignore]`: `ERiC` und die Hersteller-ID gehoeren nicht auf einen Runner
//! (Entscheid Julius 2026-09-12). Ein `#[ignore]`-Test ohne Make-Ziel laeuft nie, und niemand merkt es, wenn jemand `#[ignore]`
//! entfernt, den Test umbenennt oder das Ziel loescht. `make checkest-rust` ruft sie lokal; die CI fuehrt NUR diese Verdrahtung aus
//! und deckt `checkESt` selbst NICHT. Wie `checkest_blockmatrix.rs::die_blockmatrix_ist_verdrahtet`, aber mit einer Kopie der Logik
//! fuer alle drei Tests statt einer je Datei: die Tests liegen in zwei Paketen und drei Dateien.
//!
//! WAS GEPRUEFT WIRD, je Test: das Make-Ziel gibt es; ein eigener `cargo test`-Aufruf nennt den Test genau einmal, mit dem richtigen
//! `-p` und `--test`, mit `--ignored` und `--exact`; das Ziel zaehlt `ok. 1 passed` und verlangt genau so viele Treffer wie Tests; der
//! Quelltext des Tests traegt `#[test]` und `#[ignore`. Die Quelltexte werden zur Laufzeit gelesen, nicht eingebunden.
#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const MAKE_ZIEL: &str = "checkest-rust";
/// Die Zeile, mit der das Ziel die bestandenen Tests zaehlt.
const ZAEHLUNG: &str = "grep -c 'test result: ok. 1 passed'";

/// Ein echter Test: wo er liegt und wie das Make-Ziel ihn ruft.
struct EchterTest {
    paket: &'static str,
    /// Name der Testdatei ohne `.rs` (`--test <datei>`).
    datei: &'static str,
    name: &'static str,
    /// Pfad des Quelltexts von der Wurzel des Repos.
    pfad: &'static str,
}

const ECHTE_TESTS: [EchterTest; 3] = [
    EchterTest {
        paket: "api",
        datei: "xml_export_hermetisch",
        name: "die_geschriebene_datei_besteht_checkest",
        pfad: "rust/api/tests/xml_export_hermetisch.rs",
    },
    EchterTest {
        paket: "bescheid",
        datei: "versorgung_kz_im_xml",
        name: "checkest_nimmt_die_zeilen_11_bis_13_an",
        pfad: "rust/bescheid/tests/versorgung_kz_im_xml.rs",
    },
    EchterTest {
        paket: "bescheid",
        datei: "checkest_privatverkauf_spike",
        name: "privatverkauf_kz_pflichtliste_gegen_echtes_checkest",
        pfad: "rust/bescheid/tests/checkest_privatverkauf_spike.rs",
    },
];

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Die Zeilen des Rezepts eines Make-Ziels (die Tab-Zeilen nach `ziel:` bis zur ersten anderen Zeile), Fortsetzungszeilen mit `\`
/// zusammengezogen. Wie in `checkest_blockmatrix.rs`.
fn rezept(makefile: &str, ziel: &str) -> Option<String> {
    let kopf = format!("{ziel}:");
    let mut zeilen = makefile.lines().skip_while(|z| !z.starts_with(&kopf));
    zeilen.next()?;
    let text: Vec<&str> = zeilen.take_while(|z| z.starts_with('\t') || z.trim().is_empty()).collect();
    Some(text.join("\n").replace("\\\n", " "))
}

/// Die Woerter jedes `cargo`-Aufrufs im Rezept, je Aufruf bis zum ersten `&` oder `)`.
fn aufrufe(rezept: &str) -> Vec<Vec<&str>> {
    rezept
        .split("cargo ")
        .skip(1)
        .map(|stueck| stueck.split(['&', ')']).next().unwrap_or("").split_whitespace().collect())
        .collect()
}

/// `Ok`, wenn Make-Ziel und Quelltext des Tests `t` zusammenpassen, sonst der Grund. `alle` sind alle echten Tests des Ziels.
fn verdrahtung(makefile: &str, quelle: &str, t: &EchterTest, alle: &[EchterTest]) -> Result<(), String> {
    let Some(r) = rezept(makefile, MAKE_ZIEL) else {
        return Err(format!("Make-Ziel `{MAKE_ZIEL}` fehlt"));
    };
    let name = t.name;
    let treffer: Vec<Vec<&str>> = aufrufe(&r).into_iter().filter(|a| a.contains(&name)).collect();
    let [aufruf] = treffer.as_slice() else {
        return Err(format!(
            "`{MAKE_ZIEL}` ruft den Test `{name}` in {} cargo-Aufrufen statt in einem (Name weicht ab, oder er fehlt)",
            treffer.len()
        ));
    };
    // Ganze Woerter, nicht Teilketten: `--test versorgung_kz_im_xml2` ist eine andere Testdatei.
    if aufruf.first() != Some(&"test") {
        return Err(format!("`{MAKE_ZIEL}` ruft `{name}` nicht mit `cargo test`"));
    }
    if !aufruf.windows(2).any(|w| w == ["-p", t.paket]) {
        return Err(format!("`{MAKE_ZIEL}` ruft `{name}` nicht mit `-p {}`", t.paket));
    }
    if !aufruf.windows(2).any(|w| w == ["--test", t.datei]) {
        return Err(format!("`{MAKE_ZIEL}` ruft `{name}` nicht mit `--test {}`", t.datei));
    }
    if !aufruf.contains(&"--ignored") {
        return Err(format!("`{MAKE_ZIEL}` ruft `{name}` ohne `--ignored`: der echte Test liefe nie"));
    }
    if !aufruf.contains(&"--exact") {
        return Err(format!("`{MAKE_ZIEL}` ruft `{name}` ohne `--exact`: ein Namensteil traefe fremde Tests"));
    }
    if let Some(fremd) = alle.iter().find(|a| a.name != name && aufruf.contains(&a.name)) {
        return Err(format!("`{MAKE_ZIEL}` ruft `{name}` zusammen mit `{}` in einem Aufruf", fremd.name));
    }
    let zaehl = format!("-eq {}", alle.len());
    if !r.contains(ZAEHLUNG) || !r.contains(&zaehl) {
        return Err(format!("`{MAKE_ZIEL}` zaehlt nicht mit `{ZAEHLUNG}` und `{zaehl}`: ein falscher Name liefe als 0 Tests gruen"));
    }
    // Der Test steht im Quelltext und traegt `#[test]` und `#[ignore`: sonst liefe er in `cargo test --workspace` (ohne ERiC) oder
    // gar nicht.
    let kopf = format!("fn {name}()");
    let Some(stelle) = quelle.find(&kopf) else {
        return Err(format!("`{kopf}` steht nicht in {} (Name weicht ab)", t.pfad));
    };
    let davor: Vec<&str> = quelle[..stelle].lines().rev().take(3).collect();
    if !davor.iter().any(|z| z.trim_start().starts_with("#[ignore")) {
        return Err(format!("`{name}` traegt kein `#[ignore`: ohne es braucht jedes Tor ERiC und die ID"));
    }
    if !davor.iter().any(|z| z.trim() == "#[test]") {
        return Err(format!("`{name}` traegt kein `#[test]`"));
    }
    Ok(())
}

fn lese(pfad: &str) -> String {
    std::fs::read_to_string(wurzel().join(pfad)).unwrap_or_else(|e| panic!("{pfad} nicht lesbar: {e}"))
}

fn pruefe(i: usize) {
    let t = &ECHTE_TESTS[i];
    if let Err(grund) = verdrahtung(&lese("Makefile"), &lese(t.pfad), t, &ECHTE_TESTS) {
        panic!("{grund}");
    }
}

#[test]
fn der_xml_export_test_ist_verdrahtet() {
    pruefe(0);
}

#[test]
fn der_versorgungs_test_ist_verdrahtet() {
    pruefe(1);
}

#[test]
fn der_privatverkauf_test_ist_verdrahtet() {
    pruefe(2);
}

/// Der `cargo`-Aufruf von `t`, wie ihn das Make-Ziel schreibt.
fn aufruf_von(t: &EchterTest) -> String {
    format!("cargo test -p {} --test {} -- --ignored --nocapture --exact {}", t.paket, t.datei, t.name)
}

/// Ein Make-Ziel, das `verdrahtung` annimmt.
fn muster_ziel() -> String {
    let aufrufe: Vec<String> = ECHTE_TESTS.iter().map(aufruf_von).collect();
    format!(
        "{MAKE_ZIEL}:\n\t(cd rust && {}) > $$log 2>&1; rc=$$?; \\\n\t[ $$rc -eq 0 ] && [ \"$$({ZAEHLUNG} $$log)\" -eq {} ]\n",
        aufrufe.join(" && "),
        ECHTE_TESTS.len()
    )
}

/// Ein Quelltext, den `verdrahtung` annimmt. Die Namen stehen hier nie als `fn <Name>()` im Quelltext dieser Datei, sonst faende
/// `verdrahtung` die Nachbildung statt der echten Tests.
fn muster_quelle(t: &EchterTest) -> String {
    let mut quelle = String::new();
    writeln!(quelle, "#[test]\n#[ignore = \"x\"]\nfn {}() {{}}", t.name).unwrap();
    quelle
}

#[test]
fn der_verdrahtungs_test_wird_bei_jedem_bruch_rot() {
    let (ziel, quellen): (String, Vec<String>) = (muster_ziel(), ECHTE_TESTS.iter().map(muster_quelle).collect());
    for (i, t) in ECHTE_TESTS.iter().enumerate() {
        assert_eq!(verdrahtung(&ziel, &quellen[i], t, &ECHTE_TESTS), Ok(()), "Ausgangslage von {} muss gruen sein", t.name);
    }
    for (i, t) in ECHTE_TESTS.iter().enumerate() {
        let (quelle, aufruf) = (&quellen[i], aufruf_von(t));
        let rot = |makefile: &str, quelle: &str, soll: &str| {
            let r = verdrahtung(makefile, quelle, t, &ECHTE_TESTS);
            assert!(r.as_ref().is_err_and(|g| g.contains(soll)), "{}: {soll:?} erwartet, erhalten {r:?}", t.name);
        };
        // 1. das Make-Ziel fehlt (auch: ein anderes Ziel mit dem Namen als Vorsilbe zaehlt nicht)
        rot("anderes:\n\ttrue\n", quelle, "fehlt");
        rot(&ziel.replace("checkest-rust:", "checkest-rustx:"), quelle, "fehlt");
        // 2. der Aufruf des Tests fehlt, oder sein Name weicht im Ziel oder im Quelltext ab
        rot(&ziel.replace(&format!(" && {aufruf}"), "").replace(&format!("{aufruf} && "), ""), quelle, "0 cargo-Aufrufen");
        rot(&ziel.replace(&aufruf, &aufruf.replace(t.name, "anderer_name")), quelle, "0 cargo-Aufrufen");
        rot(&ziel, &quelle.replace(t.name, "anderer_name"), "Name weicht ab");
        // 3. `-p`, `--test`, `--ignored` oder `--exact` fehlt oder weicht ab
        rot(&ziel.replace(&aufruf, &aufruf.replace(&format!("-p {}", t.paket), "-p anderes")), quelle, "nicht mit `-p");
        rot(&ziel.replace(&aufruf, &aufruf.replace(&format!("--test {}", t.datei), &format!("--test {}2", t.datei))), quelle, "nicht mit `--test");
        rot(&ziel.replace(&aufruf, &aufruf.replace("--ignored", "")), quelle, "ohne `--ignored`");
        rot(&ziel.replace(&aufruf, &aufruf.replace("--exact", "")), quelle, "ohne `--exact`");
        // 4. ein zweiter echter Test im selben Aufruf: `ok. 1 passed` trafe nicht mehr
        let anderer = &ECHTE_TESTS[(i + 1) % ECHTE_TESTS.len()];
        rot(&ziel.replace(&aufruf, &format!("{aufruf} {}", anderer.name)), quelle, "zusammen mit");
        // 5. die Zaehlung fehlt oder verlangt weniger Treffer
        rot(&ziel.replace(&format!("-eq {}", ECHTE_TESTS.len()), &format!("-eq {}", ECHTE_TESTS.len() - 1)), quelle, "zaehlt nicht");
        rot(&ziel.replace("grep -c", "grep -q"), quelle, "zaehlt nicht");
        // 6. `#[ignore]` oder `#[test]` am echten Test fehlt
        rot(&ziel, &quelle.replacen("#[ignore = \"x\"]\n", "", 1), "#[ignore");
        rot(&ziel, &quelle.replacen("#[test]\n", "", 1), "#[test]");
    }
}
