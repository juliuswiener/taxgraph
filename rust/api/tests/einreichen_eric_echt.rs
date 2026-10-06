//! V2 (Gegenstueck zu `tests/test_einreichen_durchstich.py` und `make abgabeweg-freigabe`): `POST
//! /fall/{id}/einreichen` ueber einen ECHTEN Server-Prozess (`taxgraph-api`, echter TCP-Socket) mit der
//! ECHTEN ERiC-Bibliothek und der ECHTEN Hersteller-ID. Nichts ist ersetzt: Fall und Ereignisse gehen
//! ueber die Routen, das XML baut der Dienst, `checkESt` prueft es.
//!
//! WARUM ES DIESEN TEST GIBT. `einreichen_attrappe_hermetisch.rs` laedt eine Attrappe von
//! `libericapi.so` und bewacht, was der Dienst aus ihrem Urteil macht, nicht das Urteil selbst. Mit der
//! Platzhalter-ID antwortet das echte `ERiC` `rc=610301202` (gesperrt, nicht geprueft). Nur die
//! registrierte ID macht aus `checkESt` ein Urteil. Ohne diesen Test meldet jeder Lauf ohne `ERiC` "gruen",
//! und das heisst nicht "der Abgabeweg wurde geprueft".
//!
//! KEIN VERSAND. Der Test ruft genau drei Routen: `POST /fall`, `POST /fall/{id}/event`, `POST
//! /fall/{id}/einreichen`. Die dritte prueft nur (`ERIC_VALIDIERE`, ohne Zertifikat, ohne Server-Antwort;
//! die Attrappe belegt `flags=2, crypto=NULL, serverantwort=NULL`) und antwortet `eingereicht: false`;
//! der Test prueft das. Das Versand-Ziel (`taxgraph-versand`) wird nicht gestartet.
//!
//! KEIN SKIP. Der Test ist `#[ignore]`: `cargo test --workspace` und die CI laufen ohne ihn (`ERiC` und die
//! ID gehoeren nicht auf einen Runner, Entscheid Julius 2026-09-12). Wer ihn mit `--ignored` ruft, will
//! den echten Weg: jede fehlende Voraussetzung ist ein `panic`, nie ein `return`.
//! `TAXGRAPH_OHNE_XSD=1` gilt hier NICHT (anders als bei den Schema-Tests), weil der Lauf ausdruecklich
//! verlangt wurde. Lokal: `make abgabeweg-freigabe-rust` (laedt die ID aus der gitignorierten `.env`).
//!
//! KEIN GEHEIMNIS IN DER MELDUNG. Die Meldungen nennen nur "ID fehlt" und "Bibliothek fehlt" und aus der
//! Antwort nur `status`, `grund`, `klasse`, `rc`, `plausibel` und die Laenge von `ericantwort`; nie die
//! ID, nie den Rumpf der Antwort. Stderr des Servers geht ins Leere.
//!
//! WAS DIE CI DECKT. Nur die Verdrahtung (`die_freigabe_ist_verdrahtet`): das Make-Ziel gibt es, es ruft
//! diesen Test mit `--ignored` beim richtigen Namen, der Test ist `#[ignore]`, und das Ziel wird rot,
//! wenn kein Test lief. `checkESt` selbst deckt die CI NICHT. Das ist die Restluecke von V2.
//!
//! Die Aussage ist qualitativ wie in Python (kein zweiter Zaehler fuer Restfehler): das XML kommt bis zur
//! Plausibilitaetspruefung, und der Dienst meldet das ehrlich (`rc`, `klasse`, Status, `plausibel`).
//! `rc=610301200` und `rc=610301202` haben einen leeren Fehlerpuffer und sehen wie "keine Beanstandung"
//! aus; sie sind kein Erfolg.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{json, Value};

/// Der Name des echten Tests: Make-Ziel und Verdrahtungs-Test pruefen ihn.
const ECHTER_TEST: &str = "einreichen_ueber_den_echten_endpunkt_mit_echtem_checkest";
const MAKE_ZIEL: &str = "abgabeweg-freigabe-rust";

/// Kegel einer vollstaendigen `gesamt`-Erklaerung der Angestellten, wie `_BASIS_A` in
/// `tests/test_einreichen_durchstich.py` (Cent). KAP-Detailfelder fehlen mit Absicht: `kein_kap` genuegt,
/// ein explizites Null-Feld loest bei checkESt einen eigenen Fehler aus.
fn basis() -> Vec<(&'static str, Value)> {
    vec![
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("vor_an_anteil_rv", json!(4_200_000)),
        ("vor_ag_anteil_rv", json!(1_200_000)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("stammdaten_nachname", json!("Maier")),
        ("stammdaten_vorname", json!("Hans")),
        ("stammdaten_geburtsdatum", json!("05.05.1955")),
        ("stammdaten_strasse", json!("Musterstr.")),
        ("stammdaten_hausnummer", json!("55")),
        ("stammdaten_plz", json!("55555")),
        ("stammdaten_wohnort", json!("Musterort")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_art_est_erklaerung", json!(true)),
        ("kist_konfession", json!("keine")),
        // Praefix 9181 muss zur Finanzamtsnummer des Empfaengers passen, sonst baut der Dienst kein XML.
        ("stammdaten_steuernummer", json!("9181081508155")),
        ("steuerklasse", json!("1")),
        ("p36_lohnsteuer", json!(1_200_000)),
        ("veranlagung", json!("einzel")),
    ]
}

/// Der Server-Prozess; endet mit dem Test.
struct Prozess {
    kind: Child,
    port: u16,
    /// Haelt das Rohr offen, damit der Server beim Schreiben nicht auf ein geschlossenes Rohr trifft.
    _stdout: BufReader<std::process::ChildStdout>,
}

impl Drop for Prozess {
    fn drop(&mut self) {
        let _ = self.kind.kill();
        let _ = self.kind.wait();
    }
}

fn starte(daten: &Path) -> Prozess {
    let mut kind = Command::new(env!("CARGO_BIN_EXE_taxgraph-api"))
        .arg("0")
        .current_dir(daten) // ERiC legt seine Protokolle ins Arbeitsverzeichnis, nicht ins Repo
        .env("TAXGRAPH_NO_AUTH", "1")
        .env("TAXGRAPH_DATEN", daten)
        .env("TAXGRAPH_AUDIT_DIR", daten)
        .env("TAXGRAPH_USER_STORE", daten.join("users.json"))
        .env("TAXGRAPH_FLOW", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("taxgraph-api startet nicht");
    let mut aus = BufReader::new(kind.stdout.take().unwrap());
    // Die erste Zeile nennt die Adresse; ohne Antwort in 60 s ist der Start gescheitert.
    let (tx, rx) = mpsc::channel();
    let leser = std::thread::spawn(move || {
        let mut zeile = String::new();
        let _ = aus.read_line(&mut zeile);
        let _ = tx.send((zeile, aus));
    });
    let (zeile, aus) = rx
        .recv_timeout(Duration::from_secs(60))
        .unwrap_or_else(|_| panic!("taxgraph-api meldet in 60 s keinen Port"));
    let _ = leser.join();
    let port = zeile
        .trim()
        .rsplit(':')
        .next()
        .and_then(|p| p.split_whitespace().next())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or_else(|| panic!("kein Port in der ersten Zeile des Servers"));
    Prozess {
        kind,
        port,
        _stdout: aus,
    }
}

/// Ein Request an den Server-Prozess; `(Status, Rumpf als JSON)`. Die Pruefung von `ERiC` dauert Sekunden;
/// nach 180 s ohne Antwort ist der Lauf gescheitert.
fn sende(port: u16, pfad: &str, rumpf: &Value) -> (u16, Value) {
    let text = rumpf.to_string();
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(180))).unwrap();
    write!(
        s,
        "POST {pfad} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
        text.len()
    )
    .unwrap();
    let mut roh = Vec::new();
    s.read_to_end(&mut roh)
        .unwrap_or_else(|e| panic!("POST {pfad}: keine vollstaendige Antwort ({})", e.kind()));
    let ende = roh
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("POST {pfad}: Antwort ohne Kopfende"));
    let kopf = String::from_utf8_lossy(&roh[..ende]).into_owned();
    let status = kopf
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or_else(|| panic!("POST {pfad}: kein Status"));
    (
        status,
        serde_json::from_slice(&roh[ende + 4..]).unwrap_or(Value::Null),
    )
}

#[test]
#[ignore = "braucht die ERiC-Bibliothek und die registrierte Hersteller-ID (Umgebungs-Gate, Entscheid Julius 2026-09-12): lokal mit `make abgabeweg-freigabe-rust`"]
fn einreichen_ueber_den_echten_endpunkt_mit_echtem_checkest() {
    // --- Voraussetzungen: jede fehlende ist ein Fehler, kein Skip. Nie den Wert der ID nennen.
    assert!(
        elster::find_eric_lib().is_some(),
        "Bibliothek fehlt: libericapi.so liegt weder unter $ERIC_DIR noch unter ~/02_Software/eric"
    );
    assert!(
        std::env::var("ELSTER_HERSTELLER_ID").is_ok_and(|v| !v.trim().is_empty()),
        "ID fehlt: ELSTER_HERSTELLER_ID ist nicht gesetzt (make abgabeweg-freigabe-rust laedt sie aus der .env)"
    );

    let tmp = tempfile::tempdir().unwrap();
    let p = starte(tmp.path());

    // --- Fall und Ereignisse ueber die Routen.
    let (s, a) = sende(
        p.port,
        "/fall",
        &json!({"fall_id": "echt1", "scheibe": "gesamt", "veranlagungszeitraum": 2025}),
    );
    assert_eq!(s, 201, "POST /fall: {a}");
    for (feld, wert) in basis() {
        let rumpf = json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (s, _) = sende(p.port, "/fall/echt1/event", &rumpf);
        assert_eq!(s, 201, "POST /event {feld}");
    }

    // --- Einreichen: nur pruefen. Der Rumpf der Antwort kommt nie in eine Meldung.
    let (status, a) = sende(p.port, "/fall/echt1/einreichen", &json!({}));
    let grund = a["grund"].as_str().unwrap_or("");
    let klasse = a["klasse"].as_str().unwrap_or("");
    let rc = a["rc"].as_i64();
    let eric_laenge = a["ericantwort"].as_str().map(str::len);
    let kurz = format!(
        "status={status} grund={grund:?} klasse={klasse:?} rc={rc:?} plausibel={} ericantwort_bytes={eric_laenge:?}",
        a["plausibel"]
    );

    assert_ne!(
        grund, "deklaration_unvollstaendig",
        "Fixtur unvollstaendig, der Test misst den falschen Pfad: {kurz}"
    );
    assert_ne!(
        grund, "xml_nicht_baubar",
        "der Dienst baut kein XML (etwa ohne ID im Dienst-Prozess): {kurz}"
    );
    assert_ne!(
        grund, "eric_nicht_verfuegbar",
        "ERiC ist im Dienst-Prozess nicht ladbar: {kurz}"
    );
    assert_eq!(
        a["eingereicht"],
        json!(false),
        "der Test darf nichts senden: {kurz}"
    );
    assert_ne!(
        klasse, "io_gate_nicht_geprueft",
        "das XML bricht VOR der Plausibilitaetspruefung ab (leerer Fehlerpuffer sieht wie Erfolg aus): {kurz}"
    );
    assert_ne!(
        klasse, "hersteller_id_gesperrt",
        "die Hersteller-ID ist gesperrt oder nicht registriert, ERiC hat nicht geprueft: {kurz}"
    );
    assert!(
        matches!(klasse, "plausibel" | "plausibilitaet_fehler"),
        "weder plausibel noch Plausibilitaetsfehler: {kurz}"
    );
    // Falsch-Gruen-Kern: 200 und `plausibel: true` genau bei rc 0.
    if rc == Some(0) {
        assert!(
            status == 200 && a["plausibel"] == json!(true),
            "rc=0 ohne 200/plausibel: {kurz}"
        );
    } else {
        assert!(
            status == 422 && a["plausibel"] != json!(true),
            "rc!=0, aber der Dienst meldet Erfolg: {kurz}"
        );
        assert!(
            eric_laenge.is_some_and(|n| n > 0),
            "Plausibilitaetsfehler ohne ericantwort, der Nutzer saehe keine Regel: {kurz}"
        );
    }
}

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Die Zeilen des Rezepts eines Make-Ziels (die Tab-Zeilen nach `ziel:` bis zur ersten anderen Zeile),
/// Fortsetzungszeilen mit `\` zusammengezogen.
fn rezept(makefile: &str, ziel: &str) -> Option<String> {
    let kopf = format!("{ziel}:");
    let mut zeilen = makefile.lines().skip_while(|z| !z.starts_with(&kopf));
    zeilen.next()?;
    let text: Vec<&str> = zeilen
        .take_while(|z| z.starts_with('\t') || z.trim().is_empty())
        .collect();
    Some(text.join("\n").replace("\\\n", " "))
}

/// `Ok`, wenn Make-Ziel und Quelltext des echten Tests zusammenpassen, sonst der Grund.
fn verdrahtung(makefile: &str, quelle: &str, test_datei: &str) -> Result<(), String> {
    let Some(r) = rezept(makefile, MAKE_ZIEL) else {
        return Err(format!("Make-Ziel `{MAKE_ZIEL}` fehlt"));
    };
    // Ganze Woerter, nicht Teilketten: `--test einreichen_eric_echt2` ist eine andere Testdatei.
    let woerter: Vec<&str> = r.split(|c: char| c.is_whitespace() || c == '(' || c == ')').collect();
    if !woerter.contains(&"cargo") || !woerter.windows(2).any(|w| w == ["--test", test_datei]) {
        return Err(format!("`{MAKE_ZIEL}` ruft `cargo test --test {test_datei}` nicht"));
    }
    if !woerter.contains(&"--ignored") {
        return Err(format!("`{MAKE_ZIEL}` ruft den Test ohne `--ignored`: der echte Test liefe nie"));
    }
    if !woerter.contains(&ECHTER_TEST) {
        return Err(format!("`{MAKE_ZIEL}` nennt den Test `{ECHTER_TEST}` nicht (Name weicht ab)"));
    }
    if !r.contains("1 passed") {
        return Err(format!(
            "`{MAKE_ZIEL}` prueft nicht, dass genau ein Test lief: ein falscher Name liefe als 0 Tests gruen"
        ));
    }
    // Der Test steht im Quelltext und traegt `#[ignore`: sonst liefe er in `cargo test --workspace`.
    let kopf = format!("fn {ECHTER_TEST}()");
    let Some(stelle) = quelle.find(&kopf) else {
        return Err(format!("`{kopf}` steht nicht im Quelltext (Name weicht ab)"));
    };
    let davor: Vec<&str> = quelle[..stelle].lines().rev().take(3).collect();
    if !davor.iter().any(|z| z.trim_start().starts_with("#[ignore")) {
        return Err(format!("`{ECHTER_TEST}` traegt kein `#[ignore`: ohne es braucht jedes Tor ERiC und die ID"));
    }
    if !davor.iter().any(|z| z.trim() == "#[test]") {
        return Err(format!("`{ECHTER_TEST}` traegt kein `#[test]`"));
    }
    Ok(())
}

#[test]
fn die_freigabe_ist_verdrahtet() {
    let makefile = std::fs::read_to_string(wurzel().join("Makefile")).unwrap();
    // Der Quelltext dieser Datei, zur Uebersetzungszeit; der Name der Testdatei ist der Name des Binaers.
    let quelle = include_str!("einreichen_eric_echt.rs");
    if let Err(grund) = verdrahtung(&makefile, quelle, env!("CARGO_CRATE_NAME")) {
        panic!("{grund}");
    }
}

/// Ein Make-Ziel, das `verdrahtung` annimmt.
fn muster_ziel() -> String {
    format!(
        "{MAKE_ZIEL}:\n\tcd rust && cargo test -p api --test einreichen_eric_echt -- --ignored --exact {ECHTER_TEST} > $$o 2>&1; rc=$$?; \\\n\tgrep -q '1 passed' $$o\n"
    )
}

/// Ein Quelltext, den `verdrahtung` annimmt. Der Name steht hier nie als `fn <Name>()` im Quelltext
/// dieser Datei, sonst faende `verdrahtung` die Nachbildung statt des echten Tests.
fn muster_quelle() -> String {
    format!("#[test]\n#[ignore = \"x\"]\nfn {ECHTER_TEST}() {{}}\n")
}

#[test]
fn der_verdrahtungs_test_wird_bei_jedem_bruch_rot() {
    const DATEI: &str = "einreichen_eric_echt";
    let (ziel, quelle) = (muster_ziel(), muster_quelle());
    assert_eq!(verdrahtung(&ziel, &quelle, DATEI), Ok(()), "Ausgangslage muss gruen sein");
    let rot = |makefile: &str, quelle: &str, datei: &str, soll: &str| {
        let r = verdrahtung(makefile, quelle, datei);
        assert!(r.as_ref().is_err_and(|g| g.contains(soll)), "{soll:?} erwartet, erhalten {r:?}");
    };
    // 1. das Make-Ziel fehlt (auch: ein anderes Ziel mit dem Namen als Vorsilbe zaehlt nicht)
    rot("anderes:\n\ttrue\n", &quelle, DATEI, "fehlt");
    rot(&ziel.replace("freigabe-rust:", "freigabe-rustx:"), &quelle, DATEI, "fehlt");
    // 2. `--ignored` fehlt
    rot(&ziel.replace("--ignored", ""), &quelle, DATEI, "--ignored");
    // 3. der Testname im Rezept weicht ab, oder der Quelltext traegt den Namen nicht mehr
    rot(&ziel.replace("mit_echtem_checkest", "mit_checkest"), &quelle, DATEI, "Name weicht ab");
    rot(&ziel, &quelle.replace("mit_echtem_checkest", "mit_checkest"), DATEI, "Name weicht ab");
    // 4. `#[ignore]` am echten Test fehlt
    rot(&ziel, &quelle.replace("#[ignore = \"x\"]\n", ""), DATEI, "#[ignore");
    // dazu: falsche Testdatei (auch als Vorsilbe), kein Nachweis, dass ein Test lief
    rot(&ziel, &quelle, "anderer_test", "--test anderer_test");
    rot(&ziel.replace("--test einreichen_eric_echt", "--test einreichen_eric_echt2"), &quelle, DATEI, "--test einreichen_eric_echt");
    rot(&ziel.replace("mit_echtem_checkest", "mit_echtem_checkest2"), &quelle, DATEI, "Name weicht ab");
    rot(&ziel.replace("1 passed", "ok"), &quelle, DATEI, "genau ein Test");
}
