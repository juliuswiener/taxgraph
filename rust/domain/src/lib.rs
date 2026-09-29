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

mod feld_id;
mod herkunft;
mod meet;
mod money;
mod sperrgrund;
mod veranlagung;
mod vz;
mod wert;
mod zustand;

pub use feld_id::{BasisId, FeldId, FeldIdFehler};
pub use herkunft::{Achsenwert, Herkunft, HerkunftAlt, HerkunftVektor, LeererAchsenwert, Schreiber, KONFLIKT};
pub use meet::{meet_herkunft, meet_zustand, MeetFehler};
pub use money::{Cent, CentUeberlauf, Euro};
pub use sperrgrund::{Sperrgrund, UnbekannterSperrgrund, UNBEKANNTER_SPERRGRUND};
pub use veranlagung::{Person, Scheibe, UnbekannteScheibe, Veranlagung};
pub use vz::{UngueltigeVz, Vz};
pub use wert::{Feldtyp, Wert, WertFehler};
pub use zustand::{Feldzustand, LeeresSignal2, PruefTiefe, Signal2, Zustand};
