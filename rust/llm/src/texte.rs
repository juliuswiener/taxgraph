//! Feste Textteile der LLM-Prompts und die drei JSON-Schemas. Von Hand gepflegt: je Konstante eine Datei unter
//! `rust/llm/texte/` (`include_str!`), byte-genau. Ein Text traegt keinen Zeilenumbruch am Ende, den er nicht selbst
//! hat; mehrere beginnen oder enden auf `\n`. Ursprung: `produkt/haut/api_llm.py` und
//! `produkt/eingang/kontoauszug_writer.py`, Stand `2dd056a6` (der Erzeuger ist geloescht; Verlauf:
//! `git show 2dd056a6:tools/parity/gen_llm_texte.py`). `rust/llm/tests/prompt_entscheidungen.rs` haelt Zeichenzahl
//! und FNV-1a-64 der Systemtexte: ein Prompt-Edit aendert Text und Pin im selben Commit.
#![allow(clippy::doc_markdown)]

pub(crate) const AUSSAGEN_SYSTEM: &str = include_str!("../texte/aussagen_system.txt");
pub(crate) const THEMEN_KOPF: &str = include_str!("../texte/themen_kopf.txt");
pub(crate) const THEMEN_MITTE: &str = include_str!("../texte/themen_mitte.txt");
pub(crate) const THEMEN_ENDE: &str = include_str!("../texte/themen_ende.txt");
pub(crate) const DIALOG_KOPF: &str = include_str!("../texte/dialog_kopf.txt");
pub(crate) const DIALOG_REGELN: &str = include_str!("../texte/dialog_regeln.txt");
pub(crate) const AUSSAGEN_BLOCK_KOPF: &str = include_str!("../texte/aussagen_block_kopf.txt");
pub(crate) const AUSSAGEN_BLOCK_ENDE: &str = include_str!("../texte/aussagen_block_ende.txt");
pub(crate) const DIALOG_ENDE: &str = include_str!("../texte/dialog_ende.txt");
pub(crate) const KONTOAUSZUG_SYSTEM: &str = include_str!("../texte/kontoauszug_system.txt");
pub(crate) const DIALOG_SCHEMA_JSON: &str = include_str!("../texte/dialog_schema.json");
pub(crate) const AUSSAGEN_SCHEMA_JSON: &str = include_str!("../texte/aussagen_schema.json");
pub(crate) const ZUORDNUNG_SCHEMA_JSON: &str = include_str!("../texte/zuordnung_schema.json");
