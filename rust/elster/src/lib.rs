//! `elster` — die ELSTER-Seite des Produkts (`REWRITE_PLAN.md` §2.2, Schritt 6):
//!
//! - [`deklariere`] / [`zuruecklesen`] / [`instanzen`] / [`parse_instanz`]: Store-Snapshot →
//!   Kz-Deklaration und zurueck (`produkt/mapping/est_mapping.py`).
//! - [`cent_nach_kz`] / [`kz_wert`]: Kz-Format mit Rundung zugunsten der Steuerpflichtigen.
//! - [`erzeuge_xml`]: Deklaration → ELSTER-E10-XML, byte-gleich zum Original
//!   (`produkt/eingang/elster_xml.py`).
//! - [`schema_info`] / [`pruefe_bindung`]: das amtliche XSD als Quelle der Kz-Pfade und das
//!   Pruefwerkzeug fuer die Bindung (`produkt/mapping/xsd_verify.py`).
//! - [`validiere`] / [`klassifiziere_rc`]: checkESt ueber ERiC-FFI, NUR `ERIC_VALIDIERE`
//!   (`elster/checkest_gate.py`, `elster/smoke_test.py`).
//! - [`validiere_xsd`]: `xmllint`-Struktur-Gate (`elster/submission/validate_xsd.py`).
//!
//! Nicht portiert: `elster/versand.py` (Echtversand ist Julius vorbehalten) und das Tooling
//! `kz_extract`, `validate_mapping`, `bench`, `fuzz`, `eric_gate`.
#![deny(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

mod deklaration;
mod eric;
mod geordnet;
mod instanz;
mod kz_format;
mod py;
mod tabellen;
mod xml;
mod xmllint;
mod xsd;

pub use deklaration::{
    deklariere, Aggregat, AnlageInstanz, BindungIndex, Deklaration, DeklarationsFehler, Eintrag,
    Felder, KindAnlage,
};
pub use eric::{
    eric_log_pfad, find_eric_lib, gekappt_verdacht, klassifiziere_rc, nicht_geprueft, validiere,
    EricFehler, EricKlasse, ERIC_VALIDIERE, RC_DATENARTVERSION_UNBEKANNT, RC_HERSTELLER_GESPERRT,
    RC_IO_SCHEMA_VALIDIERUNGSFEHLER, RC_IO_UNERWARTETE_ELEMENTE, RC_OK, RC_PLAUSIBILITAET,
    VALIDIERE_MELDUNGEN_MAX,
};
pub use instanz::{instanzen, parse_instanz, zuruecklesen, Instanz, Rueckgelesen};
pub use kz_format::{
    cent_nach_kz, jahr_aus_kz_wert, kz_format, kz_wert, KzBetrag, KzFormat, ABZUGS_KZ, DATUMS_KZ,
    KOMMA_OHNE_E60_KZ, NULL_UNZULAESSIG_KZ,
};
pub use py::PyFehler;
pub use tabellen::{IBAN_TRANSFORM_ZIEL_KZ, KONSTANTE_KZ};
pub use xml::{erzeuge_xml, XmlFehler, XmlOptionen, NS_ELSTER, TESTMERKER_ERIC};
pub use xmllint::{finde_xsd_schema, validiere_xsd, validiere_xsd_text};
pub use xsd::{
    ernte_est_mapping_kz, finde_schema, ist_ja_typ, kz_meta, pruefe_bindung, schema_info, xsd_walk,
    FeldPruefung, JahrPruefung, KzFundstellen, KzMeta, KzPruefling, PflichtKinder, PruefStatus,
    Pruefbericht, SchemaInfo, XsdFehler, MAX_DEPTH,
};
