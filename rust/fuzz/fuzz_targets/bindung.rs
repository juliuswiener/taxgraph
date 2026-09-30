#![no_main]
//! Bindungs-/Params-/Kohorten-YAML: die Lader nehmen Pfade, daher Temp-Datei je Ausfuehrung.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let p = taxgraph_fuzz::temp_datei("bindung.yaml", data);
    let _ = bindung::lade_bindung(&p);
    let _ = bindung::lade_params(&p);
    let _ = bindung::lade_kohorten(&p);
});
