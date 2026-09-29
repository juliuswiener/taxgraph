//! eDaten-Uebernahme (`produkt/eingang/elster_writer.py`): Daten der Finanzverwaltung „gelten
//! als Angaben des Steuerpflichtigen, soweit er nicht abweichende Angaben macht" (§ 150 Abs. 7
//! S. 2 AO) — der einzige Importer, der BESTAETIGT schreibt ([`EdatenEvent`]). Eine eigene
//! Angabe hat Vorrang: nie ueberschreiben.
use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};
use store::{BindungNachschlag, Store};

use crate::vorschlag::{EdatenEvent, SchreibFehler};

/// Ein uebermittelter Satz (`{"feld_id", "wert", "kategorie"}`).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Satz {
    pub feld_id: String,
    pub wert: Value,
    #[serde(default)]
    pub kategorie: Value,
}

/// `uebernehme_edaten(store, edaten_record, ts=, bindung=)`.
///
/// `bindung = None` wie Python: keine Scheiben-Grenze und keine Auflage T (der Store bekommt
/// einen leeren Nachschlag). ponytail: ohne Bindung wird nichts geprueft und ein verworfener Satz
/// nicht gemeldet — wer den Writer verdrahtet, setzt die Bindung immer.
///
/// # Errors
/// [`SchreibFehler`] beim ersten abgewiesenen Satz (vorherige bleiben geschrieben, wie Python).
pub fn uebernehme(store: &mut Store, saetze: &[Satz], ts: Option<&str>, bindung: Option<BindungNachschlag<'_>>) -> Result<usize, SchreibFehler> {
    let leer = HashMap::new();
    let nachschlag = bindung.unwrap_or_else(|| BindungNachschlag::neu(&leer));
    // Python liest `aktiv` EINMAL vorab und ergaenzt es NICHT: ein doppelter Satz im selben
    // Aufruf trifft deshalb auf Auflage B im Store (Abweisung), statt still uebersprungen zu werden.
    let aktiv: HashSet<String> = store.aktive().map(|(f, _)| f.to_owned()).collect();
    let mut n = 0;
    for s in saetze {
        if bindung.is_some() && nachschlag.get(&s.feld_id).is_none() {
            continue;
        }
        if aktiv.contains(&s.feld_id) {
            continue;
        }
        let signal_1 = json!({"typ": "edaten", "quell_feld_id": s.feld_id, "quell_wert": s.wert, "kategorie": s.kategorie});
        EdatenEvent { feld_id: s.feld_id.clone(), wert: s.wert.clone(), signal_1 }.schreibe(store, nachschlag, ts)?;
        n += 1;
    }
    Ok(n)
}
