//! `domain` — geteilte Typen fuer die Rust-Portierung: Geld (`Cent`/`Euro`), Veranlagungszeitraum
//! (`Vz`), Feld-Ids, der Zwei-Signal-Zustand samt Herkunfts-Algebra (`meet_zustand`/
//! `meet_herkunft`), Feldwerte (Auflage T) und der abschliessende Sperrgrund-Katalog.
//!
//! Quelle: `produkt/store/store.py`, `produkt/haut/api_constants.py`,
//! `produkt/bescheid/bescheid_deklaration.py`, `produkt/traverser/traverser.py` (siehe die
//! Modul-Dokumentation jeder Datei fuer die genaue Zeilen-Herkunft).
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

mod fall_id;
mod feld_id;
mod herkunft;
mod konfession;
mod kz;
mod lage;
mod meet;
mod money;
mod py_text;
mod py_wert;
mod rentenart;
mod sperrgrund;
#[cfg(feature = "testhilfe")]
pub mod testhilfe;
mod veranlagung;
mod vorschlag_typ;
mod vz;
mod wert;
mod zustand;

pub use fall_id::{FallId, UngueltigeFallId};
pub use feld_id::{BasisId, FeldId, FeldIdFehler};
pub use herkunft::{
    Achsenwert, Herkunft, HerkunftAlt, HerkunftVektor, LeererAchsenwert, Schreiber, KONFLIKT,
};
pub use konfession::Konfession;
pub use kz::{Kz, UngueltigeKz};
pub use lage::Lage;
pub use meet::{meet_herkunft, meet_zustand, MeetFehler};
pub use money::{Cent, CentUeberlauf, Euro, Km, Satz};
pub use py_text::{py_strip, repr_float};
pub use py_wert::{PyFehler, PyWert};
pub use rentenart::Rentenart;
pub use sperrgrund::{Sperrgrund, UnbekannterSperrgrund, UNBEKANNTER_SPERRGRUND};
pub use veranlagung::{Person, Scheibe, UnbekannteScheibe, Veranlagung};
pub use vorschlag_typ::{UnbekannterVorschlagTyp, VorschlagTyp};
pub use vz::{UngueltigeVz, Vz};
pub use wert::{nur_xml_zeichen, Feldtyp, Wert, WertFehler};
pub use zustand::{Feldzustand, LeeresSignal2, PruefTiefe, Signal2, Zustand};
