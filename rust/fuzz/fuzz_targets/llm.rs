#![no_main]
//! LLM-Ausgabe ist externe Eingabe: jeder Parser darf nie panicken, egal was das Modell liefert.
use std::collections::HashSet;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&n, rest)) = data.split_first() else { return };
    let Ok(text) = std::str::from_utf8(rest) else { return };
    // Text vor der ersten Zeile = Modellausgabe, danach = gefilterter Nutzertext fuer die Belege.
    let (ausgabe, frei) = text.split_once('\n').unwrap_or((text, ""));
    let (gefiltert, _) = llm::pii::filtere(frei);

    let _ = llm::parse::chat_parse(ausgabe);
    let _ = llm::parse::rueckfragen_parse(ausgabe, usize::from(n));
    let _ = llm::parse::antwort_parse(ausgabe);
    let _ = llm::parse::aussagen_parse(ausgabe, &gefiltert);
    let erlaubt: HashSet<String> = ["r1", "r2", "a"].iter().map(|s| (*s).to_owned()).collect();
    let _ = llm::parse::zuordnung_parse(ausgabe, &erlaubt, usize::from(n));
    let _ = llm::kontoauszug::parse_kategorie(ausgabe);
});
