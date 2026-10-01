//! `store::EventId::von_json` (Rust) gegen `produkt/store/store.py::event_id` (Python, ueber
//! `tools/parity/oracle.py`) fuer die Content-Adressierung des Sachverhalts-Stores. Deliverable
//! #7 der `store`-Crate.
//!
//! Interpretation von "1000 proptest sequences" aus der Deliverable-Vorgabe: `event_id`/
//! `canonical_json` sind reine Funktionen ueber einem JSON-Objekt, keine zustandsbehaftete
//! `Store::append`-Sequenz -- eine Paritaet ueber Ableitungen/Auflagen wuerde denselben
//! Bindungs-Registry-Zustand in Python UND Rust synchron halten muessen, weit ausserhalb der
//! `sha256(canonical_json(...))`-Content-Adressierung, um die es hier geht. Deshalb: 1000
//! proptest-generierte JSON-Objekte (statt Aufruf-Sequenzen), jedes einzeln durch beide
//! Implementierungen gereicht.
//!
//! Braucht die Catala-Opam-Toolchain (oracle.py importiert sie eager, s. Moduldoku dort) +
//! `python3` mit dem Repo-Umfeld -- in CI standardmaessig SKIP, lokal erzwingen:
//!
//!   `PARITY`=1 `cargo` test -p parity --test `store_paritaet` -- --nocapture
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use parity::{diff_event_id, Oracle};
use proptest::prelude::*;
use serde_json::Value;
use store::EventId;

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip_ohne_parity_env() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

/// Ein beliebig verschachteltes JSON-Objekt als oberste Ebene (Python `event.items()` braucht
/// ein Dict) mit Strings, Ganzzahlen, Bool, `null`, Arrays und geschachtelten Objekten als
/// Blaetter -- KEINE Floats (dokumentierter Coverage-Gap, s. `store/src/canonical.rs`), da deren
/// Python-`repr()`-Formatierung nicht Byte-fuer-Byte mit `serde_json` verglichen ist.
fn arbitrary_json_leaf() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::from),
        any::<i64>().prop_map(Value::from),
        // Unicode, Kontrollzeichen, Anfuehrungszeichen, Rueckwaertsschraegstrich -- dieselben
        // Faelle, die `canonical.rs`s Unit-Tests schon einzeln pruefen, hier als freier String.
        ".*".prop_map(Value::from),
    ]
}

fn arbitrary_json_object() -> impl Strategy<Value = Value> {
    let leaf = arbitrary_json_leaf();
    leaf.prop_recursive(3, 32, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Value::from),
            prop::collection::hash_map(".{1,12}", inner, 0..6)
                .prop_map(|m| Value::Object(m.into_iter().collect())),
        ]
    })
}

/// Erzwingt ein JSON-*Objekt* auf oberster Ebene (Python `event.items()`).
fn arbitrary_event() -> impl Strategy<Value = Value> {
    prop::collection::hash_map(".{1,16}", arbitrary_json_object(), 1..8)
        .prop_map(|m| Value::Object(m.into_iter().collect()))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// `store::EventId::von_json` matcht `produkt/store/store.py::event_id` byte-identisch
    /// ueber 1000 zufaellig generierte JSON-Objekte (s. Moduldoku fuer die Interpretation).
    #[test]
    fn event_id_paritaet_ueber_zufaellige_json_objekte(event in arbitrary_event()) {
        if skip_ohne_parity_env() {
            return Ok(());
        }
        let mut oracle = Oracle::spawn(&repo_root()).expect("oracle.py startet");
        let rust_hex = EventId::von_json(&event).to_string();
        let abweichung = diff_event_id(&mut oracle, &event, &rust_hex)
            .expect("Orakel-Aufruf laeuft durch");
        prop_assert_eq!(abweichung, None);
    }
}

/// Beweist, dass `diff_event_id` selbst eine Abweichung findet: EIN Rust-Hash wird verstuemmelt
/// (letztes Hex-Zeichen geflippt), der Rest bleibt echt. Ohne diesen Test koennte `diff_event_id`
/// kaputt sein (z. B. immer `None` liefern) und der Sweep oben liefe trotzdem gruen durch.
#[test]
fn negativkontrolle_erkennt_genau_eine_abweichung() {
    if skip_ohne_parity_env() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht Catala-Toolchain + Python-Umfeld)");
        return;
    }
    let mut oracle = Oracle::spawn(&repo_root()).expect("oracle.py startet");
    let mut abweichungen = Vec::new();

    for i in 0..20i64 {
        let event =
            serde_json::json!({"feld_id": "ep_arbeitstage", "wert": i, "zustand": "bestaetigt"});
        let mut rust_hex = EventId::von_json(&event).to_string();
        if i == 10 {
            // Letztes Hex-Zeichen kontrolliert kippen -- bleibt gueltiges Hex, garantiert
            // ungleich dem echten Hash.
            let letztes = rust_hex.pop().expect("hex ist nie leer");
            let geflippt = if letztes == '0' { '1' } else { '0' };
            rust_hex.push(geflippt);
        }
        if let Some(_abweichung) =
            diff_event_id(&mut oracle, &event, &rust_hex).expect("Orakel-Aufruf laeuft durch")
        {
            abweichungen.push(i);
        }
    }

    eprintln!(
        "negativkontrolle: {} Abweichungen gefunden (erwartet: 1)",
        abweichungen.len()
    );
    assert_eq!(
        abweichungen.len(),
        1,
        "Kontrollprobe muss GENAU eine Abweichung finden"
    );
}

/// Deliverable #7a: `event_id`-Paritaet ueber jedes Event jeder ECHTEN Fall-Datei unter
/// `~/.local/share/taxgraph/faelle/` (bzw. `$TAXGRAPH_DATEN`, s. [`faelle_verzeichnis`]).
///
/// SICHERHEIT: diese Dateien tragen echte Nutzer-Steuerdaten. Nur lokal, in-prozess, ausschliesslich
/// zum Hash-Vergleich lesen -- NIEMALS ins Repo/Fixtures kopieren. In Testausgabe/Assertions duerfen
/// ausschliesslich Zaehlwerte und Hash-Werte auftauchen, nie Feldwerte/Pfade/Inhalte.
///
/// Zwei Pruefungen je Event: (1) Selbstkonsistenz -- `event.berechne_event_id()` reproduziert den
/// gespeicherten `event_id` (haette Gap 1 sonst Bestandsdateien mit einem VERALTETEN Hash still
/// durchgelassen); (2) Paritaet zu Python ueber `diff_event_id`. EIN Oracle-Prozess fuer den
/// gesamten Sweep (nicht einer je Datei).
#[test]
fn event_id_paritaet_ueber_bestandsdateien_falls_vorhanden() {
    if skip_ohne_parity_env() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht Catala-Toolchain + Python-Umfeld)");
        return;
    }
    let verzeichnis = faelle_verzeichnis();
    let kandidaten: Vec<std::path::PathBuf> = walk_json(&verzeichnis);
    assert!(
        !kandidaten.is_empty(),
        "event_id_paritaet_ueber_bestandsdateien: 0 Fall-Dateien unter {} -- ein Parity-Lauf \
         ohne Korpus belegt nichts.",
        verzeichnis.display()
    );
    let erwartete_dateien = kandidaten.len() as u64;
    let mut oracle = Oracle::spawn(&repo_root()).expect("oracle.py startet");
    let mut dateien = 0u64;
    let mut geprueft = 0u64;
    let mut selbst_diffs = 0u64;
    let mut python_diffs = 0u64;
    // P10: store::lade laedt inzwischen ALLE realen Faelle (legacy Herkunft, unbegrenzte VZ) —
    // ein Ladefehler ist kein stiller Skip mehr, sondern ein harter Testabbruch.
    for pfad in kandidaten {
        let datei =
            store::lade(&pfad).unwrap_or_else(|e| panic!("store::lade({}): {e}", pfad.display()));
        dateien += 1;
        for event in &datei.events {
            if event.berechne_event_id() != event.event_id {
                selbst_diffs += 1;
            }
            let payload = serde_json::to_value(event).expect("Event serialisiert");
            let Some(obj) = payload.as_object().cloned() else {
                continue;
            };
            let mut ohne_id = obj;
            ohne_id.remove("event_id");
            let event_json = Value::Object(ohne_id);
            let rust_hex = event.event_id.to_string();
            let abweichung = diff_event_id(&mut oracle, &event_json, &rust_hex)
                .expect("Orakel-Aufruf laeuft durch");
            if abweichung.is_some() {
                python_diffs += 1;
            }
            geprueft += 1;
        }
    }
    eprintln!(
        "event_id_paritaet_ueber_bestandsdateien: {dateien} Dateien, {geprueft} Events, \
         {selbst_diffs} Selbst-Diffs, {python_diffs} Python-Diffs (keine Pfade/Werte ausgegeben)"
    );
    assert_eq!(
        dateien, erwartete_dateien,
        "store::lade sollte alle realen Faelle laden"
    );
    assert_eq!(
        selbst_diffs, 0,
        "event_id-Selbstpruefung weicht ab (Anzahl s.o., keine Pfade/Werte)"
    );
    assert_eq!(
        python_diffs, 0,
        "event_id-Paritaet zu Python weicht ab (Anzahl s.o., keine Pfade/Werte)"
    );
}

/// Deliverable #7b (Instructor-Nachforderung): jede reale Fall-Datei behaelt nach `lade`→
/// `speichere` dieselbe Top-Level-Schluesselmenge UND denselben Wert -- sonst haette `StoreDatei`
/// ein Feld, das Python schreibt (`scheibe`/`user_id`/`vorjahr_referenz`, gefunden ueber
/// `schema.json` + `grep produkt/ -- 'store\[.*\] ='`), lautlos verloren.
///
/// `speichere` schreibt NIE in den echten Fall-Ordner -- nur in ein Scratch-Verzeichnis unter
/// `std::env::temp_dir()`, das der Test am Ende wieder entfernt (SICHERHEIT: reale Nutzer-
/// Steuerdaten, s. Moduldoku oben).
///
/// Braucht keinen Oracle-Prozess (reiner Rust-Rundlauf), bleibt aber hinter [`skip_ohne_parity_env`]
/// -- dieselbe Opt-in-Schranke wie jeder andere Test, der die realen Fallakten anfasst.
#[test]
fn lade_speichere_roundtrip_ueber_bestandsdateien_falls_vorhanden() {
    if skip_ohne_parity_env() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht die realen Fallakten)");
        return;
    }
    let verzeichnis = faelle_verzeichnis();
    let kandidaten: Vec<std::path::PathBuf> = walk_json(&verzeichnis);
    assert!(
        !kandidaten.is_empty(),
        "lade_speichere_roundtrip: 0 Fall-Dateien unter {} -- ein Parity-Lauf ohne Korpus \
         belegt nichts.",
        verzeichnis.display()
    );
    let erwartete_dateien = kandidaten.len() as u64;
    let scratch = std::env::temp_dir().join(format!(
        "taxgraph-store-roundtrip-{}-{}",
        std::process::id(),
        jetzt_ns()
    ));
    std::fs::create_dir_all(&scratch).expect("scratch-verzeichnis anlegen");

    let mut dateien = 0u64;
    let mut schluesselsatz_diffs = 0u64;
    let mut werte_diffs = 0u64;
    for (i, pfad) in kandidaten.iter().enumerate() {
        let roh_text = std::fs::read_to_string(pfad)
            .unwrap_or_else(|e| panic!("lesen({}): {e}", pfad.display()));
        let roh: Value = serde_json::from_str(&roh_text)
            .unwrap_or_else(|e| panic!("json({}): {e}", pfad.display()));
        let roh_obj = roh
            .as_object()
            .unwrap_or_else(|| panic!("{} ist kein JSON-Objekt", pfad.display()));

        // P10: store::lade laedt inzwischen ALLE realen Faelle -- ein Ladefehler ist ein harter
        // Testabbruch, kein stiller Skip.
        let geladen =
            store::lade(pfad).unwrap_or_else(|e| panic!("store::lade({}): {e}", pfad.display()));
        dateien += 1;

        let temp_pfad = scratch.join(format!("{i}.json"));
        store::speichere(&temp_pfad, &geladen)
            .unwrap_or_else(|e| panic!("store::speichere(scratch): {e}"));
        let zurueck_text = std::fs::read_to_string(&temp_pfad).expect("scratch-datei lesen");
        let zurueck: Value = serde_json::from_str(&zurueck_text).expect("scratch-json parsen");
        let zurueck_obj = zurueck
            .as_object()
            .expect("scratch-wert ist ein JSON-Objekt");
        std::fs::remove_file(&temp_pfad).ok();

        let roh_keys: std::collections::BTreeSet<&String> = roh_obj.keys().collect();
        let zurueck_keys: std::collections::BTreeSet<&String> = zurueck_obj.keys().collect();
        if roh_keys != zurueck_keys {
            schluesselsatz_diffs += 1;
        }
        if roh != zurueck {
            werte_diffs += 1;
        }
    }
    std::fs::remove_dir_all(&scratch).ok();

    eprintln!(
        "lade_speichere_roundtrip: {dateien} Dateien, {schluesselsatz_diffs} Schluesselsatz-Diffs, \
         {werte_diffs} Werte-Diffs (keine Pfade/Werte ausgegeben)"
    );
    assert_eq!(
        dateien, erwartete_dateien,
        "store::lade sollte alle realen Faelle laden"
    );
    assert_eq!(
        schluesselsatz_diffs, 0,
        "Top-Level-Schluesselmenge weicht nach dem Rundlauf ab (Anzahl s.o.)"
    );
    assert_eq!(
        werte_diffs, 0,
        "Werte weichen nach dem Rundlauf ab (Anzahl s.o., keine Pfade/Werte)"
    );
}

fn jetzt_ns() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
}

fn expand_home(pfad: &str) -> std::path::PathBuf {
    if let Some(rest) = pfad.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home).join(rest);
        }
    }
    if pfad == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home);
        }
    }
    std::path::PathBuf::from(pfad)
}

/// `produkt/haut/api_constants.py::_daten_wurzel`/`FAELLE`: `TAXGRAPH_DATEN` (getrimmt,
/// nicht-leer) ersetzt die GESAMTE Wurzel (kein zusaetzliches `"taxgraph"`-Segment); sonst
/// `XDG_DATA_HOME` (getrimmt) oder `~/.local/share`, mit `"taxgraph"` verbunden. `/faelle` haengt
/// in JEDEM Fall am Ende an.
fn faelle_verzeichnis() -> std::path::PathBuf {
    let eigen = std::env::var("TAXGRAPH_DATEN").unwrap_or_default();
    let wurzel = if eigen.trim().is_empty() {
        let xdg = std::env::var("XDG_DATA_HOME").unwrap_or_default();
        let basis = if xdg.trim().is_empty() {
            expand_home("~").join(".local").join("share")
        } else {
            expand_home(xdg.trim())
        };
        basis.join("taxgraph")
    } else {
        expand_home(eigen.trim())
    };
    wurzel.join("faelle")
}

fn walk_json(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut gefunden = Vec::new();
    for eintrag in read.flatten() {
        let pfad = eintrag.path();
        if pfad.is_dir() {
            gefunden.extend(walk_json(&pfad));
        } else if pfad.extension().is_some_and(|e| e == "json") {
            gefunden.push(pfad);
        }
    }
    gefunden
}
