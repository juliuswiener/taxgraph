//! Die Protokolle `audit.jsonl` und `fehler.log` bei gleichzeitigem Anhaengen: kein Eintrag geht
//! verloren, keine Zeile wird zerschnitten.
//!
//! Gegenstueck zu `tests/test_audit.py::TestParallel` (4 Faeden x 5 Eintraege). Hier 8 x 150: jeder
//! Faden oeffnet die Datei bei jedem Aufruf neu (`anhaengen` haelt nichts offen), also treffen sich
//! die Faeden an `O_APPEND`. Der Test prueft die Datei selbst, nicht nur `lies`:
//!
//! - es gibt genau so viele Zeilen wie Aufrufe, keine leere und keine zerschnittene (jede ist ein
//!   JSON-Objekt),
//! - jeder (Faden, Nummer) steht genau einmal da und je Faden in der Reihenfolge des Schreibens,
//! - die Datei endet mit einem Zeilenumbruch.
//!
//! GEMESSEN (2026-10-06, Basis ohne den Fix): `writeln!` auf eine ungepufferte `File` macht zwei
//! `write`-Aufrufe (Zeile, dann Umbruch). Bei 8 Faeden x 150 stand in Zeile 7 `{..}{..}` ohne Umbruch
//! dazwischen. Anmeldung (`auth`) und Dispatcher (`api`) schreiben in dieselbe Datei; ein Protokoll,
//! das unter Last Zeilen verklebt, verliert die Spur, die es festhalten soll. Beide Protokolle gehen
//! durch `anhaenge_datei::haenge_zeile_an`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;
use store::audit::{anhaengen, lies, AuditAktion};
use store::fehler_log::{self, protokolliere, Meta, Stufe};

fn verzeichnis(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "taxgraph-audit-parallel-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Laesst `faeden` Faeden je `je_faden` Mal `schreibe(faden, nr)` rufen.
fn parallel(faeden: usize, je_faden: usize, schreibe: &(dyn Fn(usize, usize) + Sync)) {
    std::thread::scope(|s| {
        for faden in 0..faeden {
            s.spawn(move || {
                for nr in 0..je_faden {
                    schreibe(faden, nr);
                }
            });
        }
    });
}

/// Zerlegt die Rohdatei in Zeilen und prueft Form: Umbruch am Ende, genau `erwartet` Zeilen,
/// keine leere, jede ein JSON-Objekt. Gibt die Objekte in Dateireihenfolge zurueck.
fn zeilen_als_json(pfad: &Path, erwartet: usize) -> Vec<Value> {
    let text = std::fs::read_to_string(pfad).unwrap();
    assert!(
        text.ends_with('\n'),
        "die Datei endet nicht mit einem Zeilenumbruch"
    );
    let mut zeilen: Vec<&str> = text.split('\n').collect();
    // `split` liefert nach dem letzten Umbruch ein leeres Stueck.
    assert_eq!(zeilen.pop(), Some(""));
    assert_eq!(
        zeilen.len(),
        erwartet,
        "Eintraege verloren oder Zeilen zusammengeklebt"
    );
    zeilen
        .iter()
        .enumerate()
        .map(|(i, zeile)| {
            assert!(!zeile.trim().is_empty(), "leere Zeile {i}");
            let v: Value = serde_json::from_str(zeile).unwrap_or_else(|e| {
                let kopf: String = zeile.chars().take(300).collect();
                panic!("Zeile {i} ist kein JSON ({e}): {kopf:?}")
            });
            assert!(v.is_object(), "Zeile {i} ist kein Objekt");
            v
        })
        .collect()
}

/// Je Faden alle Nummern in Dateireihenfolge, aus `faden` und `nr`, die `zerlege` je Zeile liefert.
fn nummern_je_faden(
    zeilen: &[Value],
    zerlege: &dyn Fn(&Value) -> (usize, usize),
) -> BTreeMap<usize, Vec<usize>> {
    let mut je_faden: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for z in zeilen {
        let (f, n) = zerlege(z);
        je_faden.entry(f).or_default().push(n);
    }
    je_faden
}

fn erwarte_vollstaendig(je_faden: &BTreeMap<usize, Vec<usize>>, faeden: usize, je: usize) {
    assert_eq!(je_faden.len(), faeden, "Faeden fehlen: {:?}", je_faden.keys());
    let soll: Vec<usize> = (0..je).collect();
    for (faden, nummern) in je_faden {
        assert_eq!(
            nummern, &soll,
            "Faden {faden}: Eintraege fehlen, doppelt oder in falscher Reihenfolge"
        );
    }
}

fn audit_zerlegen(v: &Value) -> (usize, usize) {
    let detail = v["detail"].as_str().expect("Eintrag ohne detail");
    let (f, n) = detail
        .strip_prefix("faden=")
        .and_then(|r| r.split_once(" nr="))
        .unwrap_or_else(|| panic!("detail {detail:?} hat nicht die erwartete Form"));
    let n = n.split(' ').next().unwrap();
    let (f, n): (usize, usize) = (f.parse().unwrap(), n.parse().unwrap());
    assert_eq!(
        v["user_id"],
        format!("faden{f}"),
        "Nutzer und Faden passen nicht zusammen"
    );
    (f, n)
}

#[test]
fn gleichzeitiges_anhaengen_verliert_nichts_und_zerschneidet_nichts() {
    const FAEDEN: usize = 8;
    const JE: usize = 150;
    let dir = verzeichnis("viele");
    let pfad = dir.join("audit.jsonl");

    parallel(FAEDEN, JE, &|faden, nr| {
        let nutzer = format!("faden{faden}");
        let detail = format!("faden={faden} nr={nr}");
        anhaengen(
            &pfad,
            Some(nutzer.as_str()),
            AuditAktion::Login,
            Some("parallel-1"),
            Some(detail.as_str()),
        )
        .unwrap();
    });

    let zeilen = zeilen_als_json(&pfad, FAEDEN * JE);
    erwarte_vollstaendig(&nummern_je_faden(&zeilen, &audit_zerlegen), FAEDEN, JE);
    // `lies` sieht dasselbe wie die Rohdatei.
    assert_eq!(lies(&pfad).unwrap().len(), FAEDEN * JE);
    std::fs::remove_dir_all(&dir).ok();
}

/// Lange Zeilen (16 KiB, ueber `PIPE_BUF`): fuer eine reguläre Datei mit `O_APPEND` bleibt ein
/// `write` unteilbar, `PIPE_BUF` gilt nur fuer Rohre. Haelt die Aussage des `ponytail`-Kommentars in
/// `anhaenge_datei.rs` fest; die Aufrufer schreiben heute nur Zeilen unter 1 KiB.
#[test]
fn lange_zeilen_bleiben_ganz() {
    const FAEDEN: usize = 4;
    const JE: usize = 40;
    let dir = verzeichnis("lang");
    let pfad = dir.join("audit.jsonl");
    let fuell = "x".repeat(16 * 1024);

    parallel(FAEDEN, JE, &|faden, nr| {
        let nutzer = format!("faden{faden}");
        let detail = format!("faden={faden} nr={nr} {fuell}");
        anhaengen(
            &pfad,
            Some(nutzer.as_str()),
            AuditAktion::Login,
            None,
            Some(detail.as_str()),
        )
        .unwrap();
    });

    let zeilen = zeilen_als_json(&pfad, FAEDEN * JE);
    erwarte_vollstaendig(&nummern_je_faden(&zeilen, &audit_zerlegen), FAEDEN, JE);
    // Die Fuellung kam unversehrt an: kein Eintrag ist verkuerzt oder mit einem anderen vermischt.
    for z in &zeilen {
        let rest = z["detail"].as_str().unwrap().splitn(3, ' ').nth(2).unwrap();
        assert_eq!(rest, fuell, "ein Eintrag ist verkuerzt oder vermischt");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn das_fehlerprotokoll_verklebt_unter_last_ebenso_nicht() {
    const FAEDEN: usize = 8;
    const JE: usize = 150;
    let dir = verzeichnis("fehlerlog");
    let pfad = dir.join("fehler.log");

    // `anzahl` traegt Faden und Nummer: faden * 1000 + nr (Meta kennt nur Zahlen, keinen Text).
    parallel(FAEDEN, JE, &|faden, nr| {
        let meta = Meta {
            anzahl: Some(i64::try_from(faden * 1000 + nr).unwrap()),
            ..Meta::default()
        };
        protokolliere(&pfad, "test.parallel", &std::fmt::Error, Stufe::Warnung, None, meta).unwrap();
    });

    let zeilen = zeilen_als_json(&pfad, FAEDEN * JE);
    let je_faden = nummern_je_faden(&zeilen, &|v| {
        let a = usize::try_from(v["anzahl"].as_i64().expect("Eintrag ohne anzahl")).unwrap();
        (a / 1000, a % 1000)
    });
    erwarte_vollstaendig(&je_faden, FAEDEN, JE);
    assert_eq!(fehler_log::lies(&pfad).unwrap().len(), FAEDEN * JE);
    std::fs::remove_dir_all(&dir).ok();
}
