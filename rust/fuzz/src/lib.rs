//! Gemeinsame Helfer der Fuzz-Targets (Testcode: `unwrap`/`panic` erlaubt).
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use bindung::Bindung;

/// Die echten Bindungsdateien des Repos (`produkt/bindung/bindung_*.yaml`), einmal je Prozess.
pub fn nachschlag() -> &'static HashMap<String, &'static Bindung> {
    static N: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    N.get_or_init(|| {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../produkt/bindung");
        let reg = bindung::lade_registry(std::path::Path::new(dir)).expect("Bindungsregistry ladbar");
        let alle: Vec<Bindung> = reg.dateien.into_iter().flat_map(|(_, d)| d.bindungen).collect();
        let alle: &'static [Bindung] = Box::leak(alle.into_boxed_slice());
        alle.iter().map(|b| (b.feld_id.clone(), b)).collect()
    })
}

/// Schreibt `bytes` in eine prozess-eigene Temp-Datei (`$TMPDIR`) und gibt den Pfad zurueck.
pub fn temp_datei(name: &str, bytes: &[u8]) -> PathBuf {
    let p = std::env::temp_dir().join(format!("taxgraph-fuzz-{}-{name}", std::process::id()));
    std::fs::write(&p, bytes).expect("Temp-Datei schreibbar");
    p
}

/// Sortierte `feld_id`s der Bindung (stabiler Index fuer zeilenbasierte Fuzz-Eingaben).
pub fn feld_ids() -> &'static Vec<String> {
    static I: OnceLock<Vec<String>> = OnceLock::new();
    I.get_or_init(|| {
        let mut v: Vec<String> = nachschlag().keys().cloned().collect();
        v.sort();
        v
    })
}
