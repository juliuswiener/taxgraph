#![no_main]
//! ELSTER: Schema-XML einlesen (XSD aus der ERiC-Doku), Deklaration -> XML -> Zurueckparsen, Kz-Text.
//! Byte 0 % 4 waehlt den Zweig: 0 = XSD, 1 = Snapshot-JSON, 3 = Snapshot-Zeilenform, 2 = Kz-/Feld-ID-Text.
use elster::{Felder, XmlOptionen};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&modus, rest)) = data.split_first() else { return };
    match modus % 4 {
        0 => {
            let p = taxgraph_fuzz::temp_datei("elster.xsd", rest);
            let _ = elster::xsd_walk(&p, "E10");
            let _ = elster::kz_meta(&p, "E10");
            let _ = elster::schema_info(&p);
        }
        m @ (1 | 3) => {
            let felder = if m == 1 {
                let Ok(f) = serde_json::from_slice::<Felder>(rest) else { return };
                f
            } else {
                felder_aus_zeilen(rest)
            };
            let bindung = taxgraph_fuzz::nachschlag();
            let Ok(d) = elster::deklariere(&felder, bindung, None) else { return };
            let _ = elster::zuruecklesen(&d, bindung);
            let opt = XmlOptionen { hersteller_id: Some("00000".into()), snapshot: Some(&felder), ..XmlOptionen::default() };
            if let Ok(xml) = elster::erzeuge_xml(&d, &opt) {
                // Roundtrip-Eigenschaft: was wir erzeugen, muss wohlgeformtes XML sein.
                // Bekannt (Paritaet mit Python/ElementTree): Steuerzeichen im Textwert landen roh im
                // XML, siehe regressions/elster/xml-nul-im-textwert.bin — dieser Fall wird uebersprungen.
                if xml.chars().any(|c| !matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')) {
                    return;
                }
                assert!(roxmltree::Document::parse(&xml).is_ok(), "erzeuge_xml lieferte kein wohlgeformtes XML");
            }
        }
        _ => {
            let Ok(text) = std::str::from_utf8(rest) else { return };
            let _ = elster::parse_instanz(text);
            let (kz, wert) = text.split_once('\n').unwrap_or((text, ""));
            let _ = elster::kz_format(kz);
            let j = serde_json::from_str::<serde_json::Value>(wert).unwrap_or(serde_json::Value::String(wert.to_owned()));
            let _ = elster::jahr_aus_kz_wert(&j, kz);
            for typ in [None, Some(domain::Feldtyp::Cent), Some(domain::Feldtyp::Int), Some(domain::Feldtyp::Text)] {
                let _ = elster::kz_wert(&j, kz, typ);
            }
        }
    }
});

/// Zeilenform `<idx|feld_id>\t<json-wert>`: eine Zahl waehlt ein echtes `feld_id` der Bindung
/// (sonst kaeme der Fuzzer kaum durch `deklariere`); gerade Zeilen bestaetigt, ungerade vorlaeufig.
fn felder_aus_zeilen(rest: &[u8]) -> Felder {
    let ids = taxgraph_fuzz::feld_ids();
    let mut felder = Felder::new();
    for (i, z) in String::from_utf8_lossy(rest).lines().enumerate() {
        let (k, w) = z.split_once('\t').unwrap_or((z, "null"));
        let fid = match k.split_once("__") {
            Some((n, inst)) if n.parse::<usize>().is_ok() => format!("{}__{inst}", ids[n.parse::<usize>().unwrap_or(0) % ids.len()]),
            _ => k.parse::<usize>().map_or_else(|_| k.to_owned(), |n| ids[n % ids.len()].clone()),
        };
        let wert: serde_json::Value = serde_json::from_str(w).unwrap_or_else(|_| w.into());
        let zustand = if i % 2 == 0 { "bestaetigt" } else { "vorlaeufig" };
        let f = serde_json::json!({"wert": wert, "zustand": zustand,
            "herkunft": {"herkunft": "nutzer", "pruef_tiefe": "amtlich", "haftung": "nutzer"}});
        if let Ok(f) = serde_json::from_value(f) {
            felder.insert(fid, f);
        }
    }
    felder
}
