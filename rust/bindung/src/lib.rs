//! `bindung` — Typen und Lader fuer die YAML-Konfigurationsschicht: Bindungstabellen
//! (`rust/bindung/daten/bindung_*.yaml`, Schema: `rust/bindung/daten/schema.json`) und die
//! Parameterschicht (`params/<vz>/*.yaml`, `params/kohorten/*.yaml`, Dokumentation:
//! `params/schema.md`). Haengt von `domain` ab (wiederverwendet `Feldtyp` fuer den
//! Bindungstyp, statt ihn ein zweites Mal zu deklarieren).
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

mod bindung_datei;
mod feld_bestand;
mod kohorten_datei;
mod params_datei;
mod params_zugriff;
mod registry;

pub use bindung_datei::{
    ist_gueltige_feld_id, lade_bindung, Ableitung, AbleitungArt, AnkerRef, Bereich, Beweist,
    Bindung, BindungDatei, BindungFehler, Bindungspunkt, FeldBedingung, InstanzGruppe, KzStatus,
    Luecke, Quelle, RegelBedingung, SlotBeitrag, ThemaZuerst, Vorjahr, VorschlagsSchreiber,
};
pub use feld_bestand::{
    bestand_zeilen, lade_feld_bestand, lade_feld_bestand_der_wurzel, nicht_erfasste, verschwundene,
    wiederaufgetauchte, zu_knappe_begruendungen, FeldBestand, FeldBestandFehler,
    FELD_BESTAND_DATEI, MIN_ZEICHEN_BEGRUENDUNG,
};
pub use kohorten_datei::{lade_kohorten, KohortenDatei, KohortenFehler};
pub use params_datei::{lade_params, Authority, ParamsDatei, ParamsFehler};
pub use params_zugriff::{
    AltersentlastungKohorte, ArbeitszimmerSaetze, DhfGrenzen, EntfernungspauschaleSaetze, Params,
    ParamsWertFehler, VerpflegungSaetze,
};
pub use params_zugriff::{
    FahrtkostenPauschalen, P33bPauschbetraege, SatzHoechstbetrag, VersorgungsfreibetragKohorte,
};
pub use registry::{
    lade_registry, lade_registry_der_wurzel, Registry, RegistryFehler, BINDUNG_VERZEICHNIS,
};
