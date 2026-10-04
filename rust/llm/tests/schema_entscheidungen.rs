//! Wann nimmt `llm::parse` eine Modellantwort als schemagerecht an? (`schema.rs`: die `deny_unknown_fields`-Strukturen zu
//! `DIALOG_SCHEMA`, `AUSSAGEN_SCHEMA`, `ZUORDNUNG_SCHEMA`), am Aufrufort der Parser geprueft (N4, Mutationsmessung
//! `rust/llm`, Teil `schema.rs`). Die Erwartung je Dokument ist der Urteilsspruch von `jsonschema` (Draft 2020-12) ueber
//! das Schema der Python-Referenz (`api_llm.DIALOG_SCHEMA` usw.) — nicht Rusts eigene Lesart. Schemagerecht heisst
//! `Antwort::Schemagerecht`, sonst `Antwort::Tolerant` (lesbar, aber nicht schematreu). Kein Test ruft Python.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::HashSet;

use llm::parse::{antwort_parse, aussagen_parse, zuordnung_parse};
use llm::pii::filtere;
use llm::Antwort;

/// Stufe 3 (`DIALOG_SCHEMA`): Pflichtfelder, `wert` nur Text/Zahl/Wahrheitswert, `rechenweg` Pflicht aber `null` erlaubt, `aussage` ganzzahlig auch -1, keine fremden Felder.
#[test]
#[rustfmt::skip]
fn dialog_schemagerecht_wie_jsonschema() {
    let faelle: &[(&str, bool)] = &[
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [], \"rueckfragen\": [], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}, {\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}, {\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": \"text\", \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1.5, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": true, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": false, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": null, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": [1], \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": {\"a\": 1}, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": -3, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 0, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": {\"basis\": 1, \"faktor\": 0.5, \"erklaerung\": \"e\"}}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": {\"basis\": 1.5, \"faktor\": 2, \"erklaerung\": \"e\"}}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": {\"faktor\": 2, \"erklaerung\": \"e\"}}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": {\"basis\": 1, \"erklaerung\": \"e\"}}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": {\"basis\": \"1\", \"faktor\": 2, \"erklaerung\": \"e\"}}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": {\"basis\": 1, \"faktor\": \"2\", \"erklaerung\": \"e\"}}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": {\"basis\": 1, \"faktor\": 2}}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": {\"basis\": 1, \"faktor\": 2, \"erklaerung\": \"e\", \"x\": 1}}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": {\"basis\": 1, \"faktor\": 2, \"erklaerung\": 5}}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": 5}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": \"text\"}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": []}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": true}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": -1, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": -1}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 3, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 3}], \"antwort\": \"x\", \"unsicher\": false}", true),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 1.5, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 1.5}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": \"0\", \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": \"0\"}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": null, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": null}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": true, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": true}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false, \"x\": 1}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null, \"x\": 1}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0, \"x\": 1}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\"}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\"}", false),
        ("{\"vorschlaege\": [{\"feld_id\": 5, \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": null, \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": 1, \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": 1, \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": null, \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": null, \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": 5, \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": \"false\"}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": 0}", false),
        ("{\"vorschlaege\": {}, \"rueckfragen\": [{\"frage\": \"f\", \"feld_id\": \"a\", \"aussage\": 0}], \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{\"vorschlaege\": [{\"feld_id\": \"a\", \"wert\": 1, \"beleg\": \"b\", \"begruendung\": \"c\", \"aussage\": 0, \"rechenweg\": null}], \"rueckfragen\": \"x\", \"antwort\": \"x\", \"unsicher\": false}", false),
        ("{}", false),
    ];
    for (text, erwartet) in faelle {
        assert_eq!(matches!(antwort_parse(text), Antwort::Schemagerecht(_)), *erwartet, "{text}");
    }
}

/// Stufe 1 (`AUSSAGEN_SCHEMA`).
#[test]
#[rustfmt::skip]
fn aussagen_schemagerecht_wie_jsonschema() {
    let faelle: &[(&str, bool)] = &[
        ("{\"aussagen\": [{\"text\": \"t\", \"beleg\": \"b\"}]}", true),
        ("{\"aussagen\": []}", true),
        ("{\"aussagen\": [{\"text\": \"t\", \"beleg\": \"b\"}, {\"text\": \"u\", \"beleg\": \"c\"}]}", true),
        ("{\"aussagen\": [{\"text\": \"t\", \"beleg\": \"b\"}], \"x\": 1}", false),
        ("{\"aussagen\": [{\"text\": \"t\", \"beleg\": \"b\", \"x\": 1}]}", false),
        ("{\"aussagen\": [{\"beleg\": \"b\"}]}", false),
        ("{\"aussagen\": [{\"text\": \"t\"}]}", false),
        ("{\"aussagen\": [{\"text\": 5, \"beleg\": \"b\"}]}", false),
        ("{\"aussagen\": [{\"text\": \"t\", \"beleg\": null}]}", false),
        ("{\"aussagen\": {}}", false),
        ("{\"aussagen\": \"x\"}", false),
        ("{}", false),
        ("{\"aussagen\": [5]}", false),
    ];
    for (text, erwartet) in faelle {
        assert_eq!(matches!(aussagen_parse(text, &filtere("x").0), Antwort::Schemagerecht(_)), *erwartet, "{text}");
    }
}

/// Stufe 2 (`ZUORDNUNG_SCHEMA`): `aussage` ganzzahlig auch -1, `regeln` Liste von Texten, keine fremden Felder.
#[test]
#[rustfmt::skip]
fn zuordnung_schemagerecht_wie_jsonschema() {
    let faelle: &[(&str, bool)] = &[
        ("{\"zuordnungen\": [{\"aussage\": 0, \"regeln\": [\"r\"]}]}", true),
        ("{\"zuordnungen\": []}", true),
        ("{\"zuordnungen\": [{\"aussage\": -1, \"regeln\": [\"r\"]}]}", true),
        ("{\"zuordnungen\": [{\"aussage\": 0, \"regeln\": [\"r\"]}]}", true),
        ("{\"zuordnungen\": [{\"aussage\": 5, \"regeln\": [\"r\"]}]}", true),
        ("{\"zuordnungen\": [{\"aussage\": \"0\", \"regeln\": [\"r\"]}]}", false),
        ("{\"zuordnungen\": [{\"aussage\": 1.5, \"regeln\": [\"r\"]}]}", false),
        ("{\"zuordnungen\": [{\"aussage\": null, \"regeln\": [\"r\"]}]}", false),
        ("{\"zuordnungen\": [{\"aussage\": 0, \"regeln\": [\"r\"]}], \"x\": 1}", false),
        ("{\"zuordnungen\": [{\"aussage\": 0, \"regeln\": [\"r\"], \"x\": 1}]}", false),
        ("{\"zuordnungen\": [{\"regeln\": [\"r\"]}]}", false),
        ("{\"zuordnungen\": [{\"aussage\": 0}]}", false),
        ("{\"zuordnungen\": [{\"aussage\": 0, \"regeln\": [1]}]}", false),
        ("{\"zuordnungen\": [{\"aussage\": 0, \"regeln\": \"r\"}]}", false),
        ("{\"zuordnungen\": [{\"aussage\": 0, \"regeln\": []}]}", true),
        ("{\"zuordnungen\": {}}", false),
        ("{}", false),
        ("{\"zuordnungen\": [5]}", false),
    ];
    for (text, erwartet) in faelle {
        assert_eq!(matches!(zuordnung_parse(text, &HashSet::<String>::new(), 1), Antwort::Schemagerecht(_)), *erwartet, "{text}");
    }
}
