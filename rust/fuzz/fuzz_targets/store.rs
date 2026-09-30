#![no_main]
//! Die Store-Datei (YAML/JSON), Audit- und Fehler-Log (JSONL) sind externe Eingabe (nur lesend).
//! Roundtrip: lade -> speichere -> lade muss denselben Inhalt liefern.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&modus, rest)) = data.split_first() else { return };
    if modus % 4 == 2 {
        let p = taxgraph_fuzz::temp_datei("audit.jsonl", rest);
        let _ = store::audit::lies(&p);
        return;
    }
    if modus % 4 == 3 {
        let p = taxgraph_fuzz::temp_datei("fehler.jsonl", rest);
        let _ = store::fehler_log::lies(&p);
        return;
    }
    let p = taxgraph_fuzz::temp_datei("store.yaml", rest);
    let Ok(datei) = store::lade(&p) else { return };
    let Ok(v1) = serde_json::to_value(&datei) else { return };
    let p2 = taxgraph_fuzz::temp_datei("store-rt.json", b"");
    store::speichere(&p2, &datei).expect("speichere nach erfolgreichem lade");
    let datei2 = store::lade(&p2).expect("eigene Ausgabe muss ladbar sein");
    let v2 = serde_json::to_value(&datei2).expect("erneut serialisierbar");
    assert_eq!(v1, v2, "Store lade->speichere->lade nicht stabil");
});
